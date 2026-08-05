/**
 * Builds the Private Client icon source from the brand mark.
 *
 * The authored mark is a white stroke on black with a soft outer glow. This
 * lifts the stroke out as a coverage mask, drops the glow, and recomposes it on
 * the product's near-black surface at the size Tauri's icon generator expects.
 *
 * The thin-line rendition of the same logo is not usable here: at 32px, and
 * especially at the 16px Windows uses in a title bar, hairline strokes fall
 * below one pixel and disappear. The heavy-stroke rendition is the same mark at
 * a weight that survives, which is why it is the source of truth.
 *
 *   node scripts/design/generate-logo.mjs [--preview]
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve as resolvePath } from "node:path";
import { fileURLToPath } from "node:url";
import { decodePng, encodePng } from "./lib/png.mjs";

const ROOT = resolvePath(dirname(fileURLToPath(import.meta.url)), "../..");

/** Authored artwork, checked in so the derivation is reproducible. */
const SOURCE = resolvePath(ROOT, "artifacts/design/brand/private-client-mark.png");
const OUTPUT = resolvePath(ROOT, "apps/launcher/src/assets/private-client-icon-source.png");

const SIZE = 1024;
/** Share of the canvas the mark occupies. Leaves the margin icons need. */
const MARK_SCALE = 0.66;
/** Luminance ramp that separates the stroke from its glow. */
const CUT_LOW = 232;
const CUT_HIGH = 250;
/** Near-black, matching the launcher's --ink-0. Never pure black. */
const BACKGROUND = 0x0b;

/** Extract the stroke as coverage, discarding the authored glow. */
function extractMask(source) {
  const { width, height, gray } = source;
  const mask = new Float32Array(width * height);
  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = y * width + x;
      let coverage = (gray[index] - CUT_LOW) / (CUT_HIGH - CUT_LOW);
      coverage = coverage < 0 ? 0 : coverage > 1 ? 1 : coverage;
      mask[index] = coverage;
      if (coverage > 0.5) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }
    }
  }

  if (maxX < 0) {
    throw new Error("no mark found in source: check the luminance cut");
  }
  return { mask, width, height, minX, minY, maxX, maxY };
}

/**
 * Area-average resample. A box filter over the source footprint of each target
 * pixel, which is the correct filter for downscaling and keeps the stroke from
 * breaking up.
 */
function resample(mask, width, sourceBox, targetSize) {
  const [x0, y0, x1, y1] = sourceBox;
  const boxWidth = x1 - x0;
  const boxHeight = y1 - y0;
  const out = new Float32Array(targetSize * targetSize);

  for (let ty = 0; ty < targetSize; ty += 1) {
    const sy0 = y0 + (ty / targetSize) * boxHeight;
    const sy1 = y0 + ((ty + 1) / targetSize) * boxHeight;
    for (let tx = 0; tx < targetSize; tx += 1) {
      const sx0 = x0 + (tx / targetSize) * boxWidth;
      const sx1 = x0 + ((tx + 1) / targetSize) * boxWidth;

      let total = 0;
      let weight = 0;
      for (let sy = Math.floor(sy0); sy < Math.ceil(sy1); sy += 1) {
        const coverY = Math.min(sy + 1, sy1) - Math.max(sy, sy0);
        if (coverY <= 0) continue;
        for (let sx = Math.floor(sx0); sx < Math.ceil(sx1); sx += 1) {
          const coverX = Math.min(sx + 1, sx1) - Math.max(sx, sx0);
          if (coverX <= 0) continue;
          const area = coverX * coverY;
          total += mask[sy * width + sx] * area;
          weight += area;
        }
      }
      out[ty * targetSize + tx] = weight === 0 ? 0 : total / weight;
    }
  }
  return out;
}

function main() {
  const preview = process.argv.includes("--preview");
  const source = decodePng(readFileSync(SOURCE));
  const extracted = extractMask(source);

  // Square the crop around the mark so resampling cannot distort it.
  const centreX = (extracted.minX + extracted.maxX + 1) / 2;
  const centreY = (extracted.minY + extracted.maxY + 1) / 2;
  const side = Math.max(
    extracted.maxX - extracted.minX + 1,
    extracted.maxY - extracted.minY + 1,
  );
  const half = side / 2;

  const markSize = Math.round(SIZE * MARK_SCALE);
  const mark = resample(
    extracted.mask,
    extracted.width,
    [centreX - half, centreY - half, centreX + half, centreY + half],
    markSize,
  );

  const canvas = new Uint8Array(SIZE * SIZE).fill(BACKGROUND);
  const offset = Math.round((SIZE - markSize) / 2);
  for (let y = 0; y < markSize; y += 1) {
    for (let x = 0; x < markSize; x += 1) {
      const coverage = mark[y * markSize + x];
      if (coverage <= 0) continue;
      const index = (y + offset) * SIZE + (x + offset);
      canvas[index] = Math.round(BACKGROUND * (1 - coverage) + 255 * coverage);
    }
  }

  const destination = preview
    ? resolvePath(ROOT, "artifacts/design/icon-source.png")
    : OUTPUT;
  mkdirSync(dirname(destination), { recursive: true });
  const png = encodePng(SIZE, SIZE, canvas, { rgb: true });
  writeFileSync(destination, png);

  console.log(
    `icon source  ${SIZE}x${SIZE}  ${(png.length / 1024).toFixed(0)} KB  ` +
      `mark ${side}px -> ${markSize}px`,
  );
}

main();
