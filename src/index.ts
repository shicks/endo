import {DnaProcessor} from './dna.js';
import {endo} from './endo.js';
import {Rope} from './rope.js';

export {DnaProcessor, endo}

// TODO - only works b/c transpile down to CJS
if (require.main === module) {
  const prefix = process.argv[2] || '';
  const p = new DnaProcessor(Rope.cat(prefix, endo));
  p.execute();
  for (const r of p.rna) {
    console.log(r);
  }
}
