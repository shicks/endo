use dna::{DnaState, State, iterate};
use rope::Rope;
use base::{Base, BaseLike};

fn main() {
  let mut dna = Base::collect::<Rope<_>>("IIPIPICPIICICIIFICCIFCCCPPIICCFPC");
  let mut state = DnaState::new();
  iterate(&mut dna, &mut state);
}
