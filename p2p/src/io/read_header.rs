use futures::{Async, Future, Poll};
use std::io;
use tokio_io::io::{read_exact, ReadExact};
use tokio_io::AsyncRead;
use zebra_message::{MessageHeader, MessageResult};
use zebra_network::Magic;

pub fn read_header<A>(a: A, magic: Magic) -> ReadHeader<A>
where
    A: AsyncRead,
{
    ReadHeader {
        reader: read_exact(a, [0u8; 24]),
        magic: magic,
    }
}

pub struct ReadHeader<A> {
    reader: ReadExact<A, [u8; 24]>,
    magic: Magic,
}

impl<A> Future for ReadHeader<A>
where
    A: AsyncRead,
{
    type Item = (A, MessageResult<MessageHeader>);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (read, data) = try_ready!(self.reader.poll());
        let header = MessageHeader::deserialize(&data, self.magic);
        Ok(Async::Ready((read, header)))
    }
}

#[cfg(test)]
mod tests {
    use super::read_header;
    use bytes::Bytes;
    use futures::Future;
    use zebra_message::{Error, MessageHeader};
    use zebra_network::Network;

    #[test]
    fn test_read_header() {
        let raw: Bytes = "24e927646164647200000000000000001f000000ed52399b".into();
        let expected = MessageHeader {
            magic: Network::Mainnet.magic(),
            command: "addr".into(),
            len: 0x1f,
            checksum: "ed52399b".into(),
        };

        assert_eq!(
            read_header(raw.as_ref(), Network::Mainnet.magic())
                .wait()
                .unwrap()
                .1,
            Ok(expected)
        );
        assert_eq!(
            read_header(raw.as_ref(), Network::Testnet.magic())
                .wait()
                .unwrap()
                .1,
            Err(Error::InvalidMagic)
        );
    }

    #[test]
    fn test_read_header_with_invalid_magic() {
        let raw: Bytes = "f9beb4d86164647200000000000000001f000000ed52399b".into();
        assert_eq!(
            read_header(raw.as_ref(), Network::Testnet.magic())
                .wait()
                .unwrap()
                .1,
            Err(Error::InvalidMagic)
        );
    }

    #[test]
    fn test_read_too_short_header() {
        let raw: Bytes = "24e927646164647200000000000000001f000000ed5239".into();
        assert!(read_header(raw.as_ref(), Network::Mainnet.magic())
            .wait()
            .is_err());
    }
}
