import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const JUSTFILE_PATH = path.resolve("justfile");

test("kernel justfile quotes GOV_ROOT-backed node script paths", () => {
  const justfile = fs.readFileSync(JUSTFILE_PATH, "utf8");

  assert.equal(
    /(^\s*@?node )\{\{GOV_ROOT\}\}\//m.test(justfile),
    false,
    "found unquoted node {{GOV_ROOT}}/ invocation",
  );
  assert.equal(
    /;\s*node \{\{GOV_ROOT\}\}\//m.test(justfile),
    false,
    "found unquoted inline node {{GOV_ROOT}}/ invocation",
  );
  assert.match(
    justfile,
    /integration-validator-closeout-check wp-id:\s*\r?\n\s*@node "\{\{GOV_ROOT\}\}\/roles\/validator\/checks\/integration-validator-closeout-check\.mjs"/m,
  );
});
