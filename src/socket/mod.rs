use std::io;

use addr::ToSingleSocketAddr;

use packet::GafferPacket;


pub mod blocking;
pub mod non_blocking;

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

