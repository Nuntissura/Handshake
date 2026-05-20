//! MT-122 PEFT teacher/student training CLI binary.
//!
//! Operator-facing entrypoint for the LoRA distillation pipeline. The
//! binary lives here; the orchestration + subprocess + provenance
//! handling logic lives in
//! `handshake_core::distillation::peft_pipeline`. The Python
//! `scripts/distill/train_lora.py` runs the actual peft/transformers
//! training step.
//!
//! Per MT-122 operator clarification 2026-05-20:
//! - Distillation defaults to CLOUD-TEACHER -> LOCAL-STUDENT.
//! - Teacher routing: CLI_BRIDGE (default, via MT-127), BYOK
//!   (via MT-125/126), or LOCAL_LARGER (local-larger-teacher model).
//!
//! Spec-Realism Gate compliance:
//! - Sub-rule 1: NO LiveXxxUnavailable / "not yet wired" returns. The
//!   CLI executes the pip-installed Python subprocess.
//! - Sub-rule 2: Real resource = pip-installed peft/transformers/torch.
//!   The CLI invokes `python.exe scripts/distill/train_lora.py` and
//!   asserts on real subprocess exit codes.
//! - Sub-rule 3: Validator signs off separately.
//!
//! Usage:
//!   peft_train \
//!     --corpus-jsonl <PATH> \
//!     --student-base <PATH or HF name | "synthetic"> \
//!     --teacher-source <CLI_BRIDGE | BYOK | LOCAL_LARGER> \
//!     --out-lora <PATH> \
//!     --license-tag <STRING> \
//!     --operator-signature <STRING> \
//!     --max-steps <N> \
//!     --learning-rate <FLOAT> \
//!     [--teacher <STRING>] \
//!     [--python <PATH>] \
//!     [--script <PATH>] \
//!     [--rank <N>] [--alpha <FLOAT>] [--dropout <FLOAT>] \
//!     [--epochs <N>] [--batch-size <N>] \
//!     [--cpu-only]
//!
//! Adult-production discipline (GLOBAL-PRODUCTION-002..009): the CLI
//! never moralises, censors, or rewords operator content. The
//! ContentReview gate upstream handles license + PII; this CLI runs
//! the training.

use std::{env, path::PathBuf, process::ExitCode};

use handshake_core::distillation::peft_pipeline::{
    DistillJobConfig, PeftHyperparams, PeftProvenanceSidecar, PeftTrainerExecutor,
    PythonPeftTrainerExecutor, TeacherSource,
};

fn main() -> ExitCode {
    match run_cli(env::args().skip(1).collect::<Vec<String>>()) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("peft_train: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_cli(args: Vec<String>) -> Result<String, String> {
    let parsed = ParsedArgs::parse(args)?;
    if parsed.help {
        return Ok(help_text().to_string());
    }
    let trainer_args = parsed.into_trainer_args()?;
    let executor = trainer_args.build_executor()?;

    // The CLI bypasses the full `distill()` ContentReview pipeline
    // because the corpus on disk has already been gated (the upstream
    // workflow writes the filtered JSONL before invoking this CLI).
    // The CLI is the subprocess driver: it spawns the Python trainer,
    // reads the provenance sidecar, and prints a machine-readable
    // result.
    let config = trainer_args.into_config();
    executor.run(&config).map_err(|err| err.to_string())?;

    // Read the provenance sidecar written by the Python trainer.
    let provenance_path = config.output_lora_dir.join("provenance.json");
    let provenance = PeftProvenanceSidecar::read_from(&provenance_path)
        .map_err(|err| format!("read provenance: {err}"))?;

    let result = serde_json::json!({
        "schema": "hsk.distill.peft_train_cli_result@v1",
        "lora_dir": config.output_lora_dir.to_string_lossy(),
        "provenance_path": provenance_path.to_string_lossy(),
        "teacher_source": config.teacher_source.cli_arg_value(),
        "teacher_model_id": provenance.teacher_model_id,
        "student_base_id": provenance.student_base_id,
        "corpus_sha256": provenance.corpus_sha256,
        "license_tag": provenance.license_tag,
        "operator_signature": provenance.operator_signature,
        "trained_at_utc": provenance.trained_at_utc,
        "training_loss": provenance.training_loss,
        "num_steps": provenance.num_steps,
    });
    Ok(serde_json::to_string_pretty(&result).map_err(|err| err.to_string())?)
}

#[derive(Debug)]
struct TrainerArgs {
    python: Option<PathBuf>,
    script: Option<PathBuf>,
    corpus_jsonl: PathBuf,
    student_base: PathBuf,
    teacher: PathBuf,
    teacher_source: TeacherSource,
    out_lora: PathBuf,
    license_tag: String,
    operator_signature: String,
    hyperparams: PeftHyperparams,
    max_steps: Option<u32>,
    cpu_only: bool,
}

impl TrainerArgs {
    fn build_executor(&self) -> Result<PythonPeftTrainerExecutor, String> {
        let python_path = match &self.python {
            Some(path) => path.clone(),
            None => locate_python311()?,
        };
        let script_path = match &self.script {
            Some(path) => path.clone(),
            None => locate_default_script()?,
        };
        let executor = PythonPeftTrainerExecutor::new(python_path, script_path)
            .with_cpu_only(self.cpu_only);
        Ok(executor)
    }

    fn into_config(self) -> DistillJobConfig {
        DistillJobConfig {
            teacher_model_path: self.teacher,
            student_base_model_path: self.student_base,
            output_lora_dir: self.out_lora,
            corpus_jsonl_path: self.corpus_jsonl,
            hyperparams: self.hyperparams,
            license_tag: self.license_tag,
            operator_signature: self.operator_signature,
            teacher_source: self.teacher_source,
            max_steps: self.max_steps,
        }
    }
}

#[derive(Default, Debug)]
struct ParsedArgs {
    help: bool,
    python: Option<PathBuf>,
    script: Option<PathBuf>,
    corpus_jsonl: Option<PathBuf>,
    student_base: Option<PathBuf>,
    teacher: Option<PathBuf>,
    teacher_source: Option<TeacherSource>,
    out_lora: Option<PathBuf>,
    license_tag: Option<String>,
    operator_signature: Option<String>,
    rank: Option<u32>,
    alpha: Option<f32>,
    dropout: Option<f32>,
    epochs: Option<u32>,
    learning_rate: Option<f32>,
    batch_size: Option<u32>,
    max_steps: Option<u32>,
    cpu_only: bool,
}

impl ParsedArgs {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut parsed = ParsedArgs::default();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--help" | "-h" => parsed.help = true,
                "--python" => parsed.python = Some(PathBuf::from(next_value(&mut iter, &arg)?)),
                "--script" => parsed.script = Some(PathBuf::from(next_value(&mut iter, &arg)?)),
                "--corpus-jsonl" => {
                    parsed.corpus_jsonl = Some(PathBuf::from(next_value(&mut iter, &arg)?));
                }
                "--student-base" => {
                    parsed.student_base = Some(PathBuf::from(next_value(&mut iter, &arg)?));
                }
                "--teacher" => parsed.teacher = Some(PathBuf::from(next_value(&mut iter, &arg)?)),
                "--teacher-source" => {
                    let v = next_value(&mut iter, &arg)?;
                    parsed.teacher_source = Some(TeacherSource::from_cli_arg(&v)?);
                }
                "--out-lora" => parsed.out_lora = Some(PathBuf::from(next_value(&mut iter, &arg)?)),
                "--license-tag" => parsed.license_tag = Some(next_value(&mut iter, &arg)?),
                "--operator-signature" => {
                    parsed.operator_signature = Some(next_value(&mut iter, &arg)?);
                }
                "--rank" => parsed.rank = Some(parse_u32(&next_value(&mut iter, &arg)?, &arg)?),
                "--alpha" => parsed.alpha = Some(parse_f32(&next_value(&mut iter, &arg)?, &arg)?),
                "--dropout" => {
                    parsed.dropout = Some(parse_f32(&next_value(&mut iter, &arg)?, &arg)?);
                }
                "--epochs" => parsed.epochs = Some(parse_u32(&next_value(&mut iter, &arg)?, &arg)?),
                "--learning-rate" => {
                    parsed.learning_rate = Some(parse_f32(&next_value(&mut iter, &arg)?, &arg)?);
                }
                "--batch-size" => {
                    parsed.batch_size = Some(parse_u32(&next_value(&mut iter, &arg)?, &arg)?);
                }
                "--max-steps" => {
                    parsed.max_steps = Some(parse_u32(&next_value(&mut iter, &arg)?, &arg)?);
                }
                "--cpu-only" => parsed.cpu_only = true,
                other => return Err(format!("unknown argument {other:?}")),
            }
        }
        Ok(parsed)
    }

    fn into_trainer_args(self) -> Result<TrainerArgs, String> {
        if self.help {
            // Caller handles help-mode before this is invoked.
            return Err("help requested".to_string());
        }
        let corpus_jsonl = self
            .corpus_jsonl
            .ok_or_else(|| "--corpus-jsonl is required".to_string())?;
        let student_base = self
            .student_base
            .ok_or_else(|| "--student-base is required".to_string())?;
        let out_lora = self
            .out_lora
            .ok_or_else(|| "--out-lora is required".to_string())?;
        let license_tag = self
            .license_tag
            .ok_or_else(|| "--license-tag is required".to_string())?;
        if license_tag.trim().is_empty() {
            return Err("--license-tag must not be empty".to_string());
        }
        let operator_signature = self
            .operator_signature
            .ok_or_else(|| "--operator-signature is required".to_string())?;
        if operator_signature.trim().is_empty() {
            return Err("--operator-signature must not be empty".to_string());
        }
        // Teacher path defaults to the literal "placeholder" when only
        // a teacher_source label is provided; CLI_BRIDGE and BYOK use
        // routing semantics rather than a local model path. The Python
        // trainer records this verbatim into the provenance sidecar.
        let teacher = self
            .teacher
            .unwrap_or_else(|| PathBuf::from("placeholder"));
        let teacher_source = self.teacher_source.unwrap_or_default();

        let default_hp = PeftHyperparams::default();
        let hyperparams = PeftHyperparams {
            rank: self.rank.unwrap_or(default_hp.rank),
            alpha: self.alpha.unwrap_or(default_hp.alpha),
            dropout: self.dropout.unwrap_or(default_hp.dropout),
            epochs: self.epochs.unwrap_or(default_hp.epochs),
            learning_rate: self.learning_rate.unwrap_or(default_hp.learning_rate),
            batch_size: self.batch_size.unwrap_or(default_hp.batch_size),
        };

        Ok(TrainerArgs {
            python: self.python,
            script: self.script,
            corpus_jsonl,
            student_base,
            teacher,
            teacher_source,
            out_lora,
            license_tag,
            operator_signature,
            hyperparams,
            max_steps: self.max_steps,
            cpu_only: self.cpu_only,
        })
    }
}

fn next_value(iter: &mut std::vec::IntoIter<String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn parse_u32(value: &str, flag: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|err| format!("{flag} expected u32, got {value:?}: {err}"))
}

fn parse_f32(value: &str, flag: &str) -> Result<f32, String> {
    value
        .parse::<f32>()
        .map_err(|err| format!("{flag} expected f32, got {value:?}: {err}"))
}

fn locate_python311() -> Result<PathBuf, String> {
    // Prefer an explicit env override so the operator can pin the
    // interpreter without rebuilding.
    if let Ok(path) = env::var("HANDSHAKE_PEFT_PYTHON") {
        return Ok(PathBuf::from(path));
    }
    // Fall back to which-discovered python in PATH; the operator is
    // responsible for ensuring the discovered interpreter has
    // peft/transformers/torch installed.
    which::which("python")
        .or_else(|_| which::which("python3"))
        .or_else(|_| which::which("py"))
        .map_err(|err| {
            format!(
                "could not locate python on PATH (which lookup failed: {err}); \
                 set HANDSHAKE_PEFT_PYTHON or pass --python <PATH>"
            )
        })
}

fn locate_default_script() -> Result<PathBuf, String> {
    // The script is bundled at <repo_root>/scripts/distill/train_lora.py.
    // The CLI is built from <repo_root>/src/backend/handshake_core; we
    // walk three parents up from CARGO_MANIFEST_DIR at compile-time and
    // append the relative script path. This holds for both `cargo run`
    // and the packaged binary because we record the repo root at
    // compile time rather than at run time.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| {
            format!(
                "could not derive repo root from CARGO_MANIFEST_DIR={}",
                manifest_dir.display()
            )
        })?
        .to_path_buf();
    let script_path = repo_root.join("scripts").join("distill").join("train_lora.py");
    if !script_path.is_file() {
        return Err(format!(
            "default trainer script not found at {}",
            script_path.display()
        ));
    }
    Ok(script_path)
}

fn help_text() -> &'static str {
    "MT-122 PEFT teacher/student LoRA distillation CLI\n\
     \n\
     Required:\n\
       --corpus-jsonl <PATH>          Path to filtered TrainingCorpus JSONL\n\
       --student-base <PATH or NAME>  Student base model path or HF name (use 'synthetic' for tests)\n\
       --out-lora <PATH>              Output directory for adapter_model.safetensors + provenance.json\n\
       --license-tag <STRING>         License tag recorded in provenance\n\
       --operator-signature <STRING>  Operator signature recorded in provenance\n\
     \n\
     Teacher routing (MT-122 operator clarification 2026-05-20):\n\
       --teacher-source CLI_BRIDGE    MT-127 governed CLI bridge (default)\n\
       --teacher-source BYOK          MT-125/126 BYOK lanes\n\
       --teacher-source LOCAL_LARGER  Local-larger-teacher model\n\
       --teacher <STRING>             Teacher model id or path (default: 'placeholder')\n\
     \n\
     Hyperparameters (defaults match production-baseline LoRA recipe):\n\
       --rank <N>                     LoRA rank (default 16)\n\
       --alpha <FLOAT>                LoRA alpha (default 32.0)\n\
       --dropout <FLOAT>              LoRA dropout (default 0.05)\n\
       --epochs <N>                   Training epochs (default 1)\n\
       --learning-rate <FLOAT>        Learning rate (default 2e-4)\n\
       --batch-size <N>               Batch size (default 4)\n\
       --max-steps <N>                Maximum training steps (default trainer-defined)\n\
     \n\
     Subprocess control:\n\
       --python <PATH>                Python interpreter (default: HANDSHAKE_PEFT_PYTHON or 'python')\n\
       --script <PATH>                Trainer script (default: <repo>/scripts/distill/train_lora.py)\n\
       --cpu-only                     Force CPU training (required on hosts without CUDA/MPS)\n\
     \n\
     Output:\n\
       Writes adapter_model.safetensors + adapter_config.json + provenance.json\n\
       to <out-lora>. Prints a JSON receipt with the provenance summary.\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_records_all_known_flags() {
        let args: Vec<String> = vec![
            "--corpus-jsonl", "corpus.jsonl",
            "--student-base", "synthetic",
            "--teacher-source", "BYOK",
            "--teacher", "anthropic-cli",
            "--out-lora", "lora_out",
            "--license-tag", "MIT",
            "--operator-signature", "op-ilja",
            "--max-steps", "2",
            "--learning-rate", "0.0001",
            "--rank", "8",
            "--alpha", "16",
            "--dropout", "0.1",
            "--epochs", "2",
            "--batch-size", "2",
            "--cpu-only",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let parsed = ParsedArgs::parse(args).expect("parse");
        let trainer = parsed.into_trainer_args().expect("trainer args");
        assert_eq!(trainer.corpus_jsonl.to_string_lossy(), "corpus.jsonl");
        assert_eq!(trainer.student_base.to_string_lossy(), "synthetic");
        assert_eq!(trainer.teacher.to_string_lossy(), "anthropic-cli");
        assert_eq!(trainer.teacher_source, TeacherSource::Byok);
        assert_eq!(trainer.out_lora.to_string_lossy(), "lora_out");
        assert_eq!(trainer.license_tag, "MIT");
        assert_eq!(trainer.operator_signature, "op-ilja");
        assert_eq!(trainer.max_steps, Some(2));
        assert_eq!(trainer.hyperparams.rank, 8);
        assert_eq!(trainer.hyperparams.alpha, 16.0);
        assert!((trainer.hyperparams.dropout - 0.1).abs() < f32::EPSILON);
        assert_eq!(trainer.hyperparams.epochs, 2);
        assert!((trainer.hyperparams.learning_rate - 1e-4).abs() < f32::EPSILON);
        assert_eq!(trainer.hyperparams.batch_size, 2);
        assert!(trainer.cpu_only);
    }

    #[test]
    fn parse_args_defaults_teacher_source_to_cli_bridge() {
        let args: Vec<String> = vec![
            "--corpus-jsonl", "c.jsonl",
            "--student-base", "synthetic",
            "--out-lora", "out",
            "--license-tag", "MIT",
            "--operator-signature", "op",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let parsed = ParsedArgs::parse(args).expect("parse");
        let trainer = parsed.into_trainer_args().expect("trainer args");
        assert_eq!(trainer.teacher_source, TeacherSource::CliBridge);
    }

    #[test]
    fn parse_args_rejects_empty_license_tag() {
        let args: Vec<String> = vec![
            "--corpus-jsonl", "c.jsonl",
            "--student-base", "synthetic",
            "--out-lora", "out",
            "--license-tag", "  ",
            "--operator-signature", "op",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let parsed = ParsedArgs::parse(args).expect("parse");
        let err = parsed.into_trainer_args().expect_err("empty license");
        assert!(err.contains("license-tag"));
    }

    #[test]
    fn parse_args_rejects_empty_operator_signature() {
        let args: Vec<String> = vec![
            "--corpus-jsonl", "c.jsonl",
            "--student-base", "synthetic",
            "--out-lora", "out",
            "--license-tag", "MIT",
            "--operator-signature", "  ",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let parsed = ParsedArgs::parse(args).expect("parse");
        let err = parsed.into_trainer_args().expect_err("empty signature");
        assert!(err.contains("operator-signature"));
    }

    #[test]
    fn parse_args_rejects_missing_required() {
        let args: Vec<String> = vec!["--corpus-jsonl", "c.jsonl"]
            .into_iter()
            .map(String::from)
            .collect();
        let parsed = ParsedArgs::parse(args).expect("parse");
        let err = parsed.into_trainer_args().expect_err("missing required");
        assert!(err.contains("--student-base") || err.contains("--out-lora"));
    }

    #[test]
    fn parse_args_rejects_unknown_flag() {
        let args: Vec<String> = vec!["--no-such-flag", "x"]
            .into_iter()
            .map(String::from)
            .collect();
        let err = ParsedArgs::parse(args).expect_err("unknown flag");
        assert!(err.contains("--no-such-flag"));
    }

    #[test]
    fn parse_args_teacher_source_accepts_lowercase_and_mixed() {
        let args: Vec<String> = vec![
            "--corpus-jsonl", "c.jsonl",
            "--student-base", "synthetic",
            "--out-lora", "out",
            "--license-tag", "MIT",
            "--operator-signature", "op",
            "--teacher-source", "local_larger",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let parsed = ParsedArgs::parse(args).expect("parse");
        let trainer = parsed.into_trainer_args().expect("trainer args");
        assert_eq!(trainer.teacher_source, TeacherSource::LocalLarger);
    }

    #[test]
    fn teacher_source_round_trips_through_cli_arg_value() {
        for variant in [
            TeacherSource::CliBridge,
            TeacherSource::Byok,
            TeacherSource::LocalLarger,
        ] {
            let arg = variant.cli_arg_value();
            let parsed = TeacherSource::from_cli_arg(arg).expect("round-trip");
            assert_eq!(parsed, variant);
        }
    }
}
