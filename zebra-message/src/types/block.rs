use ser::{Reader, Stream};
use std::io;
use zebra_chain::Block as ChainBlock;
use {MessageResult, Payload};

#[derive(Debug, PartialEq)]
pub struct Block {
    pub block: ChainBlock,
}

impl Block {
    pub fn with_block(block: ChainBlock) -> Self {
        Block { block: block }
    }
}

impl Payload for Block {
    fn version() -> u32 {
        0
    }

    fn command() -> &'static str {
        "block"
    }

    fn deserialize_payload<T>(reader: &mut Reader<T>, _version: u32) -> MessageResult<Self>
    where
        T: io::Read,
    {
        let tx = Block {
            block: try!(reader.read()),
        };

        Ok(tx)
    }

    fn serialize_payload(&self, stream: &mut Stream, _version: u32) -> MessageResult<()> {
        stream.append(&self.block);
        Ok(())
    }
}
