import { deflateSync, inflateSync } from "node:zlib";

const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c;
  }
  return table;
})();

function crc32(buffer) {
  let c = 0xffffffff;
  for (let i = 0; i < buffer.length; i += 1) {
    c = CRC_TABLE[(c ^ buffer[i]) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const typed = Buffer.concat([Buffer.from(type, "latin1"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typed), 0);
  return Buffer.concat([length, typed, crc]);
}

/**
 * Paeth predictor, per the PNG specification. Filtering before deflate is what
 * makes a smooth gradient compress; without it these backgrounds are ~10x
 * larger on disk.
 */
function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  if (pb <= pc) return b;
  return c;
}

/**
 * Encode 8-bit samples as a PNG.
 *
 * @param {number} width
 * @param {number} height
 * @param {Uint8Array} samples one byte per pixel (greyscale source)
 * @param {{ rgb?: boolean, alpha?: Uint8Array }} [options] `rgb` emits
 *   truecolour instead of greyscale; Minecraft's texture path is happiest with
 *   truecolour, and the launcher runs in a browser engine that takes greyscale
 *   fine. Passing `alpha` emits RGBA, where `samples` becomes the colour and
 *   `alpha` the coverage. Icon glyphs need that so they can be tinted at draw
 *   time instead of baked to one colour.
 */
export function encodePng(width, height, samples, options = {}) {
  const alpha = options.alpha;
  const rgb = alpha ? true : (options.rgb ?? true);
  const channels = alpha ? 4 : rgb ? 3 : 1;
  const stride = width * channels;

  // Filtered scanlines: 1 filter byte + stride bytes per row.
  const raw = Buffer.alloc((stride + 1) * height);
  const current = Buffer.alloc(stride);
  const previous = Buffer.alloc(stride);

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = y * width + x;
      const value = samples[index];
      if (alpha) {
        current[x * 4] = value;
        current[x * 4 + 1] = value;
        current[x * 4 + 2] = value;
        current[x * 4 + 3] = alpha[index];
      } else if (rgb) {
        current[x * 3] = value;
        current[x * 3 + 1] = value;
        current[x * 3 + 2] = value;
      } else {
        current[x] = value;
      }
    }

    const rowStart = y * (stride + 1);
    raw[rowStart] = 4; // Paeth
    for (let i = 0; i < stride; i += 1) {
      const left = i >= channels ? current[i - channels] : 0;
      const up = previous[i];
      const upLeft = i >= channels ? previous[i - channels] : 0;
      raw[rowStart + 1 + i] = (current[i] - paeth(left, up, upLeft)) & 0xff;
    }

    current.copy(previous);
  }

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = alpha ? 6 : rgb ? 2 : 0; // colour type
  ihdr[10] = 0; // deflate
  ihdr[11] = 0; // adaptive filtering
  ihdr[12] = 0; // no interlace

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/**
 * Decode a PNG to 8-bit greyscale plus alpha.
 *
 * Supports the subset this pipeline produces and consumes: bit depth 8,
 * non-interlaced, colour types 0/2/4/6. Enough to round-trip our own assets and
 * read authored source art without pulling in a native image dependency.
 *
 * @returns {{ width: number, height: number, gray: Uint8Array, alpha: Uint8Array }}
 */
export function decodePng(buffer) {
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  for (let i = 0; i < signature.length; i += 1) {
    if (buffer[i] !== signature[i]) throw new Error("not a PNG");
  }

  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colourType = 0;
  const idat = [];

  let offset = 8;
  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("latin1", offset + 4, offset + 8);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colourType = data[9];
      if (data[12] !== 0) throw new Error("interlaced PNG is not supported");
    } else if (type === "IDAT") {
      idat.push(Buffer.from(data));
    } else if (type === "IEND") {
      break;
    }
    offset += 12 + length;
  }

  if (bitDepth !== 8) throw new Error(`unsupported bit depth ${bitDepth}`);
  const channels = { 0: 1, 2: 3, 4: 2, 6: 4 }[colourType];
  if (!channels) throw new Error(`unsupported colour type ${colourType}`);

  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const gray = new Uint8Array(width * height);
  const alpha = new Uint8Array(width * height);

  const line = Buffer.alloc(stride);
  const previous = Buffer.alloc(stride);

  for (let y = 0; y < height; y += 1) {
    const rowStart = y * (stride + 1);
    const filter = raw[rowStart];
    for (let i = 0; i < stride; i += 1) {
      const value = raw[rowStart + 1 + i];
      const left = i >= channels ? line[i - channels] : 0;
      const up = previous[i];
      const upLeft = i >= channels ? previous[i - channels] : 0;
      let restored;
      switch (filter) {
        case 0: restored = value; break;
        case 1: restored = value + left; break;
        case 2: restored = value + up; break;
        case 3: restored = value + ((left + up) >> 1); break;
        case 4: restored = value + paeth(left, up, upLeft); break;
        default: throw new Error(`unsupported filter ${filter}`);
      }
      line[i] = restored & 0xff;
    }

    for (let x = 0; x < width; x += 1) {
      const base = x * channels;
      const index = y * width + x;
      if (channels === 1) {
        gray[index] = line[base];
        alpha[index] = 255;
      } else if (channels === 2) {
        gray[index] = line[base];
        alpha[index] = line[base + 1];
      } else {
        // Rec. 601 luma. These sources are monochrome anyway, but weighting
        // correctly keeps any stray tint from shifting the extracted mask.
        gray[index] = Math.round(
          line[base] * 0.299 + line[base + 1] * 0.587 + line[base + 2] * 0.114,
        );
        alpha[index] = channels === 4 ? line[base + 3] : 255;
      }
    }

    line.copy(previous);
  }

  return { width, height, gray, alpha };
}
