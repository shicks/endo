use std::cmp;

#[derive(Clone, Debug)]
pub struct Rope<T: Copy>(Option<Box<Node<T>>>);

// TODO: Consider a custom Debug impl for Rope?
// See https://gist.github.com/shicks/22265fbb1dc1c8c38d5424c3dcd0a7f2

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node<T: Copy> {
  App(App<T>),
  Leaf(Vec<T>),
}

#[derive(Clone, Debug)]
struct App<T: Copy> {
  left: Rope<T>,
  right: Rope<T>,
  length: usize,
  depth: i8,
}

// NOTE: Not using the derived equals because we want to make a custom
// structure-agnostic equality for Rope<T>.
impl<T: Copy + PartialEq> PartialEq<App<T>> for App<T> {
  fn eq(&self, other: &App<T>) -> bool {
    self.length == other.length
      && self.left.0 == other.left.0
      && self.right.0 == other.right.0
  }
}
impl<T: Copy + Eq> Eq for App<T> {}

////////////////////////////////////////////////////////////////
// Rope Methods

impl<T: Copy> FromIterator<T> for Rope<T> {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    Rope::from_vec(Vec::from_iter(iter))
  }
}

impl<T: Copy> Rope<T> {

  ////////////////////////////////////////////////////////////////
  // Constructors

  pub fn new() -> Self { Rope(None) }

  pub fn from_slice(slice: &[T]) -> Self {
    Rope(if slice.len() == 0 {
      None
    } else {
      Some(Box::new(Node::Leaf(Vec::from(slice))))
    })
  }

  pub fn from_vec(vec: Vec<T>) -> Self {
    Rope(if vec.len() == 0 {
      None
    } else {
      Some(Box::new(Node::Leaf(vec)))
    })
  }

  ////////////////////////////////////////////////////////////////
  // Accessors

  #[inline]
  pub fn len(&self) -> usize {
    match self.0.as_deref() {
      None => 0,
      Some(Node::Leaf(v)) => v.len(),
      Some(Node::App(App{length, ..})) => *length,
    }
  }

  #[inline]
  pub fn dep(&self) -> i8 {
    match self.0.as_deref() {
      None|Some(Node::Leaf(_)) => 0 as i8,
      Some(Node::App(App{depth, ..})) => *depth,
    }
  }

  #[inline]
  pub fn cursor<'a>(&'a self) -> RopeCursor<'a, T> {
    RopeCursor::new(self)
  }

  // Alternative name, for convention
  #[inline]
  pub fn iter<'a>(&'a self) -> RopeCursor<'a, T> {
    RopeCursor::new(self)
  }

  ////////////////////////////////////////////////////////////////
  // Joiners

  pub fn append_rope(&mut self, mut right: Rope<T>) {
    // Assume both ropes are balanced
    if right.len() == 0 { return; }
    if self.len() == 0 {
      std::mem::swap(self, &mut right);
      return;
    }
    let length = self.len() + right.len();
    let depth = cmp::max(self.dep(), right.dep()) + 1; // unneeded?
    let left = self.0.take();
    let out = Some(Box::new(Node::App(App{left: Rope(left), right, length, depth})));
    self.0 = out;
    self.rebalance();
  }

  pub fn append_slice(&mut self, right: &[T]) {
    self.append_rope(Rope::from_slice(right))
  }

  pub fn prepend_slice(&mut self, left: &[T]) {
    let mut other = Rope::from_slice(left);
    std::mem::swap(self, &mut other);
    self.append_rope(other);
  }

  ////////////////////////////////////////////////////////////////
  // Splice

  pub fn splice(&mut self, start: usize, length: usize,
                insert: Option<Vec<T>>) {
    let insert_len = insert.as_ref().map(|v| v.len()).unwrap_or_default();
    self.splice_internal(start, start + length,
                         insert_len as isize - length as isize,
                         insert);
  }

  fn splice_internal(&mut self, start: usize, end: usize,
                     delta: isize, insert: Option<Vec<T>>) {
    match self.0.as_deref_mut() {
      None => {
        self.0 = insert.map(|v| Box::new(Node::Leaf(v)));
      }
      Some(Node::App(App{ref mut left, ref mut right,
                         length: ref mut app_len,
                         depth: ref mut dep})) => {
        let left_len = left.len();
        if start >= left_len {
          // Only need to touch right child
          right.splice_internal(start - left_len, end - left_len, delta, insert);
        } else if end <= left_len {
          // Only need to touch left child
          left.splice_internal(start, end, delta, insert);
        } else {
          // Need to remove parts of both
          let left_delta = start as isize - left_len as isize;
          left.splice_internal(start, left_len, left_delta, None);
          right.splice_internal(0, end - left_len, delta - left_delta, insert);
        }
        *app_len = (*app_len as isize + delta) as usize;
        *dep = cmp::max(left.dep(), right.dep()) + 1;
        self.rebalance();
      }
      Some(Node::Leaf(ref mut arr)) => {
        let arr_len = arr.len();
        let new_len = ((arr_len as isize) + delta) as usize;
        let mut left: Rope<T> = Rope(None);
        let mut right: Rope<T> = Rope(None);
        let mut middle: Rope<T> = Rope(None);
//   let insert_len = insert.as_ref().map(|x| x.len());
        if end < arr_len {
          right = Rope(Some(Box::new(Node::Leaf(arr.split_off(end)))));
          // assert_eq!(arr.len(), end);
          // assert_eq!(right.len(), arr_len - end);
        }
        if start > 0 {
          arr.truncate(start);
          left = Rope(self.0.take());
        }
        if let Some(v) = insert {
          middle = Rope(Some(Box::new(Node::Leaf(v))));
        }
        if right.len() == 0 {
          right = middle;
        } else if left.len() == 0 {
          left = middle;
        } else if middle.len() > 0 {
          let left_len = left.len();
          let right_len = right.len();
// eprintln!("3-way join: {}, {}, {}", left_len, middle.len(), right_len);
          if left_len < right_len {
            left = Rope(Some(Box::new(Node::App(App{
              length: left_len + middle.len(), depth: 1,
              left, right: middle}))));
          } else {
            right = Rope(Some(Box::new(Node::App(App{
              length: right_len + middle.len(), depth: 1,
              left: middle, right}))));
          }
        }
        // At this point, middle is empty (and probably moved)
        // All that's left to do is combine left and right, if
        // both are present.
        let left_len = left.len();
        if left_len == 0 || right.len() == 0 {
          self.0 = if left_len > 0 { left.0 } else { right.0 };
        } else {
          let depth = cmp::max(left.dep(), right.dep()) + 1;
          // if new_len != left.len() + right.len() {
          //   eprintln!("arr_len = {}, delta = {}, new_len = {}, insert = {:?}, start = {}, end = {}",
          //            arr_len, delta, new_len, insert_len, start, end);
          // }
          // assert_eq!(new_len, left.len() + right.len());
          self.0 = Some(Box::new(Node::App(App{
              left, right, length: new_len, depth})));
        }
      }
    }
  }

  ////////////////////////////////////////////////////////////////
  // Internal

  #[inline]
  fn balance_factor(&self) -> i8 {
    match self.0.as_deref() {
      None|Some(Node::Leaf(_)) => 0,
      Some(Node::App(App{left, right, ..})) => left.dep() - right.dep(),
    }
  }

  fn check_invariants(&self) {
    match self.0.as_deref() {
      None|Some(Node::Leaf(_)) => {},
      Some(Node::App(App{left, right, length, depth})) => {
        if *length != left.len() + right.len() {
          panic!("Bad length {} from left {} and right {}",
                 length, left.len(), right.len());
        } else if *depth != cmp::max(left.dep(), right.dep()) + 1 {
          panic!("Bad depth {} from left {} and right {}",
                 depth, left.dep(), right.dep());
        } else if depth - left.dep() > 2 {
          panic!("Unbalanced small left child {} vs right {}",
                 left.dep(), right.dep());
        } else if depth - right.dep() > 2 {
          panic!("Unbalanced small right child {} vs left {}",
                 right.dep(), left.dep());
        }        
      }
    }
  }

  fn rebalance(&mut self) {
    // Assumption: left and right branches are individually balanced,
    // but the balance factor of self may be way off.
    loop {
      let bf = self.balance_factor();
      if bf > 1 {
        // left is taller
        let (mut l, r) = self.take_children();
        let (ll, mut lr) = l.take_children();
        if lr.dep() > ll.dep() {
          // middle is taller: split up LR and pivot it to R
          let (lrl, lrr) = lr.take_children();
          lr.set_children(lrr, r);
          lr.rebalance(); // right could be _way_ shorter
          l.set_children(ll, lrl); // don't need to rebalance if l was balanced
          self.set_children(l, lr);          
        } else {
          // middle not taller: simple 3-way rotate right, pivot L to R
          l.set_children(lr, r);
          l.rebalance();
          self.set_children(ll, l);
        }
      } else if bf < -1 {
        // right is taller
        let (l, mut r) = self.take_children();
        let (mut rl, rr) = r.take_children();
        if rl.dep() > rr.dep() {
          // middle is taller: split up RL and pivot it to L
          let (rll, rlr) = rl.take_children();
          rl.set_children(l, rll);
          rl.rebalance(); // left could be _way_ shorter
          r.set_children(rlr, rr); // don't need to rebalance if r was balanced
          self.set_children(rl, r);          
        } else {
          // middle not taller: simple 3-way rotate right, pivot L to R
          r.set_children(l, rl);
          r.rebalance();
          self.set_children(r, rr);
        }
      } else {
        break;
      }
    }
  }

  #[inline]
  fn set_children(&mut self, left: Self, right: Self) {
    if let Some(Node::App(a)) = self.0.as_deref_mut() {
      a.length = left.len() + right.len();
      a.depth = cmp::max(left.dep(), right.dep()) + 1;
      a.left = left;
      a.right = right;
    } else {
      panic!("set_children on a leaf");
    }
  }

  #[inline]
  fn take_children(&mut self) -> (Self, Self) {
    if let Some(Node::App(App{left, right, ..})) = self.0.as_deref_mut() {
      (Rope(left.0.take()), Rope(right.0.take()))
    } else {
      panic!("take_children on a leaf")
    }
  }
}


// fn splice_suffix<T>(arr: &mut Vec<T>, mid: usize) -> Vec<T> {
//   unsafe {
//     // let mut copy: Vec<T> = &mut *arr;
//     // let (ptr, len, cap) = copy.into_raw_parts();
//     let len = arr.len();
//     let cap = arr.capacity();
//     let ptr = arr.as_mut_ptr();
//     let right = Vec::from_raw_parts(ptr.add(mid), len - mid, cap - mid);
//     *arr = Vec::from_raw_parts(ptr, mid, mid);
//     right
//   }
// }


////////////////////////////////////////////////////////////////
// Cursor/Iterator

pub struct RopeCursor<'a, T: Copy> {
  root: &'a Rope<T>,
  stack: Vec<&'a Rope<T>>,
  start: usize,
  index: usize,
  leaf: Option<&'a Vec<T>>,
}

impl<'a, T: Copy> RopeCursor<'a, T> {
  #[inline]
  fn new(root: &'a Rope<T>) -> Self {
    RopeCursor{root, stack: vec![root], start: 0, index: 0, leaf: None}
  }

  #[inline]
  pub fn root(&self) -> &'a Rope<T> {
    self.root
  }

  #[inline]
  pub fn full_len(&self) -> usize {
    self.root.len()
  }

  #[inline]
  pub fn at_end(&self) -> bool {
    self.index >= self.full_len()
  }

  #[inline]
  pub fn pos(&self) -> usize {
    self.index
  }

  #[inline]
  pub fn seek(&mut self, pos: usize) {
    self.index = pos;
  }

  #[inline]
  pub fn skip(&mut self, delta: isize) {
    self.index = (self.index as isize + delta) as usize;
  }

  #[inline]
  pub fn peek(&mut self) -> Option<T> {
    if self.index < self.root.len() {
      Some(self.at(self.index))
    } else {
      None
    }
  }

  #[inline]
  pub fn at(&mut self, pos: usize) -> T {
    self.seek_internal(pos);
    self.leaf.unwrap()[pos - self.start]
  }

  #[inline]
  pub fn try_at(&mut self, pos: usize) -> Option<T> {
    if !self.at_end() { Some(self.at(pos)) } else { None }
  }

  fn seek_internal(&mut self, mut pos: usize) {
    if pos < self.start {
      self.start = 0;
      self.leaf = None;
      self.stack = vec![self.root]; // truncate and assign?
    }
    if let Some(arr) = self.leaf {
      let arr_len = arr.len();
      if pos < self.start + arr_len { return; }
      self.start += arr_len;
      self.leaf = None;
    }
    pos -= self.start;
    while let Some(top) = self.stack.pop() {
      let top_len = top.len();
      if pos >= top_len {
        self.start += top_len;
        pos -= top_len;
        continue;
      }
      match top.0.as_deref() {
        None => {}
        Some(Node::App(App{left, right, ..})) => {
          self.stack.push(right);
          self.stack.push(left);
        }
        Some(Node::Leaf(arr)) => {
          self.leaf = Some(arr);
          return;
        }
      }
      
    }
    panic!("Out of bounds?");
  }
}

impl<'a, T: Copy> Iterator for RopeCursor<'a, T> {
  type Item = T;
  fn next(&mut self) -> Option<Self::Item> {
    if self.index >= self.root.len() {
      None
    } else {
      let result = self.at(self.index);
      self.index += 1;
      Some(result)
    }
  }
  fn size_hint(&self) -> (usize, Option<usize>) {
    let rest = self.root.len() - self.index;
    (rest, Some(rest))
  }
}


#[cfg(test)]
mod rope_tests {
  use super::*;
  use itertools::assert_equal;
  use quickcheck_macros::quickcheck;

  macro_rules! app {
    { $( $key:ident : $val:expr ),* }
      => {
        Rope(Some(Box::new(Node::App(App{$( $key: $val ),*}))))
      }
  }

  macro_rules! assert_rope_eq {
    ( $left:expr , $right:expr )
      => {
        assert_eq!($left.0, $right.0)
      }
  }

  // macro_rules! leaf {
  //   [ $( $val:expr ),* ]
  //     => {
  //       Rope(Some(Box::new(Node::Leaf(vec![$( $val ),*]))))
  //     }
  // }

  fn leaf<T: Copy>(data: &[T]) -> Rope<T> {
    Rope(Some(Box::new(Node::Leaf(Vec::from(data)))))
  }

  #[test]
  fn new_rope() {
    let rope = Rope::<u8>::new();
    assert_rope_eq!(rope, Rope(None));
    assert_eq!(rope.len(), 0);
  }

  #[test]
  fn from_slice() {
    let rope = Rope::from_slice(&[1, 3, 2, 4, 8]);
    assert_rope_eq!(rope, leaf(&[1, 3, 2, 4, 8]));
    assert_eq!(rope.len(), 5);
  }

  #[test]
  fn append_rope_short() {
    // NOTE: We need large leafs to avoid the consolidation threshold
    let s1 = &[2, 5, 4, 1, 6];
    let s2 = &[3, 7, 9, 8, 0];
    let mut left = Rope::from_slice(s1);
    let right = Rope::from_slice(s2);
    left.append_rope(right);
    let mut out = s1.iter().map(|x| *x).collect::<Vec<_>>();
    out.extend_from_slice(s2);
    assert_rope_eq!(left, leaf(&out));
  }

  #[test]
  fn append_rope() {
    // NOTE: We need large leafs to avoid the consolidation threshold
    let s1 = &[2, 5, 4, 1, 6].iter().cycle().take(300).collect::<Vec<_>>();
    let s2 = &[3, 7, 9, 8, 0].iter().cycle().take(300).collect::<Vec<_>>();
    let mut left = Rope::from_slice(s1);
    let right = Rope::from_slice(s2);
    left.append_rope(right);
    assert_rope_eq!(left,
                    app!{left: leaf(s1), right: leaf(s2),
                         length: 600, depth: 1});
  }

  #[test]
  fn append_slice_short() {
    let s1 = &[2, 5, 4, 1, 6];
    let s2 = &[3, 7, 9, 8, 0];
    let mut rope = Rope::from_slice(s1);
    rope.append_slice(s2);
    let mut out = s1.iter().map(|x| *x).collect::<Vec<_>>();
    out.extend_from_slice(s2);
    assert_rope_eq!(rope, leaf(&out));
  }

  #[test]
  fn append_slice() {
    let s1 = &[2, 5, 4, 1, 6].iter().cycle().take(300).collect::<Vec<_>>();
    let s2 = &[3, 7, 9, 8, 0].iter().cycle().take(300).collect::<Vec<_>>();
    let mut rope = Rope::from_slice(s1);
    rope.append_slice(s2);
    assert_rope_eq!(rope,
                    app!{left: leaf(s1), right: leaf(s2),
                         length: 600, depth: 1});
  }

  #[test]
  fn prepend_slice_short() {
    let s1 = &[2, 5, 4, 1, 6];
    let s2 = &[3, 7, 9, 8, 0];
    let mut rope = Rope::from_slice(s1);
    rope.prepend_slice(s2);
    let mut out = s2.iter().map(|x| *x).collect::<Vec<_>>();
    out.extend_from_slice(s1);
    assert_rope_eq!(rope, leaf(&out));
  }

  #[test]
  fn prepend_slice() {
    let s1 = &[2, 5, 4, 1, 6].iter().cycle().take(300).collect::<Vec<_>>();
    let s2 = &[3, 7, 9, 8, 0].iter().cycle().take(300).collect::<Vec<_>>();
    let mut rope = Rope::from_slice(s1);
    rope.prepend_slice(s2);
    assert_rope_eq!(rope,
                    app!{left: leaf(s2), right: leaf(s1),
                         length: 600, depth: 1});
  }

  #[test]
  fn cursor() {
    let rope = Rope::from_slice(&[1, 3, 5, 7, 9]);
    let mut cursor = rope.cursor();
    assert_eq!(cursor.pos(), 0);
    assert_eq!(cursor.next(), Some(1));
    assert_eq!(cursor.pos(), 1);
    assert_eq!(cursor.next(), Some(3));
    assert_eq!(cursor.pos(), 2);
    assert_eq!(cursor.next(), Some(5));
    assert_eq!(cursor.pos(), 3);
    assert_eq!(cursor.next(), Some(7));
    assert_eq!(cursor.pos(), 4);
    assert_eq!(cursor.at(1), 3); // at() doesn't move pos
    assert_eq!(cursor.pos(), 4);
    assert_eq!(cursor.next(), Some(9));
    assert_eq!(cursor.pos(), 5);
    assert_eq!(cursor.next(), None);
    assert_eq!(cursor.pos(), 5);
    assert_eq!(cursor.next(), None);
    cursor.seek(2); // seek() does move pos
    assert_eq!(cursor.pos(), 2);
    assert_eq!(cursor.next(), Some(5));
  }

  #[quickcheck]
  fn iterator_parity(xs: Vec<u32>) {
    let rope = xs.iter().cloned().collect::<Rope<_>>();
    rope.check_invariants();
    assert_equal(rope.iter(), xs.iter().map(|x| *x))
  }

  #[quickcheck]
  fn splice_parity(ops: Vec<SpliceOp>) {
    let mut v: Vec<u32> = vec![];
    let mut r: Rope<u32> = Rope::new();
    let mut i: u32 = 0;
    for op in ops {
      op.apply(&mut i, &mut v, &mut r);
      assert_equal(r.iter(), v.iter().map(|x| *x));
      r.check_invariants();
    }
  }

  #[derive(Clone, Debug)]
  struct SpliceOp {
    start: f32,
    len: f32,
    insert: Option<u16>,
  }
  impl quickcheck::Arbitrary for SpliceOp {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
      SpliceOp{
        // start is uniform on the range (though it would be nice to bias
        // toward 0 and 1 if there were an easy way to do it...
        start: u64::arbitrary(g) as f32 / u64::MAX as f32,
        // len should bias toward smaller numbers to avoid taking out
        // entire rope all the time
        len: (u64::arbitrary(g) as f32 / u64::MAX as f32).powi(3),
        // insert ranges 0..2k, with extra clumping from 0..64
        insert: match u8::arbitrary(g) % 3 {
          0 => None,
          1 => Some(u16::arbitrary(g) / 32),
          2 => Some(u16::arbitrary(g) / 1024),
          _ => panic!(),
        }
      }
    }
  }
  impl SpliceOp {
    fn apply(&self, i: &mut u32, v: &mut Vec<u32>, r: &mut Rope<u32>) {
      let insert: Option<Vec<u32>> = self.insert.map(|len| {
        *i += len as u32;
        (*i-len as u32 .. *i).collect()
      });
      let replace_with = insert.clone().unwrap_or_else(|| vec![]).into_iter();
      let start = f32::round(self.start * (v.len()) as f32) as usize;
      let length = f32::round(self.len * (v.len() - start) as f32) as usize;
      v.splice(start .. start + length, replace_with);
      r.splice(start, length, insert);
    }
  }
}
