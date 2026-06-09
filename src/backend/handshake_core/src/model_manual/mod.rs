pub mod content;
pub mod projection;
pub mod types;

pub use content::{model_manual, MODEL_MANUAL};
pub use projection::render_model_manual_markdown;
pub use types::{
    CommandReference, CommandStatus, Manual, ManualCommand, ManualFeatureGroup,
    ManualSafetyConstraint, ManualWorkflow,
};

pub const MANUAL_VERSION: &str = "1.2.0";
