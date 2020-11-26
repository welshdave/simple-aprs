use std::error::Error;
use std::time::Duration;

use futures::sink::SinkExt;
use futures::StreamExt;

use log::{error, info, trace, warn};

use tokio::net::TcpStream;
use tokio::time;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite, LinesCodec};

pub struct APRSMessage {
    pub raw: Vec<u8>,
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
    pub fn new(settings: ISSettings, message_handler: MessageHandler) -> IS {
        IS {
            settings,
            message_handler,
        }
    }

    #[tokio::main]
    pub async fn connect(&self) -> Result<(), Box<dyn Error>> {
        let address = format!("{}:{}", self.settings.host, self.settings.port);

        let stream = TcpStream::connect(address).await?;

        let (r, w) = stream.into_split();

        let mut writer = FramedWrite::new(w, LinesCodec::new());
        let mut reader = FramedRead::new(r, BytesCodec::new());

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

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                writer.send("# keep alive").await.unwrap();
            }
        });

        while let Some(message) = reader.next().await {
            match message {
                Ok(mut message) => {
                    message.truncate(message.len() - 2);
                    if message[0] == b'#' {
                        match String::from_utf8(message.to_vec()) {
                            Ok(server_message) => {
                                trace!("Recieved server response: {}", server_message)
                                // check logged in etc.
                            }
                            Err(err) => warn!("Error processing server response: {}", err),
                        }
                    } else {
                        (self.message_handler)(APRSMessage {
                            raw: (message).to_vec(),
                        });
                    }
                }
                Err(err) => {
                    warn!("Error processing message from APRS-IS server: {}", err);
                }
            }
        }

        Ok(())
    }
}
