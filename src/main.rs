use log::info;

use udp_logger::UdpLoggerBuilder;

fn main() {
    UdpLoggerBuilder::try_init("127.0.0.1:1999", log::Level::Info).unwrap();

    loop {
        info!("testing {} things", 1);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
