import { describe, expect, it } from "vitest";

import { decodeChunk } from "./terminal";

// These tests target the pure, jsdom-safe parts of the terminal IPC client:
// the base64 -> raw bytes decode that feeds xterm. The Tauri `invoke` / `listen`
// wrappers are exercised by the Playwright real-app path, not here (no Tauri
// runtime under jsdom). The decode is the correctness-critical seam: terminal
// output carries ANSI control sequences and multibyte UTF-8 that MUST round-trip
// byte-for-byte; stringifying corrupts both.

function toBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

describe("decodeChunk", () => {
  it("round-trips ASCII bytes exactly", () => {
    const bytes = new Uint8Array([0x68, 0x69, 0x0a]); // "hi\n"
    expect(Array.from(decodeChunk(toBase64(bytes)))).toEqual([0x68, 0x69, 0x0a]);
  });

  it("preserves ANSI control sequences without corruption", () => {
    // ESC [ 3 1 m  (red), bytes 0x1b 0x5b 0x33 0x31 0x6d
    const ansi = new Uint8Array([0x1b, 0x5b, 0x33, 0x31, 0x6d, 0x41, 0x1b, 0x5b, 0x30, 0x6d]);
    expect(Array.from(decodeChunk(toBase64(ansi)))).toEqual(Array.from(ansi));
  });

  it("preserves multibyte UTF-8 bytes (e.g. 'é' = 0xc3 0xa9) byte-for-byte", () => {
    const utf8 = new Uint8Array([0xc3, 0xa9]);
    const out = decodeChunk(toBase64(utf8));
    expect(out.length).toBe(2);
    expect(out[0]).toBe(0xc3);
    expect(out[1]).toBe(0xa9);
  });

  it("round-trips INVALID UTF-8 bytes without loss or replacement chars", () => {
    // 0xff 0xfe are not valid UTF-8 lead bytes; a TextDecoder path would mangle
    // these into U+FFFD. The byte decode must keep them exactly.
    const invalid = new Uint8Array([0xff, 0xfe, 0x00, 0x80, 0xc0]);
    const out = decodeChunk(toBase64(invalid));
    expect(Array.from(out)).toEqual([0xff, 0xfe, 0x00, 0x80, 0xc0]);
  });

  it("decodes an empty chunk to an empty buffer", () => {
    expect(decodeChunk("").length).toBe(0);
  });

  it("returns a Uint8Array (never a string)", () => {
    expect(decodeChunk(toBase64(new Uint8Array([1, 2, 3])))).toBeInstanceOf(Uint8Array);
  });
});
