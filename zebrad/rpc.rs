use rpc_apis::{self, ApiSet};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use zebra_keys::Address;
use zebra_network::ConsensusParams;
use zebra_p2p;
use zebra_rpc::{start_http, Compatibility, MetaIoHandler, Server};
use zebra_storage;
use zebra_sync;

pub struct Dependencies {
    pub consensus: ConsensusParams,
    pub local_sync_node: zebra_sync::LocalNodeRef,
    pub storage: zebra_storage::SharedStore,
    pub p2p_context: Arc<zebra_p2p::Context>,
    pub miner_address: Option<Address>,
}

#[derive(Debug, PartialEq)]
pub struct HttpConfiguration {
    pub enabled: bool,
    pub interface: String,
    pub port: u16,
    pub apis: ApiSet,
    pub cors: Option<Vec<String>>,
    pub hosts: Option<Vec<String>>,
}

impl HttpConfiguration {
    pub fn with_port(port: u16) -> Self {
        HttpConfiguration {
            enabled: true,
            interface: "127.0.0.1".into(),
            port: port,
            apis: ApiSet::default(),
            cors: None,
            hosts: Some(Vec::new()),
        }
    }
}

pub fn new_http(conf: HttpConfiguration, deps: Dependencies) -> Result<Option<Server>, String> {
    if !conf.enabled {
        return Ok(None);
    }

    let url = format!("{}:{}", conf.interface, conf.port);
    let addr = try!(url
        .parse()
        .map_err(|_| format!("Invalid JSONRPC listen host/port given: {}", url)));
    Ok(Some(try!(setup_http_rpc_server(
        &addr, conf.cors, conf.hosts, conf.apis, deps
    ))))
}

pub fn setup_http_rpc_server(
    url: &SocketAddr,
    cors_domains: Option<Vec<String>>,
    allowed_hosts: Option<Vec<String>>,
    apis: ApiSet,
    deps: Dependencies,
) -> Result<Server, String> {
    let server = setup_rpc_server(apis, deps);
    let start_result = start_http(url, cors_domains, allowed_hosts, server);
    match start_result {
		Err(ref err) if err.kind() == io::ErrorKind::AddrInUse => {
			Err(format!("RPC address {} is already in use, make sure that another instance of a Zcash node is not running or change the address using the --jsonrpc-port and --jsonrpc-interface options.", url))
		},
		Err(e) => Err(format!("RPC error: {:?}", e)),
		Ok(server) => Ok(server),
	}
}

fn setup_rpc_server(apis: ApiSet, deps: Dependencies) -> MetaIoHandler<()> {
    rpc_apis::setup_rpc(
        MetaIoHandler::with_compatibility(Compatibility::Both),
        apis,
        deps,
    )
}
