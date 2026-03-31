import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";

function resolveRepoRoot() {
  const fileRelativeRepoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  try {
    const out = execFileSync("git", ["-C", fileRelativeRepoRoot, "rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore and fall back to file-relative resolution.
  }
  return fileRelativeRepoRoot;
}

function parseVersion(fileName) {
  const match = String(fileName || "").match(/Handshake_Master_Spec_v(\d+)\.(\d+)\.md/);
  if (!match) return null;
  return { major: Number(match[1]), minor: Number(match[2]) };
}

function compareVersions(a, b) {
  if (a.major !== b.major) return a.major - b.major;
  return a.minor - b.minor;
}

function findLatestSpec(repoRoot) {
  const specDir = path.join(repoRoot, GOV_ROOT_REPO_REL, "spec");
  const files = fs.readdirSync(specDir).filter((name) => /^Handshake_Master_Spec_v\d+\.\d+\.md$/.test(name));
  const parsed = files
    .map((name) => ({ name, version: parseVersion(name) }))
    .filter((entry) => entry.version)
    .sort((a, b) => compareVersions(a.version, b.version));
  if (parsed.length === 0) {
    throw new Error(`No Handshake_Master_Spec_v*.md files found in ${GOV_ROOT_REPO_REL}/spec`);
  }
  return {
    ...parsed[parsed.length - 1],
    specDir,
  };
}

function main() {
  const repoRoot = resolveRepoRoot();
  process.chdir(repoRoot);

  const latest = findLatestSpec(repoRoot);
  const latestTag = `v${String(latest.version.major).padStart(2, "0")}.${String(latest.version.minor).padStart(3, "0")}`;
  const grandfatherCutoff = { major: 2, minor: 141 };

  if (compareVersions(latest.version, grandfatherCutoff) <= 0) {
    console.log(`spec-growth-discipline-check ok: grandfathered ${latest.name}`);
    return;
  }

  const specPath = path.join(latest.specDir, latest.name);
  const specText = fs.readFileSync(specPath, "utf8");
  const currentAddMarker = `[ADD ${latestTag}]`;
  const appendicesStart = [
    specText.indexOf("<!-- HS_APPENDIX:BEGIN id=HS-APPX-FEATURE-REGISTRY"),
    specText.indexOf("<!-- HS_APPENDIX:BEGIN id=HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX"),
    specText.indexOf("<!-- HS_APPENDIX:BEGIN id=HS-APPX-INTERACTION-MATRIX"),
  ].filter((idx) => idx >= 0).sort((a, b) => a - b)[0] ?? specText.length;
  const roadmapStart = specText.indexOf("### 7.6");
  const mainBodySlice = specText.slice(0, appendicesStart);
  const roadmapSlice = roadmapStart >= 0 ? specText.slice(roadmapStart, appendicesStart) : "";
  const appendixSlice = specText.slice(appendicesStart);

  const errors = [];

  if (/^#{1,6}\s+.*addendum/i.test(specText)) {
    errors.push(`${latest.name} contains addendum-style headings; patch canonical sections in place instead.`);
  }
  if (roadmapSlice.includes(currentAddMarker) && !mainBodySlice.includes(currentAddMarker)) {
    errors.push(`Roadmap contains ${currentAddMarker} but Main Body does not; Main Body must lead roadmap growth.`);
  }
  if (appendixSlice.includes(currentAddMarker) && !mainBodySlice.includes(currentAddMarker)) {
    errors.push(`Appendix content contains ${currentAddMarker} but Main Body does not; Main Body must lead appendix growth.`);
  }

  if (roadmapSlice.includes(currentAddMarker)) {
    const requiredFields = [
      { label: "Goal", re: /\*\*Goal\*\*|^- Goal:/mi },
      { label: "MUST deliver", re: /\*\*MUST deliver\*\*|^- MUST deliver:/mi },
      { label: "Key risks addressed in Phase n", re: /\*\*Key risks addressed in Phase\s+\d+\*\*|^- Key risks addressed in Phase n:/mi },
      { label: "Acceptance criteria", re: /\*\*Acceptance criteria\*\*|^- Acceptance criteria:/mi },
      { label: "Explicitly OUT of scope", re: /\*\*Explicitly OUT of scope\*\*|^- Explicitly OUT of scope:/mi },
      { label: "Mechanical Track", re: /\*\*Mechanical Track(?:\s+\(Phase \d+\))?\*\*|^- Mechanical Track:/mi },
      { label: "Atelier Track", re: /\*\*Atelier Track(?:\s+\(Phase \d+\))?\*\*|^- Atelier Track:/mi },
      { label: "Distillation Track", re: /\*\*Distillation Track(?:\s+\(Phase \d+\))?\*\*|^- Distillation Track:/mi },
      { label: "Vertical slice", re: /\*\*Vertical slice\*\*|^- Vertical slice:/mi },
    ];
    for (const field of requiredFields) {
      if (!field.re.test(roadmapSlice)) {
        errors.push(`Roadmap touched by ${currentAddMarker} is missing fixed field: ${field.label}`);
      }
    }
  }

  if (errors.length > 0) {
    for (const error of errors) console.error(`FAIL: ${error}`);
    process.exit(1);
  }

  console.log(`spec-growth-discipline-check ok: ${latest.name}`);
}

main();
