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
    /// The page carries a usable, VISIBLE text layer (MT-086 #6): visible
    /// text-show operators present, at least `MIN_TEXT_LAYER_CHARS`
    /// non-whitespace characters extracted, and no decode/extraction error.
    /// Pages whose only text is invisible (`Tr 3`) or below the minimum are
    /// treated as image-only / no-text and routed to the OCR-needed path.
    pub fn has_text_layer(&self) -> bool {
        self.has_text_operators
            && self.extracted_chars >= MIN_TEXT_LAYER_CHARS
            && self.error.is_none()
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

/// Minimum non-whitespace characters a page must yield to count as a real
/// text layer (MT-086 #6). A page whose only "text" is a stray invisible or
/// sub-glyph operator producing one or two characters is treated as
/// image-only / no-text rather than a usable layer.
const MIN_TEXT_LAYER_CHARS: usize = 3;

/// PDF text render mode 3 = "invisible" (`<n> Tr` with n == 3). Text drawn in
/// this mode is not visible to a reader and is a classic OCR-overlay / hidden
/// watermark shape; we do not count it as a usable text layer.
const TEXT_RENDER_MODE_INVISIBLE: i64 = 3;

/// Run a closure that calls into lopdf, converting any panic raised inside
/// the parser into a typed error instead of aborting the whole ingestion pass
/// (MT-086/087 #5). One poison PDF degrades to one failed file.
///
/// The process-global panic hook is intentionally NOT swapped here: doing so
/// races other threads in a parallel runtime. `catch_unwind` still stops the
/// unwind at this boundary; the default hook may print a backtrace, which is
/// harmless noise.
fn guard_lopdf<T>(
    what: &str,
    f: impl FnOnce() -> Result<T, PdfAnalysisError>,
) -> Result<T, PdfAnalysisError> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(inner) => inner,
        Err(payload) => Err(PdfAnalysisError {
            class: IngestionErrorClass::Internal,
            detail: format!("lopdf panicked during {what}: {}", panic_message(&payload)),
        }),
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// Read a PDF numeric operand (integer or real) as an i64 for render-mode
/// comparison. Returns `None` for non-numeric operands.
fn object_as_i64(object: &Object) -> Option<i64> {
    object
        .as_i64()
        .ok()
        .or_else(|| object.as_float().ok().map(|f| f as i64))
}

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
///
/// A panic raised inside lopdf (crafted/malformed PDF) is caught and returned
/// as a typed `INTERNAL` error so one poison file degrades to one failed file
/// instead of aborting the whole ingestion pass (MT-086/087 #5).
pub fn analyze_text_layer(bytes: &[u8]) -> Result<PdfTextLayerReport, PdfAnalysisError> {
    guard_lopdf("analyze_text_layer", || analyze_text_layer_inner(bytes))
}

fn analyze_text_layer_inner(bytes: &[u8]) -> Result<PdfTextLayerReport, PdfAnalysisError> {
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
                    // Track text render mode (`<n> Tr`): a text-show operator
                    // only proves a VISIBLE text layer when the current mode is
                    // not 3 (invisible). MT-086 #6.
                    let mut render_mode: i64 = 0;
                    let mut has_visible_text = false;
                    for op in &content.operations {
                        match op.operator.as_str() {
                            "Tr" => {
                                if let Some(mode) =
                                    op.operands.first().and_then(|o| object_as_i64(o))
                                {
                                    render_mode = mode;
                                }
                            }
                            other if TEXT_SHOW_OPERATORS.contains(&other) => {
                                if render_mode != TEXT_RENDER_MODE_INVISIBLE {
                                    has_visible_text = true;
                                }
                            }
                            // Inline images (BI/ID/EI) are image content.
                            "BI" => analysis.has_images = true,
                            _ => {}
                        }
                    }
                    analysis.has_text_operators = has_visible_text;
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
    guard_lopdf("extract_pdf_text", || extract_pdf_text_inner(bytes))
}

fn extract_pdf_text_inner(bytes: &[u8]) -> Result<PdfExtraction, PdfAnalysisError> {
    let report = analyze_text_layer_inner(bytes)?;
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
            } else if analysis.has_text_operators && analysis.extracted_chars < MIN_TEXT_LAYER_CHARS
            {
                format!(
                    "weak_text_layer_below_minimum (extracted_chars={}, minimum_required={})",
                    analysis.extracted_chars, MIN_TEXT_LAYER_CHARS
                )
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
        /// An image-only page that ALSO carries an invisible text operator
        /// (`3 Tr` then a tiny `Tj`) — the OCR-overlay / hidden-text shape.
        /// MT-086 #6: invisible text must NOT count as a usable text layer.
        ImageOnlyWithInvisibleText(String),
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
                FixturePage::ImageOnlyWithInvisibleText(hidden) => {
                    // Same 2x2 image as ImageOnly, plus an INVISIBLE text run
                    // (`3 Tr` before the Tj). A reader sees only the image; a
                    // naive text-operator check would wrongly call this a text
                    // layer. MT-086 #6.
                    let image_stream = Stream::new(
                        dictionary! {
                            "Type" => "XObject",
                            "Subtype" => "Image",
                            "Width" => 2,
                            "Height" => 2,
                            "ColorSpace" => "DeviceRGB",
                            "BitsPerComponent" => 8,
                        },
                        vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0],
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
                        Operation::new("BT", vec![]),
                        Operation::new("Tf", vec!["F1".into(), 12.into()]),
                        Operation::new("Td", vec![72.into(), 700.into()]),
                        // Render mode 3 = invisible.
                        Operation::new("Tr", vec![3.into()]),
                        Operation::new("Tj", vec![Object::string_literal(hidden.clone())]),
                        Operation::new("ET", vec![]),
                    ];
                    (
                        dictionary! {
                            "Font" => dictionary! { "F1" => font_id },
                            "XObject" => dictionary! { "Im0" => image_id },
                        },
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

    /// Single-page image-only PDF with an INVISIBLE text run (`3 Tr`).
    /// MT-086 #6 fixture: a naive text-operator check would mis-classify this
    /// as a text layer; the detector must call it NO_TEXT_LAYER.
    pub fn invisible_text_pdf(hidden_text: &str) -> Vec<u8> {
        build_pdf(&[FixturePage::ImageOnlyWithInvisibleText(
            hidden_text.to_string(),
        )])
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
    fn truncated_pdf_header_fails_typed_not_panic() {
        // A PDF magic header followed by truncated garbage: must return a
        // typed error, never unwind (MT-086/087 #5).
        let bytes = b"%PDF-1.5\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendo";
        let result = analyze_text_layer(bytes);
        assert!(result.is_err(), "truncated PDF must be a typed error");
        let extract = extract_pdf_text(bytes);
        assert!(extract.is_err());
    }

    #[test]
    fn guard_lopdf_converts_panic_to_typed_internal_error() {
        // MT-086/087 #5: a panic inside the lopdf-calling closure degrades to
        // one typed failed file, it does NOT abort the process.
        let err = guard_lopdf::<()>("unit_panic", || {
            panic!("simulated lopdf parser panic");
        })
        .expect_err("panic must become a typed error");
        assert_eq!(err.class, IngestionErrorClass::Internal);
        assert!(err.detail.contains("panicked"));
        // The happy path still returns its value through the guard.
        let ok = guard_lopdf("unit_ok", || Ok::<u8, PdfAnalysisError>(7)).expect("ok path");
        assert_eq!(ok, 7);
    }

    #[test]
    fn invisible_text_pdf_is_not_a_text_layer() {
        // MT-086 #6: image-only page whose only text is invisible (`3 Tr`).
        let bytes = super::fixtures::invisible_text_pdf("hidden overlay words here");
        let report = analyze_text_layer(&bytes).expect("analyze");
        assert_eq!(
            report.verdict,
            PdfTextLayerVerdict::NoTextLayer,
            "invisible text must not count as a text layer: {:?}",
            report.pages
        );
        assert!(report.pages.iter().all(|p| !p.has_text_layer()));
        // It carries the image, so it is OCR-needed, not blank.
        assert_eq!(report.image_only_pages, 1);

        let err = extract_pdf_text(&bytes).expect_err("must not empty-succeed");
        assert_eq!(err.class, IngestionErrorClass::NoTextLayer);
    }

    #[test]
    fn tiny_text_below_minimum_is_not_a_text_layer() {
        // A page with a visible but sub-minimum text run (< MIN_TEXT_LAYER_CHARS
        // non-whitespace chars) is not a usable layer. MT-086 #6.
        let bytes = build_pdf(&[FixturePage::Text("a".to_string())]);
        let report = analyze_text_layer(&bytes).expect("analyze");
        assert_eq!(report.verdict, PdfTextLayerVerdict::NoTextLayer);
    }

    #[test]
    fn blank_pages_are_not_text_and_not_image() {
        let bytes = build_pdf(&[FixturePage::Blank]);
        let report = analyze_text_layer(&bytes).expect("analyze");
        assert_eq!(report.verdict, PdfTextLayerVerdict::NoTextLayer);
        assert_eq!(report.image_only_pages, 0);
    }
}
