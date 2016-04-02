use std::io;
use std::net::{
  SocketAddr,
  UdpSocket
};
use std::collections::HashMap;

use super::{
  GafferSocket,
  ToSingleSocketAddr,
  Connection
};

#[allow(dead_code)]
pub struct ManagedGafferSocket {
  recv_socket: UdpSocket,
  send_socket: UdpSocket,
  connections: HashMap<SocketAddr, Connection>
}

#[allow(dead_code)]
impl ManagedGafferSocket {
  fn bind<A: ToSingleSocketAddr>(addr: A) -> io::Result<Self> {
    let first_addr = addr.to_single_socket_addr().unwrap();
    UdpSocket::bind(&first_addr).and_then(|sock| {
      let clone_sock = try!(sock.try_clone());
      Ok(ManagedGafferSocket {
        recv_socket: clone_sock,
        send_socket: sock,
        connections: HashMap::new()
      })
    })
  }
}
