use std::cmp;
use std::fmt;
use std::mem;

pub trait BaseLike: Copy {
  fn to_base(self) -> Base;
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
#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct SourceBase(u32);

impl BaseLike for SourceBase {
  #[inline]
  fn to_base(self) -> Base { Base::from_u8((self.0 & 3) as u8) }
  fn from_base(base: Base) -> Self { SourceBase(base as u32) }
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
    if esc < 31 {
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
      out.push(BaseLike::from_base(Base::from_u8(i)));
    } else {
      BaseLike::push(i - 4, out);
      BaseLike::push(i - 3, out);
    }
  }
}
