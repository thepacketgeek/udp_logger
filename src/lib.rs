use std::collections::VecDeque;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use log::{self, Level, Metadata, Record, SetLoggerError};

/// UdpLogger is a Log adaptor for sending messages as UDP datagrams
///
/// It is backed by two UDP sending strategies: unbuffered and buffered
pub struct UdpLogger {
    writer: Box<dyn Writer>,
    level: Level,
}

impl UdpLogger {
    /// Create a new, unbuffered UdpLogger that sends datagrams to the given destination
    pub fn new(destination: impl ToSocketAddrs) -> io::Result<Self> {
        let writer = UdpWriter::new(destination)?;
        Ok(Self {
            writer: Box::new(writer),
            level: Level::Info,
        })
    }

    /// Create a new, buffered UdpLogger that sends datagrams to the given destination
    pub fn new_buffered(destination: impl ToSocketAddrs) -> io::Result<Self> {
        let writer = UdpBufferedWriter::new(destination)?;
        Ok(Self {
            writer: Box::new(writer),
            level: Level::Info,
        })
    }

    /// Modify the log level (default == INFO)
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
            let _ = self.writer.push(format!(
                "{} [{}] {}\n",
                record.level(),
                Utc::now().to_rfc3339(),
                record.args()
            ));
        }
    }

    fn flush(&self) {}
}

/// Easily initialize the UdpLogger adapter with `Log` using this UdpLogger builder
/// ```
/// use log::info;
/// use udp_logger::UdpLoggerBuilder;
///
/// // Init the UdpLoggerBuilder and use `log` macros to send log messages over UDP
/// UdpLoggerBuilder::try_init("127.0.0.1:1999", log::Level::Info).unwrap();
///
/// info!("This will get sent via UDP!");
/// ```
pub struct UdpLoggerBuilder;

impl UdpLoggerBuilder {
    /// Initialize an unbuffered UdpLogger as a destination for `Log` macros
    pub fn try_init(
        destination: impl ToSocketAddrs,
        level: Level,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut logger = UdpLogger::new(destination).unwrap();
        logger.set_level(level);
        UdpLoggerBuilder::init(logger).map_err(|e| e.into())
    }

    /// Initialize a buffered UdpLogger as a destination for `Log` macros
    pub fn try_buffered_init(
        destination: impl ToSocketAddrs,
        level: Level,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut logger = UdpLogger::new(destination)?;
        logger.set_level(level);
        UdpLoggerBuilder::init(logger).map_err(|e| e.into())
    }

    fn init(logger: UdpLogger) -> Result<(), SetLoggerError> {
        let level_filter = logger.level.to_level_filter();
        let r = log::set_boxed_logger(Box::new(logger)).map(|()| log::set_max_level(level_filter));
        r
    }
}

/// Writer is used by UdpLogger to send UDP datagrams
trait Writer: Send + Sync {
    fn push(&self, message: String) -> io::Result<()>;
}

/// UdpWriter is an unbuffered writer and datagrams will be sent immediately
/// via a UdpSocket (system determined IP & port)
struct UdpWriter {
    out: UdpSocket,
    destination: SocketAddr,
}

impl UdpWriter {
    /// Create a new UdpWriter that sends messages to the given destination `SocketAddr`
    pub fn new(destination: impl ToSocketAddrs) -> io::Result<Self> {
        let dest = destination
            .to_socket_addrs()?
            .into_iter()
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, ""))?;
        Ok(Self {
            destination: dest,
            out: UdpSocket::bind("0.0.0.0:0")?,
        })
    }
}

impl Writer for UdpWriter {
    /// This is used by `Log` to write the message as a datagram
    fn push(&self, message: String) -> io::Result<()> {
        self.out
            .send_to(message.as_bytes(), self.destination)
            .map(|_| ())
    }
}

/// UdpBufferedWriter is an alternate UdpWriter that buffers submitted messages
/// and sends in a background thread
struct UdpBufferedWriter {
    messages: Arc<Mutex<VecDeque<String>>>,
}

impl UdpBufferedWriter {
    /// Create a new UdpBufferedWriter that sends messages to the given destination `SocketAddr`
    pub fn new(destination: impl ToSocketAddrs) -> io::Result<Self> {
        let messages: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        {
            let m_clone = messages.clone();
            let dest = destination
                .to_socket_addrs()?
                .into_iter()
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, ""))?;

            let out = UdpSocket::bind("0.0.0.0:0")?;
            thread::spawn(move || loop {
                if let Ok(mut messages) = m_clone.lock() {
                    while let Some(message) = messages.pop_front() {
                        out.send_to(message.as_bytes(), dest)
                            .map_err(|e| eprintln!("Error sending message: {}", e))
                            .ok();
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            });
        }
        Ok(Self { messages })
    }
}

impl Writer for UdpBufferedWriter {
    fn push(&self, message: String) -> io::Result<()> {
        self.messages.lock().unwrap().push_back(message);
        Ok(())
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
        let _ = UdpLogger::new("127.0.0.1:1999")
            .expect("Can bind to localhost")
            .set_level(Level::Debug);

        info!("testing");
    }
}
