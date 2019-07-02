use ser::{Reader, Stream};
use std::io;
use {MessageResult, Payload};

#[derive(Debug, PartialEq)]
pub struct FilterClear;

impl Payload for FilterClear {
    fn version() -> u32 {
        70001
    }

    fn command() -> &'static str {
        "filterclear"
    }

    fn deserialize_payload<T>(_reader: &mut Reader<T>, _version: u32) -> MessageResult<Self>
    where
        T: io::Read,
    {
        Ok(FilterClear)
    }

    fn serialize_payload(&self, _stream: &mut Stream, _version: u32) -> MessageResult<()> {
        Ok(())
    }
}
