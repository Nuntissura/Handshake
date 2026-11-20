use super::*;
use serde_json::json;

fn parse(content: serde_json::Value) -> BlockNode {
    crate::rich_editor::document_model::doc_json::from_json_value(&json!({"type":"doc","content":content})).unwrap()
}

#[test]
fn persisted_identity_survives_reordering_and_keeps_nested_paths() {
    let paragraph = json!({"type":"paragraph","attrs":{"block_id":"persisted-opaque"},"content":[{"type":"text","text":"Same"}]});
    let mut document = parse(json!([
        {"type":"blockquote","content":[paragraph.clone()]},
        {"type":"paragraph","content":[{"type":"text","text":"Same"}]},
        {"type":"heading","attrs":{"level":2,"block_id":"heading-id"},"content":[{"type":"text","text":"Title"}]}
    ]));
    let rows = index(&document).unwrap();
    assert_eq!(rows.len(), 2, "missing attributes must not acquire generated identities");
    assert_eq!(rows[0].path, vec![0, 0]);
    let target = rows[0].target.clone();
    document.children.swap(0, 2);
    assert_eq!(target.locate(&document).unwrap(), vec![2, 0]);
    let mut wrong_kind = target.clone();
    wrong_kind.kind = ReferenceTargetKind::Heading;
    assert!(wrong_kind.locate(&document).is_err());
    let link = target.link("owner-note", "Exact Alias");
    assert_eq!(Target::from_link(&link).unwrap(), target);
    assert_eq!(link.label, "Exact Alias");
}

#[test]
fn duplicate_or_malformed_persisted_ids_fail_closed_and_absent_ids_are_not_guessed() {
    let duplicate = parse(json!([
        {"type":"paragraph","attrs":{"block_id":"same"}},
        {"type":"paragraph","attrs":{"block_id":"same"}}
    ]));
    assert!(index(&duplicate).is_err());
    let absent = parse(json!([{"type":"paragraph"},{"type":"heading","attrs":{"level":1}}]));
    assert!(index(&absent).unwrap().is_empty());
    let target = Target { block_id:"KBL-guessed".into(), kind:ReferenceTargetKind::Block };
    assert!(target.locate(&absent).is_err());
    let mut malformed = parse(json!([{"type":"paragraph"}]));
    malformed.children[0].as_block_mut().unwrap().attrs.insert("block_id".into(), json!(4));
    assert!(index(&malformed).is_err());
}

#[test]
fn partial_and_unresolved_structural_links_never_fall_back_to_note_title_creation() {
    let target = Target { block_id:"persisted".into(), kind:ReferenceTargetKind::Block };
    let mut link = target.link("note", "alias");
    link.resolved = false;
    assert!(structural_link(&link));
    assert!(Target::from_link(&link).is_err());
    link.resolved = true;
    link.target_kind = None;
    assert!(structural_link(&link));
    assert!(Target::from_link(&link).is_err());
    link.target_block_id = None;
    assert!(!structural_link(&link));
}
