use std::cmp::max;
use rope::*;
use base::{Base, BaseLike};

pub struct Emit<T: BaseLike> {
  rna: [T; 7],
}

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

pub enum PItem<T: BaseLike> {
  Bases(Vec<T>),
  // Also stores the index where we found it, for debugging...?
  Skip(u32),
  Search(Vec<T>),
  OpenGroup,
  CloseGroup,
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
        cursor.skip(*i as isize);
        if cursor.at_end() { return false; }
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
    u32::parse(cursor).map(PItem::Skip)
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


pub enum TItem<T: BaseLike> {
  Bases(Vec<T>),
  // Also stores the index where we found it, for debugging...?
  Len(u32),
  Ref{group: u32, level: u32},
}

impl<T: BaseLike> Template<T> for TItem<T> {
  fn expand(&self, vec: &mut Vec<T>, env: &[(usize, usize)],
            cursor: &mut RopeCursor<T>) {
    panic!()
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
    u32::parse(cursor).map(TItem::Len)
  }

  fn make_ref(cursor: &mut RopeCursor<T>) -> Option<Self> {
    cursor.skip(2);
    if let Some(level) = u32::parse(cursor) {
      if let Some(group) = u32::parse(cursor) {
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
  fn parse_internal(cursor: &mut RopeCursor<T>, acc: u32) -> Option<Self>;
}
pub trait Bases<T: BaseLike>: Sized {
  fn parse(cursor: &mut RopeCursor<T>) -> Self;
}

impl<T: BaseLike> Num<T> for u32 {
  fn parse(cursor: &mut RopeCursor<T>) -> Option<Self> {
    return Self::parse_internal(cursor, 0);
  }
  fn parse_internal(cursor: &mut RopeCursor<T>, acc: u32) -> Option<Self> {
    let mut v: u32 = 0;
    let mut mask: u32 = 1;
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
  fn rna(&mut self, cursor: &mut RopeCursor<T>);
  fn finish(&mut self);

  #[inline]
  fn or_finish<U>(&mut self, o: Option<U>) -> Option<U> {
    if o.is_none() { self.finish(); }
    o
  }
}

pub trait Pattern<T: BaseLike>: Sized {
  fn exec<S: BaseLike>(&self, cursor: &mut RopeCursor<S>, env: &mut Env) -> bool;
  fn make_bases(cursor: &mut RopeCursor<T>) -> Self;
  fn make_skip(cursor: &mut RopeCursor<T>) -> Option<Self>;
  fn make_search(cursor: &mut RopeCursor<T>) -> Self;
  fn make_open(cursor: &mut RopeCursor<T>) -> Self;
  fn make_close(cursor: &mut RopeCursor<T>) -> Self;
  fn parse<S: State<T>>(cursor: &mut RopeCursor<T>, depth: &mut usize,
                        state: &mut S) -> Option<Self> {
    let pos = cursor.pos();
    let i = cursor.peek();
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
        state.rna(cursor);
        Self::parse(cursor, depth, state)
      }
    }
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

  fn parse<S: State<T>>(cursor: &mut RopeCursor<T>,
                        state: &mut S) -> Option<Self> {
    let pos = cursor.pos();
    let i = cursor.peek();
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
        state.rna(cursor);
        Self::parse(cursor, state)
      }
    }
  }
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
  use base::{Base, SourceBase, from_str};

  #[test]
  fn find_simple() {
    let mut haystack = from_str("ICFPIICFCPFIICICFC").collect::<Rope<SourceBase>>().cursor();
    let needle = from_str("IIC").collect::<Vec<SourceBase>>();
    assert_eq!(find(&haystack, &needle, 0), Some(4));
    assert_eq!(find(&haystack, &needle, 1), Some(4));
    assert_eq!(find(&haystack, &needle, 3), Some(4));
    assert_eq!(find(&haystack, &needle, 4), Some(4));
    assert_eq!(find(&haystack, &needle, 5), Some(11));
    assert_eq!(find(&haystack, &needle, 8), Some(11));
    assert_eq!(find(&haystack, &needle, 11), Some(11));
    assert_eq!(find(&haystack, &needle, 12), None);
    assert_eq!(find(&haystack, &needle, 14), None);
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
    let needle = from_str(needle_str).collect::<Vec<Base>>();
    let expected = haystack_str[i..].find(needle_str).map(|j| i + j);
    assert_eq!(find(&haystack, &needle, i), expected);
  }

}
