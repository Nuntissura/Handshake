//! Native Facial plugin boundary.
//!
//! MT-019 establishes the registry/run backbone. Feature-specific parity modules
//! attach here in later microtasks after the registry and run contract are stable.

pub const PLUGIN_BACKBONE_STATUS: &str = "registry_ready_feature_modules_deferred";
