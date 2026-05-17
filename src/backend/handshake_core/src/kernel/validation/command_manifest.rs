//! MT-037: Command manifest.
//!
//! Acceptance: validators can replay or reason about command intent. The
//! manifest records exactly which commands/checks ran during validation,
//! each tagged with a typed `CommandIntent`. Replay-equivalence requires
//! capturing the program, argv, and working-directory hint (we deliberately
//! avoid full env capture here — that's MT-036's manifest).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandIntent {
    /// Building the candidate (compile/check/lint at build time).
    Build,
    /// Running tests.
    Test,
    /// Static analysis / lints.
    Lint,
    /// Custom check declared by the descriptor (e.g., schema validation).
    DescriptorCheck,
    /// Validation runner internal bookkeeping (artifact capture, etc.).
    Runner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    pub intent: CommandIntent,
    pub program: String,
    pub argv: Vec<String>,
    pub workdir_hint: Option<String>,
    pub descriptor: Option<String>,
}

impl CommandRecord {
    pub fn new(intent: CommandIntent, program: impl Into<String>, argv: Vec<String>) -> Self {
        Self {
            intent,
            program: program.into(),
            argv,
            workdir_hint: None,
            descriptor: None,
        }
    }

    pub fn with_workdir(mut self, dir: impl Into<String>) -> Self {
        self.workdir_hint = Some(dir.into());
        self
    }

    pub fn with_descriptor(mut self, name: impl Into<String>) -> Self {
        self.descriptor = Some(name.into());
        self
    }

    /// Stable replay key — equal programs/argv/workdir produce equal keys
    /// regardless of which descriptor invoked them.
    pub fn replay_key(&self) -> String {
        let argv_joined = self.argv.join("\x1f");
        match &self.workdir_hint {
            Some(wd) => format!("{}\x1e{}\x1e{}", self.program, argv_joined, wd),
            None => format!("{}\x1e{}", self.program, argv_joined),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommandManifest {
    pub commands: Vec<CommandRecord>,
}

impl CommandManifest {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, cmd: CommandRecord) {
        self.commands.push(cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_captures_intent_and_argv() {
        let mut m = CommandManifest::new();
        m.push(
            CommandRecord::new(
                CommandIntent::Test,
                "cargo",
                vec!["test".into(), "-p".into(), "handshake_core".into()],
            )
            .with_workdir("src/backend/handshake_core")
            .with_descriptor("artifact_hashes_valid"),
        );
        assert_eq!(m.commands.len(), 1);
        assert_eq!(m.commands[0].intent, CommandIntent::Test);
        assert_eq!(m.commands[0].argv.len(), 3);
        assert_eq!(
            m.commands[0].descriptor.as_deref(),
            Some("artifact_hashes_valid")
        );
    }

    #[test]
    fn replay_key_is_stable_across_descriptor_attribution() {
        let a = CommandRecord::new(CommandIntent::Lint, "cargo", vec!["clippy".into()])
            .with_workdir("src/backend/handshake_core")
            .with_descriptor("d_one");
        let b = CommandRecord::new(CommandIntent::Lint, "cargo", vec!["clippy".into()])
            .with_workdir("src/backend/handshake_core")
            .with_descriptor("d_two");
        assert_eq!(a.replay_key(), b.replay_key());
    }
}
