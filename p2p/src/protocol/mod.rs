mod addr;
mod ping;
mod sync;

use crate::bytes::Bytes;
use message::common::Command;
use message::Error;

pub use self::addr::{AddrProtocol, SeednodeProtocol};
pub use self::ping::PingProtocol;
pub use self::sync::{
    InboundSyncConnection, InboundSyncConnectionRef, InboundSyncConnectionState,
    InboundSyncConnectionStateRef, LocalSyncNode, LocalSyncNodeRef, OutboundSyncConnection,
    OutboundSyncConnectionRef, SyncProtocol,
};

pub trait Protocol: Send {
    /// Initialize the protocol.
    fn initialize(&mut self) {}

    /// Maintain the protocol.
    fn maintain(&mut self) {}

    /// Handle the message.
    fn on_message(&mut self, command: &Command, payload: &Bytes) -> Result<(), Error>;

    /// On disconnect.
    fn on_close(&mut self) {}

    /// Boxes the protocol.
    fn boxed(self) -> Box<Protocol>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}
