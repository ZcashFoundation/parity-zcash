extern crate byteorder;
extern crate primitives;
extern crate rustc_hex as hex;

mod compact_integer;
mod fixed_array;
mod impls;
mod list;
mod reader;
mod stream;

pub use crate::compact_integer::CompactInteger;
pub use crate::list::List;
pub use primitives::{bytes, compact, hash};
pub use crate::reader::{deserialize, deserialize_iterator, Deserializable, Error, ReadIterator, Reader};
pub use crate::stream::{serialize, serialize_list, serialized_list_size, Serializable, Stream};
