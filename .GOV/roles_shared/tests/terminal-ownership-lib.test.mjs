import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import {
  defaultRegistry,
  loadSessionRegistry,
  mutateSessionRegistrySync,
  resetBatchLaunchMode,
  saveSessionRegistry,
} from "../scripts/session/session-registry-lib.mjs";
import {
  launchOwnedSystemTerminal,
  reclaimOwnedSessionTerminals,
  recordOwnedTerminalLaunch,
} from "../scripts/session/terminal-ownership-lib.mjs";

function makeTempRepoRoot(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function removeTree(targetPath) {
  fs.rmSync(targetPath, { recursive: true, force: true });
}

test("launchOwnedSystemTerminal parses the launched terminal pid from PowerShell output", () => {
  const launch = launchOwnedSystemTerminal({
    worktreeAbs: "D:/tmp/repo",
    launchScriptPath: "D:/tmp/launch.ps1",
    terminalTitle: "CODER WP-TEST",
    runner: () => "4567\r\n",
  });
  assert.equal(launch.processId, 4567);
  assert.equal(launch.hostKind, "SYSTEM_TERMINAL");
});

test("recordOwnedTerminalLaunch writes governed terminal ownership into the session registry", () => {
  const repoRoot = makeTempRepoRoot("handshake-owned-terminal-");
  try {
    saveSessionRegistry(repoRoot, defaultRegistry());
    recordOwnedTerminalLaunch(repoRoot, {
      wp_id: "WP-TEST",
      role: "CODER",
      local_branch: "feat/WP-TEST",
      local_worktree_dir: "../wtc-test",
      terminal_title: "CODER WP-TEST",
      requested_model: "gpt-5.4",
    }, {
      processId: 4242,
      hostKind: "SYSTEM_TERMINAL",
      terminalTitle: "CODER WP-TEST",
    });

    const { registry } = loadSessionRegistry(repoRoot);
    const session = registry.sessions[0];
    assert.equal(session.terminal_ownership_scope, "GOVERNED_SESSION");
    assert.equal(session.owned_terminal_process_id, 4242);
    assert.equal(session.owned_terminal_host_kind, "SYSTEM_TERMINAL");
    assert.match(session.owned_terminal_batch_id, /^TBATCH-/);
    assert.equal(session.owned_terminal_reclaim_status, "OWNED");
  } finally {
    removeTree(repoRoot);
  }
});

test("reclaimOwnedSessionTerminals marks the owned terminal as reclaimed and clears the pid", () => {
  const repoRoot = makeTempRepoRoot("handshake-reclaim-terminal-");
  try {
    saveSessionRegistry(repoRoot, defaultRegistry());
    recordOwnedTerminalLaunch(repoRoot, {
      wp_id: "WP-TEST",
      role: "WP_VALIDATOR",
      local_branch: "feat/WP-TEST",
      local_worktree_dir: "../wtc-test",
      terminal_title: "WPVAL WP-TEST",
      requested_model: "gpt-5.4",
    }, {
      processId: 9898,
      hostKind: "SYSTEM_TERMINAL",
      terminalTitle: "WPVAL WP-TEST",
    });

    const alive = new Set([9898]);
    const results = reclaimOwnedSessionTerminals(repoRoot, { wpId: "WP-TEST" }, {
      inspectProcess: (pid) => alive.has(pid),
      stopProcess: (pid) => {
        alive.delete(pid);
      },
    });

    assert.equal(results.length, 1);
    assert.equal(results[0].reclaim_status, "RECLAIMED");

    const { registry } = loadSessionRegistry(repoRoot);
    const session = registry.sessions[0];
    assert.equal(session.owned_terminal_process_id, 0);
    assert.equal(session.owned_terminal_reclaim_status, "RECLAIMED");
    assert.match(session.owned_terminal_reclaimed_at, /\d{4}-\d{2}-\d{2}T/);
  } finally {
    removeTree(repoRoot);
  }
});

test("reclaimOwnedSessionTerminals honors terminal batch filtering", () => {
  const repoRoot = makeTempRepoRoot("handshake-reclaim-terminal-batch-");
  try {
    saveSessionRegistry(repoRoot, defaultRegistry());

    const firstLaunch = recordOwnedTerminalLaunch(repoRoot, {
      wp_id: "WP-TEST",
      role: "CODER",
      local_branch: "feat/WP-TEST",
      local_worktree_dir: "../wtc-test",
      terminal_title: "CODER WP-TEST",
      requested_model: "gpt-5.4",
    }, {
      processId: 1111,
      hostKind: "SYSTEM_TERMINAL",
      terminalTitle: "CODER WP-TEST",
    });

    const { registry: beforeResetRegistry } = loadSessionRegistry(repoRoot);
    const firstBatchId = beforeResetRegistry.active_terminal_batch_id;
    assert.equal(firstLaunch.owned_terminal_batch_id, firstBatchId);

    const secondBatchId = mutateSessionRegistrySync(repoRoot, (registry) => {
      resetBatchLaunchMode(registry, "operator-approved new governed batch");
      return registry.active_terminal_batch_id;
    });

    const secondLaunch = recordOwnedTerminalLaunch(repoRoot, {
      wp_id: "WP-TEST",
      role: "WP_VALIDATOR",
      local_branch: "feat/WP-TEST",
      local_worktree_dir: "../wtc-test",
      terminal_title: "WPVAL WP-TEST",
      requested_model: "gpt-5.4",
    }, {
      processId: 2222,
      hostKind: "SYSTEM_TERMINAL",
      terminalTitle: "WPVAL WP-TEST",
    });

    assert.notEqual(firstBatchId, secondBatchId);
    assert.equal(secondLaunch.owned_terminal_batch_id, secondBatchId);

    const alive = new Set([1111, 2222]);
    const results = reclaimOwnedSessionTerminals(repoRoot, {
      wpId: "WP-TEST",
      terminalBatchId: secondBatchId,
    }, {
      inspectProcess: (pid) => alive.has(pid),
      stopProcess: (pid) => {
        alive.delete(pid);
      },
    });

    assert.equal(results.length, 1);
    assert.equal(results[0].process_id, 2222);
    assert.equal(results[0].terminal_batch_id, secondBatchId);

    const { registry } = loadSessionRegistry(repoRoot);
    const coderSession = registry.sessions.find((session) => session.role === "CODER");
    const validatorSession = registry.sessions.find((session) => session.role === "WP_VALIDATOR");
    assert.equal(coderSession.owned_terminal_process_id, 1111);
    assert.equal(coderSession.owned_terminal_reclaim_status, "OWNED");
    assert.equal(validatorSession.owned_terminal_process_id, 0);
    assert.equal(validatorSession.owned_terminal_reclaim_status, "RECLAIMED");
  } finally {
    removeTree(repoRoot);
  }
});
