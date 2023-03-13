mod abort;
mod codec;
mod error;

use aprs_parser::{AprsPacket, DecodeError};
use async_stream::try_stream;
use futures::sink::SinkExt;
use futures::{Stream, StreamExt};
use log::{info, trace};
use std::io;
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

// how many seconds to wait for a line from the APRS-IS server
// used to detect a connection that has hanged
const HEARTBEAT_TIME: u64 = 60;

type Reader = FramedRead<OwnedReadHalf, codec::ByteLinesCodec>;
type Writer = FramedWrite<OwnedWriteHalf, codec::ByteLinesCodec>;

pub struct RawPacket {
    pub raw: Vec<u8>,
}

impl RawPacket {
    pub fn parsed(&self) -> Result<AprsPacket, DecodeError> {
        AprsPacket::decode_textual(&self.raw)
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

pub struct ISReader {
    reader: Reader,

    // Used to kill the heartbeat task
    // Once reader + writer are out of scope
    #[allow(dead_code)]
    handler: Arc<Mutex<abort::AbortOnDrop<()>>>,
}

impl ISReader {
    pub fn stream(&mut self) -> impl Stream<Item = Result<RawPacket, error::ISReadError>> + '_ {
        try_stream! {
            while let Some(packet) = tokio::time::timeout(Duration::from_secs(HEARTBEAT_TIME), self.reader.next()).await? {
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
}

#[derive(Clone)]
pub struct ISWriter {
    writer: Arc<Mutex<Writer>>,

    // Used to kill the heartbeat task
    // Once reader + writer are out of scope
    #[allow(dead_code)]
    handler: Arc<Mutex<abort::AbortOnDrop<()>>>,
}

impl ISWriter {
    pub async fn send(&mut self, packet: &AprsPacket) -> Result<(), error::ISSendError> {
        let mut buf = vec![];
        packet.encode_textual(&mut buf)?;

        self.writer.lock().await.send(&buf).await?;

        Ok(())
    }
}

pub struct ISConnection {
    reader: ISReader,
    writer: ISWriter,
}

impl ISConnection {
    pub async fn connect(settings: &ISSettings) -> Result<Self, io::Error> {
        let (reader, mut writer) = Self::init_connect(settings).await?;
        Self::login(settings, &mut writer).await?;

        let writer = Arc::new(Mutex::new(writer));
        let handler = Arc::new(Mutex::new(abort::AbortOnDrop::new(
            Self::heartbeat(writer.clone()).await,
        )));

        let reader = ISReader {
            reader,
            handler: handler.clone(),
        };
        let writer = ISWriter { writer, handler };

        Ok(Self { reader, writer })
    }

    pub fn stream(&mut self) -> impl Stream<Item = Result<RawPacket, error::ISReadError>> + '_ {
        self.reader.stream()
    }

    pub async fn send(&mut self, packet: &AprsPacket) -> Result<(), error::ISSendError> {
        self.writer.send(packet).await
    }

    pub fn split(self) -> (ISReader, ISWriter) {
        (self.reader, self.writer)
    }

    async fn init_connect(settings: &ISSettings) -> io::Result<(Reader, Writer)> {
        let address = format!("{}:{}", settings.host, settings.port);

        let stream = TcpStream::connect(address).await?;

        let (r, w) = stream.into_split();

        let writer = FramedWrite::new(w, codec::ByteLinesCodec::new());
        let reader = FramedRead::new(r, codec::ByteLinesCodec::new());

        Ok((reader, writer))
    }

    async fn login(settings: &ISSettings, writer: &mut Writer) -> io::Result<()> {
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

    async fn heartbeat(writer: Arc<Mutex<Writer>>) -> JoinHandle<()> {
        // Automatically terminates once the reader and writer are dropped
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                info!("Sending keep alive message to APRS-IS server");
                if writer
                    .lock()
                    .await
                    .send("# keep alive".as_bytes())
                    .await
                    .is_err()
                {
                    break;
                }
            }
        })
    }
}
