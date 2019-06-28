extern crate bitcrypto;
extern crate byteorder;
extern crate chain;
extern crate db;
extern crate storage;
#[macro_use]
extern crate log;
extern crate bit_vec;
extern crate futures;
extern crate keys;
extern crate linked_hash_map;
extern crate message;
extern crate miner;
extern crate murmur3;
extern crate network;
extern crate p2p;
extern crate parking_lot;
extern crate primitives;
extern crate rand;
extern crate script;
extern crate serialization as ser;
extern crate time;
extern crate verification;

mod blocks_writer;
mod inbound_connection;
mod inbound_connection_factory;
mod local_node;
mod synchronization_chain;
mod synchronization_client;
mod synchronization_client_core;
mod synchronization_executor;
mod synchronization_manager;
mod synchronization_peers;
mod synchronization_peers_tasks;
mod synchronization_server;
mod synchronization_verifier;
mod types;
mod utils;

pub use types::LocalNodeRef;
pub use types::PeersRef;

use network::{ConsensusParams, Network};
use parking_lot::RwLock;
use primitives::hash::H256;
use std::sync::Arc;
use verification::BackwardsCompatibleChainVerifier as ChainVerifier;

/// Sync errors.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Too many orphan blocks.
    TooManyOrphanBlocks,
    /// Database error.
    Database(storage::Error),
    /// Block verification error.
    Verification(String),
}

#[derive(Debug, Clone)]
/// Verification parameters.
pub struct VerificationParameters {
    /// Blocks verification level.
    pub verification_level: verification::VerificationLevel,
    /// Blocks verification edge: all blocks before this are validated using verification_level.
    /// All blocks after this (inclusive) are validated using VerificationLevel::Full level.
    pub verification_edge: H256,
}

/// Synchronization events listener
pub trait SyncListener: Send + 'static {
    /// Called when node switches to synchronization state
    fn synchronization_state_switched(&self, is_synchronizing: bool);
    /// Called when new best storage block is inserted
    fn best_storage_block_inserted(&self, block_hash: &H256);
}

/// Create blocks writer.
pub fn create_sync_blocks_writer(
    db: storage::SharedStore,
    consensus: ConsensusParams,
    verification_params: VerificationParameters,
) -> blocks_writer::BlocksWriter {
    blocks_writer::BlocksWriter::new(db, consensus, verification_params)
}

/// Create synchronization peers
pub fn create_sync_peers() -> PeersRef {
    use synchronization_peers::PeersImpl;

    Arc::new(PeersImpl::default())
}

/// Creates local sync node for given `db`
pub fn create_local_sync_node(
    consensus: ConsensusParams,
    db: storage::SharedStore,
    peers: PeersRef,
    verification_params: VerificationParameters,
) -> LocalNodeRef {
    use local_node::LocalNode as SyncNode;
    use miner::MemoryPool;
    use synchronization_chain::Chain as SyncChain;
    use synchronization_client::SynchronizationClient;
    use synchronization_client_core::{
        Config as SynchronizationConfig, CoreVerificationSink, SynchronizationClientCore,
    };
    use synchronization_executor::LocalSynchronizationTaskExecutor as SyncExecutor;
    use synchronization_server::ServerImpl;
    use synchronization_verifier::AsyncVerifier;
    use types::SynchronizationStateRef;
    use utils::SynchronizationState;

    let network = consensus.network;
    let sync_client_config = SynchronizationConfig {
        // during regtests, peer is providing us with bad blocks => we shouldn't close connection because of this
        close_connection_on_bad_block: network != Network::Regtest,
    };

    let memory_pool = Arc::new(RwLock::new(MemoryPool::new()));
    let sync_state = SynchronizationStateRef::new(SynchronizationState::with_storage(db.clone()));
    let sync_chain = SyncChain::new(db.clone(), memory_pool.clone());

    let light_chain_verifier = Arc::new(ChainVerifier::new(db.clone(), consensus.clone()));
    let heavy_chain_verifier = Arc::new(ChainVerifier::new(db.clone(), consensus.clone()));
    let sync_executor = SyncExecutor::new(peers.clone());
    let sync_server = Arc::new(ServerImpl::new(
        peers.clone(),
        db.clone(),
        memory_pool.clone(),
        sync_executor.clone(),
    ));
    let sync_client_core = SynchronizationClientCore::new(
        sync_client_config,
        sync_state.clone(),
        peers.clone(),
        sync_executor.clone(),
        sync_chain,
    );
    let verifier_sink = Arc::new(CoreVerificationSink::new(sync_client_core.clone()));
    let light_verifier = AsyncVerifier::new(
        "Light verification".into(),
        light_chain_verifier,
        db.clone(),
        memory_pool.clone(),
        verifier_sink.clone(),
        verification_params.clone(),
    );
    let heavy_verifier = AsyncVerifier::new(
        "Heavy verification".into(),
        heavy_chain_verifier,
        db.clone(),
        memory_pool.clone(),
        verifier_sink,
        verification_params,
    );
    let sync_client = SynchronizationClient::new(
        sync_state.clone(),
        sync_client_core,
        light_verifier,
        heavy_verifier,
    );
    Arc::new(SyncNode::new(
        consensus,
        db,
        memory_pool,
        peers,
        sync_state,
        sync_client,
        sync_server,
    ))
}

/// Create inbound synchronization connections factory for given local sync node.
pub fn create_sync_connection_factory(
    peers: PeersRef,
    local_sync_node: LocalNodeRef,
) -> p2p::LocalSyncNodeRef {
    use inbound_connection_factory::InboundConnectionFactory as SyncConnectionFactory;

    SyncConnectionFactory::new(peers, local_sync_node).boxed()
}
