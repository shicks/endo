import {AbstractDna, Rope} from './dna.js';
import {performance} from 'perf_hooks';

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
    return arg instanceof StringRope;
  }
  rope(s: Iterable<string>): Rope<string> {
    return s instanceof StringRope ? s : new StringRope(str(s));
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
    while (stack.length) {
      const cur = stack.pop()!;
      if (isLeaf(cur)) {
        yield* cur;
      } else {
        stack.push(cur.right, cur.left);
      }
    }
  }
  at(index: number) {
    return at(this.node, index);
  }
  find(needle: Iterable<string>, start: number) {
    let needleStr = str(needle);
    findStart = performance.now();
    let result = find(this.node, needleStr, start);
    findTotal += (performance.now() - findStart);
    let stat = findStats.get(needleStr);
    if (!stat) findStats.set(needleStr, stat = new FindStat());
    stat.add(start, result);
    return result;
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

let findStart!: number;
let findTotal = 0;
class FindStat {
  total = 0;
  found = 0;
  delta = new Map<number, number>();
  get notFound() { return this.total - this.found; }
  add(start: number, result: number) {
    this.total++;
    if (result < 0) return;
    this.found++;
    const delta = result - start;
    this.delta.set(delta, (this.delta.get(delta) || 0) + 1);
  }
}
const findStats = new Map<string, FindStat>();
export function reportFindStats() {
  console.log(`FIND STATS\nTotal time: ${findTotal / 1000} ms`);
  const finds = [...findStats].sort((a, b) => b[1].total - a[1].total);
  let total = 0;
  for (const [,s] of finds) total += s.total;
  console.log(`Total searches: ${total}`);
  console.log(`Unique needles: ${finds.length}`);
  console.log(`Most common:`);
  for (let i = 0; i < 20 && i < finds.length; i++) {
    const stat = finds[i][1];
    let sum1 = 0;
    let sum2 = 0;
    let count = 0;
    for (const [n, c] of stat.delta) {
      count += c;
      sum1 += c * n;
      sum2 += c * n * n;
    }
    if (count !== stat.found) console.error(`mismatch ${finds[i][0]}: ${count} vs ${stat.found}`);
    const mean = sum1 / count;
    const variance = sum2 / count - mean * mean;
    const stdev = Math.pow(variance, 0.5);
    console.log(`  ${finds[i][0]}: #${count}/${stat.total}, delta ${mean}±${stdev}`);
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
    if (start >= cur.length) {
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
  if (!node) {
//console.dir(n);console.log(origStart, origEnd);
//throw new Error('BAD SLICE 1');
return ''; // never found start?
}
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
// if (node.length !== origEnd - origStart) {
// console.dir(n);console.dir(node);console.log(origStart, origEnd);
// throw new Error(`BAD SLICE 2`);}
  return node;
}

function splice(n: Node, start: number, len: number, insert: string): Node {
// const origLen = len;
// const origStart = start;
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
      } else {
        len -= (cur.length - start);
      }
      break;
    } else {
      stack.push(cur.right, cur.left);
    }
  }
  while (len && stack.length) {
    const cur = stack.pop()!;
    if (len >= cur.length) {
      len -= cur.length;
    } else if (isLeaf(cur)) {
      stack.push(cur.substring(len));
      break;
    } else {
      stack.push(cur.right, cur.left);
    }
  }
  if (insert) node = join(node, insert);
  while (stack.length) {
    node = join(node, stack.pop()!);
  }
//  if (node.length !== n.length - origLen + insert.length) {
//console.dir(n);
//console.dir(node);
//throw new Error(`BAD SPLICE ${n.length}@${origStart} - ${origLen} + ${insert.length} ${insert}`);}
  return node;
}

function join(left: Node, right: Node): Node {
  // Move nodes around until we're balanced...
//const origLeft = left;
//const origRight = right;
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
  if (!left) throw `no left`;
  if (!right) throw `no right`;
// if (toString(left)+toString(right) !== toString(origLeft)+toString(origRight))throw new Error(`bad join`);
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

