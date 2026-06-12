//! MT-205 UserManualVisualDebugProof: deterministic page projections with
//! STABLE selectors so manual pages are provably readable, navigable, and
//! linkable from product surfaces.
//!
//! Two formats (projections only — PostgreSQL rows stay authority):
//! * HTML — semantic structure with `data-hs-manual-*` attributes on every
//!   addressable element (visual-debug law: stable element identifiers).
//!   Section bodies are markdown ESCAPED into the HTML (this crate bundles no
//!   markdown renderer; the desktop app renders markdown with its own
//!   bundled renderer — the projection guarantees structure + selectors +
//!   lossless text, which is what DOM-level assertions need).
//! * Markdown — `<topic>`-tagged projection matching the repo prose format
//!   (topic id per section, blank-line padding).
//!
//! GUI declaration (MT-205 contract: "use visual debugging or declare why no
//! GUI surface exists"): the operator-facing manual panel is owned by the
//! CONCURRENT frontend lane (app/** — DEC-001/DEC-002 unified work surface).
//! The backend proof here is DOM-equivalent: the HTML projection is asserted
//! at runtime for stable selectors, escaped (non-overlapping) text, ordered
//! sections, and resolvable navigation links; the frontend panel consumes
//! THIS projection, so its selectors are pinned before the panel lands.

use super::store::{UserManualAnchor, UserManualPage, UserManualSection};

/// Render a manual page as semantic HTML with stable selectors.
pub fn render_page_html(
    page: &UserManualPage,
    sections: &[UserManualSection],
    anchors: &[UserManualAnchor],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<article data-hs-manual-page=\"{}\" data-hs-manual-version=\"{}\" data-hs-manual-kind=\"{}\">\n",
        attr_escape(&page.slug),
        attr_escape(&page.manual_version),
        attr_escape(&page.page_kind),
    ));
    out.push_str(&format!(
        "<h1 data-hs-manual-title=\"{}\">{}</h1>\n",
        attr_escape(&page.slug),
        text_escape(&page.title)
    ));
    for section in sections {
        out.push_str(&format!(
            "<section data-hs-manual-section=\"{}\" data-hs-section-kind=\"{}\" data-hs-section-position=\"{}\">\n",
            attr_escape(&section.section_id),
            attr_escape(&section.section_kind),
            section.position,
        ));
        out.push_str(&format!("<h2>{}</h2>\n", text_escape(&section.title)));
        out.push_str(&format!(
            "<div class=\"hs-manual-md\" data-hs-manual-md=\"true\">{}</div>\n",
            text_escape(&section.body_md)
        ));
        out.push_str("</section>\n");
    }
    if !anchors.is_empty() {
        out.push_str("<nav data-hs-manual-links=\"true\">\n<ul>\n");
        for anchor in anchors {
            let href = match anchor.anchor_kind.as_str() {
                "page_link" => format!("/usermanual/pages/{}", anchor.anchor_value),
                "http_route" => anchor.anchor_value.clone(),
                _ => String::new(),
            };
            out.push_str(&format!(
                "<li data-hs-manual-link=\"{}\" data-hs-anchor-kind=\"{}\"{}>{}{}</li>\n",
                attr_escape(&anchor.anchor_value),
                attr_escape(&anchor.anchor_kind),
                if href.is_empty() {
                    String::new()
                } else {
                    format!(" data-hs-href=\"{}\"", attr_escape(&href))
                },
                if anchor.http_method.is_empty() {
                    String::new()
                } else {
                    format!("{} ", text_escape(&anchor.http_method))
                },
                text_escape(&anchor.anchor_value),
            ));
        }
        out.push_str("</ul>\n</nav>\n");
    }
    out.push_str("</article>\n");
    out
}

/// Render a manual page as a `<topic>`-tagged markdown projection.
pub fn render_page_markdown(
    page: &UserManualPage,
    sections: &[UserManualSection],
    anchors: &[UserManualAnchor],
) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("file_id: usermanual-{}\n", page.slug));
    out.push_str("file_kind: UserManualPageProjection\n");
    out.push_str(&format!("manual_version: \"{}\"\n", page.manual_version));
    out.push_str(&format!("updated_at: \"{}\"\n", page.updated_at.to_rfc3339()));
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n", page.title));
    out.push_str(
        "This is an on-demand projection. The PostgreSQL UserManual rows remain canonical.\n\n",
    );
    for section in sections {
        out.push_str(&format!(
            "<topic id=\"{}-{}\" status=\"current\" version=\"{}\" summary=\"{}\">\n\n",
            attr_escape(&page.slug),
            section.position,
            attr_escape(&page.manual_version),
            attr_escape(&section.title),
        ));
        out.push_str(&format!("## {}\n\n", section.title));
        out.push_str(&section.body_md);
        out.push_str("\n\n</topic>\n\n");
    }
    if !anchors.is_empty() {
        out.push_str(&format!(
            "<topic id=\"{}-anchors\" status=\"current\" version=\"{}\" summary=\"Anchors\">\n\n",
            attr_escape(&page.slug),
            attr_escape(&page.manual_version),
        ));
        out.push_str("## Anchors\n\n");
        for anchor in anchors {
            if anchor.http_method.is_empty() {
                out.push_str(&format!(
                    "- `{}`: `{}`\n",
                    anchor.anchor_kind, anchor.anchor_value
                ));
            } else {
                out.push_str(&format!(
                    "- `{}`: `{} {}`\n",
                    anchor.anchor_kind, anchor.http_method, anchor.anchor_value
                ));
            }
        }
        out.push_str("\n</topic>\n\n");
    }
    out
}

fn attr_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn text_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn page() -> UserManualPage {
        UserManualPage {
            page_id: "UMP-test".into(),
            slug: "test-page".into(),
            title: "Test <Page> & Co".into(),
            page_kind: "purpose".into(),
            audience: "model".into(),
            body: json!({}),
            content_hash: "0".repeat(64),
            manual_version: "2.0.0".into(),
            source_kind: "builtin_seed".into(),
            spec_anchors: vec![],
            status: "current".into(),
            superseded_by_slug: None,
            ledger_event_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn html_projection_escapes_and_carries_stable_selectors() {
        let sections = vec![UserManualSection {
            section_id: "UMS-1".into(),
            page_id: "UMP-test".into(),
            position: 0,
            section_kind: "purpose".into(),
            title: "What".into(),
            body_md: "Body with <script>alert(1)</script> & markdown".into(),
            body_json: None,
        }];
        let anchors = vec![UserManualAnchor {
            anchor_id: "UMA-1".into(),
            page_id: "UMP-test".into(),
            anchor_kind: "page_link".into(),
            anchor_value: "other-page".into(),
            http_method: "".into(),
        }];
        let html = render_page_html(&page(), &sections, &anchors);
        assert!(html.contains("data-hs-manual-page=\"test-page\""));
        assert!(html.contains("data-hs-section-kind=\"purpose\""));
        assert!(html.contains("data-hs-manual-link=\"other-page\""));
        assert!(html.contains("data-hs-href=\"/usermanual/pages/other-page\""));
        // Injection defense: raw script tags never survive into the DOM.
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn markdown_projection_uses_topic_tags_with_blank_line_padding() {
        let sections = vec![UserManualSection {
            section_id: "UMS-1".into(),
            page_id: "UMP-test".into(),
            position: 0,
            section_kind: "purpose".into(),
            title: "What".into(),
            body_md: "Body".into(),
            body_json: None,
        }];
        let md = render_page_markdown(&page(), &sections, &[]);
        assert!(md.contains("<topic id=\"test-page-0\""));
        assert!(md.contains("\">\n\n"));
        assert!(md.contains("\n\n</topic>"));
        assert!(md.starts_with("---\nfile_id: usermanual-test-page\n"));
    }
}
