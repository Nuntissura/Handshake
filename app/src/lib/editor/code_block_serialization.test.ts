// WP-KERNEL-009 / MT-168 — CodeBlockPersistenceBridge tests.
//
// Proves the code-block (de)serialization round-trips language + code + hash
// through the backend code-node payload shape, the hash is deterministic and
// detects out-of-band corruption, and deserialization tolerates partial/legacy
// shapes (a load never throws on a slightly-off code node).

import { describe, it, expect } from "vitest";
import {
  makeCodeBlockAttrs,
  codeBlockHash,
  verifyCodeBlockIntegrity,
  verifyDocCodeBlockIntegrity,
  serializeCodeNode,
  deserializeCodeNode,
} from "./code_block_serialization";

describe("code-block serialization bridge (MT-168)", () => {
  it("mints normalized attrs with a matching round-trip hash", () => {
    const attrs = makeCodeBlockAttrs("ts", "const x = 1;");
    expect(attrs.language).toBe("typescript"); // normalized from alias
    expect(attrs.code).toBe("const x = 1;");
    expect(verifyCodeBlockIntegrity(attrs)).toBe(true);
  });

  it("computes a deterministic hash (same input → same hash)", () => {
    expect(codeBlockHash("rust", "fn main() {}")).toBe(codeBlockHash("rust", "fn main() {}"));
    // Different code → different hash (sanity, not a collision guarantee).
    expect(codeBlockHash("rust", "fn main() {}")).not.toBe(codeBlockHash("rust", "fn other() {}"));
    // Language is part of the hash.
    expect(codeBlockHash("rust", "x")).not.toBe(codeBlockHash("go", "x"));
  });

  it("round-trips through the backend code-node payload (serialize -> deserialize)", () => {
    const original = makeCodeBlockAttrs("python", "print('hi')");
    const payload = serializeCodeNode(original);
    expect(payload).toEqual({
      language: "python",
      code: "print('hi')",
      content_hash: original.contentHash,
    });
    const restored = deserializeCodeNode(payload);
    expect(restored).toEqual(original);
    expect(verifyCodeBlockIntegrity(restored)).toBe(true);
  });

  it("detects out-of-band corruption via the hash", () => {
    const attrs = makeCodeBlockAttrs("json", '{"a":1}');
    const tampered = { ...attrs, code: '{"a":2}' }; // code changed, hash stale
    expect(verifyCodeBlockIntegrity(tampered)).toBe(false);
  });

  it("tolerates partial/legacy code-node shapes and repairs a missing hash", () => {
    // No hash, alias language, missing code.
    const restored = deserializeCodeNode({ language: "rs" });
    expect(restored.language).toBe("rust");
    expect(restored.code).toBe("");
    expect(verifyCodeBlockIntegrity(restored)).toBe(true); // hash recomputed
    // Accepts camelCase contentHash too.
    const camel = deserializeCodeNode({ language: "go", code: "x", contentHash: codeBlockHash("go", "x") });
    expect(verifyCodeBlockIntegrity(camel)).toBe(true);
    // Completely empty input does not throw.
    expect(() => deserializeCodeNode(null)).not.toThrow();
    expect(deserializeCodeNode(undefined).language).toBe("plaintext");
  });

  it("walks a whole document and reports per-block integrity violations (iteration-3 M9)", () => {
    const good = makeCodeBlockAttrs("typescript", "const x = 1;");
    const tampered = { ...makeCodeBlockAttrs("json", '{"a":1}'), code: '{"a":2}' };
    const doc = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "intro" }] },
        { type: "monacoCodeBlock", attrs: good },
        {
          type: "blockquote",
          content: [{ type: "monacoCodeBlock", attrs: tampered }],
        },
      ],
    };
    const result = verifyDocCodeBlockIntegrity(doc);
    expect(result.checked).toBe(2);
    expect(result.violations).toHaveLength(1);
    expect(result.violations[0]).toMatchObject({
      index: 1,
      language: "json",
      storedHash: tampered.contentHash,
      expectedHash: codeBlockHash("json", '{"a":2}'),
    });
  });

  it("doc integrity walk passes clean docs, skips empty legacy blocks, never throws on junk", () => {
    const clean = {
      type: "doc",
      content: [{ type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("rust", "fn main() {}") }],
    };
    expect(verifyDocCodeBlockIntegrity(clean).violations).toHaveLength(0);
    // Legacy: empty block without a hash is exempt (nothing to corrupt).
    const legacy = {
      type: "doc",
      content: [{ type: "monacoCodeBlock", attrs: { language: "plaintext", code: "", contentHash: "" } }],
    };
    expect(verifyDocCodeBlockIntegrity(legacy).violations).toHaveLength(0);
    expect(verifyDocCodeBlockIntegrity(null)).toEqual({ checked: 0, violations: [] });
    expect(verifyDocCodeBlockIntegrity("garbage")).toEqual({ checked: 0, violations: [] });
    expect(verifyDocCodeBlockIntegrity({ type: "doc" })).toEqual({ checked: 0, violations: [] });
  });
});
