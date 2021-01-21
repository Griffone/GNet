//! Message-based networking over UDP for real-time applications.
// TODO: list important traits and structs

#![warn(clippy::all)]

pub mod byte;
pub mod connection;
pub mod endpoint;
pub mod id;
pub mod listen;
pub mod packet;

pub use connection::{Connection, ConnectionError, PendingConnection, PendingConnectionError};
pub use listen::{AcceptError, ConnectionListener};

use crate::byte::ByteSerialize;

/// Possible message that is passed by connections.
pub trait Parcel: ByteSerialize {}

#[cfg(test)]
impl Parcel for () {}
