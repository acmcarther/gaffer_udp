use std::collections::{HashMap, HashSet};

trait Challengee {
  type Challenge;
  type Response;

  fn answer_challenge(&mut self, challenge: Self::Challenge) -> Self::Response;
}


trait Challenger {
  type Challenge;
  type Response;

  fn generate_challenge(&mut self, domain: u32) -> Self::Challenge;
  fn accept_response(&mut self, r: Self::Response);
}


struct EchoChallengee {
  domains: u32,
  problem_state: HashMap<u32, u32>,
}

impl EchoChallengee {

  fn new(domain_count: u32) -> EchoChallengee {
    EchoChallengee {
      domains: domain_count,
      problem_state: HashMap::new(),
    }
  }
}

impl Challengee for EchoChallengee {
  type Challenge = EchoChallenge;
  type Response = Option<(EchoChallenge, u32)>;

  fn answer_challenge(&mut self, c: EchoChallenge) -> Option<(EchoChallenge, u32)> {
    match self.problem_state.get_mut(&c.domain) {
      Some(last_id) => {
        if *last_id >= c.id {
          return None;
        } else {
          *last_id = c.id;
          return Some((c.clone(), c.value));
        }
      },
      _ => ()
    };

    // sidestepping minor issue with lifetime of problem_state
    self.problem_state.insert(c.domain, c.id);
    return Some((c.clone(), c.value));
  }
}

#[derive(Clone, Eq, PartialEq)]
struct EchoChallenge {
  pub domain: u32,
  pub id: u32,
  pub value: u32
}

struct EchoChallenger {
  domains: u32,
  problem_state: HashMap<u32, u32>,
  answer_state: HashMap<u32, HashSet<u32>>
}

impl EchoChallenger {
  fn new(domain_count: u32) -> EchoChallenger {
    EchoChallenger {
      domains: domain_count,
      problem_state: HashMap::new(),
      answer_state: HashMap::new(),
    }
  }
}

impl Challenger for EchoChallenger {
  type Challenge = EchoChallenge;
  type Response = Option<(EchoChallenge, u32)>;

  fn generate_challenge(&mut self, domain: u32) -> EchoChallenge {
    match self.problem_state.get_mut(&domain) {
      Some(id) => {
        *id = *id + 1;
        return EchoChallenge {domain: domain, id: id.clone(), value: 1}; // TODO: RNG
      },
      _ => ()
    };

    // sidestepping minor issue with lifetime of problem_state
    self.problem_state.insert(domain, 0);
    return EchoChallenge {domain: domain, id: 0, value: 1}; // TODO: RNG
  }

  fn accept_response(&mut self, r: Option<(EchoChallenge, u32)>) {
    match r {
      None => (),
      Some((challenge, _)) => {
        match self.answer_state.get_mut(&challenge.domain) {
          Some(hashset) => {
            hashset.insert(challenge.id);
            return;
          }
          None => ()
        };
        // sidestepping minor issue with lifetime of problem_state
        let mut set = HashSet::new();
        set.insert(challenge.id);
        self.answer_state.insert(challenge.domain, set);
      }
    }
  }
}
