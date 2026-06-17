// WP-KERNEL-009 / MT-263 — parity-proof tamper guard.
//
// Contract acceptance: "Tampering check: removing a proof makes the suite fail
// (proven)." This spec proves the SINGLE verdict function the runner uses
// (resolveVerdict) cannot mint a false PASS / DESCOPED, AND proves the relocated
// (backed) proofs are tamper-evident: removing or emptying the named real-
// backend backing spec flips the row to FAIL. It covers the THREE proof paths:
//
//   - inline / harness: a REMOVED / THROWING per-row proof resolves to FAIL,
//     never PASS — a row can never be marked passing without an executed,
//     non-throwing per-row proof.
//   - backed: a known DEC is NOT sufficient. The named backing spec must exist
//     AND be non-trivial. A missing / emptied backing spec resolves to FAIL —
//     so deleting the relocated proof turns the gate red (the central
//     anti-shortcut guarantee for the rows the offline lane cannot execute).
//   - descoped: a row citing an UNKNOWN DEC resolves to FAIL, never DESCOPED —
//     a row can never be descoped without a real operator decision.
//
// It then re-runs the REAL registry against the REAL packet DEC set AND the REAL
// filesystem and asserts (a) every descoped/backed row cites a known DEC and
// (b) every backed row's backing spec actually exists and is non-trivial — so a
// future edit that introduces a bogus descope, a relocated proof to a deleted
// spec, or a stub backing file is caught here too.

import { expect, test } from "@playwright/test";
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { PARITY_ROWS, resolveVerdict } from "./parity/wp009_parity_registry";

const repoRoot = path.resolve(__dirname, "..", "..");
const packetPath = path.join(
  repoRoot,
  ".GOV",
  "task_packets",
  "WP-KERNEL-009-Project-Knowledge-Index-Loom-Rich-Editor-v1",
  "packet.json",
);

// Mirror the runner's backing-spec gate (see wp009_parity_proof_suite.spec.ts).
const MIN_BACKING_SPEC_BYTES = 512;

function knownDecs(): Set<string> {
  const packet = JSON.parse(readFileSync(packetPath, "utf8")) as {
    lifecycle?: { operator_decisions?: Array<{ id: string }> };
    operator_decisions?: Array<{ id: string }>;
  };
  const decisions = packet.lifecycle?.operator_decisions ?? packet.operator_decisions ?? [];
  return new Set(decisions.map((d) => d.id));
}

function backingSpecBytes(spec: string): number {
  const abs = path.join(repoRoot, ...spec.split("/"));
  if (!existsSync(abs) || !statSync(abs).isFile()) return 0;
  return statSync(abs).size;
}

test.describe("WP-KERNEL-009 MT-263 parity-proof tamper guard", () => {
  test("a removed/throwing inline or harness proof resolves to FAIL, never PASS", () => {
    // Simulate the proof being removed (it throws when invoked) — the runner
    // catches the throw and passes the error string to resolveVerdict.
    expect(resolveVerdict({ kind: "inline", inlineError: "proof removed" })).toBe("FAIL");
    expect(resolveVerdict({ kind: "harness", inlineError: "proof removed" })).toBe("FAIL");

    // A genuinely-executed, non-throwing proof is the ONLY path to PASS.
    expect(resolveVerdict({ kind: "inline", inlineError: null })).toBe("PASS");
    expect(resolveVerdict({ kind: "harness", inlineError: null })).toBe("PASS");
  });

  test("a backed row whose backing spec is removed/emptied resolves to FAIL, never DESCOPED", () => {
    // Known DEC alone is NOT enough — the relocated proof must still exist.
    expect(resolveVerdict({ kind: "backed", decKnown: true, specOk: false })).toBe("FAIL");
    // Both conditions met -> DESCOPED (proven in the real-backend lane).
    expect(resolveVerdict({ kind: "backed", decKnown: true, specOk: true })).toBe("DESCOPED");
    // A bogus DEC also fails even if the spec exists.
    expect(resolveVerdict({ kind: "backed", decKnown: false, specOk: true })).toBe("FAIL");
  });

  test("a descope citing an unknown DEC resolves to FAIL, never DESCOPED", () => {
    expect(resolveVerdict({ kind: "descoped", decKnown: false })).toBe("FAIL");
    expect(resolveVerdict({ kind: "descoped", decKnown: true })).toBe("DESCOPED");
  });

  test("every descoped AND backed row in the shipped registry cites a REAL operator DEC", () => {
    const decs = knownDecs();
    expect(decs.size, "no operator decisions found in packet.json").toBeGreaterThan(0);
    const offenders = PARITY_ROWS.filter(
      (r) => (r.proof.kind === "descoped" || r.proof.kind === "backed") && !decs.has(r.proof.dec),
    ).map((r) => ({ id: r.id, dec: (r.proof as { dec: string }).dec }));
    expect(offenders, `rows citing unknown DECs: ${JSON.stringify(offenders)}`).toEqual([]);
  });

  test("every backed row's relocated backing spec exists on disk and is non-trivial", () => {
    // This is the real tamper guarantee for the rows the offline lane cannot
    // execute: if any cited backing spec were deleted or stubbed, this fails.
    const offenders = PARITY_ROWS.filter((r) => r.proof.kind === "backed")
      .map((r) => ({ id: r.id, spec: (r.proof as { spec: string }).spec }))
      .map((r) => ({ ...r, bytes: backingSpecBytes(r.spec) }))
      .filter((r) => r.bytes < MIN_BACKING_SPEC_BYTES);
    expect(
      offenders,
      `backed rows whose relocated proof is missing/trivial: ${JSON.stringify(offenders)}`,
    ).toEqual([]);
    // Sanity: there ARE backed rows (the guard isn't vacuously satisfied).
    expect(PARITY_ROWS.filter((r) => r.proof.kind === "backed").length).toBeGreaterThan(0);
  });
});
