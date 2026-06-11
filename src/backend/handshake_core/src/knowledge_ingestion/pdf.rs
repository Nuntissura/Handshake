//! MT-086 PdfTextLayerDetector + MT-087 PdfTranscriptImportPath.
//!
//! Crate decision (documented per MT-086): `lopdf` 0.41 (MIT, pure Rust, no
//! native deps, no daemon — deny.toml compatible). It parses real PDF
//! structure (xref, object streams, page tree, content streams) and ships
//! text extraction with font/encoding handling; `pdf-extract` was rejected
//! because it layers extra font-parsing dependencies on top of this same
//! crate for marginal gain at our fixture/report fidelity level. lopdf also
//! CREATES PDFs, so the test fixtures (text-layer page, image-only page) are
//! generated programmatically — no opaque binary fixtures in the repo.
//!
//! Detection model: a page HAS a text layer when its decoded content stream
//! contains text-showing operators (`Tj`, `TJ`, `'`, `"`) AND extraction
//! yields non-whitespace characters. A page with image XObjects (or inline
//! images) and no text layer is `image_only` — the scanned-document shape
//! that needs OCR. Handshake runs NO OCR (no external runtime dependency,
//! MT-087): an image-only PDF yields a typed `NO_TEXT_LAYER` result with
//! repairable metadata (`OCR_NEEDED` guidance in the detail), NEVER an
//! empty-success.

use lopdf::content::Content;
use lopdf::{Document, Object, ObjectId};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::receipts::IngestionErrorClass;

/// Extractor identity recorded on every PDF extraction receipt.
pub const PDF_EXTRACTOR_ID: &str = "pdf_text_layer";
/// Extractor version: bump when the detection/extraction logic changes.
pub const PDF_EXTRACTOR_VERSION: &str = "lopdf-0.41-v1";

/// Per-page analysis result.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PdfPageAnalysis {
    /// 1-based page number.
    pub page: u32,
    pub has_text_operators: bool,
    pub has_images: bool,
    /// Non-whitespace characters the extractor produced for the page.
    pub extracted_chars: usize,
    /// Content-stream decode / extraction error, if any.
    pub error: Option<String>,
}

impl PdfPageAnalysis {
    /// The page carries a usable text layer.
    pub fn has_text_layer(&self) -> bool {
        self.has_text_operators && self.extracted_chars > 0 && self.error.is_none()
    }
}

/// Whole-document text-layer verdict.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PdfTextLayerVerdict {
    /// Every page has a text layer.
    TextLayer,
    /// Some pages do, some do not (or failed to decode).
    PartialTextLayer,
    /// No page has a text layer.
    NoTextLayer,
}

/// MT-086 typed detection result.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PdfTextLayerReport {
    pub verdict: PdfTextLayerVerdict,
    pub total_pages: usize,
    pub pages_with_text: usize,
    /// Image-bearing pages without text: the OCR-needed population.
    pub image_only_pages: usize,
    pub pages: Vec<PdfPageAnalysis>,
}

impl PdfTextLayerReport {
    /// Repairable-metadata payload for receipts (MT-086: "repairable
    /// metadata instead of silent partial indexing").
    pub fn detail_json(&self) -> serde_json::Value {
        json!({
            "verdict": self.verdict,
            "total_pages": self.total_pages,
            "pages_with_text": self.pages_with_text,
            "image_only_pages": self.image_only_pages,
            "repair_hint": if self.image_only_pages > 0 {
                "OCR_NEEDED: run external OCR over the listed pages and re-import the produced transcript artifact"
            } else {
                "no extractable text operators present"
            },
            "pages": self.pages,
        })
    }
}

/// Typed whole-file analysis failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfAnalysisError {
    pub class: IngestionErrorClass,
    pub detail: String,
}

const TEXT_SHOW_OPERATORS: &[&str] = &["Tj", "TJ", "'", "\""];

fn object_is_image(doc: &Document, object: &Object) -> bool {
    let resolved = match object {
        Object::Reference(id) => match doc.get_object(*id) {
            Ok(resolved) => resolved,
            Err(_) => return false,
        },
        other => other,
    };
    resolved
        .as_stream()
        .ok()
        .and_then(|stream| stream.dict.get(b"Subtype").ok())
        .and_then(|subtype| subtype.as_name().ok())
        .map(|name| name == b"Image")
        .unwrap_or(false)
}

fn page_has_images(doc: &Document, page_id: ObjectId) -> bool {
    let Ok((resource_dict, resource_ids)) = doc.get_page_resources(page_id) else {
        return false;
    };
    let mut dicts: Vec<&lopdf::Dictionary> = Vec::new();
    if let Some(dict) = resource_dict {
        dicts.push(dict);
    }
    for id in resource_ids {
        if let Ok(dict) = doc.get_dictionary(id) {
            dicts.push(dict);
        }
    }
    for resources in dicts {
        let Ok(xobjects) = resources.get(b"XObject").and_then(Object::as_dict) else {
            continue;
        };
        if xobjects.iter().any(|(_, obj)| object_is_image(doc, obj)) {
            return true;
        }
    }
    false
}

fn load_document(bytes: &[u8]) -> Result<Document, PdfAnalysisError> {
    let doc = Document::load_mem(bytes).map_err(|err| PdfAnalysisError {
        class: IngestionErrorClass::ParseError,
        detail: format!("not a parseable PDF: {err}"),
    })?;
    if doc.is_encrypted() {
        return Err(PdfAnalysisError {
            class: IngestionErrorClass::Encrypted,
            detail: "PDF is encrypted; provide a decrypted copy for ingestion".to_string(),
        });
    }
    Ok(doc)
}

/// MT-086: analyze whether the PDF carries a text layer, per page.
pub fn analyze_text_layer(bytes: &[u8]) -> Result<PdfTextLayerReport, PdfAnalysisError> {
    let doc = load_document(bytes)?;
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(PdfAnalysisError {
            class: IngestionErrorClass::ParseError,
            detail: "PDF has no pages".to_string(),
        });
    }

    let mut analyses = Vec::with_capacity(pages.len());
    for (&page_no, &page_id) in &pages {
        let mut analysis = PdfPageAnalysis {
            page: page_no,
            has_text_operators: false,
            has_images: page_has_images(&doc, page_id),
            extracted_chars: 0,
            error: None,
        };

        match doc.get_page_content(page_id) {
            Ok(content_bytes) => match Content::decode(&content_bytes) {
                Ok(content) => {
                    analysis.has_text_operators = content
                        .operations
                        .iter()
                        .any(|op| TEXT_SHOW_OPERATORS.contains(&op.operator.as_str()));
                    // Inline images (BI/ID/EI) count as image content, not text.
                    if content.operations.iter().any(|op| op.operator == "BI") {
                        analysis.has_images = true;
                    }
                }
                Err(err) => analysis.error = Some(format!("content decode: {err}")),
            },
            Err(err) => analysis.error = Some(format!("page content: {err}")),
        }

        if analysis.has_text_operators {
            match doc.extract_text(&[page_no]) {
                Ok(text) => {
                    analysis.extracted_chars = text.chars().filter(|c| !c.is_whitespace()).count();
                }
                Err(err) => analysis.error = Some(format!("text extraction: {err}")),
            }
        }
        analyses.push(analysis);
    }

    let total_pages = analyses.len();
    let pages_with_text = analyses.iter().filter(|p| p.has_text_layer()).count();
    let image_only_pages = analyses
        .iter()
        .filter(|p| p.has_images && !p.has_text_layer())
        .count();
    let verdict = if pages_with_text == total_pages {
        PdfTextLayerVerdict::TextLayer
    } else if pages_with_text == 0 {
        PdfTextLayerVerdict::NoTextLayer
    } else {
        PdfTextLayerVerdict::PartialTextLayer
    };

    Ok(PdfTextLayerReport {
        verdict,
        total_pages,
        pages_with_text,
        image_only_pages,
        pages: analyses,
    })
}

/// One extracted page.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PdfPageText {
    /// 1-based page number.
    pub page: u32,
    pub text: String,
}

/// One page that produced no text.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PdfPageFailure {
    pub page: u32,
    pub reason: String,
}

/// MT-087 extraction result: page texts + explicit page failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfExtraction {
    pub pages: Vec<PdfPageText>,
    pub failed_pages: Vec<PdfPageFailure>,
    pub report: PdfTextLayerReport,
}

impl PdfExtraction {
    pub fn is_partial(&self) -> bool {
        !self.failed_pages.is_empty()
    }
}

/// MT-087: extract page texts from a text-layer PDF.
///
/// * No page with a text layer -> typed `NO_TEXT_LAYER` error (never an
///   empty success), with OCR guidance in the detail when images exist.
/// * Some pages without text -> Ok with `failed_pages` populated; the
///   caller MUST write a `partial` receipt.
pub fn extract_pdf_text(bytes: &[u8]) -> Result<PdfExtraction, PdfAnalysisError> {
    let report = analyze_text_layer(bytes)?;
    if report.pages_with_text == 0 {
        let class = IngestionErrorClass::NoTextLayer;
        return Err(PdfAnalysisError {
            class,
            detail: serde_json::to_string(&report.detail_json())
                .unwrap_or_else(|_| "no text layer; OCR needed for image-only pages".to_string()),
        });
    }

    let doc = load_document(bytes)?;
    let mut pages = Vec::new();
    let mut failed_pages = Vec::new();
    for analysis in &report.pages {
        if analysis.has_text_layer() {
            match doc.extract_text(&[analysis.page]) {
                Ok(text) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        failed_pages.push(PdfPageFailure {
                            page: analysis.page,
                            reason: "extraction yielded only whitespace".to_string(),
                        });
                    } else {
                        pages.push(PdfPageText {
                            page: analysis.page,
                            text: trimmed.to_string(),
                        });
                    }
                }
                Err(err) => failed_pages.push(PdfPageFailure {
                    page: analysis.page,
                    reason: format!("text extraction: {err}"),
                }),
            }
        } else {
            let reason = if let Some(error) = &analysis.error {
                format!("page failed analysis: {error}")
            } else if analysis.has_images {
                "image_only_no_text_layer (OCR_NEEDED)".to_string()
            } else {
                "page has no text operators".to_string()
            };
            failed_pages.push(PdfPageFailure {
                page: analysis.page,
                reason,
            });
        }
    }

    if pages.is_empty() {
        return Err(PdfAnalysisError {
            class: IngestionErrorClass::NoTextLayer,
            detail: "every text-bearing page failed extraction".to_string(),
        });
    }
    Ok(PdfExtraction {
        pages,
        failed_pages,
        report,
    })
}

/// Programmatic fixture PDFs (test builds only): real PDFs built through the
/// same library the detector parses, so fixtures exercise genuine PDF
/// structure instead of opaque committed binaries.
#[cfg(any(test, feature = "test-utils"))]
pub mod fixtures {
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Document, Object, Stream};

    /// What each page of a generated fixture contains.
    #[derive(Clone, Debug)]
    pub enum FixturePage {
        /// A page with a real text layer (Helvetica, one Tj op per line).
        Text(String),
        /// An image-only page: a 2x2 RGB XObject drawn, zero text operators.
        ImageOnly,
        /// A page with neither text nor images.
        Blank,
    }

    /// Build a real PDF with the given pages and return its bytes.
    pub fn build_pdf(pages: &[FixturePage]) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });

        let mut kids: Vec<Object> = Vec::new();
        for page in pages {
            let (resources, operations) = match page {
                FixturePage::Text(text) => {
                    let mut ops = vec![
                        Operation::new("BT", vec![]),
                        Operation::new("Tf", vec!["F1".into(), 12.into()]),
                        Operation::new("Td", vec![72.into(), 720.into()]),
                    ];
                    for line in text.lines() {
                        ops.push(Operation::new(
                            "Tj",
                            vec![Object::string_literal(line.to_string())],
                        ));
                        ops.push(Operation::new("Td", vec![0.into(), (-14).into()]));
                    }
                    ops.push(Operation::new("ET", vec![]));
                    (
                        dictionary! { "Font" => dictionary! { "F1" => font_id } },
                        ops,
                    )
                }
                FixturePage::ImageOnly => {
                    // 2x2 raw RGB image XObject (red, green, blue, yellow).
                    let image_stream = Stream::new(
                        dictionary! {
                            "Type" => "XObject",
                            "Subtype" => "Image",
                            "Width" => 2,
                            "Height" => 2,
                            "ColorSpace" => "DeviceRGB",
                            "BitsPerComponent" => 8,
                        },
                        vec![
                            255, 0, 0, 0, 255, 0, // row 1: red, green
                            0, 0, 255, 255, 255, 0, // row 2: blue, yellow
                        ],
                    );
                    let image_id = doc.add_object(image_stream);
                    let ops = vec![
                        Operation::new("q", vec![]),
                        Operation::new(
                            "cm",
                            vec![
                                200.into(),
                                0.into(),
                                0.into(),
                                200.into(),
                                100.into(),
                                500.into(),
                            ],
                        ),
                        Operation::new("Do", vec!["Im0".into()]),
                        Operation::new("Q", vec![]),
                    ];
                    (
                        dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
                        ops,
                    )
                }
                FixturePage::Blank => (dictionary! {}, vec![]),
            };

            let content = Content { operations };
            let content_id = doc.add_object(Stream::new(
                dictionary! {},
                content.encode().expect("encode fixture content stream"),
            ));
            let resources_id = doc.add_object(resources);
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            kids.push(page_id.into());
        }

        let count = kids.len() as i64;
        doc.set_object(
            pages_id,
            dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => count,
            },
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("serialize fixture PDF");
        bytes
    }

    /// One text page per entry.
    pub fn text_pdf(pages: &[&str]) -> Vec<u8> {
        let pages: Vec<FixturePage> = pages
            .iter()
            .map(|t| FixturePage::Text((*t).to_string()))
            .collect();
        build_pdf(&pages)
    }

    /// Image-only (scanned-document shape) PDF.
    pub fn image_only_pdf(page_count: usize) -> Vec<u8> {
        build_pdf(&vec![FixturePage::ImageOnly; page_count])
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{build_pdf, image_only_pdf, text_pdf, FixturePage};
    use super::*;

    #[test]
    fn text_pdf_is_detected_as_text_layer_and_extracts_pages() {
        let bytes = text_pdf(&["Alpha page one content", "Beta page two content"]);
        let report = analyze_text_layer(&bytes).expect("analyze");
        assert_eq!(report.verdict, PdfTextLayerVerdict::TextLayer);
        assert_eq!(report.total_pages, 2);
        assert_eq!(report.pages_with_text, 2);
        assert_eq!(report.image_only_pages, 0);

        let extraction = extract_pdf_text(&bytes).expect("extract");
        assert_eq!(extraction.pages.len(), 2);
        assert!(!extraction.is_partial());
        assert!(extraction.pages[0].text.contains("Alpha page one content"));
        assert!(extraction.pages[1].text.contains("Beta page two content"));
        assert_eq!(extraction.pages[0].page, 1);
        assert_eq!(extraction.pages[1].page, 2);
    }

    #[test]
    fn image_only_pdf_yields_typed_no_text_layer_never_empty_success() {
        let bytes = image_only_pdf(2);
        let report = analyze_text_layer(&bytes).expect("analyze");
        assert_eq!(report.verdict, PdfTextLayerVerdict::NoTextLayer);
        assert_eq!(report.image_only_pages, 2);
        assert!(report.pages.iter().all(|p| p.has_images));
        assert!(report.detail_json()["repair_hint"]
            .as_str()
            .expect("repair hint")
            .contains("OCR_NEEDED"));

        let err = extract_pdf_text(&bytes).expect_err("image-only must not empty-succeed");
        assert_eq!(err.class, IngestionErrorClass::NoTextLayer);
        assert!(err.detail.contains("image_only_pages"));
    }

    #[test]
    fn mixed_pdf_extracts_partially_with_explicit_page_failures() {
        let bytes = build_pdf(&[
            FixturePage::Text("Readable first page".to_string()),
            FixturePage::ImageOnly,
            FixturePage::Text("Readable third page".to_string()),
        ]);
        let report = analyze_text_layer(&bytes).expect("analyze");
        assert_eq!(report.verdict, PdfTextLayerVerdict::PartialTextLayer);
        assert_eq!(report.pages_with_text, 2);
        assert_eq!(report.image_only_pages, 1);

        let extraction = extract_pdf_text(&bytes).expect("partial extraction");
        assert!(extraction.is_partial());
        assert_eq!(extraction.pages.len(), 2);
        assert_eq!(extraction.failed_pages.len(), 1);
        assert_eq!(extraction.failed_pages[0].page, 2);
        assert!(extraction.failed_pages[0].reason.contains("OCR_NEEDED"));
    }

    #[test]
    fn garbage_bytes_fail_typed_parse_error() {
        let err = analyze_text_layer(b"this is not a pdf at all").expect_err("garbage");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
    }

    #[test]
    fn blank_pages_are_not_text_and_not_image() {
        let bytes = build_pdf(&[FixturePage::Blank]);
        let report = analyze_text_layer(&bytes).expect("analyze");
        assert_eq!(report.verdict, PdfTextLayerVerdict::NoTextLayer);
        assert_eq!(report.image_only_pages, 0);
    }
}
