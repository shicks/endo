use dna::{DnaState, State, crc, iterate};
use rope::Rope;
use base::{Base, BaseLike, Join};
use std::io::{BufReader, Read};
use std::fs::File;
use flate2::read::GzDecoder;

fn main() {
  let file = File::open("endo.dna.gz").unwrap();
  let reader = BufReader::new(file);
  let mut decoder = GzDecoder::new(reader);
  let mut endo_dna: String = String::new();
  decoder.read_to_string(&mut endo_dna).unwrap();

  let mut dna = Base::collect::<Rope<_>>(&endo_dna);
  let mut state = DnaState::new();
  let mut i = 0;
  while !state.finished() {
    i += 1;
    // if i % 5000 == 0 {
    //   println!("Iteration {}: {} bases, depth {}", i, dna.len(), dna.dep());
    // }
//println!("{}", Join(&dna.iter().take(320).collect::<Vec<_>>(), ""));
    iterate(&mut dna, &mut state);
    if state.finished() { break; }
  }
  println!("Finished {} iterations, {} RNA", i, state.rna().len());
}
