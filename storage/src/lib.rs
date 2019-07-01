extern crate bit_vec;
extern crate elastic_array;
extern crate lru_cache;
extern crate parking_lot;
#[macro_use]
extern crate display_derive;

extern crate zebra_chain;
extern crate zebra_crypto;
extern crate zebra_primitives;
extern crate zebra_serialization as ser;
#[macro_use]
extern crate lazy_static;
extern crate zebra_network;

mod best_block;
mod block_ancestors;
mod block_chain;
mod block_impls;
mod block_iterator;
mod block_origin;
mod block_provider;
mod block_ref;
mod duplex_store;
mod error;
mod nullifier_tracker;
mod store;
mod transaction_meta;
mod transaction_provider;
mod tree_state;
mod tree_state_provider;

pub use zebra_primitives::{bytes, hash};

pub use best_block::BestBlock;
pub use block_ancestors::BlockAncestors;
pub use block_chain::{BlockChain, ForkChain, Forkable};
pub use block_iterator::BlockIterator;
pub use block_origin::{BlockOrigin, SideChainOrigin};
pub use block_provider::{BlockHeaderProvider, BlockProvider};
pub use block_ref::BlockRef;
pub use duplex_store::{DuplexTransactionOutputProvider, NoopStore};
pub use error::Error;
pub use nullifier_tracker::NullifierTracker;
pub use store::{AsSubstore, CanonStore, SharedStore, Store};
pub use transaction_meta::TransactionMeta;
pub use transaction_provider::{
    CachedTransactionOutputProvider, TransactionMetaProvider, TransactionOutputProvider,
    TransactionProvider,
};
pub use tree_state::{
    Dim as TreeDim, SaplingTreeState, SproutTreeState, TreeState, H32 as H32TreeDim,
};
pub use tree_state_provider::TreeStateProvider;

use hash::H256;

/// Epoch tag.
///
/// Sprout and Sapling nullifiers/commitments are considered disjoint,
/// even if they have the same bit pattern.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EpochTag {
    /// Sprout epoch.
    Sprout,
    /// Sapling epoch.
    Sapling,
}

/// H256-reference to some object that is valid within single epoch (nullifiers, commitment trees, ...).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpochRef {
    epoch: EpochTag,
    hash: H256,
}

impl EpochRef {
    /// New reference.
    pub fn new(epoch: EpochTag, hash: H256) -> Self {
        EpochRef {
            epoch: epoch,
            hash: hash,
        }
    }

    /// Epoch tag
    pub fn epoch(&self) -> EpochTag {
        self.epoch
    }

    /// Hash reference
    pub fn hash(&self) -> &H256 {
        &self.hash
    }
}

impl From<(EpochTag, H256)> for EpochRef {
    fn from(tuple: (EpochTag, H256)) -> Self {
        EpochRef {
            epoch: tuple.0,
            hash: tuple.1,
        }
    }
}
