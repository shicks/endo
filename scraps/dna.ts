// DNA processor

import {Reader} from './reader.js';
import {Rope} from './rope.js';

type Base = 'I'|'C'|'F'|'P';
type Skip = number;
type Search = {search: Rope};
type Group = '('|')';

type Pattern = readonly PItem[];
type PItem = Base | Skip | Search | Group;

type Template = readonly TItem[];
type TItem = Base | [lvl: number, num: number] | number;

export class DnaProcessor {
  readonly rna: string[] = [];
  done = false;
  step = 0;

  constructor(public dna: Rope) {}

  execute() {
    while (!this.done) {
      this.iterate();
    }
  }

  iterate(): boolean {
    this.step++;
    const reader = new Reader(this.dna);
    const p = this.pattern(reader);
    const t = this.template(reader);
    this.dna = reader.slice();
    this.matchReplace(p, t);
    return this.done;
  }

  pattern(reader = new Reader(this.dna)): Pattern {
    let p: PItem[] = [];
    let lvl = 0;
    while (!this.done) {
      switch (reader.read()) {
        case 'C': // C -> I
          p.push('I');
          break;
        case 'F': // F -> C
          p.push('C');
          break;
        case 'P': // P -> F
          p.push('F');
          break;
        case 'I':
          switch (reader.read()) {
            case 'C': // IC -> P
              p.push('P');
              break;
            case 'P': // IP -> !n
              p.push(this.nat(reader));
              break;
            case 'F': // IF -> ?s
              reader.read(); // throw away one char
              p.push(this.consts(reader));
              break;
            case 'I':
              switch (reader.read()) {
                case 'P': // IIP -> (
                  lvl++;
                  p.push('(');
                  break;
                case 'C': case 'F': // IIC,IIF -> )
                  if (!lvl) return p;
                  lvl--;
                  p.push(')');
                  break;
                case 'I': // III -> emit RNA
                  this.emitRna(reader);
                  break;
                default:
                  this.done = true;
                  return [];
              }
              break;
            default:
              this.done = true;
              return [];
          }
          break;
        default:
          this.done = true;
          return [];
      }
    }
    return [];
  }

  nat(reader = new Reader(this.dna)): number {
    // IIP -> 0
    // ICP -> 2
    // CIP -> 1  (IP = P)
    let bits: number[] = [];
    while (!this.done) {
      switch (reader.read()) {
        case 'P':
          let result = 0;
          while (bits.length) {
            result = (result << 1) | bits.pop()!;
          }
          return result;
        case 'I': case 'F':
          bits.push(0);
          break;
        case 'C':
          bits.push(1);
          break;
        default:
          this.done = true;
      }
    }
    return 0;
  }

  consts(reader = new Reader(this.dna)): Search {
    let s: Rope = '';
    for (;;) {
      switch (reader.read()) {
        case 'C': s += 'I'; break;
        case 'F': s += 'C'; break;
        case 'P': s += 'F'; break;
        case 'I':
          switch (reader.read()) {
            case 'C': s += 'P'; break;
            default:
              reader.unread(2);
              return {search: s};
          }
        default:
          reader.unread();
          return {search: s};
      }
    }
  }

  template(reader = new Reader(this.dna)): Template {
    let t: TItem[] = [];
    while (!this.done) {
      switch (reader.read()) {
        case 'C': // C -> I
          t.push('I');
          break;
        case 'F': // F -> C
          t.push('C');
          break;
        case 'P': // P -> F
          t.push('F');
          break;
        case 'I':
          switch (reader.read()) {
            case 'C': // IC -> P
              t.push('P');
              break;
            case 'F': case 'P': // IF, IP -> n_l
              t.push([this.nat(reader), this.nat(reader)]);
              break;
            case 'I':
              switch (reader.read()) {
                case 'C': case 'F': // IIC,IIF -> done
                  return t;
                case 'P': // IIP -> |n|
                  t.push(this.nat(reader));
                  break;
                case 'I': // III -> emit RNA
                  this.emitRna(reader);
                  break;
                default:
                  this.done = true;
                  return [];
              }
              break;
            default:
              this.done = true;
              return [];
          }
          break;
        default:
          this.done = true;
          return [];
      }
    }
    return [];
  }

  matchReplace(pat: Pattern, t: Template) {
    let i = 0;
    const env: Rope[] = [];
    const c: number[] = [];
    for (const p of pat) {
      switch (p) {
        case 'I': case 'C': case 'F': case 'P':
          if (this.dna.charAt(i) !== p) return;
          i++;
          break;
        case '(':
          c.push(i);
          break;
        case ')':
          if (!c.length) throw 'impossible ) without (';
          env.push(this.dna.substring(c.pop()!, i));
          break;
        default:
          if (typeof p === 'number') { // skip
            i += p;
            if (i > this.dna.length) return;
          } else { // search
            const s = String(p.search);
            const j = this.dna.indexOf(s, i);
            i = j + s.length;
          }
      }
    }
    console.log(`${this.step}. Found pattern: `, pat, i, env.map(e=>abbrev(String(e))));
    this.dna = this.dna.substring(i);
    this.replace(t, env);
  }

  replace(tpl: Template, env: Rope[]) {
    //console.log(`Executing template: `, tpl);
    let r = '';
    for (const t of tpl) {
      if (typeof t === 'string') {
        //console.log(`literal ${t}`);
        r += t;
      } else if (typeof t === 'number') {
        //console.log(`len ${t}`);
        r += asNat(env[t]?.length || 0);
      } else {
        const [lvl, i] = t;
        //console.log(`backref ${i}_${lvl}`);
        // NOTE: probably a bug if undefined?
        r += protect(lvl, env[i] || '');
      }
    }
    console.log(`${this.step}. Executed template: `, tpl, r.length/*, r.toString()*/);
    this.dna = Rope.cat(r, this.dna);
  }

  emitRna(reader = new Reader(this.dna)) {
    let rna = '';
    for (let i = 0; i < 7; i++) {
      rna += reader.read();
    }
    console.log(`\x1b[1;31mRNA: ${rna}\x1b[m`);
    if (rna.startsWith('C')) {console.log((reader as any).seen); process.exit(1);}
    this.rna.push(rna);
  }
}

function abbrev(x: unknown, len=100): string {
  let s = '';
  if (typeof x === 'string' || typeof x === 'number') {
    s = String(x);
  } else if (Array.isArray(x)) {
    s = '[' + x.map(e => abbrev(e, Infinity)).join(', ') + ']';
  } else {
    s = '{' + Object.keys(x as object).map(k => k + ': ' + abbrev((x as any)[k], Infinity)).join(', ') + '}';
  }
  return s.length > len ? s.substring(0, len) + '...(' + (s.length - len) + ' more)' : s;
}

function asNat(n: number): string {
  let s = '';
  while (n) {
    s += (n & 1) ? 'C' : 'I';
    n >>>= 1;
  }
  return s + 'P';
}

const quoteMaps: Array<ReadonlyMap<Base, string>> = [new Map([
  ['I', 'C'],
  ['C', 'F'],
  ['F', 'P'],
  ['P', 'IC'],
])];
function quotes(lvl: number) {
  while (quoteMaps.length < lvl) {
    console.log(`constructing quoteMap ${lvl}`);
    const lastMap = quoteMaps[quoteMaps.length - 1];
    console.log(`  last: ${[...lastMap].map(([k,v])=>k+':'+v).join(',')}`);
    const nextMap =
        new Map([...lastMap].map(([k, v]) => [k, String(protect(1, v))]));
    console.log(`  next: ${[...nextMap].map(([k,v])=>k+':'+v).join(',')}`);
    quoteMaps.push(nextMap);
  }
  return quoteMaps[lvl - 1];
}

// TODO - skip the loop by just mapping each char directly,
// maybe keep a quoteMap?
function protect(l: number, s: Rope): Rope {
  if (!l || !s?.length) return s;
console.log(`protect ${l} ${s.length < 100 ? s : s.length}`);
  const map = quotes(l);
  // TODO - there should be no way to get aything other than ICFP here?
const res= [...s].map(c => map.get(c as Base)).join('');
console.log(` => ${res.length < 120 ? res : res.length}`);return res;
}
