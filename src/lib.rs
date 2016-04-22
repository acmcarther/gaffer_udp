extern crate byteorder;
extern crate itertools;
extern crate mio;

pub mod addr;
pub mod packet;
pub mod connection;
pub mod socket;

#[cfg(test)]
mod test {
  pub use addr::ToSingleSocketAddr;
  pub use packet::*;
  pub use connection::*;
  pub use socket::*;

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
