import {AbstractDna, Rope, StringDna, setDnaVerbose} from './dna.js';
import {RopeDna, StringRopeDna} from './ropedna.js';
import {endo} from './endo.js';
import yargs from 'yargs';

require('source-map-support').install();


// TODO - separate the RNA from the DNA -> output just a list of RNA
// commands from the DNA processor so that we can debug the RNA part?

// TODO - optional to output snapshots (also more optional when bitmaps>1)
//      - flag for output filename

const validRna = new Set([
  'PIPIIIC', 'PIPIIIP', 'PIPIICC', 'PIPIICF', 'PIPIICP', 'PIPIIFC',
  'PIPIIFF', 'PIPIIPC', 'PIPIIPF', 'PIPIIPP', 'PIIPICP', 'PIIIIIP',
  'PCCCCCP', 'PFFFFFP', 'PCCIFFP', 'PFFICCP', 'PIIPIIP', 'PCCPFFP',
  'PFFPCCP', 'PFFICCF',
]);
  

async function run() {
  const argv = await yargs(process.argv.slice(2))
      .option('strategy', {
        alias: 's',
        description: `DNA implementation to use.`,
        choices: ['string', 'rope', 'stringrope'] as const,
        default: 'rope',
      }).option('warmup', {
        alias: 'w',
        description: `Benchmarking warm-up iterations`,
        type: 'number',
      }).option('benchmark', {
        alias: 'b',
        description: `Don't actually emit RNA, just benchmark.`,
      }).option('verbose', {
        alias: 'v',
        type: 'boolean',
        default: false,
      }).option('invalid', {
        alias: 'i',
        description: 'Emit invalid RNA.  By default, invalid RNA are skipped.',
        type: 'boolean',
        default: false,
      })
      .help()
      .argv;
  if (argv._.length > 1) {
    throw new Error(`Expected a single prefix`);
  }
  const prefix = argv._[0] || '';
  const start = Date.now();
  let benchmarkStart = start;
  if (argv.verbose) setDnaVerbose(true);
  
  let DNA!: AbstractDna<any>;
  switch (argv.strategy) {
  case 'stringrope':
    DNA = new StringRopeDna();
    break;
  case 'rope':
    DNA = new RopeDna();
    break;
  case 'string':
    DNA = new StringDna();
    break;
  default:
    throw new Error(`Bad strategy`);
  }
  let dna: Rope<any>|undefined = DNA.init(prefix + endo);
  let iters = 0;
  let rnaCount = 0;
  const benchmark =
      Number(argv.benchmark === true ? 25000 : argv.benchmark || Infinity);
  const warmup = argv.warmup ?? benchmark;
  const maxIters = warmup + benchmark;
  let warmupRna = 0;
  
  while (dna) {
    if (iters === warmup) {
      warmupRna = rnaCount;
      benchmarkStart = Date.now();
    }
    if (iters >= maxIters) break;
    iters++;
    const result = DNA.iterate(dna);
    dna = result.dna;
    for (const emit of result.rna) {
      if (!argv.invalid && !validRna.has(emit.rna)) continue;
      if (!argv.benchmark) console.log(emit.rna); // TODO - binary format?
    }
    rnaCount += result.rna.length;
    if (argv.verbose && iters % 500 === 0) {
      console.error(`Iteration ${iters}: processed ${result.rna.length} RNA (${
          rnaCount} total in ${
          Math.floor((Date.now() - start) / 6000) / 10} minutes)`);
    }
  }
  if (argv.benchmark) {
    iters -= warmup;
    rnaCount -= warmupRna;
    const time = Date.now() - benchmarkStart;
    const mins = Math.floor(time / 60000);
    const secs = (time - 60000 * mins) / 1000;
    const timeStr = mins ? `${mins}m${secs}s` : `${secs}s`;
    console.log(`Processed ${iters} iterations (${rnaCount} RNA) in ${
                 timeStr} (${(iters / time).toFixed(3)} iters/ms)`);
  }
}

run();
