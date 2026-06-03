import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DEFAULT_WP_ID = "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";
const REPORT_SCHEMA_ID = "hsk.a3_inspector_smoke.report@1";

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function parseArgs(argv) {
  const args = { repoRoot: "", report: "", wpId: DEFAULT_WP_ID, help: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (arg === "--repo-root") {
      args.repoRoot = argv[++index] || "";
      continue;
    }
    if (arg === "--report") {
      args.report = argv[++index] || "";
      continue;
    }
    if (arg === "--wp") {
      args.wpId = argv[++index] || "";
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function usage() {
  return [
    "Usage: node .GOV/roles_shared/checks/hbr-inspector-smoke.mjs [--repo-root <path>] [--report <path>] [--wp <wp-id>]",
    "",
    "Runs the MT-033 A.3 inspector-plane cargo smoke and validates its JSONL report.",
  ].join("\n");
}

function scriptRepoRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function artifactRoot(repoRoot) {
  return process.env.HANDSHAKE_ARTIFACT_ROOT
    ? path.resolve(process.env.HANDSHAKE_ARTIFACT_ROOT)
    : path.resolve(repoRoot, "..", "..", "Handshake_Artifacts");
}

function timestampSlug(date = new Date()) {
  return date.toISOString().replace(/[:.]/g, "").replace(/Z$/, "Z");
}

function reportPathFor(repoRoot, injectedReport) {
  if (isNonEmptyString(injectedReport)) return path.resolve(injectedReport);
  return path.join(
    artifactRoot(repoRoot),
    "hbr-inspector-smoke",
    `a3-inspector-smoke-${timestampSlug()}.jsonl`,
  );
}

function readReportRows(reportPath) {
  if (!fs.existsSync(reportPath)) return [];
  return fs.readFileSync(reportPath, "utf8")
    .trim()
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function failureRecord(reason, details = {}) {
  return {
    check: "hbr-inspector-smoke",
    verdict: "FAIL",
    reason,
    ...details,
  };
}

function truncate(value, limit = 8000) {
  const text = String(value || "");
  if (text.length <= limit) return text;
  return `${text.slice(0, limit)}...<truncated>`;
}

function runCargoSmoke(repoRoot, reportPath, wpId) {
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  if (fs.existsSync(reportPath)) fs.rmSync(reportPath, { force: true });
  const artifacts = artifactRoot(repoRoot);
  fs.mkdirSync(artifacts, { recursive: true });

  const env = {
    ...process.env,
    HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
    HANDSHAKE_ARTIFACT_ROOT: artifacts,
    HANDSHAKE_INSPECTOR_SMOKE_REPORT: reportPath,
    HANDSHAKE_INSPECTOR_SMOKE_WP: wpId,
    CARGO_TARGET_DIR: path.join(artifacts, "handshake-cargo-target"),
  };
  return spawnSync("cargo", [
    "test",
    "--manifest-path",
    path.join(repoRoot, "src/backend/handshake_core/Cargo.toml"),
    "-p",
    "handshake_core",
    "--target-dir",
    path.join(artifacts, "handshake-cargo-target"),
    "--no-default-features",
    "--features",
    "runtime-full,inspector",
    "--test",
    "a3_inspector_smoke",
  ], {
    cwd: repoRoot,
    env,
    encoding: "utf8",
  });
}

function validateRows(rows, reportPath) {
  const row = rows.find((candidate) => candidate.schema_id === REPORT_SCHEMA_ID);
  if (!row) {
    return failureRecord("inspector smoke report row missing", { report_path: reportPath });
  }
  if (Number(row.endpoints_passed || 0) < 7) {
    return failureRecord("inspector smoke did not pass all HTTP endpoints", { report_path: reportPath, row });
  }
  if (Number(row.ipc_passed || 0) < 7) {
    return failureRecord("inspector smoke did not pass all IPC-equivalent payloads", { report_path: reportPath, row });
  }
  if (!Array.isArray(row.trace_projection_fields) || row.trace_projection_fields.length < 7) {
    return failureRecord("TraceProjection did not populate all seven fields", { report_path: reportPath, row });
  }
  if (row.replay_drive_success !== true || row.replay_drive_event_type !== "INSPECTOR_REPLAY_DRIVE") {
    return failureRecord("replay-drive dispatch did not emit the expected event receipt", { report_path: reportPath, row });
  }
  if (row.compile_leak_detected !== false) {
    return failureRecord("InspectorReadV1 compile-leak status is not fail-closed", { report_path: reportPath, row });
  }
  return null;
}

export function runCli(argv = process.argv.slice(2)) {
  try {
    const args = parseArgs(argv);
    if (args.help) {
      console.log(usage());
      return 0;
    }

    const repoRoot = path.resolve(args.repoRoot || process.env.HANDSHAKE_ACTIVE_REPO_ROOT || scriptRepoRoot());
    const wpId = args.wpId || DEFAULT_WP_ID;
    const reportPath = reportPathFor(repoRoot, args.report);
    const result = runCargoSmoke(repoRoot, reportPath, wpId);
    if (result.status !== 0) {
      console.error(JSON.stringify(failureRecord("cargo inspector smoke failed", {
        status: result.status,
        error: result.error ? String(result.error.message || result.error) : undefined,
        report_path: reportPath,
        stdout: truncate(result.stdout),
        stderr: truncate(result.stderr),
      })));
      return 2;
    }

    const rows = readReportRows(reportPath);
    const failure = validateRows(rows, reportPath);
    if (failure) {
      console.error(JSON.stringify(failure));
      return 2;
    }

    console.log(`hbr-inspector-smoke ok (report=${reportPath})`);
    return 0;
  } catch (error) {
    console.error(JSON.stringify(failureRecord(error instanceof Error ? error.message : String(error))));
    return 3;
  }
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  return fs.realpathSync.native(path.resolve(process.argv[1]))
    === fs.realpathSync.native(fileURLToPath(import.meta.url));
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
