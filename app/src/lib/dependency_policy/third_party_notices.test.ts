// WP-KERNEL-009 / MT-029 — LicenseAndNoticeReceipts sync test.
//
// The committed notices receipt app/src-tauri/resources/THIRD_PARTY_NOTICES.json
// must stay in lock-step with the pnpm lockfile and the cargo metadata graph:
// this test REGENERATES the document via the real generator script
// (app/scripts/generate-third-party-notices.mjs --check, which performs a
// byte-for-byte diff) and fails on any drift. It then pins the receipt's
// structural guarantees: every allowlist bundled-library family is present,
// every license is in its family's allowed set, and the registries are the
// two canonical ones. The file ships inside the product bundle
// (tauri.conf.json bundle.resources includes "resources/").
//
// Regenerate after dependency changes: pnpm run generate:third-party-notices

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { RUNTIME_DEPENDENCY_ALLOWLIST } from "./allowlist";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const noticesPath = join(appDir, "src-tauri", "resources", "THIRD_PARTY_NOTICES.json");

interface Notice {
  name: string;
  version: string;
  license: string;
  license_evidence: string;
  registry: string;
  ecosystem: "npm" | "cargo";
  family: string;
}

interface NoticesDocument {
  schema: string;
  mt_id: string;
  notice_count: number;
  notices: Notice[];
}

const document = JSON.parse(readFileSync(noticesPath, "utf8")) as NoticesDocument;

describe("MT-029 third-party notices receipt", () => {
  it("matches a fresh regeneration (no drift against lockfile/cargo metadata)", () => {
    // Real generator run — spawns cargo metadata; generous timeout.
    const stdout = execFileSync(
      process.execPath,
      [join(appDir, "scripts", "generate-third-party-notices.mjs"), "--check"],
      { cwd: appDir, encoding: "utf8" },
    );
    const result = JSON.parse(stdout) as { pass: boolean };
    expect(result.pass, stdout).toBe(true);
  }, 120_000);

  it("has the expected schema and a sane notice count", () => {
    expect(document.schema).toBe("handshake.third_party_notices@1");
    expect(document.mt_id).toBe("MT-029");
    expect(document.notice_count).toBe(document.notices.length);
    expect(document.notices.length).toBeGreaterThanOrEqual(50);
  });

  it("covers every bundled-library family declared in the allowlist", () => {
    const familiesInReceipt = new Set(document.notices.map((n) => n.family));
    for (const rule of RUNTIME_DEPENDENCY_ALLOWLIST.bundled_libraries) {
      expect(familiesInReceipt.has(rule.family), `family ${rule.family} missing`).toBe(true);
    }
  });

  it("includes the named MT-029 anchors (monaco, tiptap, prosemirror transitives, yjs, xterm, excalidraw, tree-sitter)", () => {
    const names = new Set(document.notices.map((n) => n.name));
    for (const required of [
      "monaco-editor",
      "@tiptap/core",
      "@tiptap/starter-kit",
      "prosemirror-model",
      "prosemirror-state",
      "prosemirror-view",
      "yjs",
      "@xterm/xterm",
      "@excalidraw/excalidraw",
      "tree-sitter",
      "tree-sitter-rust",
      "tree-sitter-javascript",
      "tree-sitter-typescript",
    ]) {
      expect(names.has(required), `${required} missing from notices`).toBe(true);
    }
    // Transitive walk really happened: more @tiptap/* packages than the 8
    // directly declared in app/package.json.
    const tiptapCount = document.notices.filter((n) => n.name.startsWith("@tiptap/")).length;
    expect(tiptapCount).toBeGreaterThan(8);
  });

  it("keeps every license inside its family's allowed set with named evidence", () => {
    for (const notice of document.notices) {
      const rule = RUNTIME_DEPENDENCY_ALLOWLIST.bundled_libraries.find(
        (r) => r.family === notice.family,
      );
      expect(rule, `family ${notice.family} not in allowlist`).toBeDefined();
      const alternatives = notice.license.split(/\s+OR\s+/i).map((s) => s.trim());
      expect(
        alternatives.some((alt) => rule!.allowed_licenses.includes(alt)),
        `${notice.name}@${notice.version}: ${notice.license} not allowed for ${notice.family}`,
      ).toBe(true);
      expect(notice.license_evidence.length).toBeGreaterThan(0);
      expect(notice.version).toMatch(/^\d+\.\d+\.\d+/);
    }
  });

  it("uses only the two canonical registries", () => {
    for (const notice of document.notices) {
      expect(notice.registry).toBe(
        notice.ecosystem === "npm" ? "https://registry.npmjs.org" : "https://crates.io",
      );
    }
  });
});
