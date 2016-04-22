use addr::ToSingleSocketAddr;

use packet::{
  CompleteGafferPacket,
  GafferPacket
};

use connection::Connection;

use socket::GafferSocket;

use std::io;

use std::net::SocketAddr;

use std::collections::HashMap;

use std::net::UdpSocket;

/// GafferSocket using an external to handle protocol guarantees
/// TODO: Implement
/// TODO: Implement "secured" version
pub struct SimpleGafferSocket {
  udp_socket: UdpSocket,
  connections: HashMap<SocketAddr, Connection>
}

impl SimpleGafferSocket {
  fn assemble_packet( seq_num: u16, p: GafferPacket, connection: &Connection) -> CompleteGafferPacket {
    CompleteGafferPacket {
      seq: seq_num,
      ack_seq: connection.their_acks.last_seq,
      ack_field: connection.their_acks.field,
      payload: p.payload
    }
  }

  /// Send a single message
  ///
  /// - Get and increment sequence number
  /// - Remember packet
  /// - Add all headers
  ///   - Sequence #
  ///   - Current ack
  ///   - Ack bitfield
  /// - Send packet
  fn single_send(&mut self, p: GafferPacket) -> io::Result<usize> {
    let connection = self.connections.entry(p.addr).or_insert(Connection::new());
    let seq_num: u16 = connection.seq_num;
    connection.waiting_packets.enqueue(seq_num, p.clone());
    let destination = p.addr.clone(); // TODO: this is unnecessary
    let final_packet = SimpleGafferSocket::assemble_packet(seq_num, p.clone(), connection);
    let bytes = final_packet.serialized();
    self.udp_socket.send_to(bytes.as_ref(), &destination).map(|result| {
       connection.seq_num = seq_num.wrapping_add(1);
       result
    })
  }

  fn dropped_packets(&mut self, addr: SocketAddr) -> Vec<GafferPacket> {
    let connection = self.connections.entry(addr).or_insert(Connection::new());
    connection.dropped_packets.drain(..).collect()
  }
}

impl GafferSocket for SimpleGafferSocket {
  fn bind<A: ToSingleSocketAddr>(addr: A) -> io::Result<Self> {
    let first_addr = addr.to_single_socket_addr().unwrap();
    UdpSocket::bind(&first_addr).map(|sock| {
      SimpleGafferSocket { udp_socket: sock, connections: HashMap::new() }
    })
  }

  /// Receive a normal message
  ///
  /// - Get next message
  /// - Add its sequence # to our memory
  /// - Identify dropped packets from message header
  /// - Forget own acked packets
  /// - Enqueue Sure-Dropped packets into resubmit-queue
  fn recv(&mut self) -> io::Result<GafferPacket> {
    let mut res = [0; 1024];
    self.udp_socket.recv_from(&mut res)
      .and_then(|(_, addr)| {
        // NOTE: Copy in to_ve is suboptimal
        CompleteGafferPacket::deserialize(res.to_vec()).map(|res| (addr, res))
      })
      .map(|(addr, packet)| {
        let connection = self.connections.entry(addr).or_insert(Connection::new());
        connection.their_acks.ack(packet.seq);
        let dropped_packets = connection.waiting_packets.ack(packet.ack_seq, packet.ack_field);
        connection.dropped_packets = dropped_packets.into_iter().map(|(_, p)| p).collect();
        GafferPacket { addr: addr, payload: packet.payload }
      })
  }

  /// Send a normal message
  ///
  /// - Send dropped packets
  /// - Send packet
  fn send(&mut self, p: GafferPacket) -> io::Result<usize> {
    let dropped_packets = self.dropped_packets(p.addr);
    for packet in dropped_packets.into_iter() {
      // TODO: if this fails, a bunch of packets are dropped
      try!(self.single_send(packet));
    }
    self.single_send(p)
  }
}

