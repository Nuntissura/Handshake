use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

use super::errors::{McpError, McpResult};

pub fn validate_instance(schema: &Value, instance: &Value) -> McpResult<()> {
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .map_err(|source| McpError::SchemaValidation {
            details: source.to_string(),
        })?;

    if let Err(errors) = compiled.validate(instance) {
        let mut messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        messages.sort();
        let details = messages
            .into_iter()
            .map(|m| format!("- {m}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(McpError::SchemaValidation { details });
    }
    Ok(())
}
