import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  listWorkPacketEntriesAt,
  repoPathAbs,
  resolveWorkPacketPathAtRepo,
  taskBoardPathAtRepo,
  workPacketAbsPathAtRepo,
  workPacketPathAtRepo,
} from "../scripts/lib/runtime-paths.mjs";

test("listWorkPacketEntriesAt discovers flat and folder packets while skipping README and excluded dirs", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "runtime-paths-"));

  try {
    fs.writeFileSync(path.join(tempRoot, "README.md"), "# packets\n", "utf8");
    fs.writeFileSync(path.join(tempRoot, "WP-1-Flat-v1.md"), "# flat packet\n", "utf8");
    fs.mkdirSync(path.join(tempRoot, "WP-1-Folder-v1"), { recursive: true });
    fs.writeFileSync(path.join(tempRoot, "WP-1-Folder-v1", "packet.md"), "# folder packet\n", "utf8");
    fs.mkdirSync(path.join(tempRoot, "stubs"), { recursive: true });
    fs.writeFileSync(path.join(tempRoot, "stubs", "WP-1-Stub-v1.md"), "# stub packet\n", "utf8");

    const entries = listWorkPacketEntriesAt(tempRoot, ".GOV/task_packets", { skipDirNames: ["stubs"] });

    assert.deepEqual(entries, [
      {
        wpId: "WP-1-Flat-v1",
        packetPath: ".GOV/task_packets/WP-1-Flat-v1.md",
        packetDir: ".GOV/task_packets",
        isFolder: false,
      },
      {
        wpId: "WP-1-Folder-v1",
        packetPath: ".GOV/task_packets/WP-1-Folder-v1/packet.md",
        packetDir: ".GOV/task_packets/WP-1-Folder-v1",
        isFolder: true,
      },
    ]);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("repoPathAbs anchors repo-relative paths while preserving absolute paths", () => {
  const relativePath = ".GOV/task_packets/WP-TEST/packet.md";
  const resolvedRelative = repoPathAbs(relativePath);
  assert.equal(path.isAbsolute(resolvedRelative), true);
  assert.match(resolvedRelative.replace(/\\/g, "/"), /\/\.GOV\/task_packets\/WP-TEST\/packet\.md$/);

  const absolutePath = path.resolve(os.tmpdir(), "handshake-runtime-paths-absolute.txt");
  assert.equal(repoPathAbs(absolutePath), absolutePath);
});

test("resolveWorkPacketPathAtRepo accepts canonical work_packets roots during compatibility migration", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "runtime-paths-work-packets-"));
  try {
    const packetPath = path.join(repoRoot, ".GOV", "work_packets", "WP-TEST-WORK-PACKETS-v1", "packet.md");
    fs.mkdirSync(path.dirname(packetPath), { recursive: true });
    fs.writeFileSync(packetPath, "# packet\n", "utf8");

    const resolved = resolveWorkPacketPathAtRepo(repoRoot, "WP-TEST-WORK-PACKETS-v1");
    assert.ok(resolved);
    assert.equal(resolved.packetPath, ".GOV/work_packets/WP-TEST-WORK-PACKETS-v1/packet.md");
    assert.equal(resolved.isFolder, true);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("workPacketPathAtRepo and workPacketAbsPathAtRepo use shared fallback truth", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "runtime-paths-fallback-"));
  try {
    assert.equal(
      workPacketPathAtRepo(repoRoot, "WP-TEST-FALLBACK-v1"),
      ".GOV/task_packets/WP-TEST-FALLBACK-v1.md",
    );
    assert.equal(
      workPacketAbsPathAtRepo(repoRoot, "WP-TEST-FALLBACK-v1"),
      path.join(repoRoot, ".GOV", "task_packets", "WP-TEST-FALLBACK-v1.md"),
    );
    assert.equal(
      taskBoardPathAtRepo(repoRoot),
      path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md"),
    );
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});
