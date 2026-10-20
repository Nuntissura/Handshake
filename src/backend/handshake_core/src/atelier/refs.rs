//! Typed `atelier://` ref builders and parsers shared by the CKC surfaces (ported verbatim from
//! `feat/WP-CKC-posekit-overhaul`; storage-agnostic).

pub use handshake_ckc_provenance::{
    character_ref, collection_ref, media_asset_ref, parse_character_ref, parse_collection_ref,
    parse_media_asset_ref, parse_sheet_artifact_ref, parse_sheet_version_ref, parse_tag_ref,
    sheet_artifact_ref, sheet_version_ref, tag_ref, validate_character_ref, validate_collection_ref,
    validate_media_asset_ref, validate_sheet_artifact_ref, validate_sheet_version_ref, validate_tag_ref,
};
