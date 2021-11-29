import {expect} from './chai.js';
import {DnaProcessor} from '../src/dna.js';
import {Reader} from '../src/reader.js';

describe.only('DnaProcessor', () => {
  describe('pattern()', () => {
    const d = new DnaProcessor('');
    it('should parse a base', () => {
      const r = new Reader('CIIC')
      expect(d.pattern(r)).to.eql(['I']);
      expect(r.slice()).to.equal('');
    });
    it('should parse a skip', () => {
      const r = new Reader('IIPIPICPIICICIIF');
      expect(d.pattern(r)).to.eql(['(', 2, ')', 'P']);
      expect(r.slice()).to.equal('');
    });
  });
});
