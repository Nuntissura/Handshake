use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::Utc;
use handshake_core::model_runtime::{
    BaseModelTag, LicenseTag, LoraDescriptor, LoraId, LoraStackEntry, LoraStackOps,
    LoraStackSnapshot, LoraStackSnapshotEntry, LoraStrength, ModelRuntimeError,
};

#[derive(Default)]
struct InMemoryLoraStack {
    base_model: BaseModelTag,
    active: Mutex<Vec<LoraStackSnapshotEntry>>,
}

impl InMemoryLoraStack {
    fn with_base_model(base_model: impl Into<String>) -> Self {
        Self {
            base_model: BaseModelTag::new(base_model),
            active: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl LoraStackOps for InMemoryLoraStack {
    async fn mount(
        &self,
        desc: LoraDescriptor,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        if desc.base_model_compat != self.base_model {
            return Err(ModelRuntimeError::LoraStackError(format!(
                "base model mismatch: expected {}, got {}",
                self.base_model.as_str(),
                desc.base_model_compat.as_str()
            )));
        }

        self.active.lock().unwrap().push(LoraStackSnapshotEntry {
            descriptor: desc,
            strength,
            mounted_at_utc: Utc::now(),
        });
        Ok(())
    }

    async fn unmount(&self, id: LoraId) -> Result<(), ModelRuntimeError> {
        self.active
            .lock()
            .unwrap()
            .retain(|entry| entry.descriptor.id != id);
        Ok(())
    }

    fn list_active(&self) -> Vec<LoraStackEntry> {
        self.active
            .lock()
            .unwrap()
            .iter()
            .map(|entry| LoraStackEntry {
                id: entry.descriptor.id,
                strength: entry.strength.clone(),
                mounted_at_utc: entry.mounted_at_utc,
            })
            .collect()
    }

    async fn set_strength(
        &self,
        id: LoraId,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        let mut active = self.active.lock().unwrap();
        let entry = active
            .iter_mut()
            .find(|entry| entry.descriptor.id == id)
            .ok_or_else(|| ModelRuntimeError::LoraStackError(format!("unknown LoRA {id}")))?;
        entry.strength = strength;
        Ok(())
    }

    async fn swap(
        &self,
        new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
        let previous = LoraStackSnapshot {
            entries: self.active.lock().unwrap().clone(),
        };

        if let Some((desc, _)) = new_stack
            .iter()
            .find(|(desc, _)| desc.base_model_compat != self.base_model)
        {
            return Err(ModelRuntimeError::LoraStackError(format!(
                "base model mismatch during swap: {}",
                desc.base_model_compat.as_str()
            )));
        }

        *self.active.lock().unwrap() = new_stack
            .into_iter()
            .map(|(desc, strength)| LoraStackSnapshotEntry {
                descriptor: desc,
                strength,
                mounted_at_utc: Utc::now(),
            })
            .collect();
        Ok(previous)
    }
}

#[test]
fn model_runtime_lora_tests_ops_are_object_safe_and_strength_is_validated() {
    fn assert_object_safe(_: Box<dyn LoraStackOps>) {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Box<dyn LoraStackOps>>();
    assert_object_safe(Box::new(InMemoryLoraStack::with_base_model("llama-3.1-8b")));

    assert_eq!(LoraStrength::try_new(0.0).unwrap().value(), 0.0);
    assert_eq!(LoraStrength::try_new(2.0).unwrap().value(), 2.0);
    assert!(LoraStrength::try_new(-0.01).is_err());
    assert!(LoraStrength::try_new(2.01).is_err());
    assert!(LoraStrength::try_new(f32::NAN).is_err());
    assert!(LoraStrength::try_new(f32::INFINITY).is_err());
    assert!(LoraStrength::try_new(f32::NEG_INFINITY).is_err());
    assert!(BaseModelTag::try_new("").is_err());
    assert!(LicenseTag::try_new("   ").is_err());
}

#[test]
fn model_runtime_lora_tests_ids_are_v7_and_descriptor_is_engine_agnostic() {
    let descriptor = descriptor("story", "llama-3.1-8b");

    assert_eq!(descriptor.id.as_uuid().get_version_num(), 7);
    assert_eq!(descriptor.base_model_compat.as_str(), "llama-3.1-8b");
    assert_eq!(descriptor.license_tag.as_str(), "operator-local");
    assert_eq!(descriptor.target_modules, vec!["q_proj".to_string()]);

    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/model_runtime/lora.rs"),
    )
    .expect("read lora.rs");
    let normalized = source.to_ascii_lowercase();
    for banned in ["llama_cpp_2::", "candle_core::", "candle_transformers::"] {
        assert!(
            !normalized.contains(banned),
            "lora surface must not leak engine-specific type `{banned}`"
        );
    }
}

#[test]
fn model_runtime_lora_tests_mount_set_unmount_and_atomic_swap_rollback() {
    let stack = InMemoryLoraStack::with_base_model("llama-3.1-8b");
    let first = descriptor("story", "llama-3.1-8b");
    let first_id = first.id;

    futures::executor::block_on(stack.mount(first, LoraStrength::try_new(0.75).unwrap())).unwrap();
    assert_eq!(stack.list_active().len(), 1);
    assert_eq!(stack.list_active()[0].id, first_id);

    futures::executor::block_on(stack.set_strength(first_id, LoraStrength::try_new(1.25).unwrap()))
        .unwrap();
    assert_eq!(stack.list_active()[0].strength.value(), 1.25);

    let second = descriptor("domain", "llama-3.1-8b");
    let wrong_base = descriptor("wrong-base", "mistral-7b");
    let failed = futures::executor::block_on(stack.swap(vec![
        (second.clone(), LoraStrength::try_new(1.0).unwrap()),
        (wrong_base, LoraStrength::try_new(1.0).unwrap()),
    ]));
    assert!(matches!(failed, Err(ModelRuntimeError::LoraStackError(_))));
    assert_eq!(
        stack.list_active(),
        vec![LoraStackEntry {
            id: first_id,
            strength: LoraStrength::try_new(1.25).unwrap(),
            mounted_at_utc: stack.list_active()[0].mounted_at_utc,
        }],
        "failed swap must leave the previous active stack intact"
    );

    let previous = futures::executor::block_on(
        stack.swap(vec![(second.clone(), LoraStrength::try_new(0.5).unwrap())]),
    )
    .unwrap();
    assert_eq!(previous.entries.len(), 1);
    assert_eq!(previous.entries[0].descriptor.id, first_id);
    assert_eq!(stack.list_active().len(), 1);
    assert_eq!(stack.list_active()[0].id, second.id);

    futures::executor::block_on(stack.unmount(second.id)).unwrap();
    assert!(stack.list_active().is_empty());
}

#[test]
fn model_runtime_lora_tests_base_model_mismatch_is_rejected_before_mount() {
    let stack = InMemoryLoraStack::with_base_model("llama-3.1-8b");
    let error = futures::executor::block_on(stack.mount(
        descriptor("bad", "mistral-7b"),
        LoraStrength::try_new(1.0).unwrap(),
    ))
    .unwrap_err();

    assert!(matches!(error, ModelRuntimeError::LoraStackError(_)));
    assert!(stack.list_active().is_empty());
}

#[test]
fn model_runtime_lora_tests_handle_forwards_to_ops() {
    let stack = Arc::new(InMemoryLoraStack::with_base_model("llama-3.1-8b"));
    let handle = handshake_core::model_runtime::LoraStackHandle::with_ops("runtime-stack", stack);
    let desc = descriptor("story", "llama-3.1-8b");
    let id = desc.id;

    futures::executor::block_on(handle.mount(desc, LoraStrength::try_new(1.0).unwrap())).unwrap();
    assert_eq!(handle.list_active().len(), 1);
    futures::executor::block_on(handle.set_strength(id, LoraStrength::try_new(0.25).unwrap()))
        .unwrap();
    assert_eq!(handle.list_active()[0].strength.value(), 0.25);
    let previous = futures::executor::block_on(handle.swap(Vec::new())).unwrap();
    assert_eq!(previous.entries[0].descriptor.id, id);
    assert!(handle.list_active().is_empty());
}

fn descriptor(name: &str, base_model: &str) -> LoraDescriptor {
    LoraDescriptor {
        id: LoraId::new_v7(),
        artifact_path: Path::new("loras").join(format!("{name}.safetensors")),
        sha256: [7; 32],
        rank: 16,
        target_modules: vec!["q_proj".to_string()],
        base_model_compat: BaseModelTag::new(base_model),
        license_tag: LicenseTag::new("operator-local"),
    }
}

mod model_runtime {
    pub mod lora {
        use handshake_core::model_runtime::{
            BaseModelTag, LicenseTag, LoraId, LoraStackHandle, LoraStrength,
        };

        #[test]
        fn filter_visible_contract_smoke() {
            assert_eq!(LoraId::new_v7().as_uuid().get_version_num(), 7);
            assert!(LoraStrength::try_new(f32::INFINITY).is_err());
            assert!(BaseModelTag::try_new("").is_err());
            assert!(LicenseTag::try_new("").is_err());
            assert!(LoraStackHandle::new("unbound").list_active().is_empty());
        }
    }
}
