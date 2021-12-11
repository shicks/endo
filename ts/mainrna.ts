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
  const rnas = String(await fs.readFile(String(file))).split(/\n+/g);
  for (const rna of rnas) {
    canvas.process(rna);
  }
  canvas.snapshot(argv.out);
}

run().then();
