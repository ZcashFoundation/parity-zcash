use crate::io::SharedTcpStream;
use message::common::Services;
use message::types;
use network::Magic;
use std::net;

pub struct Connection {
    pub stream: SharedTcpStream,
    pub version: u32,
    pub version_message: types::Version,
    pub magic: Magic,
    pub services: Services,
    pub address: net::SocketAddr,
}
