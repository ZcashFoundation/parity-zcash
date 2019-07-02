use io::SharedTcpStream;
use std::net;
use zebra_message::common::Services;
use zebra_message::types;
use zebra_network::Magic;

pub struct Connection {
    pub stream: SharedTcpStream,
    pub version: u32,
    pub version_message: types::Version,
    pub magic: Magic,
    pub services: Services,
    pub address: net::SocketAddr,
}
