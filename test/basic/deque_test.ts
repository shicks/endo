import {expect} from './chai.js';
import {Deque} from '../src/deque.js';

describe('Deque', () => {
  it('should start empty', () => {
    const d = new Deque<number>();
    expect([...d]).to.be.empty();
    expect(d.length).to.equal(0);
    expect(d.at(0)).to.be.undefined();
    expect(d.last()).to.be.undefined();
    expect(d.first()).to.be.undefined();
  });

  it('push should add an element', () => {
    const d = new Deque<number>();
    d.push(42);
    expect([...d]).to.eql([42]);
    expect(d.length).to.equal(1);
    expect(d.at(0)).to.equal(42);
    expect(d.last()).to.equal(42);
    expect(d.first()).to.equal(42);
    expect(d.at(1)).to.be.undefined();
  });

  it('push should add a second element', () => {
    const d = new Deque<number>();
    d.push(42);
    d.push(23);
    expect([...d]).to.eql([42, 23]);
    expect(d.length).to.equal(2);
    expect(d.at(0)).to.equal(42);
    expect(d.at(1)).to.equal(23);
    expect(d.last()).to.equal(23);
    expect(d.first()).to.equal(42);
    expect(d.at(2)).to.be.undefined();
  });

  it('unshift should add an element', () => {
    const d = new Deque<number>();
    d.unshift(12);
    expect([...d]).to.eql([12]);
    expect(d.length).to.equal(1);
    expect(d.at(0)).to.equal(12);
    expect(d.last()).to.equal(12);
    expect(d.first()).to.equal(12);
    expect(d.at(1)).to.be.undefined();
  });

  it('unshift should add a second element', () => {
    const d = new Deque();
    d.unshift(12);
    d.unshift(19);
    expect([...d]).to.eql([19, 12]);
    expect(d.length).to.equal(2);
    expect(d.at(0)).to.equal(19);
    expect(d.at(1)).to.equal(12);
    expect(d.last()).to.equal(12);
    expect(d.first()).to.equal(19);
    expect(d.at(2)).to.be.undefined();
  });

  it('should track many array manipulations', () => {
    // TODO: consider using a simple PRNG to make a handful of different
    // 128-bit numbers, using each pair of bits to represent an operation,
    // and testing each sequence separately on a new pair.
    // (Though these would be biased toward empty deques, generally, since
    // removals would be as likely as additions).
    const d = new Deque<number>();
    const a: number[] = [];
    type Op = 'push' | 'pop' | 'shift' | 'unshift';
    const ops: Op[] = [
      'push', 'unshift', 'push', 'push', 'unshift',
      'pop', 'unshift', 'push', 'shift', 'push', 'push',
      'shift', 'pop', 'push', 'pop', 'push', 'unshift',
      'shift', 'unshift', 'pop', 'unshift', 'unshift',
      'pop', 'unshift', 'push', 'shift', 'push', 'push',
      'unshift', 'pop', 'push', 'shift'];
    for (let i = 0; i < ops.length; i++) {
      if (ops[i] === 'push') {
        d.push(i);
        a.push(i);
      } else if (ops[i] === 'pop') {
        expect(d.pop()).to.equal(a.pop());
      } else if (ops[i] === 'unshift') {
        d.unshift(i);
        a.unshift(i);
      } else if (ops[i] === 'shift') {
        expect(d.shift()).to.equal(a.shift());
      }
      expect(d.length).to.equal(a.length);
      expect([...d]).to.eql(a);
      expect(d.first()).to.equal(a[0]);
      expect(d.last()).to.equal(a[a.length - 1]);
      for (let j = 0; j < d.length; j++) {
        expect(d.at(j)).to.equal(a[j]);
      }
    }
  });
});
