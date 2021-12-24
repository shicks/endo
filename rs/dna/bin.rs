
use base::BaseLike;
use dna::{DnaState, State};
use rope::Rope;

use flate2::read::GzDecoder;
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};

type B = base::SourceBase;

fn main() {
  let file = File::open("endo.dna.gz").unwrap();
  let reader = BufReader::new(file);
  let mut decoder = GzDecoder::new(reader);
  let mut endo_dna: String = String::new();
  decoder.read_to_string(&mut endo_dna).unwrap();

  // TODO: can we use a #define for this that can be set
  // at build time?  Or just clone the function and call
  // the appropriate one to keep optimizations?
  let mut dna = B::collect_from::<Rope<_>>(&endo_dna);

  let args = env::args().collect::<Vec<String>>();
  if args.len() > 1 {
    dna.splice(0, 0, Some(B::collect_from::<Vec<_>>(&args[1])));
  }

  let mut state = DnaState::<B>::new();
  state.print = true;
  state.print_verbose = true;
  let mut i = 0;
  while !state.finished() {
    i += 1;
//eprintln!("\nIteration {}: {} bases, depth {} CRC {}", i, dna.len(), dna.dep(), dna::crc(&dna));
    // if i % 50000 == 0 {
    //   eprintln!("Iteration {}: {} bases, depth {}", i, dna.len(), dna.dep());
    // }
//eprintln!("{}", Join(&dna.iter().take(320).collect::<Vec<_>>(), ""));
    state.iterate(&mut dna);
    if state.finished() { break; }
  }
  eprintln!("Finished {} iterations, {} RNA", i, state.rna().len());

  // Potentially we want some sort of serialization format
  // for the coverage stats?
  if B::HAS_SOURCE {

    let mut covered: HashMap<usize, BTreeSet<i8>> = HashMap::new();
    let mut splices: HashMap<usize, BTreeSet<i8>> = HashMap::new();
    for ((addr, lvl), stat) in state.coverage.iter() {
      covered.entry(*addr).or_insert_with(BTreeSet::new).insert(*lvl);
      if stat.splice {
        splices.entry(*addr).or_insert_with(BTreeSet::new).insert(*lvl);
      }
    }
    let endo_bytes = endo_dna.as_bytes();
    let chunk_start = std::cell::Cell::new(0 as usize);
    let i = std::cell::Cell::new(0 as usize);

    let print_chunk = || {
      if chunk_start.get() == i.get() { return; }
      println!("{:08} {}", chunk_start.get(),
               std::str::from_utf8(&endo_bytes[chunk_start.get()..i.get()]).unwrap());
      chunk_start.set(i.get());
    };

    while i.get() < endo_bytes.len() {
      if let Some(levels) = splices.get(&i.get()) {
        print_chunk();
        println!("--- {} ---", levels.iter().map(|x| format!("{}", x)).collect::<Vec<_>>().join(", "));
      } else if i.get() - chunk_start.get() >= 50 {
        print_chunk();
      }
      if let Some(cov_lvls) = covered.get(&i.get()) {
        // Start a new covered bit at the given levels.
        let mut to_remove = vec![];
        for lvl in cov_lvls {
          // What do we have? Gather it and any continuations at same escape level?
          let stat = state.coverage.get(&(i.get(), *lvl)).unwrap();
          if stat.usage.is_some() {
            let prefix = format!("{:08}@{} [{}..{} #{}]", i.get(), lvl, stat.first, stat.last, stat.count);
            let (suffix, used) = state.source_dump(i.get(), *lvl);
            to_remove.extend(used);
            println!("{} {}", prefix, suffix);
          }
        }
        for (a, l) in to_remove {
          if let Some(set) = covered.get_mut(&a) { set.remove(&l); }
        }
      }
      i.set(i.get() + 1);
    }
    //   // catchup missing bits
    //   if i > *addr { continue; } // ignore for now
    //   if i < *addr {
    //     let mut s = String::new();
    //     let mut start = i;
    //     while i < *addr {
    //       s.push(endo_bytes[i] as char);
    //       i += 1;
    //       if i % 50 == 0 {
    //         println!("{:08} {}", start, s);
    //         start = i;
    //         s.clear();
    //       }
    //     }
    //     if s.len() > 0 {
    //       println!("{:08} {}", start, s);
    //     }
    //   }
    //   // figure out what to print, how many bases, etc...
      
    // }
  }
}
