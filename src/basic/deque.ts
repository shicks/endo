interface L extends Deque<any> {
  length: number
}

export class Deque<T> {
  private elems: T[] = [];
  private start = 0;
  readonly length: number = 0;

  * [Symbol.iterator]() {
    for (let i = 0; i < this.length; i++) {
      yield this.elems[(this.start + i) % this.elems.length];
    }
  }

  push(elem: T) {
    if (this.length >= this.elems.length) {
      if (this.start !== 0) {
        this.reallocate();
      } else {
        this.elems.length++;
      }
    }
    const index =
        (this.start + ((this as L).length++)) % (this.elems.length || 1);
    this.elems[index] = elem;
  }

  pop(): T|undefined {
    if (!this.length) return undefined;
    const index = (this.start + --(this as L).length) % this.elems.length;
    return this.elems[index];
  }

  last(): T|undefined {
    if (!this.length) return undefined;
    return this.elems[(this.start + this.length - 1) % this.elems.length];
  }

  unshift(elem: T) {
    if (this.length >= this.elems.length) {
      if (this.start !== 0) {
        this.reallocate();
      } else {
        this.elems.length++;
      }
    }
    this.start--;
    if (this.start < 0) this.start += (this.elems.length || 1);
    (this as L).length++;
    this.elems[this.start] = elem;
  }

  shift(): T|undefined {
    if (!this.length) return undefined;
    const elem = this.elems[this.start++];
    (this as L).length--;
    if (this.start >= this.elems.length) this.start -= this.elems.length;
    return elem;
  }

  first(): T|undefined {
    if (!this.length) return undefined;
    return this.elems[this.start];
  }

  at(i: number): T|undefined {
    if (i < 0) {
      i += this.length;
    } else if (i >= this.length) {
      return undefined;
    }
    return this.elems[(this.start + i) % this.elems.length];
  }

  private reallocate() {
    if (this.start + this.length < this.elems.length) {
      this.elems = [...this.elems.slice(this.start, this.start + this.length)];
    } else {
      this.elems = [...this.elems.slice(this.start), ...this.elems.slice(0, (this.start + this.length) % this.elems.length)];
    }
    this.elems.length *= 2;
    this.start = 0;
  }
}
