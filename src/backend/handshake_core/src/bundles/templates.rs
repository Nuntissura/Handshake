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
    let (first_seen, last_seen, count) = timeline.unwrap_or_else(|| {
        let now = Utc::now();
        (now, now, 0)
    });

    let wsid = scope.wsid.clone().unwrap_or_else(|| "n/a".to_string());
    let job_id = job
        .map(|j| j.job_id.clone())
        .or_else(|| scope.job_id.clone())
        .unwrap_or_else(|| "n/a".to_string());
    let diagnostic_id = diagnostic
        .map(|d| d.id.clone())
        .or_else(|| scope.problem_id.clone())
        .unwrap_or_else(|| "n/a".to_string());

    format!(
        "# Reproduction Steps\n\n\
## Environment\n\
- App Version: {app_version}\n\
- Build: {build_hash}\n\
- Platform: {os} / {arch}\n\
- Workspace: {wsid}\n\n\
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
- Job ID: {job_id}\n\
- Diagnostic ID: {diagnostic_id}\n\
- See `trace.jsonl` for full event sequence\n",
        app_version = env.app_version,
        build_hash = env.build_hash,
        os = env.platform.os,
        arch = env.platform.arch,
        wsid = wsid,
        first_seen = first_seen.to_rfc3339(),
        last_seen = last_seen.to_rfc3339(),
        count = count,
        steps = if steps_known {
            "1. <step_1>\n2. <step_2>\n..."
        } else {
            "Steps to reproduce are unknown. The following context may help:\n- User action that triggered: unknown\n- Active document/surface: unknown"
        },
        expected = "Expected behavior is not provided.",
        actual = "Actual behavior is not provided.",
        job_id = job_id,
        diagnostic_id = diagnostic_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render_coder_prompt(
    diagnostic: Option<&BundleDiagnostic>,
    env: &BundleEnv,
    scope: &ManifestScope,
    job: Option<&BundleJob>,
    jobs_file_name: &str,
    missing: &[MissingEvidence],
    event_count: usize,
    diagnostic_count: usize,
    event_ids: &[String],
) -> String {
    let diag_title = diagnostic
        .map(|d| d.title.clone())
        .unwrap_or_else(|| "Unknown issue".to_string());
    let diag_code = diagnostic
        .map(|d| d.code.clone())
        .unwrap_or_else(|| "n/a".to_string());
    let diag_severity = diagnostic
        .map(|d| d.severity.as_str().to_string())
        .unwrap_or_else(|| "info".to_string());
    let diag_msg = diagnostic
        .map(|d| d.message.clone())
        .unwrap_or_else(|| "Message not captured".to_string());

    let wsid = scope
        .wsid
        .clone()
        .or_else(|| job.and_then(|j| j.wsid.clone()))
        .unwrap_or_else(|| "n/a".to_string());
    let job_id = scope
        .job_id
        .clone()
        .or_else(|| job.map(|j| j.job_id.clone()))
        .unwrap_or_else(|| "n/a".to_string());
    let diag_id = scope
        .problem_id
        .clone()
        .or_else(|| diagnostic.map(|d| d.id.clone()))
        .unwrap_or_else(|| "n/a".to_string());

    let job_line = job
        .map(|j| {
            let mut line = format!(
                "Job `{}` ({}) ended with status `{}`.",
                j.job_kind,
                j.job_id,
                j.status.as_str()
            );
            if let Some(err) = j.error.as_ref() {
                line.push_str(&format!("\nError: {} - {}", err.code, err.message));
            }
            line
        })
        .unwrap_or_else(|| "No job context captured.".to_string());

    let missing_lines: Vec<String> = missing
        .iter()
        .map(|m| {
            format!(
                "- **{}** `{}`: {}",
                m.kind.as_str(),
                m.id,
                m.reason.as_str()
            )
        })
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

    let workflow_run_id = job
        .and_then(|j| j.workflow_run_id.as_deref())
        .unwrap_or("n/a");

    let event_ids_line = if event_ids.is_empty() {
        "n/a".to_string()
    } else {
        event_ids
            .iter()
            .map(|id| format!("`{}`", id))
            .collect::<Vec<_>>()
            .join(", ")
    };

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
| `{jobs_file_name}` | Job metadata and status |\n\
| `diagnostics.jsonl` | Normalized diagnostics ({diagnostic_count} entries) |\n\
| `trace.jsonl` | Flight Recorder events ({event_count} entries) |\n\
| `env.json` | Environment context (redacted) |\n\
| `retention_report.json` | Evidence availability |\n\
| `redaction_report.json` | What was redacted |\n\n\
## Key IDs for Investigation\n\
- Diagnostic ID: `{diag_id}`\n\
- Job ID: `{job_id}`\n\
- Workflow Run ID: `{workflow_run_id}`\n\
- Event IDs: {event_ids_line}\n\n\
## Policy Notes\n\
No policy restrictions applied.\n\n\
## Missing Evidence\n\
{missing_section}\n\n\
## Instructions for Coder\n\
1. Start by reading this prompt and understanding the issue\n\
2. Examine `{jobs_file_name}` for the failing job's context\n\
3. Search `diagnostics.jsonl` for related errors\n\
4. Trace the event sequence in `trace.jsonl`\n\
5. Check `retention_report.json` for any evidence gaps\n\
6. Propose a fix based on the evidence\n",
        title = diag_title,
        severity = diag_severity,
        code = diag_code,
        message = diag_msg,
        wsid = wsid,
        job_id = job_id,
        diag_id = diag_id,
        time_range = time_range,
        app_version = env.app_version,
        build_hash = env.build_hash,
        os = env.platform.os,
        arch = env.platform.arch,
        runtime = env.config.model_runtime,
        job_line = job_line,
        event_count = event_count,
        missing_section = missing_section,
        jobs_file_name = jobs_file_name,
        diagnostic_count = diagnostic_count,
        workflow_run_id = workflow_run_id,
        event_ids_line = event_ids_line,
    )
}
