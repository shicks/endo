import {Rope} from './rope.js';

export class Reader {
  private count = 0;
  private iter: Iterator<string>;
  private seen: string = '';
  constructor(private orig: Rope) {
    this.iter = orig[Symbol.iterator]();
  }
  read(): string {
    if (this.count < this.seen.length) {
      return this.seen[this.count++];
    }
    this.count++;
    const {value, done} = this.iter.next();
    if (done) return '';
    this.seen += value;
    return value;
  }
  unread(n = 1) {
    this.count -= n;
  }
  slice(): Rope {
    console.log(`Sliced ${this.seen}`);
    return this.orig.substring(this.count);
  }
}
