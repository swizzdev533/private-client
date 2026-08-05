/**
 * Minimal software rasteriser for the Private Client art pipeline.
 *
 * Everything works on a single-channel linear-light Float32Array. The brand is
 * monochrome, so one channel is all we need, and linear light means the
 * atmospheric falloff behaves like real exposure instead of banding out in the
 * shadows. Gamma encoding happens once, at downsample time.
 */

/** Deterministic PRNG. Seeded so a rebuild produces byte-identical assets. */
export function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function smoothstep(t) {
  return t * t * (3 - 2 * t);
}

/** Seeded 2D value noise. Cheap, and smooth enough for a heightfield. */
export function makeValueNoise(seed) {
  const random = mulberry32(seed);
  const size = 256;
  const table = new Float32Array(size * size);
  for (let i = 0; i < table.length; i += 1) {
    table[i] = random();
  }

  return (x, y) => {
    const x0 = Math.floor(x);
    const y0 = Math.floor(y);
    const fx = smoothstep(x - x0);
    const fy = smoothstep(y - y0);
    const at = (ix, iy) =>
      table[(((iy % size) + size) % size) * size + (((ix % size) + size) % size)];
    const top = at(x0, y0) * (1 - fx) + at(x0 + 1, y0) * fx;
    const bottom = at(x0, y0 + 1) * (1 - fx) + at(x0 + 1, y0 + 1) * fx;
    return top * (1 - fy) + bottom * fy;
  };
}

export function createCanvas(width, height, fill = 0) {
  const data = new Float32Array(width * height);
  if (fill !== 0) data.fill(fill);
  return { width, height, data };
}

/**
 * Fill a convex polygon by testing the sign of the edge cross products.
 * Painter's algorithm handles occlusion, so this writes opaquely.
 */
export function fillConvex(canvas, points, value) {
  const { width, height, data } = canvas;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const [px, py] of points) {
    if (px < minX) minX = px;
    if (px > maxX) maxX = px;
    if (py < minY) minY = py;
    if (py > maxY) maxY = py;
  }

  const x0 = Math.max(0, Math.floor(minX));
  const x1 = Math.min(width - 1, Math.ceil(maxX));
  const y0 = Math.max(0, Math.floor(minY));
  const y1 = Math.min(height - 1, Math.ceil(maxY));
  if (x0 > x1 || y0 > y1) return;

  const n = points.length;
  for (let y = y0; y <= y1; y += 1) {
    const py = y + 0.5;
    for (let x = x0; x <= x1; x += 1) {
      const px = x + 0.5;
      let positive = false;
      let negative = false;
      for (let i = 0; i < n; i += 1) {
        const [ax, ay] = points[i];
        const [bx, by] = points[(i + 1) % n];
        const cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
        if (cross > 0) positive = true;
        else if (cross < 0) negative = true;
        if (positive && negative) break;
      }
      if (!(positive && negative)) {
        data[y * width + x] = value;
      }
    }
  }
}

/**
 * Solid fill over an arbitrary region, driven by an inside test.
 *
 * Antialiasing comes from rendering above the target resolution and box
 * filtering down, so the test itself can stay a hard boolean. Used for icon
 * glyphs, where shapes are defined by distance functions rather than polygons.
 */
export function fillMask(canvas, bounds, inside, value) {
  const { width, height, data } = canvas;
  const x0 = Math.max(0, Math.floor(bounds[0]));
  const y0 = Math.max(0, Math.floor(bounds[1]));
  const x1 = Math.min(width - 1, Math.ceil(bounds[2]));
  const y1 = Math.min(height - 1, Math.ceil(bounds[3]));

  for (let y = y0; y <= y1; y += 1) {
    const py = y + 0.5;
    for (let x = x0; x <= x1; x += 1) {
      if (inside(x + 0.5, py)) {
        data[y * width + x] = value;
      }
    }
  }
}

/** Additive line with a soft falloff, used for the light-catching block edges. */
export function addLine(canvas, a, b, thickness, value) {
  const { width, height, data } = canvas;
  const [ax, ay] = a;
  const [bx, by] = b;
  const dx = bx - ax;
  const dy = by - ay;
  const lengthSq = dx * dx + dy * dy;
  if (lengthSq === 0) return;

  const pad = thickness + 1;
  const x0 = Math.max(0, Math.floor(Math.min(ax, bx) - pad));
  const x1 = Math.min(width - 1, Math.ceil(Math.max(ax, bx) + pad));
  const y0 = Math.max(0, Math.floor(Math.min(ay, by) - pad));
  const y1 = Math.min(height - 1, Math.ceil(Math.max(ay, by) + pad));

  for (let y = y0; y <= y1; y += 1) {
    for (let x = x0; x <= x1; x += 1) {
      const px = x + 0.5 - ax;
      const py = y + 0.5 - ay;
      let t = (px * dx + py * dy) / lengthSq;
      t = t < 0 ? 0 : t > 1 ? 1 : t;
      const ox = px - dx * t;
      const oy = py - dy * t;
      const distance = Math.sqrt(ox * ox + oy * oy);
      if (distance > thickness) continue;
      const falloff = 1 - distance / thickness;
      const index = y * width + x;
      data[index] += value * falloff * falloff;
    }
  }
}

/** Additive radial bloom. This is the volumetric light in the scene. */
export function addRadial(canvas, cx, cy, radius, value, exponent = 2.2) {
  const { width, height, data } = canvas;
  const x0 = Math.max(0, Math.floor(cx - radius));
  const x1 = Math.min(width - 1, Math.ceil(cx + radius));
  const y0 = Math.max(0, Math.floor(cy - radius));
  const y1 = Math.min(height - 1, Math.ceil(cy + radius));

  for (let y = y0; y <= y1; y += 1) {
    for (let x = x0; x <= x1; x += 1) {
      const dx = x + 0.5 - cx;
      const dy = y + 0.5 - cy;
      const distance = Math.sqrt(dx * dx + dy * dy);
      if (distance > radius) continue;
      const falloff = 1 - distance / radius;
      data[y * width + x] += value * Math.pow(falloff, exponent);
    }
  }
}

/** Multiplicative elliptical vignette. Keeps the frame edges from competing. */
export function applyVignette(canvas, strength, softness = 1.35) {
  const { width, height, data } = canvas;
  const cx = width / 2;
  const cy = height / 2;
  const maxDistance = Math.sqrt(cx * cx + cy * cy);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const dx = (x + 0.5 - cx) / cx;
      const dy = (y + 0.5 - cy) / cy;
      const distance = Math.sqrt(dx * dx + dy * dy) / (maxDistance / cx);
      const falloff = 1 - strength * Math.pow(Math.min(1, distance), softness);
      const index = y * width + x;
      data[index] *= falloff < 0 ? 0 : falloff;
    }
  }
}

/**
 * Downsample by an integer factor with a box filter, then gamma-encode to 8-bit.
 * The box filter over an integer factor is exactly supersampled antialiasing,
 * which is what keeps the hairline edges crisp instead of stair-stepped.
 */
export function resolve(canvas, factor, { gamma = 2.2, grain = 0, seed = 1 } = {}) {
  const width = Math.floor(canvas.width / factor);
  const height = Math.floor(canvas.height / factor);
  const out = new Uint8Array(width * height);
  const samples = factor * factor;
  const invGamma = 1 / gamma;
  const random = mulberry32(seed);

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      let total = 0;
      for (let sy = 0; sy < factor; sy += 1) {
        const row = (y * factor + sy) * canvas.width + x * factor;
        for (let sx = 0; sx < factor; sx += 1) {
          total += canvas.data[row + sx];
        }
      }
      let value = total / samples;
      value = value < 0 ? 0 : value > 1 ? 1 : value;
      // Encode after averaging: filtering in linear light, grain in display
      // space, which is where film grain actually lives.
      let encoded = Math.pow(value, invGamma) * 255;
      if (grain > 0) {
        encoded += (random() - 0.5) * grain;
      }
      out[y * width + x] = encoded < 0 ? 0 : encoded > 255 ? 255 : Math.round(encoded);
    }
  }

  return { width, height, samples: out };
}
