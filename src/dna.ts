
export abstract class Dna {
  abstract readonly length: number;

  cursor(index: number): Cursor {

  }
}

const BASES = 'ICFP';

class Cursor {
  index: number;
  stack: Dna[];
  // Allows backtracking...?
  rightParents: Dna[];

  seek(index: number) {}
  find(needle: Dna): boolean {}
  next(): string {}
}

// class



    1
   / \
  2   3
 /\   /\
a  b c  d

[1]
[3,1] [2,1]
[3,1] [b,2] [a,2]  | 1
[3,1] [b,2]        | 1 2   => a
[3,1]              | 1     => b
[d,3] [c,3]        | 1 ??
