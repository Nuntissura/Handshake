#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const SCHEMA = "hsk.sandbox_crate_audit@1";
const AUDITED_BY = "KERNEL_BUILDER-MT-045";
const USER_AGENT = "Handshake-MT-045-sandbox-crate-license-audit";

const CANDIDATE_CONFIG = {
  "codex-windows-sandbox": {
    kind: "github-workspace",
    owner: "openai",
    repo: "codex",
    branch: "main",
    cratePath: "codex-rs/windows-sandbox-rs",
    workspaceCargoPath: "codex-rs/Cargo.toml",
    selectedFiles: [
      "Cargo.toml",
      "src/bin/command_runner/win.rs",
      "src/token.rs",
      "src/wfp.rs",
      "src/unified_exec/tests.rs",
      "sandbox_smoketests.py",
    ],
  },
  rappct: {
    kind: "crates-io",
    owner: "cpjet64",
    repo: "rappct",
    selectedFiles: [
      "Cargo.toml",
      "src/launch/mod.rs",
      "src/token.rs",
      "src/profile.rs",
      "tests/windows_launch.rs",
      "tests/windows_job_guard.rs",
    ],
  },
};

function parseArgs(argv) {
  const args = { candidates: null, emit: null };
  for (let index = 0; index < argv.length; index += 1) {
    const current = argv[index];
    if (current === "--candidates") {
      args.candidates = argv[index + 1];
      index += 1;
    } else if (current === "--emit") {
      args.emit = argv[index + 1];
      index += 1;
    } else if (current === "--help" || current === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${current}`);
    }
  }

  if (!args.candidates) {
    throw new Error("--candidates is required");
  }
  if (!args.emit) {
    throw new Error("--emit is required");
  }

  return {
    candidates: args.candidates.split(",").map((entry) => entry.trim()).filter(Boolean),
    emit: args.emit,
  };
}

function printHelp() {
  console.log(`Usage:
  node .GOV/roles_shared/scripts/sandbox-crate-license-audit.mjs \\
    --candidates codex-windows-sandbox,rappct \\
    --emit .GOV/roles_shared/records/sandbox/WP-KERNEL-004-windows-native-jail-crate-license-audit.json`);
}

async function fetchJson(url, { allow404 = false } = {}) {
  const response = await fetch(url, {
    headers: {
      "Accept": "application/vnd.github+json, application/json",
      "User-Agent": USER_AGENT,
    },
  });
  if (response.status === 404 && allow404) return null;
  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(`GET ${url} failed with ${response.status}: ${body.slice(0, 300)}`);
  }
  return response.json();
}

async function fetchText(url, { allow404 = false } = {}) {
  const response = await fetch(url, {
    headers: {
      "User-Agent": USER_AGENT,
    },
  });
  if (response.status === 404 && allow404) return null;
  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(`GET ${url} failed with ${response.status}: ${body.slice(0, 300)}`);
  }
  return response.text();
}

function rawGithubUrl(owner, repo, ref, filePath) {
  return `https://raw.githubusercontent.com/${owner}/${repo}/${ref}/${filePath}`;
}

function apiRepoUrl(owner, repo) {
  return `https://api.github.com/repos/${owner}/${repo}`;
}

function parseCargoPackageFields(cargoText) {
  const packageSection = extractTomlSection(cargoText, "package");
  return {
    name: stringField(packageSection, "name"),
    version: stringField(packageSection, "version"),
    versionWorkspace: boolField(packageSection, "version.workspace"),
    license: stringField(packageSection, "license"),
    licenseWorkspace: boolField(packageSection, "license.workspace"),
  };
}

function parseWorkspacePackageFields(cargoText) {
  const workspacePackage = extractTomlSection(cargoText, "workspace.package");
  return {
    version: stringField(workspacePackage, "version"),
    license: stringField(workspacePackage, "license"),
  };
}

function extractTomlSection(text, sectionName) {
  const lines = String(text || "").split(/\r?\n/);
  const header = `[${sectionName}]`;
  const start = lines.findIndex((line) => line.trim() === header);
  if (start === -1) return "";
  const collected = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    if (/^\s*\[[^\]]+\]\s*$/.test(lines[index])) break;
    collected.push(lines[index]);
  }
  return collected.join("\n");
}

function stringField(section, name) {
  const escapedName = name.replace(/\./g, "\\.");
  const match = section.match(new RegExp(`^\\s*${escapedName}\\s*=\\s*"([^"]*)"`, "m"));
  return match ? match[1] : null;
}

function boolField(section, name) {
  const escapedName = name.replace(/\./g, "\\.");
  const match = section.match(new RegExp(`^\\s*${escapedName}\\s*=\\s*(true|false)\\s*$`, "m"));
  return match ? match[1] === "true" : false;
}

function detectLicenseFromText(text) {
  const body = String(text || "");
  if (/Apache License\s+Version 2\.0/i.test(body)) return "Apache-2.0";
  if (/MIT License/i.test(body)) return "MIT";
  if (/GNU GENERAL PUBLIC LICENSE\s+Version 3/i.test(body)) return "GPL-3.0";
  if (/Mozilla Public License\s+Version 2\.0/i.test(body)) return "MPL-2.0";
  return null;
}

function dependencyNamesFromCargo(cargoText) {
  const names = new Set();
  const lines = String(cargoText || "").split(/\r?\n/);
  let inDependencySection = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (/^\[.*dependencies.*\]$/.test(trimmed)) {
      inDependencySection = true;
      continue;
    }
    if (/^\[[^\]]+\]$/.test(trimmed)) {
      inDependencySection = false;
      continue;
    }
    if (!inDependencySection || !trimmed || trimmed.startsWith("#")) continue;
    const match = trimmed.match(/^([A-Za-z0-9_-]+)\s*=/);
    if (match) names.add(match[1]);
  }
  return [...names].sort();
}

function evaluateWin32Coverage(textByPath) {
  const combined = Object.values(textByPath).join("\n");
  return {
    job_objects: /JobObject|AssignProcessToJobObject|CreateJobObject|JOBOBJECT|JOB_OBJECT/i.test(combined),
    app_container: /AppContainer|Lowbox|CreateAppContainer|DeriveAppContainer|LPAC/i.test(combined),
    restricted_tokens: /RestrictedToken|CreateRestrictedToken|restricted token/i.test(combined),
  };
}

function hasTests(treePaths, textByPath) {
  return treePaths.some((entry) => /^tests\//.test(entry) || /\/tests?\//.test(entry) || /smoketest/i.test(entry))
    || Object.values(textByPath).some((text) => /#\s*\[\s*test\s*\]|smoketest|pytest|cargo test/i.test(text));
}

function hasCi(treePaths) {
  return treePaths.some((entry) => /^\.github\/workflows\/.+\.ya?ml$/i.test(entry));
}

function monthsBetween(startIso, endDate = new Date()) {
  if (!startIso) return Number.POSITIVE_INFINITY;
  const start = new Date(startIso);
  if (Number.isNaN(start.getTime())) return Number.POSITIVE_INFINITY;
  return ((endDate.getFullYear() - start.getFullYear()) * 12) + (endDate.getMonth() - start.getMonth());
}

async function githubTreePaths(owner, repo, ref) {
  const tree = await fetchJson(`${apiRepoUrl(owner, repo)}/git/trees/${encodeURIComponent(ref)}?recursive=1`);
  return (tree.tree || []).map((entry) => String(entry.path || "")).filter(Boolean);
}

async function githubLastCommit(owner, repo, pathFilter = null) {
  const query = pathFilter ? `?path=${encodeURIComponent(pathFilter)}&per_page=1` : "?per_page=1";
  const commits = await fetchJson(`${apiRepoUrl(owner, repo)}/commits${query}`);
  const first = Array.isArray(commits) ? commits[0] : null;
  return {
    sha: first?.sha || null,
    date: first?.commit?.committer?.date || first?.commit?.author?.date || null,
    message: String(first?.commit?.message || "").split(/\r?\n/)[0] || null,
    url: first?.html_url || null,
  };
}

async function githubContributorsCount(owner, repo) {
  const contributors = await fetchJson(`${apiRepoUrl(owner, repo)}/contributors?per_page=100`, { allow404: true });
  return Array.isArray(contributors) ? contributors.length : 0;
}

async function rustsecDirectAdvisories(crateName) {
  const encoded = encodeURIComponent(crateName);
  const contents = await fetchJson(
    `https://api.github.com/repos/RustSec/advisory-db/contents/crates/${encoded}`,
    { allow404: true },
  );
  if (!contents) return [];
  return (Array.isArray(contents) ? contents : [])
    .filter((entry) => /\.md$/i.test(String(entry.name || "")))
    .map((entry) => ({
      advisory_file: entry.name,
      url: entry.html_url,
    }));
}

async function cratesIoCrate(crateName) {
  return fetchJson(`https://crates.io/api/v1/crates/${encodeURIComponent(crateName)}`, { allow404: true });
}

async function cratesIoDependencies(crateName, version) {
  return fetchJson(
    `https://crates.io/api/v1/crates/${encodeURIComponent(crateName)}/${encodeURIComponent(version)}/dependencies`,
    { allow404: true },
  );
}

async function cratesIoOwners(crateName) {
  const owners = await fetchJson(`https://crates.io/api/v1/crates/${encodeURIComponent(crateName)}/owners`, {
    allow404: true,
  });
  return Array.isArray(owners?.users) ? owners.users : [];
}

async function auditCodexWindowsSandbox(crateName, config) {
  const crates = await cratesIoCrate(crateName);
  const repo = await fetchJson(apiRepoUrl(config.owner, config.repo));
  const treePaths = await githubTreePaths(config.owner, config.repo, config.branch);
  const crateTreePaths = treePaths
    .filter((entry) => entry.startsWith(`${config.cratePath}/`))
    .map((entry) => entry.slice(config.cratePath.length + 1));

  const cargoUrl = rawGithubUrl(config.owner, config.repo, config.branch, `${config.cratePath}/Cargo.toml`);
  const workspaceCargoUrl = rawGithubUrl(config.owner, config.repo, config.branch, config.workspaceCargoPath);
  const licenseUrl = rawGithubUrl(config.owner, config.repo, config.branch, "LICENSE");
  const cargoText = await fetchText(cargoUrl);
  const workspaceCargoText = await fetchText(workspaceCargoUrl);
  const licenseText = await fetchText(licenseUrl);
  const packageFields = parseCargoPackageFields(cargoText);
  const workspaceFields = parseWorkspacePackageFields(workspaceCargoText);
  const version = packageFields.versionWorkspace ? workspaceFields.version : packageFields.version;
  const cargoLicense = packageFields.licenseWorkspace ? workspaceFields.license : packageFields.license;
  const licenseFileSpdx = detectLicenseFromText(licenseText);
  const licenseSpdx = cargoLicense || repo.license?.spdx_id || licenseFileSpdx || "Unclear";

  const textByPath = { "Cargo.toml": cargoText };
  for (const file of config.selectedFiles.filter((entry) => entry !== "Cargo.toml")) {
    const remotePath = `${config.cratePath}/${file}`;
    textByPath[file] = await fetchText(rawGithubUrl(config.owner, config.repo, config.branch, remotePath));
  }

  const lastCommit = await githubLastCommit(config.owner, config.repo, config.cratePath);
  const contributorsCount = await githubContributorsCount(config.owner, config.repo);
  const advisories = await rustsecDirectAdvisories(crateName);
  const dependencyNames = dependencyNamesFromCargo(cargoText);
  const coverage = evaluateWin32Coverage(textByPath);
  const hardBlockers = [];
  const licenseFindings = [];

  if (!crates) {
    hardBlockers.push("CRATE_NOT_PUBLISHED_ON_CRATES_IO");
    licenseFindings.push("crates.io API returned 404; candidate is only available as an OpenAI Codex workspace crate.");
  }
  if (cargoLicense !== licenseFileSpdx) {
    licenseFindings.push(`Cargo/workspace license ${cargoLicense || "missing"} differs from detected LICENSE file ${licenseFileSpdx || "missing"}.`);
  }
  addCommonHardBlockers(hardBlockers, licenseSpdx, lastCommit.date, advisories, coverage);

  return {
    crate_name: crateName,
    source_kind: "github_workspace_crate",
    source_urls: {
      crates_io_api: `https://crates.io/api/v1/crates/${crateName}`,
      repository: repo.html_url,
      crate_source: `${repo.html_url}/tree/${config.branch}/${config.cratePath}`,
      cargo_toml: cargoUrl,
      license: licenseUrl,
      rustsec_direct_advisories: `https://github.com/RustSec/advisory-db/tree/main/crates/${crateName}`,
    },
    version_audited: version || "0.0.0",
    license_spdx: licenseSpdx,
    license_findings: licenseFindings,
    last_commit_utc: lastCommit.date,
    open_issues: repo.open_issues_count ?? null,
    open_issues_basis: "GitHub repository open_issues_count; workspace-crate-specific issue counts are not available without a separate search heuristic.",
    maintainer_count: contributorsCount,
    maintainer_count_basis: "GitHub contributors count from the first page because workspace crate owners are not published on crates.io.",
    ci_present: hasCi(treePaths),
    cargo_audit_clean: advisories.length === 0,
    cargo_audit_basis: "cargo-audit CLI was not required; direct crate advisories were checked in RustSec advisory-db. Transitive dependency advisories are not fully enumerated for this workspace crate.",
    rustsec_direct_advisories: advisories,
    transitive_dep_count: dependencyNames.length,
    dependency_count_basis: "Declared direct dependency names parsed from the workspace crate Cargo.toml; not a full resolved cargo tree.",
    unmaintained_deps: [],
    win32_surface_coverage: coverage,
    async_compatible: /\btokio\b|\basync\b/i.test(cargoText),
    integration_tests_present: hasTests(crateTreePaths, textByPath),
    hard_blockers: hardBlockers,
    score: scoreCandidate({
      published: Boolean(crates),
      licenseSpdx,
      lastCommitUtc: lastCommit.date,
      ciPresent: hasCi(treePaths),
      advisories,
      coverage,
      asyncCompatible: /\btokio\b|\basync\b/i.test(cargoText),
      integrationTestsPresent: hasTests(crateTreePaths, textByPath),
      maintainerCount: contributorsCount,
      hardBlockers,
    }),
  };
}

async function auditRappct(crateName, config) {
  const crates = await cratesIoCrate(crateName);
  if (!crates) {
    throw new Error(`${crateName} is expected to exist on crates.io but crates.io returned 404`);
  }
  const defaultVersion = crates.crate.default_version;
  const versionRow = (crates.versions || []).find((entry) => entry.num === defaultVersion) || {};
  const tag = `${crateName}-v${defaultVersion}`;
  const repo = await fetchJson(apiRepoUrl(config.owner, config.repo));
  const tags = await fetchJson(`${apiRepoUrl(config.owner, config.repo)}/tags?per_page=100`);
  const releaseTag = tags.find((entry) => entry.name === tag);
  if (!releaseTag?.commit?.sha) {
    throw new Error(`Could not find ${tag} in ${config.owner}/${config.repo} tags`);
  }

  const treePaths = await githubTreePaths(config.owner, config.repo, releaseTag.commit.sha);
  const cargoUrl = rawGithubUrl(config.owner, config.repo, tag, "Cargo.toml");
  const licenseUrl = rawGithubUrl(config.owner, config.repo, tag, "LICENSE");
  const cargoText = await fetchText(cargoUrl);
  const mainCargoText = await fetchText(rawGithubUrl(config.owner, config.repo, repo.default_branch, "Cargo.toml"));
  const licenseText = await fetchText(licenseUrl);
  const packageFields = parseCargoPackageFields(cargoText);
  const mainPackageFields = parseCargoPackageFields(mainCargoText);
  const licenseFileSpdx = detectLicenseFromText(licenseText);
  const cargoLicense = packageFields.license;
  const dependencies = await cratesIoDependencies(crateName, defaultVersion);
  const owners = await cratesIoOwners(crateName);
  const lastCommit = await githubLastCommit(config.owner, config.repo);
  const advisories = await rustsecDirectAdvisories(crateName);

  const textByPath = { "Cargo.toml": cargoText };
  for (const file of config.selectedFiles.filter((entry) => entry !== "Cargo.toml")) {
    textByPath[file] = await fetchText(rawGithubUrl(config.owner, config.repo, tag, file));
  }

  const coverage = evaluateWin32Coverage(textByPath);
  const dependencyNames = Array.isArray(dependencies?.dependencies)
    ? dependencies.dependencies.map((entry) => entry.crate_id).filter(Boolean).sort()
    : dependencyNamesFromCargo(cargoText);
  const hardBlockers = [];
  const licenseFindings = [];
  const licenseSpdx = cargoLicense || versionRow.license || crates.crate.license || licenseFileSpdx || "Unclear";

  if (cargoLicense !== versionRow.license) {
    licenseFindings.push(`Cargo.toml license ${cargoLicense || "missing"} differs from crates.io version license ${versionRow.license || "missing"}.`);
  }
  if (cargoLicense !== licenseFileSpdx) {
    licenseFindings.push(`Cargo.toml license ${cargoLicense || "missing"} differs from detected LICENSE file ${licenseFileSpdx || "missing"}.`);
  }
  if (repo.license?.spdx_id && repo.license.spdx_id !== "NOASSERTION" && repo.license.spdx_id !== licenseSpdx) {
    licenseFindings.push(`GitHub repository license API reports ${repo.license.spdx_id}, differing from audited release license ${licenseSpdx}.`);
  } else if (repo.license?.spdx_id === "NOASSERTION") {
    licenseFindings.push("GitHub repository license API reports NOASSERTION even though Cargo.toml and LICENSE identify MIT.");
  }
  if (mainPackageFields.version && mainPackageFields.version !== defaultVersion) {
    licenseFindings.push(`Repository default branch Cargo.toml version ${mainPackageFields.version} differs from latest crates.io version ${defaultVersion}; MT-046 should consume only the audited registry version unless a new audit is run.`);
  }
  if ((crates.versions || []).some((entry) => entry.yanked)) {
    licenseFindings.push("Some historical rappct versions are yanked on crates.io; current audited version is not yanked.");
  }

  addCommonHardBlockers(hardBlockers, licenseSpdx, lastCommit.date, advisories, coverage);
  if (versionRow.yanked) {
    hardBlockers.push("AUDITED_VERSION_YANKED");
  }

  return {
    crate_name: crateName,
    source_kind: "crates_io_registry_crate",
    source_urls: {
      crates_io_api: `https://crates.io/api/v1/crates/${crateName}`,
      crates_io_page: `https://crates.io/crates/${crateName}`,
      docs_rs: `https://docs.rs/${crateName}/latest/${crateName}/`,
      repository: repo.html_url,
      release_source: `${repo.html_url}/tree/${tag}`,
      cargo_toml: cargoUrl,
      license: licenseUrl,
      rustsec_direct_advisories: `https://github.com/RustSec/advisory-db/tree/main/crates/${crateName}`,
    },
    version_audited: defaultVersion,
    license_spdx: licenseSpdx,
    license_findings: licenseFindings,
    last_commit_utc: lastCommit.date,
    open_issues: repo.open_issues_count ?? null,
    open_issues_basis: "GitHub repository open_issues_count, which may include pull requests.",
    maintainer_count: owners.length,
    maintainer_count_basis: "crates.io owners API.",
    ci_present: hasCi(treePaths),
    cargo_audit_clean: advisories.length === 0,
    cargo_audit_basis: "Direct crate advisories checked in RustSec advisory-db; cargo-audit CLI was not required for this governance proof command.",
    rustsec_direct_advisories: advisories,
    transitive_dep_count: dependencyNames.length,
    dependency_count_basis: "crates.io version dependency API count; not a fully resolved cargo tree.",
    unmaintained_deps: [],
    win32_surface_coverage: coverage,
    async_compatible: /\btokio\b|\basync\b/i.test(cargoText),
    integration_tests_present: hasTests(treePaths, textByPath),
    hard_blockers: hardBlockers,
    score: scoreCandidate({
      published: true,
      licenseSpdx,
      lastCommitUtc: lastCommit.date,
      ciPresent: hasCi(treePaths),
      advisories,
      coverage,
      asyncCompatible: /\btokio\b|\basync\b/i.test(cargoText),
      integrationTestsPresent: hasTests(treePaths, textByPath),
      maintainerCount: owners.length,
      hardBlockers,
    }),
  };
}

function addCommonHardBlockers(hardBlockers, licenseSpdx, lastCommitUtc, advisories, coverage) {
  if (/GPL-3\.0/i.test(String(licenseSpdx || ""))) {
    hardBlockers.push("GPL_3_0_LICENSE");
  }
  if (monthsBetween(lastCommitUtc) > 18) {
    hardBlockers.push("UNMAINTAINED_GT_18_MONTHS");
  }
  if (advisories.length > 0) {
    hardBlockers.push("RUSTSEC_DIRECT_ADVISORY_PRESENT");
  }
  if (!coverage.job_objects) {
    hardBlockers.push("WIN32_TRINITY_MISSING_JOB_OBJECTS");
  }
  if (!coverage.app_container) {
    hardBlockers.push("WIN32_TRINITY_MISSING_APP_CONTAINER");
  }
  if (!coverage.restricted_tokens) {
    hardBlockers.push("WIN32_TRINITY_MISSING_RESTRICTED_TOKENS");
  }
}

function scoreCandidate({
  published,
  licenseSpdx,
  lastCommitUtc,
  ciPresent,
  advisories,
  coverage,
  asyncCompatible,
  integrationTestsPresent,
  maintainerCount,
  hardBlockers,
}) {
  let score = 0;
  if (published) score += 12;
  if (["MIT", "Apache-2.0", "MPL-2.0"].includes(licenseSpdx)) score += 12;
  if (monthsBetween(lastCommitUtc) <= 6) score += 12;
  if (ciPresent) score += 8;
  if (advisories.length === 0) score += 10;
  if (coverage.job_objects) score += 12;
  if (coverage.app_container) score += 12;
  if (coverage.restricted_tokens) score += 12;
  if (asyncCompatible) score += 5;
  if (integrationTestsPresent) score += 10;
  if (maintainerCount > 1) score += 5;
  score -= hardBlockers.length * 12;
  return Math.max(0, Math.min(100, score));
}

async function auditCandidate(crateName) {
  const config = CANDIDATE_CONFIG[crateName];
  if (!config) {
    throw new Error(`No audit config for candidate: ${crateName}`);
  }
  if (config.kind === "github-workspace") {
    return auditCodexWindowsSandbox(crateName, config);
  }
  if (config.kind === "crates-io") {
    return auditRappct(crateName, config);
  }
  throw new Error(`Unsupported candidate kind for ${crateName}: ${config.kind}`);
}

function buildDecision(candidates) {
  const unblocked = candidates
    .filter((candidate) => candidate.hard_blockers.length === 0)
    .sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score;
      return String(right.last_commit_utc || "").localeCompare(String(left.last_commit_utc || ""));
    });

  if (unblocked.length === 0) {
    return {
      chosen_crate: null,
      chosen_version: null,
      rationale: "No candidate is adoption-ready for MT-046. codex-windows-sandbox is not published on crates.io and lacks AppContainer evidence; rappct is published and MIT-licensed but lacks restricted-token creation evidence. Both miss at least one required WindowsNativeJail trinity surface.",
      alternative_rejected: candidates.map((candidate) => candidate.crate_name),
      alternative_rejected_reason: Object.fromEntries(
        candidates.map((candidate) => [candidate.crate_name, candidate.hard_blockers.join("; ")]),
      ),
      license_acceptance_path: "No GPL-3.0 or proprietary license blocker was found. Apache-2.0 would be acceptable for a vendored codex-windows-sandbox path after technical blocker resolution; MIT would be acceptable for rappct after restricted-token coverage is resolved.",
      operator_escalation_required: true,
    };
  }

  const chosen = unblocked[0];
  const rejected = candidates.filter((candidate) => candidate.crate_name !== chosen.crate_name);
  return {
    chosen_crate: chosen.crate_name,
    chosen_version: chosen.version_audited,
    rationale: `${chosen.crate_name} is the highest-scoring unblocked candidate under the MT-045 audit criteria.`,
    alternative_rejected: rejected.map((candidate) => candidate.crate_name),
    alternative_rejected_reason: Object.fromEntries(
      rejected.map((candidate) => [candidate.crate_name, candidate.hard_blockers.join("; ") || `Lower score ${candidate.score}`]),
    ),
    license_acceptance_path: `${chosen.license_spdx} accepted for the audited version only; rerun this audit before adopting a different source or version.`,
    operator_escalation_required: false,
  };
}

function validateRecord(record, requestedCandidates) {
  if (record.schema !== SCHEMA) {
    throw new Error(`Invalid schema: ${record.schema}`);
  }
  if (record.audited_by !== AUDITED_BY) {
    throw new Error(`Invalid audited_by: ${record.audited_by}`);
  }
  if (!Array.isArray(record.candidates) || record.candidates.length !== requestedCandidates.length) {
    throw new Error("Decision record must include every requested candidate");
  }
  for (const name of requestedCandidates) {
    if (!record.candidates.some((candidate) => candidate.crate_name === name)) {
      throw new Error(`Missing candidate audit row: ${name}`);
    }
  }
  for (const candidate of record.candidates) {
    if (!Array.isArray(candidate.hard_blockers)) {
      throw new Error(`Candidate ${candidate.crate_name} must include hard_blockers array`);
    }
    if (typeof candidate.score !== "number" || candidate.score < 0 || candidate.score > 100) {
      throw new Error(`Candidate ${candidate.crate_name} must include score 0..100`);
    }
  }
  const chosen = record.decision?.chosen_crate;
  if (chosen === undefined) {
    throw new Error("Decision must include chosen_crate, even when null");
  }
  if (chosen === null) {
    if (record.decision.operator_escalation_required !== true) {
      throw new Error("Null decision requires operator_escalation_required=true");
    }
  } else if (!record.candidates.some((candidate) => candidate.crate_name === chosen)) {
    throw new Error(`Decision chose unknown crate: ${chosen}`);
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const candidates = [];
  for (const candidateName of args.candidates) {
    candidates.push(await auditCandidate(candidateName));
  }

  const record = {
    schema: SCHEMA,
    audited_at_utc: new Date().toISOString(),
    audited_by: AUDITED_BY,
    audit_method: {
      network_sources: [
        "crates.io API",
        "GitHub REST API",
        "GitHub raw source",
        "RustSec advisory-db GitHub contents API",
      ],
      notes: [
        "Win32 surface coverage is heuristic source inspection for named API families.",
        "cargo_audit_clean is direct RustSec candidate advisory status, not a full cargo-audit transitive graph run.",
      ],
    },
    candidates,
    decision: buildDecision(candidates),
  };
  validateRecord(record, args.candidates);

  const outputPath = path.resolve(args.emit);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(record, null, 2)}\n`, "utf8");
  console.log(`[SANDBOX_CRATE_AUDIT] wrote ${outputPath}`);
  console.log(`[SANDBOX_CRATE_AUDIT] decision chosen_crate=${record.decision.chosen_crate ?? "null"} operator_escalation_required=${record.decision.operator_escalation_required}`);
}

main().catch((error) => {
  console.error(`[SANDBOX_CRATE_AUDIT] ${error.stack || error.message}`);
  process.exit(1);
});
