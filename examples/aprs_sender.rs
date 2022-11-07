extern crate pretty_env_logger;

use aprs_parser::{AprsData, AprsMessage, AprsPacket, Callsign};
use std::convert::TryFrom;
use std::env;

use simple_aprs::*;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let args = arguments::parse(env::args()).unwrap();

    let callsign = args.get::<String>("callsign").unwrap();
    let passcode = args.get::<String>("passcode").unwrap();
    let to = args.get::<String>("to").unwrap();
    let msg = args.get::<String>("message").unwrap();

    let settings = ISSettings::new(
        "euro.aprs2.net".to_string(),
        14580,
        callsign.to_string(),
        passcode.to_string(),
        "".to_string(),
    );

    let packet = AprsPacket {
        from: Callsign::try_from(callsign.as_bytes()).unwrap(),
        to: Callsign::try_from(to.as_bytes()).unwrap(),
        via: vec![Callsign::new("TCPIP*", None)],
        data: AprsData::Message(AprsMessage {
            addressee: to.as_bytes().to_vec(),
            text: msg.as_bytes().to_vec(),
            id: None,
        }),
    };

    let mut aprs_is = ISConnection::connect(&settings)
        .await
        .expect("An error occurred while connecting");
    aprs_is.send(&packet).await.unwrap();
}
