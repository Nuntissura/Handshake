use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

pub const HBR_VIS_GAP_RECEIPT_KIND: &str = "HBR_VIS_GAP";
pub const HBR_VIS_GAP_SCHEMA_VERSION: u32 = 1;
pub const HBR_VIS_GAP_HBR_ID: &str = "HBR-VIS-005";
pub const HBR_VIS_GAP_REQUIRED_ACTION: &str =
    "Open a follow-up WP for the missing automation hook before PASS closure.";

pub type Packet = Value;

#[derive(Debug, Error)]
pub enum VisGapError {
    #[error("HBR_VIS_GAP_VALIDATION: {0}")]
    Validation(String),
    #[error("HBR_VIS_GAP_SERIALIZE: {source}")]
    Serialize { source: serde_json::Error },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapClass {
    NoCdpHandle,
    NativeChildWindow,
    OpaqueCanvas,
    ShadowRootInaccessible,
    Other,
}

impl GapClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCdpHandle => "no_cdp_handle",
            Self::NativeChildWindow => "native_child_window",
            Self::OpaqueCanvas => "opaque_canvas",
            Self::ShadowRootInaccessible => "shadow_root_inaccessible",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisGap {
    pub receipt_kind: String,
    pub schema_version: u32,
    pub receipt_uuid: Uuid,
    pub wp_id: String,
    pub surface_name: String,
    pub surface_path: String,
    pub gap_class: GapClass,
    pub proposed_followup_wp: Option<String>,
    pub evidence_pointer: Option<String>,
    pub emitted_at_utc: DateTime<Utc>,
}

impl VisGap {
    pub fn new(
        wp_id: &str,
        surface_name: &str,
        surface_path: &str,
        gap_class: GapClass,
        proposed_followup_wp: Option<&str>,
        evidence_pointer: Option<&str>,
    ) -> Self {
        Self {
            receipt_kind: HBR_VIS_GAP_RECEIPT_KIND.to_string(),
            schema_version: HBR_VIS_GAP_SCHEMA_VERSION,
            receipt_uuid: Uuid::now_v7(),
            wp_id: wp_id.to_string(),
            surface_name: surface_name.to_string(),
            surface_path: surface_path.to_string(),
            gap_class,
            proposed_followup_wp: proposed_followup_wp.map(str::to_string),
            evidence_pointer: evidence_pointer.map(str::to_string),
            emitted_at_utc: Utc::now(),
        }
    }

    pub fn emit(packet: &mut Packet, gap: VisGap) -> Result<VisGap, VisGapError> {
        gap.validate()?;
        append_open_blocker(packet, &gap)?;
        Ok(gap)
    }

    pub fn blocker_id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}|{}|{}",
            self.wp_id,
            self.surface_path,
            self.gap_class.as_str()
        ));
        let digest = hex::encode(hasher.finalize());
        format!("hbr-vis-gap-{}", &digest[..12])
    }

    pub fn to_canonical_jsonl(&self) -> Result<String, VisGapError> {
        self.validate()?;
        serde_json::to_string(&self.to_canonical_value())
            .map(|json| format!("{json}\n"))
            .map_err(|source| VisGapError::Serialize { source })
    }

    pub fn to_canonical_value(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "emitted_at_utc".to_string(),
            Value::String(
                self.emitted_at_utc
                    .to_rfc3339_opts(SecondsFormat::Secs, true),
            ),
        );
        map.insert(
            "evidence_pointer".to_string(),
            optional_string_value(self.evidence_pointer.as_deref()),
        );
        map.insert(
            "gap_class".to_string(),
            Value::String(self.gap_class.as_str().to_string()),
        );
        map.insert(
            "hbr_id".to_string(),
            Value::String(HBR_VIS_GAP_HBR_ID.to_string()),
        );
        map.insert(
            "proposed_followup_wp".to_string(),
            optional_string_value(self.proposed_followup_wp.as_deref()),
        );
        map.insert(
            "receipt_kind".to_string(),
            Value::String(HBR_VIS_GAP_RECEIPT_KIND.to_string()),
        );
        map.insert(
            "receipt_uuid".to_string(),
            Value::String(self.receipt_uuid.to_string()),
        );
        map.insert(
            "schema_version".to_string(),
            Value::Number(HBR_VIS_GAP_SCHEMA_VERSION.into()),
        );
        map.insert(
            "surface_name".to_string(),
            Value::String(self.surface_name.clone()),
        );
        map.insert(
            "surface_path".to_string(),
            Value::String(self.surface_path.clone()),
        );
        map.insert("wp_id".to_string(), Value::String(self.wp_id.clone()));
        Value::Object(map)
    }

    fn validate(&self) -> Result<(), VisGapError> {
        if self.receipt_kind != HBR_VIS_GAP_RECEIPT_KIND {
            return Err(VisGapError::Validation(format!(
                "receipt_kind must be {HBR_VIS_GAP_RECEIPT_KIND}"
            )));
        }
        if self.schema_version != HBR_VIS_GAP_SCHEMA_VERSION {
            return Err(VisGapError::Validation(format!(
                "schema_version must be {HBR_VIS_GAP_SCHEMA_VERSION}"
            )));
        }
        if self.receipt_uuid.get_version_num() != 7 {
            return Err(VisGapError::Validation(
                "receipt_uuid must be UUID v7".to_string(),
            ));
        }
        if self.wp_id.trim().is_empty() {
            return Err(VisGapError::Validation(
                "wp_id must be non-empty".to_string(),
            ));
        }
        if self.surface_name.trim().is_empty() {
            return Err(VisGapError::Validation(
                "surface_name must be non-empty".to_string(),
            ));
        }
        if self.surface_path.trim().is_empty() {
            return Err(VisGapError::Validation(
                "surface_path must be non-empty".to_string(),
            ));
        }
        if self
            .proposed_followup_wp
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || !value.starts_with("WP-"))
        {
            return Err(VisGapError::Validation(
                "proposed_followup_wp must be null or start with WP-".to_string(),
            ));
        }
        if self
            .evidence_pointer
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(VisGapError::Validation(
                "evidence_pointer must be null or non-empty".to_string(),
            ));
        }
        Ok(())
    }
}

fn append_open_blocker(packet: &mut Packet, gap: &VisGap) -> Result<(), VisGapError> {
    let Some(packet_object) = packet.as_object_mut() else {
        return Err(VisGapError::Validation(
            "packet JSON must be an object".to_string(),
        ));
    };

    if !packet_object.contains_key("open_blockers") {
        packet_object.insert("open_blockers".to_string(), Value::Array(Vec::new()));
    }

    let Some(open_blockers) = packet_object
        .get_mut("open_blockers")
        .and_then(Value::as_array_mut)
    else {
        return Err(VisGapError::Validation(
            "packet.open_blockers must be an array when present".to_string(),
        ));
    };

    let blocker = open_blocker_for_gap(gap);
    let blocker_id = blocker
        .get("blocker_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if let Some(existing) = open_blockers.iter_mut().find(|entry| {
        entry
            .get("blocker_id")
            .and_then(Value::as_str)
            .is_some_and(|value| value == blocker_id)
    }) {
        *existing = blocker;
    } else {
        open_blockers.push(blocker);
    }
    Ok(())
}

fn open_blocker_for_gap(gap: &VisGap) -> Value {
    let mut map = Map::new();
    map.insert("blocker_id".to_string(), Value::String(gap.blocker_id()));
    map.insert(
        "blocker_kind".to_string(),
        Value::String(HBR_VIS_GAP_RECEIPT_KIND.to_string()),
    );
    map.insert("status".to_string(), Value::String("OPEN".to_string()));
    map.insert(
        "hbr_id".to_string(),
        Value::String(HBR_VIS_GAP_HBR_ID.to_string()),
    );
    map.insert("wp_id".to_string(), Value::String(gap.wp_id.clone()));
    map.insert(
        "surface_name".to_string(),
        Value::String(gap.surface_name.clone()),
    );
    map.insert(
        "surface_path".to_string(),
        Value::String(gap.surface_path.clone()),
    );
    map.insert(
        "gap_class".to_string(),
        Value::String(gap.gap_class.as_str().to_string()),
    );
    map.insert(
        "receipt_uuid".to_string(),
        Value::String(gap.receipt_uuid.to_string()),
    );
    map.insert(
        "receipt_ref".to_string(),
        Value::String(format!("receipt://{}", gap.receipt_uuid)),
    );
    map.insert(
        "evidence_pointer".to_string(),
        optional_string_value(gap.evidence_pointer.as_deref()),
    );
    map.insert(
        "proposed_followup_wp".to_string(),
        optional_string_value(gap.proposed_followup_wp.as_deref()),
    );
    map.insert(
        "created_at_utc".to_string(),
        Value::String(
            gap.emitted_at_utc
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        ),
    );
    map.insert(
        "required_action".to_string(),
        Value::String(HBR_VIS_GAP_REQUIRED_ACTION.to_string()),
    );
    Value::Object(map)
}

fn optional_string_value(value: Option<&str>) -> Value {
    value
        .map(|entry| Value::String(entry.to_string()))
        .unwrap_or(Value::Null)
}
