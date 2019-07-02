use ser::{Reader, Stream};
use std::io;
use zebra_chain::Transaction;
use {MessageResult, Payload};

#[derive(Debug, PartialEq)]
pub struct Tx {
    pub transaction: Transaction,
}

impl Tx {
    pub fn with_transaction(transaction: Transaction) -> Self {
        Tx {
            transaction: transaction,
        }
    }
}

impl Payload for Tx {
    fn version() -> u32 {
        0
    }

    fn command() -> &'static str {
        "tx"
    }

    fn deserialize_payload<T>(reader: &mut Reader<T>, _version: u32) -> MessageResult<Self>
    where
        T: io::Read,
    {
        let tx = Tx {
            transaction: try!(reader.read()),
        };

        Ok(tx)
    }

    fn serialize_payload(&self, stream: &mut Stream, _version: u32) -> MessageResult<()> {
        stream.append(&self.transaction);
        Ok(())
    }
}
