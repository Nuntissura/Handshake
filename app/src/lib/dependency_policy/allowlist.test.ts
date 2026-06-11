// WP-KERNEL-009 / MT-017 — RuntimeDependencyAllowlist consumption gate.
//
// Proves the allowlist is a live test gate, not a dead JSON file:
//  - the document is structurally valid (and the validator rejects drifted docs),
//  - every forbidden npm package is absent from the real product manifest,
//  - every declared editor-stack dependency in app/package.json is covered by
//    a bundled-library rule (new editor deps fail this gate until declared).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  AllowlistShapeError,
  RUNTIME_DEPENDENCY_ALLOWLIST,
  bundledNpmRuleFor,
  classifyExternalRuntimeInputPath,
  forbiddenClassById,
  forbiddenNpmPackageNames,
  forbiddenCargoCrateSubstrings,
  isEditorStackNpmPackage,
  matchesPackagePattern,
  validateAllowlistDocument,
} from "./allowlist";

const appRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function readAppPackageJson(): {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
} {
  return JSON.parse(readFileSync(join(appRoot, "package.json"), "utf8"));
}

describe("MT-017 runtime dependency allowlist", () => {
  it("loads and validates the committed allowlist document", () => {
    expect(RUNTIME_DEPENDENCY_ALLOWLIST.schema).toBe(
      "handshake.runtime_dependency_allowlist@1",
    );
    expect(RUNTIME_DEPENDENCY_ALLOWLIST.wp_id).toContain("WP-KERNEL-009");
    expect(RUNTIME_DEPENDENCY_ALLOWLIST.allowed_external_runtime_inputs).toHaveLength(4);
    expect(
      RUNTIME_DEPENDENCY_ALLOWLIST.forbidden_runtime_dependency_classes.length,
    ).toBeGreaterThanOrEqual(5);
  });

  it("declares every external runtime input operator-gated and default-off", () => {
    for (const input of RUNTIME_DEPENDENCY_ALLOWLIST.allowed_external_runtime_inputs) {
      expect(input.operator_gated, `${input.kind} must be operator gated`).toBe(true);
      expect(input.default_enabled, `${input.kind} must default off`).toBe(false);
    }
  });

  it("rejects structurally drifted documents (negative cases)", () => {
    expect(() => validateAllowlistDocument(null)).toThrow(AllowlistShapeError);
    expect(() => validateAllowlistDocument({})).toThrow(AllowlistShapeError);
    expect(() =>
      validateAllowlistDocument({
        ...structuredClone(RUNTIME_DEPENDENCY_ALLOWLIST),
        schema: "handshake.runtime_dependency_allowlist@99",
      }),
    ).toThrow(/schema/);
    // A document whose CUI gate defaults open must be rejected.
    const gateOpen = structuredClone(RUNTIME_DEPENDENCY_ALLOWLIST) as unknown as {
      allowed_external_runtime_inputs: Array<{ kind: string; default_enabled: boolean }>;
    };
    const cui = gateOpen.allowed_external_runtime_inputs.find(
      (i) => i.kind === "cui_portable_artifact",
    );
    expect(cui).toBeDefined();
    cui!.default_enabled = true;
    expect(() => validateAllowlistDocument(gateOpen)).toThrow(/default to disabled/);
    // A document missing the sqlite forbidden class must be rejected.
    const noSqlite = structuredClone(RUNTIME_DEPENDENCY_ALLOWLIST) as unknown as {
      forbidden_runtime_dependency_classes: Array<{ id: string }>;
    };
    noSqlite.forbidden_runtime_dependency_classes =
      noSqlite.forbidden_runtime_dependency_classes.filter((c) => c.id !== "sqlite");
    expect(() => validateAllowlistDocument(noSqlite)).toThrow(/sqlite/);
  });

  it("keeps the sqlite and docker forbidden classes scannable", () => {
    expect(forbiddenClassById("sqlite").cargo_crate_name_substrings).toContain("sqlite");
    expect(forbiddenClassById("sqlite").npm_package_names).toContain("better-sqlite3");
    expect(forbiddenClassById("docker_default").source_scan_patterns).toContain(
      "docker run",
    );
    expect(forbiddenClassById("cdn_runtime_asset").source_scan_patterns.length)
      .toBeGreaterThanOrEqual(5);
    expect(forbiddenCargoCrateSubstrings()).toContain("sqlite");
  });

  it("gates the real product manifest: no forbidden npm package is declared", () => {
    const pkg = readAppPackageJson();
    const declared = new Set([
      ...Object.keys(pkg.dependencies ?? {}),
      ...Object.keys(pkg.devDependencies ?? {}),
    ]);
    for (const forbidden of forbiddenNpmPackageNames()) {
      expect(declared.has(forbidden), `forbidden package ${forbidden} declared in app/package.json`).toBe(false);
    }
  });

  it("covers every declared editor-stack dependency with a bundled-library rule", () => {
    const pkg = readAppPackageJson();
    const declared = Object.keys(pkg.dependencies ?? {});
    // Editor-stack families that must be present and covered.
    const editorDeps = declared.filter(
      (name) =>
        name.startsWith("@tiptap/") ||
        name === "monaco-editor" ||
        name === "yjs" ||
        name.startsWith("@xterm/") ||
        name.startsWith("@excalidraw/") ||
        name.startsWith("prosemirror-"),
    );
    expect(editorDeps.length).toBeGreaterThan(0);
    for (const dep of editorDeps) {
      const rule = bundledNpmRuleFor(dep);
      expect(rule, `editor-stack dependency ${dep} is not covered by the allowlist`).not.toBeNull();
      expect(rule!.allowed_licenses.length).toBeGreaterThan(0);
      expect(isEditorStackNpmPackage(dep)).toBe(true);
    }
  });

  it("matches package patterns exactly or by prefix only", () => {
    expect(matchesPackagePattern("@tiptap/core", "@tiptap/*")).toBe(true);
    expect(matchesPackagePattern("@tiptap-evil/core", "@tiptap/*")).toBe(false);
    expect(matchesPackagePattern("monaco-editor", "monaco-editor")).toBe(true);
    expect(matchesPackagePattern("monaco-editor-fake", "monaco-editor")).toBe(false);
  });

  it("classifies operator-provided artifact paths into declared kinds only", () => {
    expect(classifyExternalRuntimeInputPath("C:/models/llama.GGUF")).toBe("model_gguf");
    expect(classifyExternalRuntimeInputPath("/data/weights.safetensors")).toBe(
      "model_safetensors",
    );
    expect(classifyExternalRuntimeInputPath("steer.npz")).toBe("tensor_artifact");
    expect(classifyExternalRuntimeInputPath("bundle.zip")).toBe("cui_portable_artifact");
    expect(classifyExternalRuntimeInputPath("script.exe")).toBeNull();
    expect(classifyExternalRuntimeInputPath("db.sqlite3")).toBeNull();
  });
});

// Adversarial-review hardening (H1/H2/H3): the typed accessor must validate the
// hardened authority blocks with the same strictness as the .mjs loader
// (app/scripts/lib/dependency_policy_scans.mjs), so neither consumer can run on
// a document whose hardening blocks were dropped or malformed.
describe("MT-017 hardened authority blocks (H1/H2/H3 validation parity)", () => {
  it("exposes the hardened esm.sh exception through the typed accessor", () => {
    const esm = (RUNTIME_DEPENDENCY_ALLOWLIST.built_output_scan_exceptions ?? []).find(
      (e) => e.pattern === "esm.sh",
    );
    expect(esm).toBeDefined();
    expect(esm!.forward_only).toBe(true);
    expect(esm!.max_marker_distance).toBeLessThanOrEqual(64);
    expect(esm!.max_total_occurrences).toBe(1);
  });

  it("exposes the H3 exact self-exempt paths and the H2 docker artifact config", () => {
    expect(RUNTIME_DEPENDENCY_ALLOWLIST.scan_self_exempt_paths.paths.length).toBeGreaterThan(0);
    // The real product harness file must NOT be in the exempt list.
    expect(
      RUNTIME_DEPENDENCY_ALLOWLIST.scan_self_exempt_paths.paths.some((p) =>
        p.includes("harness"),
      ),
    ).toBe(false);
    expect(RUNTIME_DEPENDENCY_ALLOWLIST.docker_artifact_scan.filename_globs).toContain(
      "Dockerfile",
    );
    expect(RUNTIME_DEPENDENCY_ALLOWLIST.docker_artifact_scan.filename_globs).toContain(
      "Containerfile",
    );
  });

  it("rejects a document missing scan_self_exempt_paths (H3 cannot be dropped silently)", () => {
    const doc = structuredClone(RUNTIME_DEPENDENCY_ALLOWLIST) as unknown as Record<
      string,
      unknown
    >;
    delete doc.scan_self_exempt_paths;
    expect(() => validateAllowlistDocument(doc)).toThrow(/scan_self_exempt_paths/);
  });

  it("rejects a document missing docker_artifact_scan (H2 cannot be dropped silently)", () => {
    const doc = structuredClone(RUNTIME_DEPENDENCY_ALLOWLIST) as unknown as Record<
      string,
      unknown
    >;
    delete doc.docker_artifact_scan;
    expect(() => validateAllowlistDocument(doc)).toThrow(/docker_artifact_scan/);
  });

  it("rejects malformed hardening fields (empty paths; bad occurrence cap)", () => {
    const emptyPaths = structuredClone(RUNTIME_DEPENDENCY_ALLOWLIST) as unknown as {
      scan_self_exempt_paths: { paths: unknown };
    };
    emptyPaths.scan_self_exempt_paths.paths = [];
    expect(() => validateAllowlistDocument(emptyPaths)).toThrow(/non-empty array/);

    const badCap = structuredClone(RUNTIME_DEPENDENCY_ALLOWLIST) as unknown as {
      built_output_scan_exceptions: Array<{ max_total_occurrences?: unknown }>;
    };
    badCap.built_output_scan_exceptions[0].max_total_occurrences = 0;
    expect(() => validateAllowlistDocument(badCap)).toThrow(/max_total_occurrences/);
  });
});
