use std::{io, usize};

use bytes::{BufMut, BytesMut};

use tokio_util::codec::{Decoder, Encoder};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ByteLinesCodec {
    next_index: usize,
}

impl ByteLinesCodec {
    pub fn new() -> ByteLinesCodec {
        ByteLinesCodec { next_index: 0 }
    }
}

fn without_carriage_return(b: &mut BytesMut) {
    if let Some(&b'\r') = b.last() {
        b.truncate(b.len() - 1);
    }
}

impl Decoder for ByteLinesCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<BytesMut>, io::Error> {
        let read_to = buf.len();

        let newline_offset = buf[self.next_index..read_to]
            .iter()
            .position(|b| *b == b'\n');

        match newline_offset {
            Some(offset) => {
                let newline_index = offset + self.next_index;
                self.next_index = 0;
                let mut line = buf.split_to(newline_index + 1);
                line.truncate(line.len() - 1);
                without_carriage_return(&mut line);
                Ok(Some(line))
            }
            None => {
                self.next_index = read_to;
                Ok(None)
            }
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<BytesMut>, io::Error> {
        Ok(match self.decode(buf)? {
            Some(frame) => Some(frame),
            None => {
                if buf.is_empty() || buf == &b"\r"[..] {
                    None
                } else {
                    let mut line = buf.split_to(buf.len());
                    without_carriage_return(&mut line);
                    self.next_index = 0;
                    Some(line)
                }
            }
        })
    }
}

impl<T> Encoder<T> for ByteLinesCodec
where
    T: AsRef<[u8]>,
{
    type Error = io::Error;

    fn encode(&mut self, line: T, buf: &mut BytesMut) -> Result<(), io::Error> {
        let line = line.as_ref();
        buf.reserve(line.len() + 1);
        buf.put(line);
        buf.put_u8(b'\n');
        Ok(())
    }
}

impl Default for ByteLinesCodec {
    fn default() -> Self {
        Self::new()
    }
}
