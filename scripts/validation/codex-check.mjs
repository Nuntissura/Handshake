import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

function fail(message, details = "") {
  console.error(message);
  if (details) {
    console.error(details);
  }
  process.exit(1);
}

function listFilesRecursive(rootDir) {
  const out = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const current = stack.pop();
    if (!current) continue;
    let entries;
    try {
      entries = fs.readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (entry.isFile()) {
        out.push(full);
      }
    }
  }
  return out;
}

function toPosix(p) {
  return p.split(path.sep).join("/");
}

function findLineHits(filePath, predicate) {
  let content;
  try {
    content = fs.readFileSync(filePath, "utf8");
  } catch {
    return [];
  }
  const lines = content.split(/\r?\n/);
  const hits = [];
  const relPath = toPosix(path.relative(repoRoot, filePath));
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (predicate(line)) {
      hits.push(`${relPath}:${i + 1}:${line}`);
    }
  }
  return hits;
}

process.chdir(repoRoot);

// 1) Spec drift guard: SPEC_CURRENT must point to the latest master spec.
await import("../spec-current-check.mjs");

// 2) Frontend fetch guard: only the shared API client may call fetch.
{
  const tsFiles = listFilesRecursive(path.join(repoRoot, "app", "src")).filter((filePath) => {
    const ext = path.extname(filePath);
    return ext === ".ts" || ext === ".tsx";
  });
  const hits = tsFiles.flatMap((filePath) =>
    findLineHits(filePath, (line) => /\bfetch\s*\(/.test(line))
  );
  const violations = hits.filter((hit) => !hit.startsWith("app/src/lib/api.ts:"));
  if (violations.length > 0) {
    fail("Forbidden fetch() usage outside API client:", violations.join("\n"));
  }
}

// 3) Backend println/eprintln guard: disallow direct stdout logging in production code.
{
  const rustFiles = listFilesRecursive(path.join(repoRoot, "src", "backend", "handshake_core", "src")).filter(
    (filePath) => path.extname(filePath) === ".rs"
  );
  const hits = rustFiles.flatMap((filePath) =>
    findLineHits(filePath, (line) => line.includes("println!") || line.includes("eprintln!"))
  );
  if (hits.length > 0) {
    fail("Forbidden println!/eprintln! in backend source:", hits.join("\n"));
  }
}

// 4) TODO tagging guard: TODOs must be annotated with HSK issue IDs.
{
  const roots = [
    path.join(repoRoot, "src", "backend", "handshake_core", "src"),
    path.join(repoRoot, "app", "src")
  ];
  const files = roots
    .flatMap((root) => listFilesRecursive(root))
    .filter((filePath) => [".rs", ".ts", ".tsx"].includes(path.extname(filePath)));

  const hits = files.flatMap((filePath) => findLineHits(filePath, (line) => line.includes("TODO")));
  const violations = hits.filter((hit) => !/TODO\(HSK-\d+\)/.test(hit));
  if (violations.length > 0) {
    fail("Untracked TODOs found (require TODO(HSK-####)):", violations.join("\n"));
  }
}

// 5) Task board guard: keep Done/Superseded minimal and machine-checkable.
await import("./task-board-check.mjs");
await import("./task-packet-claim-check.mjs");

console.log("codex-check ok");
