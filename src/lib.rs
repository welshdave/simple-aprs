mod codec;
mod error;

use aprs_parser::{AprsError, AprsPacket};
use async_stream::try_stream;
use futures::sink::SinkExt;
use futures::{Stream, StreamExt};
use log::{info, trace, warn};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_util::codec::{FramedRead, FramedWrite};

type Reader = FramedRead<OwnedReadHalf, codec::ByteLinesCodec>;
type Writer = FramedWrite<OwnedWriteHalf, codec::ByteLinesCodec>;

pub struct RawPacket {
    pub raw: Vec<u8>,
}

impl RawPacket {
    pub fn parsed(&self) -> Result<AprsPacket, AprsError> {
        aprs_parser::parse(&self.raw)
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

pub struct ISConnection {
    reader: Reader,
    writer: Arc<Mutex<Writer>>,
    heartbeat_handler: JoinHandle<()>,
}

impl ISConnection {
    pub async fn connect(settings: &ISSettings) -> Result<Self, Box<dyn Error>> {
        let (mut writer, reader) = Self::init_connect(settings).await?;
        Self::login(settings, &mut writer).await?;
        let writer = Arc::new(Mutex::new(writer));
        let heartbeat_handler = Self::heartbeat(writer.clone()).await;

        Ok(Self {
            reader,
            writer,
            heartbeat_handler,
        })
    }

    pub fn stream(&mut self) -> impl Stream<Item = Result<RawPacket, error::ISReadError>> + '_ {
        try_stream! {
            while let Some(packet) = self.reader.next().await {
                let packet = packet?;
                if packet[0] == b'#' {
                    let server_message = String::from_utf8(packet.to_vec())?;
                    trace!("Received server response: {}", server_message);
                    if server_message.contains("unverified") {
                        info!("User not verified on APRS-IS server");
                        continue;
                    }
                    if server_message.contains(" verified") {
                        info!("User verified on APRS-IS server");
                    }
                } else {
                    trace!("{:?}", packet);
                    yield RawPacket {
                        raw: packet.to_vec(),
                    };
                }
            }
        }
    }

    async fn init_connect(settings: &ISSettings) -> Result<(Writer, Reader), Box<dyn Error>> {
        let address = format!("{}:{}", settings.host, settings.port);

        let stream = TcpStream::connect(address).await?;

        let (r, w) = stream.into_split();

        let writer = FramedWrite::new(w, codec::ByteLinesCodec::new());
        let reader = FramedRead::new(r, codec::ByteLinesCodec::new());

        Ok((writer, reader))
    }

    async fn login(settings: &ISSettings, writer: &mut Writer) -> Result<(), Box<dyn Error>> {
        let login_message = {
            let name = option_env!("CARGO_PKG_NAME").unwrap_or("unknown");
            let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");

            format!(
                "user {} pass {} vers {} {}{}",
                settings.callsign,
                settings.passcode,
                name,
                version,
                if settings.filter.is_empty() {
                    "".to_string()
                } else {
                    format!(" filter {}", settings.filter)
                }
            )
        };

        info!("Logging on to APRS-IS server");
        trace!("Login message: {}", login_message);
        writer.send(login_message.as_bytes()).await?;

        Ok(())
    }

    // TODO need a way for this to terminate
    async fn heartbeat(writer: Arc<Mutex<Writer>>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                info!("Sending keep alive message to APRS-IS server");
                writer
                    .lock()
                    .await
                    .send("# keep alive".as_bytes())
                    .await
                    .unwrap();
            }
        })
    }
}

impl Drop for ISConnection {
    fn drop(&mut self) {
        self.heartbeat_handler.abort();
    }
}
