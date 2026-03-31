import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { listWorkPacketEntriesAt, repoPathAbs } from "../scripts/lib/runtime-paths.mjs";

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
