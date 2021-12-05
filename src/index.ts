import {Rope, StringDna} from './dna.js';
import {RopeDna} from './ropedna.js';
import {endo} from './endo.js';
import {PngRnaCanvas} from './pngrna.js';

// TODO - only works b/c transpile down to CJS
if (typeof require !== 'undefined' && typeof module !== 'undefined' &&
    require.main === module) {
  const start = Date.now();
  require('source-map-support').install();
  const canvas = new PngRnaCanvas();
  const prefix = process.argv[2] || '';
  const DNA = new RopeDna();
  let dna:Rope<string>|undefined = DNA.init(prefix + endo);
  const rna: string[] = [];
  let iters = 0;
  let rnaCount = 0;
  let rnaCountTotal = 0;
  while (dna) {
    iters++;
    const result = DNA.iterate(dna);
    dna = result.dna;
    rna.push(...result.rna.map(e => e.rna));
    //
    //if (rna.length) console.log(`Got ${rna.length} RNA`);
    rnaCount += rna.length;
    if (iters % 100 === 0) {
      rnaCountTotal += rnaCount;
      console.log(`Iteration ${iters}: processed ${rnaCount} RNA (${rnaCountTotal} total in ${Math.floor((Date.now() - start) / 6000) / 10} minutes)`);
      rnaCount = 0;
    }
    for (const r of rna) {
      canvas.process(r);
    }
    rna.splice(0, rna.length);
  }  
  console.log(`Execution complete after ${iters} iterations: ${rna.length} RNA commands`);
  canvas.snapshot();
  // for (const r of rna) {
  //   console.log(new Dna(r.rna).toString());
  // }
}
