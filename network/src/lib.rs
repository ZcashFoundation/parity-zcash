#[macro_use]
extern crate lazy_static;

extern crate rustc_hex as hex;
extern crate zebra_chain;
extern crate zebra_crypto;
extern crate zebra_keys;
extern crate zebra_primitives;
extern crate zebra_serialization;

mod consensus;
mod deployments;
mod network;

pub use zebra_primitives::{compact, hash};

pub use consensus::ConsensusParams;
pub use deployments::Deployment;
pub use network::{Magic, Network};
