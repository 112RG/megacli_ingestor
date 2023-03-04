use chrono::{DateTime, Utc};
use influxdb::Client;
use influxdb::{Error, InfluxDbWriteable};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::Semaphore;
#[derive(Debug, Default, InfluxDbWriteable)]
#[allow(dead_code)]
struct Drive {
    time: DateTime<Utc>,
    enclosure_device_id: u32,
    #[influxdb(tag)]
    slot_number: u32,
    media_error_count: u32,
    other_error_count: u32,
    firmware_state: String,
    inquiry_data: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let output = Command::new("/usr/sbin/megacli")
        .arg("-PDList")
        .arg("-aAll")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let egrep_output = Command::new("egrep")
        .arg("Enclosure Device ID:|Slot Number:|Inquiry Data:|Error Count:|state")
        .stdin(output.stdout.unwrap())
        .output()
        .expect("Failed to execute command");

    println!("status: {}", egrep_output.status);
    println!("stderr: {}", String::from_utf8_lossy(&egrep_output.stderr));
    let input = String::from_utf8_lossy(&egrep_output.stdout);
    println!("{:}", input);
    let mut drives: Vec<Drive> = Vec::new();
    let time = SystemTime::now().into();
    for line in input.lines() {
        let (key, value) = line.split_once(": ").unwrap();
        match key.trim() {
            "Enclosure Device ID" => {
                let enclosure_device_id = value.trim().parse::<u32>().unwrap();
                drives.push(Drive {
                    time,
                    enclosure_device_id,
                    slot_number: 0,
                    media_error_count: 0,
                    other_error_count: 0,
                    firmware_state: String::new(),
                    inquiry_data: String::new(),
                });
            }
            "Slot Number" => {
                let slot_number = value.trim().parse::<u32>().unwrap();
                drives.last_mut().unwrap().slot_number = slot_number;
            }
            "Media Error Count" => {
                let media_error_count = value.trim().parse::<u32>().unwrap();
                drives.last_mut().unwrap().media_error_count = media_error_count;
            }
            "Other Error Count" => {
                let other_error_count = value.trim().parse::<u32>().unwrap();
                drives.last_mut().unwrap().other_error_count = other_error_count;
            }
            "Firmware state" => {
                drives.last_mut().unwrap().firmware_state = match value.trim() {
                    "Failed" => String::from("0"),
                    "Online, Spun Up" => String::from("1"),
                    _ => String::from("2"),
                };
            }
            "Inquiry Data" => {
                let inquiry_data = value.trim().to_string();
                drives.last_mut().unwrap().inquiry_data = inquiry_data;
            }
            _ => {}
        }
    }

    println!("{:#?}", drives);

    let client = Client::new("http://localhost:8086", "telegraf");
    let (tx, mut rx) = unbounded_channel::<Result<String, Error>>();
    let concurrency_limit = Arc::new(Semaphore::new(5));
    for m in drives {
        let permit = concurrency_limit.clone().acquire_owned().await;
        let client_task = client.clone();
        let tx_task = tx.clone();
        tokio::spawn(async move {
            let res = client_task.query(&m.into_query("raid")).await;
            let _ = tx_task.send(res);
            drop(permit);
        });
    }
    drop(tx);
    let mut successful_count = 0;
    let mut error_count = 0;
    while let Some(res) = rx.recv().await {
        if res.is_err() {
            error_count += 1;
        } else {
            successful_count += 1;
        }
    }
    println!(
        "{} successful requests, {} errors",
        successful_count, error_count
    );
}
