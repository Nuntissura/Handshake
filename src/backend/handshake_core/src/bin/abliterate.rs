//! MT-106 INF-6 abliterate CLI binary.
//!
//! Operator-facing entrypoint for offline weight orthogonalisation. The
//! binary lives here; the algorithm + safetensors I/O orchestration live
//! in `handshake_core::distillation::abliterate`. The hot-path invariant
//! (`distillation::abliterate` MUST NOT be referenced from any runtime
//! `generate.rs`) is enforced by `tests/abliterate_tool_tests.rs`.
//!
//! Per WP-KERNEL-004 wp_validator_final_disposition the model I/O path
//! uses the Candle safetensors adapter (option_c), so this binary is
//! gated on the `candle-runtime-engine` cargo feature in Cargo.toml
//! `[[bin]] required-features`. Build with
//! `cargo build --bin abliterate --features candle-runtime-engine`.
//!
//! Usage:
//!   abliterate \
//!     --base-model <PATH> \
//!     --refusal-direction <PATH> \
//!     --out-model <PATH> \
//!     --license-tag <STRING> \
//!     --provenance-note <STRING> \
//!     --operator-signature <STRING>
//!
//! All six arguments are mandatory; `--license-tag` and
//! `--operator-signature` are explicitly checked per MT-106
//! red_team minimum_controls.

use std::{env, path::PathBuf, process::ExitCode};

use handshake_core::distillation::abliterate::{
    provenance_sidecar_path, run_abliteration_offline, AbliterationConfig,
};

fn main() -> ExitCode {
    match run_cli(env::args().skip(1)) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn run_cli<I>(args: I) -> Result<String, String>
where
    I: IntoIterator<Item = String>,
{
    let config = parse_args(args)?;
    if config.help {
        return Ok(help_text().to_string());
    }
    let abliteration_config = config.into_abliteration_config()?;
    // The CLI on a host without an attached Postgres ledger writer
    // passes `None`; integration tests pass `Some(&LedgerBatcher)` so
    // the engine_kind=AbliterationTool row registration is exercised.
    match run_abliteration_offline(&abliteration_config, None) {
        Ok(provenance) => {
            let sidecar = provenance_sidecar_path(&abliteration_config.out_model_path);
            Ok(format!(
                "abliteration succeeded; output={}; provenance_sidecar={}; provenance:\n{}",
                abliteration_config.out_model_path.display(),
                sidecar.display(),
                serde_json::to_string_pretty(&provenance)
                    .unwrap_or_else(|err| format!("(provenance serialise failed: {err})"))
            ))
        }
        Err(other) => Err(format!("{other}")),
    }
}

#[derive(Debug, Default)]
struct ParsedCli {
    base_model: Option<PathBuf>,
    refusal_direction: Option<PathBuf>,
    out_model: Option<PathBuf>,
    license_tag: Option<String>,
    provenance_note: Option<String>,
    operator_signature: Option<String>,
    help: bool,
}

impl ParsedCli {
    fn into_abliteration_config(self) -> Result<AbliterationConfig, String> {
        let base_model_path = self
            .base_model
            .ok_or_else(|| "missing required --base-model".to_string())?;
        let refusal_direction_path = self
            .refusal_direction
            .ok_or_else(|| "missing required --refusal-direction".to_string())?;
        let out_model_path = self
            .out_model
            .ok_or_else(|| "missing required --out-model".to_string())?;
        let license_tag = self
            .license_tag
            .ok_or_else(|| "missing required --license-tag (mandatory per MT-106)".to_string())?;
        let provenance_note = self
            .provenance_note
            .ok_or_else(|| "missing required --provenance-note".to_string())?;
        let operator_signature = self
            .operator_signature
            .ok_or_else(|| "missing required --operator-signature".to_string())?;
        Ok(AbliterationConfig {
            base_model_path,
            refusal_direction_path,
            out_model_path,
            license_tag,
            provenance_note,
            operator_signature,
        })
    }
}

fn parse_args<I>(args: I) -> Result<ParsedCli, String>
where
    I: IntoIterator<Item = String>,
{
    let mut cli = ParsedCli::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => cli.help = true,
            "--base-model" => cli.base_model = Some(PathBuf::from(next_value(&mut iter, &arg)?)),
            "--refusal-direction" => {
                cli.refusal_direction = Some(PathBuf::from(next_value(&mut iter, &arg)?))
            }
            "--out-model" => cli.out_model = Some(PathBuf::from(next_value(&mut iter, &arg)?)),
            "--license-tag" => cli.license_tag = Some(next_value(&mut iter, &arg)?),
            "--provenance-note" => cli.provenance_note = Some(next_value(&mut iter, &arg)?),
            "--operator-signature" => cli.operator_signature = Some(next_value(&mut iter, &arg)?),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(cli)
}

fn next_value<I>(iter: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    iter.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn help_text() -> &'static str {
    "Usage: abliterate \\\n\
     \x20  --base-model <PATH> \\\n\
     \x20  --refusal-direction <PATH> \\\n\
     \x20  --out-model <PATH> \\\n\
     \x20  --license-tag <STRING> \\\n\
     \x20  --provenance-note <STRING> \\\n\
     \x20  --operator-signature <STRING>\n\
     \n\
     OFFLINE tool only - NEVER inserted into model_runtime::generate.\n\
     See Master Spec §4.7.4 + MT-106 contract."
}
