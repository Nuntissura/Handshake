import { describe, expect, it } from "vitest";
import { parseLoomSearchOperators } from "./loom_search_query";

describe("parseLoomSearchOperators", () => {
  it("extracts supported inline operators and leaves the free-text query", () => {
    expect(
      parseLoomSearchOperators('Alpha roadmap tag:#tag-1 tag:tag-2 mention:MT-258 kind:document path:"src/app notes"'),
    ).toEqual({
      q: "Alpha roadmap",
      tagIds: ["tag-1", "tag-2"],
      mentionIds: ["MT-258"],
      sourceKinds: ["document"],
      path: "src/app notes",
      errors: [],
    });
  });

  it("supports folder alias plus comma-separated operator values", () => {
    expect(parseLoomSearchOperators('beta folder:"Daily Notes/2026" kind:loom_block,tag_hub tag:one,two')).toEqual({
      q: "beta",
      tagIds: ["one", "two"],
      mentionIds: [],
      sourceKinds: ["loom_block", "tag_hub"],
      path: "Daily Notes/2026",
      errors: [],
    });
  });

  it("fails closed for invalid kind operators", () => {
    expect(parseLoomSearchOperators("alpha kind:not-a-kind")).toEqual({
      q: "alpha",
      tagIds: [],
      mentionIds: [],
      sourceKinds: [],
      path: undefined,
      errors: ["Invalid kind operator: not-a-kind"],
    });
  });
});
