use std::net::UdpSocket;
use std::collections::HashMap;
use gaffer_udp::SimpleGafferSocket;

pub struct SocketWorld {
  pub sockets: HashMap<u16, UdpSocket>,
  pub gaffer_sockets: HashMap<u16, SimpleGafferSocket>,
}

impl SocketWorld {
  pub fn new() -> SocketWorld {
    SocketWorld {
      sockets: HashMap::new(),
      gaffer_sockets: HashMap::new()
    }
  }
}
