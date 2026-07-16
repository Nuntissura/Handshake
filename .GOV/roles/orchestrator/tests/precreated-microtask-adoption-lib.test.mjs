import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  inspectPrecreatedMicrotaskSuite,
  LEGACY_MICROTASK_GENERATION_MODE,
  PRECREATED_MICROTASK_ADOPTION_MODE,
} from "../scripts/lib/precreated-microtask-adoption-lib.mjs";

const WP_ID = "WP-TEST-PRECREATED-MT-v1";

function withTempDir(run) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-precreated-mt-"));
  try {
    return run(dir);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

function mtContract(id, dependsOn = [], overrides = {}) {
  return {
    schema_id: "hsk.microtask_contract@1",
    schema_version: "microtask_contract_v1",
    wp_id: WP_ID,
    mt_id: id,
    lifecycle: {
      status: "PENDING",
      depends_on: dependsOn,
      active: false,
      ...(overrides.lifecycle || {}),
    },
    scope: {
      summary: `Implement ${id}`,
      ...(overrides.scope || {}),
    },
    ...Object.fromEntries(Object.entries(overrides).filter(([key]) => !["lifecycle", "scope"].includes(key))),
  };
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function writeSuite(dir, contracts, indexOverrides = {}) {
  const rows = contracts.map((contract) => ({
    mt_id: contract.mt_id,
    depends_on: contract.lifecycle.depends_on,
    status: contract.lifecycle.status,
    summary: contract.scope.summary,
  }));
  const ids = rows.map((row) => row.mt_id);
  writeJson(path.join(dir, "_MT_INDEX.json"), {
    schema: "test.mt_index@1",
    wp_id: WP_ID,
    total: rows.length,
    range: `${ids[0]}..${ids.at(-1)}`,
    microtasks: rows,
    ...indexOverrides,
  });
  for (const contract of contracts) writeJson(path.join(dir, `${contract.mt_id}.json`), contract);
}

test("an empty WP directory preserves legacy microtask generation mode", () => withTempDir((dir) => {
  const result = inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID });
  assert.equal(result.mode, LEGACY_MICROTASK_GENERATION_MODE);
  assert.deepEqual(result.declaredIds, []);
  assert.equal(result.activeId, null);
  assert.equal(result.nextId, null);
}));

test("a complete indexed suite is adopted without changing any MT JSON bytes", () => withTempDir((dir) => {
  const contracts = [
    mtContract("MT-001"),
    mtContract("MT-002", ["MT-001"]),
    mtContract("MT-003", ["MT-001"]),
  ];
  writeSuite(dir, contracts);
  const before = new Map(contracts.map((contract) => [
    contract.mt_id,
    fs.readFileSync(path.join(dir, `${contract.mt_id}.json`), "utf8"),
  ]));

  const result = inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID });

  assert.equal(result.mode, PRECREATED_MICROTASK_ADOPTION_MODE);
  assert.deepEqual(result.declaredIds, ["MT-001", "MT-002", "MT-003"]);
  assert.equal(result.activeId, null);
  assert.equal(result.nextId, "MT-001");
  for (const [id, bytes] of before) {
    assert.equal(fs.readFileSync(path.join(dir, `${id}.json`), "utf8"), bytes);
  }
}));

test("the suite's sole active contract drives aligned active_id and next_id", () => withTempDir((dir) => {
  writeSuite(dir, [
    mtContract("MT-001"),
    mtContract("MT-002", ["MT-001"], { lifecycle: { active: true } }),
  ]);

  const result = inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID });
  assert.equal(result.activeId, "MT-002");
  assert.equal(result.nextId, "MT-002");
}));

test("an MT contract without an index fails closed instead of invoking legacy generation", () => withTempDir((dir) => {
  writeJson(path.join(dir, "MT-001.json"), mtContract("MT-001"));
  assert.throws(
    () => inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID }),
    /_MT_INDEX\.json is required/,
  );
}));

test("missing and unindexed contract IDs fail closed", () => withTempDir((dir) => {
  const contracts = [mtContract("MT-001"), mtContract("MT-002", ["MT-001"])];
  writeSuite(dir, contracts);
  fs.rmSync(path.join(dir, "MT-002.json"));
  writeJson(path.join(dir, "MT-003.json"), mtContract("MT-003"));

  assert.throws(
    () => inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID }),
    /missing indexed contracts: MT-002; unindexed contracts: MT-003/,
  );
}));

test("index-to-contract dependency drift fails closed", () => withTempDir((dir) => {
  const contracts = [mtContract("MT-001"), mtContract("MT-002", ["MT-001"])];
  writeSuite(dir, contracts, {
    microtasks: [
      { mt_id: "MT-001", depends_on: [], status: "PENDING", summary: "Implement MT-001" },
      { mt_id: "MT-002", depends_on: [], status: "PENDING", summary: "Implement MT-002" },
    ],
  });

  assert.throws(
    () => inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID }),
    /MT-002: index depends_on does not match lifecycle\.depends_on/,
  );
}));

test("dependency cycles fail closed with the cycle path", () => withTempDir((dir) => {
  writeSuite(dir, [
    mtContract("MT-001", ["MT-002"]),
    mtContract("MT-002", ["MT-001"]),
  ]);

  assert.throws(
    () => inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID }),
    /dependency cycle detected: MT-001 -> MT-002 -> MT-001/,
  );
}));

test("contract identity and multiple-active mismatches fail closed", () => withTempDir((dir) => {
  writeSuite(dir, [
    mtContract("MT-001", [], { lifecycle: { active: true } }),
    mtContract("MT-002", ["MT-001"], { lifecycle: { active: true } }),
  ]);
  assert.throws(
    () => inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID }),
    /multiple contracts are active: MT-001, MT-002/,
  );

  const contractPath = path.join(dir, "MT-002.json");
  const wrongIdentity = JSON.parse(fs.readFileSync(contractPath, "utf8"));
  wrongIdentity.lifecycle.active = false;
  wrongIdentity.wp_id = "WP-WRONG-v1";
  writeJson(contractPath, wrongIdentity);
  assert.throws(
    () => inspectPrecreatedMicrotaskSuite({ wpDir: dir, wpId: WP_ID }),
    /MT-002: identity mismatch/,
  );
}));
