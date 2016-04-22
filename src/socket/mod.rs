use packet::{
  CompleteGafferPacket,
  GafferPacket
};

use connection::Connection;

use std::net::SocketAddr;

use std::collections::HashMap;


pub mod blocking;
pub mod non_blocking;

pub struct GafferState {
  connections: HashMap<SocketAddr, Connection>
}

impl GafferState {
  pub fn new() -> GafferState {
    GafferState { connections: HashMap::new() }
  }

  pub fn preprocess_packet(&mut self, p: GafferPacket) -> (SocketAddr, Vec<u8>) {
    let connection = self.connections.entry(p.addr).or_insert(Connection::new());
    connection.waiting_packets.enqueue(connection.seq_num, p.clone());
    let final_packet = helpers::assemble_packet(connection.seq_num, p.clone(), connection);
    connection.seq_num = connection.seq_num.wrapping_add(1);
    (p.addr, final_packet.serialized())
  }

  pub fn dropped_packets(&mut self, addr: SocketAddr) -> Vec<GafferPacket> {
    let connection = self.connections.entry(addr).or_insert(Connection::new());
    connection.dropped_packets.drain(..).collect()
  }

  fn receive(&mut self, addr: SocketAddr, packet: CompleteGafferPacket) -> GafferPacket {
    let connection = self.connections.entry(addr).or_insert(Connection::new());
    connection.their_acks.ack(packet.seq);
    let dropped_packets = connection.waiting_packets.ack(packet.ack_seq, packet.ack_field);
    connection.dropped_packets = dropped_packets.into_iter().map(|(_, p)| p).collect();
    GafferPacket { addr: addr, payload: packet.payload }
  }
}



pub mod helpers {
  use connection::Connection;

  use packet::{
    CompleteGafferPacket,
    GafferPacket
  };

  pub fn assemble_packet( seq_num: u16, p: GafferPacket, connection: &Connection) -> CompleteGafferPacket {
    CompleteGafferPacket {
      seq: seq_num,
      ack_seq: connection.their_acks.last_seq,
      ack_field: connection.their_acks.field,
      payload: p.payload
    }
  }
}
