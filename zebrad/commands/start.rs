use super::super::rpc;
use config;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use util::{init_db, node_table_path};
use zebra_p2p;
use zebra_primitives::hash::H256;
use zebra_sync::{
    create_local_sync_node, create_sync_connection_factory, create_sync_peers, SyncListener,
};
use {ZCASH_PROTOCOL_MINIMUM, ZCASH_PROTOCOL_VERSION};

enum BlockNotifierTask {
    NewBlock(H256),
    Stop,
}

struct BlockNotifier {
    tx: Sender<BlockNotifierTask>,
    is_synchronizing: Arc<AtomicBool>,
    worker_thread: Option<thread::JoinHandle<()>>,
}

impl BlockNotifier {
    pub fn new(block_notify_command: String) -> Self {
        let (tx, rx) = channel();
        let is_synchronizing = Arc::new(AtomicBool::default());
        BlockNotifier {
            tx: tx,
            is_synchronizing: is_synchronizing.clone(),
            worker_thread: Some(
                thread::Builder::new()
                    .name("Block notification thread".to_owned())
                    .spawn(move || BlockNotifier::worker(rx, block_notify_command))
                    .expect("Error creating block notification thread"),
            ),
        }
    }

    fn worker(rx: Receiver<BlockNotifierTask>, block_notify_command: String) {
        for cmd in rx {
            match cmd {
                BlockNotifierTask::NewBlock(new_block_hash) => {
                    let new_block_hash = new_block_hash.to_reversed_str();
                    let command = block_notify_command.replace("%s", &new_block_hash);
                    let c_command = ::std::ffi::CString::new(command.clone()).unwrap();
                    unsafe {
                        use libc::system;

                        let err = system(c_command.as_ptr());
                        if err != 0 {
                            error!(target: "zebra", "Block notification command {} exited with error code {}", command, err);
                        }
                    }
                }
                BlockNotifierTask::Stop => break,
            }
        }
        trace!(target: "zebra", "Block notification thread stopped");
    }
}

impl SyncListener for BlockNotifier {
    fn synchronization_state_switched(&self, is_synchronizing: bool) {
        self.is_synchronizing
            .store(is_synchronizing, Ordering::SeqCst);
    }

    fn best_storage_block_inserted(&self, block_hash: &H256) {
        if !self.is_synchronizing.load(Ordering::SeqCst) {
            self.tx
                .send(BlockNotifierTask::NewBlock(block_hash.clone()))
                .expect("Block notification thread have the same lifetime as `BlockNotifier`")
        }
    }
}

impl Drop for BlockNotifier {
    fn drop(&mut self) {
        if let Some(join_handle) = self.worker_thread.take() {
            let _ = self.tx.send(BlockNotifierTask::Stop);
            join_handle.join().expect("Clean shutdown.");
        }
    }
}

pub fn start(cfg: config::Config) -> Result<(), String> {
    let mut el = zebra_p2p::event_loop();

    init_db(&cfg)?;

    let nodes_path = node_table_path(&cfg);

    let p2p_cfg = zebra_p2p::Config {
        threads: cfg.p2p_threads,
        inbound_connections: cfg.inbound_connections,
        outbound_connections: cfg.outbound_connections,
        connection: zebra_p2p::NetConfig {
            protocol_version: ZCASH_PROTOCOL_VERSION,
            protocol_minimum: ZCASH_PROTOCOL_MINIMUM,
            magic: cfg.consensus.magic(),
            local_address: SocketAddr::new(cfg.host, cfg.port),
            services: cfg.services,
            user_agent: cfg.user_agent,
            start_height: 0,
            relay: true,
        },
        peers: cfg.connect.map_or_else(|| vec![], |x| vec![x]),
        seeds: cfg.seednodes,
        node_table_path: nodes_path,
        preferable_services: cfg.services,
        internet_protocol: cfg.internet_protocol,
    };

    let sync_peers = create_sync_peers();
    let local_sync_node = create_local_sync_node(
        cfg.consensus.clone(),
        cfg.db.clone(),
        sync_peers.clone(),
        cfg.verification_params,
    );
    let sync_connection_factory =
        create_sync_connection_factory(sync_peers.clone(), local_sync_node.clone());

    if let Some(block_notify_command) = cfg.block_notify_command {
        local_sync_node.install_sync_listener(Box::new(BlockNotifier::new(block_notify_command)));
    }

    let p2p = try!(
        zebra_p2p::P2P::new(p2p_cfg, sync_connection_factory, el.handle())
            .map_err(|x| x.to_string())
    );
    let rpc_deps = rpc::Dependencies {
        consensus: cfg.consensus,
        storage: cfg.db,
        local_sync_node: local_sync_node,
        p2p_context: p2p.context().clone(),
        miner_address: cfg.miner_address,
    };
    let _rpc_server = try!(rpc::new_http(cfg.rpc_config, rpc_deps));

    try!(p2p.run().map_err(|_| "Failed to start p2p module"));
    el.run(zebra_p2p::forever()).unwrap();
    Ok(())
}
