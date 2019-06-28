use futures::{Future, Poll};
use io::{accept_handshake, deadline, AcceptHandshake, Deadline};
use message::MessageResult;
use net::{Config, Connection};
use network::Magic;
use std::time::Duration;
use std::{io, net};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;

pub fn accept_connection(
    stream: TcpStream,
    handle: &Handle,
    config: &Config,
    address: net::SocketAddr,
) -> Deadline<AcceptConnection> {
    let accept = AcceptConnection {
        handshake: accept_handshake(
            stream,
            config.magic,
            config.version(&address),
            config.protocol_minimum,
        ),
        magic: config.magic,
        address: address,
    };

    deadline(Duration::new(5, 0), handle, accept).expect("Failed to create timeout")
}

pub struct AcceptConnection {
    handshake: AcceptHandshake<TcpStream>,
    magic: Magic,
    address: net::SocketAddr,
}

impl Future for AcceptConnection {
    type Item = MessageResult<Connection>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (stream, result) = try_ready!(self.handshake.poll());
        let result = match result {
            Ok(result) => result,
            Err(err) => return Ok(Err(err).into()),
        };
        let connection = Connection {
            stream: stream.into(),
            services: result.version.services(),
            version: result.negotiated_version,
            version_message: result.version,
            magic: self.magic,
            address: self.address,
        };
        Ok(Ok(connection).into())
    }
}
