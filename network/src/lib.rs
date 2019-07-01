#[macro_use]
extern crate lazy_static;

extern crate bitcrypto as crypto;
extern crate chain;
extern crate keys;
extern crate primitives;
extern crate rustc_hex as hex;
extern crate serialization;

mod consensus;
mod deployments;
mod network;

pub use primitives::{compact, hash};

pub use crate::consensus::ConsensusParams;
pub use crate::deployments::Deployment;
pub use crate::network::{Magic, Network};
