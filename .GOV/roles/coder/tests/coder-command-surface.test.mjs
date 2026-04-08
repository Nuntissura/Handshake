import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const JUSTFILE_PATH = path.resolve("justfile");

function recipeExists(text, recipeName) {
  const escaped = recipeName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`^${escaped}(?:\\s|:|$)`, "m").test(text);
}

test("justfile exposes the live coder command surface referenced by coder docs and helpers", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");
  const requiredRecipes = [
    "phase-check",
    "coder-skeleton-checkpoint",
    "skeleton-approved",
    "backup-push",
    "validator-scan",
    "product-scan",
    "validator-dal-audit",
    "validator-git-hygiene",
    "cargo-clean",
    "spec-debt-open",
    "spec-debt-sync",
  ];

  for (const recipeName of requiredRecipes) {
    assert.equal(recipeExists(justfile, recipeName), true, `Missing just recipe: ${recipeName}`);
  }
});
