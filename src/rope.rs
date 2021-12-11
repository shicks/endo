use std::cell::Cell;
use std::cmp;
use std::fmt;
use std::mem;
use std::ptr;

fn main() {
  let mut r: Rope<Base> = Rope::from_bases("IICIFPIFCPCIFP");
  println!("{}", r.base_str());
  let r2: Rope<Base> = Rope::from_bases("CIFPCICIFPCI");
  for i in (0 .. 100) {
    r = if i & 1 != 0 {
      Rope::join(r, r2.clone())
    } else {
      Rope::join(r2.clone(), r)
    }
  }
  let x: u32 = 0xffffffff;
  let y = x as i32;
  println!("{} {}", x, y);
  println!("{}", r.at(800));
  //println!("{}", r.base_str());
  //println!("{:?}", r);
}

const THRESHOLD: usize = 500;

trait BaseLike: Copy {
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
enum Base {
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
  fn from_u8(i: u8) -> Self {
    unsafe {
      mem::transmute::<u8, Base>(i & 3)
    }
  }
  #[inline]
  fn char(&self) -> char { BASE_CHARS[*self as usize] }
}

impl BaseLike for Base {
  #[inline]
  fn to_base(self) -> Base { self }
  fn from_base(base: Base) -> Self { base }
  fn from_base_pos(base: Base, _: usize) -> Self { base }
}

#[derive(Clone, Copy, Debug)]
struct SourceBase(u32);

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

#[derive(Clone)]
enum Node<T: Copy> { // Does it need to be BaseLike?
  App(App<T>),
  Leaf(Vec<T>),
}

impl<T: Copy> fmt::Debug for Node<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Node::App(a) => {
        write!(f, "(L{}{:?} R{}{:?})",
               a.depth, a.unwrap_left(), a.depth, a.unwrap_right())
      }
      Node::Leaf(a) => write!(f, "(0#{})", a.len())
    }
  }
}


#[derive(Clone)]
struct App<T: Copy> {
  // Note: these are ~always present, except during rebalancing.
  left: Option<Box<Node<T>>>,
  right: Option<Box<Node<T>>>,
  length: usize,
  depth: i8,
}

#[derive(Debug)]
struct Rope<T: Copy> {
  node: Box<Node<T>>,
  finger_index: Cell<usize>,
  // NOTE: Use a raw pointer because we have no way to
  // tie this lifetime to a borrow from inside the node.
  // We must MANUALLY maintain the invariant that any time
  // we mutate the node, we zero the finger. 
  finger_leaf: Cell<*const Vec<T>>,
}

// Manual clone impl to avoid cloning finger_leaf!
impl<T: Copy> Clone for Rope<T> {
  fn clone(&self) -> Rope<T> { Rope::from_node(self.node.clone()) }
}

impl<T: Copy> App<T> {
  #[inline]
  fn take_children(&mut self) -> (Box<Node<T>>, Box<Node<T>>) {
    (self.left.take().unwrap(), self.right.take().unwrap())
  }

  #[inline]
  fn set_children(&mut self, left: Box<Node<T>>, right: Box<Node<T>>) {
    self.depth = cmp::max(left.dep(), right.dep()) + 1;
    self.length = left.len() + right.len();
    self.left.insert(left);
    self.right.insert(right);
  }

  #[inline]
  fn unwrap_left(&self) -> &Node<T> {
    self.left.as_ref().unwrap()
  }

  #[inline]
  fn unwrap_right(&self) -> &Node<T> {
    self.right.as_ref().unwrap()
  } 
}

impl<T: Copy> Node<T> {
  fn dep(&self) -> i8 {
    match self {
      Node::App(App{depth, ..}) => *depth,
      Node::Leaf(..) => 0,
    }
  }

  fn len(&self) -> usize {
    match self {
      Node::App(App{length, ..}) => *length,
      Node::Leaf(arr) => arr.len(),
    }
  }

  fn at(&self, index: usize, orig: usize, parent: &Rope<T>) -> T {
    match self {
      Node::App(a) => {
        let left = a.unwrap_left();
        let len = left.len();
        if index < len {
          left.at(index, orig, parent)
        } else {
          a.unwrap_right().at(index - len, orig, parent)
        }
      },
      Node::Leaf(arr) => {
        // TODO - range checking?
        parent.finger_index.set(orig - index);
        parent.finger_leaf.set(&*arr);
        arr[index]
      }
    }     
  }

  #[inline]
  fn unwrap_app(&self) -> &App<T> {
    match self {
      Node::App(a) => a,
      _ => panic!("Expected an App"),
    }
  }

  #[inline]
  fn unwrap_mut_app(&mut self) -> &mut App<T> {
    match self {
      Node::App(a) => a,
      _ => panic!("Expected an App"),
    }
  }

  // #[inline]
  // fn unwrap_app(&mut self) -> &mut App {
  //   match self {
  //     Node::App(a) => a,
  //     default => panic!("Expected an App"),
  //   }
  // }

  #[inline]
  fn take_children(&mut self) -> (Box<Node<T>>, Box<Node<T>>) {
    self.unwrap_mut_app().take_children()
  }

  #[inline]
  fn set_children(&mut self, left: Box<Node<T>>, right: Box<Node<T>>) {
    self.unwrap_mut_app().set_children(left, right);
  }

  fn rebalance(&mut self) {
    // Here's where things get real.
    // Should we move this into join?  We want to be able to
    // do a simple rebalancing after splicing in/out data.
    // Moving to join would make it more of a persistent deal?
    // Maybe we need to write splice?
    if let Node::Leaf(..) = self { return; }
    loop {
      let app = self.unwrap_mut_app();
      if app.length < THRESHOLD {
        *self = Node::Leaf(self.to_vec());
        return;
      }
      let dl = app.unwrap_left().dep();
      let dr = app.unwrap_right().dep();
      if (dl - dr).abs() <= 1 { return; }
      if dl > dr { // taller left
        let (mut l, r) = app.take_children();
        let (ll, mut lr) = l.take_children();
        if lr.dep() > ll.dep() { // tall middle
          let (lrl, lrr) = lr.take_children();
          l.set_children(ll, lrl);
          lr.set_children(lrr, r);
          lr.rebalance();
          app.set_children(l, lr);
        } else { // left-skewed: simple rotate
          l.set_children(lr, r);
          l.rebalance();
          app.set_children(ll, l);
        }
      } else { // taller right
        let (l, mut r) = app.take_children();
        let (mut rl, rr) = r.take_children();
        if rl.dep() > rr.dep() { // tall middle
          let (rll, rlr) = rl.take_children();
          r.set_children(rlr, rr);
          rl.set_children(l, rll);
          rl.rebalance();
          app.set_children(rl, r);
        } else { // right-skewed: simple rotate
          r.set_children(l, rl);
          r.rebalance();
          app.set_children(r, rr);
        }
      }
    }
  }

  fn check_balance(&self) -> bool {
    match self {
      Node::Leaf(..) => true,
      Node::App(a) => {
        let bf = a.unwrap_left().dep() - a.unwrap_right().dep();
        if bf.abs() > 1 {
          println!("Bad balance: {} {:#?}", bf, self);
          return false;
        }
        a.unwrap_left().check_balance() && a.unwrap_right().check_balance()
      }
    }
  }

  fn to_vec(&self) -> Vec<T> {
    let len = self.len();
    let mut vec = Vec::with_capacity(len);
    unsafe { vec.set_len(len); } // initialized below
    let mut stack = vec![self];
    let mut index: usize = 0;
    while let Some(top) = stack.pop() {
      match top {
        Node::Leaf(a) => {
          let l = a.len();
          vec[index .. index + l].copy_from_slice(a);
          index += l;
        }
        Node::App(a) => {
          stack.push(a.unwrap_right());
          stack.push(a.unwrap_left());
        }
      }
    }
    return vec;
  }

  // Takes ownership of both, so we can just concat the nodes
  fn join(left: Box<Node<T>>, right: Box<Node<T>>) -> Box<Node<T>> {
    // NOTE: assume that nodes are balanced on the way in
    if left.len() == 0 {
      return right;
    } else if right.len() == 0 {
      return left;
    }
    let depth = cmp::max(left.dep(), right.dep()) + 1;
    let length = left.len() + right.len();
    let app = App{left: Some(left), right: Some(right), depth, length};
    let mut node = Node::App(app);
    node.rebalance();
    Box::new(node)
  }

  fn splice(&mut self, start: usize, length: usize, insert: Option<Vec<T>>) {
    panic!("not implemented");
  }
}


impl<T: Copy> Rope<T> {
  #[inline]
  pub fn len(&self) -> usize {
    self.node.len()
  }

  pub fn splice(&mut self, start: usize, length: usize, insert: Option<Vec<T>>) {
    // Mutating operation requires nulling out the finger.
    self.node.splice(start, length, insert);
    self.finger_index.set(0);
    self.finger_leaf.set(ptr::null());
  }

  // Range check and -> Option<Base> ?
  pub fn at(&self, index: usize) -> T {
    let leaf = self.finger_leaf.get();
    if !leaf.is_null() {
      let finger = self.finger_index.get();
      unsafe {
        if index >= finger && index < finger + (*leaf).len() {
          return (*leaf)[index - finger];
        }
      }
    }
    self.node.at(index, index, self)
  }

  fn from_node(node: Box<Node<T>>) -> Rope<T> {
    Rope{node,
         finger_index: Cell::new(0),
         finger_leaf: Cell::new(ptr::null())}
  }

  pub fn join(left: Rope<T>, right: Rope<T>) -> Rope<T> {
    Rope::from_node(Node::join(left.node, right.node))
  }
}

impl<T: BaseLike> Rope<T> {
  fn from_bases(s: &str) -> Rope<T> {
    let mut vec = Vec::new();
    let mut i = 0;
    for c in s.chars() {
      vec.push(match c {
        'I' => BaseLike::from_base_pos(Base::I, i),
        'C' => BaseLike::from_base_pos(Base::C, i),
        'F' => BaseLike::from_base_pos(Base::F, i),
        'P' => BaseLike::from_base_pos(Base::P, i),
        _ => panic!("Bad base {}", c),
      });
      i += 1; 
    }
    //let slice: [Base] = &vec;
    Rope::from_node(Box::new(Node::Leaf(vec.into())))
  }

  fn base_str(&self) -> String {
    let mut buf = String::with_capacity(self.len());
    let mut stack: Vec<&Node<T>> = vec![&self.node];
    while stack.len() > 0 {
      match stack.pop().unwrap() {
        Node::Leaf(arr) => {
          for base in arr.iter() {
            match base.to_base() {
              Base::I => buf.push('I'),
              Base::C => buf.push('C'),
              Base::F => buf.push('F'),
              Base::P => buf.push('P'),
            }
          }
        }
        Node::App(a) => {
          stack.push(a.unwrap_right());
          stack.push(a.unwrap_left());
        }
      }
    }
    buf
  }
}
