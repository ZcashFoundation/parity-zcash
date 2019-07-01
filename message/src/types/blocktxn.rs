use crate::common::BlockTransactions;
use crate::ser::{Reader, Stream};
use std::io;
use crate::{MessageResult, Payload};

#[derive(Debug, PartialEq)]
pub struct BlockTxn {
    pub request: BlockTransactions,
}

impl Payload for BlockTxn {
    fn version() -> u32 {
        70014
    }

    fn command() -> &'static str {
        "blocktxn"
    }

    fn deserialize_payload<T>(reader: &mut Reader<T>, _version: u32) -> MessageResult<Self>
    where
        T: io::Read,
    {
        let block = BlockTxn {
            request: r#try!(reader.read()),
        };

        Ok(block)
    }

    fn serialize_payload(&self, stream: &mut Stream, _version: u32) -> MessageResult<()> {
        stream.append(&self.request);
        Ok(())
    }
}
