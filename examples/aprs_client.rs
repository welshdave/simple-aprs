use std::env;

use simple_aprs::*;

fn aprs_message_handler(message: APRSMessage) {
    println!("{}", message.raw);
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

    let mut aprs_is = IS::new(settings);

    aprs_is.register_message_handler(aprs_message_handler);

    aprs_is.connect();
}
