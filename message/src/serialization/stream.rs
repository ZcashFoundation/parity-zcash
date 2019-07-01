use crate::bytes::Bytes;
use crate::ser::Stream;
use crate::{Error, MessageResult, Payload};

pub fn serialize_payload<T>(t: &T, version: u32) -> MessageResult<Bytes>
where
    T: Payload,
{
    let mut stream = PayloadStream::new(version);
    r#try!(stream.append(t));
    Ok(stream.out())
}

pub struct PayloadStream {
    stream: Stream,
    version: u32,
}

impl PayloadStream {
    pub fn new(version: u32) -> Self {
        PayloadStream {
            stream: Stream::new(),
            version: version,
        }
    }

    pub fn append<T>(&mut self, t: &T) -> MessageResult<()>
    where
        T: Payload,
    {
        if T::version() > self.version {
            return Err(Error::InvalidVersion);
        }

        t.serialize_payload(&mut self.stream, self.version)
    }

    pub fn out(self) -> Bytes {
        self.stream.out()
    }
}
