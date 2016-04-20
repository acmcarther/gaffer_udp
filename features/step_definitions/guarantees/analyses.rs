use super::application::{
  EchoChallenger,
  EchoChallengee,
  Challenger,
  Challengee,
  EchoChallenge
};

trait NetworkValidation {
  fn reliability(&self) -> f32;
}

impl NetworkValidation for EchoChallenger {
  fn reliability(&self) -> f32 {
    let tot = (0..self.domains)
      .fold(0.0, |sum, idx| sum + self.answer_state.get(&idx).unwrap().len() as f32 / self.problem_state.get(&idx).unwrap().clone() as f32);

    tot as f32 / self.domains as f32
  }
}
