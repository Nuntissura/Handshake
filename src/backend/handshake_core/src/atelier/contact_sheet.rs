//! Native Atelier contact-sheet generation.
//!
//! The generator intentionally produces deterministic SVG plus a JSON receipt.
//! Source image bytes stay behind their existing refs; this module builds a
//! visual review artifact and lineage record that agents can inspect without
//! pulling a large dataset onto the UI thread.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONTACT_SHEET_EXPORT_SCHEMA_ID: &str = "hsk.atelier.contact_sheet_export@1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactSheetExportItem {
    pub item_id: String,
    pub label: String,
    pub source_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_ref: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactSheetLayout {
    pub rows: usize,
    pub columns: usize,
    pub dpi: usize,
    pub cell_width_px: usize,
    pub cell_height_px: usize,
    pub gap_px: usize,
    pub margin_px: usize,
    pub label_height_px: usize,
    pub width_px: usize,
    pub height_px: usize,
    pub cell_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactSheetExport {
    pub schema_id: String,
    pub source_kind: String,
    pub source_ref: String,
    pub thumbnail_fit: String,
    pub output_path: Option<String>,
    pub layout: ContactSheetLayout,
    pub item_count: usize,
    pub rendered_item_count: usize,
    pub omitted_item_count: usize,
    pub include_labels: bool,
    pub svg: String,
    pub svg_sha256: String,
    pub receipt_json: serde_json::Value,
    pub receipt_sha256: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerateContactSheetRequest {
    pub source_kind: String,
    pub source_ref: String,
    pub rows: usize,
    pub columns: usize,
    pub dpi: usize,
    pub include_labels: bool,
    pub thumbnail_fit: String,
    pub output_path: Option<String>,
    pub items: Vec<ContactSheetExportItem>,
}

pub fn generate_contact_sheet_export(
    request: GenerateContactSheetRequest,
) -> Result<ContactSheetExport, String> {
    let source_kind = require_token("source_kind", &request.source_kind)?;
    let source_ref = require_ref("source_ref", &request.source_ref)?;
    let thumbnail_fit = require_thumbnail_fit(&request.thumbnail_fit)?;
    let output_path = request
        .output_path
        .as_deref()
        .map(|value| require_ref("output_path", value))
        .transpose()?;
    if request.items.is_empty() {
        return Err("contact sheet requires at least one source item".to_owned());
    }
    if request.items.len() > 10_000 {
        return Err("contact sheet item_count must be <= 10000".to_owned());
    }
    for item in &request.items {
        require_ref("item_id", &item.item_id)?;
        require_ref("item.source_ref", &item.source_ref)?;
        if let Some(media_ref) = item.media_ref.as_deref() {
            require_ref("item.media_ref", media_ref)?;
        }
    }

    let rows = require_range("rows", request.rows, 1, 24)?;
    let columns = require_range("columns", request.columns, 1, 24)?;
    let dpi = require_range("dpi", request.dpi, 72, 1200)?;
    let cell_count = rows.saturating_mul(columns);
    let rendered_item_count = request.items.len().min(cell_count);
    let omitted_item_count = request.items.len().saturating_sub(rendered_item_count);
    let layout = contact_sheet_layout(rows, columns, dpi, request.include_labels);
    let rendered_items = &request.items[..rendered_item_count];
    let svg = render_contact_sheet_svg(
        &layout,
        rendered_items,
        request.include_labels,
        &thumbnail_fit,
    );
    let svg_sha256 = sha256_hex(svg.as_bytes());
    let receipt_json = serde_json::json!({
        "schema_id": "hsk.atelier.contact_sheet_export_receipt@1",
        "export_schema_id": CONTACT_SHEET_EXPORT_SCHEMA_ID,
        "source_kind": source_kind,
        "source_ref": source_ref,
        "layout": layout,
        "thumbnail_fit": thumbnail_fit,
        "output_path": output_path,
        "item_count": request.items.len(),
        "rendered_item_count": rendered_item_count,
        "omitted_item_count": omitted_item_count,
        "include_labels": request.include_labels,
        "source_items": request.items,
        "svg_sha256": svg_sha256,
    });
    let receipt_bytes = serde_json::to_vec(&receipt_json)
        .map_err(|err| format!("serialize contact sheet receipt failed: {err}"))?;
    let receipt_sha256 = sha256_hex(&receipt_bytes);
    let content_hash = sha256_hex(
        format!(
            "{}|{}|{}|{}|{}",
            CONTACT_SHEET_EXPORT_SCHEMA_ID,
            source_ref,
            svg_sha256,
            receipt_sha256,
            rendered_item_count
        )
        .as_bytes(),
    );
    Ok(ContactSheetExport {
        schema_id: CONTACT_SHEET_EXPORT_SCHEMA_ID.to_owned(),
        source_kind,
        source_ref,
        thumbnail_fit,
        output_path,
        layout,
        item_count: request.items.len(),
        rendered_item_count,
        omitted_item_count,
        include_labels: request.include_labels,
        svg,
        svg_sha256,
        receipt_json,
        receipt_sha256,
        content_hash,
    })
}

fn contact_sheet_layout(
    rows: usize,
    columns: usize,
    dpi: usize,
    include_labels: bool,
) -> ContactSheetLayout {
    let cell_width_px = dpi;
    let label_height_px = if include_labels { (dpi / 5).max(18) } else { 0 };
    let cell_height_px = dpi + label_height_px;
    let gap_px = (dpi / 18).max(8);
    let margin_px = (dpi / 12).max(12);
    let width_px = margin_px
        .saturating_mul(2)
        .saturating_add(columns.saturating_mul(cell_width_px))
        .saturating_add(columns.saturating_sub(1).saturating_mul(gap_px));
    let height_px = margin_px
        .saturating_mul(2)
        .saturating_add(rows.saturating_mul(cell_height_px))
        .saturating_add(rows.saturating_sub(1).saturating_mul(gap_px));
    ContactSheetLayout {
        rows,
        columns,
        dpi,
        cell_width_px,
        cell_height_px,
        gap_px,
        margin_px,
        label_height_px,
        width_px,
        height_px,
        cell_count: rows.saturating_mul(columns),
    }
}

fn render_contact_sheet_svg(
    layout: &ContactSheetLayout,
    items: &[ContactSheetExportItem],
    include_labels: bool,
    thumbnail_fit: &str,
) -> String {
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img">"#,
        layout.width_px, layout.height_px, layout.width_px, layout.height_px
    ));
    svg.push_str(r##"<rect width="100%" height="100%" fill="#101316"/>"##);
    for (index, item) in items.iter().enumerate() {
        let row = index / layout.columns;
        let column = index % layout.columns;
        let x = layout.margin_px + column * (layout.cell_width_px + layout.gap_px);
        let y = layout.margin_px + row * (layout.cell_height_px + layout.gap_px);
        let image_height = layout.cell_height_px - layout.label_height_px;
        let hue = stable_hue(&item.item_id);
        let escaped_item_id = xml_escape(&item.item_id);
        let escaped_source_ref = xml_escape(&item.source_ref);
        svg.push_str(&format!(
            r##"<g data-item-id="{}" data-source-ref="{}"><title>item_id={} source_ref={}</title><rect x="{}" y="{}" width="{}" height="{}" rx="6" fill="hsl({}, 34%, 22%)" stroke="#66707a" stroke-width="2"/>"##,
            escaped_item_id,
            escaped_source_ref,
            escaped_item_id,
            escaped_source_ref,
            x,
            y,
            layout.cell_width_px,
            image_height,
            hue
        ));
        svg.push_str(&format!(
            r##"<text x="{}" y="{}" fill="#d8dee4" font-size="{}" font-family="Inter, Segoe UI, sans-serif" text-anchor="middle">{} </text>"##,
            x + layout.cell_width_px / 2,
            y + image_height / 2,
            (layout.dpi / 11).clamp(14, 54),
            xml_escape(&short_label(&item.label, &item.item_id))
        ));
        svg.push_str(&format!(
            r##"<text x="{}" y="{}" fill="#8f9aa7" font-size="{}" font-family="Inter, Segoe UI, sans-serif" text-anchor="middle">{}</text>"##,
            x + layout.cell_width_px / 2,
            y + image_height / 2 + (layout.dpi / 10).max(16),
            (layout.dpi / 26).clamp(10, 24),
            xml_escape(&short_label(&item.source_ref, &item.item_id))
        ));
        if source_ref_is_svg_usable(&item.source_ref) {
            let preserve_aspect_ratio = match thumbnail_fit {
                "cover" => "xMidYMid slice",
                "stretch" => "none",
                _ => "xMidYMid meet",
            };
            svg.push_str(&format!(
                r##"<image href="{}" x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="{}" opacity="0.72"/>"##,
                escaped_source_ref,
                x + 4,
                y + 4,
                layout.cell_width_px.saturating_sub(8),
                image_height.saturating_sub(8),
                preserve_aspect_ratio
            ));
        }
        if include_labels {
            svg.push_str(&format!(
                r##"<text x="{}" y="{}" fill="#aeb7c2" font-size="{}" font-family="Inter, Segoe UI, sans-serif">{}</text>"##,
                x,
                y + image_height + (layout.label_height_px / 2).max(14),
                (layout.dpi / 18).clamp(12, 36),
                xml_escape(&item.label)
            ));
        }
        svg.push_str("</g>");
    }
    svg.push_str("</svg>");
    svg
}

fn require_token(field: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value || trimmed.chars().any(char::is_whitespace) {
        return Err(format!("{field} must be non-empty and whitespace-free"));
    }
    Ok(trimmed.to_owned())
}

fn require_ref(field: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value || trimmed.chars().any(char::is_control) {
        return Err(format!("{field} must be a non-empty source ref"));
    }
    Ok(trimmed.to_owned())
}

fn require_range(field: &str, value: usize, min: usize, max: usize) -> Result<usize, String> {
    if !(min..=max).contains(&value) {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(value)
}

fn require_thumbnail_fit(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" => Ok("contain".to_owned()),
        "contain" | "cover" | "stretch" => Ok(value.trim().to_ascii_lowercase()),
        other => Err(format!(
            "thumbnail_fit must be contain/cover/stretch, got {other:?}"
        )),
    }
}

fn source_ref_is_svg_usable(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("file:")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("artifact://")
        || lower.starts_with("asset://")
        || lower.starts_with("data:image/")
}

fn short_label(label: &str, fallback: &str) -> String {
    let source = if label.trim().is_empty() {
        fallback
    } else {
        label.trim()
    };
    source.chars().take(18).collect()
}

fn stable_hue(value: &str) -> usize {
    value
        .bytes()
        .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize))
        % 360
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> ContactSheetExportItem {
        ContactSheetExportItem {
            item_id: id.to_owned(),
            label: format!("{id}.png"),
            source_ref: format!("dataset://source/{id}.png"),
            media_ref: None,
        }
    }

    #[test]
    fn contact_sheet_layout_changes_with_rows_columns_and_dpi() {
        let a = generate_contact_sheet_export(GenerateContactSheetRequest {
            source_kind: "ingest_batch".to_owned(),
            source_ref: "batch-1".to_owned(),
            rows: 2,
            columns: 3,
            dpi: 150,
            include_labels: true,
            thumbnail_fit: "contain".to_owned(),
            output_path: None,
            items: vec![item("i1"), item("i2")],
        })
        .expect("contact sheet");
        let b = generate_contact_sheet_export(GenerateContactSheetRequest {
            source_kind: "ingest_batch".to_owned(),
            source_ref: "batch-1".to_owned(),
            rows: 4,
            columns: 5,
            dpi: 300,
            include_labels: true,
            thumbnail_fit: "cover".to_owned(),
            output_path: Some("artifact://atelier/contact-sheets/b.svg".to_owned()),
            items: vec![item("i1"), item("i2")],
        })
        .expect("contact sheet");

        assert_eq!(a.layout.rows, 2);
        assert_eq!(a.layout.columns, 3);
        assert_eq!(a.layout.dpi, 150);
        assert_ne!(a.layout.width_px, b.layout.width_px);
        assert_ne!(a.layout.height_px, b.layout.height_px);
        assert_ne!(a.svg_sha256, b.svg_sha256);
    }

    #[test]
    fn contact_sheet_receipt_preserves_lineage_and_omitted_items() {
        let export = generate_contact_sheet_export(GenerateContactSheetRequest {
            source_kind: "ingest_batch".to_owned(),
            source_ref: "batch-2".to_owned(),
            rows: 1,
            columns: 2,
            dpi: 120,
            include_labels: true,
            thumbnail_fit: "contain".to_owned(),
            output_path: None,
            items: vec![item("i1"), item("i2"), item("i3")],
        })
        .expect("contact sheet");

        assert_eq!(export.item_count, 3);
        assert_eq!(export.rendered_item_count, 2);
        assert_eq!(export.omitted_item_count, 1);
        assert_eq!(export.receipt_json["source_ref"], "batch-2");
        assert_eq!(export.receipt_json["source_items"][2]["item_id"], "i3");
        assert!(export.svg.contains("data-item-id=\"i1\""));
        assert!(export
            .svg
            .contains("data-source-ref=\"dataset://source/i1.png\""));
    }

    #[test]
    fn contact_sheet_rejects_out_of_range_layout_inputs() {
        let err = generate_contact_sheet_export(GenerateContactSheetRequest {
            source_kind: "ingest_batch".to_owned(),
            source_ref: "batch-3".to_owned(),
            rows: 0,
            columns: 4,
            dpi: 300,
            include_labels: true,
            thumbnail_fit: "contain".to_owned(),
            output_path: None,
            items: vec![item("i1")],
        })
        .expect_err("rows below range must fail");
        assert!(err.contains("rows must be between 1 and 24"));

        let err = generate_contact_sheet_export(GenerateContactSheetRequest {
            source_kind: "ingest_batch".to_owned(),
            source_ref: "batch-3".to_owned(),
            rows: 3,
            columns: 4,
            dpi: 2400,
            include_labels: true,
            thumbnail_fit: "contain".to_owned(),
            output_path: None,
            items: vec![item("i1")],
        })
        .expect_err("dpi above range must fail");
        assert!(err.contains("dpi must be between 72 and 1200"));
    }

    #[test]
    fn contact_sheet_accepts_local_source_refs_as_lineage() {
        let mut local = item("i1");
        local.source_ref = r"D:\datasets\frame-a.png".to_owned();
        let export = generate_contact_sheet_export(GenerateContactSheetRequest {
            source_kind: "manual".to_owned(),
            source_ref: "manual-source".to_owned(),
            rows: 1,
            columns: 1,
            dpi: 120,
            include_labels: true,
            thumbnail_fit: "stretch".to_owned(),
            output_path: Some(r"D:\exports\contact-sheet.svg".to_owned()),
            items: vec![local],
        })
        .expect("local source path is allowed as lineage data");
        assert_eq!(
            export.receipt_json["source_items"][0]["source_ref"],
            r"D:\datasets\frame-a.png"
        );
    }
}
