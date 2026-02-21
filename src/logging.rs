use std::net::TcpStream;
use std::io::Write;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_elk_logging(logstash_host: &str) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    Ok(())
}

pub struct LogstashWriter {
    stream: Option<TcpStream>,
    host: String,
}

impl LogstashWriter {
    pub fn new(host: String) -> Self {
        Self {
            stream: TcpStream::connect(&host).ok(),
            host,
        }
    }

    pub fn send(&mut self, log: &str) {
        if self.stream.is_none() {
            self.stream = TcpStream::connect(&self.host).ok();
        }

        if let Some(ref mut stream) = self.stream {
            let _ = writeln!(stream, "{}", log);
        }
    }
}
