use super::SyncListener;
use futures::Future;
use crate::local_node::LocalNode;
use miner::MemoryPool;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use storage;
use crate::synchronization_client::SynchronizationClient;
use crate::synchronization_executor::LocalSynchronizationTaskExecutor;
use crate::synchronization_peers::Peers;
use crate::synchronization_server::ServerImpl;
use crate::synchronization_verifier::AsyncVerifier;
use crate::utils::SynchronizationState;

pub use crate::utils::BlockHeight;

/// Network request id
pub type RequestId = u32;

/// Peer is indexed using this type
pub type PeerIndex = usize;

// No-error, no-result future
pub type EmptyBoxFuture = Box<Future<Item = (), Error = ()> + Send>;

/// Reference to storage
pub type StorageRef = storage::SharedStore;

/// Reference to memory pool
pub type MemoryPoolRef = Arc<RwLock<MemoryPool>>;

/// Shared synchronization state reference
pub type SynchronizationStateRef = Arc<SynchronizationState>;

/// Reference to peers
pub type PeersRef = Arc<Peers>;

/// Reference to synchronization tasks executor
pub type ExecutorRef<T> = Arc<T>;

/// Reference to synchronization client
pub type ClientRef<T> = Arc<T>;

/// Reference to synchronization client core
pub type ClientCoreRef<T> = Arc<Mutex<T>>;

/// Reference to synchronization server
pub type ServerRef<T> = Arc<T>;

/// Reference to local node
pub type LocalNodeRef = Arc<
    LocalNode<ServerImpl, SynchronizationClient<LocalSynchronizationTaskExecutor, AsyncVerifier>>,
>;

/// Synchronization events listener reference
pub type SyncListenerRef = Box<SyncListener>;
