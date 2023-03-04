# megacli ingestor

This is a small rust program to read the status of a raid from ``megacli``


The command that is run to read the data is 
``megacli -PDList -aAll | egrep "Enclosure Device ID:|Slot Number:|Inquiry Data:|Error Count:|state"``

# Insert strucutre

slot_number
media_error_count
other_error_count
firmware_state

