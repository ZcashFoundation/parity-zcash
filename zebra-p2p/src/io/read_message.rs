use futures::{Async, Future, Poll};
use io::{read_header, read_payload, ReadHeader, ReadPayload};
use std::io;
use std::marker::PhantomData;
use tokio_io::AsyncRead;
use zebra_message::{Error, MessageResult, Payload};
use zebra_network::Magic;

pub fn read_message<M, A>(a: A, magic: Magic, version: u32) -> ReadMessage<M, A>
where
    A: AsyncRead,
    M: Payload,
{
    ReadMessage {
        state: ReadMessageState::ReadHeader {
            version: version,
            future: read_header(a, magic),
        },
        message_type: PhantomData,
    }
}

enum ReadMessageState<M, A> {
    ReadHeader { version: u32, future: ReadHeader<A> },
    ReadPayload { future: ReadPayload<M, A> },
}

pub struct ReadMessage<M, A> {
    state: ReadMessageState<M, A>,
    message_type: PhantomData<M>,
}

impl<M, A> Future for ReadMessage<M, A>
where
    A: AsyncRead,
    M: Payload,
{
    type Item = (A, MessageResult<M>);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                ReadMessageState::ReadHeader {
                    version,
                    ref mut future,
                } => {
                    let (read, header) = try_ready!(future.poll());
                    let header = match header {
                        Ok(header) => header,
                        Err(err) => return Ok((read, Err(err)).into()),
                    };

                    if header.command != M::command() {
                        return Ok((read, Err(Error::InvalidCommand)).into());
                    }
                    let future = read_payload(read, version, header.len as usize, header.checksum);
                    ReadMessageState::ReadPayload { future: future }
                }
                ReadMessageState::ReadPayload { ref mut future } => {
                    let (read, payload) = try_ready!(future.poll());
                    return Ok(Async::Ready((read, payload)));
                }
            };
            self.state = next_state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::read_message;
    use bytes::Bytes;
    use futures::Future;
    use zebra_message::types::{Ping, Pong};
    use zebra_message::Error;
    use zebra_network::Network;

    #[test]
    fn test_read_message() {
        let raw: Bytes = "24e9276470696e6700000000000000000800000083c00c765845303b6da97786".into();
        let ping = Ping::new(u64::from_str_radix("8677a96d3b304558", 16).unwrap());
        assert_eq!(
            read_message(raw.as_ref(), Network::Mainnet.magic(), 0)
                .wait()
                .unwrap()
                .1,
            Ok(ping)
        );
        assert_eq!(
            read_message::<Ping, _>(raw.as_ref(), Network::Testnet.magic(), 0)
                .wait()
                .unwrap()
                .1,
            Err(Error::InvalidMagic)
        );
        assert_eq!(
            read_message::<Pong, _>(raw.as_ref(), Network::Mainnet.magic(), 0)
                .wait()
                .unwrap()
                .1,
            Err(Error::InvalidCommand)
        );
    }

    #[test]
    fn test_read_too_short_message() {
        let raw: Bytes = "24e9276470696e6700000000000000000800000083c00c765845303b6da977".into();
        assert!(
            read_message::<Ping, _>(raw.as_ref(), Network::Mainnet.magic(), 0)
                .wait()
                .is_err()
        );
    }

    #[test]
    fn test_read_message_with_invalid_checksum() {
        let raw: Bytes = "24e9276470696e6700000000000000000800000083c01c765845303b6da97786".into();
        assert_eq!(
            read_message::<Ping, _>(raw.as_ref(), Network::Mainnet.magic(), 0)
                .wait()
                .unwrap()
                .1,
            Err(Error::InvalidChecksum)
        );
    }
}
