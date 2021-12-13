import {AbstractRnaCanvas, W, H, SIZE} from './rna.js';
import output from 'image-output';

let i = 1;

export class PngRnaCanvas extends AbstractRnaCanvas {
  snapshots = false;
  snapshotOverlays = false;
  snapshotDir = 'snapshot';
  override snapshot(filename?: string) {
    if (!filename) {
      if (!this.snapshots) return;
      if (this.bitmaps.length > 1 && !this.snapshotOverlays) return;
      filename = `${this.snapshotDir}/${String(i++).padStart(5, '0')}${
          this.bitmaps.length > 1 ? `_${this.bitmaps.length - 1}` : ''}.png`;
    }
    const data = new Uint8Array(W * H * 4);
    const view = new DataView(data.buffer);
    const bitmap = this.bitmaps[this.bitmaps.length - 1].data;
    for (let i = 0; i < SIZE; i++) {
      view.setUint32(i << 2, bitmap[i], false);
    }
    output({data, width: W, height: H}, filename);
    console.error(`Wrote ${filename}`);
  }
}
