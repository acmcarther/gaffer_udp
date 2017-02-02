use addr::ToSingleSocketAddr;

use packet::{
  CompleteGafferPacket,
  GafferPacket
};

use socket::GafferState;

use std::io;

use std::net::UdpSocket;

pub struct GafferSocket {
  udp_socket: UdpSocket,
  state: GafferState,
  recv_buffer: [u8; 8192]
}

impl GafferSocket {
  pub fn bind<A: ToSingleSocketAddr>(addr: A) -> io::Result<Self> {
    let first_addr = addr.to_single_socket_addr().unwrap();
    UdpSocket::bind(&first_addr).map(|sock| {
      GafferSocket {
        udp_socket: sock,
        state: GafferState::new(),
        recv_buffer: [8; 8192]
      }
    })
  }

  /// Receive a normal message
  ///
  /// - Get next message
  /// - Add its sequence # to our memory
  /// - Identify dropped packets from message header
  /// - Forget own acked packets
  /// - Enqueue Sure-Dropped packets into resubmit-queue
  pub fn recv(&mut self) -> io::Result<GafferPacket> {
    let output = self.udp_socket.recv_from(&mut self.recv_buffer);

    output
      // TODO: Fix to_vec, it is suboptimal here
      .and_then(|(len, addr)| CompleteGafferPacket::deserialize(self.recv_buffer[..len].to_vec()).map(|res| (addr, res)) )
      .map(|(addr, packet)| self.state.receive(addr, packet))
  }

  /// Send a normal message
  ///
  /// - Send dropped packets
  /// - Send packet
  pub fn send(&mut self, p: GafferPacket) -> io::Result<usize> {
    let dropped_packets = self.state.dropped_packets(p.addr);
    for packet in dropped_packets.into_iter() {
      // TODO: if this fails, a bunch of packets are dropped
      try!(self.single_send(packet));
    }
    self.single_send(p)
  }

  ///
  /// - Get and increment sequence number
  /// - Remember packet
  /// - Add all headers
  ///   - Sequence #
  ///   - Current ack
  ///   - Ack bitfield
  /// - Send packet
  fn single_send(&mut self, p: GafferPacket) -> io::Result<usize> {
    let (destination, payload) = self.state.preprocess_packet(p);

    self.udp_socket.send_to(payload.as_ref(), &destination)
  }
}

