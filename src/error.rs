use aprs_parser::EncodeError;
use std::{io, string};
use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Error, Debug)]
pub enum ISReadError {
    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error("TCP read timeout - connection died")]
    TcpReadTimeout(#[from] Elapsed),

    #[error("Server command was not valid UTF8")]
    ServerCommandInvalidUtf8(#[from] string::FromUtf8Error),
}

#[derive(Error, Debug)]
pub enum ISSendError {
    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error("Could not convert APRS packet to byte array")]
    EncodeError(#[from] EncodeError),
}
