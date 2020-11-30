extern crate aprs;
extern crate fap;

use std::error::Error;
use std::time::Duration;

use futures::sink::SinkExt;
use futures::StreamExt;

use log::{info, trace, warn};

use tokio::net::TcpStream;
use tokio::time;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite, LinesCodec};

pub struct APRSPacket {
    pub raw: Vec<u8>,
}

impl APRSPacket {
    pub fn parsed(&self) -> Result<Box<dyn aprs::Packet>, Box<dyn Error>> {
        let raw_packet = self.raw.clone();
        let parsed = fap::Packet::new(raw_packet);
        match parsed {
            Ok(packet) => {
                let boxed_packet = Box::new(packet);
                return Ok(boxed_packet);
            }
            Err(err) => {
                let boxed_error = Box::new(err);
                return Err(boxed_error);
            }
        }
    }
}

pub struct ISSettings {
    pub host: String,
    pub port: u16,
    pub callsign: String,
    pub passcode: String,
    pub filter: String,
}

impl ISSettings {
    pub fn new(
        host: String,
        port: u16,
        callsign: String,
        passcode: String,
        filter: String,
    ) -> ISSettings {
        ISSettings {
            host,
            port,
            callsign,
            passcode,
            filter,
        }
    }
}

pub type PacketHandler = fn(APRSPacket);

pub struct IS {
    settings: ISSettings,
    packet_handler: PacketHandler,
}

impl IS {
    pub fn new(settings: ISSettings, packet_handler: PacketHandler) -> IS {
        IS {
            settings,
            packet_handler,
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
                "user {} pass {} vers {} {}{}",
                self.settings.callsign,
                self.settings.passcode,
                name,
                version,
                if self.settings.filter == "" {
                    "".to_string()
                } else {
                    format!(" filter {}", self.settings.filter)
                }
            )
        };

        info!("Logging on to APRS-IS server");
        trace!("Login message: {}", login_message);
        writer.send(login_message).await?;

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                info!("Sending keep alive message to APRS-IS server");
                writer.send("# keep alive").await.unwrap();
            }
        });

        while let Some(packet) = reader.next().await {
            match packet {
                Ok(mut packet) => {
                    if packet.len() <= 2 {
                        continue;
                    }
                    packet.truncate(packet.len() - 2);
                    if packet[0] == b'#' {
                        match String::from_utf8(packet.to_vec()) {
                            Ok(server_message) => {
                                trace!("Received server response: {}", server_message);
                                if server_message.contains("unverified") {
                                    info!("User not verified on APRS-IS server");
                                    continue;
                                }
                                if server_message.contains(" verified") {
                                    info!("User verified on APRS-IS server");
                                }
                            }
                            Err(err) => warn!("Error processing server response: {}", err),
                        }
                    } else {
                        trace!("{:?}", packet);
                        (self.packet_handler)(APRSPacket {
                            raw: packet.to_vec(),
                        });
                    }
                }
                Err(err) => {
                    warn!("Error processing packet from APRS-IS server: {}", err);
                }
            }
        }

        Ok(())
    }
}
