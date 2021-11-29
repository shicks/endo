//import {Deque} from './deque';

const LEAF_SIZE = 200;

interface D {
  readonly depth: number;
}

// interface IRope {
//   substring(start: number, end?: number): Rope;
//   indexOf(needle: string, start?: number): number;
//   charAt(index: number): string;
// }
// interface IRopeCtor {
//   of(str: string): Rope;
//   cat(...strs: Rope[]): Rope;
//   splice(start: number, length: number, content?: Rope): Rope;
// }

export class Rope {
  private constructor(private readonly node: Node) {}

  append(right: Rope|string) {
    if (!this.node) return right instanceof Rope ? right : new Rope(right);
    const rightNode = right instanceof Rope ? right.node : right;
    if (!rightNode) return this;
    return new Rope(append(this.node, rightNode));
  }

  substring(start: number, end?: number): Rope {
    const n = subrope(this.node, start, end);
    if (n === this.node) return this;
    return new Rope(n);
  }

  indexOf(needle: string, startIndex = 0): number {
    return find(this.node, needle, startIndex);
  }

  toString() {
    return toString(this.node);
  }

  get length() {
    return this.node.length;
  }

  static of(str: string): Rope {
    return new Rope(str);
  }

  debugString() {
    return debugString(this.node);
  }
}

function debugString(n: Node): string {
  return isLeaf(n) ? n : `(${debugString(n.left)})(${debugString(n.right)})`;
}

const fibArray = [1, 2];
function fib(i: number): number {
  while (i >= fibArray.length) {
    fibArray.push(
        fibArray[fibArray.length - 2] + fibArray[fibArray.length - 1]);
  }
  return fibArray[i];
}
const sqrt5 = Math.sqrt(5);
const ilogPhi = 1 / Math.log((sqrt5 + 1) / 2);
function ifib(n: number): number {
  // [0, 1): 0;  [1, 2): 1;  [2, 3): 2;  [3, 5): 3;  [5, 8): 4 ...
  if (n <= 0) return 0;
  let i = Math.floor(ilogPhi * Math.log(n * sqrt5 + 0.5) - 1);
  // Correct any bad offsets
  while (n < fib(i - 1)) i--;
  while (n >= fib(i)) i++;
  return i;
}

declare const BALANCED: unique symbol;
type Balanced = string | (Concat & {[BALANCED]: true});
interface Concat {
  readonly depth: number;
  readonly length: number;
  readonly left: Node;
  readonly right: Node;
}
type Node = string|Concat;

function isLeaf(n: Node): n is string {
  return typeof n === 'string';
}

function toString(root: Node): string {
  if (isLeaf(root)) return root;
  let str = '';
  const stack: Node[] = [root];
  let node: Node|undefined;
  while ((node = stack.pop())) {
    if (isLeaf(node)) {
      str += node;
    } else {
      stack.push(node.right, node.left);
    }
  }
  return str;
}

function concat(left: Node, right: Node): Node {
  const length = left.length + right.length;
  if (length < LEAF_SIZE) return left.toString() + right.toString();
  const depth = Math.max((left as D).depth || 0, (right as D).depth || 0) + 1;
  return {depth, length, left, right};
}

// basically same as concat but will maybe rebalance...
function append(left: Node, right: Node): Node {
  if (!right.length) return left;
  if (!left.length) return right;
  if (left.length + right.length < LEAF_SIZE) {
    return toString(left) + toString(right);
  }
  return maybeRebalance(concat(left, right));
}

function isBalanced(n: Node): n is Balanced {
  if (typeof n === 'string') return true;
  return n.length >= fib(n.depth) * LEAF_SIZE;
}



function maybeRebalance(root: Node): Node {
  //console.log(`maybe rebalancing`, root);
  if (typeof root === 'string') return root;
  if (root.length >= fib(root.depth - 2) * LEAF_SIZE) return root;
  return rebalance(root);
}

function rebalance(root: Node): Node {
  // seq is decreasing in slot, chunks are in correct order
  const seq: Array<readonly [slot: number, chunk: Node]> = [];
  const stack: Node[] = [root];
  let node: Node|undefined;
  while (node = stack.pop()) {
    if (!isBalanced(node)) {
      stack.push(node.right, node.left);
    } else {
      // n is a balanced node
      let slot = ifib(node.length / LEAF_SIZE);
      while (seq[seq.length - 1]?.[0]! <= slot) {
        node = concat(seq.pop()![1], node);
        slot = ifib(node.length / LEAF_SIZE);
      }
      seq.push([slot, node]);
    }
  }
  // finally consolidate the whole stack?
  node = seq.pop()?.[1];
  if (!node) throw new Error(`empty stack`);
  while (seq.length) {
    let next = seq.pop()![1];
    node = concat(next, node);
  }
  return node;
}

function subrope(n: Node, start: number, end = n.length): Node {
  if (start <= 0 && n.length <= end) return n;
  if (isLeaf(n)) {
    return n.substring(Math.max(start, 0), Math.min(end, n.length));
  }
  const a = n.left.length;
  if (start >= a) return subrope(n.right, start - a, end - a);
  if (end <= a) return subrope(n.left, start, end);
  return concat(subrope(n.left, start, a), subrope(n.right, 0, end - a));
}

function find(haystack: Node, needle: string, startIndex: number): number {
  if (needle.length === 0) return startIndex;
//  console.log(`\n\nFIND [${needle}] in [${debugString(haystack)}] from ${startIndex}`);
  const charTable = new Map<string, number>();
  const offsetTable: number[] = [];
  // TODO - consider caching these?!?
  // Make char table
  for (let i = 0; i < needle.length - 1; i++) {
    charTable.set(needle[i], needle.length - 1 - i);
  }
//console.log(`chartable: ${[...charTable].map(([c,i])=>`${c}: ${i}`).join(', ')}`);
  // Make offset table
  {
    let lastPrefixPos = needle.length;
    for (let i = needle.length; i > 0; i--) {
      let isPrefix = true;
      for (let ii = i, j = 0; ii < needle.length; ii++, j++) {
        if (needle[ii] !== needle[j]) {
          isPrefix = false;
          break;
        }
      }
      if (isPrefix) lastPrefixPos = i;
      offsetTable[needle.length - i] = lastPrefixPos - i + needle.length;
    }
    for (let i = 0; i < needle.length - 1; i++) {
      let slen = 0;
      for (let ii = i, j = needle.length - 1;
           ii >= 0 && needle[ii] === needle[j]; ii--, j--) {
        slen++;
      }
      offsetTable[slen] = needle.length - 1 - i + slen;
    }
//console.log(`offsettable: ${offsetTable.join(', ')}`);
  }
  // Now search
  const stack: Node[] = [haystack];
  let cursor = 0;
  function charAt(index: number) {
//const orig=index;
    index -= cursor;
    let i = stack.length - 1;
    while (stack[i].length <= index) {
      index -= stack[i--].length;
    }
    let n = stack[i];
    while (!isLeaf(n)) {
      const a = n.left.length;
      if (index < a) {
        n = n.left;
      } else {
        index -= a;
        n = n.right;
      }
    }
    return n[index];
  }
  function trimTo(index: number) {
//    console.log(`trimTo(${index}): c=${cursor} stk=${stack.map(debugString).join(', ')}`);
    index -= cursor;
    let i = stack.length - 1;
    while (i >= 0 && (stack[i].length < index || !isLeaf(stack[i]))) {
      if (!isLeaf(stack[i])) {
        const n = stack.pop() as Concat;
        stack.push(n.right, n.left);
        i++;
        continue;
      }
      if (index <= 0) break;
      const len = stack.pop()!.length;
      index -= len;
      cursor += len;
      i--;
    }
//    console.log(` => c=${cursor} stk=${stack.map(debugString).join(', ')}`);
  }
  if (startIndex) trimTo(startIndex);
  for (let i = startIndex + needle.length - 1, j; i < haystack.length;) {
//console.log(`i=${i}`);
    let c!: string;
    for (j = needle.length - 1; needle[j] === (c = charAt(i)); --i, --j) {
//console.log(`j=${j}, i=${i}, c=${c}`);
      if (j == 0) {
//console.log(`found ${i}`);
        return i;
      }
    }
//console.log(`skip: offset=${offsetTable[needle.length - 1 - j]} char[${c}]=${charTable.get(c) ?? needle.length}`);
    i += Math.max(offsetTable[needle.length - 1 - j],
                  charTable.get(c) ?? needle.length);
    trimTo(i - needle.length);
  }
//console.log(`could not find`);
  return -1;
}
