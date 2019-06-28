use futures::{Async, Future, Poll};
use io::{deadline, handshake, Deadline, Handshake};
use message::types::Version;
use message::Error;
use net::{Config, Connection};
use network::Magic;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio_core::net::{TcpStream, TcpStreamNew};
use tokio_core::reactor::Handle;

pub fn connect(address: &SocketAddr, handle: &Handle, config: &Config) -> Deadline<Connect> {
    let connect = Connect {
        state: ConnectState::TcpConnect {
            future: TcpStream::connect(address, handle),
            version: Some(config.version(address)),
        },
        magic: config.magic,
        address: *address,
        protocol_minimum: config.protocol_minimum,
    };

    deadline(Duration::new(5, 0), handle, connect).expect("Failed to create timeout")
}

enum ConnectState {
    TcpConnect {
        future: TcpStreamNew,
        version: Option<Version>,
    },
    Handshake(Handshake<TcpStream>),
    Connected,
}

pub struct Connect {
    state: ConnectState,
    magic: Magic,
    address: SocketAddr,
    protocol_minimum: u32,
}

impl Future for Connect {
    type Item = Result<Connection, Error>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (next, result) = match self.state {
            ConnectState::TcpConnect {
                ref mut future,
                ref mut version,
            } => {
                let stream = try_ready!(future.poll());
                let version = version.take().expect("state TcpConnect must have version");
                let handshake = handshake(stream, self.magic, version, self.protocol_minimum);
                (ConnectState::Handshake(handshake), Async::NotReady)
            }
            ConnectState::Handshake(ref mut future) => {
                let (stream, result) = try_ready!(future.poll());
                let result = match result {
                    Ok(result) => result,
                    Err(err) => return Ok(Async::Ready(Err(err))),
                };
                let connection = Connection {
                    stream: stream.into(),
                    services: result.version.services(),
                    version: result.negotiated_version,
                    version_message: result.version,
                    magic: self.magic,
                    address: self.address,
                };
                (ConnectState::Connected, Async::Ready(Ok(connection)))
            }
            ConnectState::Connected => panic!("poll Connect after it's done"),
        };

        self.state = next;
        match result {
            // by polling again, we register new future
            Async::NotReady => self.poll(),
            result => Ok(result),
        }
    }
}
