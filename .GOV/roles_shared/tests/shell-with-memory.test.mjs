import assert from "node:assert/strict";
import test from "node:test";

import {
  buildShellExecutionArgs,
  buildShellMemoryMetadata,
} from "../scripts/memory/shell-with-memory.mjs";

test("buildShellExecutionArgs selects the expected host shell invocation", () => {
  assert.deepEqual(buildShellExecutionArgs("powershell", "git status"), {
    tool: "powershell.exe",
    args: ["-NoLogo", "-NonInteractive", "-Command", "git status"],
  });
  assert.deepEqual(buildShellExecutionArgs("cmd", "dir"), {
    tool: "cmd.exe",
    args: ["/d", "/s", "/c", "dir"],
  });
  assert.deepEqual(buildShellExecutionArgs("bash", "pwd"), {
    tool: "bash",
    args: ["-lc", "pwd"],
  });
});

test("buildShellMemoryMetadata records command-family trigger context", () => {
  const metadata = buildShellMemoryMetadata({
    commandFamily: "cargo-test",
    rawCommand: "cargo test -p handshake_core",
    shell: "powershell",
    exitCode: 101,
    action: "COMMAND",
  });

  assert.deepEqual(metadata, {
    command_family: "cargo-test",
    raw_command: "cargo test -p handshake_core",
    shell: "powershell",
    exit_code: 101,
    trigger: "cargo-test",
    action: "COMMAND",
    wrapper: "shell-with-memory",
  });
});
