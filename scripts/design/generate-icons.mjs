/**
 * Private Client main-menu icon atlas.
 *
 * The menu glyphs used to be drawn with drawRect primitives, one scanline at a
 * time, at integer GUI coordinates. That has no antialiasing by construction,
 * and Minecraft's GUI scale then multiplies every step into a 2x2 or 3x3 block.
 * Rendering them here instead, well above their display size and with real
 * coverage antialiasing, is the same fix the nametag badge needed.
 *
 * Glyphs are white with a coverage alpha so the GUI can tint them per state
 * rather than baking one colour into the texture.
 *
 *   node scripts/design/generate-icons.mjs [--preview]
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve as resolvePath } from "node:path";
import { fileURLToPath } from "node:url";
import { createCanvas, fillConvex, fillMask, resolve as resolveCanvas } from "./lib/raster.mjs";
import { encodePng } from "./lib/png.mjs";

const ROOT = resolvePath(dirname(fileURLToPath(import.meta.url)), "../..");

/** Display size is 30px inside a 30px button, so 128 leaves 4x headroom. */
const CELL = 128;
const SSAA = 3;
const ICON_COUNT = 6;

const ON = 1;
const OFF = 0;

// ------------------------------------------------------------ shape tests

const disc = (cx, cy, r) => (x, y) => {
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= r * r;
};

const roundedRect = (cx, cy, hw, hh, r) => (x, y) => {
  const dx = Math.abs(x - cx) - (hw - r);
  const dy = Math.abs(y - cy) - (hh - r);
  if (dx <= 0 || dy <= 0) {
    return Math.abs(x - cx) <= hw && Math.abs(y - cy) <= hh;
  }
  return dx * dx + dy * dy <= r * r;
};

const rotatedRoundedRect = (cx, cy, hw, hh, r, angle) => {
  const cos = Math.cos(-angle);
  const sin = Math.sin(-angle);
  const local = roundedRect(0, 0, hw, hh, r);
  return (x, y) => {
    const dx = x - cx;
    const dy = y - cy;
    return local(dx * cos - dy * sin, dx * sin + dy * cos);
  };
};

const capsule = (ax, ay, bx, by, r) => (x, y) => {
  const dx = bx - ax;
  const dy = by - ay;
  const lengthSq = dx * dx + dy * dy;
  let t = lengthSq === 0 ? 0 : ((x - ax) * dx + (y - ay) * dy) / lengthSq;
  t = t < 0 ? 0 : t > 1 ? 1 : t;
  const ox = x - (ax + dx * t);
  const oy = y - (ay + dy * t);
  return ox * ox + oy * oy <= r * r;
};

/** Annulus with an optional angular gap, measured clockwise from straight up. */
const ring = (cx, cy, outer, inner, gapHalfAngle = 0) => (x, y) => {
  const dx = x - cx;
  const dy = y - cy;
  const distanceSq = dx * dx + dy * dy;
  if (distanceSq > outer * outer || distanceSq < inner * inner) return false;
  if (gapHalfAngle <= 0) return true;
  // atan2 with screen-space y: straight up is -PI/2.
  let delta = Math.atan2(dy, dx) + Math.PI / 2;
  while (delta > Math.PI) delta -= Math.PI * 2;
  while (delta < -Math.PI) delta += Math.PI * 2;
  return Math.abs(delta) > gapHalfAngle;
};

const union = (...tests) => (x, y) => tests.some((test) => test(x, y));

/** Grow a shape outward, used to knock a clean gap between overlapping forms. */
const dilate = (test, amount) => (x, y) => {
  for (let a = 0; a <= amount; a += amount) {
    if (test(x, y)) return true;
  }
  for (let angle = 0; angle < Math.PI * 2; angle += Math.PI / 6) {
    if (test(x + Math.cos(angle) * amount, y + Math.sin(angle) * amount)) return true;
  }
  return false;
};

// ----------------------------------------------------------------- glyphs

/**
 * One entry per menu action, in the order the menu builds its button row:
 * Singleplayer, Multiplayer, Mods, Options, Accounts, Quit.
 *
 * Coordinates are local to a 128px cell centred on (0, 0). Weights are tuned
 * against each other rather than in isolation, which is what makes the row read
 * as one set instead of six unrelated drawings.
 */
const GLYPHS = [
  // Singleplayer: one person.
  (paint) => {
    paint(disc(0, -24, 18), ON);
    paint(roundedRect(0, 30, 32, 20, 16), ON);
  },

  // Multiplayer: two people, the front one knocked out of the back one so the
  // silhouettes stay separate at small sizes.
  (paint) => {
    const backHead = disc(17, -22, 14);
    const backBody = roundedRect(17, 26, 25, 16, 13);
    paint(union(backHead, backBody), ON);

    const frontHead = disc(-15, -18, 16);
    const frontBody = roundedRect(-15, 30, 28, 18, 14);
    paint(dilate(union(frontHead, frontBody), 6), OFF);
    paint(union(frontHead, frontBody), ON);
  },

  // Mods: a package. Lid and body carry a real gap rather than touching.
  (paint) => {
    paint(roundedRect(0, -30, 44, 13, 5), ON);
    paint(roundedRect(0, 16, 36, 29, 6), ON);
  },

  // Options: a gear. Four crossing bars give eight teeth, then the hub and bore.
  (paint) => {
    for (let i = 0; i < 4; i += 1) {
      paint(rotatedRoundedRect(0, 0, 10, 45, 5, (i * Math.PI) / 4), ON);
    }
    paint(disc(0, 0, 31), ON);
    paint(disc(0, 0, 13), OFF);
  },

  // Accounts: two opposing arrows.
  (paint, canvas, toCanvas) => {
    paint(capsule(-28, -15, 14, -15, 6), ON);
    paint(capsule(28, 15, -14, 15, 6), ON);
    fillConvex(canvas, [toCanvas(38, -15), toCanvas(13, -30), toCanvas(13, 0)], ON);
    fillConvex(canvas, [toCanvas(-38, 15), toCanvas(-13, 0), toCanvas(-13, 30)], ON);
  },

  // Quit: the power symbol. The gap under the stem is what makes it read.
  (paint) => {
    paint(ring(0, 4, 34, 23, 0.62), ON);
    paint(roundedRect(0, -22, 6, 24, 6), ON);
  },
];

function main() {
  const preview = process.argv.includes("--preview");
  const width = CELL * ICON_COUNT;
  const canvas = createCanvas(width * SSAA, CELL * SSAA);

  GLYPHS.forEach((glyph, index) => {
    const originX = (index * CELL + CELL / 2) * SSAA;
    const originY = (CELL / 2) * SSAA;
    const toCanvas = (x, y) => [originX + x * SSAA, originY + y * SSAA];

    const paint = (test, value) => {
      const half = (CELL / 2) * SSAA;
      fillMask(
        canvas,
        [originX - half, originY - half, originX + half, originY + half],
        (px, py) => test((px - originX) / SSAA, (py - originY) / SSAA),
        value,
      );
    };

    glyph(paint, canvas, toCanvas);
  });

  // Coverage must stay linear: this resolves to an alpha channel, not to a
  // displayed colour, so gamma encoding it would thin every edge.
  const resolved = resolveCanvas(canvas, SSAA, { gamma: 1 });
  const white = new Uint8Array(resolved.width * resolved.height).fill(255);
  const png = encodePng(resolved.width, resolved.height, white, { alpha: resolved.samples });

  const destination = preview
    ? resolvePath(ROOT, "artifacts/design/menu-icons.png")
    : resolvePath(
        ROOT,
        "minecraft/private-client-core/src/main/resources/assets/privateclientcore/textures/gui/menu-icons.png",
      );

  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, png);
  console.log(
    `menu-icons  ${resolved.width}x${resolved.height}  ${(png.length / 1024).toFixed(1)} KB`,
  );

  if (preview) {
    const sheet = resolvePath(ROOT, "artifacts/design/menu-icons-sheet.png");
    writeFileSync(sheet, buildContactSheet(resolved));
    console.log(`contact sheet -> ${sheet}`);
  }
}

/**
 * Composite the glyphs over the button's real idle colours at their real
 * display size. Judging an icon set on a transparent checkerboard at 4x is how
 * you ship something that turns to mush in the actual menu.
 */
function buildContactSheet(resolved) {
  const sizes = [30, 60, 120];
  const gap = 10;
  const width = sizes[sizes.length - 1] * ICON_COUNT + gap * (ICON_COUNT + 1);
  const height = sizes.reduce((total, size) => total + size + gap, gap);

  const BUTTON_FILL = 0x14;
  const GLYPH = 0xe8;
  const out = new Uint8Array(width * height).fill(0x0a);

  let cursorY = gap;
  for (const size of sizes) {
    for (let icon = 0; icon < ICON_COUNT; icon += 1) {
      const originX = gap + icon * (sizes[sizes.length - 1] + gap);
      for (let y = 0; y < size; y += 1) {
        for (let x = 0; x < size; x += 1) {
          // Box-average the source cell so the preview matches what linear
          // filtering will do on the GPU.
          const scale = CELL / size;
          const sx0 = Math.floor(icon * CELL + x * scale);
          const sy0 = Math.floor(y * scale);
          const step = Math.max(1, Math.floor(scale));
          let total = 0;
          let count = 0;
          for (let sy = sy0; sy < sy0 + step && sy < resolved.height; sy += 1) {
            for (let sx = sx0; sx < sx0 + step && sx < resolved.width; sx += 1) {
              total += resolved.samples[sy * resolved.width + sx];
              count += 1;
            }
          }
          const alpha = count === 0 ? 0 : total / count / 255;
          const value = BUTTON_FILL * (1 - alpha) + GLYPH * alpha;
          out[(cursorY + y) * width + originX + x] = Math.round(value);
        }
      }
    }
    cursorY += size + gap;
  }

  return encodePng(width, height, out, { rgb: true });
}

main();
