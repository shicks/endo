import {StringDna, Rope} from './dna.js';
import {endo} from './endo.js';
import {DomRnaCanvas} from './domrna.js';

//let dna!: Dna|undefined;
//(document.getElementById('dna-prefix') as HTMLInputElement).value;

const DNA = new StringDna();

let canvas!: DomRnaCanvas;
let timeoutId!: any;
let running = false;
let prefix = '';
let dna: Rope<string>|undefined = DNA.init(prefix + endo);
let iters = 0;
let rnaCount = 0;

function reset() {
  iters = rnaCount = 0;
  prefix = (document.getElementById('dna-prefix') as HTMLInputElement).value;
  DNA.init(prefix + endo);
  canvas = new DomRnaCanvas(
      document.getElementById('main-canvas') as HTMLDivElement);
  running = false;
  clearTimeout(timeoutId);
  timeoutId = -1;
}
reset();

function listen(id: string, f: () => void) {
  const el = document.getElementById(id) as HTMLInputElement;
  el.addEventListener('click', f);
}
listen('start-button', () => {
  if (running) return;
  running = true;
  step();
});
listen('stop-button', () => {
  running = false;
  clearTimeout(timeoutId);
  timeoutId = -1;
});
listen('reset-button', () => {
  running = false;
  clearTimeout(timeoutId);
  timeoutId = -1;
  reset();
});
listen('back-100', () => {
  canvas.advance(-100);
});
listen('back-10', () => {
  canvas.advance(-10);
});
listen('back-1', () => {
  canvas.advance(-1);
});
listen('plus-1', () => {
  canvas.advance(1);
});
listen('plus-10', () => {
  canvas.advance(10);
});
listen('plus-100', () => {
  canvas.advance(100);
});

function step() {
  const id = timeoutId = setTimeout(() => {
    if (running && timeoutId !== id) return;
    if (!dna) return;
    const start = Date.now();
    while (dna && Date.now() - start < 10) {
      iters++;
      const result = DNA.iterate(dna);
      dna = result.dna;
      if (result.rna.length) {
        rnaCount += result.rna.length;
        for (const emit of result.rna) {
          canvas.process(emit.rna);
        }
      }
    }
    document.getElementById('status-bar')!.textContent =
      `Iteration ${iters}; Processed ${rnaCount} RNA; Snapshot ${
       canvas.cursor} / ${canvas.snapshots.length}`;
    if (dna) {
      step();
    } else {
      running = false;
      timeoutId = -1;
    }
  }, 1);
}

// for (const r of rna) {
//   console.log(new Dna(r.rna).toString());
// }
