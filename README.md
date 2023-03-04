# megacli ingestor

This is a small rust program to read the status of a raid from ``megacli``


The command that is run to read the data is 
``megacli -PDList -aAll | egrep "Enclosure Device ID:|Slot Number:|Inquiry Data:|Error Count:|state"``

# Insert strucutre

```
time: DateTime<Utc>,
enclosure_device_id: u32,
#[influxdb(tag)]
slot_number: u32,
media_error_count: u32,
other_error_count: u32,
firmware_state: String,
inquiry_data: String,
```

