use std::cmp;
use std::fmt;
use std::mem;
use std::marker::PhantomData;

pub trait BaseLike: Copy + PartialEq {
  fn to_base(self) -> Base;
  fn to_u2(self) -> u8 { self.to_base() as u8 }
  fn from_base(base: Base) -> Self;
  fn from_base_pos(base: Base, pos: usize) -> Self;
  fn protect(self, level: u8, out: &mut Vec<Self>) {
    BaseLike::push(self.to_base() as u8 + level, out);
  }
  // TODO - how to make this private?
  fn push(i: u8, out: &mut Vec<Self>) {
    if i < 4 {
      out.push(BaseLike::from_base(Base::from_u8(i)));
    } else {
      BaseLike::push(i - 4, out);
      BaseLike::push(i - 3, out);
    }
  }
  fn unprotect(self) -> Self {
    // NOTE: for IC -> P, unprotect the I, not the C.
    BaseLike::from_base(Base::from_u8(self.to_base() as u8 + 3))
  }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base {
  I = 0,
  C = 1,
  F = 2,
  P = 3,
}
const BASE_CHARS: [char; 4] = ['I', 'C', 'F', 'P'];

impl fmt::Display for Base {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.char())
  }
}

// TODO - how to do this generally?
impl fmt::Display for SourceBase {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.to_base().char())
  }
}

impl Base {
  #[inline]
  pub fn from_u8(i: u8) -> Self {
    unsafe {
      mem::transmute::<u8, Base>(i & 3)
    }
  }
  #[inline]
  pub fn char(&self) -> char { BASE_CHARS[*self as usize] }
}

impl BaseLike for Base {
  #[inline]
  fn to_base(self) -> Base { self }
  fn from_base(base: Base) -> Self { base }
  fn from_base_pos(base: Base, _: usize) -> Self { base }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceBase(u32);

impl SourceBase {
  pub fn addr(self) -> u32 {
    (self.0 & ADDR_MASK) >> 2
  }

  pub fn level(self) -> i8 {
    ((self.0 as i32) >> 26) as i8
  }

  pub fn from_parts(base: Base, addr: u32, level: i8) -> Self {
    SourceBase(base as u8 as u32
               | (addr << 2) & ADDR_MASK
               | ((level as i32) << 26) as u32)
  }
}

impl BaseLike for SourceBase {
  #[inline]
  fn to_base(self) -> Base { Base::from_u8((self.0 & 3) as u8) }
  #[inline]
  fn to_u2(self) -> u8 { (self.0 & 3) as u8 }
  #[inline]
  fn from_base(base: Base) -> Self { SourceBase(base as u32) }
  #[inline]
  fn from_base_pos(base: Base, pos: usize) -> Self {
    SourceBase(base as u32 | ((pos & 0xffffff) as u32) << 2)
  }
  fn protect(self, level: u8, out: &mut Vec<Self>) {
    let mut esc = self.0 as i32 >> 26;
    if esc > -31 {
      esc = cmp::min(31, esc + level as i32);
    }
    let mask = (esc << 26) as u32 | (self.0 & ADDR_MASK);
    SourceBase::push(self.to_base() as u8 + level, mask, out);
  }
  fn unprotect(self) -> Self {
    let mut esc = self.0 as i32 >> 26;
    if esc < 31 && esc > -31 {
      esc = cmp::max(-31, esc - 1 as i32);
    }
    let mask = (esc << 26) as u32 | (self.0 & ADDR_MASK);
    let base = ((self.0 & 3) + 3) & 3;
    SourceBase(mask | base)
  }
}

const ADDR_MASK: u32 = 0xffffff << 2;

impl SourceBase {
  fn push(i: u8, mask: u32, out: &mut Vec<Self>) {
    if i < 4 {
      out.push(SourceBase(mask | (i as u32)));
    } else {
      SourceBase::push(i - 4, mask, out);
      SourceBase::push(i - 3, mask, out);
    }
  }
}

pub struct BaseLikeIterator<'a, T: BaseLike> {
  s: &'a [u8],
  i: usize,
  pos: usize,
  phantom: PhantomData<T>,
}

impl<'a, T: BaseLike> Iterator for BaseLikeIterator<'a, T> {
  type Item = T;
  fn next(&mut self) -> Option<T> {
    loop {
      if self.i < self.s.len() {
        let b = self.s[self.i];
        let base = match b {
          b'I' => Some(Base::I),
          b'C' => Some(Base::C),
          b'F' => Some(Base::F),
          b'P' => Some(Base::P),
          b' ' => None,
          b'\n' => None,
          _ => panic!("Bad character: {}", b),
        };
        self.i += 1;
        if let Some(base) = base {
          let pos = self.pos;
          self.pos += 1;
          return Some(T::from_base_pos(base, pos));
        }
      } else {
        return None;
      }
    }
  }
}


pub fn from_str<'a, T: BaseLike>(s: &'a str) -> BaseLikeIterator<'a, T> {
  BaseLikeIterator{s: s.as_bytes(), i: 0, pos: 0, phantom: PhantomData}
}


#[cfg(test)]
mod base_tests {
  use super::*;
  //use itertools::assert_equal;
  use quickcheck_macros::quickcheck;

  #[test]
  fn from_u8_base() {
    assert_eq!(Base::from_u8(0), Base::I);
    assert_eq!(Base::from_u8(1), Base::C);
    assert_eq!(Base::from_u8(2), Base::F);
    assert_eq!(Base::from_u8(3), Base::P);
  }

  #[quickcheck]
  fn from_u8_base_quick(x: u8) {
    assert_eq!(Base::from_u8(x), Base::from_u8(x & 3));
  }

  #[quickcheck]
  fn to_u2_base(x: u8) {
    assert_eq!(Base::from_u8(x).to_u2(), x & 3);
  }

  #[quickcheck]
  fn to_u2_sourcebase(x: u32) {
    assert_eq!(SourceBase(x).to_u2(), (x & 3) as u8);
  }

  #[test]
  fn from_str_base() {
    let s = "ICFPIIC";
    let v: Vec<Base> = from_str(s).collect::<Vec<Base>>();
    assert_eq!(v, vec![Base::I, Base::C, Base::F, Base::P, Base::I, Base::I, Base::C]);
  }

  #[test]
  fn from_str_sourcebase() {
    let s = "ICFPIIC";
    let v: Vec<SourceBase> = from_str(s).collect::<Vec<SourceBase>>();
    assert_eq!(v, vec![
      SourceBase(0 << 2 | 0), SourceBase(1 << 2 | 1),
      SourceBase(2 << 2 | 2), SourceBase(3 << 2 | 3),
      SourceBase(4 << 2 | 0), SourceBase(5 << 2 | 0),
      SourceBase(6 << 2 | 1),
    ]);
  }

  #[quickcheck]
  fn to_base_sourcebase(x: u32) {
    assert_eq!(SourceBase(x).to_base(), Base::from_u8(x as u8));
  }

  fn protect<T: BaseLike>(b: T, i: u8) -> Vec<T> {
    let mut v: Vec<T> = vec![];
    b.protect(i, &mut v);
    v
  }

  fn unprotect<T: BaseLike>(b: &[T]) -> Vec<T> {
    let mut v: Vec<T> = vec![];
    let mut i = 0;
    while i < b.len() {
      let base = b[i];
      if base.to_base() == Base::I {
        assert_eq!(b[i + 1].to_base(), Base::C);
        i += 1;
      }
      v.push(base.unprotect());
      i += 1;
    }
    v
  }
  
  #[test]
  fn protect_base() {
    assert_eq!(protect(Base::I, 0), vec![Base::I]);
    assert_eq!(protect(Base::C, 0), vec![Base::C]);
    assert_eq!(protect(Base::F, 0), vec![Base::F]);
    assert_eq!(protect(Base::P, 0), vec![Base::P]);

    assert_eq!(protect(Base::I, 1), vec![Base::C]);
    assert_eq!(protect(Base::C, 1), vec![Base::F]);
    assert_eq!(protect(Base::F, 1), vec![Base::P]);
    assert_eq!(protect(Base::P, 1), vec![Base::I, Base::C]);

    assert_eq!(protect(Base::I, 2), vec![Base::F]);
    assert_eq!(protect(Base::C, 2), vec![Base::P]);
    assert_eq!(protect(Base::F, 2), vec![Base::I, Base::C]);
    assert_eq!(protect(Base::P, 2), vec![Base::C, Base::F]);

    assert_eq!(protect(Base::P, 5), vec![Base::I, Base::C, Base::C, Base::F]);
  }

  #[quickcheck]
  fn protect_zero_base(x: u8) {
    assert_eq!(protect(Base::from_u8(x), 0), vec![Base::from_u8(x)]);
  }

  #[quickcheck]
  fn protect_zero_sourcebase(x: u32) {
    assert_eq!(protect(SourceBase(x), 0), vec![SourceBase(x)]);
  }

  #[test]
  fn sourcebase_addr() {
    assert_eq!(SourceBase(0x123456 << 2).addr(), 0x123456);
  }

  #[test]
  fn sourcebase_level() {
    assert_eq!(SourceBase((15 << 26) | 0x123456 << 2).level(), 15);
    assert_eq!(SourceBase((-15 << 26) as u32 | 0x123456 << 2).level(), -15);
    assert_eq!(SourceBase((31 << 26) | 0x123456 << 2).level(), 31);
    assert_eq!(SourceBase((-31 << 26) as u32 | 0x123456 << 2).level(), -31);
    assert_eq!(SourceBase((-32 << 26) as u32 | 0x123456 << 2).level(), -32);
  }

  #[quickcheck]
  fn sourcebase_from_parts(x: u32) {
    let orig = SourceBase(x);
    let rebuilt
        = SourceBase::from_parts(orig.to_base(), orig.addr(), orig.level());
    assert_eq!(rebuilt, orig);
  }

  #[test]
  fn protect_sourcebase() {
    assert_eq!(protect(SourceBase::from_parts(Base::F, 42, 3), 2),
               vec![SourceBase::from_parts(Base::I, 42, 5),
                    SourceBase::from_parts(Base::C, 42, 5)]);
  }

  #[test]
  fn unprotect_sourcebase() {
    assert_eq!(unprotect(&[SourceBase::from_parts(Base::F, 23, -4)]),
               vec![SourceBase::from_parts(Base::C, 23, -5)]);
  }

  #[quickcheck]
  fn protect_sourcebase_roundtrip(x: u32, mut i: u8) {
    i &= 63;
    let orig = SourceBase(x);
    let protected = protect(orig, i);
    let escaped_level = cmp::min(orig.level() + i as i8, 31);
    let expected_level = match (orig.level(), escaped_level) {
      (-32, _) => -32,
      (-31, _) => -31,
      (_, 31) => 31,
      (x, _) => x,
    };

    let mut unprotected = protected.clone();
    for _ in 0..i {
      unprotected = unprotect(&unprotected);
    }
    let expected_base
        = SourceBase::from_parts(orig.to_base(), orig.addr(), expected_level);

    assert_eq!(unprotected, vec![expected_base]);
  }
}
