use simple_aprs::*;

fn aprs_message_handler(message: APRSMessage) {
    println!("{}", message.raw);
}

fn main() {
    let settings = ISSettings::new(
        "aprs.glidernet.org".to_string(),
        10152,
        "test".to_string(),
        "-1".to_string(),
        "".to_string(),
    );

    let mut aprs_is = IS::new(settings);

    aprs_is.register_message_handler(aprs_message_handler);

    aprs_is.connect();
}
