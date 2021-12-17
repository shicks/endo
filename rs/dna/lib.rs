use std::cmp::max;
use std::fmt;
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

fn str<T: BaseLike>(dna: &Rope<T>) -> String {
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
        Ok(PItem::Bases(T::collect(s)))
      }
      (b'!', _) => match s[1..].parse::<usize>() {
        Ok(i) => Ok(PItem::Skip(i)),
        Err(_) => Err(()),
      },
      (b'?', _) if v[1] == b'<' && v[v.len() - 1] == b'>' =>
          Ok(PItem::Search(T::collect(&s[2..(v.len()-1)]))),
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
          Some(index) => { cursor.seek(index); }
          None => { return false; }
        }
      }
    }
    true
  }

  fn make_bases(cursor: &mut RopeCursor<T>) -> Self {
    PItem::Bases(Bases::parse(cursor))
  }

  fn make_skip(cursor: &mut RopeCursor<T>) -> Option<Self> {
    cursor.skip(2);
    usize::parse(cursor).map(PItem::Skip)
  }

  fn make_search(cursor: &mut RopeCursor<T>) -> Self {
    cursor.skip(3);
    PItem::Search(Bases::parse(cursor))
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
      TItem::Ref{group, level} =>
          write!(f, "${}{}", "\\".repeat(*level as usize), group)
    }
  }
}

impl<T: BaseLike> FromStr for TItem<T> {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let v = s.as_bytes();
    match (v[0], v.len()) {
      (b'I', ..)|(b'C', ..)|(b'F', ..)|(b'P', ..) => {
        Ok(TItem::Bases(T::collect(s)))
      }
      (b'|', _) if v[v.len() - 1] == b'|' => {
        match s[1..v.len()-1].parse::<usize>() {
          Ok(i) => Ok(TItem::Len(i)),
          Err(_) => Err(()),
        }
      },
      (b'$', _) => {
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
  let mut v = vec![T::from_base(Base::P)];
  while i > 0 {
    // TODO - keep address from op, set level to -32
    v.push(match i & 1 {
      0 => T::from_base(Base::I),
      1 => T::from_base(Base::C),
      _ => unreachable!(),
    });
    i >>= 1;    
  }
  v.reverse();
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

  fn make_len(cursor: &mut RopeCursor<T>) -> Option<Self> {
    cursor.skip(3);
    usize::parse(cursor).map(TItem::Len)
  }

  fn make_ref(cursor: &mut RopeCursor<T>) -> Option<Self> {
    cursor.skip(2);
    if let Some(level) = usize::parse(cursor) {
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
}

pub struct DnaState<T: BaseLike> {
  finished: bool,
  rna: Vec<Rna<T>>,
}

impl<T: BaseLike> State<T> for DnaState<T> {
  fn new() -> Self {
    DnaState{finished: false, rna: Vec::new()}
  }
  fn emit(&mut self, c: &mut RopeCursor<T>) {
    let i = c.pos() + 3;
    c.skip(10);
    if c.at_end() { return; }
    self.rna.push([c.at(i), c.at(i + 1), c.at(i + 2), c.at(i + 3),
                   c.at(i + 4), c.at(i + 5), c.at(i + 6)]);
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
}

pub trait Pattern<T: BaseLike>: Sized {
  fn exec<S: BaseLike>(&self, cursor: &mut RopeCursor<S>, env: &mut Env) -> bool;
  fn make_bases(cursor: &mut RopeCursor<T>) -> Self;
  fn make_skip(cursor: &mut RopeCursor<T>) -> Option<Self>;
  fn make_search(cursor: &mut RopeCursor<T>) -> Self;
  fn make_open(cursor: &mut RopeCursor<T>) -> Self;
  fn make_close(cursor: &mut RopeCursor<T>) -> Self;
  fn parse_item<S: State<T>>(cursor: &mut RopeCursor<T>, depth: &mut usize,
                        state: &mut S) -> Option<Self> {
    let next = next_op(cursor);
    match next {
      OpCode::Invalid => { state.finish(); None }
      OpCode::C|OpCode::F|OpCode::P|OpCode::IC => {
        Some(Self::make_bases(cursor))
      }
      OpCode::IF => {
        Some(Self::make_search(cursor))
      }
      OpCode::IP => {
        state.or_finish(Self::make_skip(cursor))
      }
      OpCode::IIC|OpCode::IIF => {
        if *depth == 0 {
          cursor.skip(3);
          None
        } else {
          *depth -= 1;
          Some(Self::make_close(cursor))
        }
      }
      OpCode::IIP => {
        *depth += 1;
        Some(Self::make_open(cursor))
      }
      OpCode::III => {
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
  fn make_len(cursor: &mut RopeCursor<T>) -> Option<Self>;
  fn make_ref(cursor: &mut RopeCursor<T>) -> Option<Self>;

  fn parse_item<S: State<T>>(cursor: &mut RopeCursor<T>,
                             state: &mut S) -> Option<Self> {
    let next = next_op(cursor);
    match next {
      OpCode::Invalid => { state.finish(); None }
      OpCode::C|OpCode::F|OpCode::P|OpCode::IC => {
        Some(Self::make_bases(cursor))
      }
      OpCode::IF|OpCode::IP => {
        state.or_finish(Self::make_ref(cursor))
      }
      OpCode::IIC|OpCode::IIF => {
        cursor.skip(3);
        None
      }
      OpCode::IIP => {
        state.or_finish(Self::make_len(cursor))
      }
      OpCode::III => {
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


pub fn iterate<B: BaseLike, S: State<B>>(dna: &mut Rope<B>, state: &mut S) {
  // TODO - find a way to parametrize on Pattern and Template.
println!("Iterate: {}", str(&dna));
  let mut cursor = dna.cursor();
  let pat = PItem::parse(&mut cursor, state);
  if state.finished() { return; }
println!("Pat: {}", Join(&pat, " "));
  let tpl = TItem::parse(&mut cursor, state);
  if state.finished() { return; }
println!("Tpl: {}", Join(&tpl, " "));
  let start = cursor.pos();
  match_replace(dna, &pat, &tpl, start);
}

fn match_replace<T: BaseLike>(dna: &mut Rope<T>, pat: &[PItem<T>],
                              tpl: &[TItem<T>], start: usize) {
  let mut cursor = dna.cursor();
  cursor.seek(start);
  let mut env = Env{starts: vec![], groups: vec![]};
  for p in pat {
    if !p.exec(&mut cursor, &mut env) {
      dna.splice(0, start, None);
println!("No match: splicing to {}", str(&dna));
      return;
    }
  }
  // Matched - figure out where to splice...
  // Go thru template and find ordered unescaped groups
  //  - if any overlap, they'll be in order:
  //      ( !5 ( !5 ) ) => $0 $1
  //    then $0 = 5..10, $1 = 0..10, so keep $1 as splice

  // TODO - factor out a find_splice_points() function here so we can
  // test it separately!

  // let mut splice_points: Vec<(usize, usize)> = vec![];
  // for (i, t) in tpl.iter().enumerate() {
  //   if let Some(cur) = t.as_unprotected_group() {
  //     let mut cur = cur as isize;
  //     // is this a valid splice point? i.e. does it overlap with previous end?
  //     if (cur as usize) < env.groups.len() {
  //       let g1 = env.groups[cur as usize];
  //       while let Some(last) = splice_points.pop() {
  //         let g0 = env.groups[last.1];
  //         if g0.1 <= g1.0 || g0.1 - g0.0 > g1.1 - g1.0 {
  //           // If there's no overlap or the existing one is bigger, put it back
  //           splice_points.push(last);
  //           cur = -1;
  //           break;
  //         }
  //       }
  //       if cur >= 0 {
  //         splice_points.push((i, cur as usize));
  //       }
  //     }
  //   }
  // }

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
//println!("Splices: {:?}", splices);
  for ((start, end), bases) in splices {
    let insert = if bases.len() > 0 { Some(bases) } else { None };
    dna.splice(*start, end - start, insert);
  }
}


type Rng = (usize, usize);
fn find_splice<'a, T: BaseLike>(tpl: &'a [TItem<T>], env: &[Rng], range: Rng)
                                -> Vec<(Rng, &'a [TItem<T>])> {
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
  
  fn internal<'a, T: BaseLike>(rest: &[(usize, Rng)], range: Rng,
              tpl: &'a [TItem<T>], out: &mut Vec<(Rng, &'a [TItem<T>])>) {
    // find the max in rest
    let rest: Vec<(usize, Rng)> =
        rest.iter().filter(|(_, r)| r.0 >= range.0 && r.1 <= range.1)
        .map(|x| *x).collect();
    let option_i = rest.iter().enumerate().max_by_key(|(_, (_, r))| r.1 - r.0);

    if let Some((i1, (i2, r))) = option_i {
      // split at i
      internal(&rest[i1 + 1 ..], (r.1, range.1), &tpl[i2 + 1 ..], out);
      internal(&rest[.. i1], (range.0, r.0), &tpl[.. *i2], out);
    } else {
      out.push((range, tpl));
    }
  }
  let mut out: Vec<(Rng, &'a [TItem<T>])> = Vec::new();
  internal(&unprotected, range, &tpl, &mut out);
  
  //out.reverse();
  out

  // This gives the "splice plan" - next, expand all the templates
  // BEFORE doing any splicing so as to not mess up the refs, then
  // splice everything RIGHT TO LEFT to keep indexes correct.
}


fn find<T: BaseLike, S: BaseLike>(haystack: &mut RopeCursor<T>, needle: &[S], start: usize) -> Option<usize> {
  let needle_len = needle.len();
  if needle_len == 0 { return Some(start); }
  let haystack_len = haystack.full_len();
  let char_table = build_char_table(needle);
  let offset_table = build_offset_table(needle);
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
      if needle[i + j] != needle[j] {
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
      if needle[ii] == needle[j] {
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
    let dna = SourceBase::collect::<Rope<_>>("ICFPIICFCPFIICICFC");
    let mut haystack = dna.cursor();
    let needle = SourceBase::collect::<Vec<_>>("IIC");
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
    let haystack = v.iter().map(|x| Base::from_u8(*x)).collect::<Rope<Base>>();
    let start = (start % haystack.len() as u64) as usize;
    let i = (i % haystack.len() as u64) as usize;
    let len = if start < haystack.len() - 1 {
      ((len % (haystack.len() - start - 1) as u64) + 1) as usize
    } else {
      0
    };
    let haystack_str =
        haystack.iter().map(|x| format!("{}", x)).collect::<Vec<_>>().join("");
    let needle_str = &haystack_str[start .. start + len];
    let needle = Base::collect::<Vec<_>>(needle_str);
    let expected = haystack_str[i..].find(needle_str).map(|j| i + j);
    assert_eq!(find(&mut haystack.cursor(), &needle, i), expected);
  }

  #[test]
  fn parse_pattern_1() {
    let dna = SourceBase::collect::<Rope<_>>("CIIC");
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
    let dna = Base::collect::<Rope<_>>("IIPIPICPIICICIIF");
    let mut state = DnaState::<Base>::new();
    let mut c = dna.cursor();
    let pat = PItem::parse(&mut c, &mut state);
    assert_eq!(pat,
               "( !2 ) P".split(' ').map(|s| s.parse::<PItem<Base>>().unwrap())
                   .collect::<Vec<_>>());
    assert_eq!(c.pos(), c.full_len());
    assert_eq!(state.finished, false);
    assert_eq!(state.rna, Vec::<[Base;7]>::new());
  }

  #[test]
  fn full_iteration_1() {
    let mut dna = Base::collect::<Rope<_>>("IIPIPICPIICICIIFICCIFPPIICCFPC");
    let mut state = DnaState::new();
    iterate(&mut dna, &mut state);
    assert_eq!(&str(&dna), "PICFC");
  }

  #[test]
  fn full_iteration_2() {
    let mut dna = Base::collect::<Rope<_>>("IIPIPICPIICICIIFICCIFCCCPPIICCFPC");
    let mut state = DnaState::new();
    iterate(&mut dna, &mut state);
    assert_eq!(&str(&dna), "PIICCFCFFPC");
  }

  #[test]
  fn full_iteration_3() {
    let mut dna = Base::collect::<Rope<_>>("IIPIPIICPIICIICCIICFCFC");
    let mut state = DnaState::new();
    iterate(&mut dna, &mut state);
    assert_eq!(&str(&dna), "I");
  }
}
