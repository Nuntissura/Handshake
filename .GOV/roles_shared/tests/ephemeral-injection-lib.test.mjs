import assert from "node:assert/strict";
import test from "node:test";

import {
  buildEphemeralContextBlock,
  normalizeEphemeralTrust,
} from "../scripts/session/ephemeral-injection-lib.mjs";

test("buildEphemeralContextBlock wraps context in a governed user-message fence", () => {
  const block = buildEphemeralContextBlock({
    source: "orchestrator-route",
    trust: "required",
    body: "DIRECT_ROLE_MESSAGE\n- Continue MT-002.",
  });

  assert.match(block, /^\[INFORMATIONAL - not user input\. Source: orchestrator-route\. Trust: required\.\]/);
  assert.match(block, /<governance-context source="orchestrator-route" trust="required">/);
  assert.match(block, /DIRECT_ROLE_MESSAGE/);
  assert.match(block, /<\/governance-context>$/);
});

test("buildEphemeralContextBlock escapes source attribute text", () => {
  const block = buildEphemeralContextBlock({
    source: "operator \"route\" <ctx>",
    trust: "advisory",
    body: "Use compact context.",
  });

  assert.match(block, /source="operator &quot;route&quot; &lt;ctx&gt;"/);
});

test("normalizeEphemeralTrust rejects unknown trust levels", () => {
  assert.equal(normalizeEphemeralTrust("REQUIRED"), "required");
  assert.throws(
    () => buildEphemeralContextBlock({ source: "x", trust: "system", body: "y" }),
    /unknown trust level/i,
  );
});
