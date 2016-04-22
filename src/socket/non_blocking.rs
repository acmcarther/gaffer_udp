use std::collections::HashMap;

use std::net::SocketAddr;

use mio::udp::UdpSocket;

use connection::Connection;

#[allow(dead_code)]
pub struct NonBlockingGafferSocket {
  udp_socket: UdpSocket,
  connections: HashMap<SocketAddr, Connection>
}
