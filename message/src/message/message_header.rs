use common::Command;
use crypto::checksum;
use hash::H32;
use network::Magic;
use ser::{Reader, Serializable, Stream};
use Error;

#[derive(Debug, PartialEq)]
pub struct MessageHeader {
    pub magic: Magic,
    pub command: Command,
    pub len: u32,
    pub checksum: H32,
}

impl MessageHeader {
    pub fn for_data(magic: Magic, command: Command, data: &[u8]) -> Self {
        MessageHeader {
            magic: magic,
            command: command,
            len: data.len() as u32,
            checksum: checksum(data),
        }
    }
}

impl MessageHeader {
    pub fn deserialize(data: &[u8], expected: Magic) -> Result<Self, Error> {
        if data.len() != 24 {
            return Err(Error::Deserialize);
        }

        let mut reader = Reader::new(data);
        let magic: u32 = try!(reader.read());
        let magic = Magic::from(magic);
        if expected != magic {
            return Err(Error::InvalidMagic);
        }

        let header = MessageHeader {
            magic: magic,
            command: try!(reader.read()),
            len: try!(reader.read()),
            checksum: try!(reader.read()),
        };

        Ok(header)
    }
}

impl Serializable for MessageHeader {
    fn serialize(&self, stream: &mut Stream) {
        stream
            .append(&self.magic)
            .append(&self.command)
            .append(&self.len)
            .append(&self.checksum);
    }
}

#[cfg(test)]
mod tests {
    use super::MessageHeader;
    use bytes::Bytes;
    use network::Network;
    use ser::serialize;

    #[test]
    fn test_message_header_serialization() {
        let expected = "24e927646164647200000000000000001f000000ed52399b".into();
        let header = MessageHeader {
            magic: Network::Mainnet.magic(),
            command: "addr".into(),
            len: 0x1f,
            checksum: "ed52399b".into(),
        };

        assert_eq!(serialize(&header), expected);
    }

    #[test]
    fn test_message_header_deserialization() {
        let raw: Bytes = "24e927646164647200000000000000001f000000ed52399b".into();
        let expected = MessageHeader {
            magic: Network::Mainnet.magic(),
            command: "addr".into(),
            len: 0x1f,
            checksum: "ed52399b".into(),
        };

        assert_eq!(
            expected,
            MessageHeader::deserialize(&raw, Network::Mainnet.magic()).unwrap()
        );
    }
}
