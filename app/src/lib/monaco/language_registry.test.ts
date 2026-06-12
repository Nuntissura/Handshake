// WP-KERNEL-009 / MT-166 — MonacoLanguageRegistration tests.
//
// Proves language detection is deterministic across fence hints, file
// extensions, and aliases (the inputs that drive the persisted code-node
// language id), always resolving to a curated id or the plaintext fallback, and
// that the live-registry helpers register/verify against a fake monaco.languages
// without booting the real monaco runtime.

import { describe, it, expect } from "vitest";
import {
  HANDSHAKE_CODE_LANGUAGES,
  HANDSHAKE_CODE_LANGUAGE_IDS,
  DEFAULT_CODE_LANGUAGE,
  normalizeLanguageHint,
  languageFromFileName,
  languageFromFenceInfo,
  codeLanguage,
  ensureHandshakeLanguagesRegistered,
  isLanguageRegistered,
} from "./language_registry";

describe("language registry (MT-166)", () => {
  it("has unique ids and includes the core operator languages", () => {
    const ids = HANDSHAKE_CODE_LANGUAGE_IDS;
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ["typescript", "rust", "python", "json", "sql", "shell", "powershell", "plaintext"]) {
      expect(ids).toContain(id);
    }
  });

  it("normalizes aliases (case-insensitive) to canonical ids", () => {
    expect(normalizeLanguageHint("ts")).toBe("typescript");
    expect(normalizeLanguageHint("TS")).toBe("typescript");
    expect(normalizeLanguageHint("rs")).toBe("rust");
    expect(normalizeLanguageHint("golang")).toBe("go");
    expect(normalizeLanguageHint("pgsql")).toBe("sql");
    expect(normalizeLanguageHint("bash")).toBe("shell");
  });

  it("falls back to plaintext for unknown or empty hints", () => {
    expect(normalizeLanguageHint("klingon")).toBe(DEFAULT_CODE_LANGUAGE);
    expect(normalizeLanguageHint("")).toBe(DEFAULT_CODE_LANGUAGE);
    expect(normalizeLanguageHint(null)).toBe(DEFAULT_CODE_LANGUAGE);
    expect(normalizeLanguageHint(undefined)).toBe(DEFAULT_CODE_LANGUAGE);
  });

  it("detects language from a file name extension", () => {
    expect(languageFromFileName("src/app.tsx")).toBe("typescript");
    expect(languageFromFileName("main.rs")).toBe("rust");
    expect(languageFromFileName("query.SQL")).toBe("sql");
    expect(languageFromFileName("Dockerfile")).toBe("dockerfile");
    expect(languageFromFileName("path/to/Dockerfile")).toBe("dockerfile");
    expect(languageFromFileName("noext")).toBe(DEFAULT_CODE_LANGUAGE);
    expect(languageFromFileName("trailingdot.")).toBe(DEFAULT_CODE_LANGUAGE);
  });

  it("detects language from a fenced-code info string (first token only)", () => {
    expect(languageFromFenceInfo("ts")).toBe("typescript");
    expect(languageFromFenceInfo("json title=config")).toBe("json");
    expect(languageFromFenceInfo("")).toBe(DEFAULT_CODE_LANGUAGE);
  });

  it("returns descriptors by id", () => {
    expect(codeLanguage("rust")?.label).toBe("Rust");
    expect(codeLanguage("nope")).toBeUndefined();
  });

  it("registers missing languages and verifies registration against a fake monaco", () => {
    // Fake monaco that ships only typescript; the helper must register the rest.
    const registered: Array<{ id: string }> = [{ id: "typescript" }];
    const fakeMonaco = {
      languages: {
        getLanguages: () => registered,
        register: (language: { id: string }) => {
          registered.push({ id: language.id });
        },
      },
    };
    const newlyRegistered = ensureHandshakeLanguagesRegistered(fakeMonaco);
    expect(newlyRegistered).not.toContain("typescript"); // already present
    expect(newlyRegistered).toContain("rust");
    // Every curated language is now registered.
    for (const lang of HANDSHAKE_CODE_LANGUAGES) {
      expect(isLanguageRegistered(fakeMonaco, lang.id)).toBe(true);
    }
    // Idempotent: a second call registers nothing new.
    expect(ensureHandshakeLanguagesRegistered(fakeMonaco)).toEqual([]);
  });
});
