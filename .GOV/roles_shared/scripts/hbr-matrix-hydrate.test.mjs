import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  hydratePacketAcceptanceMatrix,
  hydratePacketFile,
} from "./hbr-matrix-hydrate.mjs";

const scriptsDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptsDir, "..", "..", "..");
const scriptPath = path.join(scriptsDir, "hbr-matrix-hydrate.mjs");
const fixedAddedAtUtc = "2026-05-18T12:00:00.000Z";

function basePacket(overrides = {}) {
  return {
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-HBR-MATRIX-HYDRATE-TEST-v1",
    scope: {
      allowed_paths: [],
      ...(overrides.scope || {}),
    },
    hbr: {
      tags_declared: ["model_invocation", "automation_surface"],
      ...(overrides.hbr || {}),
    },
    ...Object.fromEntries(
      Object.entries(overrides).filter(([key]) => key !== "scope" && key !== "hbr"),
    ),
  };
}

function ids(rows) {
  return new Set(rows.map((row) => row.hbr_id));
}

function writeTempPacket(packet) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "hbr-matrix-hydrate-"));
  const packetPath = path.join(dir, "packet.json");
  fs.writeFileSync(packetPath, `${JSON.stringify(packet, null, 2)}\n`, "utf8");
  return { dir, packetPath };
}

test("hydrates applicable HBR rows and records not applicable reasons", () => {
  const hydrated = hydratePacketAcceptanceMatrix(basePacket(), { addedAtUtc: fixedAddedAtUtc });

  assert.equal(hydrated.acceptance_matrix.schema_version, 1);
  const hbrIds = ids(hydrated.acceptance_matrix.hbr);
  assert(hbrIds.has("HBR-INT-002"), "model invocation rule should be applicable");
  assert(hbrIds.has("HBR-QUIET-001"), "automation work should derive quiet-run applicability");
  assert(hbrIds.has("HBR-QUIET-002"), "automation surface rule should be applicable");

  const int002 = hydrated.acceptance_matrix.hbr.find((row) => row.hbr_id === "HBR-INT-002");
  assert.deepEqual(int002, {
    hbr_id: "HBR-INT-002",
    status: "PENDING",
    evidence_pointer: null,
    validator_verdict: null,
    added_at_utc: fixedAddedAtUtc,
  });

  const notApplicable = hydrated.acceptance_matrix.hbr_not_applicable.find((row) =>
    row.hbr_id === "HBR-INT-004"
  );
  assert.equal(notApplicable.source, "applicability_evaluator");
  assert.match(notApplicable.reason, /No declared tags or touched paths matched HBR-INT-004/);
});

test("hydratePacketFile is idempotent and preserves existing PROVED rows", () => {
  const { dir, packetPath } = writeTempPacket(basePacket({
    acceptance_matrix: {
      schema_version: 1,
      hbr: [
        {
          hbr_id: "HBR-INT-002",
          status: "PROVED",
          evidence_pointer: "receipts://hbr-int-002",
          validator_verdict: "PROVED",
          added_at_utc: "2026-05-18T11:00:00.000Z",
        },
      ],
      hbr_not_applicable: [],
    },
  }));

  try {
    hydratePacketFile(packetPath, { addedAtUtc: fixedAddedAtUtc });
    const firstBytes = fs.readFileSync(packetPath, "utf8");
    hydratePacketFile(packetPath, { addedAtUtc: fixedAddedAtUtc });
    const secondBytes = fs.readFileSync(packetPath, "utf8");
    assert.equal(secondBytes, firstBytes);

    const hydrated = JSON.parse(secondBytes);
    const int002 = hydrated.acceptance_matrix.hbr.find((row) => row.hbr_id === "HBR-INT-002");
    assert.equal(int002.status, "PROVED");
    assert.equal(int002.evidence_pointer, "receipts://hbr-int-002");
    assert.equal(int002.validator_verdict, "PROVED");
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("hydrator fails closed on a PROVED row with a non-PROVED validator_verdict", () => {
  // MT-002 finding #1 negative test: a PROVED status that does not carry
  // validator_verdict="PROVED" is corruption (or an in-flight downgrade) and
  // must error rather than be laundered through hydration.
  assert.throws(() => hydratePacketAcceptanceMatrix(basePacket({
    acceptance_matrix: {
      schema_version: 1,
      hbr: [
        {
          hbr_id: "HBR-INT-002",
          status: "PROVED",
          evidence_pointer: "receipts://hbr-int-002",
          validator_verdict: "PENDING",
        },
      ],
      hbr_not_applicable: [],
    },
  }), { addedAtUtc: fixedAddedAtUtc }), /refusing to hydrate HBR-INT-002/);
});

test("hydrator fails closed on a PROVED row with an empty evidence_pointer", () => {
  assert.throws(() => hydratePacketAcceptanceMatrix(basePacket({
    acceptance_matrix: {
      schema_version: 1,
      hbr: [
        {
          hbr_id: "HBR-INT-002",
          status: "PROVED",
          evidence_pointer: "",
          validator_verdict: "PROVED",
        },
      ],
      hbr_not_applicable: [],
    },
  }), { addedAtUtc: fixedAddedAtUtc }), /non-empty evidence_pointer/);
});

test("operator not-applicable overrides require a non-empty reason", () => {
  assert.throws(() => hydratePacketAcceptanceMatrix(basePacket({
    hbr: {
      tags_declared: ["model_invocation"],
      not_applicable_overrides: [{ hbr_id: "HBR-INT-002", reason: " " }],
    },
  }), { addedAtUtc: fixedAddedAtUtc }), /non-empty reason/);
});

test("tagless packets hydrate from allowed path declarations", () => {
  const hydrated = hydratePacketAcceptanceMatrix({
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-HBR-MATRIX-TAGLESS-v1",
    scope: {
      allowed_paths: ["src/backend/handshake_core/src/model_runtime/**"],
    },
  }, { addedAtUtc: fixedAddedAtUtc });

  assert(ids(hydrated.acceptance_matrix.hbr).has("HBR-INT-002"));
});

test("tagless automation paths derive quiet-run applicability", () => {
  const hydrated = hydratePacketAcceptanceMatrix({
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-HBR-MATRIX-AUTOMATION-PATH-v1",
    scope: {
      allowed_paths: ["app/src-tauri/src/**"],
    },
  }, { addedAtUtc: fixedAddedAtUtc });

  const hbrIds = ids(hydrated.acceptance_matrix.hbr);
  assert(hbrIds.has("HBR-QUIET-001"));
  assert(hbrIds.has("HBR-QUIET-002"));
});

test("requires_foreground true derives HBR-QUIET-004 applicability", () => {
  const hydrated = hydratePacketAcceptanceMatrix({
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-HBR-MATRIX-FOREGROUND-v1",
    requires_foreground: true,
    scope: {
      allowed_paths: [],
    },
  }, { addedAtUtc: fixedAddedAtUtc });

  assert(ids(hydrated.acceptance_matrix.hbr).has("HBR-QUIET-004"));
});

test("foreground_required tag keeps HBR-QUIET-004 applicable for matrix gate enforcement", () => {
  const hydrated = hydratePacketAcceptanceMatrix({
    schema_id: "hsk.work_packet_contract@1",
    wp_id: "WP-HBR-MATRIX-FOREGROUND-TAG-v1",
    requires_foreground: false,
    hbr: {
      tags_declared: ["foreground_required"],
    },
  }, { addedAtUtc: fixedAddedAtUtc });

  assert(ids(hydrated.acceptance_matrix.hbr).has("HBR-QUIET-004"));
});

test("dry-run CLI prints hydrated JSON without mutating packet file", () => {
  const { dir, packetPath } = writeTempPacket(basePacket());

  try {
    const before = fs.readFileSync(packetPath, "utf8");
    const stdout = execFileSync(process.execPath, [
      scriptPath,
      "--packet",
      packetPath,
      "--dry-run",
      "--added-at-utc",
      fixedAddedAtUtc,
    ], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });

    assert.equal(fs.readFileSync(packetPath, "utf8"), before);
    const hydrated = JSON.parse(stdout);
    assert(ids(hydrated.acceptance_matrix.hbr).has("HBR-QUIET-002"));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
