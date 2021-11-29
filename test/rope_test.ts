import {expect} from './chai.js';
import {Rope} from '../src/rope.js';

describe.only('Rope', () => {
  describe('Rope.of', () => {
    it('should accept a string', () => {
      expect(Rope.of('abcdef').toString()).to.equal('abcdef');
    });
  });

  describe('append', () => {
    it('should accept a raw string', () => {
      const r = Rope.of('abc');
      expect(r.append('def').toString()).to.equal('abcdef');
    });
    it('should accept a rope', () => {
      const left = Rope.of('xy');
      const right = Rope.of('zzy');
      expect(left.append(right).toString()).to.equal('xyzzy');
    });
    it('should return `this` if arg is empty', () => {
      const r = Rope.of('xyzzy');
      expect(r.append('')).to.equal(r);
      expect(r.append(Rope.of(''))).to.equal(r);
    });
    it('should not modify any arguments', () => {
      const left = Rope.of('xyz');
      const right = Rope.of('abc');
      left.append(right);
      expect(left.toString()).to.equal('xyz');
      expect(right.toString()).to.equal('abc');
    });
  });

  it('should pass a load test', () => {
    // loadTest([
    //   60, 20, 5, 0, 12, 100, 86, 1000, 2100, 5, 1, 160,
    //   50, 500, 0, 12, 2, 3600, 0, 180, 14, 300, 118, 0,
    // ]);
    loadTest([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
    loadTest([1, 2, 1, 2, 1, 2, 1, 3, 1, 3, 1, 2, 1, 3, 1, 2, 1]);
    loadTest([
      120, 120, 120, 120, 120, 120, 120, 120, 120, 120,
      120, 120, 120, 120, 120, 120, 120, 120, 120, 120,
      120, 120, 120, 120, 120, 120, 120, 120, 120, 120,
    ]);
  });
});

function loadTest(lengths: number[]) {
  let r = Rope.of('');
  let s = '';
  for (let i = 0; i < lengths.length; i++) {
    const chars = String.fromCharCode('A'.charCodeAt(0) + i).repeat(lengths[i]);
    r = r.append(chars);
    s = s + chars;
    expect(r.toString()).to.equal(s);
    for (let j = 0; j < s.length; j = Math.ceil(j * 1.5) + 1) {
      const sub = s.substring(j, 2 * j);
      expect(r.substring(j, 2 * j).toString()).to.equal(sub);
      if (r.indexOf(sub, j + 1) != s.indexOf(sub, j + 1)) {
        console.log(`r`, r, `j`, j, `sub`, sub, `ind`, r.indexOf(sub));
      }


      expect(r.indexOf(sub)).to.equal(s.indexOf(sub));
      expect(r.indexOf(sub, j + 1)).to.equal(s.indexOf(sub, j + 1));
    }
  }
  //console.log(r.debugString());
}
