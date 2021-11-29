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

  constructor(public dna: Rope) {}

  execute() {
    while (!this.done) {
      this.iterate();
    }
  }

  iterate(): boolean {
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

  matchReplace(p: Pattern, t: Template) {
    // TODO - write me!
  }

  emitRna(reader = new Reader(this.dna)) {
    let rna = '';
    for (let i = 0; i < 7; i++) {
      rna += reader.read();
    }
    this.rna.push(rna);
  }
}
