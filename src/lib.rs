extern crate byteorder;
extern crate itertools;
extern crate mio;

use std::io::{self, Cursor};

use std::net::{
  ToSocketAddrs,
  SocketAddr
};

use std::collections::HashMap;

//use mio::udp::UdpSocket;
use std::net::UdpSocket;

use itertools::Itertools;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub trait ToSingleSocketAddr {
  fn to_single_socket_addr(&self) -> io::Result<SocketAddr>;
}

impl <T> ToSingleSocketAddr for T where T: ToSocketAddrs {
  fn to_single_socket_addr(&self) -> io::Result<SocketAddr> {
    self.to_socket_addrs().and_then(|mut iter| {
      iter.next().ok_or(io::Error::new(io::ErrorKind::Other, "There was no socket addr"))
    })
  }
}

/// TODO: consider slice
pub type GafferPayload = Vec<u8>;

/// Highly redundant, ordered, non-congesting pseudo-udp protocol
///
/// TODO: More documentation
///
/// NOTE: This protocol is very prone to starvations -- it expects high throughput!
pub trait GafferSocket: Sized {
  fn bind<A: ToSingleSocketAddr>(addr: A) -> io::Result<Self>;
  fn recv(&mut self) -> io::Result<GafferPacket>;
  fn send(&mut self, packet: GafferPacket) -> io::Result<usize>;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GafferPacket {
  pub addr: SocketAddr,
  pub payload: GafferPayload
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompleteGafferPacket {
  pub seq: u16,
  pub ack_seq: u16,
  pub ack_field: u32,
  pub payload: GafferPayload
}

impl CompleteGafferPacket {
  pub fn serialized(&self) -> Vec<u8> {
    let mut wtr = Vec::new();
    wtr.write_u16::<BigEndian>(self.seq).unwrap();
    wtr.write_u16::<BigEndian>(self.ack_seq).unwrap();
    wtr.write_u32::<BigEndian>(self.ack_field).unwrap();
    wtr.append(&mut self.payload.clone());
    wtr
  }

  pub fn deserialize(mut bytes: Vec<u8>) -> io::Result<CompleteGafferPacket> {
    let payload = bytes.split_off(8);
    let mut rdr = Cursor::new(bytes);

    let seq = try!(rdr.read_u16::<BigEndian>());
    let ack_seq = try!(rdr.read_u16::<BigEndian>());
    let ack_field = try!(rdr.read_u32::<BigEndian>());

    Ok(CompleteGafferPacket {
      seq: seq,
      ack_seq: ack_seq,
      ack_field: ack_field,
      payload: payload
    })
  }
}

impl GafferPacket {
  pub fn dummy_packet() -> GafferPacket {
    GafferPacket::new("0.0.0.0:7878", GafferPayload::new())
  }

  pub fn new<A: ToSingleSocketAddr>(addr: A, payload: GafferPayload) -> GafferPacket {
    let first_addr = addr.to_single_socket_addr().unwrap();
    GafferPacket { addr: first_addr, payload: payload }
  }
}

/// GafferSocket using an external to handle protocol guarantees
/// TODO: Implement
/// TODO: Implement "secured" version
pub struct SimpleGafferSocket {
  udp_socket: UdpSocket,
  connections: HashMap<SocketAddr, Connection>
}

/// Connection to a known third party
///
/// Contains:
/// - own unacked sent-packets
/// - ack-state of third party's packets
/// - own dropped packets
/// - own sequence number
#[derive(Debug)]
pub struct Connection {
  pub seq_num: u16,
  pub dropped_packets: Vec<GafferPacket>,
  pub waiting_packets: AckRecord,
  pub their_acks: ExternalAcks,
}

impl Connection {
  pub fn new() -> Connection {
    Connection {
      seq_num: 0,
      dropped_packets: Vec::new(),
      waiting_packets: AckRecord::new(),
      their_acks: ExternalAcks::new()
    }
  }
}

/// Third party's ack information
///
/// Holds the latest seq_num we've seen from them and the 32 bit bitfield 
/// for extra redundancy
#[derive(Debug)]
pub struct ExternalAcks {
  pub last_seq: u16,
  pub field: u32,
  initialized: bool
}

impl ExternalAcks {
  pub fn new() -> ExternalAcks {
    ExternalAcks { last_seq: 0, field: 0, initialized: false }
  }

  pub fn ack(&mut self, seq_num: u16) {
    if !self.initialized {
      self.last_seq = seq_num;
      self.initialized = true;
      return;
    }

    let pos_diff = seq_num.wrapping_sub(self.last_seq);
    let neg_diff = self.last_seq.wrapping_sub(seq_num);
    if pos_diff == 0 {
      return;
    } if pos_diff < 32000 {
      if pos_diff <= 32 {
        self.field = ((self.field << 1 ) | 1) << (pos_diff - 1);
      } else {
        self.field = 0;
      }
      self.last_seq = seq_num;
    } else if neg_diff <= 32 {
      self.field = self.field | (1 << neg_diff - 1);
    }
  }
}

/// Packets waiting for an ack
///
/// Holds up to 32 packets waiting for ack
///
/// Additionally, holds packets "forward" of the current ack packet
#[derive(Debug)]
pub struct AckRecord {
  packets: HashMap<u16, GafferPacket>
}

impl AckRecord {
  pub fn new() -> AckRecord {
    AckRecord { packets: HashMap::new() }
  }

  pub fn is_empty(&mut self) -> bool {
    self.packets.is_empty()
  }

  pub fn len(&mut self) -> usize {
    self.packets.len()
  }

  /// Adds a packet to the waiting packets
  pub fn enqueue(&mut self, seq: u16, packet: GafferPacket) {
    // TODO: Handle overwriting other packet?
    //   That really shouldn't happen, but it should be encoded here
    self.packets.insert(seq, packet);
  }

  /// Finds and removes acked packets, returning dropped packets
  #[allow(unused_parens)]
  pub fn ack(&mut self, seq: u16, seq_field: u32) -> Vec<(u16, GafferPacket)> {
    let mut dropped_packets = Vec::new();
    let mut acked_packets = Vec::new();
    self.packets.keys().foreach(|k| {
      let diff = seq.wrapping_sub(*k);
      if diff == 0 {
        acked_packets.push(*k);
      } else if diff <= 32 {
        let field_acked = (seq_field & (1 << diff - 1) != 0);
        if field_acked {
          acked_packets.push(*k);
        }
      } else if diff < 32000 {
        dropped_packets.push(*k);
      }
    });
    acked_packets.into_iter().foreach(|seq| { self.packets.remove(&seq); });
    dropped_packets.into_iter().map(|seq| (seq, self.packets.remove(&seq).unwrap())).collect()
  }
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
