import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  absorberHitsLogPath,
  normalizeBulletPrefixedFields,
  normalizeDashes,
  normalizeFieldValueWhitespace,
  normalizeHeadingLevels,
  normalizeHeadingPrefix,
  normalizeJsonStringVsArray,
  normalizeLineEndings,
  normalizeNullishFieldValues,
  normalizeSmartQuotes,
  normalizeTrailingNewline,
  normalizeWindowsPathEscapes,
  runAbsorber,
} from "../../scripts/lib/artifact-normalizers/index.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const NORMALIZERS = {
  normalizeLineEndings,
  normalizeTrailingNewline,
  normalizeSmartQuotes,
  normalizeDashes,
  normalizeJsonStringVsArray,
  normalizeBulletPrefixedFields,
  normalizeHeadingPrefix,
  normalizeFieldValueWhitespace,
  normalizeWindowsPathEscapes,
  normalizeNullishFieldValues,
  normalizeHeadingLevels,
};

for (const fixtureFile of fs.readdirSync(__dirname).filter((name) => name.endsWith(".json")).sort()) {
  const fixture = JSON.parse(fs.readFileSync(path.join(__dirname, fixtureFile), "utf8"));
  test(`${fixture.absorber} fixture ${fixtureFile}`, () => {
    const normalizer = NORMALIZERS[fixture.absorber];
    assert.equal(typeof normalizer, "function", `missing normalizer ${fixture.absorber}`);
    const result = normalizer(fixture.input);
    assert.equal(result.applied, fixture.applied);
    if (fixture.compareJson) {
      assert.deepEqual(JSON.parse(result.output), JSON.parse(fixture.output));
    } else {
      assert.equal(result.output, fixture.output);
    }
  });
}

test("runAbsorber applies declared order and writes hit log for applied normalizers", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "artifact-normalizer-hit-"));
  try {
    const result = runAbsorber("- GOVERNANCE_VERDICT: \u201cPASS\u201d\r\n", {
      artifactKind: "validator_report",
      wpId: "WP-TEST-v1",
      runtimeRootAbs: root,
    });
    assert.equal(result.output, "GOVERNANCE_VERDICT: \"PASS\"\n");
    assert.deepEqual(
      result.applied.map((entry) => entry.name),
      [
        "normalizeLineEndings",
        "normalizeSmartQuotes",
        "normalizeBulletPrefixedFields",
      ],
    );
    const logPath = absorberHitsLogPath({ runtimeRootAbs: root });
    const rows = fs.readFileSync(logPath, "utf8").trim().split(/\r?\n/).map((line) => JSON.parse(line));
    assert.equal(rows.length, 1);
    assert.equal(rows[0].wp_id, "WP-TEST-v1");
    assert.equal(rows[0].artifactKind, "validator_report");
    assert.equal(rows[0].applied.length, 3);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
