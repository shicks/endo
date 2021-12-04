import {Rope, StringDna} from './dna.js';
import {endo} from './endo.js';

export {StringDna, endo}

// TODO - only works b/c transpile down to CJS
if (typeof require !== 'undefined' && typeof module !== 'undefined' &&
    require.main === module) {
  require('source-map-support').install();
  Error.stackTraceLimit = 30;
  const prefix = process.argv[2] || '';
  const DNA = new StringDna();
  let dna:Rope<string>|undefined = DNA.init(prefix + endo);
  const rna: string[] = [];
  let iters = 0;
  while (dna) {
    iters++;
    const result = DNA.iterate(dna);
    dna = result.dna;
    rna.push(...result.rna.map(e => e.rna));
    //
    for (const r of rna) { console.log(r); }
    rna.splice(0, rna.length);
  }  
  console.log(`Execution complete after ${iters} iterations: ${rna.length} RNA commands`);
  // for (const r of rna) {
  //   console.log(new Dna(r.rna).toString());
  // }
}
