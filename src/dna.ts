// Dna is a specialized rope whose elements must be one of 0..3, corresponding
// to the four bases, ICFP.  These are stored in the low 2 bits of an int32,
// with the next 24 bits storing the "source map" of where the base originally
// came from, and the upper 6 bits storing a (signed) escape level between -30
// and +30.  ±31 is used as "infinity", and -32 is a NaN, i.e. it didn't
// come directly from a base in the source.

export class Dna {
  constructor(readonly node: Node) {}

  cursor(index: number = 0): Cursor {
    return new Cursor(this.node, index);
  }

  static of(str: string): Dna {
    return new Dna(Uint32Array.from(str, (c, i) => INV_BASES.get(c)! | i << 2));
  }
}

// Maintain an AVL tree
type Node = App | Str;
type Str = Uint32Array;
interface App {
  readonly left: Node;
  readonly right: Node;
  readonly length: number;
  readonly depth: number;
}

const BASES = 'ICFP';
const INV_BASES = new Map([...BASES].map((v, i) => [v, i] as const));

class Cursor {
  // Index within the entire string.
  index: number;
  // Stack of parents
  stack: Array<readonly [node: App, branch: number]> = [];
  // Current node
  cur: Node
  // Index within the current leaf.
  pos: number;
  // Total length
  length: number;

  constructor(node: Node, index: number = 0) {
    this.length = node.length;
    this.index = this.pos = index;
    this.cur = node;
  }

  descend() {
    while (!isLeaf(this.cur)) {
      const leftLen = this.cur.left.length;
      if (this.pos < leftLen) {
        this.stack.push([this.cur, 0]);
        this.cur = this.cur.left;
      } else {
        this.stack.push([this.cur, 1]);
        this.cur = this.cur.right;
        this.pos -= leftLen;
      }
    }
    // Now we're pointing at a leaf?
  }

  // Only need to call this.ascend() after changing index/pos
  ascend() {
    while (this.pos < 0 || this.pos >= this.cur.length) {
      if (!this.stack.length) return;
      const [node, dir] = this.stack.pop()!;
      // NOTE: if we allowed k-ary trees, then this would be a
      // for loop from dir-1 down to 0, adding the lengths
      if (dir) this.pos += node.left.length;
      this.cur = node;
    }
  }

  seek(index: number) {
    this.pos += (index - this.index);
    this.index = index;
    this.ascend();
  }

  find(needle: Dna): boolean {
    throw 'not implemented';
  }

  next(): number|undefined {
    if (this.index >= this.length) return undefined;
    this.descend();
    if (!isLeaf(this.cur)) return undefined;
    const result = this.cur[this.pos];
    if (result == null) return undefined;
    this.pos++;
    this.index++;
    this.ascend();
    return result;
  }
  nextStr(): string {
    const num = this.next();
    return num != null ? BASES[num & 3] : '';
  }

  prev(): number|undefined {
    if (this.index <= 0) return undefined;
    this.descend();
    if (!isLeaf(this.cur)) return undefined;
    this.pos--;
    this.index--;
    const result = this.cur[this.pos];
    this.ascend();
    return result;
  }
  prevStr(): string {
    const num = this.prev();
    return num != null ? BASES[num & 3] : '';
  }

  // Return just the suffix from the cursor
  suffix(): Node {
    this.descend();
    let stack = [...this.stack];
    let node = this.cur;
    if (this.pos > 0) {
      if (!isLeaf(node)) throw new Error(`splitting on non-leaf`);
      node = node.subarray(this.pos);
    }
    while (stack.length) {
      const [parent, dir] = stack.pop()!
      node =
          dir ? node :
          node === parent.left ? parent :
          join(node, parent.right);
    }
    return node;
  }

  peek(): string {
    this.descend();
    if (!isLeaf(this.cur)) return '';
    const num = this.cur[this.pos];
    if (num == null) return '';
    return BASES[num & 3];
  }

  peek2(): string {
    this.descend();
    if (!isLeaf(this.cur)) return '';
    if (this.pos + 1 < this.cur.length) {
      return BASES[this.cur[this.pos + 1] & 3];
    }
    for (let i = this.stack.length - 1; i >= 0; i--) {
      if (this.stack[i][1]) continue; // skip right-hand parents
      let node = this.stack[i][0].right;
      while (!isLeaf(node)) node = node.left;
      return BASES[node[0] & 3];
    }
    return '';
  }

  patternItem(): PItem|undefined {
    const first = this.peek();
    if (first === 'I') {
      const second = this.peek2();
      if (second === 'I') {
        // need the third (note: this may push past end)
        const op = this.slice(3)!;
        switch (indexStr(op, 2)) {
          case 'I': return {type: 'emit', op, rna: this.slice(7)!};
          case 'C': case 'F': return {type: 'group', op, open: false};
          case 'P': return {type: 'group', op, open: true};
          default: return undefined;
        }
      } else if (second === 'C') {
        // start a run of escaped chars
        return this.bases();
      } else if (second === 'F') {
        // two-char op: IF (search)
        const op = this.slice(3)!; // throwaway
        return {type: 'search', op, query: this.consts()};
      } else if (second === 'P') {
        // two-char op: IP (skip)
        const op = this.slice(2)!;
        const [node, count] = this.nat();
        return {type: 'skip', op: join(op, node), count};
      } else {
        this.next(); // push us off the end
      }
    } else if (first) {
      // start a run of escaped chars
      return this.bases();
    }
    return undefined;
  }

  slice(count: number): Node|undefined {
    let node!: Node|undefined;
    while (count > 0) {
      this.descend();
      if (!isLeaf(this.cur) || this.pos >= this.cur.length) break;
      const chars = Math.min(this.cur.length - this.pos, count);
      const part = this.cur.subarray(this.pos, this.pos + count);
      node = node ? join(node, part) : part;
      this.index += count;
      this.pos += count;
      this.ascend();
      count -= chars;
    }
    return node;
  }

  bases(): Bases {
    return {type: 'bases', bases: this.consts()};
  }

  consts(): Node {
    // Read (C|F|P|IC)+ into a single rope, unescaping it.
    let bases: number[] = [];
    for (;;) {
      const base = this.next()!;
      if (!(base & 3)) {
        if (this.nextStr() !== 'C') throw new Error(`expected 'C'`);
      }
      let escapeLevel = base >> LVL_SHIFT;
      const addr = base & ADDR_MASK;
      const bottom = (base + 3) & 3;
      if (Math.abs(escapeLevel) < MAX_LVL) escapeLevel--;
      bases.push(escapeLevel << LVL_SHIFT | addr | bottom);
      const next = this.peek();
      if (!next || (next === 'I' && this.peek2() !== 'C')) break;
    }
    return Uint32Array.from(bases);
  }

  nat(): [Node, number] {
    let node!: Node|undefined;
    // Read until we hit a P, or the end
    //  - preserve arrays...
    this.descend();
    let bits = [];
    let done = false;
    while (isLeaf(this.cur) && this.pos < this.cur.length) {
      let start = this.pos;
      while (this.pos < this.cur.length && (this.cur[this.pos] & 3) !== 3) {
        bits.push(this.cur[this.pos++] & 1);
        this.index++;
      }
      if ((this.cur[this.pos] & 3) === 3) {
        this.pos++;
        this.index++;
        done = true;
      }
      const arr = this.cur.subarray(start, this.pos);
      node = node ? join(node, arr) : arr;
      if (done) break;
      this.ascend();
      this.descend();
    }
    let num = 0;
    while (bits.length) {
      num = (num | bits.pop()!) << 1;
    }
    if (!node) throw new Error(`Expected nat`);
    return [node, num];
  }
}

const MAX_LVL = 31;
const LVL_SHIFT = 26;
const ADDR_MASK = 0x03FF_FFFC;
const ESCAPES: number[][] = [[1], [2], [3], [0, 1]];
function getEscape(index: number): number[] {
  while (index < ESCAPES.length) {
    ESCAPES.push(ESCAPES[ESCAPES.length - 1].flatMap(n => ESCAPES[n]));
  }
  return ESCAPES[index];
}

function esc(node: Node, lvl: number): Node {
  if (!isLeaf(node)) return join(esc(node.left, lvl), esc(node.right, lvl));
  const nums: number[] = [];
  for (const n of node) {
    let thisLvl = n >> LVL_SHIFT;
    if (thisLvl > -MAX_LVL) thisLvl = Math.min(lvl, MAX_LVL);
    const bits = thisLvl << LVL_SHIFT | n & ADDR_MASK;
    for (const i of getEscape(lvl + (n & 3))) {
      nums.push(bits | i);
    }
  }
  return Uint32Array.from(nums);
}

function indexStr(node: Node, index: number): string {
  while (!isLeaf(node)) {
    const leftLen = node.left.length;
    node = (index < leftLen) ? node.left : (index -= leftLen, node.right);
  }
  const num = node[index];
  return num != null ? BASES[num & 3] : '';
}

interface Bases {
  type: 'bases';
  bases: Node;
}
interface Group {
  type: 'group';
  op: Node;
  open: boolean;
}
interface Skip {
  type: 'skip';
  op: Node;
  count: number;
}
interface Search {
  type: 'search';
  op: Node;
  query: Node;
}
interface Emit {
  type: 'emit';
  op: Node;
  rna: Node;
}
type PItem = Bases|Group|Skip|Search|Emit;

function isLeaf(node: Node): node is Str {
  return node instanceof Uint32Array;
}

function depth(node: Node): number {
  return (node as App).depth || 0;
}

function join(left: Node, right: Node): App {
  // Move nodes around until we're balanced...
  if (!left.length) return right as App;
  if (!right.length) return left as App;
  let dl = depth(left);
  let dr = depth(right);
  while (Math.abs(dl - dr) > 1) {
    if (dl > dr) {
      // left is taller
      const l = left as App;
      if (depth(l.right) > depth(l.left)) {
        // middle is tallest: split it up
        const lr = l.right as App;
        dl = (left = join(l.left, lr.left)).depth || 0;
        dr = (right = join(lr.right, right)).depth || 0;
      } else {
        // rotate middle to right
        dl = depth(left = l.left);
        dr = (right = join(l.right, right)).depth || 0;
      }
    } else {
      // right is taller
      const r = right as App;
      if (depth(r.left) > depth(r.right)) {
        // middle is tallest: split it up
        const rl = r.left as App;
        dl = (left = join(left, rl.left)).depth || 0;
        dr = (right = join(rl.right, r.right)).depth || 0;
      } else {
        // rotate middle to left
        dl = (left = join(left, r.left)).depth || 0;
        dr = depth(right = r.right);
      }
    }
  }
  // balanced, just join
  return {
    left, right,
    depth: Math.max(dl, dr) + 2,
    length: left.length + right.length,
  };
}
