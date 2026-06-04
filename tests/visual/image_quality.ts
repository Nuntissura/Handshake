const PixelmatchModule = require("pixelmatch");
const { PNG } = require("pngjs");

const pixelmatch = PixelmatchModule.default ?? PixelmatchModule;

export type RenderedPngQualityMetrics = {
  width: number;
  height: number;
  total_pixels: number;
  distinct_color_buckets: number;
  background_pixel_ratio: number;
  non_background_pixel_ratio: number;
  saturated_pixels: number;
  saturated_pixel_ratio: number;
};

export type RenderedPngDeltaMetrics = {
  before_width: number;
  before_height: number;
  after_width: number;
  after_height: number;
  compared_pixels: number;
  changed_pixels: number;
  changed_pixel_ratio: number;
  changed_pixel_percent: number;
};

export type RenderedPngQualityThresholds = {
  minWidth?: number;
  minHeight?: number;
  minDistinctColorBuckets?: number;
  maxBackgroundPixelRatio?: number;
  minNonBackgroundPixelRatio?: number;
  minSaturatedPixels?: number;
  minSaturatedPixelRatio?: number;
};

export type RenderedPngDeltaThresholds = {
  minChangedPixels?: number;
  minChangedPixelRatio?: number;
};

type DecodedPng = {
  width: number;
  height: number;
  data: Uint8Array;
};

const DEFAULT_THRESHOLDS: Required<RenderedPngQualityThresholds> = {
  minWidth: 1,
  minHeight: 1,
  minDistinctColorBuckets: 16,
  maxBackgroundPixelRatio: 0.995,
  minNonBackgroundPixelRatio: 0.006,
  minSaturatedPixels: 80,
  minSaturatedPixelRatio: 0.0003,
};

const DEFAULT_DELTA_THRESHOLDS: Required<RenderedPngDeltaThresholds> = {
  minChangedPixels: 1,
  minChangedPixelRatio: 0.000001,
};

export function inspectRenderedPngQuality(bytes: Buffer | Uint8Array): RenderedPngQualityMetrics {
  const png = PNG.sync.read(Buffer.from(bytes)) as DecodedPng;
  const totalPixels = png.width * png.height;
  if (totalPixels <= 0) {
    return {
      width: png.width,
      height: png.height,
      total_pixels: 0,
      distinct_color_buckets: 0,
      background_pixel_ratio: 1,
      non_background_pixel_ratio: 0,
      saturated_pixels: 0,
      saturated_pixel_ratio: 0,
    };
  }

  const bg = pixelAt(png, 0, 0);
  const buckets = new Set<string>();
  let backgroundPixels = 0;
  let saturatedPixels = 0;

  for (let y = 0; y < png.height; y += 1) {
    for (let x = 0; x < png.width; x += 1) {
      const pixel = pixelAt(png, x, y);
      buckets.add(colorBucket(pixel));
      if (colorDistance(pixel, bg) <= 8) backgroundPixels += 1;
      if (isSaturatedVisiblePixel(pixel)) saturatedPixels += 1;
    }
  }

  const backgroundRatio = backgroundPixels / totalPixels;
  const saturatedRatio = saturatedPixels / totalPixels;
  return {
    width: png.width,
    height: png.height,
    total_pixels: totalPixels,
    distinct_color_buckets: buckets.size,
    background_pixel_ratio: Number(backgroundRatio.toFixed(6)),
    non_background_pixel_ratio: Number((1 - backgroundRatio).toFixed(6)),
    saturated_pixels: saturatedPixels,
    saturated_pixel_ratio: Number(saturatedRatio.toFixed(6)),
  };
}

export function assertRenderedPngQuality(
  bytes: Buffer | Uint8Array,
  thresholds: RenderedPngQualityThresholds = {},
  label = "rendered screenshot",
): RenderedPngQualityMetrics {
  const merged = { ...DEFAULT_THRESHOLDS, ...thresholds };
  const metrics = inspectRenderedPngQuality(bytes);
  const failures: string[] = [];

  if (metrics.width < merged.minWidth) {
    failures.push(`width ${metrics.width} < ${merged.minWidth}`);
  }
  if (metrics.height < merged.minHeight) {
    failures.push(`height ${metrics.height} < ${merged.minHeight}`);
  }
  if (metrics.distinct_color_buckets < merged.minDistinctColorBuckets) {
    failures.push(`distinct_color_buckets ${metrics.distinct_color_buckets} < ${merged.minDistinctColorBuckets}`);
  }
  if (metrics.background_pixel_ratio > merged.maxBackgroundPixelRatio) {
    failures.push(`background_pixel_ratio ${metrics.background_pixel_ratio} > ${merged.maxBackgroundPixelRatio}`);
  }
  if (metrics.non_background_pixel_ratio < merged.minNonBackgroundPixelRatio) {
    failures.push(`non_background_pixel_ratio ${metrics.non_background_pixel_ratio} < ${merged.minNonBackgroundPixelRatio}`);
  }
  if (metrics.saturated_pixels < merged.minSaturatedPixels) {
    failures.push(`saturated_pixels ${metrics.saturated_pixels} < ${merged.minSaturatedPixels}`);
  }
  if (metrics.saturated_pixel_ratio < merged.minSaturatedPixelRatio) {
    failures.push(`saturated_pixel_ratio ${metrics.saturated_pixel_ratio} < ${merged.minSaturatedPixelRatio}`);
  }

  if (failures.length > 0) {
    throw new Error(`${label} failed PNG quality gate: ${failures.join("; ")}; metrics=${JSON.stringify(metrics)}`);
  }
  return metrics;
}

export function inspectRenderedPngDelta(beforeBytes: Buffer | Uint8Array, afterBytes: Buffer | Uint8Array): RenderedPngDeltaMetrics {
  const before = PNG.sync.read(Buffer.from(beforeBytes)) as DecodedPng;
  const after = PNG.sync.read(Buffer.from(afterBytes)) as DecodedPng;

  if (before.width !== after.width || before.height !== after.height) {
    const comparedPixels = Math.max(before.width * before.height, after.width * after.height);
    return {
      before_width: before.width,
      before_height: before.height,
      after_width: after.width,
      after_height: after.height,
      compared_pixels: comparedPixels,
      changed_pixels: comparedPixels,
      changed_pixel_ratio: comparedPixels === 0 ? 0 : 1,
      changed_pixel_percent: comparedPixels === 0 ? 0 : 100,
    };
  }

  const comparedPixels = before.width * before.height;
  const diff = new PNG({ width: before.width, height: before.height });
  const changedPixels = pixelmatch(
    before.data,
    after.data,
    diff.data,
    before.width,
    before.height,
    { threshold: 0.1 },
  );
  const changedRatio = comparedPixels === 0 ? 0 : changedPixels / comparedPixels;
  return {
    before_width: before.width,
    before_height: before.height,
    after_width: after.width,
    after_height: after.height,
    compared_pixels: comparedPixels,
    changed_pixels: changedPixels,
    changed_pixel_ratio: Number(changedRatio.toFixed(6)),
    changed_pixel_percent: Number((changedRatio * 100).toFixed(6)),
  };
}

export function assertRenderedPngDelta(
  beforeBytes: Buffer | Uint8Array,
  afterBytes: Buffer | Uint8Array,
  thresholds: RenderedPngDeltaThresholds = {},
  label = "rendered screenshot pair",
): RenderedPngDeltaMetrics {
  const merged = { ...DEFAULT_DELTA_THRESHOLDS, ...thresholds };
  const metrics = inspectRenderedPngDelta(beforeBytes, afterBytes);
  const failures: string[] = [];

  if (metrics.changed_pixels < merged.minChangedPixels) {
    failures.push(`changed_pixels ${metrics.changed_pixels} < ${merged.minChangedPixels}`);
  }
  if (metrics.changed_pixel_ratio < merged.minChangedPixelRatio) {
    failures.push(`changed_pixel_ratio ${metrics.changed_pixel_ratio} < ${merged.minChangedPixelRatio}`);
  }

  if (failures.length > 0) {
    throw new Error(`${label} failed PNG delta gate: ${failures.join("; ")}; metrics=${JSON.stringify(metrics)}`);
  }
  return metrics;
}

function pixelAt(png: DecodedPng, x: number, y: number): [number, number, number, number] {
  const i = (png.width * y + x) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2], png.data[i + 3]];
}

function colorBucket([r, g, b, a]: [number, number, number, number]): string {
  return `${r >> 3}:${g >> 3}:${b >> 3}:${a >> 5}`;
}

function colorDistance(
  [r1, g1, b1, a1]: [number, number, number, number],
  [r2, g2, b2, a2]: [number, number, number, number],
): number {
  return Math.abs(r1 - r2) + Math.abs(g1 - g2) + Math.abs(b1 - b2) + Math.abs(a1 - a2);
}

function isSaturatedVisiblePixel([r, g, b, a]: [number, number, number, number]): boolean {
  if (a < 16) return false;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  return max - min >= 28 && max < 252;
}
