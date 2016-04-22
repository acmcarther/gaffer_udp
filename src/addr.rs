use std::io;

use std::net::{
  ToSocketAddrs,
  SocketAddr
};

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

