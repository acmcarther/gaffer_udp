use std::net::UdpSocket;
use std::collections::HashMap;
use gaffer_udp::SyncGafferSocket;

pub struct SocketWorld {
  pub sockets: HashMap<u16, UdpSocket>,
  pub gaffer_sockets: HashMap<u16, SyncGafferSocket>,
  // TODO: Move these into a struct
  pub gaffer_socket: Option<SyncGafferSocket>,
  pub socket: UdpSocket,
  pub seq: u16,
  pub ack: u16,
  pub packet_record: Vec<Vec<u8>>
}

impl SocketWorld {
  pub fn new() -> SocketWorld {
    SocketWorld {
      sockets: HashMap::new(),
      gaffer_sockets: HashMap::new(),
      // TODO: Move these into a struct
      gaffer_socket: None,
      socket: UdpSocket::bind(&("127.0.0.1", 9355)).unwrap(),
      seq: 0,
      ack: 0,
      packet_record: Vec::new()
    }
  }
}
