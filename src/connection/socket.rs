//! Special UdpSocket wrapper that demultiplexes multiple connections.

use std::collections::HashMap;
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::sync::Mutex;

use super::connection::ConnectionId;
use super::packet;
use super::packet::{PacketBuffer, PACKET_SIZE};
use super::StableBuildHasher;

/// An error associated with a socket.
#[derive(Debug)]
pub(super) enum SocketError {
	/// Ie WouldBlock
	NoPendingPackets,
	Io(IoError),
}

/// A client-side socket.
#[derive(Debug)]
pub(super) struct ClientSocket {
	socket: UdpSocket,
	pub(super) packet_buffer: PacketBuffer,
}

/// A server-side socket.
/// 
/// Buffers incoming packets per-connection, supplying only connection-specific data.
#[derive(Debug)]
pub(super) struct ServerSocket {
	socket: UdpSocket,
	pub(super) packet_buffer: PacketBuffer,
	connections: HashMap<ConnectionId, Vec<u8>>,
}

/// A socket backed by either server or client version.
#[derive(Debug)]
pub(super) enum Socket {
	Client(ClientSocket),
	Server(Mutex<ServerSocket>),
}

impl std::fmt::Display for SocketError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SocketError::NoPendingPackets => write!(f, "no pending packets"),
			SocketError::Io(error) => error.fmt(f)
		}
	}
}

impl Error for SocketError {
	#[inline]
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			SocketError::NoPendingPackets => None,
			SocketError::Io(error) => Some(error as &dyn Error),
		}
	}
}

impl From<IoError> for SocketError {
	#[inline]
	fn from(err: IoError) -> Self {
		match err.kind() {
			IoErrorKind::WouldBlock => Self::NoPendingPackets,
			_ => Self::Io(err),
		}
	}
}

impl ClientSocket {
	/// Create a new client socket using a provided udp-socket.
	#[inline]
	pub(super) fn new(socket: UdpSocket) -> Result<Self, IoError> {
		socket.set_nonblocking(true)?;
		Ok(Self { socket, packet_buffer: packet::new_buffer() })
	}

	/// Receive any pending packets optionally filtered by connection id.
	pub(super) fn recv_all<H: StableBuildHasher>(&mut self, buffer: &mut Vec<u8>, connection_id: Option<ConnectionId>, hash_builder: &H) -> Result<usize, SocketError> {
		let mut received_bytes = 0;
		loop {
			match self.socket.recv_from(&mut self.packet_buffer) {
				Ok((packet_size, _)) => {
					if packet_size == PACKET_SIZE && packet::valid_hash(&self.packet_buffer, hash_builder.build_hasher())
					{
						if let Some(connection_id) = connection_id {
							if connection_id == packet::get_header(&self.packet_buffer).connection_id {
								received_bytes += packet_size;
								buffer.extend_from_slice(&self.packet_buffer);
							}
						} else {
							received_bytes += packet_size;
							buffer.extend_from_slice(&self.packet_buffer);
						}
					}
				},
				Err(error) => break match error.kind() {
					IoErrorKind::WouldBlock => if received_bytes > 0 {
						Ok(received_bytes)
					} else {
						Err(SocketError::NoPendingPackets)
					},
					_ => Err(SocketError::Io(error)),
				}
			}
		}
	}

	/// Send data in a given slice to provided addr.
	#[inline]
	pub(super) fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<usize, IoError> {
		self.socket.send_to(data, addr)
	}
}

impl ServerSocket {
	/// Receive any pending packets for a provided connection id.
	pub(super) fn recv_all<H: StableBuildHasher>(&mut self, buffer: &mut Vec<u8>, connection_id: ConnectionId, hash_builder: &H) -> Result<usize, SocketError> {
		loop {
			match self.socket.recv(&mut self.packet_buffer) {
				Ok(packet_size) => {
					if packet_size == PACKET_SIZE && packet::valid_hash(&self.packet_buffer, hash_builder.build_hasher()) {
						// intentionally shadowed connection_id
						let connection_id = packet::get_header(&self.packet_buffer).connection_id;
						self.connections.get_mut(&connection_id).unwrap().extend_from_slice(&self.packet_buffer);
					}
				},
				Err(error) => match error.kind() {
					IoErrorKind::WouldBlock => break,
					_ => return Err(SocketError::Io(error)),
				}
			}
		};
		let reference_buffer = self.connections.get_mut(&connection_id).unwrap();
		if reference_buffer.is_empty() {
			Err(SocketError::NoPendingPackets)
		} else {
			buffer.extend(&reference_buffer[..]);
			let received_bytes = reference_buffer.len();
			reference_buffer.clear();
			Ok(received_bytes)
		}
	}

	/// Send data in a given slice to provided addr.
	#[inline]
	pub(super) fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<usize, IoError> {
		self.socket.send_to(data, addr)
	}
}

impl Socket {
	/// Send data in packet-buffer over the network.
	#[inline]
	pub(super) fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<(), IoError> {
		debug_assert!(data.len() == PACKET_SIZE);
		match self {
			Self::Client(client) => client.send_to(data, addr)?,
			Self::Server(server) => server.lock().unwrap().send_to(data, addr)?,
		};
		Ok(())
	}

	pub(super) fn recv_all<H: StableBuildHasher>(&mut self, buffer: &mut Vec<u8>, connection_id: ConnectionId, hash_builder: &H) -> Result<(), SocketError> {
		match self {
			Self::Client(client) => client.recv_all(buffer, Some(connection_id), hash_builder)?,
			Self::Server(server) => server.lock().unwrap().recv_all(buffer, connection_id, hash_builder)?,
		};
		Ok(())
	}
}
