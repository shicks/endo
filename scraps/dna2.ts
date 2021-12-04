// Dna is a specialized rope whose elements must be one of 0..3, corresponding
// to the four bases, ICFP.  These are stored in the low 2 bits of an int32,
// with the next 24 bits storing the "source map" of where the base originally
// came from, and the upper 6 bits storing a (signed) escape level between -30
// and +30.  ±31 is used as "infinity", and -32 is a NaN, i.e. it didn't
// come directly from a base in the source.

// export class Dna {
//   readonly length: number;
//   constructor(readonly node: Node) {
//     this.length = node.length;
//   }

//   cursor(index: number = 0): Cursor {
//     return new Cursor([], this.node, index, index, this.node.length);
//   }

//   toString(): string {
//     let str = '';
//     const cursor = this.cursor();
//     let c: string|undefined;
//     while ((c = cursor.nextStr())) {
//       str += c;
//     }
//     return str;
//   }

//   * [Symbol.iterator](): Iterator<number> {
//     const stack: Node[] = [this.node];
//     let top: Node|undefined;
//     while ((top = stack.pop())) {
//       if (isLeaf(top)) {
//         yield* top;
//       } else {
//         stack.push(top.right, top.left);
//       }
//     }
//   }

//   iterate(emit: Emit[]): Dna|undefined {
//     const c = this.cursor();
//     const pat = c.pattern(emit);
//     if (!pat) {
// //      console.log(this.toString().substring(0, 1000));
//       return undefined;
//     }
// //    console.log(`Pattern:  ${showList(pat.map(showItem))}`);
//     const tpl = c.template(emit);
//     if (!tpl) return undefined;
// //    console.log(`Template: ${showList(tpl.map(showItem))}`);
// //    console.log(`Position: ${c.index} / ${c.length}`);
//     return new Dna(c.matchReplace(pat, tpl));
//   }

//   execute(stats: {iters?: number} = {}): Emit[] {
//     const emit: Emit[] = [];
//     let dna: Dna|undefined = this;
//     while (dna) {
//       stats.iters = (stats.iters || 0) + 1;
// //      console.log(`\nIteration ${stats.iters}: ${dna.length}`);
//       dna = dna.iterate(emit);
//     }
//     return emit;
//   }

//   static join(...nodes: Node[]): Dna {
//     let node: Node|undefined = undefined;
//     for (const n of nodes) {
// //console.log('node', node, 'n', n);
//       node = node ? join(node, n) : n;
//     }
//     if (!node) throw new Error('empty nodes');
//     return new Dna(node);
//   }

//   static of(str: string): Dna {
//     return new Dna(Int32Array.from(str, (c, i) => INV_BASES.get(c)! | i << 2));
//   }
// }

function showItem(item: PItem|TItem): string {
  switch (item.type) {
    case 'bases': return new Dna(item.bases).toString();
    case 'close': return ')';
    case 'open': return '(';
    case 'skip': return '!' + item.count.val;
    case 'search': return '?' + new Dna(item.query).toString();
    case 'len': return '#' + item.group.val;
    case 'ref': return '<' + item.group.val + '>' + item.level.val;
  }
}
function showList(items: string[], space = ' '): string {
  if (items.length > 100) return `${items.slice(0, 70).join(space)} ... ${items.length - 95} more ...  ${items.slice(items.length - 25).join(space)}`;
  return items.join(space);
}

type Str = Int32Array;
interface App {
  readonly left: Node;
  readonly right: Node;
  readonly length: number;
  readonly depth: number;
}

class Cursor {
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


  replace(tpl: TItem[], env: [number, number][]) {
    let keep = -1;
    let keepIndex = -1;
    let i = -1;
    for (const t of tpl) {
      i++;
      if (t.type !== 'ref' || t.level.val) continue;
      const g = t.group.val;
      if (keep < 0 || env[g][1] - env[g][0] > env[keep][1] - env[keep][0]) {
        keep = g;
        keepIndex = -1;
      }
    }
    let dropPrefix = 0;
    let dropInfix = this.index;
    let addPrefix = tpl;
    let addInfix: TItem[] = [];
    if (keep >= 0) {
      [dropPrefix, dropInfix] = env[keep];
      addPrefix = tpl.slice(0, keepIndex - 1);
      addInfix = tpl.slice(keepIndex + 1);
    }

    // TODO - compute the addPrefix and addInfix replacements
    //      - splice them into the appropriate spots

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

  atEnd(): boolean {
    return this.index >= this.length;
  }

  slice(count: number): Node|undefined {
    let node!: Node|undefined;
    while (count > 0) {
      this.descend();
      if (!isLeaf(this.cur) || this.pos >= this.cur.length) break;
      const chars = Math.min(this.cur.length - this.pos, count);
      const part = this.cur.subarray(this.pos, this.pos + chars);
      node = node ? join(node, part) : part;
      this.index += chars;
      this.pos += chars;
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
    while (isLeaf(this.cur) && this.pos < this.cur.length && !done) {
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
