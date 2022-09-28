extern crate pretty_env_logger;

use futures::stream::StreamExt;
use std::env;

use simple_aprs::*;

async fn aprs_packet_handler(packet: RawPacket) {
    match packet.parsed() {
        Ok(parsed) => {
            println!("Source: {}", parsed.from);
            println!("Destination: {}", parsed.to);
        }
        Err(err) => {
            println!("Error parsing packet: {}", err);
            match String::from_utf8(packet.raw) {
                Ok(msg) => println!("{:?}", msg),
                Err(err) => println!("Error converting APRS packet to UTF8: {}", err),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let args = arguments::parse(env::args()).unwrap();

    let callsign = args.get::<String>("callsign").unwrap();
    let passcode = args.get::<String>("passcode").unwrap();

    let settings = ISSettings::new(
        "euro.aprs2.net".to_string(),
        14580,
        callsign.to_string(),
        passcode.to_string(),
        "r/55/-4/600".to_string(),
    );

    let mut aprs_is = ISConnection::connect(&settings)
        .await
        .expect("An error occurred while connecting");

    aprs_is
        .stream()
        .for_each(|x| aprs_packet_handler(x.expect("Error occurred while receiving packets")))
        .await;
}
