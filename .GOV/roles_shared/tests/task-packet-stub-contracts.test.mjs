import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  readStubContractForMarkdownPath,
  stubContractPathFromMarkdownPath,
  stubMarkdownPathFromContractPath,
  validateHandAuthoredStubContract,
} from "../scripts/wp/task-packet-stub-contracts.mjs";

// --- pure path helpers ------------------------------------------------------

test("stubContractPathFromMarkdownPath swaps .md -> .contract.json", () => {
  assert.equal(
    stubContractPathFromMarkdownPath(".GOV/task_packets/stubs/WP-1-Foo-v1.md"),
    ".GOV/task_packets/stubs/WP-1-Foo-v1.contract.json",
  );
});

test("stubMarkdownPathFromContractPath swaps .contract.json -> .md", () => {
  assert.equal(
    stubMarkdownPathFromContractPath(
      ".GOV/task_packets/stubs/WP-1-Foo-v1.contract.json",
    ),
    ".GOV/task_packets/stubs/WP-1-Foo-v1.md",
  );
});

// --- readStubContractForMarkdownPath path-suffix-agnostic ------------------

test("readStubContractForMarkdownPath accepts a .md path and reads the .contract.json sibling", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "stub-contracts-"));
  try {
    const contractPath = path.join(tempRoot, "WP-1-Test-v1.contract.json");
    const payload = { schema_id: "hsk.work_packet_stub_contract@1", wp_id: "WP-1-Test-v1" };
    fs.writeFileSync(contractPath, JSON.stringify(payload), "utf8");
    const result = readStubContractForMarkdownPath(
      path.join(tempRoot, "WP-1-Test-v1.md"),
    );
    assert.equal(result.ok, true);
    assert.equal(result.contract.wp_id, "WP-1-Test-v1");
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("readStubContractForMarkdownPath accepts a .contract.json path directly", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "stub-contracts-"));
  try {
    const contractPath = path.join(tempRoot, "WP-1-Test-v1.contract.json");
    const payload = { schema_id: "hsk.work_packet_stub_contract@1", wp_id: "WP-1-Test-v1" };
    fs.writeFileSync(contractPath, JSON.stringify(payload), "utf8");
    const result = readStubContractForMarkdownPath(contractPath);
    assert.equal(result.ok, true);
    assert.equal(result.contract.wp_id, "WP-1-Test-v1");
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("readStubContractForMarkdownPath returns MISSING when contract is absent", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "stub-contracts-"));
  try {
    const result = readStubContractForMarkdownPath(
      path.join(tempRoot, "WP-1-Nope-v1.md"),
    );
    assert.equal(result.ok, false);
    assert.equal(result.source, "MISSING");
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

// --- validateHandAuthoredStubContract --------------------------------------

function validContract() {
  return {
    schema_id: "hsk.work_packet_stub_contract@1",
    contract_authority: "PRIMARY_MACHINE_READABLE_STUB",
    artifact_policy: { authority_surface: "MACHINE_CONTRACT" },
    wp_id: "WP-1-Test-v1",
    base_wp_id: "WP-1-Test",
    lifecycle: { status: "STUB (NOT READY FOR DEV)" },
    build_order: {
      domain: "CROSS_BOUNDARY",
      tech_blocker: "NO",
      value_tier: "HIGH",
      risk_tier: "LOW",
      depends_on: [],
      blocks: [],
    },
    spec_trace: {
      roadmap_pointer: "Test brief Week 0 Test slot",
    },
    activation_contract: { may_start_coder: false, may_start_validator: false },
  };
}

test("validateHandAuthoredStubContract returns no failures for a valid contract", () => {
  const failures = validateHandAuthoredStubContract(validContract(), "test-path");
  assert.deepEqual(failures, []);
});

test("validateHandAuthoredStubContract rejects wrong schema_id", () => {
  const c = validContract();
  c.schema_id = "wrong";
  const failures = validateHandAuthoredStubContract(c, "test-path");
  assert.ok(failures.some((f) => f.includes("schema_id must equal")));
});

test("validateHandAuthoredStubContract rejects wrong contract_authority", () => {
  const c = validContract();
  c.contract_authority = "GENERATED_FROM_MARKDOWN";
  const failures = validateHandAuthoredStubContract(c, "test-path");
  assert.ok(failures.some((f) => f.includes("contract_authority must equal")));
});

test("validateHandAuthoredStubContract rejects wrong artifact_policy.authority_surface", () => {
  const c = validContract();
  c.artifact_policy = { authority_surface: "MARKDOWN_AUTHORED" };
  const failures = validateHandAuthoredStubContract(c, "test-path");
  assert.ok(failures.some((f) => f.includes("authority_surface must equal")));
});

test("validateHandAuthoredStubContract rejects empty wp_id or non-WP- prefix", () => {
  const empty = validContract();
  empty.wp_id = "";
  assert.ok(
    validateHandAuthoredStubContract(empty, "p").some((f) => f.includes("wp_id")),
  );
  const wrongPrefix = validContract();
  wrongPrefix.wp_id = "Test-1";
  assert.ok(
    validateHandAuthoredStubContract(wrongPrefix, "p").some((f) => f.includes("wp_id")),
  );
});

test("validateHandAuthoredStubContract rejects missing lifecycle.status", () => {
  const c = validContract();
  c.lifecycle = {};
  const failures = validateHandAuthoredStubContract(c, "p");
  assert.ok(failures.some((f) => f.includes("lifecycle.status")));
});

test("validateHandAuthoredStubContract rejects missing build_order fields", () => {
  for (const field of ["domain", "tech_blocker", "value_tier", "risk_tier"]) {
    const c = validContract();
    c.build_order[field] = "";
    const failures = validateHandAuthoredStubContract(c, "p");
    assert.ok(
      failures.some((f) => f.includes(`build_order.${field}`)),
      `expected failure for build_order.${field}`,
    );
  }
});

test("validateHandAuthoredStubContract rejects non-array build_order.depends_on / blocks", () => {
  const c = validContract();
  c.build_order.depends_on = "not-an-array";
  const failures = validateHandAuthoredStubContract(c, "p");
  assert.ok(failures.some((f) => f.includes("build_order.depends_on")));
});

test("validateHandAuthoredStubContract rejects empty spec_trace.roadmap_pointer", () => {
  const c = validContract();
  c.spec_trace = {};
  const failures = validateHandAuthoredStubContract(c, "p");
  assert.ok(failures.some((f) => f.includes("roadmap_pointer")));
});

test("validateHandAuthoredStubContract rejects activation_contract that permits launching coder or validator", () => {
  const cCoder = validContract();
  cCoder.activation_contract.may_start_coder = true;
  assert.ok(
    validateHandAuthoredStubContract(cCoder, "p").some((f) =>
      f.includes("may_start_coder must be false"),
    ),
  );
  const cValidator = validContract();
  cValidator.activation_contract.may_start_validator = true;
  assert.ok(
    validateHandAuthoredStubContract(cValidator, "p").some((f) =>
      f.includes("may_start_validator must be false"),
    ),
  );
});

test("validateHandAuthoredStubContract returns a clear failure when input is not an object", () => {
  const failures = validateHandAuthoredStubContract(null, "p");
  assert.ok(failures.length > 0);
  assert.ok(failures[0].includes("contract is not an object"));
});
