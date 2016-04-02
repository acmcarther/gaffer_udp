use cucumber::{
  CucumberRegistrar,
  InvokeResponse,
};

use support::env::SocketWorld;

use gaffer_udp::{
  GafferSocket,
  SyncGafferSocket,
  GafferPacket,
  CompleteGafferPacket,
};

use std::str;

pub fn register_steps(c: &mut CucumberRegistrar<SocketWorld>) {
  Given!(c, "^a gaffer socket$", |_, world: &mut SocketWorld, _| {
    let _ = world.gaffer_socket.take(); // Dealloc a past socket to free the port
    world.packet_record = Vec::new();
    world.seq = 0;
    world.ack = 0;
    world.gaffer_socket = Some(SyncGafferSocket::bind(&("127.0.0.1", 9356)).unwrap());
    InvokeResponse::Success
  });

  When!(c, "^the socket sends a packet with payload \"(.*)\"$", |_, world: &mut SocketWorld, (payload,): (String,)| {
    match world.gaffer_socket {
      None => InvokeResponse::fail_from_str("No gaffer socket to send from"),
      Some(ref mut gaffer_socket) => {
        let bytes = payload.as_bytes().to_vec();
        InvokeResponse::expect(gaffer_socket.send(GafferPacket::new(("127.0.0.1", 9355), bytes)).is_ok(), "Could not send packet")
      }
    }
  });

  When!(c, "^the socket sends (\\d+) packets?$", |_, world: &mut SocketWorld, (count,): (u32,)| {
    match world.gaffer_socket {
      None => InvokeResponse::fail_from_str("No gaffer socket to send from"),
      Some(ref mut gaffer_socket) => {
        (0..count).fold(InvokeResponse::Success, |result, _| {
          if result != InvokeResponse::Success { return result; }
          InvokeResponse::expect(gaffer_socket.send(GafferPacket::new(("127.0.0.1", 9355), Vec::new())).is_ok(), "Could not send packet")
        })
      }
    }
  });

  When!(c, "^(\\d+) packets? (?:is|are) received$",|_, world: &mut SocketWorld, (count,): (u32,)| {
    // Collect to release borrow on world
    let raw_buffers = (0..count).map(|_| {
      let mut buffer = [0; 1024];
      world.socket.recv_from(&mut buffer).map(|_| buffer).map_err(|_| InvokeResponse::fail_from_str("Couldn't recv"))
    }).collect::<Vec<Result<[u8; 1024], InvokeResponse>>>();

    raw_buffers.into_iter().map(|buffer_result| {
      buffer_result.and_then(|buffer| CompleteGafferPacket::deserialize(buffer.to_vec()).map_err(|_| InvokeResponse::fail_from_str("Could not deserialize")))
    }).fold(InvokeResponse::Success, |result, item| {
      if result != InvokeResponse::Success { return result; }
      item.map(|packet| {
        if packet.seq > world.ack { world.ack = packet.seq; }
        world.packet_record.push(packet.payload);
        InvokeResponse::Success
      }).unwrap_or_else(|v| v)
    })
  });


  When!(c, "^(\\d+) packets? (?:is|are) dropped$",|_, world: &mut SocketWorld, (count,): (u32,)| {
    (0..count).map(|_| {
      let mut buffer = [0; 1024];
      world.socket.recv_from(&mut buffer).map(|_| buffer).map_err(|_| InvokeResponse::fail_from_str("Couldn't recv"))
    }).map(|buffer_result| {
      buffer_result.and_then(|buffer| CompleteGafferPacket::deserialize(buffer.to_vec()).map_err(|_| InvokeResponse::fail_from_str("Could not deserialize")))
    }).fold(InvokeResponse::Success, |result, item| {
      if result != InvokeResponse::Success { return result; }
      item.map(|_| InvokeResponse::Success).unwrap_or_else(|v| v)
    })
  });

  When!(c, "^the socket is sent (\\d+) packets?(?: to provide ack information)?$", |_, world: &mut SocketWorld, (count,): (u32,)| {
    (0..count).fold(InvokeResponse::Success, |result, _| {
      if result != InvokeResponse::Success { return result; }
      let packet = CompleteGafferPacket {
        seq: world.seq,
        ack_seq: world.ack,
        ack_field: !0,
        payload: Vec::new()
      };
      world.seq = world.seq + 1;
      world.socket.send_to(packet.serialized().as_slice(), &("127.0.0.1", 9356))
        .map_err(|_| InvokeResponse::fail_from_str("Could not send packet"))
        .and_then(|_| world.gaffer_socket.as_mut().ok_or(InvokeResponse::fail_from_str("No gaffer socket")).map(|mut socket| socket.recv()).map_err(|_|InvokeResponse::fail_from_str("Could not recv packet")))
        .map(|_| InvokeResponse::Success)
        .unwrap_or_else(|v| v)
    })
  });

  Then!(c, "^the socket's last (\\d+) payloads? include:$", |_, world: &mut SocketWorld, (count, table): (usize, Vec<Vec<String>>)| {
    if count > world.packet_record.len() { return InvokeResponse::fail_from_str("Can't validate more payloads than are recorded"); }
    let payloads_to_check: Vec<String> = world.packet_record.iter().skip(world.packet_record.len() - count)
      .map(|bytes| str::from_utf8(bytes.as_slice()).unwrap().trim_right_matches('\0').to_owned())
      .collect();

    table.into_iter().fold(InvokeResponse::Success, |result, row| {
      if result != InvokeResponse::Success { return result; }
      if row.len() != 1 {
        InvokeResponse::fail_from_str("Row should contain a single string")
      } else {
        if payloads_to_check.contains(row.get(0).unwrap()) {
          InvokeResponse::Success
        } else {
          InvokeResponse::fail_from_str(&format!("Could not find payload {}", row.get(0).unwrap()))
        }
      }
    })
  });
}
