//! Zebrad Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use std::net::SocketAddr;
use std::path::PathBuf;

use abscissa::Config;
use serde::{Deserialize, Serialize};

/// Zebrad Configuration
#[derive(Clone, Config, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ZebradConfig {
    /// Which Zcash network to connect to.
    #[serde(default)]
    pub zcash_network: zebra_network::Network,
    /// Server resource configuration.
    #[serde(default)]
    pub server: ServerSection,
    /// Network connection configuration.
    #[serde(default)]
    pub network: NetworkSection,
    /// RPC configuration.
    #[serde(default)]
    pub rpc: RPCSection,
}

/// Network Configuration
///
/// Contains settings related to (internet) networking.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct NetworkSection {
    /// Whether to use IPv4 or IPv6 or both.
    pub network_type: zebra_p2p::InternetProtocol,
    /// If specified, the host address for the node including a port.
    pub host_addr: Option<SocketAddr>,
    /// A list of peers to connect to.
    pub peers: Vec<SocketAddr>,
    /// A list of seed nodes to retrieve peer addresses from.
    pub seeds: Vec<String>,
}

impl Default for NetworkSection {
    fn default() -> Self {
        NetworkSection {
            network_type: zebra_p2p::InternetProtocol::default(),
            // The default port depends on the zcash_network parameter
            // in the global config, which we don't know here.
            // The host addr depends on the IP version.  If we set a
            // default for it here, and the network_type was
            // overridden, we might have the wrong type.  So it's
            // defined as an Option and set to None.
            host_addr: None,
            peers: Vec::default(),
            seeds: Vec::default(),
        }
    }
}

/// Server Configuration
///
/// Contains settings determining server resources.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ServerSection {
    /// The size of the database cache, in MB.
    pub db_cache_size: usize,
    /// The server's data directory.
    pub data_dir: PathBuf,
    /// The maximum number of inbound connections.
    pub max_inbound_connections: usize,
    /// The maximum number of outbound connections.
    pub max_outbound_connections: usize,
    /// The number of threads used by p2p (?? maybe?)
    pub p2p_thread_count: usize,
    /// The user-agent for this server (?? why is this an option?)
    pub user_agent: String,
    /// Whether to suppress output.
    pub quiet: bool,
    /// Block notify command (?? do we need this?)
    pub block_notify_command: Option<String>,
}

impl Default for ServerSection {
    fn default() -> Self {
        use app_dirs::{app_dir, AppDataType};

        ServerSection {
            db_cache_size: 512,
            data_dir: app_dir(AppDataType::UserData, &crate::APP_INFO, "db")
                .expect("Could not construct default data dir"),
            max_inbound_connections: 10,
            max_outbound_connections: 10,
            p2p_thread_count: num_cpus::get(),
            user_agent: "zebrad".into(),
            quiet: false,
            block_notify_command: None,
        }
    }
}

/// RPC Configuration
///
/// Contains settings relating to the RPC interface.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct RPCSection {
    /// Whether or not the RPC is enabled
    pub enabled: bool,
    // XXX fill in this section
}

impl Default for RPCSection {
    fn default() -> Self {
        RPCSection { enabled: false }
    }
}
