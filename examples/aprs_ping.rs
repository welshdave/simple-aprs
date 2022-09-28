/// Looks for packets from a specified callsign.
/// When we see one, respond with the specified message
extern crate pretty_env_logger;

use aprs_parser::{AprsData, AprsMessage, AprsPacket, Callsign};
use futures::stream::StreamExt;
use std::convert::TryFrom;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use simple_aprs::*;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Don't send more than one packet per 5 minutes
    let min_duration = Duration::from_secs(300);

    let args = arguments::parse(env::args()).unwrap();

    let callsign = args.get::<String>("callsign").unwrap();
    let from_callsign = args.get::<String>("them").unwrap();
    let passcode = args.get::<String>("passcode").unwrap();
    let msg = args.get::<String>("message").unwrap();

    let us = Callsign::try_from(callsign.as_bytes()).unwrap();
    let them = Callsign::try_from(from_callsign.as_bytes()).unwrap();

    let settings = ISSettings::new(
        "euro.aprs2.net".to_string(),
        14580,
        callsign.to_string(),
        passcode.to_string(),
        format!("b/{}", from_callsign),
    );

    let (mut r, w) = ISConnection::connect(&settings)
        .await
        .expect("An error occurred while connecting")
        .split();

    let last_ping = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(3600)));
    r.stream()
        .for_each(|x| async {
            if let Ok(Ok(pkt)) = x.map(|y| y.parsed()) {
                if pkt.from == them {
                    println!("Received packet: {:?}", pkt);
                    // packet is from them

                    if Instant::now() > *last_ping.lock().await + min_duration {
                        let resp = AprsPacket {
                            from: us.clone(),
                            to: them.clone(),
                            via: vec![Callsign::new("TCPIP*", None)],
                            data: AprsData::Message(AprsMessage {
                                addressee: from_callsign.as_bytes().to_vec(),
                                text: msg.as_bytes().to_vec(),
                                id: None,
                            }),
                        };

                        w.clone().send(&resp).await.unwrap();
                        *last_ping.lock().await = Instant::now();
                        println!("Sent packet: {:?}", resp);
                    }
                }
            }
        })
        .await;
}
