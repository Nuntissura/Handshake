import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { defaultIntegrationValidatorWorktreeDir } from "../session/session-policy.mjs";

function parseSingleField(packetText, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(packetText || "").match(re);
  return match ? match[1].trim() : "";
}

function stripTicks(value) {
  return String(value || "").trim().replace(/^`|`$/g, "");
}

function normalizeRepoRelativePath(value) {
  return stripTicks(value).replace(/\\/g, "/");
}

function parseIntField(value) {
  const parsed = Number.parseInt(String(value || "").trim(), 10);
  return Number.isFinite(parsed) ? parsed : null;
}

export function parseSignedScopeValidationEntries(packetText) {
  const lines = String(packetText || "").split(/\r?\n/);
  const entries = [];
  let current = null;

  function flush() {
    if (!current?.filePath) return;
    entries.push(current);
    current = null;
  }

  for (const line of lines) {
    const fileMatch = line.match(/^\s*-\s+\*\*Target File\*\*:\s*`?([^`]+?)`?\s*$/i);
    if (fileMatch) {
      flush();
      current = {
        filePath: normalizeRepoRelativePath(fileMatch[1]),
        start: null,
        end: null,
        lineDelta: null,
      };
      continue;
    }
    if (!current) continue;

    const startMatch = line.match(/^\s*-\s+\*\*Start\*\*:\s*`?([^`]+?)`?\s*$/i);
    if (startMatch) {
      current.start = parseIntField(startMatch[1]);
      continue;
    }

    const endMatch = line.match(/^\s*-\s+\*\*End\*\*:\s*`?([^`]+?)`?\s*$/i);
    if (endMatch) {
      current.end = parseIntField(endMatch[1]);
      continue;
    }

    const deltaMatch = line.match(/^\s*-\s+\*\*Line Delta\*\*:\s*`?([^`]+?)`?\s*$/i);
    if (deltaMatch) {
      current.lineDelta = parseIntField(deltaMatch[1]);
      continue;
    }
  }

  flush();
  return entries;
}

export function parseSignedScopePatchArtifacts(packetText) {
  const artifacts = new Set();
  const matches = String(packetText || "").matchAll(/^\s*-\s+\*\*Artifacts\*\*:\s*(.+)\s*$/gmi);
  for (const match of matches) {
    const rawValue = String(match[1] || "");
    const patchRefs = rawValue.match(/`([^`]+?\.patch)`/gi) || [];
    for (const ref of patchRefs) {
      artifacts.add(normalizeRepoRelativePath(ref.slice(1, -1)));
    }
  }
  return Array.from(artifacts);
}

export function parseUnifiedDiffSummary(diffText) {
  const files = new Map();
  let currentFilePath = "";
  let current = null;

  function ensureFile(filePath) {
    const normalized = normalizeRepoRelativePath(filePath);
    if (!files.has(normalized)) {
      files.set(normalized, {
        filePath: normalized,
        lineDelta: 0,
        hunks: [],
      });
    }
    currentFilePath = normalized;
    current = files.get(normalized);
  }

  const lines = String(diffText || "").split(/\r?\n/);
  for (const line of lines) {
    const diffMatch = line.match(/^diff --git a\/(.+?) b\/(.+)$/);
    if (diffMatch) {
      ensureFile(diffMatch[2]);
      continue;
    }

    const newFileMatch = line.match(/^\+\+\+ b\/(.+)$/);
    if (newFileMatch) {
      ensureFile(newFileMatch[1]);
      continue;
    }

    if (!current) continue;

    if (line.startsWith("@@")) {
      const hunkMatch = line.match(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/);
      if (!hunkMatch) continue;
      const oldStart = Number.parseInt(hunkMatch[1], 10);
      const oldCount = Number.parseInt(hunkMatch[2] || "1", 10);
      const newStart = Number.parseInt(hunkMatch[3], 10);
      const newCount = Number.parseInt(hunkMatch[4] || "1", 10);
      const oldEnd = oldCount === 0 ? oldStart : oldStart + oldCount - 1;
      const newEnd = newCount === 0 ? newStart : newStart + newCount - 1;
      current.hunks.push({
        oldStart,
        oldCount,
        oldEnd,
        newStart,
        newCount,
        newEnd,
        touchedStart: Math.min(oldStart, newStart),
        touchedEnd: Math.max(oldEnd, newEnd),
      });
      continue;
    }

    if ((line.startsWith("+") || line.startsWith("-")) && !line.startsWith("+++") && !line.startsWith("---")) {
      current.lineDelta += 1;
    }
  }

  return {
    files: Array.from(files.values()),
    normalizedDiff: normalizeUnifiedDiff(diffText),
  };
}

export function normalizeUnifiedDiff(diffText) {
  const normalizedLines = [];
  for (const line of String(diffText || "").split(/\r?\n/)) {
    if (/^diff --git a\/.+ b\/.+$/.test(line)) {
      normalizedLines.push(line.replace(/\\/g, "/"));
      continue;
    }
    if (/^(---|\+\+\+) /.test(line)) {
      normalizedLines.push(line.replace(/\\/g, "/"));
      continue;
    }
    if (line.startsWith("@@")) {
      normalizedLines.push("@@");
      continue;
    }
    if ((line.startsWith("+") || line.startsWith("-")) && !line.startsWith("+++") && !line.startsWith("---")) {
      normalizedLines.push(line);
    }
  }
  return normalizedLines.join("\n").trim();
}

function compareSummaryAgainstDeclaredSurface(summary, declaredEntries, label) {
  const errors = [];
  const declaredByFile = new Map(declaredEntries.map((entry) => [entry.filePath, entry]));
  const summaryByFile = new Map((summary?.files || []).map((entry) => [entry.filePath, entry]));

  for (const [filePath, declared] of declaredByFile.entries()) {
    const actual = summaryByFile.get(filePath);
    if (!actual) {
      errors.push(`${label}: missing diff for declared file ${filePath}`);
      continue;
    }
    if (Number.isFinite(declared.lineDelta) && actual.lineDelta !== declared.lineDelta) {
      errors.push(
        `${label}: ${filePath} line delta ${actual.lineDelta} does not match declared ${declared.lineDelta}`,
      );
    }
    for (const hunk of actual.hunks) {
      if (Number.isFinite(declared.start) && hunk.touchedStart < declared.start) {
        errors.push(
          `${label}: ${filePath} hunk starts at ${hunk.touchedStart}, before declared window start ${declared.start}`,
        );
      }
      if (Number.isFinite(declared.end) && hunk.touchedEnd > declared.end) {
        errors.push(
          `${label}: ${filePath} hunk ends at ${hunk.touchedEnd}, after declared window end ${declared.end}`,
        );
      }
    }
  }

  for (const filePath of summaryByFile.keys()) {
    if (!declaredByFile.has(filePath)) {
      errors.push(`${label}: undeclared file changed in signed surface ${filePath}`);
    }
  }

  return errors;
}

function defaultGitRunner(worktreeAbs, args) {
  const result = spawnSync("git", args, {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    code: typeof result.status === "number" ? result.status : 1,
    output: `${result.stdout || ""}${result.stderr || ""}`,
  };
}

function resolveMainWorktreeAbs(packetText, repoRoot) {
  const declared = parseSingleField(packetText, "INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR")
    || defaultIntegrationValidatorWorktreeDir("");
  return path.resolve(repoRoot || process.cwd(), declared);
}

function readContainedCommitDiff({ packetText, repoRoot, mergedMainCommit, gitRunner }) {
  const mainWorktreeAbs = resolveMainWorktreeAbs(packetText, repoRoot);
  const runGit = typeof gitRunner === "function"
    ? gitRunner
    : (args) => defaultGitRunner(mainWorktreeAbs, args);

  const parentResult = runGit(["rev-list", "--parents", "-n", "1", mergedMainCommit]);
  if (parentResult.code !== 0) {
    return {
      ok: false,
      error: `cannot resolve parents for merged commit ${mergedMainCommit}`,
      mainWorktreeAbs,
    };
  }
  const parentTokens = String(parentResult.output || "").trim().split(/\s+/).filter(Boolean);
  if (parentTokens.length < 2) {
    return {
      ok: false,
      error: `merged commit ${mergedMainCommit} has no parent to diff against`,
      mainWorktreeAbs,
    };
  }
  const parentSha = parentTokens[1];
  const diffResult = runGit(["diff", "--unified=0", "--no-ext-diff", parentSha, mergedMainCommit]);
  if (diffResult.code !== 0) {
    return {
      ok: false,
      error: `cannot read containment diff for ${mergedMainCommit}`,
      mainWorktreeAbs,
    };
  }
  return {
    ok: true,
    diffText: String(diffResult.output || ""),
    parentSha,
    mainWorktreeAbs,
  };
}

function readCandidateTargetDiff({
  packetText,
  repoRoot,
  targetHeadSha,
  currentMainHeadSha,
  gitRunner,
}) {
  const mainWorktreeAbs = resolveMainWorktreeAbs(packetText, repoRoot);
  const runGit = typeof gitRunner === "function"
    ? gitRunner
    : (args) => defaultGitRunner(mainWorktreeAbs, args);

  const mergeBaseResult = runGit(["merge-base", currentMainHeadSha, targetHeadSha]);
  if (mergeBaseResult.code !== 0) {
    return {
      ok: false,
      error: `cannot resolve merge-base between local main ${currentMainHeadSha} and target ${targetHeadSha}`,
      mainWorktreeAbs,
    };
  }
  const mergeBaseSha = String(mergeBaseResult.output || "").trim();
  if (!mergeBaseSha) {
    return {
      ok: false,
      error: `merge-base between local main ${currentMainHeadSha} and target ${targetHeadSha} is empty`,
      mainWorktreeAbs,
    };
  }

  const diffResult = runGit(["diff", "--unified=0", "--no-ext-diff", mergeBaseSha, targetHeadSha]);
  if (diffResult.code !== 0) {
    return {
      ok: false,
      error: `cannot read candidate target diff for ${targetHeadSha}`,
      mainWorktreeAbs,
    };
  }

  return {
    ok: true,
    diffText: String(diffResult.output || ""),
    mergeBaseSha,
    mainWorktreeAbs,
  };
}

export function validateSignedScopeSurface(packetText, {
  repoRoot = process.cwd(),
  artifactText = null,
} = {}) {
  const errors = [];
  const declaredEntries = parseSignedScopeValidationEntries(packetText);
  if (declaredEntries.length === 0) {
    errors.push("signed scope surface is missing VALIDATION target file declarations");
    return { ok: false, errors, declaredEntries, patchArtifactPath: "", artifactSummary: null };
  }

  const patchArtifacts = parseSignedScopePatchArtifacts(packetText);
  if (patchArtifacts.length !== 1) {
    errors.push(
      `signed scope surface requires exactly one unique patch artifact reference (found ${patchArtifacts.length})`,
    );
    return { ok: false, errors, declaredEntries, patchArtifactPath: patchArtifacts[0] || "", artifactSummary: null };
  }

  const patchArtifactPath = path.resolve(repoRoot, patchArtifacts[0]);
  if (artifactText == null && !fs.existsSync(patchArtifactPath)) {
    errors.push(`signed scope patch artifact is missing: ${patchArtifactPath.replace(/\\/g, "/")}`);
    return { ok: false, errors, declaredEntries, patchArtifactPath, artifactSummary: null };
  }

  const patchText = artifactText == null ? fs.readFileSync(patchArtifactPath, "utf8") : String(artifactText || "");
  const artifactSummary = parseUnifiedDiffSummary(patchText);
  errors.push(...compareSummaryAgainstDeclaredSurface(artifactSummary, declaredEntries, "signed patch artifact"));

  return {
    ok: errors.length === 0,
    errors,
    declaredEntries,
    patchArtifactPath,
    artifactSummary,
    artifactNormalizedDiff: artifactSummary.normalizedDiff,
  };
}

export function validateContainedMainCommitAgainstSignedScope(packetText, {
  repoRoot = process.cwd(),
  mergedMainCommit = "",
  actualDiffText = null,
  gitRunner = null,
} = {}) {
  const surface = validateSignedScopeSurface(packetText, { repoRoot });
  const errors = [...surface.errors];
  if (!mergedMainCommit) {
    errors.push("contained main commit validation requires MERGED_MAIN_COMMIT");
    return {
      ok: false,
      errors,
      surface,
      actualSummary: null,
    };
  }

  let diffText = String(actualDiffText || "");
  let parentSha = "";
  let mainWorktreeAbs = resolveMainWorktreeAbs(packetText, repoRoot);
  if (actualDiffText == null) {
    const diffResult = readContainedCommitDiff({
      packetText,
      repoRoot,
      mergedMainCommit,
      gitRunner,
    });
    mainWorktreeAbs = diffResult.mainWorktreeAbs || mainWorktreeAbs;
    if (!diffResult.ok) {
      errors.push(diffResult.error);
      return {
        ok: false,
        errors,
        surface,
        actualSummary: null,
        mainWorktreeAbs,
      };
    }
    diffText = diffResult.diffText;
    parentSha = diffResult.parentSha;
  }

  const actualSummary = parseUnifiedDiffSummary(diffText);
  errors.push(...compareSummaryAgainstDeclaredSurface(actualSummary, surface.declaredEntries, "contained main diff"));

  if (surface.ok && actualSummary.normalizedDiff !== surface.artifactNormalizedDiff) {
    errors.push(
      "contained main diff does not match the signed patch artifact after normalization; merged scope drifted from the clean-room proof surface",
    );
  }

  return {
    ok: errors.length === 0,
    errors,
    surface,
    actualSummary,
    parentSha,
    mainWorktreeAbs,
  };
}

export function validateCandidateTargetAgainstSignedScope(packetText, {
  repoRoot = process.cwd(),
  targetHeadSha = "",
  currentMainHeadSha = "",
  candidateDiffText = null,
  gitRunner = null,
} = {}) {
  const surface = validateSignedScopeSurface(packetText, { repoRoot });
  const errors = [...surface.errors];
  if (!targetHeadSha) {
    errors.push("candidate target validation requires committed target_head_sha");
    return {
      ok: false,
      errors,
      surface,
      actualSummary: null,
    };
  }
  if (!currentMainHeadSha) {
    errors.push("candidate target validation requires current local main HEAD");
    return {
      ok: false,
      errors,
      surface,
      actualSummary: null,
    };
  }

  let diffText = String(candidateDiffText || "");
  let mergeBaseSha = "";
  let mainWorktreeAbs = resolveMainWorktreeAbs(packetText, repoRoot);
  if (candidateDiffText == null) {
    const diffResult = readCandidateTargetDiff({
      packetText,
      repoRoot,
      targetHeadSha,
      currentMainHeadSha,
      gitRunner,
    });
    mainWorktreeAbs = diffResult.mainWorktreeAbs || mainWorktreeAbs;
    if (!diffResult.ok) {
      errors.push(diffResult.error);
      return {
        ok: false,
        errors,
        surface,
        actualSummary: null,
        mainWorktreeAbs,
      };
    }
    diffText = diffResult.diffText;
    mergeBaseSha = diffResult.mergeBaseSha;
  }

  const actualSummary = parseUnifiedDiffSummary(diffText);
  errors.push(...compareSummaryAgainstDeclaredSurface(actualSummary, surface.declaredEntries, "candidate target diff"));

  if (surface.ok && actualSummary.normalizedDiff !== surface.artifactNormalizedDiff) {
    errors.push(
      "candidate target diff does not match the signed patch artifact after normalization; committed target drifted from the clean-room proof surface",
    );
  }

  return {
    ok: errors.length === 0,
    errors,
    surface,
    actualSummary,
    mergeBaseSha,
    mainWorktreeAbs,
  };
}
