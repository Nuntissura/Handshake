import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  communicationMonitorState,
  evaluateWpCommunicationHealth,
} from "../scripts/lib/wp-communication-health-lib.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = path.resolve(__dirname, "../fixtures/wp-communication-health");

for (const fixtureName of fs.readdirSync(FIXTURES_DIR).filter((name) => name.endsWith(".json")).sort()) {
  const fixturePath = path.join(FIXTURES_DIR, fixtureName);
  const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));

  test(`wp communication health fixture: ${fixtureName} :: ${fixture.description}`, () => {
    const evaluation = evaluateWpCommunicationHealth(fixture.input);

    assert.equal(evaluation.applicable, fixture.expected.applicable, "applicable mismatch");
    assert.equal(evaluation.ok, fixture.expected.ok, "ok mismatch");
    assert.equal(evaluation.state, fixture.expected.state, "state mismatch");

    for (const monitorExpectation of fixture.monitor || []) {
      assert.equal(
        communicationMonitorState(evaluation, { stale: Boolean(monitorExpectation.stale) }),
        monitorExpectation.state,
        `monitor state mismatch for stale=${monitorExpectation.stale}`
      );
    }
  });
}
