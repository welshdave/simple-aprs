use std::env;

use simple_aprs::*;

fn aprs_message_handler(message: APRSMessage) {
    match String::from_utf8(message.raw) {
        Ok(msg) => {
            println!("{:?}", msg);
        }
        Err(err) => {
            println!("Error converting APRS packet to UTF8: {}", err);
        }
    }
}

fn main() {
    let args = arguments::parse(env::args()).unwrap();

    let callsign = args.get::<String>("callsign").unwrap();
    let passcode = args.get::<String>("passcode").unwrap();

    let settings = ISSettings::new(
        "euro.aprs2.net".to_string(),
        14580,
        callsign.to_string(),
        passcode.to_string(),
        "filter r/55/-4/600".to_string(),
    );

    let aprs_is = IS::new(settings, aprs_message_handler);

    aprs_is.connect();
}
