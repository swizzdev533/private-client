/**
 * Private Client procedural background generator.
 *
 * Renders a monochrome isometric block field: the brand mark is an isometric
 * cube, so the environment is that cube fragmented into architecture. Output is
 * resolution-independent by construction, which is the point. Rasterising the
 * geometry ourselves at 2-3x and box-filtering down gives genuinely sharp edges
 * at any target size, instead of upscaling a fixed-size render.
 *
 * Deterministic: same seed in, byte-identical PNG out.
 *
 *   node scripts/design/generate-backgrounds.mjs [--preview]
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve as resolvePath } from "node:path";
import { fileURLToPath } from "node:url";
import {
  addLine,
  addRadial,
  applyVignette,
  createCanvas,
  fillConvex,
  makeValueNoise,
  mulberry32,
  resolve as resolveCanvas,
} from "./lib/raster.mjs";
import { encodePng } from "./lib/png.mjs";

const ROOT = resolvePath(dirname(fileURLToPath(import.meta.url)), "../..");

/** Linear-light luminance values. Deliberately low: this sits behind UI. */
const SURFACE = {
  sky: 0.0035,
  horizon: 0.030,
  gridLine: 0.020,
  faceTop: 0.082,
  faceLeft: 0.028,
  faceRight: 0.009,
};

function renderScene(config) {
  const {
    width,
    height,
    ssaa,
    seed,
    gridSize,
    maxHeight,
    valleyCenter,
    valleyWidth,
    horizon,
    lights,
    fog,
    vignette,
    windowDensity,
  } = config;

  const W = width * ssaa;
  const H = height * ssaa;
  const canvas = createCanvas(W, H);
  const noise = makeValueNoise(seed);
  const random = mulberry32(seed ^ 0x9e3779b9);

  // ---------------------------------------------------------------- sky
  // Tight exponential lift at the horizon line only. Everything above and
  // below falls to near-black so UI text keeps its contrast.
  const horizonY = H * horizon;
  for (let y = 0; y < H; y += 1) {
    const distance = Math.abs(y - horizonY) / H;
    const glow = Math.exp(-distance * 13);
    const value = SURFACE.sky + (SURFACE.horizon - SURFACE.sky) * glow;
    canvas.data.fill(value, y * W, y * W + W);
  }

  /**
   * Accumulated light reaching a screen position. Drives both the rim light on
   * block edges and the airlight term, so geometry and atmosphere agree about
   * where the light actually is.
   */
  const lightAt = (x, y) => {
    let total = 0;
    for (const light of lights) {
      const dx = (x - light.x * W) / W;
      const dy = (y - light.y * H) / W;
      total += light.intensity * Math.exp(-Math.sqrt(dx * dx + dy * dy) / light.reach);
    }
    return total;
  };

  // ------------------------------------------------------- light sources
  // Drawn before the geometry so the blocks occlude them and read as
  // silhouettes against the light, which is where the depth comes from.
  for (const light of lights) {
    addRadial(canvas, light.x * W, light.y * H, light.radius * W, light.intensity, 2.4);
  }

  // ------------------------------------------------------------ geometry
  const tile = (W / gridSize) * 0.92;
  const tileHalf = tile / 2;
  const tileQuarter = tile / 4;
  const originX = W / 2;
  const originY = horizonY - (gridSize * tile) / 8;
  const blockMax = H * maxHeight;

  const project = (gx, gy, h) => [
    originX + (gx - gy) * tileHalf,
    originY + (gx + gy) * tileQuarter - h,
  ];

  // Ground grid. Deliberately overshoots the block field on every side so the
  // plane has no visible boundary wedge inside the frame.
  const groundFrom = -gridSize;
  const groundTo = gridSize * 2;
  for (let g = groundFrom; g <= groundTo; g += 2) {
    const fade = Math.exp(-Math.abs(g - gridSize / 2) / gridSize) * 0.9 + 0.1;
    addLine(
      canvas,
      project(g, groundFrom, 0),
      project(g, groundTo, 0),
      ssaa * 0.55,
      SURFACE.gridLine * fade,
    );
    addLine(
      canvas,
      project(groundFrom, g, 0),
      project(groundTo, g, 0),
      ssaa * 0.55,
      SURFACE.gridLine * fade,
    );
  }

  const blocks = [];
  for (let gx = 0; gx < gridSize; gx += 1) {
    for (let gy = 0; gy < gridSize; gy += 1) {
      // u runs across the frame. Larger (gx + gy) is drawn lower and later, so
      // it is CLOSER to the viewer, not further away.
      const u = (gx - gy) / gridSize;
      const nearness = (gx + gy) / (2 * gridSize);

      // Carve the corridor: nothing tall grows near the centre line, so the
      // middle of the frame stays dark and readable.
      const offset = Math.abs(u - valleyCenter);
      const valley = Math.min(1, Math.max(0, (offset - valleyWidth) / 0.42));
      if (valley <= 0.02) continue;

      // Deliberate asymmetry. A mirrored pair of masses reads as a machine
      // rendering, not as a place: the left bank runs taller and tighter, the
      // right bank lower and more broken.
      const leftSide = u < valleyCenter;
      const sideScale = leftSide ? 1.0 : 0.74;
      const sideRoughness = leftSide ? 0.78 : 1.0;

      const n =
        noise(gx * 0.31, gy * 0.31) * 0.65 + noise(gx * 0.11, gy * 0.11) * 0.35;
      const broken = noise(gx * 0.83 + 40, gy * 0.83 + 40);
      if (broken < sideRoughness * 0.22) continue;

      // Drop the height of the closest rank so the foreground does not become
      // a wall across the bottom of the frame.
      const nearFade = Math.min(1, (1 - nearness) * 3.2);
      const h =
        blockMax * sideScale * Math.pow(valley, 1.5) * (0.22 + n * 0.78) * nearFade;
      if (h < blockMax * 0.02) continue;

      blocks.push({ gx, gy, h, nearness, u });
    }
  }

  // Painter's algorithm: (gx + gy) increases toward the viewer.
  blocks.sort((a, b) => a.gx + a.gy - (b.gx + b.gy));

  for (const block of blocks) {
    const { gx, gy, h, nearness } = block;
    const distance = 1 - nearness;

    const topA = project(gx, gy, h);
    const topB = project(gx + 1, gy, h);
    const topC = project(gx + 1, gy + 1, h);
    const topD = project(gx, gy + 1, h);
    const baseB = project(gx + 1, gy, 0);
    const baseC = project(gx + 1, gy + 1, 0);
    const baseD = project(gx, gy + 1, 0);

    // Slight per-block variation stops the field reading as one flat mass.
    const variation = 0.82 + noise(gx * 0.7, gy * 0.7) * 0.36;

    // Extinction plus airlight, rather than a lerp toward the sky colour.
    // Distant geometry attenuates toward black, and only the haze near a light
    // source glows. Lerping to sky instead flattens the whole field to one
    // mid-grey, which is exactly what the first pass of this scene did.
    // The scene is backlit. Faces receive almost no direct light, so near
    // geometry reads as a near-black silhouette and everything visible in the
    // distance is airlight: haze between the viewer and the object, which
    // grows with distance. Lighting the faces directly instead produces bright
    // slabs in the foreground, which is the opposite of the intended look.
    const incident = lightAt(topC[0], topC[1]);
    const attenuation = Math.exp(-distance * fog);
    const airlight = incident * (1 - attenuation) * 0.20;
    const shade = (base) => base * variation * 0.30 * attenuation + airlight;

    fillConvex(canvas, [topB, topC, baseC, baseB], shade(SURFACE.faceRight));
    fillConvex(canvas, [topD, topC, baseC, baseD], shade(SURFACE.faceLeft));
    fillConvex(canvas, [topA, topB, topC, topD], shade(SURFACE.faceTop));

    // Rim light on the silhouette edges facing the source.
    const rim = incident * attenuation * 0.85;

    if (rim > 0.001) {
      const thickness = ssaa * 0.75;
      addLine(canvas, topA, topB, thickness, rim * 0.5);
      addLine(canvas, topB, topC, thickness, rim * 0.85);
      addLine(canvas, topD, topC, thickness, rim * 0.7);
      addLine(canvas, topC, baseC, thickness, rim * 0.6);
    }

    // Sparse lit apertures. Small, rare, and only on near blocks: this is the
    // detail that stops procedural geometry looking synthetic.
    if (nearness > 0.38 && random() < windowDensity) {
      const t = 0.25 + random() * 0.5;
      const from = [topD[0] + (baseD[0] - topD[0]) * t, topD[1] + (baseD[1] - topD[1]) * t];
      const to = [topC[0] + (baseC[0] - topC[0]) * t, topC[1] + (baseC[1] - topC[1]) * t];
      addLine(canvas, from, to, ssaa * 0.7, 0.30 * attenuation);
    }
  }

  // --------------------------------------------------------- atmosphere
  // A second bloom in front of the geometry: the haze the light travels
  // through. Kept deliberately weak and tight. Widening it washes a flat lift
  // across every block face in that half of the frame and kills the contrast
  // the extinction model just bought.
  for (const light of lights) {
    addRadial(
      canvas,
      light.x * W,
      light.y * H,
      light.radius * W * 1.6,
      light.intensity * 0.12,
      2.4,
    );
  }

  applyVignette(canvas, vignette);

  return resolveCanvas(canvas, ssaa, { grain: config.grain, seed: seed ^ 0x5bf03635 });
}

// --------------------------------------------------------------- presets

/**
 * Two lights flanking a dark corridor is the composition the existing key art
 * established. Keeping it means the redesign reads as the same product.
 */
const CORRIDOR_LIGHTS = [
  // Key light, high and left, tucked behind the taller bank.
  { x: 0.255, y: 0.20, radius: 0.20, reach: 0.20, intensity: 0.62 },
  // Fill, weaker, lower, further out. Unequal on purpose.
  { x: 0.815, y: 0.335, radius: 0.145, reach: 0.145, intensity: 0.30 },
];

const BASE = {
  ssaa: 2,
  gridSize: 44,
  maxHeight: 0.50,
  // Corridor pushed off centre so the frame is not a mirror.
  valleyCenter: 0.045,
  valleyWidth: 0.14,
  horizon: 0.40,
  lights: CORRIDOR_LIGHTS,
  /** Extinction coefficient: how fast geometry attenuates toward black. */
  fog: 3.2,
  vignette: 0.80,
  windowDensity: 0.07,
  grain: 2.4,
};

const TARGETS = [
  {
    name: "launcher-bg",
    config: { ...BASE, width: 2560, height: 1440, seed: 20260804 },
    outputs: ["apps/launcher/src/assets/private-client-bg.png"],
    rgb: false,
  },
  {
    name: "menu-background",
    // Slightly wider corridor and a lower horizon: the Minecraft main menu
    // stacks buttons down the middle and needs that column clean.
    config: {
      ...BASE,
      width: 1920,
      height: 1080,
      seed: 20260805,
      valleyWidth: 0.16,
      horizon: 0.34,
      vignette: 0.68,
    },
    outputs: [
      "minecraft/private-client-core/src/main/resources/assets/privateclientcore/textures/gui/menu-background.png",
    ],
    rgb: true,
  },
  {
    name: "loading-background",
    // The splash is centred on the mark, so the corridor opens wider still and
    // the field sits lower in frame.
    config: {
      ...BASE,
      width: 1920,
      height: 1080,
      seed: 20260806,
      gridSize: 30,
      valleyWidth: 0.21,
      maxHeight: 0.46,
      horizon: 0.31,
      vignette: 0.74,
      windowDensity: 0.05,
    },
    outputs: [
      "minecraft/private-client-core/src/main/resources/assets/privateclientcore/textures/gui/loading-background.png",
    ],
    rgb: true,
  },
];

function main() {
  const preview = process.argv.includes("--preview");
  const previewDir = process.env.PC_PREVIEW_DIR ?? resolvePath(ROOT, "artifacts/design");

  for (const target of TARGETS) {
    const started = Date.now();
    const { width, height, samples } = renderScene(target.config);
    const png = encodePng(width, height, samples, { rgb: target.rgb });
    const elapsed = ((Date.now() - started) / 1000).toFixed(1);

    const destinations = preview
      ? [resolvePath(previewDir, `${target.name}.png`)]
      : target.outputs.map((relative) => resolvePath(ROOT, relative));

    for (const destination of destinations) {
      mkdirSync(dirname(destination), { recursive: true });
      writeFileSync(destination, png);
    }

    console.log(
      `${target.name.padEnd(20)} ${width}x${height}  ${(png.length / 1024)
        .toFixed(0)
        .padStart(5)} KB  ${elapsed}s`,
    );
  }
}

main();
