use std::io;

use mio::udp::UdpSocket;

use socket::GafferState;
use addr::ToSingleSocketAddr;

use packet::{
  CompleteGafferPacket,
  GafferPacket
};

#[allow(dead_code)]
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
        recv_buffer: [0; 8192]
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
  pub fn recv(&mut self) -> io::Result<Option<GafferPacket>> {
    self.udp_socket.recv_from(&mut self.recv_buffer)
      .and_then(|opt| {
        match opt {
          Some((len, addr)) => {
            // TODO: Fix to_vec, it is suboptimal here
            CompleteGafferPacket::deserialize(self.recv_buffer[..len].to_vec())
              .map(|packet| Some(self.state.receive(addr, packet)))
          },
          None => Ok(None)
        }
      })
  }

  /// Send a normal message
  ///
  /// - Send dropped packets
  /// - Send packet
  pub fn send(&mut self, p: GafferPacket) -> io::Result<Option<usize>> {
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
  fn single_send(&mut self, p: GafferPacket) -> io::Result<Option<usize>> {
    let (destination, payload) = self.state.preprocess_packet(p);

    self.udp_socket.send_to(payload.as_ref(), &destination)
  }
}

#[cfg(test)]
mod tests{

  use super::*;
  use packet::GafferPacket;

  #[test]
  fn recv_doesnt_block() {
    let mut sock = GafferSocket::bind("0.0.0.0:45213").unwrap();

    let payload = sock.recv();
    assert!(payload.is_ok());
    assert_eq!(payload.unwrap(), None);
  }

  #[test]
  fn recv_can_recv() {
    let mut send_sock = GafferSocket::bind("0.0.0.0:45214").unwrap();
    let mut recv_sock = GafferSocket::bind("0.0.0.0:45215").unwrap();
    let send_res = send_sock.send(GafferPacket::new("127.0.0.1:45215", vec![1, 2, 3]));
    assert!(send_res.is_ok());
    assert!(send_res.unwrap().is_some());


    let packet = recv_sock.recv();
    assert!(packet.is_ok());
    let packet_payload = packet.unwrap();
    assert!(packet_payload.is_some());
    let unwrap_pkt = packet_payload.unwrap();
    assert_eq!(unwrap_pkt.payload, vec![1, 2, 3]);
    let addr = unwrap_pkt.addr;
    assert_eq!(addr.to_string(), "127.0.0.1:45214");
  }
}
