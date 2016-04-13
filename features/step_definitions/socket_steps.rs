use cucumber::{
  Cucumber,
  CucumberRegistrar,
  InvokeResponse,
  InvokeArgument,
};

use support::env::SocketWorld;

use support::packets::FromTable;

use std::net::UdpSocket;

use gaffer_udp::{
  CompleteGafferPacket,
  ToSingleSocketAddr,
};

pub fn register_steps(c: &mut CucumberRegistrar<SocketWorld>) {
  Given!(c, "^a normal socket on (\\d+)$", |_, world: &mut SocketWorld, (port,): (u16,)| {
    let addr_string = ("127.0.0.1", port);
    world.sockets.remove(&port);
    UdpSocket::bind(addr_string)
      .map(|socket| world.sockets.insert(port, socket))
      .map(|_| InvokeResponse::Success)
      .unwrap_or_else(|err| InvokeResponse::fail_from_str(&format!("Could not bind socket, {:?}", err)))
  });

  When!(c, "^the normal socket on (\\d+) sends a CompleteGafferPacket to (\\d+)$", |cuke: &Cucumber<SocketWorld>, world: &mut SocketWorld, (own_port, remote_port): (u16, u16)| {
    cuke.invoke(&format!("the normal socket on {} sends a payload to {} matching:", own_port, remote_port), world, Some(InvokeArgument::Table(vec![vec![]])))
  });


  When!(c, "^the normal socket on (\\d+) sends a CompleteGafferPacket to (\\d+) matching:$", |_, world: &mut SocketWorld, (own_port, remote_port, packet_details): (u16, u16, Vec<Vec<String>>)| {
    match CompleteGafferPacket::from_table(packet_details) {
      Err(err) => err,
      Ok(packet) => {
        world.sockets.get_mut(&own_port).ok_or(InvokeResponse::fail_from_str("No socket at that port"))
          .and_then(|socket| {
            socket.send_to(packet.serialized().as_ref(), ("127.0.0.1", remote_port))
              .map_err(|_| InvokeResponse::fail_from_str("Could not send packet"))
          })
          .map(|_| InvokeResponse::Success)
          .unwrap_or_else(|v| v)
      }
    }
  });

  Then!(c, "^the normal socket on (\\d+) receives a CompleteGafferPacket from (\\d+)$", |_, world: &mut SocketWorld, (own_port, remote_port): (u16, u16)|{
    let mut buffer = [0; 1024];
    world.sockets.get_mut(&own_port).ok_or(InvokeResponse::fail_from_str("No socket at that port"))
      .and_then(|socket| {
        socket.recv_from(&mut buffer)
         .map_err(|_| InvokeResponse::fail_from_str("Could not receive packet"))
      })
      .map(|(_, source)| {
        let addr = ("127.0.0.1", remote_port).to_single_socket_addr().unwrap();
        InvokeResponse::check_eq(source, addr)
      })
      .unwrap_or_else(|v| v)
  });

  Then!(c, "^the normal socket on (\\d+) receives a CompleteGafferPacket from (\\d+) matching:$", |_, world: &mut SocketWorld, (own_port, remote_port, packet_details): (u16, u16, Vec<Vec<String>>)|{
    match CompleteGafferPacket::from_table(packet_details) {
      Err(err) => err,
      Ok(packet) => {
        let mut buffer = [0; 1024];
        world.sockets.get_mut(&own_port).ok_or(InvokeResponse::fail_from_str("No socket at that port"))
          .and_then(|socket| {
            socket.recv_from(&mut buffer)
             .map_err(|_| InvokeResponse::fail_from_str("Could not receive packet"))
          })
          .and_then(|(_, source)| {
            if source != ("127.0.0.1", remote_port).to_single_socket_addr().unwrap() {
              Err(InvokeResponse::fail_from_str("Packet did not come from expected source"))
            } else {
              CompleteGafferPacket::deserialize(buffer.to_vec()).map_err(|_| {
                InvokeResponse::fail_from_str("Could not deserialize packet")
              })
            }
          })
          .map(|recv_packet| {
            InvokeResponse::check_eq(packet, recv_packet)
          })
          .unwrap_or_else(|v| v)
      }
    }
  })
}
