import {AbstractDna, Rope} from './dna.js';

const THRESHOLD = 500;

export class RopeDna extends AbstractDna<string> {
  base(str: string) {
    const x = {I:0,C:1,F:2,P:3}[str];
    if (x == null) throw new Error(`Invalid base: ${str}`);
    return x;
  }
  fromBase(n: number) {
    return 'ICFP'[n & 3];
  }
  isRope(arg: unknown): arg is Rope<string> {
    return typeof arg === 'string';
  }
  rope(str: string): Rope<string> {
    return new StringRope(str);
  }
  init(str: string): Rope<string> {
    return new StringRope(str);
  }
}

class StringRope implements Rope<string> {
  readonly length: number;
  constructor(readonly node: Node) {
    this.length = node.length;
  }
  * [Symbol.iterator]() {
    const stack: Node[] = [this.node];
    let cur: Node;
    while (stack.length) {
      if (!isLeaf(cur = stack.pop()!)) {
        stack.push(cur.right, cur.left);
      } else {
        yield* cur;
      }
    }
  }
  at(index: number) {
    return at(this.node, index);
  }
  find(needle: Iterable<string>, start: number) {
    return find(this.node, str(needle), start);
  }
  slice(start: number, end: number = this.length): Rope<string> {
    return new StringRope(slice(this.node, start, end));
  }
  splice(start: number, length: number,
         insert: Iterable<string> = []): Rope<string> {
    return new StringRope(splice(this.node, start, length, str(insert)));
  }
  toString() {
    return toString(this.node);
  }
}

interface App {
  left: Node;
  right: Node;
  depth: number;
  length: number;
}
type Node = string|App;
function depth(node: Node) {
  return (node as App).depth || 0;
}
function isLeaf(node: Node): node is string {
  return typeof node === 'string';
}
function str(s: Iterable<string>): string {
  return typeof s === 'string' ? s : [...s].join('');
}

function toString(n: Node): string {
  let s = '';
  const stack: Node[] = [n];
  while (stack.length) {
    const cur = stack.pop()!;
    if (isLeaf(cur)) {
      s += cur;
    } else {
      stack.push(cur.right, cur.left);
    }
  }
  return s;
}


function at(n: Node, i: number) {
  while (!isLeaf(n)) {
    const l = n.left.length;
    if (i < l) {
      n = n.left;
    } else {
      n = n.right;
      i -= l;
    }
  }
  return n[i];
}

function find(haystack: Node, needle: string, start: number): number {
  let index = start;
  const needleLen = needle.length;
  const charTable = boyerMooreCharTable(needle);
  const offsetTable = boyerMooreOffsetTable(needle);
  for (let i = index + needleLen - 1, j; i < haystack.length;) {
    let c!: string;
    for (j = needleLen - 1;
         needle[j] === (c = at(haystack, i));
         --i, --j) {
      if (j === 0) {
        return i;
      }
    }
    i += Math.max(offsetTable[needleLen - 1 - j], charTable[c]);
  }
  return -1;
}

function slice(n: Node, start: number, end: number): Node {
  let node!: Node;
  const stack: Node[] = [n];
  while (stack.length) {
    const cur = stack.pop()!;
    if (start > cur.length) {
      start -= cur.length;
      end -= cur.length;
    } else if (isLeaf(cur)) {
      if (end <= cur.length) {
        return cur.substring(start, end);
      }
      node = cur.substring(start);
      end -= cur.length;
      break;
    } else {
      stack.push(cur.right, cur.left);
    }
  }
  if (!node) return ''; // never found start?
  while (stack.length) {
    const cur = stack.pop()!;
    if (end >= cur.length) {
      end -= cur.length;
      node = join(node, cur);
    } else if (isLeaf(cur)) {
      node = join(node, cur.substring(0, end));
      break;
    } else {
      stack.push(cur.right, cur.left);
    }
  }
  return node;
}

function splice(n: Node, start: number, len: number, insert: string): Node {
  let node: Node = '';
  const stack: Node[] = [n];
  while (stack.length) {
    const cur = stack.pop()!;
    if (start > cur.length) {
      start -= cur.length;
      node = join(node, cur);
    } else if (isLeaf(cur)) {
      node = join(node, cur.substring(0, start));
      if (start + len <= cur.length) {
        stack.push(cur.substring(start + len));
        len = 0;
        break;
      } else {
        len -= (cur.length - start);
      }
    } else {
      stack.push(cur.right, cur.left);
    }
  }
  while (len && stack.length) {
    const cur = stack.pop()!;
    if (len >= cur.length) {
      len -= cur.length;
    } else if (isLeaf(cur)) {
      stack.push(cur.substring(length));
      break;
    } else {
      stack.push(cur.right, cur.left);
    }
  }
  if (insert) node = join(node, insert);
  while (stack.length) {
    node = join(node, stack.pop()!);
  }
  return node;
}

function join(left: Node, right: Node): Node {
  // Move nodes around until we're balanced...
  if (!left.length) return right as App;
  if (!right.length) return left as App;
  if (left.length + right.length < THRESHOLD) {
    return toString(left) + toString(right);
  }
  let dl = depth(left);
  let dr = depth(right);
  while (Math.abs(dl - dr) > 1) {
    if (dl > dr) {
      // left is taller
      const l = left as App;
      if (depth(l.right) > depth(l.left)) {
        // middle is tallest: split it up
        const lr = l.right as App;
        dl = depth(left = join(l.left, lr.left));
        dr = depth(right = join(lr.right, right));
      } else {
        // rotate middle to right
        dl = depth(left = l.left);
        dr = depth(right = join(l.right, right));
      }
    } else {
      // right is taller
      const r = right as App;
      if (depth(r.left) > depth(r.right)) {
        // middle is tallest: split it up
        const rl = r.left as App;
        dl = depth(left = join(left, rl.left));
        dr = depth(right = join(rl.right, r.right));
      } else {
        // rotate middle to left
        dl = depth(left = join(left, r.left));
        dr = depth(right = r.right);
      }
    }
  }
  // balanced, just join
  return {
    left, right,
    depth: Math.max(dl, dr) + 1,
    length: left.length + right.length,
  };
}

// needle is an array of [0..3], return is a 4-element array
function boyerMooreCharTable(needle: string): Record<string, number> {
  const len = needle.length;
  const table = {'I': len, 'C': len, 'F': len, 'P': len};
  for (let i = 0; i < needle.length - 1; i++) {
    table[needle[i] as 'I'|'C'|'F'|'P'] = len - 1 - i;
  }
  return table;
}
function boyerMooreOffsetTable(needle: string): number[] {
  const len = needle.length;
  const table: number[] = [];
  let lastPrefixPos = len;
  for (let i = len; i > 0; i--) {
    let isPrefix = true;
    for (let ii = i, j = 0; ii < needle.length; ii++, j++) {
      if (needle[ii] !== needle[j]) {
        isPrefix = false;
        break;
      }
    }
    if (isPrefix) lastPrefixPos = i;
    table[len - i] = lastPrefixPos - i + len;
  }
  for (let i = 0; i < len - 1; i++) {
    let slen = 0;
    for (let ii = i, j = len - 1;
         ii >= 0 && needle[ii] === needle[j]; ii--, j--) {
      slen++;
    }
    table[slen] = len - 1 - i + slen;
  }
  return table;
}

