import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export type FontWeight =
  | { type: "variable"; min: number; max: number }
  | { type: "fixed"; value: number };

export type FontLicense = {
  spdx: string;
  licenseFile: string | null;
};

export type FontRecord = {
  id: string;
  family: string;
  style: string;
  weight: FontWeight;
  source: "bundled" | "user";
  format: string;
  path: string;
  license: FontLicense;
};

export type FontManifest = {
  schemaVersion: number;
  generatedAt: string;
  packVersion: string;
  fonts: FontRecord[];
};

export type FontsImportResult = {
  imported: string[];
  skipped: string[];
  manifest: FontManifest;
};

const loadedFontIds = new Set<string>();

function sanitizeCssFamily(input: string): string {
  let out = "";
  for (const ch of input) {
    const ok = /[A-Za-z0-9 _-]/.test(ch);
    if (ok) out += ch;
  }
  const trimmed = out.trim();
  return trimmed.length === 0 ? "Unknown" : trimmed;
}

function weightToCss(weight: FontWeight): string {
  return weight.type === "variable" ? `${weight.min} ${weight.max}` : `${weight.value}`;
}

export async function fontsBootstrapPack(): Promise<void> {
  await invoke("fonts_bootstrap_pack");
}

export async function fontsList(): Promise<FontManifest> {
  return (await invoke("fonts_list")) as FontManifest;
}

export async function fontsImport(paths: string[]): Promise<FontsImportResult> {
  return (await invoke("fonts_import", { paths })) as FontsImportResult;
}

export async function fontsRemove(fontId: string): Promise<void> {
  await invoke("fonts_remove", { font_id: fontId });
}

export function canvasDefaultFamilies(): string[] {
  return ["Inter", "Space Grotesk", "JetBrains Mono"];
}

export async function loadFontFaces(fonts: FontRecord[]): Promise<void> {
  const toLoad = fonts.filter((f) => !loadedFontIds.has(f.id));
  if (toLoad.length === 0) return;

  for (const font of toLoad) {
    const family = sanitizeCssFamily(font.family);
    const url = convertFileSrc(font.path);
    const face = new FontFace(family, `url(${url})`, {
      style: font.style,
      weight: weightToCss(font.weight),
    });
    await face.load();
    document.fonts.add(face);
    loadedFontIds.add(font.id);
  }

  await document.fonts.ready;
}

export async function preloadCanvasFonts(): Promise<void> {
  const manifest = await fontsList();
  const defaults = new Set(canvasDefaultFamilies().map((f) => sanitizeCssFamily(f)));
  const fonts = manifest.fonts.filter((f) => defaults.has(sanitizeCssFamily(f.family)));
  await loadFontFaces(fonts);
}

