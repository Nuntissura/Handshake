use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Manual {
    pub version: &'static str,
    pub feature_groups: &'static [ManualFeatureGroup],
    pub command_reference: &'static [CommandReference],
    pub safety_constraints: &'static [ManualSafetyConstraint],
    pub workflows: &'static [ManualWorkflow],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ManualFeatureGroup {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub commands: &'static [&'static str],
}

pub type ManualCommand = CommandReference;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Wired,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CommandReference {
    pub id: &'static str,
    pub name: &'static str,
    pub status: CommandStatus,
    pub ipc_channel: Option<&'static str>,
    pub tauri_command: Option<&'static str>,
    pub schema_fields: &'static [&'static str],
    pub cli_flag: Option<&'static str>,
    pub description: &'static str,
    pub expected_input: &'static str,
    pub expected_output: &'static str,
    pub common_errors: &'static [&'static str],
    pub recovery_steps: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ManualWorkflow {
    pub id: &'static str,
    pub title: &'static str,
    pub prerequisites: &'static [&'static str],
    pub steps: &'static [&'static str],
    pub expected_outcome: &'static str,
    pub failure_modes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ManualSafetyConstraint {
    pub id: &'static str,
    pub constraint_text: &'static str,
    pub enforcement_point: &'static str,
}
