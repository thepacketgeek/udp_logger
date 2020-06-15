# UDP Logger

A UDP datagram logger for use with the [log crate](https://docs.rs/log/), to allow for on-demand viewing of logs:

```
use log::info;

use udp_logger::UdpLoggerBuilder;

fn main() -> std::io::Result<()> {
    UdpLoggerBuilder::try_init("127.0.0.1:1999", log::Level::Info)?;

    loop {
        info!("testing {} things", 1);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

Can be viewed as desired by running `netcat` (or similar) on the receiving host:
```
% ncat -lku localhost 1999
INFO [2020-06-15T03:15:39.740912039+00:00] testing 1 things
INFO [2020-06-15T03:15:40.741074924+00:00] testing 1 things
INFO [2020-06-15T03:15:41.741258993+00:00] testing 1 things
```