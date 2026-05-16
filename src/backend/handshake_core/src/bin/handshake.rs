use std::{
    env,
    path::PathBuf,
    process::{Command, ExitCode},
};

use handshake_core::kernel::{
    mechanical_contract_generation::{
        command_receipt_slug, write_current_candidate_command_receipt,
        CurrentCandidateCommandReceiptInputV1,
    },
    product_screenshot_capture::{
        capture_product_screenshot_from_browser_adapter, scope_cli_value,
        ProductScreenshotBrowserAdapterConfigV1, ProductScreenshotRequestV1,
        ScreenshotCaptureExecutionSurface, ScreenshotCaptureScope, ScreenshotCaptureTriggerKind,
    },
};
use serde_json::json;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<String>>();
    if args.len() >= 2 && args[0] == "screenshot" && args[1] == "capture" {
        args.drain(0..2);
        return run_screenshot_capture(&args);
    }
    if args.len() >= 3 && args[0] == "command" && args[1] == "receipt" && args[2] == "run" {
        args.drain(0..3);
        return run_command_receipt(&args);
    }
    Err(usage())
}

fn run_screenshot_capture(args: &[String]) -> Result<(), String> {
    let options = CliOptions::parse(args)?;
    let request = ProductScreenshotRequestV1 {
        request_id: options.request_id.clone(),
        scope: options.scope,
        target_ref: options.target_ref.clone(),
        requested_by_role: options.requested_by_role.clone(),
        trigger_kind: ScreenshotCaptureTriggerKind::GovernedCoderCli,
        window_title: options.window_title.clone(),
        width: options.width,
        height: options.height,
        capture_adapter_ref: "capture-adapter://app/playwright-dom-screenshot".to_string(),
        flight_recorder_ref: format!(
            "FR-EVT-VISUAL-CAPTURE-{}",
            options.request_id.replace(['.', '/', ':'], "-")
        ),
        execution_surface: ScreenshotCaptureExecutionSurface::GovernedAdapterCli,
        workdir_ref: "repo-root://".to_string(),
    };
    let command_or_api_ref = format!(
        "cli://handshake screenshot capture --scope {} --target-ref {}",
        scope_cli_value(options.scope),
        options.target_ref
    );
    let result = capture_product_screenshot_from_browser_adapter(
        &request,
        ProductScreenshotBrowserAdapterConfigV1 {
            source_url: options.source_url,
            adapter_script_path: options.adapter_script_path.to_string_lossy().into_owned(),
            node_binary: options.node_binary,
            command_or_api_ref,
        },
        options.artifact_root,
    )
    .map_err(|error| format!("screenshot capture failed: {error:?}"))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_id": "hsk.handshake_cli_screenshot_capture_result@1",
            "request_id": request.request_id,
            "scope": scope_cli_value(request.scope),
            "target_ref": request.target_ref,
            "artifact_ref": result.artifact.screenshot_ref,
            "metadata_ref": result.artifact.metadata_ref,
            "receipt_ref": result.durable_receipt.receipt_ref,
            "screenshot_path": result.screenshot_path,
            "metadata_path": result.metadata_path,
            "receipt_path": result.receipt_path,
            "adapter_exit_status": result.receipt.adapter_exit_status,
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn run_command_receipt(args: &[String]) -> Result<(), String> {
    let options = CommandReceiptOptions::parse(args)?;
    let output = shell_command_output(&options.command_line, &options.workdir)?;
    let actual_exit_code = output.status.code().unwrap_or(1);
    let candidate_sha = options
        .candidate_sha
        .unwrap_or_else(|| git_head_sha(&options.workdir).unwrap_or_else(|| "unknown".to_string()));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let slug = options
        .slug
        .clone()
        .unwrap_or_else(|| command_receipt_slug(&options.command_line));
    let blocker_refs = if actual_exit_code == options.expected_exit_code {
        Vec::new()
    } else {
        vec![format!(
            "blocker://current-candidate-command/{slug}/exit-code-{actual_exit_code}"
        )]
    };

    let result = write_current_candidate_command_receipt(
        CurrentCandidateCommandReceiptInputV1 {
            command_line: options.command_line,
            workdir: options.workdir.to_string_lossy().into_owned(),
            candidate_sha,
            expected_exit_code: options.expected_exit_code,
            actual_exit_code,
            stdout,
            stderr,
            blocker_refs,
            projection_refs: options.projection_refs,
            slug: options.slug,
        },
        options.artifact_root,
    )
    .map_err(|error| format!("command receipt write failed: {error:?}"))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&result.receipt).map_err(|error| error.to_string())?
    );
    // Propagate the wrapped command's actual exit code so just recipes and CI
    // do not silently treat a failing wrapped command as success. The receipt
    // and blocker artifacts are already on disk above.
    if actual_exit_code != options.expected_exit_code {
        std::process::exit(actual_exit_code);
    }
    Ok(())
}

struct CliOptions {
    scope: ScreenshotCaptureScope,
    target_ref: String,
    source_url: String,
    request_id: String,
    artifact_root: PathBuf,
    adapter_script_path: PathBuf,
    node_binary: String,
    width: u32,
    height: u32,
    window_title: String,
    requested_by_role: String,
}

impl CliOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut scope = None;
        let mut target_ref = None;
        let mut source_url = env::var("HANDSHAKE_SCREENSHOT_SOURCE_URL").ok();
        let mut request_id = None;
        let mut artifact_root =
            PathBuf::from("../Handshake_Artifacts/handshake-product/screenshots");
        let mut adapter_script_path = PathBuf::from("app/scripts/handshake-screenshot-capture.mjs");
        let mut node_binary = "node".to_string();
        let mut width = 1440;
        let mut height = 960;
        let mut window_title = "Handshake Desktop Shell".to_string();
        let mut requested_by_role = "CODER".to_string();

        let mut index = 0;
        while index < args.len() {
            let key = args[index].as_str();
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {key}"))?;
            match key {
                "--scope" => scope = Some(parse_scope(value)?),
                "--target-ref" | "--target" => target_ref = Some(value.clone()),
                "--source-url" => source_url = Some(value.clone()),
                "--request-id" => request_id = Some(value.clone()),
                "--artifact-root" => artifact_root = PathBuf::from(value),
                "--adapter-script" => adapter_script_path = PathBuf::from(value),
                "--node-binary" => node_binary = value.clone(),
                "--width" => width = parse_positive_u32(value, "--width")?,
                "--height" => height = parse_positive_u32(value, "--height")?,
                "--window-title" => window_title = value.clone(),
                "--requested-by-role" => requested_by_role = value.clone(),
                _ => return Err(format!("unknown option {key}\n{}", usage())),
            }
            index += 2;
        }

        let scope = scope.ok_or_else(usage)?;
        let target_ref = target_ref.unwrap_or_else(|| default_target_ref(scope).to_string());
        let source_url = source_url.ok_or_else(|| {
            "missing --source-url or HANDSHAKE_SCREENSHOT_SOURCE_URL for browser capture"
                .to_string()
        })?;
        let request_id = request_id.unwrap_or_else(|| {
            format!(
                "request.screenshot.{}",
                scope_cli_value(scope).replace('-', "_")
            )
        });

        Ok(Self {
            scope,
            target_ref,
            source_url,
            request_id,
            artifact_root,
            adapter_script_path,
            node_binary,
            width,
            height,
            window_title,
            requested_by_role,
        })
    }
}

struct CommandReceiptOptions {
    command_line: String,
    workdir: PathBuf,
    candidate_sha: Option<String>,
    expected_exit_code: i32,
    artifact_root: PathBuf,
    projection_refs: Vec<String>,
    slug: Option<String>,
}

impl CommandReceiptOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut command_line = None;
        let mut workdir = env::current_dir().map_err(|error| error.to_string())?;
        let mut candidate_sha = None;
        let mut expected_exit_code = 0;
        let mut artifact_root =
            PathBuf::from("../Handshake_Artifacts/handshake-product/command-receipts");
        let mut projection_refs = Vec::new();
        let mut slug: Option<String> = None;

        let mut index = 0;
        while index < args.len() {
            let key = args[index].as_str();
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {key}"))?;
            match key {
                "--command-line" => command_line = Some(value.clone()),
                "--workdir" => workdir = PathBuf::from(value),
                "--candidate-sha" => candidate_sha = Some(value.clone()),
                "--expected-exit-code" => {
                    expected_exit_code = value
                        .parse::<i32>()
                        .map_err(|_| "--expected-exit-code must be an integer".to_string())?
                }
                "--artifact-root" => artifact_root = PathBuf::from(value),
                "--projection-ref" => projection_refs.push(value.clone()),
                "--slug" => slug = Some(value.clone()),
                _ => return Err(format!("unknown option {key}\n{}", usage())),
            }
            index += 2;
        }

        let command_line = command_line
            .ok_or_else(|| "missing --command-line for command receipt run".to_string())?;
        if projection_refs.is_empty() {
            let projection_slug = slug
                .clone()
                .unwrap_or_else(|| command_receipt_slug(&command_line));
            projection_refs.push(format!(
                "projection://current-candidate-command/{projection_slug}"
            ));
        }

        Ok(Self {
            command_line,
            workdir,
            candidate_sha,
            expected_exit_code,
            artifact_root,
            projection_refs,
            slug,
        })
    }
}

fn parse_scope(value: &str) -> Result<ScreenshotCaptureScope, String> {
    match value {
        "full-app" | "full_app" => Ok(ScreenshotCaptureScope::FullApp),
        "panel" => Ok(ScreenshotCaptureScope::Panel),
        "module" => Ok(ScreenshotCaptureScope::Module),
        _ => Err(format!("unsupported --scope {value}")),
    }
}

fn default_target_ref(scope: ScreenshotCaptureScope) -> &'static str {
    match scope {
        ScreenshotCaptureScope::FullApp => "app://handshake",
        ScreenshotCaptureScope::Panel => "panel://kernel-dcc-projection",
        ScreenshotCaptureScope::Module => "module://kernel-dcc-session-spawn-tree",
    }
}

fn parse_positive_u32(value: &str, key: &str) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{key} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{key} must be a positive integer"));
    }
    Ok(parsed)
}

fn shell_command_output(
    command_line: &str,
    workdir: &PathBuf,
) -> Result<std::process::Output, String> {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(command_line);
        command
    } else {
        let mut command = Command::new("sh");
        command.arg("-c").arg(command_line);
        command
    };
    command
        .current_dir(workdir)
        .output()
        .map_err(|error| format!("failed to execute command receipt command: {error}"))
}

fn git_head_sha(workdir: &PathBuf) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

fn usage() -> String {
    "usage: handshake screenshot capture --scope full-app|panel|module --source-url URL [--target-ref REF] [--request-id ID] [--artifact-root PATH]\n       handshake command receipt run --command-line COMMAND [--workdir PATH] [--expected-exit-code N] [--artifact-root PATH] [--slug SLUG]".to_string()
}
