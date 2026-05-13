import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  readResolvedSpecTextAtRepo,
  resolveGovernanceReferenceFromSpecCurrentAtRepo,
  resolveSpecCurrentAtRepo,
} from "../scripts/lib/spec-current-lib.mjs";

function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

test("resolves machine-readable SPEC_CURRENT through indexed manifest modules", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-spec-current-"));

  try {
    const moduleA = "# Current indexed spec\n\n";
    const moduleB = "Module body from manifest modules.\n";
    const reconstructed = `${moduleA}${moduleB}`;
    const legacySource = "# Legacy source baseline\n\nThis must not be returned as current spec text.\n";

    fs.mkdirSync(path.join(repoRoot, ".GOV", "spec", "indexed_spec", "spec-modules"), { recursive: true });
    fs.mkdirSync(path.join(repoRoot, ".GOV", "codex"), { recursive: true });
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "spec", "indexed_spec", "spec-modules", "00-preamble.md"),
      moduleA,
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "spec", "indexed_spec", "spec-modules", "01-body.md"),
      moduleB,
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "spec", "Handshake_Master_Spec_v02.182.md"),
      legacySource,
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "codex", "Handshake_Codex_v1.4.md"),
      "# Codex\n",
      "utf8",
    );

    writeJson(path.join(repoRoot, ".GOV", "spec", "indexed_spec", "indexed-spec-manifest.json"), {
      artifact_type: "indexed-spec-manifest",
      status: "current-indexed-spec-entrypoint",
      source: {
        path: ".GOV/spec/Handshake_Master_Spec_v02.182.md",
        sha256: sha256(legacySource),
      },
      modules: [
        {
          module_id: "00",
          path: "spec-modules/00-preamble.md",
          sha256: sha256(moduleA),
          source_line_start: 1,
          source_line_end: 2,
        },
        {
          module_id: "01",
          path: "spec-modules/01-body.md",
          sha256: sha256(moduleB),
          source_line_start: 3,
          source_line_end: 3,
        },
      ],
      reconstruction: {
        module_order: ["spec-modules/00-preamble.md", "spec-modules/01-body.md"],
        reconstructed_sha256: sha256(reconstructed),
        byte_exact_match: false,
      },
    });

    writeJson(path.join(repoRoot, ".GOV", "spec", "SPEC_CURRENT.md"), {
      schema: "handshake.spec_current@1",
      updated_at: "2026-05-13T00:00:00.000Z",
      current_spec: {
        entrypoint_type: "indexed_manifest",
        entrypoint_path: ".GOV/spec/indexed_spec/indexed-spec-manifest.json",
        resolver_index_path: ".GOV/spec/indexed_spec/INDEX.json",
        version: "v02.182",
        source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md",
      },
      governance_reference: {
        path: ".GOV/codex/Handshake_Codex_v1.4.md",
      },
    });

    const resolved = resolveSpecCurrentAtRepo(repoRoot);
    assert.equal(resolved.entrypointType, "indexed_manifest");
    assert.equal(resolved.specTargetLabel, ".GOV/spec/indexed_spec/indexed-spec-manifest.json");
    assert.equal(resolved.resolverIndexPath, ".GOV/spec/indexed_spec/INDEX.json");
    assert.equal(resolved.sourceBaselinePath, ".GOV/spec/Handshake_Master_Spec_v02.182.md");
    assert.equal(resolved.versionTag, "v02.182");
    assert.equal(resolved.sha256, sha256(reconstructed));
    assert.equal(readResolvedSpecTextAtRepo(repoRoot, resolved), reconstructed);

    const governanceReference = resolveGovernanceReferenceFromSpecCurrentAtRepo(repoRoot);
    assert.equal(governanceReference.codexFilename, ".GOV/codex/Handshake_Codex_v1.4.md");
    assert.equal(fs.existsSync(governanceReference.codexPathAbs), true);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});
