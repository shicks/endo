import {Dna} from './dna.js';
import {endo} from './endo.js';

export {Dna, endo}

// TODO - only works b/c transpile down to CJS
if (require.main === module) {
  const prefix = process.argv[2] || '';
  const dna = Dna.of(prefix + endo);
  const stats = {iters: 0};
  const rna = dna.execute(stats);
  console.log(`Execution complete after ${stats.iters} iterations: ${rna.length} RNA commands`);
  // for (const r of rna) {
  //   console.log(new Dna(r.rna).toString());
  // }
}
