extern crate gaffer_udp;

#[macro_use]
extern crate cucumber;

mod step_definitions;
mod support;

use step_definitions::*;

use support::env::SocketWorld;

#[test]
fn main() {
  cucumber::start(SocketWorld::new(), &[
    &socket_steps::register_steps,
    &gaffer_socket_steps::register_steps
  ]);
}
