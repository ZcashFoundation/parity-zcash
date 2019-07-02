use super::bytes::Bytes;
use super::hash::H256;
use serde::de::{Deserialize, Deserializer, Unexpected};
use std::fmt;
use std::str::FromStr;

/// Hex-encoded block
pub type RawBlock = Bytes;

/// Block reference
#[derive(Debug)]
pub enum BlockRef {
    /// References block by its number
    Number(u32),
    /// References block by its hash
    Hash(H256),
}

impl<'a> Deserialize<'a> for BlockRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        use serde::de::Visitor;

        struct BlockRefVisitor;

        impl<'b> Visitor<'b> for BlockRefVisitor {
            type Value = BlockRef;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("either block number of hash")
            }

            fn visit_str<E>(self, value: &str) -> Result<BlockRef, E>
            where
                E: ::serde::de::Error,
            {
                if value.len() == 64 {
                    H256::from_str(value)
                        .map(BlockRef::Hash)
                        .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
                } else {
                    u32::from_str(value)
                        .map(BlockRef::Number)
                        .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
                }
            }
        }

        deserializer.deserialize_identifier(BlockRefVisitor)
    }
}
