
use dna::{DnaState, State, crc, iterate};
use rope::Rope;
use base::{Base, BaseLike, Join};

use flate2::read::GzDecoder;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};

fn main() {
  let file = File::open("endo.dna.gz").unwrap();
  let reader = BufReader::new(file);
  let mut decoder = GzDecoder::new(reader);
  let mut endo_dna: String = String::new();
  decoder.read_to_string(&mut endo_dna).unwrap();

  let mut dna = Base::collect::<Rope<_>>(&endo_dna);

  let args = env::args().collect::<Vec<String>>();
  if args.len() > 1 {
    dna.splice(0, 0, Some(Base::collect::<Vec<_>>(&args[1])));
  }

  let mut state = DnaState::new();
  state.print = true;
  let mut i = 0;
  while !state.finished() {
    i += 1;
    // if i % 50000 == 0 {
    //   eprintln!("Iteration {}: {} bases, depth {}", i, dna.len(), dna.dep());
    // }
//eprintln!("{}", Join(&dna.iter().take(320).collect::<Vec<_>>(), ""));
    iterate(&mut dna, &mut state);
    if state.finished() { break; }
  }
  eprintln!("Finished {} iterations, {} RNA", i, state.rna().len());
}
