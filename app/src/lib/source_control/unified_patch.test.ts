import { describe, expect, it } from "vitest";
import { parseUnifiedPatchSides } from "./unified_patch";

describe("parseUnifiedPatchSides", () => {
  it("reconstructs original/modified sides from a single-hunk patch", () => {
    const patch = [
      "diff --git a/tracked.txt b/tracked.txt",
      "index 1234567..89abcde 100644",
      "--- a/tracked.txt",
      "+++ b/tracked.txt",
      "@@ -1,2 +1,2 @@",
      " context line",
      "-old value",
      "+new value",
    ].join("\n");

    const sides = parseUnifiedPatchSides(patch);
    expect(sides.original).toBe("context line\nold value");
    expect(sides.modified).toBe("context line\nnew value");
    expect(sides.hunkCount).toBe(1);
    expect(sides.addedLines).toBe(1);
    expect(sides.removedLines).toBe(1);
  });

  it("handles CRLF, no-newline markers, and multiple hunks", () => {
    const patch = [
      "--- a/file.rs",
      "+++ b/file.rs",
      "@@ -1 +1 @@",
      "-fn main() {}",
      "+fn main() { run(); }",
      "\\ No newline at end of file",
      "@@ -10,2 +10,3 @@",
      " keep",
      "+added tail",
    ].join("\r\n");

    const sides = parseUnifiedPatchSides(patch);
    expect(sides.hunkCount).toBe(2);
    expect(sides.original).toBe("fn main() {}\nkeep");
    expect(sides.modified).toBe("fn main() { run(); }\nkeep\nadded tail");
    expect(sides.addedLines).toBe(2);
    expect(sides.removedLines).toBe(1);
  });

  it("returns empty sides for an empty patch and ignores pre-hunk metadata", () => {
    expect(parseUnifiedPatchSides("")).toEqual({
      original: "",
      modified: "",
      hunkCount: 0,
      addedLines: 0,
      removedLines: 0,
    });
    const metadataOnly = "diff --git a/x b/x\nindex 0000..1111 100644\n";
    expect(parseUnifiedPatchSides(metadataOnly).hunkCount).toBe(0);
  });
});
