//! MT-106: INF-6 abliterate hot-path invariant + safetensors round-trip.
//!
//! Two test classes:
//!
//! 1. **Hot-path invariant (always-runs static analysis)**: walks the
//!    runtime `generate.rs` files and asserts that no reference to the
//!    `distillation::abliterate` module appears. Per Master Spec §4.7.4
//!    and MT-106 red_team minimum_controls, abliteration is OFFLINE
//!    only; finding it in a generate path is an HBR-INT-002 violation.
//!
//! 2. **Safetensors round-trip (feature-gated on `candle-runtime-engine`)**:
//!    builds a tiny safetensors fixture with o_proj + down_proj +
//!    untouched-bias tensors, runs `run_abliteration_offline` against
//!    it, reads back the output, and asserts the target Linear weights
//!    were actually orthogonalised, the bias and non-target weights
//!    were left untouched, the provenance sidecar was written, and
//!    when a `LedgerBatcher` is supplied the
//!    engine_kind=AbliterationTool row registration is actually
//!    drained — exactly the gap the deflection note flagged on MT-106
//!    v1.

use std::{fs, path::Path, path::PathBuf};

const FORBIDDEN_REFERENCES: &[&str] = &[
    "distillation::abliterate",
    "abliterate::orthogonalise",
    "abliterate::run_abliteration_offline",
];

fn runtime_generate_files() -> Vec<PathBuf> {
    let core = Path::new(env!("CARGO_MANIFEST_DIR"));
    vec![
        core.join("src/model_runtime/llama_cpp/generate.rs"),
        core.join("src/model_runtime/candle/generate.rs"),
    ]
}

#[test]
fn hot_path_does_not_reference_abliterate_module() {
    let mut violations: Vec<String> = Vec::new();
    for path in runtime_generate_files() {
        if !path.exists() {
            // Per MT-100 / MT-103 some generate paths may not exist on
            // every host; missing file is not a violation. The test
            // still gates files that DO exist.
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        for needle in FORBIDDEN_REFERENCES {
            if source.contains(needle) {
                violations.push(format!(
                    "{} contains forbidden reference {needle} (HBR-INT-002: \
                     abliterate is OFFLINE TOOL ONLY per Master Spec §4.7.4)",
                    path.display(),
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "abliterate hot-path invariant violated:\n{}",
        violations.join("\n")
    );
}

#[test]
fn hot_path_invariant_test_actually_walks_at_least_one_runtime_file() {
    // Sanity guard: if NEITHER generate.rs exists on this host the
    // hot-path test above would be vacuously true. The static-analysis
    // test must walk at least one file for its assertion to mean
    // anything.
    let any_exists = runtime_generate_files().iter().any(|p| p.exists());
    assert!(
        any_exists,
        "expected at least one of {:?} to exist; if both are pending \
         (MT-074 not yet unblocked), the hot-path invariant test is \
         currently vacuous and should be re-run after each generate.rs \
         is added.",
        runtime_generate_files()
    );
}

#[cfg(feature = "candle-runtime-engine")]
mod safetensors_round_trip {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use candle_core::{DType, Device, Tensor};
    use handshake_core::distillation::abliterate::{
        is_abliteration_target_module, provenance_sidecar_path, run_abliteration_offline,
        weight_is_orthogonal_to, AbliterationConfig, AbliterationError, AbliterationProvenance,
        RefusalDirectionFile, ABLITERATION_TOOL_VERSION,
    };
    use handshake_core::process_ledger::{
        LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, ProcessEngineKind,
        ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore,
    };

    // Linear weight shape: rows = arbitrary "hidden" size, cols = the
    // refusal-direction width. Tiny on purpose so the fixture is fast.
    const ROWS: usize = 4;
    const COLS: usize = 6;

    fn unit_direction(cols: usize) -> Vec<f32> {
        let mut v = vec![1.0_f32; cols];
        let norm = (cols as f32).sqrt();
        for x in v.iter_mut() {
            *x /= norm;
        }
        v
    }

    fn make_fixture_safetensors(
        path: &std::path::Path,
        o_proj_weights: &[f32],
        down_proj_weights: &[f32],
        non_target_weights: &[f32],
        o_proj_bias: &[f32],
    ) {
        let device = Device::Cpu;
        let mut tensors: HashMap<String, Tensor> = HashMap::new();

        // Two target Linear weights at distinct layers — guards
        // against accidentally only touching the first match.
        tensors.insert(
            "model.layers.0.self_attn.o_proj.weight".to_string(),
            Tensor::from_slice(o_proj_weights, (ROWS, COLS), &device).unwrap(),
        );
        tensors.insert(
            "model.layers.3.mlp.down_proj.weight".to_string(),
            Tensor::from_slice(down_proj_weights, (ROWS, COLS), &device).unwrap(),
        );

        // Untouched non-target Linear weight (q_proj is not in the
        // abliteration target set).
        tensors.insert(
            "model.layers.0.self_attn.q_proj.weight".to_string(),
            Tensor::from_slice(non_target_weights, (ROWS, COLS), &device).unwrap(),
        );

        // Bias on the same module name — must not be transformed even
        // though the name contains `.o_proj.`.
        tensors.insert(
            "model.layers.0.self_attn.o_proj.bias".to_string(),
            Tensor::from_slice(o_proj_bias, ROWS, &device).unwrap(),
        );

        candle_core::safetensors::save(&tensors, path).unwrap();
    }

    fn read_back_2d(path: &std::path::Path, key: &str) -> Vec<f32> {
        let device = Device::Cpu;
        let tensors = candle_core::safetensors::load(path, &device).unwrap();
        let tensor = tensors
            .get(key)
            .unwrap_or_else(|| panic!("expected key {key} in {}", path.display()));
        assert_eq!(tensor.dtype(), DType::F32);
        assert_eq!(tensor.shape().dims(), &[ROWS, COLS]);
        tensor
            .flatten_all()
            .and_then(|t| t.to_vec1::<f32>())
            .unwrap()
    }

    fn read_back_1d(path: &std::path::Path, key: &str) -> Vec<f32> {
        let device = Device::Cpu;
        let tensors = candle_core::safetensors::load(path, &device).unwrap();
        let tensor = tensors
            .get(key)
            .unwrap_or_else(|| panic!("expected key {key} in {}", path.display()));
        assert_eq!(tensor.dtype(), DType::F32);
        tensor.to_vec1::<f32>().unwrap()
    }

    fn write_refusal_direction(path: &std::path::Path, values: &[f32]) {
        let file = RefusalDirectionFile {
            layer: 14,
            values: values.to_vec(),
            source_model_sha256: Some("ff".repeat(32)),
        };
        std::fs::write(path, serde_json::to_vec_pretty(&file).unwrap()).unwrap();
    }

    #[derive(Clone, Default)]
    struct InMemoryProcessLedgerStore {
        events: Arc<Mutex<Vec<LedgerEvent>>>,
    }

    impl InMemoryProcessLedgerStore {
        fn events(&self) -> Vec<LedgerEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ProcessLedgerStore for InMemoryProcessLedgerStore {
        async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
            self.events.lock().unwrap().extend(events);
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct RecordingOverflowSink {
        events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
    }

    impl ProcessLedgerOverflowSink for RecordingOverflowSink {
        fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    /// Builds a tiny safetensors fixture, runs
    /// `run_abliteration_offline` against it with a `LedgerBatcher`
    /// supplied, then asserts:
    ///   - the target o_proj + down_proj weights were orthogonalised
    ///     against the refusal direction (every row's projection onto
    ///     it is zero within tolerance);
    ///   - the non-target q_proj weight is byte-identical to the input;
    ///   - the o_proj bias is byte-identical to the input;
    ///   - the provenance sidecar JSON exists and matches the
    ///     returned `AbliterationProvenance`;
    ///   - the provenance carries the expected target keys, license
    ///     tag, operator signature, and a non-empty
    ///     process_ledger_record_id;
    ///   - the ProcessOwnershipLedger drain contains an
    ///     engine_kind=AbliterationTool start event with our pid.
    #[tokio::test]
    async fn abliterate_round_trips_against_safetensors_fixture() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let base_path = tempdir.path().join("base.safetensors");
        let direction_path = tempdir.path().join("refusal_direction.json");
        let out_path = tempdir.path().join("abliterated.safetensors");

        let direction = unit_direction(COLS);

        // o_proj: every row aligned with the direction; should
        // collapse to ~0 after abliteration.
        let mut o_proj = Vec::new();
        for _ in 0..ROWS {
            o_proj.extend_from_slice(&direction);
        }
        // down_proj: arbitrary "mostly aligned" rows.
        let down_proj: Vec<f32> = (0..ROWS * COLS).map(|i| (i as f32) * 0.1 + 1.0).collect();
        // q_proj: arbitrary; must not change at all.
        let q_proj: Vec<f32> = (0..ROWS * COLS).map(|i| -(i as f32) * 0.2 + 3.0).collect();
        // bias: must not change.
        let bias: Vec<f32> = (0..ROWS).map(|i| 0.5_f32 + i as f32).collect();

        make_fixture_safetensors(&base_path, &o_proj, &down_proj, &q_proj, &bias);
        write_refusal_direction(&direction_path, &direction);

        let config = AbliterationConfig {
            base_model_path: base_path.clone(),
            refusal_direction_path: direction_path.clone(),
            out_model_path: out_path.clone(),
            license_tag: "Permissive-Test".to_string(),
            provenance_note: "MT-106 integration test".to_string(),
            operator_signature: "operator-test-MT-106".to_string(),
        };

        // Set up a manual-mode LedgerBatcher so we can drain and
        // inspect what was registered. Production callers use
        // LedgerBatcher::spawn with a Postgres store.
        let overflow_concrete = Arc::new(RecordingOverflowSink::default());
        let overflow_sink: Arc<dyn ProcessLedgerOverflowSink> = overflow_concrete.clone();
        let (batcher, drain) =
            LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), overflow_sink)
                .expect("manual_for_tests");
        let store = Arc::new(InMemoryProcessLedgerStore::default());

        let provenance =
            run_abliteration_offline(&config, Some(&batcher)).expect("abliteration succeeds");

        // ---- assertions on the returned provenance ----
        assert_eq!(
            provenance.abliteration_tool_version,
            ABLITERATION_TOOL_VERSION
        );
        assert_eq!(provenance.license_tag, "Permissive-Test");
        assert_eq!(provenance.operator_signature, "operator-test-MT-106");
        assert_eq!(provenance.provenance_note, "MT-106 integration test");
        assert!(
            provenance.base_model_sha256.len() == 64,
            "base_model_sha256 must be a 64-hex-char SHA-256: {}",
            provenance.base_model_sha256
        );
        assert!(
            provenance.refusal_direction_sha256.len() == 64,
            "refusal_direction_sha256 must be a 64-hex-char SHA-256: {}",
            provenance.refusal_direction_sha256
        );
        assert_eq!(
            provenance.orthogonalised_weight_keys,
            vec![
                "model.layers.0.self_attn.o_proj.weight".to_string(),
                "model.layers.3.mlp.down_proj.weight".to_string(),
            ],
            "expected exactly o_proj + down_proj target weights to be transformed"
        );
        assert!(
            provenance.process_ledger_record_id.is_some(),
            "process_ledger_record_id must be set when a LedgerBatcher is supplied"
        );

        // ---- assertions on the on-disk output safetensors ----
        let out_o_proj = read_back_2d(&out_path, "model.layers.0.self_attn.o_proj.weight");
        let out_down_proj = read_back_2d(&out_path, "model.layers.3.mlp.down_proj.weight");
        let out_q_proj = read_back_2d(&out_path, "model.layers.0.self_attn.q_proj.weight");
        let out_bias = read_back_1d(&out_path, "model.layers.0.self_attn.o_proj.bias");

        // Target weights changed.
        assert_ne!(
            out_o_proj, o_proj,
            "o_proj weights must differ from input after abliteration"
        );
        assert_ne!(
            out_down_proj, down_proj,
            "down_proj weights must differ from input after abliteration"
        );

        // Target weights are orthogonal to the refusal direction.
        assert!(
            weight_is_orthogonal_to(&out_o_proj, ROWS, COLS, &direction, 1e-5),
            "o_proj must be orthogonal to refusal direction after abliteration"
        );
        assert!(
            weight_is_orthogonal_to(&out_down_proj, ROWS, COLS, &direction, 1e-5),
            "down_proj must be orthogonal to refusal direction after abliteration"
        );

        // Non-target weight and bias unchanged byte-for-byte.
        assert_eq!(
            out_q_proj, q_proj,
            "q_proj is not a target module; it must be passed through unchanged"
        );
        assert_eq!(
            out_bias, bias,
            "biases must not be transformed even on target Linear modules"
        );

        // Classifier sanity: helps a future reader confirm why only
        // these tensor keys were transformed.
        assert!(is_abliteration_target_module(
            "model.layers.0.self_attn.o_proj.weight"
        ));
        assert!(is_abliteration_target_module(
            "model.layers.3.mlp.down_proj.weight"
        ));
        assert!(!is_abliteration_target_module(
            "model.layers.0.self_attn.q_proj.weight"
        ));
        assert!(!is_abliteration_target_module(
            "model.layers.0.self_attn.o_proj.bias"
        ));

        // ---- provenance sidecar JSON ----
        let sidecar_path = provenance_sidecar_path(&out_path);
        assert!(
            sidecar_path.exists(),
            "provenance sidecar JSON missing at {}",
            sidecar_path.display()
        );
        let sidecar_bytes = std::fs::read(&sidecar_path).unwrap();
        let sidecar: AbliterationProvenance = serde_json::from_slice(&sidecar_bytes).unwrap();
        assert_eq!(sidecar, provenance);

        // ---- ProcessOwnershipLedger drain ----
        // Drop the batcher to close the channel so drain_available_to
        // terminates promptly.
        drop(batcher);
        drain
            .drain_available_to(store.clone())
            .await
            .expect("drain ledger to in-memory store");

        let events = store.events();
        let abliteration_starts: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                LedgerEvent::Start(start)
                    if matches!(start.engine_kind, ProcessEngineKind::AbliterationTool) =>
                {
                    Some(start)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            abliteration_starts.len(),
            1,
            "exactly one engine_kind=AbliterationTool START event expected; got {} total \
             events: {events:?}",
            abliteration_starts.len(),
        );
        let row = abliteration_starts[0];
        assert_eq!(row.mt_id.as_deref(), Some("MT-106"));
        assert_eq!(row.owner_wp.as_deref(), Some("WP-KERNEL-004"));
        assert_eq!(row.owner_role, "ABLITERATE_CLI");
        assert_eq!(
            row.model_artifact_sha256.as_ref(),
            Some(&provenance.base_model_sha256),
            "ProcessOwnershipLedger row must carry the base model SHA-256"
        );
        assert!(
            overflow_concrete.events.lock().unwrap().is_empty(),
            "no overflow events expected for a single AbliterationTool registration"
        );
    }

    /// Direction-width-mismatched fixture: abliteration must refuse
    /// to write a model file because every target tensor would have
    /// the wrong cols, and silently no-op would corrupt the
    /// downstream artifact.
    #[test]
    fn abliterate_rejects_when_direction_width_does_not_match_targets() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let base_path = tempdir.path().join("base.safetensors");
        let direction_path = tempdir.path().join("refusal_direction.json");
        let out_path = tempdir.path().join("abliterated.safetensors");

        // Real fixture cols = COLS, but the refusal direction has
        // COLS + 2 entries on purpose.
        let direction = unit_direction(COLS + 2);

        let o_proj: Vec<f32> = vec![0.0; ROWS * COLS];
        let down_proj: Vec<f32> = vec![0.0; ROWS * COLS];
        let q_proj: Vec<f32> = vec![0.0; ROWS * COLS];
        let bias: Vec<f32> = vec![0.0; ROWS];

        make_fixture_safetensors(&base_path, &o_proj, &down_proj, &q_proj, &bias);
        write_refusal_direction(&direction_path, &direction);

        let config = AbliterationConfig {
            base_model_path: base_path,
            refusal_direction_path: direction_path,
            out_model_path: out_path.clone(),
            license_tag: "Permissive-Test".to_string(),
            provenance_note: "MT-106 direction mismatch".to_string(),
            operator_signature: "operator-test-MT-106".to_string(),
        };

        let err =
            run_abliteration_offline(&config, None).expect_err("direction mismatch must reject");
        assert!(
            matches!(err, AbliterationError::WeightTransform(_)),
            "expected WeightTransform error, got {err:?}",
        );
        assert!(
            !out_path.exists(),
            "no output safetensors must be written when abliteration aborts"
        );
    }
}
