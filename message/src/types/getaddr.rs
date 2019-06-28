use ser::{Reader, Stream};
use std::io;
use {MessageResult, Payload};

#[derive(Debug, PartialEq)]
pub struct GetAddr;

impl Payload for GetAddr {
    fn version() -> u32 {
        0
    }

    fn command() -> &'static str {
        "getaddr"
    }

    fn deserialize_payload<T>(_reader: &mut Reader<T>, _version: u32) -> MessageResult<Self>
    where
        T: io::Read,
    {
        Ok(GetAddr)
    }

    fn serialize_payload(&self, _stream: &mut Stream, _version: u32) -> MessageResult<()> {
        Ok(())
    }
}
