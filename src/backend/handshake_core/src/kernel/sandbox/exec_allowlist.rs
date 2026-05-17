//! MT-024 Process Execution Allowlist.
//!
//! Acceptance (MT-024.json): "permit only registered commands/checks.
//! Acceptance: raw shell strings without descriptors are rejected."
//!
//! A `CommandDescriptorV1` is the *only* shape callers may hand to the sandbox
//! when asking it to spawn a process. Descriptors carry a stable id, an
//! argv-vector (no shell), a purpose tag, and a provenance reference. Raw
//! shell strings, single-string `bash -c "..."` payloads, and descriptors with
//! shell metacharacters are rejected.
//!
//! The gate works in two parts:
//!   * `validate_descriptor` enforces argv-only shape on a single descriptor.
//!   * `ExecAllowlistGate::check(...)` looks up the descriptor id in the policy
//!     allowlist and returns a typed denial if missing.

use serde::{Deserialize, Serialize};

use super::denial::{DenialKind, SandboxDenialRecordV1};
use super::policy::SandboxCapability;
use super::policy_default_deny::{CommandDescriptorRefV1, ProcessExecAllowlistV1};
use super::run::SandboxRunV1;

/// Fully-typed command descriptor. Argv-only; no shell layer is allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptorV1 {
    pub descriptor_id: String,
    pub program: String,
    pub args: Vec<String>,
    pub purpose_tag: String,
    pub provenance_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptorValidationError {
    EmptyProgram,
    ShellInvocation { detail: String },
    ShellMetacharacter { detail: String },
    MissingProvenance,
    MissingPurposeTag,
}

impl DescriptorValidationError {
    pub fn as_reason(&self) -> String {
        match self {
            Self::EmptyProgram => "descriptor has empty `program`".to_string(),
            Self::ShellInvocation { detail } => {
                format!("descriptor looks like raw shell invocation: {}", detail)
            }
            Self::ShellMetacharacter { detail } => {
                format!("descriptor contains shell metacharacter: {}", detail)
            }
            Self::MissingProvenance => "descriptor has empty `provenance_ref`".to_string(),
            Self::MissingPurposeTag => "descriptor has empty `purpose_tag`".to_string(),
        }
    }
}

const SHELL_PROGRAMS: &[&str] = &[
    "sh", "bash", "zsh", "dash", "ksh", "fish", "cmd", "cmd.exe", "powershell", "powershell.exe",
    "pwsh", "pwsh.exe",
    // H-A2 fix: BusyBox dispatches to multiple shells; wsl.exe can launch shells inside WSL.
    "busybox", "wsl.exe",
];
const SHELL_METACHARS: &[char] = &[';', '|', '&', '>', '<', '`', '$'];

/// H-A2 fix: language interpreters that accept `-c|-e|-r`-style code payloads.
/// `python -c "import os; os.system(...)"` is functionally equivalent to a
/// shell invocation and MUST be rejected when the descriptor mixes the two.
/// The forbidden flag is matched case-insensitively against any arg.
const INTERPRETER_CODE_PAYLOAD_PROGRAMS: &[(&str, &[&str])] = &[
    ("python", &["-c"]),
    ("python.exe", &["-c"]),
    ("python3", &["-c"]),
    ("python3.exe", &["-c"]),
    ("node", &["-e", "--eval"]),
    ("node.exe", &["-e", "--eval"]),
    ("perl", &["-e"]),
    ("perl.exe", &["-e"]),
    ("ruby", &["-e"]),
    ("ruby.exe", &["-e"]),
    ("php", &["-r"]),
    ("php.exe", &["-r"]),
];

/// Validate a single descriptor's *shape*. Does NOT check the allowlist; that
/// is `ExecAllowlistGate::check`.
pub fn validate_descriptor(d: &CommandDescriptorV1) -> Result<(), DescriptorValidationError> {
    if d.program.trim().is_empty() {
        return Err(DescriptorValidationError::EmptyProgram);
    }
    if d.purpose_tag.trim().is_empty() {
        return Err(DescriptorValidationError::MissingPurposeTag);
    }
    if d.provenance_ref.trim().is_empty() {
        return Err(DescriptorValidationError::MissingProvenance);
    }

    let program_lower = std::path::Path::new(&d.program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&d.program)
        .to_ascii_lowercase();

    if SHELL_PROGRAMS.contains(&program_lower.as_str()) {
        // Allowed only if no `-c`-style payload is present.
        let has_command_flag = d.args.iter().any(|a| {
            let lower = a.to_ascii_lowercase();
            lower == "-c"
                || lower == "/c"
                || lower == "-command"
                || lower == "-encodedcommand"
                || lower == "/k"
        });
        if has_command_flag {
            return Err(DescriptorValidationError::ShellInvocation {
                detail: format!(
                    "shell program `{}` invoked with command-string flag",
                    d.program
                ),
            });
        }
    }

    // H-A2 fix: language interpreters with -c|-e|-r are functional shell
    // invocations. python -c "import os; os.system('rm -rf /')" used to slip
    // the SHELL_PROGRAMS check.
    if let Some((_, forbidden_flags)) = INTERPRETER_CODE_PAYLOAD_PROGRAMS
        .iter()
        .find(|(name, _)| *name == program_lower)
    {
        let has_code_flag = d.args.iter().any(|a| {
            let lower = a.to_ascii_lowercase();
            forbidden_flags.iter().any(|f| *f == lower.as_str())
        });
        if has_code_flag {
            return Err(DescriptorValidationError::ShellInvocation {
                detail: format!(
                    "interpreter `{}` invoked with code-payload flag",
                    d.program
                ),
            });
        }
    }

    // Reject argv segments that contain shell metacharacters; argv is supposed
    // to be the post-tokenisation form.
    for a in std::iter::once(&d.program).chain(d.args.iter()) {
        if let Some(c) = a.chars().find(|c| SHELL_METACHARS.contains(c)) {
            return Err(DescriptorValidationError::ShellMetacharacter {
                detail: format!("argv element `{}` contains `{}`", a, c),
            });
        }
    }

    Ok(())
}

pub struct ExecAllowlistGate<'a> {
    allowlist: &'a ProcessExecAllowlistV1,
}

impl<'a> ExecAllowlistGate<'a> {
    pub fn new(allowlist: &'a ProcessExecAllowlistV1) -> Self {
        Self { allowlist }
    }

    pub fn check(
        &self,
        run: &SandboxRunV1,
        descriptor: &CommandDescriptorV1,
    ) -> Result<CommandDescriptorRefV1, SandboxDenialRecordV1> {
        // Shape first.
        if let Err(shape_err) = validate_descriptor(descriptor) {
            return Err(SandboxDenialRecordV1::new(
                run.run_id.0.clone(),
                run.policy_version_id.clone(),
                DenialKind::PolicyDenied,
                Some(SandboxCapability::ProcessSpawn),
                format!(
                    "exec descriptor `{}` rejected",
                    descriptor.descriptor_id
                ),
                shape_err.as_reason(),
            ));
        }
        // Allowlist lookup.
        let entry = self
            .allowlist
            .commands
            .iter()
            .find(|c| c.descriptor_id == descriptor.descriptor_id);
        match entry {
            Some(e) => Ok(e.clone()),
            None => Err(SandboxDenialRecordV1::new(
                run.run_id.0.clone(),
                run.policy_version_id.clone(),
                DenialKind::PolicyDenied,
                Some(SandboxCapability::ProcessSpawn),
                format!(
                    "exec descriptor `{}` rejected",
                    descriptor.descriptor_id
                ),
                if self.allowlist.commands.is_empty() {
                    "exec allowlist is empty; default-deny applies".to_string()
                } else {
                    format!(
                        "descriptor id `{}` is not in policy exec allowlist",
                        descriptor.descriptor_id
                    )
                },
            )),
        }
    }
}

/// Convenience: reject a raw shell string with a typed denial. This is the
/// "raw shell strings without descriptors are rejected" acceptance criterion
/// path that callers hit before they have any descriptor at all.
pub fn reject_raw_shell_string(run: &SandboxRunV1, raw: &str) -> SandboxDenialRecordV1 {
    SandboxDenialRecordV1::new(
        run.run_id.0.clone(),
        run.policy_version_id.clone(),
        DenialKind::PolicyDenied,
        Some(SandboxCapability::ProcessSpawn),
        format!("raw shell string `{}`", raw),
        "raw shell strings are forbidden; submit a CommandDescriptorV1 instead".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::policy_default_deny::CommandDescriptorRefV1;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "exec", "POL-1@1", "WSP-1")
    }

    fn ok_descriptor() -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            descriptor_id: "cmd_cargo_check".into(),
            program: "cargo".into(),
            args: vec!["check".into(), "--release".into()],
            purpose_tag: "proof.compile".into(),
            provenance_ref: "WP-KERNEL-003".into(),
        }
    }

    #[test]
    fn raw_shell_string_is_rejected_with_typed_denial() {
        let d = reject_raw_shell_string(&run(), "rm -rf /");
        assert_eq!(d.kind, DenialKind::PolicyDenied);
        assert_eq!(d.capability, Some(SandboxCapability::ProcessSpawn));
        assert!(d.reason.contains("CommandDescriptorV1"));
    }

    #[test]
    fn descriptor_without_provenance_is_rejected() {
        let mut d = ok_descriptor();
        d.provenance_ref = "".into();
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(err, DescriptorValidationError::MissingProvenance);
    }

    #[test]
    fn bash_dash_c_is_rejected_as_shell_invocation() {
        let d = CommandDescriptorV1 {
            descriptor_id: "evil".into(),
            program: "bash".into(),
            args: vec!["-c".into(), "echo hi".into()],
            purpose_tag: "x".into(),
            provenance_ref: "y".into(),
        };
        let err = validate_descriptor(&d).unwrap_err();
        match err {
            DescriptorValidationError::ShellInvocation { .. } => {}
            other => panic!("expected ShellInvocation, got {:?}", other),
        }
    }

    #[test]
    fn pipe_metachar_in_argv_is_rejected() {
        let mut d = ok_descriptor();
        d.args.push("foo|bar".into());
        let err = validate_descriptor(&d).unwrap_err();
        match err {
            DescriptorValidationError::ShellMetacharacter { .. } => {}
            other => panic!("expected ShellMetacharacter, got {:?}", other),
        }
    }

    #[test]
    fn descriptor_not_in_allowlist_is_typed_denial() {
        let allowlist = ProcessExecAllowlistV1::default();
        let gate = ExecAllowlistGate::new(&allowlist);
        let d = ok_descriptor();
        let den = gate.check(&run(), &d).expect_err("empty allowlist must deny");
        assert_eq!(den.kind, DenialKind::PolicyDenied);
        assert!(den.reason.contains("default-deny"));
    }

    #[test]
    fn descriptor_in_allowlist_passes() {
        let allowlist = ProcessExecAllowlistV1 {
            commands: vec![CommandDescriptorRefV1 {
                descriptor_id: "cmd_cargo_check".into(),
                purpose_tag: "proof.compile".into(),
            }],
        };
        let gate = ExecAllowlistGate::new(&allowlist);
        let d = ok_descriptor();
        let entry = gate
            .check(&run(), &d)
            .expect("allowlisted descriptor must pass");
        assert_eq!(entry.descriptor_id, "cmd_cargo_check");
    }

    #[test]
    fn ok_descriptor_validates_clean() {
        validate_descriptor(&ok_descriptor()).expect("ok descriptor must validate");
    }

    #[test]
    fn interpreter_with_code_payload_is_rejected() {
        // H-A2 regression guard. Each entry is (program, flag) that used to
        // slip through the SHELL_PROGRAMS-only gate.
        let cases = [
            ("python", "-c", "import os; os.system('id')"),
            ("python3", "-c", "print(1)"),
            ("node", "-e", "process.exit(1)"),
            ("node", "--eval", "process.exit(1)"),
            ("perl", "-e", "print 'pwn'"),
            ("ruby", "-e", "system('id')"),
            ("php", "-r", "system('id');"),
        ];
        for (prog, flag, code) in cases {
            let d = CommandDescriptorV1 {
                descriptor_id: "evil_interpreter".into(),
                program: prog.into(),
                args: vec![flag.into(), code.into()],
                purpose_tag: "x".into(),
                provenance_ref: "y".into(),
            };
            let err = validate_descriptor(&d).expect_err(prog);
            match err {
                DescriptorValidationError::ShellInvocation { .. } => {}
                other => panic!("expected ShellInvocation for {}, got {:?}", prog, other),
            }
        }
    }

    #[test]
    fn busybox_and_wsl_are_treated_as_shells() {
        // H-A2 regression guard: BusyBox and wsl.exe can dispatch to shells.
        for prog in ["busybox", "wsl.exe"] {
            let d = CommandDescriptorV1 {
                descriptor_id: "evil_dispatcher".into(),
                program: prog.into(),
                args: vec!["-c".into(), "id".into()],
                purpose_tag: "x".into(),
                provenance_ref: "y".into(),
            };
            let err = validate_descriptor(&d).expect_err(prog);
            match err {
                DescriptorValidationError::ShellInvocation { .. } => {}
                other => panic!("expected ShellInvocation for {}, got {:?}", prog, other),
            }
        }
    }

    #[test]
    fn interpreter_without_code_flag_is_allowed_by_shape() {
        // `python script.py` (running a real file) is NOT a shell invocation —
        // it's a normal program execution. The gate should only fire on -c|-e|-r.
        let d = CommandDescriptorV1 {
            descriptor_id: "python_script".into(),
            program: "python".into(),
            args: vec!["scripts/run.py".into()],
            purpose_tag: "proof.run".into(),
            provenance_ref: "WP-KERNEL-003".into(),
        };
        validate_descriptor(&d).expect("interpreter w/o code flag is fine by shape");
    }
}
