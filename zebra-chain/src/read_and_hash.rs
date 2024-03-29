use hash::H256;
use ser::{Deserializable, Error as ReaderError, Reader};
use std::io;
use zebra_crypto::{DHash256, Digest};

pub struct HashedData<T> {
    pub size: usize,
    pub hash: H256,
    pub data: T,
}

pub trait ReadAndHash {
    fn read_and_hash<T>(&mut self) -> Result<HashedData<T>, ReaderError>
    where
        T: Deserializable;
}

impl<R> ReadAndHash for Reader<R>
where
    R: io::Read,
{
    fn read_and_hash<T>(&mut self) -> Result<HashedData<T>, ReaderError>
    where
        T: Deserializable,
    {
        let mut size = 0usize;
        let mut hasher = DHash256::new();
        let data = self.read_with_proxy(|bytes| {
            size += bytes.len();
            hasher.input(bytes);
        })?;

        let result = HashedData {
            hash: hasher.finish(),
            data: data,
            size: size,
        };

        Ok(result)
    }
}
