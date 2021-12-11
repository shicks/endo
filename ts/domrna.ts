import {AbstractRnaCanvas, W, H, SIZE} from './rna.js';

export class DomRnaCanvas extends AbstractRnaCanvas {
  div: HTMLDivElement;
  img: HTMLImageElement;
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
  snapshots: string[] = [];
  cursor: number = -1;

  constructor(div: HTMLDivElement) {
    super();
    this.canvas = document.createElement('canvas');
    this.canvas.width = W;
    this.canvas.height = H;
    this.ctx = this.canvas.getContext('2d')!;
    this.div = div;
    this.img = document.createElement('img');
    div.appendChild(this.canvas);
  }

  advance(delta: number) {
    if (this.cursor = -1) this.cursor = this.snapshots.length - 1;
    this.cursor =
        Math.max(0, Math.min(this.cursor + delta, this.snapshots.length - 1));
    this.img.src = this.snapshots[this.cursor];
  }

  render() {
    const imageData = this.ctx.getImageData(0, 0, W, H);
    const view = new DataView(imageData.data.buffer);
    const data = this.bitmaps[this.bitmaps.length - 1].data;
    for (let i = 0; i < SIZE; i++) {
      view.setUint32(i << 2, data[i], false);
    }
      console.log([...new Uint32Array(view.buffer)]);
    this.ctx.putImageData(imageData, 0, 0);
    if (this.cursor < 0) this.img.src = this.canvas.toDataURL();
  }

  override snapshot() {
    this.render();
    this.snapshots.push(this.canvas.toDataURL());
    const img = document.createElement('img');
    img.src = this.snapshots[this.snapshots.length - 1];
    document.getElementById('main-canvas')!.appendChild(img);
  }
}
