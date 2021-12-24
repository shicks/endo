#[macro_use]
extern crate lazy_static;

use std::cmp::max;
use std::collections::BTreeMap;
use std::fmt;
use std::mem;
use std::str::FromStr;
use rope::*;
use base::{Base, BaseLike, Join};

// SourceMap:
//  - keep track of when a base is used as a PItem, a TItem, an Emit,
//    matches a PItem, etc; and also the escape level.
//  - map back to original address...?
//  - output: 00000  III PIIPIIP   0  emit PIIPIIP     [0,0]
//            00010  IF            0  (
//            00012  CPFIC        -1  IFCP
//            00017  IIC           0  )
//            00020  IIC           0  endpat
//            ...
//            00100  IF IICICCIP   0  !2398
//
// We're gonna end up with mixed-and-matched numbers on different skip
// bases, inserted from various places... how to represent this?

type Rna<T> = [T;7];

pub fn str<T: BaseLike>(dna: &Rope<T>) -> String {
  dna.iter().map(|b| format!("{}", b)).collect::<String>()
}

#[derive(Clone, Debug, PartialEq)]
pub enum PItem<T: BaseLike> {
  Bases(Vec<T>),
  // Also stores the index where we found it, for debugging...?
  Skip(usize),
  Search(Vec<T>),
  OpenGroup,
  CloseGroup,
}

impl<T: BaseLike> fmt::Display for PItem<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      PItem::Bases(v) => write!(f, "{}", Join(v, "")),
      PItem::Skip(i) => write!(f, "!{}", i),
      PItem::Search(v) => write!(f, "?<{}>", Join(v, "")),
      PItem::OpenGroup => write!(f, "("),
      PItem::CloseGroup => write!(f, ")"),
    }
  }
}

impl<T: BaseLike> FromStr for PItem<T> {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let v = s.as_bytes();
    match (v[0], v.len()) {
      (b'(', 1) => Ok(PItem::OpenGroup),
      (b')', 1) => Ok(PItem::CloseGroup),
      (b'I', ..)|(b'C', ..)|(b'F', ..)|(b'P', ..) => {
        Ok(PItem::Bases(T::collect_from(s)))
      }
      (b'!', _) => match s[1..].parse::<usize>() {
        Ok(i) => Ok(PItem::Skip(i)),
        Err(_) => Err(()),
      },
      (b'?', _) if v[1] == b'<' && v[v.len() - 1] == b'>' =>
          Ok(PItem::Search(T::collect_from(&s[2..(v.len()-1)]))),
      _ => Err(()),
    }
  }
}

impl<T: BaseLike> Pattern<T> for PItem<T> {
  fn exec<S: BaseLike>(&self, cursor: &mut RopeCursor<S>, env: &mut Env) -> bool {
    match self {
      PItem::OpenGroup => {
        env.starts.push(cursor.pos());
      }
      PItem::CloseGroup => {
        env.groups.push((env.starts.pop().unwrap(), cursor.pos()));
      }
      PItem::Bases(bases) => {
        for b in bases {
          if cursor.next().map(BaseLike::to_base) != Some(b.to_base()) {
            return false;
          }
        }
      }
      PItem::Skip(i) => {
        if cursor.pos() + i > cursor.full_len() { return false; }
        cursor.skip(*i as isize);
      }
      PItem::Search(bs, ..) => {
        match find(cursor, &bs, cursor.pos()) {
          Some(index) => { cursor.seek(index + bs.len()); }
          None => { return false; }
        }
      }
    }
    true
  }

  fn make_bases<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Self {
    let bases: Vec<T> = Bases::parse(cursor);
    if T::HAS_SOURCE {
      for base in bases.iter() {
        state.record_pat_base(*base);
      }
    }
    PItem::Bases(bases)
  }

  fn make_skip<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Option<Self> {
    cursor.skip(2);
    state.record_num(cursor);
    usize::parse(cursor).map(PItem::Skip)
  }

  fn make_search<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Self {
    cursor.skip(3);
    let bases: Vec<T> = Bases::parse(cursor);
    if T::HAS_SOURCE {
      for base in bases.iter() {
        state.record_search_base(*base);
      }
    }
    PItem::Search(bases)
  }

  fn make_open(cursor: &mut RopeCursor<T>) -> Self {
    cursor.skip(3);
    PItem::OpenGroup
  }

  fn make_close(cursor: &mut RopeCursor<T>) -> Self {
    cursor.skip(3);
    PItem::CloseGroup
  }
}


#[derive(Clone, Debug, PartialEq)]
pub enum TItem<T: BaseLike> {
  Bases(Vec<T>),
  // Also stores the index where we found it, for debugging...?
  Len(usize),
  Ref{group: usize, level: usize},
}

impl<T: BaseLike> fmt::Display for TItem<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      TItem::Bases(v) => write!(f, "{}", Join(v, "")),
      TItem::Len(i) => write!(f, "|{}|", i),
      TItem::Ref{group, level} => {
        if *level < 5 {
          write!(f, "${}{}", "\\".repeat(*level as usize), group)
        } else {
          write!(f, "${}\\{}", level, group)
        }
      }
    }
  }
}

impl<T: BaseLike> FromStr for TItem<T> {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let v = s.as_bytes();
    match (v[0], v.len()) {
      (b'I', ..)|(b'C', ..)|(b'F', ..)|(b'P', ..) => {
        Ok(TItem::Bases(T::collect_from(s)))
      }
      (b'|', _) if v[v.len() - 1] == b'|' => {
        match s[1..v.len()-1].parse::<usize>() {
          Ok(i) => Ok(TItem::Len(i)),
          Err(_) => Err(()),
        }
      },
      (b'$', _) => {
        // TODO - parse $6\0 format?
        let mut level: usize = 0;
        while v[1 + level as usize] == b'\\' {
          level += 1;
        }
        match s[level as usize..].parse::<usize>() {
          Ok(group) => Ok(TItem::Ref{group, level}),
          Err(_) => Err(()),
        }
      },
      _ => Err(()),
    }
  }
}

fn as_nat<T: BaseLike>(mut i: usize) -> Vec<T> {
  let mut v: Vec<T> = vec![];
  while i > 0 {
    // TODO - keep address from op, set level to -32
    v.push(match i & 1 {
      0 => T::from_parts(Base::I, 0, -32),
      1 => T::from_parts(Base::C, 0, -32),
      _ => unreachable!(),
    });
    i >>= 1;
  }
  v.push(T::from_parts(Base::P, 0, -32));
  v
}

impl<T: BaseLike> Template<T> for TItem<T> {
  fn expand(&self, out: &mut Vec<T>, env: &[(usize, usize)],
            cursor: &mut RopeCursor<T>) {
    match self {
      TItem::Bases(v) => {
        out.extend(v);
      }
      TItem::Len(i) => {
        if *i < env.len() {
          out.extend(as_nat::<T>(env[*i].1 - env[*i].0));
        } else {
          out.push(T::from_base(Base::P));
        }
      }
      TItem::Ref{group, level} => {
        if *group < env.len() {
          for i in env[*group].0 .. env[*group].1 {
            cursor.at(i).protect(*level as u8, out);
          }
        }
      }
    }
  }

  // This is necessary for finding splice points.
  fn as_unprotected_group(&self) -> Option<usize> {
    match self {
      TItem::Ref{group, level} if *level == 0 => Some(*group as usize),
      _ => None,
    }
  }


  fn make_bases(cursor: &mut RopeCursor<T>) -> Self {
    TItem::Bases(Bases::parse(cursor))
  }

  fn make_len<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Option<Self> {
    cursor.skip(3);
    state.record_num(cursor);
    usize::parse(cursor).map(TItem::Len)
  }

  fn make_ref<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Option<Self> {
    cursor.skip(2);
    state.record_num(cursor);
    if let Some(level) = usize::parse(cursor) {
      state.record_num(cursor);
      if let Some(group) = usize::parse(cursor) {
        return Some(TItem::Ref{group, level});
      }
    }
    None
  }
}

#[repr(u8)]
enum OpCode {
  C,
  F,
  P,
  IC,
  IF,
  IP,
  IIC,
  IIF,
  IIP,
  III,
  Invalid,
}

fn next_op<T: BaseLike>(cursor: &mut RopeCursor<T>) -> OpCode {
  let i = cursor.pos();
  match cursor.try_at(i).map(BaseLike::to_base) {
    None => OpCode::Invalid,
    Some(Base::C) => OpCode::C,
    Some(Base::F) => OpCode::F,
    Some(Base::P) => OpCode::P,
    Some(Base::I) => match cursor.try_at(i + 1).map(BaseLike::to_base) {
      None => OpCode::Invalid,
      Some(Base::C) => OpCode::IC,
      Some(Base::F) => OpCode::IF,
      Some(Base::P) => OpCode::IP,
      Some(Base::I) => match cursor.try_at(i + 2).map(BaseLike::to_base) {
        None => OpCode::Invalid,
        Some(Base::C) => OpCode::IIC,
        Some(Base::F) => OpCode::IIF,
        Some(Base::P) => OpCode::IIP,
        Some(Base::I) => OpCode::III,
      }
    }      
  }
}


pub struct Env {
  starts: Vec<usize>,
  groups: Vec<(usize, usize)>,
}



pub trait Num<T: BaseLike>: Sized {
  // None means finish
  fn parse(cursor: &mut RopeCursor<T>) -> Option<Self>;
}
pub trait Bases<T: BaseLike>: Sized {
  fn parse(cursor: &mut RopeCursor<T>) -> Self;
}

impl<T: BaseLike> Num<T> for usize {
  fn parse(cursor: &mut RopeCursor<T>) -> Option<Self> {
    let mut v: usize = 0;
    let mut mask: usize = 1;
    while let Some(base) = cursor.next() {
      match base.to_base() {
        Base::C => { v |= mask; }
        Base::P => { return Some(v); }
        _ => {}
      }
      mask <<= 1;
    }
    None
  }
}

impl<T: BaseLike> Bases<T> for Vec<T> {
  fn parse(cursor: &mut RopeCursor<T>) -> Self {
    let mut v: Vec<T> = vec![];
    loop {
      let p = cursor.peek();
      if p.is_none() { return v; }
      if p.unwrap().to_base() == Base::I {
        if cursor.try_at(cursor.pos() + 1).map(BaseLike::to_base) == Some(Base::C) {
          v.push(p.unwrap().unprotect());
          cursor.skip(2);
        } else {
          return v;
        }
      } else {
        v.push(p.unwrap().unprotect());
        cursor.skip(1);
      }
    }      
  }
}

pub trait State<T: BaseLike> {
  fn new() -> Self;

  fn emit(&mut self, cursor: &mut RopeCursor<T>);
  fn finish(&mut self);

  fn finished(&self) -> bool;
  fn rna(&self) -> &[Rna<T>];

  #[inline]
  fn or_finish<U>(&mut self, o: Option<U>) -> Option<U> {
    if o.is_none() { self.finish(); }
    o
  }

  fn iterate(&mut self, dna: &mut Rope<T>);

  #[inline]
  fn record_splice(&mut self, _dna: &Rope<T>, _pos: u32) {}
  #[inline]
  fn record_usage(&mut self, _cursor: &mut RopeCursor<T>,
                  _pos: u32, _usage: Usage) {}
  #[inline]
  fn record_num(&mut self, _cursor: &mut RopeCursor<T>) {}
  #[inline]
  fn record_pat_base(&mut self, _base: T) {}
  #[inline]
  fn record_search_base(&mut self, _base: T) {}
  // #[inline]
  // fn record_bases(bases: Vec<T>) -> Vec<T> { bases }
  // TODO - how to _derive_ a DebugState from DnaState?
  //  - include Pattern and Template in state??? rename to machine?
}

#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Usage {
  PatBaseI,
  PatBaseC,
  PatBaseF,
  PatBaseP,
  PatSkip,
  PatSearch,
  PatOpen,
  PatClose,
  PatEnd,
  // TplBase omitted because the unescaped version is more important
  TplLen,
  TplRef,
  TplEnd,
  Num0,
  Num1,
  NumP,
  SearchBaseI,
  SearchBaseC,
  SearchBaseF,
  SearchBaseP,
  Rna,
  RnaBaseI,
  RnaBaseC,
  RnaBaseF,
  RnaBaseP,
}

impl Usage {
  fn rna_base<T: BaseLike>(base: T) -> Self {
    unsafe {
      mem::transmute::<u8, Usage>(base.to_u2() + Usage::RnaBaseI as u8)
    }
  }
  fn pat_base<T: BaseLike>(base: T) -> Self {
    unsafe {
      mem::transmute::<u8, Usage>(base.to_u2() + Usage::PatBaseI as u8)
    }
  }
  fn search_base<T: BaseLike>(base: T) -> Self {
    unsafe {
      mem::transmute::<u8, Usage>(base.to_u2() + Usage::SearchBaseI as u8)
    }
  }
}

pub struct DnaState<T: BaseLike> {
  pub print: bool,
  pub print_verbose: bool,
  pub iters: u32,
  //pub coverage: Option<BTreeMap<usize, CoverageStat>>,
  pub coverage: BTreeMap<(usize, i8), Stat>,
  finished: bool,
  rna: Vec<Rna<T>>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Stat {
  pub splice: bool,
  pub usage: Option<Usage>,
  pub count: u32,
  pub first: u32,
  pub last: u32,
}

impl Stat {
  fn new() -> Self {
    Stat{splice: false, usage: None, count: 0, first: u32::MAX, last: 0}
  }
  fn record_usage(&mut self, iter: u32, usage: Usage) {
    self.usage = Some(usage);
    if self.first > self.last { self.first = iter; }
    self.last = iter;
    self.count += 1;
  }
  fn record_splice(&mut self) {
    self.splice = true;
  }
}

fn dump_num(coverage: &BTreeMap<(usize, i8), Stat>, addr: usize, lvl: i8)
            -> Option<(usize, Vec<(usize, i8)>, usize)> {
  let mut i = addr;
  let mut v = 0;
  let mut mask: usize = 1;
  let mut used = vec![];
  while let Some(stat) = coverage.get(&(i, lvl)) {
    // Look for a number - how to parse reasonably?
    used.push((i, lvl));
    match stat.usage {
      Some(Usage::NumP) => { return Some((v, used, addr + 1)); }
      Some(Usage::Num0) => { }
      Some(Usage::Num1) => { v |= mask; }
      _ => { return None; }
    }
    mask <<= 1;
    i += 1;
  }
  None
}

impl<T: BaseLike> DnaState<T> {
  pub fn source_dump(&self, addr: usize, lvl: i8) -> (String, Vec<(usize, i8)>) {
    let mut seen = vec![(addr, lvl)];
    let mut s = String::new();
    if let Some(stat) = self.coverage.get(&(addr, lvl)) {
      if stat.usage.is_none() { return (s, seen); }
      match stat.usage.unwrap() {
        Usage::PatBaseI|Usage::PatBaseC|Usage::PatBaseF|Usage::PatBaseP => {
          s.push(Base::from_u8(stat.usage.unwrap() as u8
                               - Usage::PatBaseI as u8).char());
          let mut skipped = 0;
          for i in (addr + 1) .. {
            let u = self.coverage.get(&(i, lvl)).and_then(|x| x.usage);
            match u {
              Some(Usage::PatBaseI)|Some(Usage::PatBaseC)|
              Some(Usage::PatBaseF)|Some(Usage::PatBaseP) => {
                skipped = 0;
                seen.push((i, lvl));
                s.push(Base::from_u8(u.unwrap() as u8
                                     - Usage::PatBaseI as u8).char());
              }
              None if skipped < 2 => { skipped += 1; }
              _ => { break; }
            }
          }
        }
        Usage::PatSkip => {
          if let Some((num, used, _)) = dump_num(&self.coverage, addr + 2, lvl) {
            s.push_str(&format!("!{}", num));
            seen.extend(used);
          } else {
            s.push_str("skip");
          }
        }
        Usage::PatSearch => {
          s.push_str("?<");
          let mut skipped = 0;
          for i in (addr + 3) .. {
            let u = self.coverage.get(&(i, lvl)).and_then(|x| x.usage);
            match u {
              Some(Usage::SearchBaseI)|Some(Usage::SearchBaseC)|
              Some(Usage::SearchBaseF)|Some(Usage::SearchBaseP) => {
                skipped = 0;
                seen.push((i, lvl));
                s.push(Base::from_u8(u.unwrap() as u8
                                     - Usage::SearchBaseI as u8).char());
              }
              None if skipped < 2 => { skipped += 1; }
              _ => { break; }
            }
          }
          s.push_str(">");
        }
        Usage::PatOpen => { s.push('('); }
        Usage::PatClose => { s.push(')'); }
        Usage::PatEnd => { s.push_str("endpat"); }
        Usage::TplLen => {
          if let Some((num, used, _)) = dump_num(&self.coverage, addr + 3, lvl) {
            s.push_str(&format!("|{}|", num));
            seen.extend(used);
          } else {
            s.push_str("len");
          }
        }
        Usage::TplRef => {
          if let Some((esc, used1, a)) = dump_num(&self.coverage, addr + 2, lvl) {
            if let Some((grp, used2, _)) = dump_num(&self.coverage, a, lvl) {
              if lvl < 5 {
                s.push_str(&format!("${}{}", "\\".repeat(esc), grp));
              } else {
                s.push_str(&format!("${}\\{}", esc, grp));
              }
              seen.extend(used1);
              seen.extend(used2);
            } else {
              s.push_str("ref");
            }
          } else {
            s.push_str("ref");
          }
        }
        Usage::TplEnd => { s.push_str("endtpl"); }
        Usage::Num0|Usage::Num1|Usage::NumP => {
          if let Some((num, used, _)) = dump_num(&self.coverage, addr, lvl) {
            s.push_str(&format!("{}", num));
            seen.extend(used);
          } else {
            s.push_str("num");
          }          
        }
        Usage::SearchBaseI|Usage::SearchBaseC|
        Usage::SearchBaseF|Usage::SearchBaseP => {
          // skip more?
          s.push_str("search base");
        }
        Usage::RnaBaseI|Usage::RnaBaseC|
        Usage::RnaBaseF|Usage::RnaBaseP => {
          // skip more?
          s.push_str("rna base");
        }
        // What about stray search bases???
        Usage::Rna => {
          s.push_str("rna ");
          let mut skipped = 0;
          for i in (addr + 3) .. {
            let u = self.coverage.get(&(i, lvl)).and_then(|x| x.usage);
            match u {
              Some(Usage::RnaBaseI)|Some(Usage::RnaBaseC)|
              Some(Usage::RnaBaseF)|Some(Usage::RnaBaseP) => {
                skipped = 0;
                seen.push((i, lvl));
                s.push(Base::from_u8(u.unwrap() as u8
                                     - Usage::RnaBaseI as u8).char());
              }
              None if skipped < 2 => { skipped += 1; }
              _ => { break; }
            }
          }
        }
      }
    }
    (s, seen)
  }
}

// TODO -
//  1. tie this deeper into BaseLike, along with PItem/TItem parsing?
//  2. simplify a bit - just keep track of
//      a. when parsing a [PT]Item: address -> level,op/num/base; iter
//      b. when splicing: where are splice points?
//         do we distinguish from in/out? (pat/tpl?)
//         - use insertion points (0..len) on _both_ sides...?
// pub struct CoverageStat {
//   first: u32,
//   last: u32,
//   count: u32,
//   pat_splices: HashSet<SpliceStat>,
//   tpl_splices: HashSet<SpliceStat>,
// }

// #[derive(PartialEq, Eq, Hash)]
// pub struct SpliceStat {
//   source_len: usize,
//   actual_len: usize,
//   splices: Vec<(usize, usize, usize)>, // start, source len, insert len
//   levels: Vec<i8>,
// }

impl<T: BaseLike> State<T> for DnaState<T> {
  fn new() -> Self {
    DnaState{finished: false, rna: Vec::new(),
             print: false, print_verbose: false, iters: 0,
             coverage: BTreeMap::new(),
    }
  }
  fn emit(&mut self, c: &mut RopeCursor<T>) {
    let i = c.pos() + 3;
    c.skip(10);
    if c.at_end() { return; }
    let rna = [c.at(i), c.at(i + 1), c.at(i + 2), c.at(i + 3),
               c.at(i + 4), c.at(i + 5), c.at(i + 6)];
    self.record_rna(rna);
    self.rna.push(rna);
    let rna_str = &rna.map(|b| b.to_base().char()).iter().collect::<String>();
    if self.print {
      if self.print_verbose {
        let addr = match (rna[0].addr(), rna[0].level()) {
          (Some(a), Some(l)) if l == 0 => format!(" @{}", a),
          (Some(a), Some(l)) => format!(" @{} \\{}", a, l),
          _ => String::new(),
        };
        println!("{} # iter {}{}", rna_str, self.iters, addr);
      } else {
        println!("{}", rna_str);
      }
    }
  }
  fn finish(&mut self) {
    self.finished = true;
  }
  fn finished(&self) -> bool {
    self.finished
  }
  fn rna(&self) -> &[Rna<T>] {
    &self.rna
  }

  fn iterate(&mut self, dna: &mut Rope<T>) {
    // TODO - find a way to parametrize on Pattern and Template.
    //eprintln!("Iterate: {}", str(&dna));
    self.iters += 1;
    let mut cursor = dna.cursor();
    //let addr = if T::HAS_SOURCE { cursor.at(0).addr() } else { 0 };
    let pat = PItem::parse(&mut cursor, self);
    if self.finished() { return; }
    // let pattern_end = cursor.pos();

    //eprintln!("Pat: {}", Join(&pat, " "));
    let tpl = TItem::parse(&mut cursor, self);
    if self.finished() { return; }
    //eprintln!("Tpl: {}", Join(&tpl, " "));
    let template_end = cursor.pos();

    // if let Some(addr) = addr {
    //   let mut cmap = self.coverage.as_mut().unwrap();
    //   let mut entry = cmap.entry(addr).or_insert(CoverageStat{
    //     first: self.iters,
    //     last: self.iters,
    //     count: 0,
    //     pat_splices: HashSet::new(),
    //     tpl_splices: HashSet::new(),
    //   });
    //   entry.last = self.iters;
    //   entry.count += 1;
    //   entry.pat_splices.insert(this.splice_stat(dna, 0, pattern_end));
    //   entry.tpl_splices.insert(this.splice_stat(dna, pattern_end, template_end));
    // }

    match_replace(dna, &pat, &tpl, template_end, self);
  }

  fn record_splice(&mut self, dna: &Rope<T>, pos: u32) {
    if !T::HAS_SOURCE { return; }
    let pos = pos as usize;
    if pos == 0 || pos >= dna.len() { return; }
    let mut c = dna.cursor();
    let left = c.at(pos - 1);
    let right = c.at(pos);
    let left_addr = left.addr().unwrap() as usize;
    let left_level = left.level().unwrap();
    let right_addr = right.addr().unwrap() as usize;
    let right_level = right.level().unwrap();
    if left_level != -32 {
      self.coverage.entry((left_addr, left_level))
          .or_insert_with(Stat::new).record_splice();
    }
    if right_level != -32 {
      self.coverage.entry((right_addr, right_level))
          .or_insert_with(Stat::new).record_splice();
    }
  }
  fn record_usage(&mut self, cursor: &mut RopeCursor<T>,
                  pos: u32, usage: Usage) {
    if !T::HAS_SOURCE { return; }
    let pos = pos as usize;
    if pos >= cursor.full_len() { return; }
    let base = cursor.at(pos);
    let addr = base.addr().unwrap();
    let level = base.level().unwrap();
    self.record(addr, level, usage);
  }
  fn record_num(&mut self, cursor: &mut RopeCursor<T>) {
    if !T::HAS_SOURCE { return; }
    for pos in cursor.pos() .. cursor.full_len() {
      let base = cursor.at(pos).to_u2();
      if base == 3 {
        self.record_usage(cursor, pos as u32, Usage::NumP);
        break;
      } else {
        self.record_usage(cursor, pos as u32,
                          if base == 1 { Usage::Num1 } else { Usage::Num0 });
      }
    }
  }

  fn record_pat_base(&mut self, base: T) {
    if !T::HAS_SOURCE { return; }
    let addr = base.addr().unwrap();
    let level = base.level().unwrap();
    self.record(addr, level, Usage::pat_base(base));
  }

  fn record_search_base(&mut self, base: T) {
    if !T::HAS_SOURCE { return; }
    let addr = base.addr().unwrap();
    let level = base.level().unwrap();
    self.record(addr, level, Usage::search_base(base));
  }
}

impl<T: BaseLike> DnaState<T> {
  fn record(&mut self, addr: u32, level: i8, usage: Usage) {
    if level != -32 {
      self.coverage.entry((addr as usize, level))
          .or_insert_with(Stat::new)
          .record_usage(self.iters, usage);
    }
  }

  fn record_rna(&mut self, bases: [T;7]) {
    if !T::HAS_SOURCE { return; }
    for base in bases {
      let addr = base.addr().unwrap();
      let level = base.level().unwrap();
      self.record(addr, level, Usage::rna_base(base));
    }
  }

//   fn splice_stat(dna: Rope<T>, start: usize, end: usize) -> SpliceStat {
//     let mut pat_splices = Vec::new();
//     let mut pat_source_len = 0;

//   }
//       let mut levels = BTreeSet::new();
//       for i in 0..pat_length {
        
//       }
//       let mut pat_splice = SpliceStat{
//         actual_len: pattern_end,
//         source_len: 
}

fn match_replace<T: BaseLike, S: State<T>>(dna: &mut Rope<T>, pat: &[PItem<T>],
                                           tpl: &[TItem<T>], start: usize,
                                           state: &mut S) {
  let mut cursor = dna.cursor();
  cursor.seek(start);
  let mut env = Env{starts: vec![], groups: vec![]};
  for p in pat {
    if !p.exec(&mut cursor, &mut env) {
      dna.splice(0, start, None);
//eprintln!("No match: splicing to {}", str(&dna));
//eprintln!("No match: splicing {}", start);
      return;
    }
  }
//eprintln!("Matched {} bases", cursor.pos() - start);
  let env = env.groups;
  let splice_plan = find_splice(tpl, &env, (0, cursor.pos()));
  let splices = splice_plan.iter()
    .map(|(r, t)| {
      let mut v: Vec<T> = Vec::new();
      for item in *t {
        item.expand(&mut v, &env, &mut cursor);
      }
      (r, v)
    }).collect::<Vec<_>>();
// TODO - still need to verify that this is correct
//eprintln!("Splices: {:?}", splices);
  for ((start, end), bases) in splices {
    let len = bases.len();
    let insert = if len > 0 { Some(bases) } else { None };
    dna.splice(*start, end - start, insert);
    state.record_splice(dna, *start as u32);
    state.record_splice(dna, (start + len) as u32);
  }
}

pub trait Pattern<T: BaseLike>: Sized {
  fn exec<S: BaseLike>(&self, cursor: &mut RopeCursor<S>, env: &mut Env) -> bool;
  fn make_bases<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Self;
  fn make_skip<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Option<Self>;
  fn make_search<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Self;
  fn make_open(cursor: &mut RopeCursor<T>) -> Self;
  fn make_close(cursor: &mut RopeCursor<T>) -> Self;
  fn parse_item<S: State<T>>(cursor: &mut RopeCursor<T>, depth: &mut usize,
                        state: &mut S) -> Option<Self> {
    let next = next_op(cursor);
    match next {
      OpCode::Invalid => { state.finish(); None }
      OpCode::C|OpCode::F|OpCode::P|OpCode::IC => {
        Some(Self::make_bases(cursor, state))
      }
      OpCode::IF => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::PatSearch);
        Some(Self::make_search(cursor, state))
      }
      OpCode::IP => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::PatSkip);
        let item = Self::make_skip(cursor, state);
        state.or_finish(item)
      }
      OpCode::IIC|OpCode::IIF => {
        if *depth == 0 {
          state.record_usage(cursor, cursor.pos() as u32, Usage::PatEnd);
          cursor.skip(3);
          None
        } else {
          state.record_usage(cursor, cursor.pos() as u32, Usage::PatClose);
          *depth -= 1;
          Some(Self::make_close(cursor))
        }
      }
      OpCode::IIP => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::PatOpen);
        *depth += 1;
        Some(Self::make_open(cursor))
      }
      OpCode::III => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::Rna);
        state.emit(cursor);
        Self::parse_item(cursor, depth, state)
      }
    }
  }

  fn parse<S: State<T>>(cursor: &mut RopeCursor<T>,
                        state: &mut S) -> Vec<Self> {
    let mut v: Vec<Self> = Vec::new();
    let mut depth: usize = 0;
    while let Some(item) = Self::parse_item(cursor, &mut depth, state) {
      v.push(item);
    }
    v
  }
}



//   fn parse_from(cursor: &mut RopeCursor<T>, depth: &mut usize) -> ParseResult<Self, T>;
//   fn match_in(&self, cursor: &mut RopeCursor<T>, env: &mut Env) -> bool;
// }

pub trait Template<T: BaseLike>: Sized {
  fn expand(&self, vec: &mut Vec<T>, env: &[(usize, usize)], cursor: &mut RopeCursor<T>);
  // This is necessary for finding splice points.
  fn as_unprotected_group(&self) -> Option<usize>;

  fn make_bases(cursor: &mut RopeCursor<T>) -> Self;
  fn make_len<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Option<Self>;
  fn make_ref<S: State<T>>(cursor: &mut RopeCursor<T>, state: &mut S) -> Option<Self>;

  fn parse_item<S: State<T>>(cursor: &mut RopeCursor<T>,
                             state: &mut S) -> Option<Self> {
    let next = next_op(cursor);
    match next {
      OpCode::Invalid => { state.finish(); None }
      OpCode::C|OpCode::F|OpCode::P|OpCode::IC => {
        Some(/*state.record_bases(*/Self::make_bases(cursor)/*)*/)
      }
      OpCode::IF|OpCode::IP => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::TplRef);
        let item = Self::make_ref(cursor, state);
        state.or_finish(item)
      }
      OpCode::IIC|OpCode::IIF => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::TplEnd);
        cursor.skip(3);
        None
      }
      OpCode::IIP => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::TplLen);
        let item = Self::make_len(cursor, state);
        state.or_finish(item)
      }
      OpCode::III => {
        state.record_usage(cursor, cursor.pos() as u32, Usage::Rna);
        state.emit(cursor);
        Self::parse_item(cursor, state)
      }
    }
  }

  fn parse<S: State<T>>(cursor: &mut RopeCursor<T>,
                        state: &mut S) -> Vec<Self> {
    let mut v: Vec<Self> = Vec::new();
    while let Some(item) = Self::parse_item(cursor, state) {
      v.push(item);
    }
    v
  }
}


type Rng = (usize, usize);
fn find_splice<'a, T: BaseLike>(tpl: &'a [TItem<T>], env: &[Rng], range: Rng)
                                -> Vec<(Rng, &'a [TItem<T>])> {

//eprintln!("find_splice: {:?} {:?}", range, env);

  let unprotected: Vec<(usize, Rng)> =
    tpl.iter().enumerate().filter_map(|(i, x)| {
      x.as_unprotected_group().and_then(|g| {
        if g < env.len() {
          Some((i, env[g]))
        } else {
          None
        }
      })
    }).collect();
  
//eprintln!("unprotected: {:?}", unprotected);

// TODO - this is a mess - pull out a struct+method?
  fn internal<'a, T: BaseLike>(rest: &[(usize, Rng)], range: Rng,
              tpl_r: (usize, usize), tpl: &'a [TItem<T>], out: &mut Vec<(Rng, &'a [TItem<T>])>) {

    // find the max in rest
//eprintln!("  internal: {:?} {:?} {:?}", range, rest, tpl_r);
    let rest: Vec<(usize, Rng)> =
        rest.iter().filter(|(_, r)| r.0 >= range.0 && r.1 <= range.1)
        .map(|x| *x).collect();
    let option_i = rest.iter().enumerate().max_by_key(|(_, (_, r))| r.1 - r.0);

    if let Some((i1, (i2, r))) = option_i {
//eprintln!("   => split {} {} {:?}", i1, i2, r);
      // split at i
      internal(&rest[i1 + 1 ..], (r.1, range.1), (i2 + 1, tpl_r.1), tpl, out);
      internal(&rest[.. i1], (range.0, r.0), (tpl_r.0, *i2), tpl, out);
    } else {
//eprintln!("   => terminal");
      out.push((range, &tpl[tpl_r.0 .. tpl_r.1]));
    }
  }
  let mut out: Vec<(Rng, &'a [TItem<T>])> = Vec::new();
  internal(&unprotected, range, (0, tpl.len()), tpl, &mut out);
  
  //out.reverse();
  out

  // This gives the "splice plan" - next, expand all the templates
  // BEFORE doing any splicing so as to not mess up the refs, then
  // splice everything RIGHT TO LEFT to keep indexes correct.
}

lazy_static!(
  static ref CRC_TABLE: [u32; 256] = make_crc_table();
);
fn make_crc_table() -> [u32; 256] {
  let mut table: [u32; 256] = [0; 256];
  for i in 0..256 {
    let mut c: u32 = i;
    for _ in 0..8 {
      c = if (c & 1) != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
    }
    table[i as usize] = c;
  }
  return table;
}
pub fn crc<T: BaseLike>(rope: &Rope<T>) -> u32 {
  let mut crc: u32 = 0xffffffff;
  let mut cursor = rope.cursor();
  while let Some(b) = cursor.next() {
    crc = (crc >> 8) ^ (*CRC_TABLE)[((crc ^ b.to_u2() as u32) & 0xff) as usize];
  }
  crc ^ 0xffffffff
}


fn find<T: BaseLike, S: BaseLike>(haystack: &mut RopeCursor<T>, needle: &[S], start: usize) -> Option<usize> {
//eprintln!("find {} from {}", Join(needle, ""), start);
  let needle_len = needle.len();
  if needle_len == 0 { return Some(start); }
  let haystack_len = haystack.full_len();
  let char_table = build_char_table(needle);
//eprintln!("char_table: {:?}", char_table);
  let offset_table = build_offset_table(needle);
//eprintln!("offset_table: {:?}", offset_table);
  let mut i = start + needle_len - 1;
  while i < haystack_len {
    let mut j = needle_len - 1 as usize;
    loop {
      let c = haystack.at(i).to_u2();
      if needle[j].to_u2() == c {
        if j == 0 { return Some(i); }
        i -= 1;
        j -= 1;
        continue;
      }
      i += max(offset_table[needle_len - 1 - j], char_table[c as usize]);
      break;
    }
  }
//eprintln!("no match");
  None
}

fn build_char_table<T: BaseLike>(needle: &[T]) -> [usize; 4] {
  let len = needle.len();
  let mut table = [len; 4];
  for i in 0 .. (len - 1) {
    table[needle[i].to_u2() as usize] = len - 1 - i;
  }
  table
}

fn build_offset_table<T: BaseLike>(needle: &[T]) -> Vec<usize> {
  let len = needle.len();
  let mut table = Vec::<usize>::with_capacity(len);
  unsafe { // SAFETY: the following for-loop fully initializes the table
    table.set_len(len);
  }
  let mut last_prefix_pos = len;
  for i in (1 ..= len).rev() {
    let mut is_prefix = true;
    for j in 0 .. len - i {
      if needle[i + j].to_u2() != needle[j].to_u2() {
        is_prefix = false;
        break;
      }
    }
    if is_prefix { last_prefix_pos = i; }
    table[len - i] = last_prefix_pos - i + len;
  }
  for i in 0 .. (len - 1) {
    let mut slen = 0;
    let mut j = len - 1;
    for ii in (0 ..= i).rev() {
      if needle[ii].to_u2() == needle[j].to_u2() {
        slen += 1;
      } else {
        break;
      }
      j -= 1;
    }
    table[slen] = len - 1 - i + slen;
  }
  table
}

#[cfg(test)]
mod dna_tests {
  use super::*;
  use quickcheck_macros::quickcheck;
  use base::{Base, SourceBase};

  #[test]
  fn find_simple() {
    let dna = SourceBase::collect_from::<Rope<_>>("ICFPIICFCPFIICICFC");
    let mut haystack = dna.cursor();
    let needle = SourceBase::collect_from::<Vec<_>>("IIC");
    assert_eq!(find(&mut haystack, &needle, 0), Some(4));
    assert_eq!(find(&mut haystack, &needle, 1), Some(4));
    assert_eq!(find(&mut haystack, &needle, 3), Some(4));
    assert_eq!(find(&mut haystack, &needle, 4), Some(4));
    assert_eq!(find(&mut haystack, &needle, 5), Some(11));
    assert_eq!(find(&mut haystack, &needle, 8), Some(11));
    assert_eq!(find(&mut haystack, &needle, 11), Some(11));
    assert_eq!(find(&mut haystack, &needle, 12), None);
    assert_eq!(find(&mut haystack, &needle, 14), None);
  }

  #[quickcheck]
  fn find_quickcheck(v: Vec<u8>, start: u64, len: u64, i: u64) {
    if v.len() == 0 { return; } // TODO - test empty?
    let haystack = v.iter().map(|x| Base::from_u8(*x)).collect_from::<Rope<Base>>();
    let start = (start % haystack.len() as u64) as usize;
    let i = (i % haystack.len() as u64) as usize;
    let len = if start < haystack.len() - 1 {
      ((len % (haystack.len() - start - 1) as u64) + 1) as usize
    } else {
      0
    };
    let haystack_str =
        haystack.iter().map(|x| format!("{}", x)).collect_from::<Vec<_>>().join("");
    let needle_str = &haystack_str[start .. start + len];
    let needle = Base::collect_from::<Vec<_>>(needle_str);
    let expected = haystack_str[i..].find(needle_str).map(|j| i + j);
    assert_eq!(find(&mut haystack.cursor(), &needle, i), expected);
  }

  #[test]
  fn parse_pattern_1() {
    let dna = SourceBase::collect_from::<Rope<_>>("CIIC");
    let mut state = DnaState::<SourceBase>::new();
    let mut c = dna.cursor();
    let pat = PItem::parse(&mut c, &mut state);
    assert_eq!(pat,
               vec![PItem::Bases(vec![
                 SourceBase::from_parts(Base::I, 0, -1)])]);
    assert_eq!(c.pos(), c.full_len());
    assert_eq!(state.finished, false);
    assert_eq!(state.rna, Vec::<[SourceBase;7]>::new());
  }

  #[test]
  fn parse_pattern_2() {
    let dna = Base::collect_from::<Rope<_>>("IIPIPICPIICICIIF");
    let mut state = DnaState::<Base>::new();
    let mut c = dna.cursor();
    let pat = PItem::parse(&mut c, &mut state);
    assert_eq!(pat,
               "( !2 ) P".split(' ').map(|s| s.parse::<PItem<Base>>().unwrap())
                   .collect_from::<Vec<_>>());
    assert_eq!(c.pos(), c.full_len());
    assert_eq!(state.finished, false);
    assert_eq!(state.rna, Vec::<[Base;7]>::new());
  }

  #[test]
  fn full_iteration_1() {
    let mut dna = Base::collect_from::<Rope<_>>("IIPIPICPIICICIIFICCIFPPIICCFPC");
    let mut state = DnaState::new();
    state.iterate(&mut dna);
    assert_eq!(&str(&dna), "PICFC");
  }

  #[test]
  fn full_iteration_2() {
    let mut dna = Base::collect_from::<Rope<_>>("IIPIPICPIICICIIFICCIFCCCPPIICCFPC");
    let mut state = DnaState::new();
    state.iterate(&mut dna);
    assert_eq!(&str(&dna), "PIICCFCFFPC");
  }

  #[test]
  fn full_iteration_3() {
    let mut dna = Base::collect_from::<Rope<_>>("IIPIPIICPIICIICCIICFCFC");
    let mut state = DnaState::new();
    state.iterate(&mut dna);
    assert_eq!(&str(&dna), "I");
  }
}
