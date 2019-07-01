use chain::Transaction;
use crate::ser::{CompactInteger, Deserializable, Error as ReaderError, Reader, Serializable, Stream};
use std::io;

#[derive(Debug, PartialEq)]
pub struct PrefilledTransaction {
    pub index: usize,
    pub transaction: Transaction,
}

impl Serializable for PrefilledTransaction {
    fn serialize(&self, stream: &mut Stream) {
        stream
            .append(&CompactInteger::from(self.index))
            .append(&self.transaction);
    }
}

impl Deserializable for PrefilledTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, ReaderError>
    where
        T: io::Read,
    {
        let compact: CompactInteger = r#try!(reader.read());
        let tx = PrefilledTransaction {
            index: compact.into(),
            transaction: r#try!(reader.read()),
        };

        Ok(tx)
    }
}
