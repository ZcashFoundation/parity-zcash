use hash::H256;
use ser::{Deserializable, Error as ReaderError, Reader, Serializable, Stream};
use std::io;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u32)]
pub enum InventoryType {
    Error = 0,
    MessageTx = 1,
    MessageBlock = 2,
    MessageFilteredBlock = 3,
}

impl InventoryType {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(InventoryType::Error),
            1 => Some(InventoryType::MessageTx),
            2 => Some(InventoryType::MessageBlock),
            3 => Some(InventoryType::MessageFilteredBlock),
            _ => None,
        }
    }
}

impl From<InventoryType> for u32 {
    fn from(i: InventoryType) -> Self {
        i as u32
    }
}

impl Serializable for InventoryType {
    fn serialize(&self, stream: &mut Stream) {
        stream.append(&u32::from(*self));
    }
}

impl Deserializable for InventoryType {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, ReaderError>
    where
        T: io::Read,
    {
        let t: u32 = try!(reader.read());
        InventoryType::from_u32(t).ok_or(ReaderError::MalformedData)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct InventoryVector {
    pub inv_type: InventoryType,
    pub hash: H256,
}

impl InventoryVector {
    pub fn tx(hash: H256) -> Self {
        InventoryVector {
            inv_type: InventoryType::MessageTx,
            hash: hash,
        }
    }

    pub fn block(hash: H256) -> Self {
        InventoryVector {
            inv_type: InventoryType::MessageBlock,
            hash: hash,
        }
    }
}

impl Serializable for InventoryVector {
    fn serialize(&self, stream: &mut Stream) {
        stream.append(&self.inv_type).append(&self.hash);
    }
}

impl Deserializable for InventoryVector {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, ReaderError>
    where
        T: io::Read,
    {
        let vec = InventoryVector {
            inv_type: try!(reader.read()),
            hash: try!(reader.read()),
        };

        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::{InventoryType, InventoryVector};
    use bytes::Bytes;
    use ser::{deserialize, serialize};

    #[test]
    fn test_inventory_serialize() {
        let expected =
            "020000000400000000000000000000000000000000000000000000000000000000000000".into();

        let inventory = InventoryVector {
            inv_type: InventoryType::MessageBlock,
            hash: 4u8.into(),
        };

        assert_eq!(serialize(&inventory), expected);
    }

    #[test]
    fn test_inventory_deserialize() {
        let raw: Bytes =
            "020000000400000000000000000000000000000000000000000000000000000000000000".into();

        let expected = InventoryVector {
            inv_type: InventoryType::MessageBlock,
            hash: 4u8.into(),
        };

        assert_eq!(expected, deserialize(raw.as_ref()).unwrap());
    }

    #[test]
    fn test_inventory_type_conversion() {
        assert_eq!(0u32, u32::from(InventoryType::Error));
        assert_eq!(1u32, u32::from(InventoryType::MessageTx));
        assert_eq!(2u32, u32::from(InventoryType::MessageBlock));
        assert_eq!(3u32, u32::from(InventoryType::MessageFilteredBlock));

        assert_eq!(InventoryType::from_u32(0).unwrap(), InventoryType::Error);
        assert_eq!(
            InventoryType::from_u32(1).unwrap(),
            InventoryType::MessageTx
        );
        assert_eq!(
            InventoryType::from_u32(2).unwrap(),
            InventoryType::MessageBlock
        );
        assert_eq!(
            InventoryType::from_u32(3).unwrap(),
            InventoryType::MessageFilteredBlock
        );
    }
}
