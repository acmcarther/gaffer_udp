use std::sync::mpsc::channel;
use std::thread;

trait Server {
}

trait Client {
}

struct Network<S: Server, C: Client> {
  packet_delay: u32,
  drop_percentage: u32,
  server: S,
  client: C
}

impl <S: Server, C: Client> Network<S, C> {
  fn simulate(&mut self) {
    let (tx, rx) = channel();

    // Start network
    thread::spawn(move || {
    })
  }
}
