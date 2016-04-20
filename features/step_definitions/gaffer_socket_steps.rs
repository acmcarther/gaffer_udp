use cucumber::{
  Cucumber,
  CucumberRegistrar,
  InvokeResponse,
  InvokeArgument
};

use support::env::SocketWorld;
use support::packets::FromTable;

use gaffer_udp::{
  GafferSocket,
  SimpleGafferSocket,
  GafferPacket,
  GafferPayload,
  ToSingleSocketAddr,
};

pub fn register_steps(c: &mut CucumberRegistrar<SocketWorld>) {
  Given!(c, "^a gaffer socket on (\\d+)$", |_, world: &mut SocketWorld, (port,): (u16,)| {
    let addr_string = ("127.0.0.1", port);
    world.gaffer_sockets.remove(&port);
    SimpleGafferSocket::bind(addr_string)
      .map(|socket| world.gaffer_sockets.insert(port, socket))
      .map(|_| InvokeResponse::Success)
      .unwrap_or_else(|err| InvokeResponse::fail_from_str(&format!("Could not bind socket, {:?}", err)))
  });

  When!(c, "^the gaffer socket on (\\d+) sends a payload to (\\d+)$", |cuke: &Cucumber<SocketWorld>, world: &mut SocketWorld, (own_port, remote_port): (u16, u16)| {
    cuke.invoke(&format!("the gaffer socket on {} sends a payload to {} matching:", own_port, remote_port), world, Some(InvokeArgument::Table(vec![vec![]])))
  });

  When!(c, "^the gaffer socket on (\\d+) sends a payload to (\\d+) matching:$", |_, world: &mut SocketWorld, (own_port, remote_port, payload_details): (u16, u16, Vec<Vec<String>>)| {
    match GafferPayload::from_table(payload_details) {
      Err(err) => err,
      Ok(payload) => {
        world.gaffer_sockets.get_mut(&own_port).ok_or(InvokeResponse::fail_from_str("No socket at that port"))
          .and_then(|socket| {
            let addr = ("127.0.0.1", remote_port).to_single_socket_addr().unwrap();
            let packet = GafferPacket { addr: addr, payload: payload };
            socket.send(packet)
              .map_err(|_| InvokeResponse::fail_from_str("Could not send packet"))
          })
          .map(|_| InvokeResponse::Success)
          .unwrap_or_else(|v| v)
      }
    }
  });

  Then!(c, "^the gaffer socket on (\\d+) receives a payload from (\\d+)$", |_, world: &mut SocketWorld, (own_port, remote_port): (u16, u16)|{
    world.gaffer_sockets.get_mut(&own_port).ok_or(InvokeResponse::fail_from_str("No socket at that port"))
      .and_then(|socket| {
        socket.recv()
          .map_err(|_| InvokeResponse::fail_from_str("Could not receive packet"))
      })
      .map(|recv_packet| {
        let addr = ("127.0.0.1", remote_port).to_single_socket_addr().unwrap();
        InvokeResponse::check_eq(recv_packet.addr, addr)
      })
      .unwrap_or_else(|v| v)
  });

  Then!(c, "^the gaffer socket on (\\d+) receives a payload from (\\d+) matching:$", |_, world: &mut SocketWorld, (own_port, remote_port, payload_details): (u16, u16, Vec<Vec<String>>)|{
    match GafferPayload::from_table(payload_details) {
      Err(err) => err,
      Ok(mut payload) => {
        world.gaffer_sockets.get_mut(&own_port).ok_or(InvokeResponse::fail_from_str("No socket at that port"))
          .and_then(|socket| {
            socket.recv()
              .map_err(|_| InvokeResponse::fail_from_str("Could not receive packet"))
          })
          .map(|recv_packet| {
            payload.resize(1016, 0);
            let expected_packet = GafferPacket {
              addr: ("127.0.0.1", remote_port).to_single_socket_addr().unwrap(),
              payload: payload
            };
            InvokeResponse::check_eq(expected_packet, recv_packet)
          })
          .unwrap_or_else(|v| v)
      }
    }
  })
}
