use std::cell::Cell;
use std::rc::Rc;
use std::ptr;

fn main() {
  let r = rope("IICIFPIFCPCIFP");
  println!("{}", rope_str(&r));
}

const THRESHOLD: usize = 500;

#[repr(u8)]
#[derive(Clone,Copy)]
enum Base {
  I = 0,
  C = 1,
  F = 2,
  P = 3,
}

#[derive(Clone)]
enum Rope {
  App {
    left: Rc<Rope>,
    right: Rc<Rope>,
    length: usize,
    depth: i8,
  },
  Leaf(Rc<[Base]>),
  Top(Rc<Rope>, Cell<usize>, Cell<*const [Base]>),
}

impl Rope {
  fn dep(&self) -> i8 {
    match self {
      Rope::Leaf(_) => 0,
      Rope::App{depth, ..} => *depth,
      Rope::Top(rope, ..) => rope.dep(),
    }
  }

  fn len(&self) -> usize {
    match self {
      Rope::Leaf(a) => a.len(),
      Rope::App{length, ..} => *length,
      Rope::Top(rope, ..) => rope.len(),
    }
  }

  fn fingeredAt(&self, index: usize, origIndex: usize, top: &Rope) -> Base {
    match self {
      Rope::Leaf(a) => {
        match top {
          Rope::Top(_, ind, leaf) => {
            ind.set(origIndex - index);
            leaf.set(&**a);
          }
          default => {}
        }
        a[index]
      }
      Rope::App{left, right, ..} => {
        let l = left.len();
        if index < l {
          left.fingeredAt(index, origIndex, top)
        } else {
          right.fingeredAt(index - l, origIndex, top)
        }
      }
      Rope::Top(rope, ..) => {
        rope.fingeredAt(index, origIndex, top)
      }
    }
  }

  fn at(&self, index: usize) -> Base {
    self.fingeredAt(index, index, self)
  }

  #[inline]
  fn unsafe_left(&self) -> &Rope {
    match self {
      Rope::App{left, ..} => left,
      Rope::Top(rope, ..) => rope.unsafe_left(),
      default => panic!("Expected an App"),
    }
  }

  #[inline]
  fn unsafe_right(&self) -> &Rope {
    match self {
      Rope::App{right, ..} => right,
      Rope::Top(rope, ..) => rope.unsafe_right(),
      default => panic!("Expected an App"),
    }
  }

  fn balance(&self) -> Rope {
    match self {
      Rope::Leaf(..) => self.clone(),
      Rope::App{left, right, ..} => {
        let dl = left.dep();
        let dr = right.dep();
        if (dl - dr).abs() <= 1 {
          return self.clone();
        }
        if dl > dr {
          // left taller
          let ll = left.unsafe_left();
          let lr = left.unsafe_right();
          if lr.dep() > ll.dep() {
            // Need to figure out ownership here...!
            join(join(ll, lr.unsafe_left()),
                 join(lr.unsafe_right(), right))
          } else {
            join(ll, join(lr, right))
          }
        } else {
          // right taller
          let rl = right.unsafe_left();
          let rr = right.unsafe_right();
          if rl.dep() > rr.dep() {
            join(join(left, rl.unsafe_left()),
                 join(rl.unsafe_right(), rr))
          } else {
            join(join(left, rl), rr)
          }
        }
      }
      Rope::Top(rope, i, l) => panic!("top inside balance")
    }
  }

  fn toLeaf(&self) -> Rope {
    match self {
      Rope::Leaf(a) => self,
      Rope::Top(rope, ..) => rope.toLeaf(),
      Rope::App(left, right, ..) => {
        let l = left.len();
        let r = right.len();
        let mut buf = Vec::with_capacity(l + r);
        let mut stack: Vec<&Rope> = Vec::new();
        let mut index: usize = 0;
        stack.push(right);
        stack.push(left);
        while stack.len() > 0 {
          match stack.pop().unwrap() {
            Rope::Top(rope) => {
              stack.push(rope);
            }
            Rope::Leaf(arr) => {
              let a = arr.len();
              buf[index .. index + l].copy_from_slice(arr);
              index += l;
            }
            Rope::App{left, right, ..} => {
              stack.push(&*right);
              stack.push(&*left);
            }
          }
        }
        Rope::Leaf(buf.into())
      }
    }
  }
}

// impl Copy for Rope {
//     #[inline]
//     fn copy(&self) -> Rope {
//         match self {
//             Rope::Leaf(arr) => Rope::Leaf(arr.clone()),
//             Rope::App({left, right, length, depth}) =>
//                 Rope::App({left: left.clone(), right: right.clone(), length, depth}),
//         }
//     }
// }



fn join(mut left: &Rope, mut right: &Rope) -> Rope {
  let l = left.len();
  let r = right.len();
  if l == 0 { return right.clone(); }
  if r == 0 { return left.clone(); }
  if l + r < THRESHOLD {
  }
  // TODO - balance height
  let mut dl = left.dep();
  let mut dr = right.dep();
  while dl - dr > 1 || dr - dl > 1 {
    if dl > dr {
      let ll = left.unsafe_left();
      let lr = left.unsafe_right();
      if lr.dep() > ll.dep() {
        // Problem - left/right want refs... who owns lifetime?
        left = join(ll, lr.unsafe_left());
        dl = left.dep();
        right = join(lr.unsafe_right(), right);
        dr = right.dep();
      } else {
        dl = (left = ll).dep();
        dr = (right = join(lr, right)).dep();
      }
    } else {
      panic!();
      
    }

  }
  panic!();
}

fn rope(s: &str) -> Rope {
  let mut vec = Vec::new();
  for c in s.chars() {
    vec.push(match c {
      'I' => Base::I,
      'C' => Base::C,
      'F' => Base::F,
      'P' => Base::P,
      _ => Base::I, // panic?
    })
  }
  //let slice: [Base] = &vec;
  Rope::Leaf(vec.into())
}

fn rope_str(r: &Rope) -> String {
  let mut buf = String::with_capacity(r.len());
  let mut stack: Vec<&Rope> = Vec::new();
  stack.push(r);
  while stack.len() > 0 {
    match stack.pop().unwrap() {
      Rope::Leaf(arr) => {
        for base in arr.iter() {
          match base {
            Base::I => buf.push('I'),
            Base::C => buf.push('C'),
            Base::F => buf.push('F'),
            Base::P => buf.push('P'),
          }
        }
      }
      Rope::App{left, right, ..} => {
        stack.push(&*right);
        stack.push(&*left);
      }
    }
  }
  buf
}
