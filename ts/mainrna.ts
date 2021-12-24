import {PngRnaCanvas} from './pngrna.js';
import {promises as fs} from 'fs';
import yargs from 'yargs';

require('source-map-support').install();

async function run() {
  const argv = await yargs(process.argv.slice(2))
      .option('snapshot', {
        alias: 's',
        description: `Whether to output snapshots.`,
        type: 'boolean',
        default: false,
      })
      .option('out', {
        alias: 'o',
        description: `Output file.`,
        default: 'rna.png',
      })
      .help()
      .argv;

  if (argv._.length > 1) {
    throw new Error(`Expected at most 1 RNA file`);
  }

  const file = argv._[0] || '/dev/stdin';
  const canvas = new PngRnaCanvas();
  if (argv.snapshot) canvas.lineSnapshot = 100; // configurable?
  if (argv.snapshot) canvas.snapshots = canvas.snapshotOverlays = true;
  const rnas = String(await fs.readFile(String(file))).split(/\n+/g);
  let i = 0;
  for (let rna of rnas) {
    rna = rna.replace(/\s*#.*$/, ''); // remove comments
    i++;
    canvas.process(rna);
  }
  canvas.finalize();
  canvas.snapshot({filename: argv.out});
  console.error(`Processed ${i} RNA`);
}

run().then();
