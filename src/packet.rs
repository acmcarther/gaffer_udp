use addr::ToSingleSocketAddr;

use std::io::{self, Cursor};

use std::net::SocketAddr;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

/// TODO: consider slice
pub type GafferPayload = Vec<u8>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GafferPacket {
  pub addr: SocketAddr,
  pub payload: GafferPayload
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

