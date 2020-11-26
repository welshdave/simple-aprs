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
    let settings = ISSettings::new(
        "aprs.glidernet.org".to_string(),
        10152,
        "test".to_string(),
        "-1".to_string(),
        "".to_string(),
    );

    let aprs_is = IS::new(settings, aprs_message_handler);

    aprs_is.connect();
}
