import { expect, test } from "@playwright/test";

import { assertRenderedPngDelta, assertRenderedPngQuality } from "./image_quality";

const { PNG } = require("pngjs");

function solidPng(width: number, height: number, rgba: [number, number, number, number]): Buffer {
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = (width * y + x) * 4;
      png.data[offset] = rgba[0];
      png.data[offset + 1] = rgba[1];
      png.data[offset + 2] = rgba[2];
      png.data[offset + 3] = rgba[3];
    }
  }
  return PNG.sync.write(png);
}

function boardLikePng(): Buffer {
  const width = 320;
  const height = 180;
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = (width * y + x) * 4;
      png.data[offset] = 255;
      png.data[offset + 1] = 255;
      png.data[offset + 2] = 255;
      png.data[offset + 3] = 255;
    }
  }

  const stripes: Array<[number, number, number]> = [
    [217, 119, 6],
    [37, 99, 235],
    [22, 163, 74],
    [220, 38, 38],
  ];
  stripes.forEach(([r, g, b], index) => {
    const x0 = 16 + index * 70;
    for (let y = 24; y < 150; y += 1) {
      for (let x = x0; x < x0 + 38; x += 1) {
        const offset = (width * y + x) * 4;
        png.data[offset] = r;
        png.data[offset + 1] = g;
        png.data[offset + 2] = b;
        png.data[offset + 3] = 255;
      }
    }
  });

  return PNG.sync.write(png);
}

function changedBoardLikePng(): Buffer {
  const png = PNG.sync.read(boardLikePng());
  for (let y = 72; y < 96; y += 1) {
    for (let x = 232; x < 270; x += 1) {
      const offset = (png.width * y + x) * 4;
      png.data[offset] = 124;
      png.data[offset + 1] = 58;
      png.data[offset + 2] = 237;
      png.data[offset + 3] = 255;
    }
  }
  return PNG.sync.write(png);
}

test("rendered PNG quality gate rejects blank screenshots", () => {
  const blank = solidPng(320, 180, [255, 255, 255, 255]);
  expect(() => assertRenderedPngQuality(blank, {}, "blank")).toThrow(/failed PNG quality gate/);
});

test("rendered PNG quality gate accepts a visibly colored board-like screenshot", () => {
  const metrics = assertRenderedPngQuality(
    boardLikePng(),
    {
      minWidth: 300,
      minHeight: 180,
      minDistinctColorBuckets: 4,
      maxBackgroundPixelRatio: 0.99,
      minNonBackgroundPixelRatio: 0.01,
      minSaturatedPixels: 120,
      minSaturatedPixelRatio: 0.0005,
    },
    "board-like",
  );

  expect(metrics.saturated_pixels).toBeGreaterThan(120);
  expect(metrics.non_background_pixel_ratio).toBeGreaterThan(0.01);
});

test("rendered PNG delta gate rejects static colorful screenshots", () => {
  const staticColorful = boardLikePng();
  expect(() => assertRenderedPngDelta(staticColorful, staticColorful, {}, "static colorful")).toThrow(
    /failed PNG delta gate/,
  );
});

test("rendered PNG delta gate accepts a visible board state change", () => {
  const metrics = assertRenderedPngDelta(
    boardLikePng(),
    changedBoardLikePng(),
    {
      minChangedPixels: 120,
      minChangedPixelRatio: 0.0005,
    },
    "changed board-like",
  );

  expect(metrics.changed_pixels).toBeGreaterThan(120);
  expect(metrics.changed_pixel_ratio).toBeGreaterThan(0.0005);
});
