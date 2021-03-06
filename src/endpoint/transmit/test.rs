//! Generic testing functions for [`Transmit`](Transmit) implementations.

use super::Transmit;

use std::cmp::max;
use std::net::SocketAddr;

const DATAGRAMS: [&[u8]; 3] = [
	b"GNET TRANSMIT TEST FIRST DATAGRAM",
	b"GNET TRANSMIT TEST SECOND DATAGRAM",
	b"GNET TRANSMIT TEST THIRD DATAGRAM",
];

/// Test that provided [`Transmit`](Transmit) implementations are able to communicate with each other.
pub fn generic_transmit_test<S: Transmit, R: Transmit>(
	(sender, sender_addr): (&S, SocketAddr),
	(receiver, receiver_addr): (&R, SocketAddr),
) {
	let max_datagram_length = DATAGRAMS.iter().fold(0, |acc, x| max(acc, x.len()));

	assert!(sender.max_datagram_length() >= max_datagram_length);
	assert!(receiver.max_datagram_length() >= max_datagram_length);
	assert!(receiver.max_datagram_length() >= sender.max_datagram_length());

	assert_eq!(
		sender.send_to(DATAGRAMS[0], receiver_addr).expect("Failed to send first datagram!"),
		DATAGRAMS[0].len(),
	);
	assert_eq!(
		sender.send_to(DATAGRAMS[1], receiver_addr).expect("Failed to send second datagram!"),
		DATAGRAMS[1].len(),
	);

	let mut buffer = vec![0; receiver.max_datagram_length()];

	let recv_result = receiver.try_recv_from(&mut buffer).unwrap();
	if &buffer[.. DATAGRAMS[0].len()] == DATAGRAMS[0] {
		assert_eq!(recv_result, (DATAGRAMS[0].len(), sender_addr));
		assert_eq!(receiver.try_recv_from(&mut buffer), Ok((DATAGRAMS[1].len(), sender_addr)));
		assert_eq!(&buffer[.. DATAGRAMS[1].len()], DATAGRAMS[1]);
	} else {
		assert_eq!(&buffer[.. DATAGRAMS[1].len()], DATAGRAMS[1]);
		assert_eq!(recv_result, (DATAGRAMS[1].len(), sender_addr));
		assert_eq!(receiver.try_recv_from(&mut buffer), Ok((DATAGRAMS[0].len(), sender_addr)));
		assert_eq!(&buffer[.. DATAGRAMS[0].len()], DATAGRAMS[0]);
	}
}
