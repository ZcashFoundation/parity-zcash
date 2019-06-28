use bytes::Bytes;
use ser::{Reader, Stream};
use std::io;
use {MessageResult, Payload};

pub const FILTERADD_MAX_DATA_LEN: usize = 520;

#[derive(Debug, PartialEq)]
pub struct FilterAdd {
    // TODO: check how this should be serialized
    pub data: Bytes,
}

impl Payload for FilterAdd {
    fn version() -> u32 {
        70001
    }

    fn command() -> &'static str {
        "filteradd"
    }

    fn deserialize_payload<T>(reader: &mut Reader<T>, _version: u32) -> MessageResult<Self>
    where
        T: io::Read,
    {
        let filteradd = FilterAdd {
            data: try!(reader.read()),
        };

        Ok(filteradd)
    }

    fn serialize_payload(&self, stream: &mut Stream, _version: u32) -> MessageResult<()> {
        stream.append(&self.data);
        Ok(())
    }
}
