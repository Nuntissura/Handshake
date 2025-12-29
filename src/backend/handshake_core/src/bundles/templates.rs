use chrono::{DateTime, Utc};

use crate::bundles::schemas::{
    BundleDiagnostic, BundleEnv, BundleJob, ManifestScope, MissingEvidence,
};

pub fn render_repro_md(
    env: &BundleEnv,
    scope: &ManifestScope,
    timeline: Option<(DateTime<Utc>, DateTime<Utc>, usize)>,
    job: Option<&BundleJob>,
    diagnostic: Option<&BundleDiagnostic>,
    steps_known: bool,
) -> String {
    let (first_seen, last_seen, count) =
        timeline
            .map(|(f, l, c)| (f, l, c))
            .unwrap_or((Utc::now(), Utc::now(), 0));

    let scope_line = match scope.kind {
        crate::bundles::schemas::ScopeKind::Job => scope
            .job_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        crate::bundles::schemas::ScopeKind::Problem => scope
            .problem_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        crate::bundles::schemas::ScopeKind::Workspace => {
            scope.wsid.clone().unwrap_or_else(|| "unknown".to_string())
        }
        crate::bundles::schemas::ScopeKind::TimeWindow => "time_window".to_string(),
    };

    let job_line = job
        .map(|j| j.job_id.clone())
        .unwrap_or_else(|| "n/a".to_string());
    let diag_line = diagnostic
        .map(|d| d.id.clone())
        .unwrap_or_else(|| "n/a".to_string());

    format!(
        "# Reproduction Steps\n\n\
## Environment\n\
- App Version: {app_version}\n\
- Build: {build_hash}\n\
- Platform: {os} / {arch}\n\
- Workspace: {workspace}\n\n\
## Timeline\n\
- First observed: {first_seen}\n\
- Last observed: {last_seen}\n\
- Occurrence count: {count}\n\n\
## Steps to Reproduce\n\
{steps}\n\n\
## Expected Behavior\n\
{expected}\n\n\
## Actual Behavior\n\
{actual}\n\n\
## Related Artifacts\n\
- Job ID: {job_line}\n\
- Diagnostic ID: {diag_line}\n\
- Scope: {scope_line}\n\
- See `trace.jsonl` for full event sequence\n",
        app_version = env.app_version,
        build_hash = env.build_hash,
        os = env.platform.os,
        arch = env.platform.arch,
        workspace = scope.wsid.clone().unwrap_or_else(|| "n/a".to_string()),
        first_seen = first_seen.to_rfc3339(),
        last_seen = last_seen.to_rfc3339(),
        count = count,
        steps = if steps_known {
            "1. Step 1\n2. Step 2"
        } else {
            "Steps to reproduce are unknown. The following context may help:\n- User action that triggered: unknown\n- Active document/surface: unknown"
        },
        expected = "Expected behavior is not provided.",
        actual = "Actual behavior is not provided.",
        job_line = job_line,
        diag_line = diag_line,
        scope_line = scope_line,
    )
}

pub fn render_coder_prompt(
    diagnostic: Option<&BundleDiagnostic>,
    env: &BundleEnv,
    scope: &ManifestScope,
    job: Option<&BundleJob>,
    missing: &[MissingEvidence],
    event_count: usize,
) -> String {
    let diag_title = diagnostic
        .map(|d| d.title.clone())
        .unwrap_or_else(|| "Unknown issue".to_string());
    let diag_code = diagnostic
        .map(|d| d.code.clone())
        .unwrap_or_else(|| "n/a".to_string());
    let diag_severity = diagnostic
        .map(|d| d.severity.clone())
        .unwrap_or_else(|| "info".to_string());
    let diag_msg = diagnostic
        .map(|d| d.message.clone())
        .unwrap_or_else(|| "Message not captured".to_string());

    let job_line = job
        .map(|j| {
            format!(
                "Job `{}` ({}) ended with status `{}`.",
                j.job_kind, j.job_id, j.status
            )
        })
        .unwrap_or_else(|| "No job context captured.".to_string());

    let missing_lines: Vec<String> = missing
        .iter()
        .map(|m| format!("- **{}** `{}`: {}", m.kind, m.id, m.reason))
        .collect();
    let missing_section = if missing_lines.is_empty() {
        "All requested evidence is included.".to_string()
    } else {
        format!(
            "The following evidence is unavailable:\n{}",
            missing_lines.join("\n")
        )
    };

    let time_range = scope
        .time_range
        .as_ref()
        .map(|r| format!("{} to {}", r.start.to_rfc3339(), r.end.to_rfc3339()))
        .unwrap_or_else(|| "n/a".to_string());

    format!(
        "# Debug Bundle for LLM Coder\n\n\
## Issue Summary\n\
**Title:** {title}\n\
**Severity:** {severity}\n\
**Code:** {code}\n\n\
## Message\n\
{message}\n\n\
## Context\n\
- **Workspace ID:** {wsid}\n\
- **Job ID:** {job_id}\n\
- **Diagnostic ID:** {diag_id}\n\
- **Time Range:** {time_range}\n\n\
## Version Information\n\
- App: {app_version} ({build_hash})\n\
- Platform: {os} / {arch}\n\
- Model Runtime: {runtime}\n\n\
## What Failed\n\
{job_line}\n\n\
## Steps to Reproduce\n\
See `repro.md` for detailed reproduction steps.\n\n\
## Expected vs Actual\n\
- **Expected:** Expected behavior is not provided.\n\
- **Actual:** Actual behavior is not provided.\n\n\
## Evidence Files\n\
| File | Description |\n|------|-------------|\n\
| `jobs.json` | Job metadata and status |\n\
| `diagnostics.jsonl` | Normalized diagnostics |\n\
| `trace.jsonl` | Flight Recorder events ({event_count} entries) |\n\
| `env.json` | Environment context (redacted) |\n\
| `retention_report.json` | Evidence availability |\n\
| `redaction_report.json` | What was redacted |\n\n\
## Missing Evidence\n\
{missing_section}\n\n\
## Instructions for Coder\n\
1. Start by reading this prompt and understanding the issue\n\
2. Examine `jobs.json` for the failing job's context\n\
3. Search `diagnostics.jsonl` for related errors\n\
4. Trace the event sequence in `trace.jsonl`\n\
5. Check `retention_report.json` for any evidence gaps\n\
6. Propose a fix based on the evidence\n",
        title = diag_title,
        severity = diag_severity,
        code = diag_code,
        message = diag_msg,
        wsid = scope.wsid.clone().unwrap_or_else(|| "n/a".to_string()),
        job_id = scope.job_id.clone().unwrap_or_else(|| "n/a".to_string()),
        diag_id = scope
            .problem_id
            .clone()
            .or_else(|| diagnostic.map(|d| d.id.clone()))
            .unwrap_or_else(|| "n/a".to_string()),
        time_range = time_range,
        app_version = env.app_version,
        build_hash = env.build_hash,
        os = env.platform.os,
        arch = env.platform.arch,
        runtime = env.config.model_runtime,
        job_line = job_line,
        event_count = event_count,
        missing_section = missing_section,
    )
}
