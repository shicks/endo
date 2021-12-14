use std::cmp::max;
use rope::*;
use base::BaseLike;

fn find<T: BaseLike>(r: &Rope<T>, needle: &[T], start: usize) -> Option<usize> {
  let needle_len = needle.len();
  if needle_len == 0 { return Some(start); }
  let mut haystack = r.cursor();
  let haystack_len = r.len();
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
    let haystack = from_str("ICFPIICFCPFIICICFC").collect::<Rope<SourceBase>>();
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
