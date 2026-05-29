use std::{error::Error, fs, path::Path};

use handshake_core::model_runtime::{KvQuantSupport, ModelCapabilities, ModelRuntimeError};
use serde_json::json;

#[test]
fn model_runtime_contract_tests_root_exports_contract_types() {
    let _capabilities = ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
        supports_activation_steering: true,
        supports_subquadratic: true,
        supports_speculative_draft: true,
        supports_eagle3: false,
    };
    let _error = ModelRuntimeError::Cancelled;
}

#[test]
fn model_runtime_contract_tests_error_variants_are_public_and_error_typed() {
    fn assert_error_traits<T: Error + Send + Sync + 'static>() {}
    assert_error_traits::<ModelRuntimeError>();

    let variants = [
        ModelRuntimeError::LoadError("load".to_string()),
        ModelRuntimeError::UnloadError("unload".to_string()),
        ModelRuntimeError::GenerateError("generate".to_string()),
        ModelRuntimeError::ScoreError("score".to_string()),
        ModelRuntimeError::EmbedError("embed".to_string()),
        ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache".to_string(),
            adapter: "candle".to_string(),
        },
        ModelRuntimeError::KvCacheError("kv".to_string()),
        ModelRuntimeError::LoraStackError("lora".to_string()),
        ModelRuntimeError::SteeringHookError("steering".to_string()),
        ModelRuntimeError::Cancelled,
        ModelRuntimeError::AdapterMismatch {
            expected: "llama_cpp".to_string(),
            got: "candle".to_string(),
        },
    ];

    assert_eq!(variants.len(), 11);
    for variant in variants {
        assert!(
            !variant.to_string().trim().is_empty(),
            "ModelRuntimeError variants must have stable display text"
        );
    }
}

#[test]
fn model_runtime_contract_tests_capabilities_serde_shape_is_stable() {
    let capabilities = ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
        supports_activation_steering: false,
        supports_subquadratic: true,
        supports_speculative_draft: false,
        supports_eagle3: true,
    };

    assert_eq!(
        serde_json::to_value(&capabilities).expect("capabilities serialize"),
        json!({
            "supports_lora": true,
            "supports_kv_prefix_cache": true,
            "supports_kv_quantization": "q4_q8_mix",
            "supports_activation_steering": false,
            "supports_subquadratic": true,
            "supports_speculative_draft": false,
            "supports_eagle3": true
        })
    );

    for (support, expected) in [
        (KvQuantSupport::None, "none"),
        (KvQuantSupport::Q4, "q4"),
        (KvQuantSupport::Q8, "q8"),
        (KvQuantSupport::Q4Q8Mix, "q4_q8_mix"),
    ] {
        assert_eq!(
            serde_json::to_value(support).expect("kv quant serializes"),
            json!(expected)
        );
    }

    let round_trip: ModelCapabilities =
        serde_json::from_value(serde_json::to_value(&capabilities).unwrap()).unwrap();
    assert_eq!(round_trip, capabilities);
    assert!(
        serde_json::from_str::<KvQuantSupport>(r#""unknown""#).is_err(),
        "unknown KV quantization variants must fail closed"
    );
}

#[test]
fn model_runtime_contract_tests_manifest_declares_engine_deps_without_sqlite() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read Cargo.toml");
    let normalized = manifest.to_ascii_lowercase();

    for (dependency, version) in [
        ("llama-cpp-2", "0.1.146"),
        ("candle-core", "0.10.2"),
        ("candle-transformers", "0.10.2"),
        ("tokenizers", "0.23.1"),
        ("futures", "0.3"),
        ("async-trait", "0.1"),
        ("tokio", "1"),
        ("serde", "1"),
        ("serde_json", "1"),
        ("thiserror", "2.0.17"),
    ] {
        assert_manifest_declares_dependency(&manifest, dependency, version);
    }

    assert!(
        !normalized.contains("rusqlite") && !normalized.contains("libsqlite3-sys"),
        "ModelRuntime dependency scaffold must not introduce SQLite crates"
    );

    let sqlx_line = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("sqlx ="))
        .expect("existing sqlx dependency remains declared");
    assert!(
        !sqlx_line.to_ascii_lowercase().contains("sqlite"),
        "sqlx dependency must remain PostgreSQL-only, not SQLite-enabled"
    );
}

#[test]
fn model_runtime_contract_tests_public_surface_is_engine_agnostic() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        ["src", "model_runtime", "mod.rs"],
        ["src", "model_runtime", "error.rs"],
        ["src", "model_runtime", "capabilities.rs"],
    ] {
        let path = relative
            .iter()
            .fold(manifest_dir.to_path_buf(), |acc, item| acc.join(item));
        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "read model_runtime scaffold file {}: {error}",
                path.display()
            )
        });
        let normalized = source.to_ascii_lowercase();

        for banned in ["llama_cpp_2::", "candle_core::", "candle_transformers::"] {
            assert!(
                !normalized.contains(banned),
                "model_runtime scaffold public surface must not leak engine-specific type `{banned}` in {}",
                path.display()
            );
        }
    }
}

fn assert_manifest_declares_dependency(manifest: &str, dependency: &str, version: &str) {
    let dependency_line = manifest
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("{dependency} =")))
        .unwrap_or_else(|| panic!("Cargo.toml must declare `{dependency}`"));
    assert!(
        dependency_line.contains(&format!("\"{version}\"")),
        "Cargo.toml dependency `{dependency}` must use version `{version}`; got `{dependency_line}`"
    );
}
