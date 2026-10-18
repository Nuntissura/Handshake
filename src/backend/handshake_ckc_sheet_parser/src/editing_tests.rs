use crate::editing::{
    apply_field_edits_to_raw_text, validate_sheet_field_selectors, SheetFieldEdit,
    SheetFieldSelector,
};
use crate::parse_sheet_template_ast;

fn edit(field_id: &str, replacement_text: &str) -> SheetFieldEdit {
    SheetFieldEdit {
        block_instance_id: None,
        field_id: field_id.to_owned(),
        replacement_text: replacement_text.to_owned(),
    }
}

fn apply(raw: &str, edits: &[SheetFieldEdit]) -> Result<(String, Vec<String>), String> {
    let ast = parse_sheet_template_ast(raw, "ckc-editing-test", None)?;
    apply_field_edits_to_raw_text(raw, &ast, edits)
}

#[test]
fn editing_preserves_mixed_line_endings_unmapped_text_and_descriptor_suffix() {
    let raw = concat!(
        "unmapped preamble\r\n",
        "CHAR-ID-001 — Character_ID:  <string> (keep this suffix)\n",
        "unmapped\tline  \r\n",
        "CHAR-ID-002 — Name: <string>\r",
    );
    let (updated, applied) = apply(raw, &[edit("CHAR-ID-001", "char-edited")]).unwrap();
    assert_eq!(updated, concat!(
        "unmapped preamble\r\n",
        "CHAR-ID-001 — Character_ID:  <char-edited> (keep this suffix)\n",
        "unmapped\tline  \r\n",
        "CHAR-ID-002 — Name: <string>\r",
    ));
    assert_eq!(applied, ["CHAR-ID-001"]);
}

#[test]
fn editing_targets_one_block_instance_and_requires_its_identity() {
    let raw = concat!(
        "WORK & OCCUPATION\n",
        "CHAR-WRK-007 — Side_Hustles: <list of Hustle_Block | optional>\n",
        "\nHustle_Block\n",
        "HUS-BLK-001 — Hustle_Name: <string>\n",
        "\n",
        "CHAR-WRK-007[0].HUS-BLK-001 — Hustle_Name: <first>\n",
        "CHAR-WRK-007[1].HUS-BLK-001 — Hustle_Name: <second>\n",
    );
    let mut selected = edit("HUS-BLK-001", "updated");
    selected.block_instance_id = Some("CHAR-WRK-007[1]".to_owned());
    let (updated, applied) = apply(raw, &[selected]).unwrap();
    assert_eq!(updated, concat!(
        "WORK & OCCUPATION\n",
        "CHAR-WRK-007 — Side_Hustles: <list of Hustle_Block | optional>\n",
        "\nHustle_Block\n",
        "HUS-BLK-001 — Hustle_Name: <string>\n",
        "\n",
        "CHAR-WRK-007[0].HUS-BLK-001 — Hustle_Name: <first>\n",
        "CHAR-WRK-007[1].HUS-BLK-001 — Hustle_Name: <updated>\n",
    ));
    assert_eq!(applied, ["CHAR-WRK-007[1].HUS-BLK-001"]);
    assert_eq!(apply(raw, &[edit("HUS-BLK-001", "unscoped")]).unwrap_err(),
        "block_instance_required: field_id=HUS-BLK-001 belongs to block schema Hustle_Block; edit requires block_instance_id");
    let mut missing = edit("HUS-BLK-001", "missing");
    missing.block_instance_id = Some("CHAR-WRK-007[2]".to_owned());
    assert_eq!(apply(raw, &[missing]).unwrap_err(),
        "sheet field edit references unknown parsed block field_id=CHAR-WRK-007[2].HUS-BLK-001");
}

#[test]
fn editing_canonicalizes_allowed_enums_and_rejects_other_values() {
    let raw = "HUS-BLK-004 — Visibility: <hidden | subtle | public | unknown>\n";
    for (replacement, expected) in [
        (" PUBLIC ", "HUS-BLK-004 — Visibility: <public>\n"),
        ("UNKNOWN", "HUS-BLK-004 — Visibility: <unknown>\n"),
    ] {
        let (updated, applied) = apply(raw, &[edit("HUS-BLK-004", replacement)]).unwrap();
        assert_eq!(updated, expected);
        assert_eq!(applied, ["HUS-BLK-004"]);
    }
    assert_eq!(apply(raw, &[edit("HUS-BLK-004", "unlisted")]).unwrap_err(),
        "invalid_enum: field_id=HUS-BLK-004 replacement_text must match an allowed enum value");
}

#[test]
fn editing_normalizes_score_boundaries_decimals_and_allowed_special_values() {
    let raw = "CHAR-PSY-010A — Openness: <score_10 | unknown>\n";
    for (replacement, expected) in [
        ("0", "CHAR-PSY-010A — Openness: <0/10>\n"),
        ("10/10", "CHAR-PSY-010A — Openness: <10/10>\n"),
        ("7.25", "CHAR-PSY-010A — Openness: <7.25/10>\n"),
        ("UNKNOWN", "CHAR-PSY-010A — Openness: <unknown>\n"),
    ] {
        assert_eq!(apply(raw, &[edit("CHAR-PSY-010A", replacement)]).unwrap().0, expected);
    }
    for replacement in ["-1", "11"] {
        assert_eq!(apply(raw, &[edit("CHAR-PSY-010A", replacement)]).unwrap_err(),
            "invalid_score_10: field_id=CHAR-PSY-010A score must be within 0..10");
    }
}

#[test]
fn editing_preserves_existing_editable_role_tokens() {
    let raw = "CHAR-ID-002 — Name: <string | editable:operator,model>\n";
    let (updated, _) = apply(raw, &[edit("CHAR-ID-002", "Edited")]).unwrap();
    assert_eq!(updated, "CHAR-ID-002 — Name: <Edited|editable:operator,model>\n");
}

#[test]
fn editing_rejects_duplicate_multiline_unknown_and_unchanged_edits() {
    let raw = "CHAR-ID-002 — Name: <original>\n";
    for (edits, expected) in [
        (vec![edit("CHAR-ID-002", "a"), edit("CHAR-ID-002", "b")],
            "duplicate sheet field edit for field_id=CHAR-ID-002"),
        (vec![edit("CHAR-ID-002", "a\nb")],
            "sheet field edit replacement_text for field_id=CHAR-ID-002 must be single-line"),
        (vec![edit("CHAR-ID-003", "new")],
            "sheet field edit references unknown parsed field_id=CHAR-ID-003"),
        (vec![edit("CHAR-ID-002", "original")],
            "sheet field edits produced no text change"),
    ] {
        assert_eq!(apply(raw, &edits).unwrap_err(), expected);
    }
}

#[test]
fn selectors_normalize_keys_and_reject_duplicate_instance_fields() {
    let selector = SheetFieldSelector {
        block_instance_id: Some(" CHAR-WRK-007[1] ".to_owned()),
        field_id: " HUS-BLK-001 ".to_owned(),
    };
    assert_eq!(validate_sheet_field_selectors(&[selector.clone()]).unwrap(),
        ["CHAR-WRK-007[1].HUS-BLK-001"]);
    assert_eq!(validate_sheet_field_selectors(&[selector.clone(), selector]).unwrap_err(),
        "duplicate sheet field selector for field_id=CHAR-WRK-007[1].HUS-BLK-001");
}
