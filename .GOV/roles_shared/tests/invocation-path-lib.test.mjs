import assert from "node:assert/strict";
import test from "node:test";

import {
  canonicalInvocationPath,
  isInvokedAsMain,
} from "../scripts/lib/invocation-path-lib.mjs";

test("canonicalInvocationPath falls back to resolved path when realpath fails", () => {
  const result = canonicalInvocationPath("D:/repo/.GOV/script.mjs", {
    realpathSync() {
      throw new Error("missing");
    },
  });

  assert.match(result.replace(/\\/g, "/"), /D:\/repo\/\.GOV\/script\.mjs$/);
});

test("isInvokedAsMain treats symlink and real module paths as equivalent", () => {
  const realScriptPath = "D:/repo/.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs";
  const symlinkScriptPath = "D:/repo/wtc-workflow-mirror-v1/.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs";
  const remap = (candidate) => {
    const normalized = String(candidate || "").replace(/\\/g, "/");
    if (normalized === symlinkScriptPath) return realScriptPath;
    return normalized;
  };

  assert.equal(
    isInvokedAsMain(`file:///${realScriptPath}`, symlinkScriptPath, { realpathSync: remap }),
    true,
  );
});

test("isInvokedAsMain returns false for different scripts", () => {
  assert.equal(
    isInvokedAsMain("file:///D:/repo/.GOV/script-a.mjs", "D:/repo/.GOV/script-b.mjs", {
      realpathSync: (candidate) => String(candidate || "").replace(/\\/g, "/"),
    }),
    false,
  );
});
