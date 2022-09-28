use std::{io, string};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ISReadError {
    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error("Server command was not valid UTF8")]
    ServerCommandInvalidUtf8(#[from] string::FromUtf8Error),
}
