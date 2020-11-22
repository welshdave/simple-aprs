use futures::sink::SinkExt;
use futures::StreamExt;

use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

use std::error::Error;

pub struct APRSMessage {
    pub raw: String,
}

pub struct ISSettings {
    pub host: String,
    pub port: u16,
    pub callsign: String,
    pub passcode: String,
    pub command: String,
}

impl ISSettings {
    pub fn new(
        host: String,
        port: u16,
        callsign: String,
        passcode: String,
        command: String,
    ) -> ISSettings {
        ISSettings {
            host,
            port,
            callsign,
            passcode,
            command,
        }
    }
}

pub type MessageHandler = fn(APRSMessage);

pub struct IS {
    settings: ISSettings,
    message_handler: MessageHandler,
}

impl IS {
    pub fn new(settings: ISSettings) -> IS {
        IS {
            settings,
            message_handler: IS::null_message_handler,
        }
    }

    pub fn register_message_handler(&mut self, handler: MessageHandler) {
        self.message_handler = handler;
    }

    #[tokio::main]
    pub async fn connect(&self) -> Result<(), Box<dyn Error>> {
        let address = format!("{}:{}", self.settings.host, self.settings.port);

        let mut stream = TcpStream::connect(address).await?;

        let (r, w) = stream.split();

        let mut writer = FramedWrite::new(w, LinesCodec::new());
        let mut reader = FramedRead::new(r, LinesCodec::new());

        let login_message = {
            let name = option_env!("CARGO_PKG_NAME").unwrap_or("unknown");
            let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");

            format!(
                "user {} pass {} vers {} {} {}",
                self.settings.callsign,
                self.settings.passcode,
                name,
                version,
                self.settings.command
            )
        };

        writer.send(login_message).await?;

        while let Some(Ok(line)) = reader.next().await {
            (self.message_handler)(APRSMessage { raw: line });
        }

        Ok(())
    }

    fn null_message_handler(_message: APRSMessage) {}
}
