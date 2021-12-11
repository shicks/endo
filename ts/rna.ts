export const W = 600;
export const H = 600;
export const SIZE = W * H;

type Pos = number;
type Pixel = number; // RGBA (R = msb, A = lsb)
type Bucket = number[];

// const LOG = document.createElement('div');
// document.body.appendChild(LOG);
// LOG.style.whiteSpace = 'pre';

const enum Dir {
  E = 0,
  S = 1,
  W = 2,
  N = 3,
}
const enum Rgb {
  Black = 0,
  Red = 1,
  Green = 2,
  Yellow = 3,
  Blue = 4,
  Magenta = 5,
  Cyan = 6,
  White = 7,
}
const enum Alpha {
  Transparent = 8,
  Opaque = 9,
}
type Color = Rgb | Alpha;

function pixel(bucket: Bucket): Pixel {
// LOG.textContent += `PIXEL: bucket=${bucket.join(' ')}\n`;
  let n = 0;
  let r = 0;
  let g = 0;
  let b = 0;
  for (let i = 0; i < 8; i++) {
    const v = bucket[i];
    n += v;
    if (i & 1) r += 255 * v;
    if (i & 2) g += 255 * v;
    if (i & 4) b += 255 * v;
  }
  r = (r / n) | 0;
  g = (g / n) | 0;
  b = (b / n) | 0;
  const o = bucket[Alpha.Opaque];
  const na = o + bucket[Alpha.Transparent];
  const a = na ? (255 * o / na) | 0 : 255;
  return ((r << 24) | (g << 16) | (b << 8) | a) >>> 0;
}

export class Bitmap {
  readonly data: Uint32Array;
  constructor(data?: Uint32Array) {
    this.data = data || new Uint32Array(SIZE);
  }

  setPixel(pos: Pos, pixel: Pixel) {
    this.data[pos] = pixel;
  }

  line(p0: Pos, p1: Pos, pixel: Pixel) {
    const x0 = p0 % W;
    const y0 = (p0 / W) | 0;
    const x1 = p1 % W;
    const y1 = (p1 / W) | 0;
    // LOG.textContent += `Line ${x0},${y0} - ${x1},${y1} @ ${pixel.toString(16)}\n`;
    const dx = x1 - x0;
    const dy = y1 - y0;
    const d = Math.max(Math.abs(dx), Math.abs(dy));
    const c = (dx * dy) <= 0 ? 1 : 0;
    let x = x0 * d + (d - c >> 1);
    let y = y0 * d + (d - c >> 1);
    for (let i = 0; i < d; i++) {
      const pos = (y / d | 0) * W + (x / d | 0);
// LOG.textContent += `  ${pos%W},${pos/W|0}`;
      x += dx;
      y += dy;
      // TODO - consider saving previous value?
      this.data[pos] = pixel;
    }
    this.data[y1 * W + x1] = pixel;
// LOG.textContent += `\n`;
  }

  tryFill(pos: Pos, pixel: Pixel) {
    // LOG.textContent += `Fill ${pos % W},${pos / W | 0} @ ${pixel.toString(16)}\n`;
    let old = this.data[pos];
    if (pixel === old) return;
    const seen = new Set<Pos>([pos]);
    for (const p of seen) {
      if (this.data[p] !== old) continue;
      this.data[p] = pixel;
      if (p >= W) seen.add(p - W);
      if (p < SIZE - W) seen.add(p + W);
      const x = p % W;
      if (x > 0) seen.add(p - 1);
      if (x < W - 1) seen.add(p + 1);
    }
  }

  // NOTE: bitmaps are mutable... We could potentially
  // use a persistent bitmap instead, but it's not clear
  // that this would be an improvement...
  compose(that: Bitmap): Bitmap {
    // Usage: bitmaps[1].compose(bitmaps[0]): r <- r1*(1-a0) + r0
    const data = new Uint32Array(SIZE);
    for (let pos = 0; pos < SIZE; pos++) {
      const p0 = that.data[pos];
      const p1 = this.data[pos];
      const r0 = p0 >>> 24;
      const r1 = p1 >>> 24;
      const g0 = (p0 >>> 16) & 255;
      const g1 = (p1 >>> 16) & 255;
      const b0 = (p0 >>> 8) & 255;
      const b1 = (p1 >>> 8) & 255;
      const a0 = p0 & 255;
      const a1 = p1 & 255;
      const r = Math.min(255, r0 + (r1 * (255 - a0) / 255 | 0));
      const g = Math.min(255, g0 + (g1 * (255 - a0) / 255 | 0));
      const b = Math.min(255, b0 + (b1 * (255 - a0) / 255 | 0));
      const a = Math.min(255, a0 + (a1 * (255 - a0) / 255 | 0));
      data[pos] = r << 24 | g << 16 | b << 8 | a;
    }
    return new Bitmap(data);
  }

  clip(that: Bitmap): Bitmap {
    // Usage: bitmaps[1].clip(bitmaps[0]): r <- r1*a0
    const data = new Uint32Array(SIZE);
    for (let pos = 0; pos < SIZE; pos++) {
      const p0 = that.data[pos];
      const a0 = p0 & 255;
      const p1 = this.data[pos];
      const r1 = p1 >>> 24;
      const g1 = (p1 >>> 16) & 255;
      const b1 = (p1 >>> 8) & 255;
      const a1 = p1 & 255;
      const r = (r1 * a0 / 255) | 0;
      const g = (g1 * a0 / 255) | 0;
      const b = (b1 * a0 / 255) | 0;
      const a = (a1 * a0 / 255) | 0;
      data[pos] = r << 24 | g << 16 | b << 8 | a;
    }
    return new Bitmap(data);
  }
}

export abstract class AbstractRnaCanvas {
  lines: number = 0;
  bitmaps: Bitmap[] = [new Bitmap()];
  bucket: Bucket = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
  pixel: Pixel|undefined = 255;
  pos: Pos = 0;
  mark: Pos = 0;
  dir: Dir = Dir.E;

  //abstract render(data: Uint32Array): void;
  abstract snapshot(): void;

  process(rna: string) {
    switch (rna) {
      case 'PIPIIIC': this.addColor(Rgb.Black); break;
      case 'PIPIIIP': this.addColor(Rgb.Red); break;
      case 'PIPIICC': this.addColor(Rgb.Green); break;
      case 'PIPIICF': this.addColor(Rgb.Yellow); break;
      case 'PIPIICP': this.addColor(Rgb.Blue); break;
      case 'PIPIIFC': this.addColor(Rgb.Magenta); break;
      case 'PIPIIFF': this.addColor(Rgb.Cyan); break;
      case 'PIPIIPC': this.addColor(Rgb.White); break;
      case 'PIPIIPF': this.addColor(Alpha.Transparent); break;
      case 'PIPIIPP': this.addColor(Alpha.Opaque); break;
      case 'PIIPICP': this.emptyBucket(); break;
      case 'PIIIIIP': this.move(); break;
      case 'PCCCCCP': this.turnCounterClockwise(); break;
      case 'PFFFFFP': this.turnClockwise(); break;
      case 'PCCIFFP': this.setMark(); break;
      case 'PFFICCP':
        this.line();
        if (++this.lines == 25) {
          this.lines = 0;
          this.snapshot();
        }
        break;
      case 'PIIPIIP':
        this.maybeSnapshot();
        this.tryFill();
        this.snapshot();
        break;
      case 'PCCPFFP':
        this.maybeSnapshot();
        this.addBitmap();
        break;
      case 'PFFPCCP':
        this.maybeSnapshot();
        this.compose();
        this.snapshot();
        break;
      case 'PFFICCF':
        this.maybeSnapshot();
        this.clip();
        this.snapshot();
        break;
      default:
        //console.log(`Got unknown RNA ${rna}`);
    }
  }
  done() {
    this.maybeSnapshot();
  }

  maybeSnapshot() {
    if (this.lines > 0) this.snapshot();
    this.lines = 0;
  }

  addColor(color: Color) {
    this.pixel = undefined;
    this.bucket[color]++;
  }
  emptyBucket() {
    this.pixel = 255;
    this.bucket.fill(0);
  }
  move() {
    let x = this.pos % W;
    let y = this.pos / W | 0;
    switch (this.dir) {
      case Dir.E: x += 1; break;
      case Dir.S: y += 1; break;
      case Dir.W: x += W - 1; break;
      case Dir.N: y += H - 1; break;
    }
    x %= W;
    y %= H;
    this.pos = y * W + x;
  }
  turnCounterClockwise() {
    this.dir = this.dir + 3 & 3;
  }
  turnClockwise() {
    this.dir = this.dir + 1 & 3;
  }
  setMark() {
    this.mark = this.pos;
  }
  line() {
    if (this.pixel == null) this.pixel = pixel(this.bucket);
    this.bitmaps[this.bitmaps.length - 1].line(this.pos, this.mark, this.pixel);
  }
  tryFill() {
    //return;
    if (this.pixel == null) this.pixel = pixel(this.bucket);
    this.bitmaps[this.bitmaps.length - 1].tryFill(this.pos, this.pixel);
  }
  addBitmap() {
    //return;
    this.bitmaps.push(new Bitmap());
  }
  compose() {
    //return;
    if (this.bitmaps.length < 2) return;
    const top = this.bitmaps.pop()!;
    const bottom = this.bitmaps.pop()!;
    this.bitmaps.push(bottom.compose(top));
  }
  clip() {
    //return;
    if (this.bitmaps.length < 2) return;
    const top = this.bitmaps.pop()!;
    const bottom = this.bitmaps.pop()!;
    this.bitmaps.push(bottom.clip(top));
  }
}
