use ser::{Reader, Stream};
use std::io;
use {MessageResult, Payload};

#[derive(Debug, PartialEq)]
pub struct Pong {
    pub nonce: u64,
}

impl Pong {
    pub fn new(nonce: u64) -> Self {
        Pong { nonce: nonce }
    }
}

impl Payload for Pong {
    fn version() -> u32 {
        0
    }

    fn command() -> &'static str {
        "pong"
    }

    fn deserialize_payload<T>(reader: &mut Reader<T>, _version: u32) -> MessageResult<Self>
    where
        T: io::Read,
    {
        let pong = Pong {
            nonce: try!(reader.read()),
        };

        Ok(pong)
    }

    fn serialize_payload(&self, stream: &mut Stream, _version: u32) -> MessageResult<()> {
        stream.append(&self.nonce);
        Ok(())
    }
}
