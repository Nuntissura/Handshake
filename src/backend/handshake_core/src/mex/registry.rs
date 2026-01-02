use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mex::envelope::{BudgetSpec, DeterminismLevel};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Failed to read registry: {0}")]
    Io(String),
    #[error("Failed to parse registry: {0}")]
    Parse(String),
}

/// Operation-level metadata for an engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationSpec {
    pub name: String,
    pub schema_ref: Option<String>,
    #[serde(default)]
    pub params_schema: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub output_types: Vec<String>,
}

/// Engine definition loaded from mechanical_engines.json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineSpec {
    pub engine_id: String,
    pub determinism_ceiling: DeterminismLevel,
    #[serde(default)]
    pub required_caps: Vec<String>,
    #[serde(default)]
    pub required_gates: Vec<String>,
    #[serde(default)]
    pub default_budget: BudgetSpec,
    #[serde(default)]
    pub ops: Vec<OperationSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct MexRegistry {
    engines: HashMap<String, EngineSpec>,
}

impl MexRegistry {
    pub fn load_from_path(path: &Path) -> Result<Self, RegistryError> {
        let data = fs::read_to_string(path)
            .map_err(|err| RegistryError::Io(format!("{} ({})", path.display(), err)))?;
        let map: HashMap<String, EngineSpec> =
            serde_json::from_str(&data).map_err(|err| RegistryError::Parse(err.to_string()))?;

        let mut engines = HashMap::new();
        for (engine_id, mut spec) in map {
            spec.engine_id = engine_id.clone();
            engines.insert(engine_id, spec);
        }

        Ok(Self { engines })
    }

    pub fn from_map(map: HashMap<String, EngineSpec>) -> Self {
        Self { engines: map }
    }

    pub fn get_engine(&self, engine_id: &str) -> Option<&EngineSpec> {
        self.engines.get(engine_id)
    }

    pub fn get_operation(&self, engine_id: &str, operation: &str) -> Option<&OperationSpec> {
        self.engines
            .get(engine_id)
            .and_then(|engine| engine.ops.iter().find(|op| op.name == operation))
    }

    pub fn engines(&self) -> impl Iterator<Item = &EngineSpec> {
        self.engines.values()
    }
}
