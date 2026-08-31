//! Legacy MT-013 closure path; MT-006 scope proof is embedded-SurrealDB-only.

#![cfg(all(feature = "test-utils", feature = "surreal-test-support"))]

include!("model_lane_cloud_consent_scope_surreal_tests.rs");
