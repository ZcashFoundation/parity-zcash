use crate::bytes::{Bytes, TaggedBytes};
use crate::common::Command;
use network::Magic;
use crate::ser::Stream;
use crate::serialization::serialize_payload;
use crate::{MessageHeader, MessageResult, Payload};

pub fn to_raw_message(magic: Magic, command: Command, payload: &Bytes) -> Bytes {
    let header = MessageHeader::for_data(magic, command, payload);
    let mut stream = Stream::default();
    stream.append(&header);
    stream.append_slice(payload);
    stream.out()
}

pub struct Message<T> {
    bytes: TaggedBytes<T>,
}

impl<T> Message<T>
where
    T: Payload,
{
    pub fn new(magic: Magic, version: u32, payload: &T) -> MessageResult<Self> {
        let serialized = r#try!(serialize_payload(payload, version));

        let message = Message {
            bytes: TaggedBytes::new(to_raw_message(magic, T::command().into(), &serialized)),
        };

        Ok(message)
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

impl<T> AsRef<[u8]> for Message<T> {
    fn as_ref(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl<T> From<Message<T>> for Bytes {
    fn from(m: Message<T>) -> Self {
        m.bytes.into_raw()
    }
}
