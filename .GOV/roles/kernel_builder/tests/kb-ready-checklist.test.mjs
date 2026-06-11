import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { resolveProductWorktreeRoot } from "../scripts/kb-ready-checklist.mjs";

const REPO_ROOT = path.resolve(".");
const KB_READY_CHECKLIST = path.join(REPO_ROOT, ".GOV", "roles", "kernel_builder", "scripts", "kb-ready-checklist.mjs");

const PRODUCT_WORKTREE_ROOT_ENV_VAR = "HANDSHAKE_PRODUCT_WORKTREE_ROOT";

function withTempGitRoot(fn) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "kb-ready-checklist-"));
  try {
    return fn(root);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}

test("resolveProductWorktreeRoot prefers env-var override when path exists", () => {
  withTempGitRoot((root) => {
    const productDir = path.join(root, "wtc-fake-001-v1");
    fs.mkdirSync(productDir, { recursive: true });
    const previous = process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR];
    try {
      process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR] = productDir;
      const resolution = resolveProductWorktreeRoot(root, "WP-FAKE-001-Demo-v1");
      assert.equal(resolution.source, "env");
      assert.equal(path.resolve(resolution.root), path.resolve(productDir));
      assert.equal(resolution.env_var, PRODUCT_WORKTREE_ROOT_ENV_VAR);
    } finally {
      if (previous === undefined) delete process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR];
      else process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR] = previous;
    }
  });
});

test("resolveProductWorktreeRoot falls back to repo root when env-var path is missing", () => {
  withTempGitRoot((root) => {
    const previous = process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR];
    try {
      process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR] = path.join(root, "does-not-exist");
      const resolution = resolveProductWorktreeRoot(root, "WP-FAKE-001-Demo-v1");
      assert.equal(resolution.source, "fallback-repo-root");
      assert.equal(path.resolve(resolution.root), path.resolve(root));
      assert.match(resolution.note, /does not exist/i);
    } finally {
      if (previous === undefined) delete process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR];
      else process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR] = previous;
    }
  });
});

test("resolveProductWorktreeRoot falls back to repo root with stem hint when nothing matches", () => {
  withTempGitRoot((root) => {
    // No env var, no git worktree list inside a non-git temp dir; expect
    // fallback with the derived stem surfaced in the note.
    const previous = process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR];
    try {
      delete process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR];
      const resolution = resolveProductWorktreeRoot(root, "WP-KERNEL-004-Sample-v1");
      assert.equal(resolution.source, "fallback-repo-root");
      assert.equal(resolution.matched_stem, "kernel-004");
      assert.match(resolution.note, /wtc-kernel-004/);
      assert.match(resolution.note, /HANDSHAKE_PRODUCT_WORKTREE_ROOT/);
    } finally {
      if (previous !== undefined) process.env[PRODUCT_WORKTREE_ROOT_ENV_VAR] = previous;
    }
  });
});

test("kb-ready-checklist --json resolves cross-worktree product root for WP-KERNEL-004 / MT-046", () => {
  // Integration smoke: run against the live repo state. Verifies the script
  // wires resolveProductWorktreeRoot into the receipt, falls into the
  // git-worktree-list source (or env when the operator has set the env var),
  // and emits real-file auto-findings instead of "file not found at repo root".
  const mtContractPath = path.join(
    REPO_ROOT,
    ".GOV",
    "task_packets",
    "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1",
    "MT-046.json",
  );
  if (!fs.existsSync(mtContractPath)) {
    // Skip if the MT contract is not present in this checkout — keeps the
    // test useful in the gov_kernel worktree without forcing other worktrees
    // to ship the contract.
    return;
  }

  const result = spawnSync(process.execPath, [
    KB_READY_CHECKLIST,
    "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1",
    "MT-046",
    "--json",
  ], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr);
  const skeleton = JSON.parse(result.stdout);
  assert.ok(skeleton.product_worktree_root_resolution, "skeleton must include product_worktree_root_resolution");
  assert.notEqual(skeleton.product_worktree_root_resolution.source, "unknown");

  // RC-002 (no dead code) auto-finding lines should NOT be dominated by
  // "file not found" entries when the resolution succeeded. If we are in the
  // fallback path, accept that the lines mention the env-var hint.
  const rc002 = skeleton.rubric_items.find((it) => it.rubric_item_id === "RC-002-NO-DEAD-CODE");
  assert.ok(rc002, "RC-002 must exist");
  const fileNotFoundLines = (rc002.auto_findings || []).filter((line) => /file not found/i.test(line));
  if (skeleton.product_worktree_root_resolution.source !== "fallback-repo-root") {
    assert.equal(
      fileNotFoundLines.length,
      0,
      `RC-002 still reports file-not-found findings despite ${skeleton.product_worktree_root_resolution.source} resolution: ${fileNotFoundLines.join(" | ")}`,
    );
  }

  // RC-006 invariant: with claimed_by set and completed_by empty, finding
  // should explicitly state INVARIANT OK at the READY_FOR_VALIDATION boundary.
  const rc006 = skeleton.rubric_items.find((it) => it.rubric_item_id === "RC-006-IMPLEMENTER-NOT-SELF-CERTIFYING");
  assert.ok(rc006, "RC-006 must exist");
  const okLines = (rc006.auto_findings || []).filter((line) => /INVARIANT OK/.test(line));
  // MT-046 has claimed_by set and completed_by empty in the live state; if
  // that changes the test should be updated rather than silently dropping
  // the assertion.
  if (okLines.length === 0) {
    // Allow VIOLATION lines if the live contract has changed shape.
    const violationLines = (rc006.auto_findings || []).filter((line) => /VIOLATION/.test(line));
    assert.ok(violationLines.length > 0, `RC-006 must either confirm INVARIANT OK or flag a VIOLATION; got: ${JSON.stringify(rc006.auto_findings)}`);
  }
});
