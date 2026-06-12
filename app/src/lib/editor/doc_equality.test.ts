// WP-KERNEL-009 iteration-3 hardening (H1) — doc_equality unit tests.

import { describe, it, expect } from "vitest";
import { jsonDeepEquals } from "./doc_equality";

describe("jsonDeepEquals", () => {
  it("treats identical references and primitives as equal", () => {
    const doc = { type: "doc", content: [] };
    expect(jsonDeepEquals(doc, doc)).toBe(true);
    expect(jsonDeepEquals("a", "a")).toBe(true);
    expect(jsonDeepEquals(3, 3)).toBe(true);
    expect(jsonDeepEquals(null, null)).toBe(true);
    expect(jsonDeepEquals(true, true)).toBe(true);
  });

  it("treats structural clones as equal (JSON round-trip)", () => {
    const doc = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "hello" }] },
        { type: "monacoCodeBlock", attrs: { language: "ts", code: "x", contentHash: "ab" } },
      ],
    };
    expect(jsonDeepEquals(doc, JSON.parse(JSON.stringify(doc)))).toBe(true);
  });

  it("detects content differences at any depth", () => {
    const a = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "hello" }] }] };
    const b = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "hellp" }] }] };
    expect(jsonDeepEquals(a, b)).toBe(false);
  });

  it("detects added/removed keys and array length changes", () => {
    expect(jsonDeepEquals({ a: 1 }, { a: 1, b: 2 })).toBe(false);
    expect(jsonDeepEquals({ a: 1, b: 2 }, { a: 1 })).toBe(false);
    expect(jsonDeepEquals([1, 2], [1, 2, 3])).toBe(false);
    expect(jsonDeepEquals([1, 2], [2, 1])).toBe(false);
  });

  it("distinguishes null / array / object shapes", () => {
    expect(jsonDeepEquals(null, {})).toBe(false);
    expect(jsonDeepEquals({}, null)).toBe(false);
    expect(jsonDeepEquals([], {})).toBe(false);
    expect(jsonDeepEquals({}, [])).toBe(false);
    expect(jsonDeepEquals(0, "0")).toBe(false);
  });
});
