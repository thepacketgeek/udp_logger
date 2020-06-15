use std::collections::VecDeque;
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use log::{self, Level, Metadata, Record, SetLoggerError};

#[derive(Debug)]
struct UdpWriter {
    messages: Arc<Mutex<VecDeque<String>>>,
}

impl UdpWriter {
    pub fn new(destination: impl ToSocketAddrs) -> std::io::Result<Self> {
        let messages: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        {
            let m_clone = messages.clone();
            let dest = destination.to_socket_addrs()?.into_iter().next().unwrap();
            thread::spawn(move || loop {
                if let Ok(mut messages) = m_clone.lock() {
                    while let Some(message) = messages.pop_front() {
                        UdpSocket::bind("0.0.0.0:0")
                            .unwrap()
                            .send_to(message.as_bytes(), dest)
                            .map_err(|e| eprintln!("Error sending message: {}", e))
                            .ok();
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            });
        }
        Ok(Self { messages })
    }

    fn push(&self, message: String) {
        self.messages.lock().unwrap().push_back(message);
    }
}

#[derive(Debug)]
pub struct UdpLogger {
    writer: UdpWriter,
    level: Level,
}

impl UdpLogger {
    pub fn new(destination: impl ToSocketAddrs) -> std::io::Result<Self> {
        let writer = UdpWriter::new(destination)?;
        Ok(Self {
            writer,
            level: Level::Info,
        })
    }

    pub fn set_level(&mut self, level: Level) -> &mut Self {
        self.level = level;
        self
    }
}

impl log::Log for UdpLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.writer.push(format!(
                "{} [{}] {}\n",
                record.level(),
                Utc::now().to_rfc3339(),
                record.args()
            ));
        }
    }

    fn flush(&self) {}
}

#[derive(Debug)]
pub struct UdpLoggerBuilder;

impl UdpLoggerBuilder {
    pub fn try_init(destination: impl ToSocketAddrs, level: Level) -> Result<(), SetLoggerError> {
        let logger = UdpLogger::new(destination).unwrap();

        let r = log::set_boxed_logger(Box::new(logger))
            .map(|()| log::set_max_level(level.to_level_filter()));
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;

    #[test]
    fn test_new() {
        UdpLogger::new("127.0.0.1:1999")
            .expect("Can bind to localhost")
            .set_level(Level::Debug);
    }

    #[test]
    fn test_message_queue() {
        let logger = UdpLogger::new("127.0.0.1:1999")
            .expect("Can bind to localhost")
            .set_level(Level::Debug);

        info!("testing");
    }
}
