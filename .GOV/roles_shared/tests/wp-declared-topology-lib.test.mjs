import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import { evaluateWpDeclaredTopology } from "../scripts/lib/wp-declared-topology-lib.mjs";

const repoRoot = path.resolve("D:/Projects/LLM projects/Handshake/Handshake Worktrees/handshake_main");
const wpId = "WP-1-Structured-Collaboration-Schema-Registry-v4";
const packetContent = `
- LOCAL_BRANCH: feat/${wpId}
- LOCAL_WORKTREE_DIR: ../wtc-schema-registry-v4
- WP_VALIDATOR_LOCAL_BRANCH: validate/${wpId}
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtv-schema-registry-v4
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
`;

test("declared WP topology accepts the declared coder and WP validator worktrees", () => {
  const evaluation = evaluateWpDeclaredTopology({
    repoRoot,
    wpId,
    packetContent,
    branchHeads: {
      [`feat/${wpId}`]: "511dc5e111111111111111111111111111111111",
      [`validate/${wpId}`]: "511dc5e111111111111111111111111111111111",
    },
    worktrees: [
      {
        path: path.resolve(repoRoot, "../handshake_main"),
        branch: "refs/heads/main",
        head: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      },
      {
        path: path.resolve(repoRoot, "../wtc-schema-registry-v4"),
        branch: `refs/heads/feat/${wpId}`,
        head: "511dc5e111111111111111111111111111111111",
      },
      {
        path: path.resolve(repoRoot, "../wtv-schema-registry-v4"),
        branch: `refs/heads/validate/${wpId}`,
        head: "511dc5e111111111111111111111111111111111",
      },
    ],
  });

  assert.equal(evaluation.ok, true);
  assert.deepEqual(evaluation.issues, []);
});

test("declared WP topology rejects legacy shared coder and WP validator worktrees", () => {
  const evaluation = evaluateWpDeclaredTopology({
    repoRoot,
    wpId,
    packetContent: packetContent
      .replace(`validate/${wpId}`, `feat/${wpId}`)
      .replace("../wtv-schema-registry-v4", "../wtc-schema-registry-v4"),
    branchHeads: {
      [`feat/${wpId}`]: "511dc5e111111111111111111111111111111111",
    },
    worktrees: [
      {
        path: path.resolve(repoRoot, "../wtc-schema-registry-v4"),
        branch: `refs/heads/feat/${wpId}`,
        head: "511dc5e111111111111111111111111111111111",
      },
    ],
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /wp validator worktree must be distinct from coder worktree/i);
});

test("declared WP topology rejects auxiliary detached check worktrees", () => {
  const evaluation = evaluateWpDeclaredTopology({
    repoRoot,
    wpId,
    packetContent,
    branchHeads: {
      [`feat/${wpId}`]: "511dc5e111111111111111111111111111111111",
    },
    worktrees: [
      {
        path: path.resolve(repoRoot, "../wtc-schema-registry-v4"),
        branch: `refs/heads/feat/${wpId}`,
        head: "511dc5e111111111111111111111111111111111",
      },
      {
        path: path.resolve(repoRoot, "../wtv-schema-registry-v4"),
        branch: `refs/heads/validate/${wpId}`,
        head: "511dc5e111111111111111111111111111111111",
      },
      {
        path: path.resolve(repoRoot, "../wtc-schema-registry-v4-check-511dc5e"),
        branch: "",
        head: "511dc5e111111111111111111111111111111111",
      },
    ],
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /undeclared WP-adjacent worktree detected/);
});

test("declared WP topology rejects token-matching detached validator clones on the same WP head", () => {
  const evaluation = evaluateWpDeclaredTopology({
    repoRoot,
    wpId,
    packetContent,
    branchHeads: {
      [`feat/${wpId}`]: "511dc5e111111111111111111111111111111111",
    },
    worktrees: [
      {
        path: path.resolve(repoRoot, "../wtc-schema-registry-v4"),
        branch: `refs/heads/feat/${wpId}`,
        head: "511dc5e111111111111111111111111111111111",
      },
      {
        path: path.resolve(repoRoot, "../wtv-schema-registry-v4"),
        branch: `refs/heads/validate/${wpId}`,
        head: "511dc5e111111111111111111111111111111111",
      },
      {
        path: path.resolve(repoRoot, "../handshake-wp1-schema-validator-511dc5e"),
        branch: "",
        head: "511dc5e111111111111111111111111111111111",
      },
    ],
  });

  assert.equal(evaluation.ok, false);
  assert.match(evaluation.issues.join("\n"), /handshake-wp1-schema-validator-511dc5e/);
});

test("declared WP topology accepts a packet-declared coder worktree confirmed by direct probe outside the local worktree family", () => {
  const evaluation = evaluateWpDeclaredTopology({
    repoRoot,
    wpId,
    packetContent,
    worktrees: [
      {
        path: path.resolve(repoRoot, "../handshake_main"),
        branch: "refs/heads/main",
        head: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      },
    ],
    declaredWorktreeProbe: (worktreeAbs) => {
      if (worktreeAbs === path.resolve(repoRoot, "../wtc-schema-registry-v4")) {
        return {
          path: worktreeAbs,
          branch: `refs/heads/feat/${wpId}`,
          head: "511dc5e111111111111111111111111111111111",
        };
      }
      if (worktreeAbs === path.resolve(repoRoot, "../wtv-schema-registry-v4")) {
        return {
          path: worktreeAbs,
          branch: `refs/heads/validate/${wpId}`,
          head: "511dc5e111111111111111111111111111111111",
        };
      }
      return null;
    },
  });

  assert.equal(evaluation.ok, true);
  assert.equal(evaluation.directProbeUsed, true);
  assert.deepEqual(evaluation.issues, []);
});
