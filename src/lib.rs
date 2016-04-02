extern crate byteorder;
extern crate itertools;
extern crate mio;

mod managed;
mod packet;
mod connection;
mod socket_addr;

use std::io;

use std::net::SocketAddr;

use std::collections::HashMap;

use std::net::UdpSocket;

pub use managed::*;
pub use packet::*;
pub use connection::*;
pub use socket_addr::*;

/// Highly redundant, ordered, non-congesting pseudo-udp protocol
///
/// TODO: More documentation
///
/// NOTE: This protocol is very prone to starvations -- it expects high throughput!
/// IDEA: A "Managed" version of the socket that sends empty packets when unbalance is too high
pub trait GafferSocket: Sized {
  fn bind<A: ToSingleSocketAddr>(addr: A) -> io::Result<Self>;
  fn recv(&mut self) -> io::Result<GafferPacket>;
  fn send(&mut self, packet: GafferPacket) -> io::Result<usize>;
}


/// GafferSocket using an external to handle protocol guarantees
/// TODO: Implement "secured" version
pub struct SyncGafferSocket {
  udp_socket: UdpSocket,
  connections: HashMap<SocketAddr, Connection>
}

impl SyncGafferSocket {
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
    let final_packet = SyncGafferSocket::assemble_packet(seq_num, p.clone(), connection);
    let bytes = final_packet.serialized();
    self.udp_socket.send_to(bytes.as_slice(), &destination).map(|result| {
       connection.seq_num = seq_num.wrapping_add(1);
       result
    })
  }

  fn dropped_packets(&mut self, addr: SocketAddr) -> Vec<GafferPacket> {
    let connection = self.connections.entry(addr).or_insert(Connection::new());
    connection.dropped_packets.drain(..).collect()
  }
}

impl GafferSocket for SyncGafferSocket {
  fn bind<A: ToSingleSocketAddr>(addr: A) -> io::Result<Self> {
    let first_addr = addr.to_single_socket_addr().unwrap();
    UdpSocket::bind(&first_addr).map(|sock| {
      SyncGafferSocket { udp_socket: sock, connections: HashMap::new() }
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

#[cfg(test)]
mod test {
  pub use super::*;

  mod complete_gaffer_packet {
    use super::*;

    #[test]
    fn it_serializes() {
      let packet = CompleteGafferPacket {
        seq: 6,
        ack_seq: 20,
        ack_field: 1,
        payload: vec![1,2,3,4]
      };
      let bytes = packet.clone().serialized();
      let new_packet = CompleteGafferPacket::deserialize(bytes).unwrap();
      assert_eq!(packet, new_packet);
    }
  }

  mod external_acks {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn acking_single_packet() {
      let mut acks = ExternalAcks::new();
      acks.ack(0);

      assert_eq!(acks.last_seq, 0);
      assert_eq!(acks.field, 0);
    }

    #[test]
    fn acking_several_packets() {
      let mut acks = ExternalAcks::new();
      acks.ack(0);
      acks.ack(1);
      acks.ack(2);

      assert_eq!(acks.last_seq, 2);
      assert_eq!(acks.field, 1 | (1 << 1));
    }

    #[test]
    fn acking_several_packets_out_of_order() {
      let mut acks = ExternalAcks::new();
      acks.ack(1);
      acks.ack(0);
      acks.ack(2);

      assert_eq!(acks.last_seq, 2);
      assert_eq!(acks.field, 1 | (1 << 1));
    }

    #[test]
    fn acking_a_nearly_full_set_of_packets() {
      let mut acks = ExternalAcks::new();
      (0..32).foreach(|idx| acks.ack(idx));

      assert_eq!(acks.last_seq, 31);
      assert_eq!(acks.field, !0 >> 1);
    }

    #[test]
    fn acking_a_full_set_of_packets() {
      let mut acks = ExternalAcks::new();
      (0..33).foreach(|idx| acks.ack(idx));

      assert_eq!(acks.last_seq, 32);
      assert_eq!(acks.field, !0);
    }

    #[test]
    fn acking_to_the_edge_forward() {
      let mut acks = ExternalAcks::new();
      acks.ack(0);
      acks.ack(32);

      assert_eq!(acks.last_seq, 32);
      assert_eq!(acks.field, 1 << 31);
    }

    #[test]
    fn acking_too_far_forward() {
      let mut acks = ExternalAcks::new();
      acks.ack(0);
      acks.ack(1);
      acks.ack(34);

      assert_eq!(acks.last_seq, 34);
      assert_eq!(acks.field, 0);
    }

    #[test]
    fn acking_a_whole_buffer_too_far_forward() {
      let mut acks = ExternalAcks::new();
      acks.ack(0);
      acks.ack(60);

      assert_eq!(acks.last_seq, 60);
      assert_eq!(acks.field, 0);
    }


    #[test]
    fn acking_too_far_backward() {
      let mut acks = ExternalAcks::new();
      acks.ack(33);
      acks.ack(0);

      assert_eq!(acks.last_seq, 33);
      assert_eq!(acks.field, 0);
    }

    #[test]
    fn acking_around_zero() {
      let mut acks = ExternalAcks::new();
      (0..33).foreach(|idx: u16| acks.ack(idx.wrapping_sub(16)));
      assert_eq!(acks.last_seq, 16);
      assert_eq!(acks.field, !0);
    }

    #[test]
    fn ignores_old_packets() {
      let mut acks = ExternalAcks::new();
      acks.ack(40);
      acks.ack(0);
      assert_eq!(acks.last_seq, 40);
      assert_eq!(acks.field, 0);
    }

    #[test]
    fn ignores_really_old_packets() {
      let mut acks = ExternalAcks::new();
      acks.ack(30000);
      acks.ack(0);
      assert_eq!(acks.last_seq, 30000);
      assert_eq!(acks.field, 0);
    }

    #[test]
    fn skips_missing_acks_correctly() {
      let mut acks = ExternalAcks::new();
      acks.ack(0);
      acks.ack(1);
      acks.ack(6);
      acks.ack(4);
      assert_eq!(acks.last_seq, 6);
      assert_eq!(acks.field,
        0        | // 5 (missing)
        (1 << 1) | // 4 (present)
        (0 << 2) | // 3 (missing)
        (0 << 3) | // 2 (missing)
        (1 << 4) | // 1 (present)
        (1 << 5)   // 0 (present)
      );
    }
  }

  mod ack_record {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn acking_single_packet() {
      let mut record = AckRecord::new();
      record.enqueue(0, GafferPacket::dummy_packet());
      let dropped = record.ack(0, 0);
      assert_eq!(dropped.len(), 0);
      assert!(record.is_empty());
    }

    #[test]
    fn acking_several_packets() {
      let mut record = AckRecord::new();
      record.enqueue(0, GafferPacket::dummy_packet());
      record.enqueue(1, GafferPacket::dummy_packet());
      record.enqueue(2, GafferPacket::dummy_packet());
      let dropped = record.ack(2, 1 | (1 << 1));
      assert_eq!(dropped.len(), 0);
      assert!(record.is_empty());
    }

    #[test]
    fn acking_a_full_set_of_packets() {
      let mut record = AckRecord::new();
      (0..33).foreach(|idx| record.enqueue(idx, GafferPacket::dummy_packet()));
      let dropped = record.ack(32, !0);
      assert_eq!(dropped.len(), 0);
      assert!(record.is_empty());
    }

    #[test]
    fn dropping_one_packet() {
      let mut record = AckRecord::new();
      (0..34).foreach(|idx| record.enqueue(idx, GafferPacket::dummy_packet()));
      let dropped = record.ack(33, !0);
      assert_eq!(dropped, vec![(0, GafferPacket::dummy_packet())]);
      assert!(record.is_empty());
    }

    #[test]
    fn acking_around_zero() {
      let mut record = AckRecord::new();
      (0..33).foreach(|idx: u16| record.enqueue(idx.wrapping_sub(16), GafferPacket::dummy_packet()));
      let dropped = record.ack(16, !0);
      assert_eq!(dropped.len(), 0);
      assert!(record.is_empty());
    }

    #[test]
    fn not_dropping_new_packets() {
      let mut record = AckRecord::new();
      record.enqueue(0, GafferPacket::dummy_packet());
      record.enqueue(1, GafferPacket::dummy_packet());
      record.enqueue(2, GafferPacket::dummy_packet());
      record.enqueue(5, GafferPacket::dummy_packet());
      record.enqueue(30000, GafferPacket::dummy_packet());
      let dropped = record.ack(1, 1);
      assert_eq!(dropped.len(), 0);
      assert_eq!(record.len(), 3);
    }

    #[test]
    fn drops_old_packets() {
      let mut record = AckRecord::new();
      record.enqueue(0, GafferPacket::dummy_packet());
      record.enqueue(40, GafferPacket::dummy_packet());
      let dropped = record.ack(40, 0);
      assert_eq!(dropped, vec![(0, GafferPacket::dummy_packet())]);
      assert!(record.is_empty());
    }

    #[test]
    fn drops_really_old_packets() {
      let mut record = AckRecord::new();
      record.enqueue(50000, GafferPacket::dummy_packet());
      record.enqueue(0, GafferPacket::dummy_packet());
      record.enqueue(1, GafferPacket::dummy_packet());
      let dropped = record.ack(1, 1);
      assert_eq!(dropped, vec![(50000, GafferPacket::dummy_packet())]);
      assert!(record.is_empty());
    }
  }
}
