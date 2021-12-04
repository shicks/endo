import {expect} from './chai.js';
import {Dna, Emit} from '../src/dna.js';

function i32(base: 'I'|'C'|'F'|'P', source: number, level = 0) {
  return 'ICFP'.indexOf(base) | source << 2 | level << 26;
}
function seq(bases: string, start: number, level = 0): Int32Array {
  const nums: number[] = [];
  for (let i = 0; i < bases.length; i++) {
    nums.push(i32(bases[i] as any, start + i, level));
  }
  return Int32Array.from(nums);
}
function unspace(str: string): string {
  return str.replace(/\s/g, '');
}
function fragment(dna: Dna, sizes: number[]): Dna {
  let nodes = [];
  const c = dna.cursor();
  while (sizes.length && !c.atEnd()) {
    nodes.push(c.slice(sizes.shift()!)!);
  }
  if (!c.atEnd()) {
    nodes.push(c.suffix());
  }
  return Dna.join(...nodes)
}

describe('Dna', () => {
  describe('of()', () => {
    it('should roundtrip with toString', () => {
      expect(Dna.of('CIIC').toString()).to.equal('CIIC');
      expect(Dna.of('ICFP').toString()).to.equal('ICFP');
    });
    it('should enumerate elements', () => {
      const d = Dna.of(unspace('IIPI PICP IICI CIIF'));
      expect([...d]).to.eql([
        i32('I', 0), i32('I', 1), i32('P', 2), i32('I', 3),
        i32('P', 4), i32('I', 5), i32('C', 6), i32('P', 7),
        i32('I', 8), i32('I', 9), i32('C', 10), i32('I', 11),
        i32('C', 12), i32('I', 13), i32('I', 14), i32('F', 15),
      ]);
    });
  });

  describe('cursor', () => {
    describe('seek', () => {
      it('should jump to an absolute position', () => {
        const d = Dna.of(unspace('IIPI PICP IICI CIIF'));
        const c = fragment(d, [3, 2, 4, 6, 1]).cursor();
        expect(c.index).to.equal(0);
        expect(c.next()).to.equal(i32('I', 0));
        expect(c.index).to.equal(1);
        expect(c.next()).to.equal(i32('I', 1));
        expect(c.index).to.equal(2);
        c.seek(0);
        expect(c.index).to.equal(0);
        expect(c.next()).to.equal(i32('I', 0));
        c.seek(4);
        expect(c.index).to.equal(4);
        expect(c.next()).to.equal(i32('P', 4));
        c.seek(8);
        expect(c.index).to.equal(8);
        expect(c.next()).to.equal(i32('I', 8));
        c.seek(12);
        expect(c.index).to.equal(12);
        expect(c.next()).to.equal(i32('C', 12));
        c.seek(2);
        expect(c.index).to.equal(2);
        expect(c.next()).to.equal(i32('P', 2));
      });
    });
    describe('prev', () => {
      it('should reverse next', () => {
        const d = Dna.of(unspace('IIPI PICP IICI CIIF'));
        const c = fragment(d, [2, 1, 3, 6, 3, 1]).cursor();
        c.next();
        expect(c.prev()).to.equal(i32('I', 0));
        expect(c.index).to.equal(0);
        c.seek(10);
        expect(c.prev()).to.equal(i32('I', 9));
        expect(c.index).to.equal(9);
        expect(c.prev()).to.equal(i32('I', 8));
        expect(c.index).to.equal(8);
        expect(c.prev()).to.equal(i32('P', 7));
        expect(c.index).to.equal(7);
      });
    });
   
    describe('find (simple leaf)', () => {
      testFind();
    });
    describe('find (fragmented rope)', () => {
      testFind([2, 1, 4, 2, 3, 2, 1, 1]);
    });
    function testFind(frags?: number[]) {
      it('seeks to end of match and returns true', () => {
        const d = Dna.of(unspace('IIPI PICP IICI CIIF'));
        const c = (frags ? fragment(d, frags) : d).cursor(0);
        expect(c.find(seq('PIP', 4))).to.be.true();
        expect(c.index).to.equal(5);
      });
      it('finds first match', () => {
        const d = Dna.of(unspace('IIPI PICP IICI CIIF'));
        const c = (frags ? fragment(d, frags) : d).cursor(1);
        expect(c.find(seq('II', 4))).to.be.true();
        expect(c.index).to.equal(10);
      });
      it('does not find earlier matches', () => {
        const d = Dna.of(unspace('IIPI PICP IICI CIIF'));
        const c = (frags ? fragment(d, frags) : d).cursor(1);
        expect(c.find(seq('II', 4))).to.be.true();
        expect(c.index).to.equal(10);
      });
      it('stays put and returns false if no match', () => {
        const d = Dna.of(unspace('IIPI PICP IICI CIIF'));
        const c = (frags ? fragment(d, frags) : d).cursor(5);
        expect(c.find(seq('PICP', 0))).to.be.false();
        expect(c.index).to.equal(5);
      });
    }

  });

  describe('pattern()', () => {
    it('should parse a simple base', () => {
      const c = Dna.of('CIIC').cursor();
      const emit: Emit[] = [];
      expect(c.pattern(emit)).to.eql([
        {type: 'bases', bases: seq('I', 0, -1)},
      ]);
      expect(c.index).to.equal(4);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should stop before end', () => {
      const c = Dna.of('CIICIC').cursor();
      const emit: Emit[] = [];
      expect(c.pattern(emit)).to.eql([
        {type: 'bases', bases: seq('I', 0, -1)},
      ]);
      expect(c.index).to.equal(4);
      expect(c.atEnd()).to.be.false();
      expect(emit).to.eql([]);
    });
    it('should parse a simple skip', () => {
      const c = Dna.of('IPICPIIF').cursor();
      const emit: Emit[] = [];
      expect(c.pattern(emit)).to.eql([
        {type: 'skip', op: seq('IP', 0), count: {node: seq('ICP', 2), val: 2}},
      ]);
      expect(c.index).to.equal(8);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should parse a simple search', () => {
      const c = Dna.of('IFCICPICFCPIIC').cursor();
      const emit: Emit[] = [];
      expect(c.pattern(emit)).to.eql([
        {type: 'search', op: seq('IFC', 0),
         query: Int32Array.of(...seq('P', 3, -1),     // NOTE: seq broken by P,
                              ...seq('FP', 5, -1),    // which jumps index by 2
                              ...seq('CIF', 8, -1))},
      ]);
      expect(c.index).to.equal(14);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should parse a more complex query', () => {
      const c = Dna.of('IIPIPICPIICICIIF').cursor();
      const emit: Emit[] = [];
      expect(c.pattern(emit)).to.eql([
        {type: 'open', op: seq('IIP', 0)},
        {type: 'skip', op: seq('IP', 3), count: {node: seq('ICP', 5), val: 2}},
        {type: 'close', op: seq('IIC', 8)},
        {type: 'bases', bases: seq('P', 11, -1)},
      ]);
      expect(c.index).to.equal(16);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should emit RNA', () => {
      const c = Dna.of('CIIIPIIFIIPIIC').cursor();
      const emit: Emit[] = [];
      expect(c.pattern(emit)).to.eql([
        {type: 'bases', bases: seq('I', 0, -1)},
      ]);
      expect(c.index).to.equal(14);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([
        {type: 'emit', op: seq('III', 1), rna: seq('PIIFIIP', 4)},
      ]);
    });
  });

  describe('template()', () => {
    it('should parse a simple base', () => {
      const c = Dna.of('CIIC').cursor();
      const emit: Emit[] = [];
      expect(c.template(emit)).to.eql([
        {type: 'bases', bases: seq('I', 0, -1)},
      ]);
      expect(c.index).to.equal(4);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should stop before end', () => {
      const c = Dna.of('CIICIC').cursor();
      const emit: Emit[] = [];
      expect(c.template(emit)).to.eql([
        {type: 'bases', bases: seq('I', 0, -1)},
      ]);
      expect(c.index).to.equal(4);
      expect(c.atEnd()).to.be.false();
      expect(emit).to.eql([]);
    });
    it('should parse a simple length', () => {
      const c = Dna.of('IIPICPIIF').cursor();
      const emit: Emit[] = [];
      expect(c.template(emit)).to.eql([
        {type: 'len', op: seq('IIP', 0), group: {node: seq('ICP', 3), val: 2}},
      ]);
      expect(c.index).to.equal(9);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should parse a simple group reference', () => {
      const c = Dna.of('IFIFCPICFCPIIC').cursor();
      const emit: Emit[] = [];
      expect(c.template(emit)).to.eql([
        {type: 'ref', op: seq('IF', 0),
         level: {node: seq('IFCP', 2), val: 4},
         group: {node: seq('ICFCP', 6), val: 10}},
      ]);
      expect(c.index).to.equal(14);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should parse a more complex query', () => {
      const c = Dna.of('IIPICPICIIF').cursor();
      const emit: Emit[] = [];
      expect(c.template(emit)).to.eql([
        {type: 'len', op: seq('IIP', 0), group: {node: seq('ICP', 3), val: 2}},
        {type: 'bases', bases: seq('P', 6, -1)},
      ]);
      expect(c.index).to.equal(11);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([]);
    });
    it('should emit RNA', () => {
      const c = Dna.of('CIIIPIIFIIPIIC').cursor();
      const emit: Emit[] = [];
      expect(c.template(emit)).to.eql([
        {type: 'bases', bases: seq('I', 0, -1)},
      ]);
      expect(c.index).to.equal(14);
      expect(c.atEnd()).to.be.true();
      expect(emit).to.eql([
        {type: 'emit', op: seq('III', 1), rna: seq('PIIFIIP', 4)},
      ]);
    });
  });

  describe('iterate', () => {
    it('should handle example #1', () => {
      const d = Dna.of('IIPIPICPIICICIIFICCIFPPIICCFPC');
      const emit: Emit[] = [];
      expect(d.iterate(emit).toString()).to.equal('PICFC');
      expect(emit).to.eql([]);
    });
    it('should handle example #2', () => {
      const d = Dna.of('IIPIPICPIICICIIFICCIFCCCPPIICCFPC');
      const emit: Emit[] = [];
      expect(d.iterate(emit).toString()).to.equal('PIICCFCFFPC');
      expect(emit).to.eql([]);
    });
    it('should handle example #3', () => {
      const d = Dna.of('IIPIPIICPIICIICCIICFCFC');
      const emit: Emit[] = [];
      expect(d.iterate(emit).toString()).to.equal('I');
      expect(emit).to.eql([]);
    });
  });
});
