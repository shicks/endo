import {expect} from './chai.js';
import {Rope} from '../src/rope.js';

describe.only('Rope', () => {
  describe('Rope.cat', () => {
    it('should return a string for short inputs', () => {
      const r = Rope.cat('abc', 'def');
      expect(r).to.equal('abcdef');
    });
    it('should return a rope for long inputs', () => {
      const r = Rope.cat('a'.repeat(200), 'b'.repeat(200));
      expect(r).not.to.be.a('string');
      expect(r.toString()).to.equal('a'.repeat(200) + 'b'.repeat(200));
    });
    it('should return the non-empty arg exactly', () => {
      const r = Rope.cat('a'.repeat(200), 'b'.repeat(200));
      expect(Rope.cat(r, '')).to.equal(r);
      expect(Rope.cat('', r)).to.equal(r);
    });
    it('should not modify any arguments', () => {
      const r = Rope.cat('a'.repeat(200), 'b'.repeat(200));
      Rope.cat(r, 'abc');
      expect(r.toString()).to.equal('a'.repeat(200) + 'b'.repeat(200));
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
  let r: Rope = '';
  let s = '';
  for (let i = 0; i < lengths.length; i++) {
    const chars = String.fromCharCode('A'.charCodeAt(0) + i).repeat(lengths[i]);
    r = Rope.cat(r, chars);
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
