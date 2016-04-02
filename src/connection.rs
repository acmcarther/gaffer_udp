use std::collections::HashMap;

use itertools::Itertools;

use packet::GafferPacket;

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
