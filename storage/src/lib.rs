extern crate bit_vec;
extern crate elastic_array;
extern crate lru_cache;
extern crate parking_lot;
#[macro_use]
extern crate display_derive;

extern crate bitcrypto as crypto;
extern crate chain;
extern crate primitives;
extern crate serialization as ser;
#[macro_use]
extern crate lazy_static;
extern crate network;

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

pub use primitives::{bytes, hash};

pub use crate::best_block::BestBlock;
pub use crate::block_ancestors::BlockAncestors;
pub use crate::block_chain::{BlockChain, ForkChain, Forkable};
pub use crate::block_iterator::BlockIterator;
pub use crate::block_origin::{BlockOrigin, SideChainOrigin};
pub use crate::block_provider::{BlockHeaderProvider, BlockProvider};
pub use crate::block_ref::BlockRef;
pub use crate::duplex_store::{DuplexTransactionOutputProvider, NoopStore};
pub use crate::error::Error;
pub use crate::nullifier_tracker::NullifierTracker;
pub use crate::store::{AsSubstore, CanonStore, SharedStore, Store};
pub use crate::transaction_meta::TransactionMeta;
pub use crate::transaction_provider::{
    CachedTransactionOutputProvider, TransactionMetaProvider, TransactionOutputProvider,
    TransactionProvider,
};
pub use crate::tree_state::{
    Dim as TreeDim, SaplingTreeState, SproutTreeState, TreeState, H32 as H32TreeDim,
};
pub use crate::tree_state_provider::TreeStateProvider;

use crate::hash::H256;

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
