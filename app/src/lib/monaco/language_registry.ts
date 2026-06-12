// WP-KERNEL-009 / MT-166 — MonacoLanguageRegistration.
//
// The Handshake code-block language surface: a curated, machine-readable
// registry of the languages the embedded Monaco code blocks (MT-165) support,
// plus the detection logic that maps a fenced-code hint (```ts), a pasted
// language token, or a file extension to a canonical Monaco language id. Monaco
// (bundled via app/src/lib/monaco/setup.ts — lockfile-governed, offline, no CDN)
// already ships ~80 basic-language grammars; this module does NOT re-implement
// them. It (1) declares which languages Handshake treats as first-class with
// stable ids/aliases/extensions for detection, and (2) normalizes arbitrary
// hints to a registered id so a code block always has a deterministic language
// (falling back to "plaintext" rather than guessing).
//
// The detection logic is PURE (no monaco import) so it is unit-testable in
// jsdom; the optional registration/verification helpers that touch the live
// monaco.languages registry are isolated and only used where a real editor is
// mounted. The persisted RichDocument code node stores the canonical language
// id (MT-168); detection here is what produces that id.

/** A Handshake first-class code language. */
export interface CodeLanguageDescriptor {
  /** Canonical Monaco language id (must match a monaco-registered id). */
  readonly id: string;
  /** Operator-facing label. */
  readonly label: string;
  /** Lower-cased aliases/hint tokens that map to this id (incl. the id). */
  readonly aliases: readonly string[];
  /** File extensions (no dot, lower-cased) that map to this id. */
  readonly extensions: readonly string[];
}

/**
 * The curated first-class language set. These are the languages an operator
 * editing Handshake project knowledge actually uses; any other monaco language
 * id still works when passed explicitly, but detection from a bare hint resolves
 * to these first. Ordered for stable iteration.
 */
export const HANDSHAKE_CODE_LANGUAGES: readonly CodeLanguageDescriptor[] = [
  { id: "typescript", label: "TypeScript", aliases: ["typescript", "ts", "tsx"], extensions: ["ts", "tsx", "mts", "cts"] },
  { id: "javascript", label: "JavaScript", aliases: ["javascript", "js", "jsx", "node"], extensions: ["js", "jsx", "mjs", "cjs"] },
  { id: "json", label: "JSON", aliases: ["json", "jsonc", "json5"], extensions: ["json", "jsonc"] },
  { id: "rust", label: "Rust", aliases: ["rust", "rs"], extensions: ["rs"] },
  { id: "python", label: "Python", aliases: ["python", "py"], extensions: ["py", "pyi"] },
  { id: "go", label: "Go", aliases: ["go", "golang"], extensions: ["go"] },
  { id: "sql", label: "SQL", aliases: ["sql", "pgsql", "postgres", "postgresql"], extensions: ["sql"] },
  { id: "shell", label: "Shell", aliases: ["shell", "bash", "sh", "zsh"], extensions: ["sh", "bash", "zsh"] },
  { id: "powershell", label: "PowerShell", aliases: ["powershell", "ps", "ps1", "pwsh"], extensions: ["ps1", "psm1"] },
  { id: "yaml", label: "YAML", aliases: ["yaml", "yml"], extensions: ["yaml", "yml"] },
  { id: "toml", label: "TOML", aliases: ["toml"], extensions: ["toml"] },
  { id: "markdown", label: "Markdown", aliases: ["markdown", "md", "mdx"], extensions: ["md", "markdown", "mdx"] },
  { id: "html", label: "HTML", aliases: ["html", "htm"], extensions: ["html", "htm"] },
  { id: "css", label: "CSS", aliases: ["css"], extensions: ["css"] },
  { id: "dockerfile", label: "Dockerfile", aliases: ["dockerfile", "docker"], extensions: [] },
  { id: "plaintext", label: "Plain text", aliases: ["plaintext", "text", "txt", "plain"], extensions: ["txt"] },
] as const;

/** The default language id when nothing can be detected. */
export const DEFAULT_CODE_LANGUAGE = "plaintext";

const ALIAS_TO_ID: ReadonlyMap<string, string> = (() => {
  const map = new Map<string, string>();
  for (const lang of HANDSHAKE_CODE_LANGUAGES) {
    map.set(lang.id.toLowerCase(), lang.id);
    for (const alias of lang.aliases) map.set(alias.toLowerCase(), lang.id);
  }
  return map;
})();

const EXT_TO_ID: ReadonlyMap<string, string> = (() => {
  const map = new Map<string, string>();
  for (const lang of HANDSHAKE_CODE_LANGUAGES) {
    for (const ext of lang.extensions) map.set(ext.toLowerCase(), lang.id);
  }
  return map;
})();

/** All curated language ids (stable order). */
export const HANDSHAKE_CODE_LANGUAGE_IDS: readonly string[] =
  HANDSHAKE_CODE_LANGUAGES.map((l) => l.id);

/** Returns the descriptor for a canonical id, or undefined. */
export function codeLanguage(id: string): CodeLanguageDescriptor | undefined {
  return HANDSHAKE_CODE_LANGUAGES.find((l) => l.id === id);
}

/**
 * Normalizes an arbitrary language hint (fenced-code token, pasted token,
 * user-typed alias) to a canonical curated language id. Unknown hints resolve
 * to DEFAULT_CODE_LANGUAGE — a code block always gets a deterministic id.
 */
export function normalizeLanguageHint(hint: string | null | undefined): string {
  if (!hint) return DEFAULT_CODE_LANGUAGE;
  const key = hint.trim().toLowerCase();
  if (key.length === 0) return DEFAULT_CODE_LANGUAGE;
  return ALIAS_TO_ID.get(key) ?? DEFAULT_CODE_LANGUAGE;
}

/**
 * Detects a language id from a file path/name by extension (e.g.
 * "src/app.tsx" → "typescript"). Falls back to DEFAULT_CODE_LANGUAGE.
 */
export function languageFromFileName(fileName: string | null | undefined): string {
  if (!fileName) return DEFAULT_CODE_LANGUAGE;
  const base = fileName.trim().toLowerCase();
  // Special-case extensionless well-known names.
  if (base === "dockerfile" || base.endsWith("/dockerfile")) return "dockerfile";
  const dot = base.lastIndexOf(".");
  if (dot < 0 || dot === base.length - 1) return DEFAULT_CODE_LANGUAGE;
  const ext = base.slice(dot + 1);
  return EXT_TO_ID.get(ext) ?? DEFAULT_CODE_LANGUAGE;
}

/**
 * Parses a fenced-code opening fence info-string (the text after ```), e.g.
 * "ts", "rust", "json title=foo" → the canonical language id. Only the first
 * whitespace-delimited token is treated as the language.
 */
export function languageFromFenceInfo(info: string | null | undefined): string {
  if (!info) return DEFAULT_CODE_LANGUAGE;
  const token = info.trim().split(/\s+/)[0] ?? "";
  return normalizeLanguageHint(token);
}

// ---------------------------------------------------------------------------
// Live-registry helpers (touch monaco only when a real editor is mounted).
// Kept dependency-light: callers pass the monaco namespace so this module stays
// importable in jsdom without booting monaco.
// ---------------------------------------------------------------------------

interface MonacoLanguagesLike {
  languages: {
    getLanguages(): Array<{ id: string }>;
    register(language: { id: string; extensions?: string[]; aliases?: string[] }): void;
  };
}

/**
 * Ensures every curated Handshake language id is known to the live monaco
 * registry, registering Handshake aliases/extensions for ids monaco already
 * ships (monaco dedupes by id). Returns the ids that were newly registered.
 * Idempotent. Monaco's built-in contributions remain the grammar source; this
 * only guarantees the ids + Handshake aliases resolve.
 */
export function ensureHandshakeLanguagesRegistered(
  monacoNs: MonacoLanguagesLike,
): string[] {
  const known = new Set(monacoNs.languages.getLanguages().map((l) => l.id));
  const registered: string[] = [];
  for (const lang of HANDSHAKE_CODE_LANGUAGES) {
    if (!known.has(lang.id)) {
      monacoNs.languages.register({
        id: lang.id,
        extensions: lang.extensions.map((e) => `.${e}`),
        aliases: [lang.label, ...lang.aliases],
      });
      registered.push(lang.id);
    }
  }
  return registered;
}

/** True when the given language id is registered in the live monaco registry. */
export function isLanguageRegistered(
  monacoNs: MonacoLanguagesLike,
  id: string,
): boolean {
  return monacoNs.languages.getLanguages().some((l) => l.id === id);
}
