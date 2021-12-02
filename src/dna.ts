// Dna is a specialized rope whose elements must be one of 0..3, corresponding
// to the four bases, ICFP.  These are stored in the low 2 bits of an int32,
// with the next 24 bits storing the "source map" of where the base originally
// came from, and the upper 6 bits storing a (signed) escape level between -30
// and +30.  ±31 is used as "infinity", and -32 is a NaN, i.e. it didn't
// come directly from a base in the source.

export class Dna {
  readonly length: number;
  constructor(readonly node: Node) {
    this.length = node.length;
  }

  cursor(index: number = 0): ICursor {
    return new Cursor([], this.node, index, index, this.node.length);
  }

  toString(): string {
    let str = '';
    const cursor = this.cursor();
    let c: string|undefined;
    while ((c = cursor.nextStr())) {
      str += c;
    }
    return str;
  }

  * [Symbol.iterator](): Iterator<number> {
    const stack: Node[] = [this.node];
    let top: Node|undefined;
    while ((top = stack.pop())) {
      if (isLeaf(top)) {
        yield* top;
      } else {
        stack.push(top.right, top.left);
      }
    }
  }

  iterate(emit: Emit[]): Dna|undefined {
    const c = this.cursor();
    const pat = c.pattern(emit);
    if (!pat) return undefined;
    const tpl = c.template(emit);
    if (!tpl) return undefined;
    return new Dna(c.matchReplace(pat, tpl));
  }

  execute(stats: {iters?: number} = {}): Emit[] {
    const emit: Emit[] = [];
    let dna: Dna|undefined = this;
    while (dna) {
      stats.iters = (stats.iters || 0) + 1;
      dna = dna.iterate(emit);
    }
    return emit;
  }

  static join(...nodes: Node[]): Dna {
    let node: Node|undefined = undefined;
    for (const n of nodes) {
//console.log('node', node, 'n', n);
      node = node ? join(node, n) : n;
    }
    if (!node) throw new Error('empty nodes');
    return new Dna(node);
  }

  static of(str: string): Dna {
    return new Dna(Int32Array.from(str, (c, i) => INV_BASES.get(c)! | i << 2));
  }
}

// Maintain an AVL tree
export type Node = App | Str;
type Str = Int32Array;
interface App {
  readonly left: Node;
  readonly right: Node;
  readonly length: number;
  readonly depth: number;
}

const BASES = 'ICFP';
const INV_BASES = new Map([...BASES].map((v, i) => [v, i] as const));

interface ICursor {
  readonly index: number;
  readonly length: number;
  // clone(): ICursor;
  seek(index: number): void;
  skip(delta: number): void;
  find(needle: Node): boolean;
  next(): number|undefined;
  nextStr(): string;
  prev(): number|undefined;
  prevStr(): string;
  suffix(): Node;
  peek(): string;
  peek2(): string;
  patternItem(): PItem|Control;
  pattern(emits: Emit[]): PItem[]|undefined;
  templateItem(): TItem|Control;
  template(emits: Emit[]): TItem[]|undefined;
  matchReplace(pat: PItem[], tpl: TItem[]): Node;
  atEnd(): boolean;
  slice(count: number): Node|undefined;  
}

class Cursor implements ICursor {
  // Index within the entire string.
  index: number;
  // Stack of parents
  stack: Array<readonly [node: App, branch: number]>;
  // Current node
  cur: Node
  // Index within the current leaf.
  pos: number;
  // Total length
  length: number;

  constructor(stack: Array<readonly [App, number]>, cur: Node,
              index: number, pos: number, length: number) {
    this.stack = stack;
    this.cur = cur;
    this.index = index;
    this.pos = pos;
    this.length = length;
  }

  // clone(): Cursor {
  //   return new Cursor([...this.stack], this.cur,
  //                     this.index, this.pos, this.length);
  // }

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

  skip(delta: number) {
    this.pos += delta;
    this.index += delta;
    this.ascend();
  }

  // return 0..3
  charAt(i: number): number {
    this.seek(i);
    this.descend();
    const val = isLeaf(this.cur) ? this.cur[this.pos] : undefined;
    return val != null ? val & 3 : -1;
  }
  find(needle: Node): boolean {
    const startIndex = this.index; // to get back if we don't find it
    const needleNums = Array.from(new Dna(needle), x => x & 3);
    const needleLen = needleNums.length;
    const charTable = boyerMooreCharTable(needleNums);
    const offsetTable = boyerMooreOffsetTable(needleNums);
    for (let i = this.index + needleLen - 1, j; i < this.length;) {
      let c!: number;
      for (j = needleLen - 1;
           needleNums[j] === (c = this.charAt(i));
           --i, --j) {
        if (j === 0) {
          this.seek(i + needleLen); // seek past end of needle
          return true;
        }
      }
      i += Math.max(offsetTable[needleLen - 1 - j], charTable[c]);
    }
    this.seek(startIndex);
    return false;
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

  patternItem(): PItem|Control {
    const first = this.peek();
    if (first === 'I') {
      const second = this.peek2();
      if (second === 'I') {
        // need the third (note: this may push past end)
        const op = this.slice(3)!;
        switch (indexStr(op, 2)) {
          case 'I': return {type: 'emit', op, rna: this.slice(7)!};
          case 'C': case 'F': return {type: 'close', op};
          case 'P': return {type: 'open', op};
          default: return FINISH;
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
        const count = this.nat();
        if (!count) return FINISH;
        return {type: 'skip', op, count};
      } else {
        this.next(); // push us off the end
        return FINISH;
      }
    } else if (first) {
      // start a run of escaped chars
      return this.bases();
    }
    return FINISH;
  }

  pattern(emits: Emit[]): PItem[]|undefined {
    let lvl = 0;
    const pattern: PItem[] = [];
    for (;;) {
      const item = this.patternItem();
      switch (item.type) {
        case 'finish':
          return undefined;
        case 'open':
          lvl++;
          break;
        case 'close':
          lvl--;
          if (lvl < 0) return pattern;
          break;
        case 'emit':
          emits.push(item);
          break;
        case 'done':
          throw new Error('impossible done pattern');
        default:
          pattern.push(item);
      }
    }
  }

  templateItem(): TItem|Control {
    const first = this.peek();
    if (first === 'I') {
      const second = this.peek2();
      if (second === 'I') {
        // need the third (note: this may push past end)
        const op = this.slice(3)!;
        switch (indexStr(op, 2)) {
          case 'I': return {type: 'emit', op, rna: this.slice(7)!};
          case 'C': case 'F': return {type: 'done'};
          case 'P': 
            const group = this.nat();
            if (!group) return FINISH;
            return {type: 'len', op, group};
          default: return FINISH;
        }
      } else if (second === 'C') {
        // start a run of escaped chars
        return this.bases();
      } else if (second === 'F' || second === 'P') {
        // two-char op: IF (group reference)
        const op = this.slice(2)!;
        const level = this.nat();
        if (!level) return FINISH;
        const group = this.nat();
        if (!group) return FINISH;
        return {type: 'ref', op, level, group};
      } else {
        this.next(); // 2nd is nothing, only 1 left: push us off the end
      }
    } else if (first) {
      // start a run of escaped chars
      return this.bases();
    }
    return FINISH;
  }

  template(emits: Emit[]): TItem[]|undefined {
    const template: TItem[] = [];
    for (;;) {
      const item = this.templateItem();
      switch (item.type) {
        case 'finish':
          return undefined;
        case 'done':
          return template;
        case 'emit':
          emits.push(item);
          break;
        default:
          template.push(item);
      }
    }
  }

  matchBases(bases: Node): boolean {
    const stack: Node[] = [bases];
    let top: Node|undefined;
    while ((top = stack.pop())) {
      if (isLeaf(top)) {
        for (const n of top) {
          if (!this.atEnd() && (n & 3) !== (this.next()! & 3)) return false;
        }
      } else {
        stack.push(top.right, top.left);
      }
    }
    return true;
  }

  matchReplace(pat: PItem[], t: TItem[]): Node {
    const startIndex = this.index; // save in case not found
    const env: Node[] = [];
    const c: number[] = [];
    // Match the pattern
    for (const p of pat) {
      switch (p.type) {
        case 'bases':
          if (!this.matchBases(p.bases)) {
            this.seek(startIndex);
            return this.suffix();
          }
          break;
        case 'open':
          c.push(this.index);
          break;
        case 'close':
          if (!c.length) throw 'impossible ) without (';
          const len = this.index - c.pop()!;
          this.skip(-len);
          env.push(this.slice(len)!);
          break;
        case 'skip':
          this.skip(p.count.val);
          if (this.index > this.length) {
            this.seek(startIndex);
            return this.suffix();
          }
          break;
        case 'search':
          if (!this.find(p.query)) {
            this.seek(startIndex);
            return this.suffix();
          }
          break;
        default:
          throw new Error(`impossible pattern: ${p!.type}`);
      }
    }
    // Replace the template
    const repl = replace(t, env);
    return repl ? join(repl, this.suffix()) : this.suffix();
  }

  atEnd(): boolean {
    return this.index >= this.length;
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
    return Int32Array.from(bases);
  }

  nat(): Num|undefined {
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
    let val = 0;
    while (bits.length) {
      val = val << 1 | bits.pop()!;
    }
    if (!node) return undefined;
    return {node, val};
  }
}

function replace(tpl: TItem[], env: Node[]): Node|undefined {
  let node!: Node|undefined;
  function push(n: Node) {
    node = node ? join(node, n) : n;
  }
  for (const t of tpl) {
    switch (t.type) {
      case 'bases':
        push(t.bases);
        break;
      case 'len':
        push(asNat(env[t.group.val]?.length || 0, addr(t.op)));
        break;
      case 'ref':
        const group = env[t.group.val];
        if (group) push(protect(group, t.level.val));
        break;
      default:
        throw new Error(`impossible template ${t!.type}`);
    }
  }
  return node;
}

function addr(node: Node): number {
  while (!isLeaf(node)) node = node.left;
  return (node[0] & ADDR_MASK) >>> 2;
}

function asNat(n: number, addr: number): Node {
  const mask = addr << 2 | NAN_LVL << LVL_SHIFT;
  const nums: number[] = [];
  while (n) {
    nums.push(((n & 1) ? 1 : 0) | mask);
    n >>>= 1;
  }
  nums.push(3 | mask);
  return Int32Array.from(nums);
}

const NAN_LVL = -32;
const MAX_LVL = 31;
const LVL_SHIFT = 26;
const ADDR_MASK = 0x03FF_FFFC;
const ESCAPES: number[][] = [[0], [1], [2], [3], [0, 1]];
function getEscape(index: number): number[] {
  while (index >= ESCAPES.length) {
    ESCAPES.push(ESCAPES[ESCAPES.length - 1].flatMap(n => ESCAPES[n + 1]));
  }
  return ESCAPES[index];
}

function protect(node: Node, lvl: number): Node {
  if (!lvl) return node;
  if (!isLeaf(node)) {
    return join(protect(node.left, lvl), protect(node.right, lvl));
  }
  const nums: number[] = [];
  for (const n of node) {
    let thisLvl = n >> LVL_SHIFT;
    if (thisLvl > -MAX_LVL) thisLvl = Math.min(lvl, MAX_LVL);
    const bits = thisLvl << LVL_SHIFT | n & ADDR_MASK;
    for (const i of getEscape(lvl + (n & 3))) {
      nums.push(bits | i);
    }
  }
  return Int32Array.from(nums);
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
  type: 'open'|'close';
  op: Node;
}
interface Skip {
  type: 'skip';
  op: Node;
  count: Num;
}
interface Search {
  type: 'search';
  op: Node;
  query: Node;
}
interface Len {
  type: 'len';
  op: Node;
  group: Num;
}
interface Ref {
  type: 'ref';
  op: Node;
  group: Num;
  level: Num;
}
export interface Emit {
  type: 'emit';
  op: Node;
  rna: Node;
}
type PItem = Bases|Group|Skip|Search;
type TItem = Bases|Len|Ref;
type Control = Emit|{type: 'done'|'finish'};
interface Num {
  node: Node;
  val: number;
}

const FINISH = {type: 'finish'} as Control;

function isLeaf(node: Node): node is Str {
  return node instanceof Int32Array;
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
    depth: Math.max(dl, dr) + 1,
    length: left.length + right.length,
  };
}

// needle is an array of [0..3], return is a 4-element array
function boyerMooreCharTable(needle: number[]): number[] {
  const len = needle.length;
  const table = [len, len, len, len];
  for (let i = 0; i < needle.length - 1; i++) {
    table[needle[i]] = len - 1 - i;
  }
  return table;
}
function boyerMooreOffsetTable(needle: number[]): number[] {
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

export {ICursor as Cursor};
