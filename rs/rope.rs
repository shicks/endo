use std::cmp;
use std::fmt;

mod base;
use base::Base;
use base::BaseLike;
//use base::SourceBase;

fn main() {
  let mut r = Rope::<Base>::from_bases("IICIFPIFCPCIFP");
  println!("{}", r.base_str());
  let r2 = Rope::<Base>::from_bases("CIFPCICIFPCI");
  for i in 0 .. 100 {
    r = if i & 1 != 0 {
      Rope::join(r, r2.clone())
    } else {
      Rope::join(r2.clone(), r)
    }
  }
  let x: u32 = 0xffffffff;
  let y = x as i32;
  println!("{} {}", x, y);
  let mut c = RopeCursor::new(&r);
  println!("{}", c.at(800));
  r = Rope::join(r, r2);
  r = Rope::splice(r, 0, 4, None);
  println!("{}", r.base_str());
  //println!("{:?}", r);
}

const THRESHOLD: usize = 500;


#[derive(Clone)]
enum Rope<T: Copy> { // Does it need to be BaseLike?
  App(App<T>),
  Leaf(Vec<T>),
}

impl<T: Copy> fmt::Debug for Rope<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Rope::App(a) => {
        write!(f, "(L{}{:?} R{}{:?})",
               a.depth, a.unwrap_left(), a.depth, a.unwrap_right())
      }
      Rope::Leaf(a) => write!(f, "(0#{})", a.len())
    }
  }
}


#[derive(Clone)]
struct App<T: Copy> {
  // Note: these are ~always present, except during rebalancing.
  left: Option<Box<Rope<T>>>,
  right: Option<Box<Rope<T>>>,
  length: usize,
  depth: i8,
}

// #[derive(Debug)]
// struct Rope<T: Copy> {
//   rope: Box<Rope<T>>,
//   finger_index: Cell<usize>,
//   // NOTE: Use a raw pointer because we have no way to
//   // tie this lifetime to a borrow from inside the rope.
//   // We must MANUALLY maintain the invariant that any time
//   // we mutate the rope, we zero the finger. 
//   finger_leaf: Cell<*const Vec<T>>,
// }

impl<T: Copy> App<T> {
  #[inline]
  fn take_children(&mut self) -> (Box<Rope<T>>, Box<Rope<T>>) {
    (self.left.take().unwrap(), self.right.take().unwrap())
  }

  #[inline]
  fn set_children(&mut self, left: Box<Rope<T>>, right: Box<Rope<T>>) {
    self.depth = cmp::max(left.dep(), right.dep()) + 1;
    self.length = left.len() + right.len();
    self.left = Some(left);
    self.right = Some(right);
    // TODO - rebalance?  or trust that it'll happen from caller?
  }

  #[inline]
  fn unwrap_left(&self) -> &Rope<T> {
    self.left.as_ref().unwrap()
  }

  #[inline]
  fn unwrap_right(&self) -> &Rope<T> {
    self.right.as_ref().unwrap()
  } 
}

impl<T: Copy> Rope<T> {
  fn dep(&self) -> i8 {
    match self {
      Rope::App(App{depth, ..}) => *depth,
      Rope::Leaf(..) => 0,
    }
  }

  fn len(&self) -> usize {
    match self {
      Rope::App(App{length, ..}) => *length,
      Rope::Leaf(arr) => arr.len(),
    }
  }

  fn at(&self, index: usize) -> T {
    match self {
      Rope::App(a) => {
        let left = a.unwrap_left();
        let len = left.len();
        if index < len {
          left.at(index)
        } else {
          a.unwrap_right().at(index - len)
        }
      }
      Rope::Leaf(arr) => arr[index]
    }     
  }

  fn find_leaf(&self, index: usize, start: usize) -> (usize, &[T])  {
    match self {
      Rope::App(a) => {
        let left = a.unwrap_left();
        let len = left.len();
        if index < len {
          left.find_leaf(index, start)
        } else {
          a.unwrap_right().find_leaf(index - len, start)
        }
      },
      Rope::Leaf(arr) => (start, arr)
    }     
  }

  #[inline]
  fn unwrap_app(&self) -> &App<T> {
    match self {
      Rope::App(a) => a,
      _ => panic!("Expected an App"),
    }
  }

  #[inline]
  fn unwrap_mut_app(&mut self) -> &mut App<T> {
    match self {
      Rope::App(a) => a,
      _ => panic!("Expected an App"),
    }
  }

  #[inline]
  fn take_app(rope: Box<Rope<T>>) -> App<T> {
    match *rope {
      Rope::App(a) => a,
      _ => panic!("Expeted an App"),
    }
  }

  #[inline]
  fn take_leaf(rope: Box<Rope<T>>) -> Vec<T> {
    match *rope {
      Rope::Leaf(a) => a,
      _ => panic!("Expeted a Leaf"),
    }
  }

  // #[inline]
  // fn unwrap_app(&mut self) -> &mut App {
  //   match self {
  //     Rope::App(a) => a,
  //     default => panic!("Expected an App"),
  //   }
  // }

  #[inline]
  fn take_children(&mut self) -> (Box<Rope<T>>, Box<Rope<T>>) {
    self.unwrap_mut_app().take_children()
  }

  #[inline]
  fn set_children(&mut self, left: Box<Rope<T>>, right: Box<Rope<T>>) {
    self.unwrap_mut_app().set_children(left, right);
  }

  fn rebalance(&mut self) {
    // Here's where things get real.
    // Should we move this into join?  We want to be able to
    // do a simple rebalancing after splicing in/out data.
    // Moving to join would make it more of a persistent deal?
    // Maybe we need to write splice?
    if let Rope::Leaf(..) = self { return; }
    loop {
      let app = self.unwrap_mut_app();
      if app.length < THRESHOLD {
        *self = Rope::Leaf(self.to_vec());
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
      Rope::Leaf(..) => true,
      Rope::App(a) => {
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
        Rope::Leaf(a) => {
          let l = a.len();
          vec[index .. index + l].copy_from_slice(a);
          index += l;
        }
        Rope::App(a) => {
          stack.push(a.unwrap_right());
          stack.push(a.unwrap_left());
        }
      }
    }
    return vec;
  }

  // Takes ownership of both, so we can just concat the ropes
  fn join(left: Box<Rope<T>>, right: Box<Rope<T>>) -> Box<Rope<T>> {
    // NOTE: assume that ropes are balanced on the way in
    if left.len() == 0 {
      return right;
    } else if right.len() == 0 {
      return left;
    }
    let depth = cmp::max(left.dep(), right.dep()) + 1;
    let length = left.len() + right.len();
    let app = App{left: Some(left), right: Some(right), depth, length};
    let mut rope = Box::new(Rope::App(app));
    rope.rebalance();
    rope
  }

  // Takes ownership of the box, but likely returns the same one.
  // At the very least, this invalidates any cursors.
  pub fn splice(mut rope: Box<Rope<T>>, start: usize, length: usize, insert: Option<Vec<T>>) -> Box<Rope<T>> {
  // }

  // pub fn splice(&mut self, start: usize, length: usize, insert: Option<Vec<T>>) {
    match rope.as_ref() {
      Rope::Leaf(a) => {
        match insert {
          None => {
            splice_out_leaf(rope, start, length)
          }
          Some(v) => {
            if start == 0 {
              replace_prefix(rope, length, v)
            } else if start + length == a.len() {
              replace_suffix(rope, start, v)
            } else {
              join3(rope, start, length, v)
            }
          }
        }
      }
      Rope::App(a) => {
        let left = a.left.as_ref().unwrap();
        let left_len = left.len();
        if start >= left_len {
          let a = Rope::take_app(rope);
          let right = a.right.unwrap();
          let mut rope = Rope::splice(right, start - left_len, length, insert);
          rope.rebalance();
          rope
        } else if start + length <= left_len {
          let a = Rope::take_app(rope);
          let left = a.left.unwrap();
          let mut rope = Rope::splice(left, start, length, insert);
          rope.rebalance();
          rope
        } else {
          // Remove parts of both.
          let (mut left, mut right) = rope.take_children();
          let left_part = left_len - start;
          let right_part = length - left_part;
          left = Rope::splice(left, start, left_part, None);
          left.rebalance();
          right = Rope::splice(right, 0, right_part, insert);
          right.rebalance();
          rope.unwrap_mut_app().set_children(left, right);
          rope.rebalance();
          rope
        }
      }
    }
  }
}


// Problem:
//  - We want to move the box into the splice method
//    so that we can consume any nodes inside.
//  - BUT, we don't want to have to return the whole
//    thing back since that's a lot of reassignments
//  - Passing a `&mut Box` doesn't cut it because we
//    can't move out of the borrowed reference.
//  - It may be that moving in/out isn't a problem?
//     - just an extra few stores?

fn splice_out_leaf<T: Copy>(rope: Box<Rope<T>>,
                            start: usize, length: usize) -> Box<Rope<T>> {
  match rope {
    Rope::App(_) => unreachable!(),
    Rope::Leaf(arr) => {
      let end = start + length;
      let arr_len = arr.len();
      let new_len = arr_len - length;
      if new_len < THRESHOLD || start == 0 || end == arr_len {
        // collapse and truncate
        arr.copy_within(end .., start);
        arr.truncate(new_len);
        rope
      } else {
        // split into an App: note - nonzero on both sides!
        let right = Some(Box::new(Rope::Leaf(Vec::from(&arr[end ..]))));
        let left: Rope<T> = *rope;
        arr.truncate(start);
        Box::new(Rope::App(App{
          left: Some(Box::new(left)),
          right,
          length: new_len,
          depth: 1,
        }))
      }
    }
  }
}

fn replace_prefix<T: Copy>(rope: Box<Rope<T>>,
                           start: usize, insert: Vec<T>) -> Box<Rope<T>> {
  panic!()

}

fn replace_suffix<T: Copy>(rope: Box<Rope<T>>,
                           end: usize, insert: Vec<T>) -> Box<Rope<T>>{
  panic!()

}

fn join3<T: Copy>(rope: Box<Rope<T>>, start: usize, length: usize,
                  insert: Vec<T>) -> Box<Rope<T>> {
  panic!()

}


// fn splice_insert_leaf<T: Copy>(rope: &mut Box<Rope<T>>,
//                                start: usize, length: usize,
//                                insert: Vec<T>) {
//   match **rope {
//     Rope::App(_) => unreachable!(),
//     Rope::Leaf(arr) => {
//       let insert_len = insert.len();
//       let orig_end = start + length;
//       let new_end = start + insert_len;
//       let arr_len = arr.len();
//       let new_len = arr_len - length + insert_len;
//       if new_len < THRESHOLD {
//         // copy within and resize
//         if length < insert_len {
//           // grow
//           unsafe { arr.set_len(new_len); }
//           arr.copy_within(orig_end.., new_end);
//           &arr[orig_end..new_end].copy_from_slice(&insert);
//         } else {
//           // shrink
//           arr.copy_within(orig_end.., new_end);
//           &arr[orig_end..new_end].copy_from_slice(&insert);
//           arr.truncate(new_len);
//         }
//       } else if start < a_len - (start + length) {
//         // smaller on left of splice point
//         //let left = 

//       } else {
//         // smaller on right of splice point
//       } else {
//         // split into an App
//         let right = Some(Box::new(Rope::Leaf(Vec::from(&arr[end ..]))));
//         let left: Rope<T> = **rope;
//         arr.truncate(start);
//         *rope = Box::new(Rope::App(App{
//           left: Some(Box::new(left)),
//           right,
//           length: new_len,
//           depth: 1,
//         }));
//       }
//     }
//   }
// }



//     match self {
//       Rope::Leaf(a) => {
//         let a_len = a.len();
//         let new_length = a_len - length + insert_len;
//         if new_length < THRESHOLD {
//           direct_splice(a, start, length, insert_len, insert.as_ref());
//         } else if insert_len == 0 {
//           // break into an App node
//           let left = &a[..start];
//           let right = &a[start+length..];
//           *self = Rope::App(App{
//             left: Some(Box::new(Rope::Leaf(Vec::from(left)))),
//             right: Some(Box::new(Rope::Leaf(Vec::from(right)))),
//             length: new_length,
//             depth: 1,
//           });
//           return;
//         } else if start < a_len - (start + length) {
//           // smaller on left of splice point
//           let left = 
          
//         } else {
//           // smaller on right of splice point
//         }
//       }
//       Rope::App(a) => {

//       }
//     }

// fn direct_splice<T: Copy>(arr: &mut Vec<T>,
//                           start: usize,
//                           length: usize,
//                           insert_len: usize,
//                           insert: Option<&Vec<T>>) {
//   let a_len = arr.len();
//   let new_length = a_len - length + insert_len;
//   if insert_len >= length { // growing: change len first
//     unsafe { arr.set_len(new_length); }
//     arr.copy_within((start + length) .., start + insert_len);
//   } else { // shrinking: move first
//     arr.copy_within((start + length) .., start + insert_len);
//     arr.truncate(new_length);
//   }
//   if insert_len > 0 {
//     arr[start .. start + insert_len].copy_from_slice(&insert.unwrap());
//   }
// }

pub struct RopeCursor<'a, T: Copy> {
  root: &'a Rope<T>,
  stack: Vec<&'a Rope<T>>,
  start: usize,
  cur: Option<&'a [T]>,
}

impl<'a, T: Copy> RopeCursor<'a, T> {
  fn new(root: &'a Rope<T>) -> Self {
    RopeCursor{root, stack: vec![], start: 0, cur: None}
  }

  fn at(&mut self, mut index: usize) -> T {
    if index >= self.start {
      match self.cur {
        Some(slice) if index < self.start + slice.len() => {
          return slice[index - self.start];
        }
        _ => ()
      }
    } else {
      self.start = 0;
      self.cur = None;
      self.stack = vec![self.root];
    }
    index -= self.start;
    while let Some(top) = self.stack.pop() {
      let len = top.len();
      if index >= len {
        self.start += len;
        index -= len;
        continue;
      }
      match top {
        Rope::Leaf(a) => {
          self.cur = Some(a);
          return a[index];
        }
        Rope::App(a) => {
          self.stack.push(a.unwrap_right());
          self.stack.push(a.unwrap_left());
        }
      }
    }
    panic!("Index out of bounds!");
  }
}

impl<T: BaseLike> Rope<T> {
  fn from_bases(s: &str) -> Box<Rope<T>> {
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
    Box::new(Rope::Leaf(vec.into()))
  }

  fn base_str(&self) -> String {
    let mut buf = String::with_capacity(self.len());
    let mut stack: Vec<&Rope<T>> = vec![&self];
    while stack.len() > 0 {
      match stack.pop().unwrap() {
        Rope::Leaf(arr) => {
          for base in arr.iter() {
            match base.to_base() {
              Base::I => buf.push('I'),
              Base::C => buf.push('C'),
              Base::F => buf.push('F'),
              Base::P => buf.push('P'),
            }
          }
        }
        Rope::App(a) => {
          stack.push(a.unwrap_right());
          stack.push(a.unwrap_left());
        }
      }
    }
    buf
  }
}
