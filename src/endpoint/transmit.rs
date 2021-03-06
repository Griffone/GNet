//! [`Transmit`](Transmit) trait definition, implementation and unit test.

mod basic;
#[cfg(test)]
pub mod test;

use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::net::SocketAddr;

/// An error associated with an endpoint.
#[derive(Debug)]
pub enum TransmitError {
	/// The receiving operation would block.
	NoPendingPackets,
	/// Received datagram is not a valid one.
	MalformedPacket,
	/// An underlying error, different from just the non-blocking flag being set.
	Io(IoError),
}

/// A trait for objects that transmit data frames across network.
///
/// Implementors of `Transmit` trait are called 'transmitters'.
///
/// 'Transmitters' are responsible for sending and receiving data packets,
/// as well as validating that received data is the sent one.
///
/// `Transmitters` are NOT responsible for any of the following:
/// - Packet deduplication
/// - Ordering packets
/// - Delivering packets reliably
/// - Filtering non-protocol packets
///
/// **NOTE**: the `Connection` implementation assumes a UDP-like underlying protocol, specifically:
/// - Packets are delivered in a best-effort manner (may be dropped).
/// - Packets are delivered in no particular order.
pub trait Transmit {
	/// Current maximum size of a sent data packet.
	fn max_datagram_length(&self) -> usize;

	/// Send provided data to the provided address.
	///
	/// Return the number of bytes sent, which must be at least the length of `data`. Or the error
	/// responsible for the failure.
	///
	/// # Note
	/// Implementation may assume data is at most [`MAX_FRAME_LENGTH`](MAX_FRAME_LENGTH) bytes.
	fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<usize, IoError>;

	/// Attempt to recover an incoming datagram.
	///
	/// Return the number of bytes written to the buffer and the origin of the datagram on success.
	///
	/// # Note
	/// - May assume the buffer is able to hold [`MAX_FRAME_LENGTH`](MAX_FRAME_LENGTH) bytes.
	fn try_recv_from(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), TransmitError>;
}

impl From<IoError> for TransmitError {
	fn from(err: IoError) -> Self {
		if let IoErrorKind::WouldBlock = err.kind() {
			Self::NoPendingPackets
		} else {
			Self::Io(err)
		}
	}
}

impl PartialEq for TransmitError {
	fn eq(&self, rhs: &Self) -> bool {
		match self {
			Self::Io(lhs_error) => if let Self::Io(rhs_error) = rhs {
				lhs_error.kind() == rhs_error.kind()
			} else {
				false
			},
			Self::MalformedPacket => matches!(rhs, Self::MalformedPacket),
			Self::NoPendingPackets => matches!(rhs, Self::NoPendingPackets),
		}
	}
}

impl std::fmt::Display for TransmitError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NoPendingPackets => {
				write!(f, "there were no pending packets for provided connection")
			},
			Self::MalformedPacket => {
				write!(f, "the received packet was malformed")
			},
			Self::Io(error) => {
				write!(f, "underlying IO error: ")?;
				error.fmt(f)
			},
		}
	}
}

impl std::error::Error for TransmitError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::NoPendingPackets => None,
			Self::MalformedPacket => None,
			Self::Io(error) => Some(error),
		}
	}
}
