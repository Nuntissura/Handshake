import fs from "node:fs";
import path from "node:path";
import { execSync, spawnSync } from "node:child_process";

const MAX_BUFFER = 1024 * 1024 * 20;
const MAX_DIFF_CHARS = 200000;
const MAX_FILE_CHARS = 60000;
const MAX_TOTAL_FILE_CHARS = 400000;
const GEMINI_TIMEOUT_MS = Number.parseInt(process.env.GEMINI_TIMEOUT_MS ?? "300000", 10);

function resolveGeminiCommand() {
  const configured = process.env.GEMINI_CLI;
  if (configured) {
    const ext = path.extname(configured).toLowerCase();
    if (ext === ".ps1") {
      return {
        command: "powershell",
        args: ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", configured],
      };
    }
    return { command: configured, args: [] };
  }
  if (process.platform === "win32") {
    const appData = process.env.APPDATA;
    if (appData) {
      const ps1 = path.join(appData, "npm", "gemini.ps1");
      if (fs.existsSync(ps1)) {
        return {
          command: "powershell",
          args: ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", ps1],
        };
      }
    }
    return { command: "gemini.cmd", args: [] };
  }
  return { command: "gemini", args: [] };
}

function run(command) {
  try {
    return execSync(command, { stdio: "pipe", maxBuffer: MAX_BUFFER }).toString().trim();
  } catch {
    return "";
  }
}

function readFileSafe(filePath) {
  if (!fs.existsSync(filePath)) {
    return "";
  }
  return fs.readFileSync(filePath, "utf8");
}

function readFileForPrompt(filePath) {
  if (!fs.existsSync(filePath)) {
    return "<missing file>";
  }
  const buffer = fs.readFileSync(filePath);
  if (buffer.includes(0)) {
    return "<binary file omitted>";
  }
  let text = buffer.toString("utf8");
  if (text.length > MAX_FILE_CHARS) {
    text = `${text.slice(0, MAX_FILE_CHARS)}\n<TRUNCATED FILE CONTENT>`;
  }
  return text;
}

function withLineNumbers(content) {
  return content
    .split("\n")
    .map((line, index) => `${index + 1}: ${line}`)
    .join("\n");
}

function extractJson(text) {
  const trimmed = text.trim();
  if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
    return trimmed;
  }
  const fenced = trimmed.match(/```json([\s\S]*?)```/i);
  if (fenced) {
    return fenced[1].trim();
  }
  const first = trimmed.indexOf("{");
  const last = trimmed.lastIndexOf("}");
  if (first !== -1 && last !== -1 && last > first) {
    return trimmed.slice(first, last + 1).trim();
  }
  return "";
}

function resolveShas() {
  let baseSha = process.env.BASE_SHA;
  let headSha = process.env.HEAD_SHA;
  const eventPath = process.env.GITHUB_EVENT_PATH;

  if ((!baseSha || !headSha) && eventPath && fs.existsSync(eventPath)) {
    const event = JSON.parse(fs.readFileSync(eventPath, "utf8"));
    baseSha = baseSha ?? event.pull_request?.base?.sha;
    headSha = headSha ?? event.pull_request?.head?.sha;
  }

  if (!headSha) {
    headSha = run("git rev-parse HEAD");
  }

  if (!baseSha) {
    const revs = run("git rev-list --max-count=2 HEAD");
    const commits = revs ? revs.split("\n") : [];
    baseSha = commits[1] ?? headSha;
  }

  return { baseSha, headSha };
}

function runGeminiCli(prompt, commandInfo) {
  const args = [...commandInfo.args, "--output-format", "text"];
  const model = process.env.GEMINI_MODEL;
  if (model) {
    args.push("--model", model);
  }

  const result = spawnSync(commandInfo.command, args, {
    input: prompt,
    encoding: "utf8",
    maxBuffer: MAX_BUFFER,
    timeout: GEMINI_TIMEOUT_MS,
  });

  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const stderr = result.stderr ? String(result.stderr).trim() : "";
    throw new Error(stderr || "Gemini CLI failed.");
  }

  return String(result.stdout ?? "").trim();
}

function writeReviewFiles(review, meta) {
  fs.writeFileSync("ai_review.json", JSON.stringify(review, null, 2), "utf8");

  const decision = meta.decision;
  const summary = typeof review.summary === "string" ? review.summary : "";
  const findings = Array.isArray(review.findings) ? review.findings : [];
  const missing = Array.isArray(review.missing) ? review.missing : [];
  const source = meta.source;
  const baseSha = meta.baseSha;
  const headSha = meta.headSha;

  const lines = [
    "# AI Review (Gemini)",
    `Decision: ${decision}`,
    `Source: ${source}`,
    `Base: ${baseSha}`,
    `Head: ${headSha}`,
    "",
    summary,
    "",
    "## Findings",
  ];

  if (findings.length === 0) {
    lines.push("- None");
  } else {
    findings.forEach((finding) => {
      const location =
        finding.file && finding.line
          ? `${finding.file}:${finding.line}`
          : finding.file || "unknown";
      const severity = finding.severity || "INFO";
      const rule = finding.rule || "unspecified";
      const evidence = finding.evidence || "no evidence provided";
      const suggestion = finding.suggestion ? ` (${finding.suggestion})` : "";
      lines.push(`- ${severity} ${location} ${rule}: ${evidence}${suggestion}`);
    });
  }

  if (missing.length > 0) {
    lines.push("", "## Missing");
    missing.forEach((item) => {
      lines.push(`- ${item}`);
    });
  }

  const markdown = lines.join("\n");
  fs.writeFileSync("ai_review.md", markdown, "utf8");

  const summaryPath = process.env.GITHUB_STEP_SUMMARY;
  if (summaryPath) {
    fs.appendFileSync(summaryPath, `${markdown}\n`, "utf8");
  }
}

const { baseSha, headSha } = resolveShas();
if (!baseSha || !headSha) {
  console.error("Unable to determine base/head SHAs for AI review.");
  process.exit(1);
}

let diff = run(`git diff --no-color --unified=3 ${baseSha} ${headSha}`);
if (diff.length > MAX_DIFF_CHARS) {
  diff = `${diff.slice(0, MAX_DIFF_CHARS)}\n<TRUNCATED DIFF>`;
}

const filesRaw = run(`git diff --name-only ${baseSha} ${headSha}`);
const changedFiles = filesRaw ? filesRaw.split("\n").filter(Boolean) : [];

const requiredDocs = {
  codex: path.join("Handshake Codex v0.7.md"),
  specCurrent: path.join("docs", "SPEC_CURRENT.md"),
  architecture: path.join("docs", "ARCHITECTURE.md"),
  runbook: path.join("docs", "RUNBOOK_DEBUG.md"),
  qualityGate: path.join("docs", "QUALITY_GATE.md"),
};

const missingDocs = Object.entries(requiredDocs)
  .filter(([, docPath]) => !fs.existsSync(docPath))
  .map(([key, docPath]) => `${key}: ${docPath}`);

if (missingDocs.length > 0) {
  console.error("Missing required docs for AI review:");
  missingDocs.forEach((item) => console.error(`- ${item}`));
  process.exit(1);
}

const docs = {
  codex: readFileSafe(requiredDocs.codex),
  specCurrent: readFileSafe(requiredDocs.specCurrent),
  architecture: readFileSafe(requiredDocs.architecture),
  runbook: readFileSafe(requiredDocs.runbook),
  qualityGate: readFileSafe(requiredDocs.qualityGate),
};

let fileSections = "";
for (const filePath of changedFiles) {
  const header = `FILE: ${filePath}`;
  const content = fs.existsSync(filePath)
    ? withLineNumbers(readFileForPrompt(filePath))
    : "<deleted or missing in workspace>";
  const section = `${header}\n${content}`;
  if (fileSections.length + section.length + 2 > MAX_TOTAL_FILE_CHARS) {
    fileSections = `${fileSections}\n\n<TRUNCATED FILE CONTENTS>`;
    break;
  }
  fileSections = fileSections ? `${fileSections}\n\n${section}` : section;
}

if (!diff && changedFiles.length === 0) {
  const review = {
    decision: "PASS",
    summary: "No changes detected between base and head.",
    findings: [],
    missing: [],
  };
  writeReviewFiles(review, {
    source: "cli",
    decision: "PASS",
    baseSha,
    headSha,
  });
  process.exit(0);
}

const commandInfo = resolveGeminiCommand();

const systemPrompt = [
  "You are a senior software engineer performing a strict code review.",
  "Return JSON only. Do not include markdown or commentary outside JSON.",
  "Every finding must cite evidence from the diff or line-numbered file content.",
  "If there is insufficient evidence, do not guess; omit the finding.",
  "Use the provided Codex and docs as binding policy.",
].join(" ");

const userPrompt = [
  "Review context:",
  "1) Handshake Codex v0.7",
  docs.codex,
  "2) SPEC_CURRENT",
  docs.specCurrent,
  "3) ARCHITECTURE",
  docs.architecture,
  "4) RUNBOOK_DEBUG",
  docs.runbook,
  "5) QUALITY_GATE",
  docs.qualityGate,
  "",
  "Changed files:",
  changedFiles.join("\n") || "<none>",
  "",
  "Diff:",
  diff || "<no diff>",
  "",
  "File contents (line-numbered, may be truncated):",
  fileSections || "<no files>",
  "",
  "Return JSON with this schema:",
  '{ "decision": "BLOCK|WARN|PASS", "summary": "string", "findings": [ { "severity": "HIGH|MEDIUM|LOW|INFO", "file": "path", "line": number|null, "rule": "string", "evidence": "string", "suggestion": "string" } ], "missing": ["string"] }',
].join("\n");

const cliPrompt = `SYSTEM:\n${systemPrompt}\n\nUSER:\n${userPrompt}`;
let rawText = "";
try {
  rawText = runGeminiCli(cliPrompt, commandInfo);
} catch (err) {
  console.error(String(err));
  process.exit(1);
}

const jsonText = extractJson(rawText);

if (!jsonText) {
  console.error("Failed to extract JSON from Gemini response.");
  console.error(rawText);
  process.exit(1);
}

let review;
try {
  review = JSON.parse(jsonText);
} catch {
  console.error("Invalid JSON from Gemini response.");
  console.error(jsonText);
  process.exit(1);
}

const decision = String(review?.decision ?? "").toUpperCase();
if (!{ BLOCK: true, WARN: true, PASS: true }[decision]) {
  console.error("Review JSON missing valid decision.");
  console.error(jsonText);
  process.exit(1);
}

writeReviewFiles(review, { source: "cli", decision, baseSha, headSha });

if (decision === "BLOCK") {
  process.exit(1);
}
