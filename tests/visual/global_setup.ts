import { ensureProbeBinary } from "./focus_audit_driver";

/**
 * MT-027 remediation (defect 1) — Playwright globalSetup.
 *
 * The a2 visual smoke's headless path drives the REAL foreground focus audit by
 * shelling out to the `focus-audit-probe` Rust binary. Before this setup,
 * nothing built that binary as a prerequisite, so against a clean/fresh cargo
 * target dir the smoke hard-FAILED with "focus-audit-probe binary not found"
 * (or flaked mid-rebuild when the binary was momentarily unavailable). That made
 * the "real audit" guarantee non-hermetic.
 *
 * This globalSetup guarantees the real probe exists before any visual spec runs
 * by building it on demand (`cargo build --bin focus-audit-probe`). It is a
 * no-op (just a path resolution) when the binary is already present, so warm
 * runs pay no build cost. `runViaProbe` also calls `ensureProbeBinary` as a
 * belt-and-suspenders fallback, so the headless path can never silently skip or
 * hard-fail on a clean target even if this setup is bypassed.
 */
export default function globalSetup(): void {
  const probe = ensureProbeBinary();
  // Surface the resolved/built path so a no-context runner can see the real
  // binary the smoke will exercise.
  // eslint-disable-next-line no-console
  console.log(`[focus-audit] probe binary ready: ${probe}`);
}
