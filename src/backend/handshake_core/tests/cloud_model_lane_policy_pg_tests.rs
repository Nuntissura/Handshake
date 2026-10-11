//! Legacy MT-013 closure path; MT-006 proof is embedded-SurrealDB-only.

#![cfg(all(feature = "test-utils", feature = "surreal-test-support"))]

include!("cloud_model_lane_policy_surreal_tests.rs");
