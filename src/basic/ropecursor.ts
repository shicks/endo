const LEAF_SIZE = 200;

class Rope {
  constructor(public root: Chunk) {}

  cursor(index = 0): Cursor {
    const c = new Cursor(this);
    c.skip(index);
    return c;
  }
}

class Cursor {
  index = 0;
  stack: Chunk[];
  subindex = 0;
  constructor(public rope: Rope) {
    // If we have to go backwards past something.....?
    // NOTE: Mutability makes this completely bonkers
    //  - we'd need to keep a modcount in order to have
    //    any confidence at all...
  }
}

interface Rope {
  substring(start: number, end?: number): Rope;
  indexOf(needle: string, start?: number): number;
  charAt(index: number): string;
  [Symbol.iterator](): Iterator<string>;
  readonly length: number;
}
interface RopeCtor {
  cat(...strs: Rope[]): Rope;
  splice(r: Rope, start: number, length: number, content?: Rope): Rope;
  debugString(r: Rope): string;
  find(haystack: Rope, needle: string, startAt?: number): number;
}

const Rope: RopeCtor = {
  find,
  splice,

  cat(...ropes: Rope[]): Rope {
    if (!ropes.length) return '';
    if (ropes.length === 1) return ropes[0];
    if (ropes.length === 2) return append(ropes[0], ropes[1]);
    // Any more than two: build with strong balance
    return rebalance(ropes.reverse());
  },

  debugString(n: Rope): string {
    return n instanceof Concat ?
        `(${Rope.debugString(n.left)})(${Rope.debugString(n.right)})` : String(n);
  },
}

export {Rope};

class Concat implements Rope {
  readonly depth: number;
  constructor(readonly left: Rope, readonly right: Rope,
              readonly length = left.length + right.length) {
    this.depth = Math.max((left as D).depth || 0, (right as D).depth || 0) + 1;
  }

  substring(start: number, end = this.length): Rope {
    if (start <= 0 && this.length <= end) return this;
    const a = this.left.length;
    if (start >= a) return this.right.substring(start - a, end - a);
    if (end <= a) return this.left.substring(start, end);
    return concat(this.left.substring(start, a),
                  this.right.substring(0, end - a));
  }

  indexOf(needle: string, startIndex = 0): number {
    return find(this, needle, startIndex);
  }

  charAt(index: number): string {
    return this.substring(index, index + 1).toString();
  }

  * [Symbol.iterator](): Iterator<string> {
    const stack: Rope[] = [this];
    let node: Rope|undefined;
    while ((node = stack.pop())) {
      if (node instanceof Concat) {
        stack.push(node.right, node.left);
      } else {
        yield* node;
      }
    }
  }

  toString() {
    return toString(this);
  }
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
type Unbalanced = Concat & {[BALANCED]: false};

function isLeaf(n: Rope): n is string {
  return typeof n === 'string';
}

function toString(root: Rope): string {
  if (isLeaf(root)) return root;
  let str = '';
  const stack: Rope[] = [root];
  let node: Rope|undefined;
  while ((node = stack.pop())) {
    if (node instanceof Concat) {
      stack.push(node.right, node.left);
    } else {
      str += node;
    }
  }
  return str;
}

// this version has fewer checks for balance
function concat(left: Rope, right: Rope): Rope {
  const length = left.length + right.length;
  if (length < LEAF_SIZE) return left.toString() + right.toString();
  return new Concat(left, right, length);
}

// basically same as concat but will maybe rebalance...
function append(left: Rope, right: Rope): Rope {
  if (!right.length) return left;
  if (!left.length) return right;
  if (left.length + right.length < LEAF_SIZE) {
    return toString(left) + toString(right);
  }
  return maybeRebalance(concat(left, right));
}

function isUnbalanced(n: Rope): n is Unbalanced {
  return n instanceof Concat && n.length < fib(n.depth) * LEAF_SIZE;
}


function maybeRebalance(root: Rope): Rope {
  if (!(root instanceof Concat)) return root;
  if (root.length >= fib(root.depth - 2) * LEAF_SIZE) return root;
  return rebalance([root]);
}

function rebalance(stack: Rope[]): Rope {
  // seq is decreasing in slot, chunks are in correct order
  const seq: Array<readonly [slot: number, chunk: Rope]> = [];
  let node: Rope|undefined;
  while (node = stack.pop()) {
    if (isUnbalanced(node)) {
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

function find(haystack: Rope, needle: string, startIndex: number): number {
  if (needle.length === 0) return startIndex;
  const charTable = new Map<string, number>();
  const offsetTable: number[] = [];
  // TODO - consider caching these?!?
  // Make char table
  for (let i = 0; i < needle.length - 1; i++) {
    charTable.set(needle[i], needle.length - 1 - i);
  }
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
  }
  // Now search
  const stack: Rope[] = [haystack];
  let cursor = 0;
  function charAt(index: number) {
    index -= cursor;
    let i = stack.length - 1;
    while (stack[i].length <= index) {
      index -= stack[i--].length;
    }
    let n = stack[i];
    while (n instanceof Concat) {
      const a = n.left.length;
      if (index < a) {
        n = n.left;
      } else {
        index -= a;
        n = n.right;
      }
    }
    return (n as string)[index];
  }
  function trimTo(index: number) {
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
  }
  if (startIndex) trimTo(startIndex);
  for (let i = startIndex + needle.length - 1, j; i < haystack.length;) {
    let c!: string;
    for (j = needle.length - 1; needle[j] === (c = charAt(i)); --i, --j) {
      if (j == 0) {
        return i;
      }
    }
    i += Math.max(offsetTable[needle.length - 1 - j],
                  charTable.get(c) ?? needle.length);
    trimTo(i - needle.length);
  }
  return -1;
}

function splice(r: Rope, start: number, length: number, insert?: Rope): Rope {
  const left = r.substring(0, start);
  const right = r.substring(start + length);
  return insert ? Rope.cat(left, insert, right) : Rope.cat(left, right);
}
