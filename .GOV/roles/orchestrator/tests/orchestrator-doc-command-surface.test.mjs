import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const JUSTFILE_PATH = path.resolve("justfile");
const DOC_PATHS = [
  path.resolve(".GOV", "roles", "orchestrator", "ORCHESTRATOR_PROTOCOL.md"),
  path.resolve(".GOV", "roles", "orchestrator", "README.md"),
];

function parseJustRecipes(text) {
  const recipes = new Set();
  const lines = String(text || "").split(/\r?\n/);
  for (const line of lines) {
    if (!line || /^\s/.test(line) || /^\s*#/.test(line)) continue;
    const match = line.match(/^([A-Za-z0-9][A-Za-z0-9-]*)\b/);
    if (!match) continue;
    recipes.add(match[1]);
  }
  return recipes;
}

function extractJustCommands(text) {
  const matches = String(text || "").matchAll(/`just\s+([a-z0-9]+(?:-[a-z0-9]+)*)(?=\s|`)/g);
  return [...new Set([...matches].map((match) => match[1]))].sort();
}

test("orchestrator docs only reference just commands that exist in the live justfile", () => {
  const recipes = parseJustRecipes(fs.readFileSync(JUSTFILE_PATH, "utf8"));
  const missing = [];

  for (const docPath of DOC_PATHS) {
    const commands = extractJustCommands(fs.readFileSync(docPath, "utf8"));
    for (const command of commands) {
      if (!recipes.has(command)) {
        missing.push(`${path.relative(process.cwd(), docPath)} -> just ${command}`);
      }
    }
  }

  assert.deepEqual(missing, []);
});
