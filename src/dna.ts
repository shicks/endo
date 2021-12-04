// Dna is a specialized rope whose elements must be one of 0..3, corresponding
// to the four bases, ICFP.  These are stored in the low 2 bits of an int32,
// with the next 24 bits storing the "source map" of where the base originally
// came from, and the upper 6 bits storing a (signed) escape level between -30
// and +30.  ±31 is used as "infinity", and -32 is a NaN, i.e. it didn't
// come directly from a base in the source.

// With optional transient finger...?
export interface Rope<T> extends Iterable<T> {
  readonly length: number;
  at(index: number): T;
  find(needle: Iterable<T>, start: number): number;
  slice(start: number, end?: number): Rope<T>;
  splice(start: number, len: number, insert?: Iterable<T>): Rope<T>;
  toString(): string;
}

interface DnaResult<T> {
  rna: Emit<T>[];
  dna?: Rope<T>;
  finish: boolean;
}

interface Bases<T> {
  type: 'bases';
  bases: Rope<T>;
}
interface Group<T> {
  type: 'open'|'close';
  op?: Rope<T>;
}
interface Skip<T> {
  type: 'skip';
  op?: Rope<T>;
  count: Num<T>;
}
interface Search<T> {
  type: 'search';
  op?: Rope<T>;
  query: Rope<T>;
}
interface Len<T> {
  type: 'len';
  op?: Rope<T>;
  group: Num<T>;
}
interface Ref<T> {
  type: 'ref';
  op?: Rope<T>;
  group: Num<T>;
  level: Num<T>;
}
export interface Emit<T> {
  type: 'emit';
  dna?: Rope<T>;
  rna: string;
}
type PItem<T> = Bases<T>|Group<T>|Skip<T>|Search<T>;
type TItem<T> = Bases<T>|Len<T>|Ref<T>;
interface Num<T> {
  dna?: Rope<T>;
  val: number;
}
interface Pattern<T> {
  pat: PItem<T>[];
  index: number;
}
interface Template<T> {
  tpl: TItem<T>[];
  index: number;
}

const ESCAPES: number[][] = [[0], [1], [2], [3], [0, 1]];
abstract class AbstractDna<T> {
  saveOp = false;

  abstract base(t: T): number;
  abstract fromBase(n: number): T;
  abstract isRope(arg: unknown): arg is Rope<T>;
  abstract rope(iterable: Iterable<T>): Rope<T>; // should short-circuit
  abstract init(str: string): Rope<T>;

  slice(dna: Rope<T>, start: number, end?: number): Rope<T>|undefined {
    if (!this.saveOp) return undefined;
    return dna.slice(start, end);
  }

  escape(ts: Iterable<T>, level: number): Iterable<T> {
    if (level === 0) return this.rope(ts);
    while (ESCAPES.length < level + 4) {
      ESCAPES.push(ESCAPES[ESCAPES.length - 1].flatMap(n => ESCAPES[n + 1]));
    }
    const bases: T[] = [];
    for (const t of ts) {
      bases.push(...ESCAPES[this.base(t) + level].map(b => this.fromBase(b)));
    }
    return bases;
  }

  unescape(ts: Iterable<T>): Iterable<T> {
    const bases: T[] = [];
    let skip = false;
    for (const t of ts) {
      if (skip) {
        skip = false;
      } else if (this.base(t) === 0) {
        skip = true;
        bases.push(this.fromBase(3));
      } else {
        bases.push(this.fromBase(this.base(t) - 1));
      }
    }
    return bases;
  }

  iterate(dna: Rope<T>): DnaResult<T> {
    const rna: Emit<T>[] = [];
    const pat = this.pattern(dna, rna, 0);
    if (pat.index < 0) return {rna, finish: true};
//    console.log(`Pattern:  ${showList(pat.map(showItem))}`);
    const tpl = this.template(dna, rna, pat.index);
    if (tpl.index < 0) return {rna, finish: true};
//    console.log(`Template: ${showList(tpl.map(showItem))}`);
//    console.log(`Position: ${c.index} / ${c.length}`);
    return {
      rna, finish: false,
      dna: this.matchReplace(dna, pat.pat, tpl.tpl, tpl.index),
    };
  }

  pattern(dna: Rope<T>, rna: Emit<T>[], index: number): Pattern<T> {
    let lvl = 0;
    const pat: PItem<T>[] = [];
    for (;;) {
      if (index === dna.length) return {pat, index: -1};
      switch (this.base(dna.at(index++))) {
        case 0: // I
          if (index === dna.length) return {pat, index: -1};
          switch (this.base(dna.at(index++))) {
            case 0: // II
              if (index === dna.length) return {pat, index: -1};
              switch (this.base(dna.at(index++))) {
                case 0: // III -> emit
                  rna.push({
                    type: 'emit',
                    dna: this.slice(dna, index - 3, index + 7),
                    rna: dna.slice(index, index + 7).toString(),
                  });
                  index += 7;
                  break;
                case 1: case 2: // IIC, IIF -> close
                  if (lvl-- > 0) {
                    pat.push({
                      type: 'close',
                      op: this.slice(dna, index - 3, index),
                    });
                  } else {
                    return {pat, index};
                  }
                  break;
                case 3: // IIP -> open
                  lvl++;
                  pat.push({
                    type: 'open',
                    op: this.slice(dna, index - 3, index),
                  });
                  break;
              }
              break;
            case 1: { // IC -> base
              const [newIndex, bases] = this.readBases(dna, index - 2);
              index = newIndex;
              pat.push({type: 'bases', bases});
              break;
            }
            case 2: { // IF -> search
              const [newIndex, query] = this.readBases(dna, index + 1);
              pat.push({
                type: 'search',
                op: this.slice(dna, index - 2, index + 1),
                query,
              });
              index = newIndex;
              break;
            }
            case 3: { // IP -> skip
              const [newIndex, count] = this.nat(dna, index);
              if (newIndex < 0) return {pat, index: -1}
              pat.push({
                type: 'skip',
                op: this.slice(dna, index - 2, index),
                count,
              });
              index = newIndex;
              break;
            }
          }
          break;
        default: { // C, F, P -> base
          const [newIndex, bases] = this.readBases(dna, index - 1);
          index = newIndex;
          pat.push({type: 'bases', bases});
          break;
        }
      }
    }
  }

  template(dna: Rope<T>, rna: Emit<T>[], index: number): Template<T> {
    const tpl: TItem<T>[] = [];
    for (;;) {
      if (index === dna.length) return {tpl, index: -1};
      switch (this.base(dna.at(index++))) {
        case 0: // I
          if (index === dna.length) return {tpl, index: -1};
          switch (this.base(dna.at(index++))) {
            case 0: // II
              if (index === dna.length) return {tpl, index: -1};
              switch (this.base(dna.at(index++))) {
                case 0: // III -> emit
                  rna.push({
                    type: 'emit',
                    dna: this.slice(dna, index - 3, index + 7),
                    rna: dna.slice(index, index + 7).toString(),
                  });
                  index += 7;
                  break;
                case 1: case 2: // IIC, IIF -> done
                  return {tpl, index};
                case 3: { // IIP -> len
                  const [newIndex, group] = this.nat(dna, index);
                  if (newIndex < 0) return {tpl, index: -1}
                  tpl.push({
                    type: 'len',
                    op: this.slice(dna, index - 3, index),
                    group,
                  });
                  index = newIndex;
                  break;
                }
              }
              break;
            case 1: { // IC -> bases
              const [newIndex, bases] = this.readBases(dna, index - 2);
              index = newIndex;
              tpl.push({type: 'bases', bases});
              break;
            }
            case 2: case 3: { // IF, IP -> ref
              const [i1, level] = this.nat(dna, index);
              if (i1 < 0) return {tpl, index: -1}
              const [i2, group] = this.nat(dna, i1);
              if (i2 < 0) return {tpl, index: -1}
              tpl.push({
                type: 'ref',
                op: this.slice(dna, index - 2, index),
                level, group,
              });
              break;
            }
          }
          break;
        default: { // C, F, P -> base
          const [newIndex, bases] = this.readBases(dna, index - 1);
          index = newIndex;
          tpl.push({type: 'bases', bases});
          break;
        }
      }
    }
  }

  readBases(dna: Rope<T>, index: number): [number, Rope<T>] {
    const start = index;
    for (;;) {
      if (index >= dna.length) {
        return [index, this.rope(this.unescape(dna.slice(start, index)))];
      }
      if (this.base(dna.at(index)) !== 0) {
        index++;
      } else {
        if (index + 1 >= dna.length || this.base(dna.at(index + 1)) !== 1) {
          return [index, this.rope(this.unescape(dna.slice(start, index)))];
        }
        index += 2;
      }
    }
  }

  nat(dna: Rope<T>, index: number): [number, Num<T>] {
    const start = 0;
    const bits = [];
    let base!: number;
    while ((base = this.base(dna.at(index++))) !== 3) {
      if (index >= dna.length) return [-1, undefined!];
      bits.push(base & 1);
    }
    let val = 0;
    while (bits.length) {
      val = val << 1 | bits.pop()!;
    }
    return [index, {dna: dna.slice(start, index), val}];
  }

  matchReplace(dna: Rope<T>, pat: PItem<T>[],
               t: TItem<T>[], index: number): Rope<T> {
    const start = index;
    const env: [number, number][] = [];
    const c: number[] = [];
    // Match the pattern
    for (const p of pat) {
      switch (p.type) {
        case 'bases':
          for (let b of p.bases) {
            if (this.base(b) !== this.base(dna.at(index++))) {
              return dna.slice(start);
            }
          }
          break;
        case 'open': c.push(index); break;
        case 'close': env.push([c.pop()!, index]); break;
        case 'skip':
          index += p.count.val;
          if (index > dna.length) return dna.slice(start);
          break;
        case 'search': {
          const newIndex = dna.find(p.query, index);
          if (newIndex < 0) return dna.slice(start);
          index = newIndex + p.query.length;
          break;
        }
      }
    }
    return this.replace(dna, index, t, env);
  }

  replace(d: Rope<T>, index: number,
          tpl: TItem<T>[], env: [number, number][]): Rope<T> {

    let keep = -1;
    let keepIndex = -1;
    for (const t of tpl) {
      if (t.type !== 'ref' || t.level.val) continue;
      const g = t.group.val;
      if (keep < 0 ||
          (g < env.length &&
           env[g][1] - env[g][0] > env[keep][1] - env[keep][0])) {
        keep = g;
        keepIndex = -1;
      }
    }
    let dropPrefix = index;
    let dropInfix = index;
    let addPrefix = tpl;
    let addInfix: TItem<T>[] = [];
    if (keep >= 0 && keep < env.length) {
      [dropPrefix, dropInfix] = env[keep];
      addPrefix = tpl.slice(0, keepIndex - 1);
      addInfix = tpl.slice(keepIndex + 1);
    }
    const prefix = this.expand(addPrefix, d, env);
    const infix = this.expand(addInfix, d, env);
    return d.splice(dropInfix, index - dropInfix, infix)
        .splice(0, dropPrefix, prefix);
  }

  expand(tpl: TItem<T>[], dna: Rope<T>, env: [number, number][]): T[] {
    let out: T[] = [];
    for (const t of tpl) {
      switch (t.type) {
        case 'bases': 
          out = out.concat([...t.bases]);
          break;
        case 'len':
          out = out.concat([...this.asNat(t, env)]);
          break;
        case 'ref':
          if (t.group.val < env.length) {
            out = out.concat([...this.escape(
                dna.slice(...env[t.group.val]), t.level.val)]);
          }
          break;
      }
    }
    return out;
  }

  asNat(t: Len<T>, env: [number, number][]): T[] {
    if (t.group.val >= env.length) return [this.fromBase(3)];
    let n = env[t.group.val][1] - env[t.group.val][0];
    const ts: T[] = [];
    while (n) {
      ts.push(this.fromBase((n & 1) ? 1 : 0));
      n >>>= 1;
    }
    ts.push(this.fromBase(3));
    return ts;
  }
}


export class StringDna extends AbstractDna<string> {
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
  constructor(private str: string) {
    this.length = str.length;
  }
  [Symbol.iterator]() {
    return this.str[Symbol.iterator]();
  }
  at(index: number) { return this.str[index]; }
  find(needle: Iterable<string>, start: number) {
    const needleStr =
        typeof needle === 'string' ? needle : [...needle].join('');
    return this.str.indexOf(needleStr, start);
  }
  slice(start: number, end: number): Rope<string> {
    return new StringRope(this.str.substring(start, end));
  }
  splice(start: number, length: number,
         insert?: Iterable<string>): Rope<string> {
    const insertStr =
        !insert ? '' :
        typeof insert === 'string' ? insert :
        [...insert].join('');
    return new StringRope(this.str.substring(0, start) +
        insertStr + this.str.substring(start + length));
  }
  toString() {
    return this.str;
  }
}
