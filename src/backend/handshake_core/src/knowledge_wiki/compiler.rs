//! WP-KERNEL-009 MT-241 `ProjectWikiBootstrapCompiler`.
//!
//! Bootstrap-compiles a navigable, typed, CITED project wiki from already-built
//! authority — the code index (`knowledge_code_files` + symbol/concept entities
//! and spans), knowledge entities/edges, and rich documents — into the EXISTING
//! `knowledge_wiki_projections` store (LM-PWIKI-005). The compiled wiki is the
//! Karpathy "compiled binary": a regenerable projection, never authority
//! (LM-PWIKI-001).
//!
//! Pages emitted (LM-PWIKI-002):
//! * `module`   — one page per source-directory cluster, token-aware packed
//!   (LM-PWIKI-004): files are grouped into `(part N)` pages within
//!   [`super::DEFAULT_PAGE_TOKEN_BUDGET`]; a single file larger than the
//!   budget gets its own page and is reported LOUDLY in the compile receipt
//!   (`oversize_files`), never silently truncated.
//! * `concept`  — the doc-passage concept entities of a directory cluster.
//! * `entity`   — one page per rich document.
//! * `decision` — one page per work-packet / micro-task / taskboard-row
//!   entity.
//! * `flow`     — type reserved (schema + verdict support exist); the
//!   bootstrap emits no flow pages until a real flow input exists in
//!   authority (emitting empty pages would violate the no-placeholder
//!   acceptance).
//! * `index`    — the catalog page listing every generated page.
//!
//! Citations are precise entity/span/source ids + content hashes
//! (LM-PWIKI-003) — never loose file-path strings. Every page is stamped
//! (LM-PWIKI-006) at write time: the stamp is a REQUIRED argument of
//! [`PostgresDatabase::upsert_knowledge_wiki_page`] (ship-together guard,
//! LM-PWIKI-009). EventLedger receives a `wiki_bootstrap_compile_started` /
//! `…_completed` receipt pair (LM-PWIKI-012); every page row references the
//! started receipt.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeEntityLifecycle, KnowledgeRichDocument, KnowledgeStore,
    KnowledgeWikiProjection, NewKnowledgeWikiPage, WikiCodeFileInput, WikiEntityWithSpan,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::Database;

use super::{
    entity_content_hash, estimate_tokens, rich_document_content_hash, CitedSource,
    CitedSourceKind, WikiCompileError, WikiCompileResult, WikiCompileStamp, WikiPageType,
    DEFAULT_PAGE_TOKEN_BUDGET, MAX_BOOTSTRAP_PAGES, MAX_PAGE_TOKEN_BUDGET, MIN_PAGE_TOKEN_BUDGET,
};

/// Caller identity for compile receipts (mirrors
/// `knowledge_code_index::engine::CodeIndexContext`).
#[derive(Clone, Debug)]
pub struct WikiCompileContext {
    pub actor: KernelActor,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub correlation_id: Option<String>,
}

impl WikiCompileContext {
    pub fn validate(&self) -> WikiCompileResult<()> {
        if self.kernel_task_run_id.trim().is_empty() || self.session_run_id.trim().is_empty() {
            return Err(WikiCompileError::Validation(
                "wiki compile context requires kernel_task_run_id and session_run_id".into(),
            ));
        }
        Ok(())
    }
}

/// Bootstrap options.
#[derive(Clone, Debug)]
pub struct WikiBootstrapOptions {
    /// Per-page token budget for module/concept clustering (clamped to
    /// [`MIN_PAGE_TOKEN_BUDGET`]..[`MAX_PAGE_TOKEN_BUDGET`]).
    pub page_token_budget: usize,
}

impl Default for WikiBootstrapOptions {
    fn default() -> Self {
        Self {
            page_token_budget: DEFAULT_PAGE_TOKEN_BUDGET,
        }
    }
}

/// Outcome of one bootstrap compile.
#[derive(Clone, Debug)]
pub struct WikiBootstrapOutcome {
    pub pages: Vec<KnowledgeWikiProjection>,
    pub started_receipt_event_id: String,
    pub completed_receipt_event_id: String,
    /// EventLedger source version every stamp of this run carries.
    pub ledger_version: i64,
    pub module_pages: usize,
    pub concept_pages: usize,
    pub entity_pages: usize,
    pub decision_pages: usize,
    /// Directory clusters that were split into multiple parts by the budget.
    pub split_clusters: usize,
    /// Files whose lone estimate exceeded the budget (LOUD, never silent).
    pub oversize_files: Vec<String>,
}

/// One file's compile bundle: index input + its symbol/concept citations.
#[derive(Clone, Debug)]
pub(crate) struct FileBundle {
    pub input: WikiCodeFileInput,
    pub symbols: Vec<WikiEntityWithSpan>,
    pub concepts: Vec<WikiEntityWithSpan>,
}

impl FileBundle {
    /// Conservative token estimate of this file's rendered footprint.
    fn estimated_tokens(&self) -> usize {
        let mut tokens = 16 + estimate_tokens(&self.input.relative_path);
        for symbol in &self.symbols {
            tokens += 10
                + estimate_tokens(&symbol.entity.display_name)
                + estimate_tokens(&symbol.entity.entity_key);
        }
        for concept in &self.concepts {
            tokens += 8 + estimate_tokens(&concept.entity.display_name);
        }
        tokens
    }
}

/// A packed module-page part (one rendered `module` page).
#[derive(Clone, Debug)]
pub(crate) struct ModulePart {
    pub dir: String,
    /// 1-based part ordinal; clusters that fit in one page have exactly one.
    pub part: usize,
    pub total_parts: usize,
    pub files: Vec<FileBundle>,
}

impl ModulePart {
    pub fn title(&self) -> String {
        module_page_title(&self.dir, self.part, self.total_parts)
    }
}

pub(crate) fn module_page_title(dir: &str, part: usize, total_parts: usize) -> String {
    if total_parts > 1 {
        format!("module: {dir} (part {part})")
    } else {
        format!("module: {dir}")
    }
}

pub(crate) fn concept_page_title(dir: &str) -> String {
    format!("concepts: {dir}")
}

pub const WIKI_INDEX_PAGE_TITLE: &str = "index: project wiki";

/// Directory cluster key of a repo-relative path (`(root)` for bare files).
pub(crate) fn cluster_dir(relative_path: &str) -> String {
    match relative_path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => "(root)".to_string(),
    }
}

/// The MT-241 bootstrap compiler. Holds a concrete PostgreSQL handle (this is
/// a PostgreSQL/EventLedger-native surface; no other backend exists for it).
pub struct ProjectWikiCompiler {
    db: Arc<PostgresDatabase>,
}

impl ProjectWikiCompiler {
    pub fn new(db: Arc<PostgresDatabase>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &PostgresDatabase {
        &self.db
    }

    /// Bootstrap-compile the project wiki for a workspace.
    pub async fn bootstrap(
        &self,
        ctx: &WikiCompileContext,
        workspace_id: &str,
        options: &WikiBootstrapOptions,
    ) -> WikiCompileResult<WikiBootstrapOutcome> {
        ctx.validate()?;
        let budget = options
            .page_token_budget
            .clamp(MIN_PAGE_TOKEN_BUDGET, MAX_PAGE_TOKEN_BUDGET);

        // LM-PWIKI-006: the EventLedger source version the stamps carry is the
        // watermark observed when the compile begins reading authority.
        let ledger_version = self.db.current_event_ledger_version().await?;

        // ---- gather compile inputs from authority --------------------------
        let code_inputs = self.db.list_wiki_code_file_inputs(workspace_id).await?;
        let mut bundles = Vec::with_capacity(code_inputs.len());
        for input in code_inputs {
            let symbols = self
                .db
                .list_wiki_source_entities_with_spans(
                    workspace_id,
                    &input.source_id,
                    KnowledgeEntityKind::Symbol,
                )
                .await?;
            let concepts = self
                .db
                .list_wiki_source_entities_with_spans(
                    workspace_id,
                    &input.source_id,
                    KnowledgeEntityKind::Concept,
                )
                .await?;
            bundles.push(FileBundle {
                input,
                symbols,
                concepts,
            });
        }

        // Directory clusters (BTreeMap: deterministic order).
        let mut clusters: BTreeMap<String, Vec<FileBundle>> = BTreeMap::new();
        for bundle in bundles {
            clusters
                .entry(cluster_dir(&bundle.input.relative_path))
                .or_default()
                .push(bundle);
        }

        // Cross-module dependency links (source-id level -> dir level).
        let cross_edges = self
            .db
            .list_wiki_cross_source_code_edges(workspace_id, 100_000)
            .await?;
        let source_dir: HashMap<String, String> = clusters
            .iter()
            .flat_map(|(dir, files)| {
                files
                    .iter()
                    .map(move |f| (f.input.source_id.clone(), dir.clone()))
            })
            .collect();
        let mut dir_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for edge in &cross_edges {
            if let (Some(from), Some(to)) = (
                source_dir.get(&edge.from_source_id),
                source_dir.get(&edge.to_source_id),
            ) {
                if from != to {
                    dir_deps.entry(from.clone()).or_default().insert(to.clone());
                }
            }
        }

        // ---- token-aware packing (LM-PWIKI-004) ----------------------------
        let mut module_parts: Vec<ModulePart> = Vec::new();
        let mut split_clusters = 0usize;
        let mut oversize_files: Vec<String> = Vec::new();
        for (dir, files) in &clusters {
            let parts = pack_files_into_parts(dir, files, budget, &mut oversize_files);
            if parts.len() > 1 {
                split_clusters += 1;
            }
            module_parts.extend(parts);
        }

        // ---- non-code page inputs ------------------------------------------
        let rich_documents = self
            .db
            .list_knowledge_rich_documents(workspace_id, None, None)
            .await?;
        let mut decision_entities = Vec::new();
        for kind in [
            KnowledgeEntityKind::WorkPacket,
            KnowledgeEntityKind::MicroTask,
            KnowledgeEntityKind::TaskboardRow,
        ] {
            decision_entities.extend(
                self.db
                    .list_knowledge_entities_by_kind(workspace_id, kind)
                    .await?
                    .into_iter()
                    .filter(|e| e.lifecycle_state == KnowledgeEntityLifecycle::Active),
            );
        }
        decision_entities.sort_by(|a, b| a.entity_key.cmp(&b.entity_key));
        // Source hashes for decision-entity citation hashing.
        let decision_ids: Vec<String> = decision_entities
            .iter()
            .map(|e| e.entity_id.clone())
            .collect();
        let decision_states = self.db.get_wiki_entity_states(&decision_ids).await?;
        let decision_source_hash: HashMap<String, Option<String>> = decision_states
            .into_iter()
            .map(|(e, h)| (e.entity_id, h))
            .collect();

        // Concept pages exist per cluster that HAS concepts.
        let concept_dirs: Vec<String> = clusters
            .iter()
            .filter(|(_, files)| files.iter().any(|f| !f.concepts.is_empty()))
            .map(|(dir, _)| dir.clone())
            .collect();

        // ---- unbounded-compile guard (page cap, LOUD) ----------------------
        let planned_pages = module_parts.len()
            + concept_dirs.len()
            + rich_documents.len()
            + decision_entities.len()
            + 1; // index
        if planned_pages > MAX_BOOTSTRAP_PAGES {
            return Err(WikiCompileError::PageCapExceeded(planned_pages));
        }

        // ---- started receipt (FK target for the page rows) ------------------
        let started_receipt_event_id = self
            .append_receipt(
                ctx,
                workspace_id,
                None,
                json!({
                    "kind": "wiki_bootstrap_compile_started",
                    "workspace_id": workspace_id,
                    "ledger_version": ledger_version,
                    "page_token_budget": budget,
                    "planned_pages": planned_pages,
                    "compiler_version": super::WIKI_COMPILER_VERSION,
                }),
            )
            .await?;

        // ---- compile + upsert pages -----------------------------------------
        let mut pages: Vec<KnowledgeWikiProjection> = Vec::with_capacity(planned_pages);

        for part in &module_parts {
            let dep_dirs = dir_deps.get(&part.dir).cloned().unwrap_or_default();
            let concept_link = concept_dirs.contains(&part.dir);
            let (rendered, citations, links) =
                render_module_page(part, &dep_dirs, concept_link, &module_parts);
            let page = self
                .upsert_page(
                    workspace_id,
                    &part.title(),
                    WikiPageType::Module,
                    rendered,
                    citations,
                    json!({
                        "kind": "module",
                        "dir": part.dir,
                        "part": part.part,
                        "total_parts": part.total_parts,
                        "source_ids": part.files.iter().map(|f| f.input.source_id.clone()).collect::<Vec<_>>(),
                    }),
                    links,
                    ledger_version,
                    &started_receipt_event_id,
                )
                .await?;
            pages.push(page);
        }
        let module_pages = module_parts.len();

        let mut concept_pages = 0usize;
        for dir in &concept_dirs {
            let files = &clusters[dir];
            let (rendered, citations, links) = render_concept_page(dir, files, &module_parts);
            let page = self
                .upsert_page(
                    workspace_id,
                    &concept_page_title(dir),
                    WikiPageType::Concept,
                    rendered,
                    citations,
                    json!({
                        "kind": "concept",
                        "dir": dir,
                        "source_ids": files
                            .iter()
                            .filter(|f| !f.concepts.is_empty())
                            .map(|f| f.input.source_id.clone())
                            .collect::<Vec<_>>(),
                    }),
                    links,
                    ledger_version,
                    &started_receipt_event_id,
                )
                .await?;
            pages.push(page);
            concept_pages += 1;
        }

        let mut entity_pages = 0usize;
        for document in &rich_documents {
            let (title, rendered, citations, links) = render_entity_page(document);
            let page = self
                .upsert_page(
                    workspace_id,
                    &title,
                    WikiPageType::Entity,
                    rendered,
                    citations,
                    json!({
                        "kind": "entity",
                        "rich_document_id": document.rich_document_id,
                    }),
                    links,
                    ledger_version,
                    &started_receipt_event_id,
                )
                .await?;
            pages.push(page);
            entity_pages += 1;
        }

        let mut decision_pages = 0usize;
        for entity in &decision_entities {
            let source_hash = decision_source_hash
                .get(&entity.entity_id)
                .cloned()
                .flatten();
            let (title, rendered, citations, links) =
                render_decision_page(entity, source_hash.as_deref());
            let page = self
                .upsert_page(
                    workspace_id,
                    &title,
                    WikiPageType::Decision,
                    rendered,
                    citations,
                    json!({
                        "kind": "decision",
                        "entity_id": entity.entity_id,
                    }),
                    links,
                    ledger_version,
                    &started_receipt_event_id,
                )
                .await?;
            pages.push(page);
            decision_pages += 1;
        }

        // ---- index/catalog page (LM-PWIKI-002) -------------------------------
        let index_page = self
            .compile_index_page(workspace_id, ledger_version, &started_receipt_event_id)
            .await?;
        pages.push(index_page);

        // ---- second pass: resolve wikilink titles -> projection ids ----------
        self.resolve_page_links(workspace_id, &mut pages).await?;

        // ---- completed receipt ------------------------------------------------
        let completed_receipt_event_id = self
            .append_receipt(
                ctx,
                workspace_id,
                Some(&started_receipt_event_id),
                json!({
                    "kind": "wiki_bootstrap_compile_completed",
                    "workspace_id": workspace_id,
                    "ledger_version": ledger_version,
                    "pages": pages
                        .iter()
                        .map(|p| json!({
                            "projection_id": p.projection_id,
                            "title": p.title,
                            "page_type": p.page_type,
                        }))
                        .collect::<Vec<_>>(),
                    "module_pages": module_pages,
                    "concept_pages": concept_pages,
                    "entity_pages": entity_pages,
                    "decision_pages": decision_pages,
                    "flow_pages": 0,
                    "index_pages": 1,
                    "split_clusters": split_clusters,
                    "oversize_files": oversize_files,
                    "compiler_version": super::WIKI_COMPILER_VERSION,
                }),
            )
            .await?;

        Ok(WikiBootstrapOutcome {
            pages,
            started_receipt_event_id,
            completed_receipt_event_id,
            ledger_version,
            module_pages,
            concept_pages,
            entity_pages,
            decision_pages,
            split_clusters,
            oversize_files,
        })
    }

    /// Compile (or refresh) the index/catalog page from the CURRENT page set.
    pub(crate) async fn compile_index_page(
        &self,
        workspace_id: &str,
        ledger_version: i64,
        receipt_event_id: &str,
    ) -> WikiCompileResult<KnowledgeWikiProjection> {
        let existing = self
            .db
            .list_knowledge_wiki_pages(workspace_id, None, true, 2_000, 0)
            .await?;
        let listed: Vec<&KnowledgeWikiProjection> = existing
            .iter()
            .filter(|p| p.title != WIKI_INDEX_PAGE_TITLE)
            .collect();

        let mut content = String::new();
        content.push_str("# index: project wiki\n");
        content.push_str("- type: index\n");
        content.push_str(&format!("- pages: {}\n", listed.len()));
        content.push_str(
            "\n_Compiled project wiki catalog. Generated projection — regenerable, never authority._\n",
        );
        let mut links = Vec::new();
        let mut current_type: Option<&str> = None;
        for page in &listed {
            let page_type = page.page_type.as_deref().unwrap_or("untyped");
            if current_type != Some(page_type) {
                content.push_str(&format!("\n## {page_type}\n"));
                current_type = Some(page_type);
            }
            content.push_str(&format!("- [[{}]]\n", page.title));
            links.push(json!({"title": page.title, "projection_id": page.projection_id}));
        }

        // The index derives from the page CATALOG, not from authority sources;
        // its exact cited-source set is empty (LM-PWIKI-006) and it is
        // refreshed on every compile/fan-out pass.
        let page = self
            .upsert_page(
                workspace_id,
                WIKI_INDEX_PAGE_TITLE,
                WikiPageType::Index,
                content,
                Vec::new(),
                json!({"kind": "index"}),
                Value::Array(links),
                ledger_version,
                receipt_event_id,
            )
            .await?;
        Ok(page)
    }

    /// Upsert one stamped page (stamp REQUIRED — ship-together guard).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn upsert_page(
        &self,
        workspace_id: &str,
        title: &str,
        page_type: WikiPageType,
        rendered_content: String,
        citations: Vec<CitedSource>,
        compile_recipe: Value,
        page_links: Value,
        ledger_version: i64,
        receipt_event_id: &str,
    ) -> WikiCompileResult<KnowledgeWikiProjection> {
        let stamp = WikiCompileStamp::new(ledger_version, citations);
        // Legacy staleness hash (MT-184 column, NOT NULL): hash of the stamp's
        // cited set — same drift signal, old shape.
        let staleness_hash = stamp_staleness_hash(&stamp);
        let source_records = stamp
            .cited_sources
            .iter()
            .map(|c| {
                let mut record = json!({
                    "record_family": citation_record_family(c.kind),
                    "record_id": c.id,
                    "content_hash": c.content_hash,
                });
                if let Some(span_id) = &c.span_id {
                    record["span_id"] = json!(span_id);
                }
                if let Some(source_id) = &c.source_id {
                    record["source_id"] = json!(source_id);
                }
                record
            })
            .collect::<Vec<_>>();
        let page = self
            .db
            .upsert_knowledge_wiki_page(NewKnowledgeWikiPage {
                workspace_id: workspace_id.to_string(),
                title: title.to_string(),
                page_type: Some(page_type.as_str().to_string()),
                source_records: Value::Array(source_records),
                rendered_content,
                staleness_hash,
                compile_stamp: stamp.to_value(),
                compile_recipe: Some(compile_recipe),
                page_links,
                rebuild_receipt_event_id: Some(receipt_event_id.to_string()),
            })
            .await?;
        Ok(page)
    }

    /// Second pass: fill `projection_id` into title-only wikilinks now that
    /// every page row exists. Wholesale replacement — idempotent.
    pub(crate) async fn resolve_page_links(
        &self,
        workspace_id: &str,
        pages: &mut [KnowledgeWikiProjection],
    ) -> WikiCompileResult<()> {
        let all = self
            .db
            .list_knowledge_wiki_pages(workspace_id, None, true, 2_000, 0)
            .await?;
        let id_by_title: HashMap<String, String> = all
            .iter()
            .map(|p| (p.title.clone(), p.projection_id.clone()))
            .collect();
        for page in pages.iter_mut() {
            let Some(links) = page.page_links.as_array() else {
                continue;
            };
            let mut changed = false;
            let resolved: Vec<Value> = links
                .iter()
                .map(|link| {
                    let title = link.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let known = link.get("projection_id").and_then(|p| p.as_str());
                    match (known, id_by_title.get(title)) {
                        (None, Some(id)) => {
                            changed = true;
                            json!({"title": title, "projection_id": id})
                        }
                        (Some(existing), Some(id)) if existing != id => {
                            changed = true;
                            json!({"title": title, "projection_id": id})
                        }
                        _ => link.clone(),
                    }
                })
                .collect();
            if changed {
                let value = Value::Array(resolved);
                self.db
                    .update_knowledge_wiki_page_links(&page.projection_id, &value)
                    .await?;
                page.page_links = value;
            }
        }
        Ok(())
    }

    pub(crate) async fn append_receipt(
        &self,
        ctx: &WikiCompileContext,
        workspace_id: &str,
        causation_id: Option<&str>,
        payload: Value,
    ) -> WikiCompileResult<String> {
        let mut builder = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            KernelEventType::KnowledgeProjectionRebuilt,
            ctx.actor.clone(),
        )
        .aggregate("knowledge_wiki", workspace_id.to_string())
        .source_component("project_wiki_compiler")
        .payload(payload);
        if let Some(correlation_id) = &ctx.correlation_id {
            builder = builder.correlation_id(correlation_id.clone());
        }
        if let Some(causation_id) = causation_id {
            builder = builder.causation_id(causation_id.to_string());
        }
        let event = self.db.append_kernel_event(builder.build()?).await?;
        Ok(event.event_id)
    }
}

/// Pack a cluster's files into budgeted parts (file granularity; a lone
/// over-budget file gets its own part and is reported in `oversize_files`).
pub(crate) fn pack_files_into_parts(
    dir: &str,
    files: &[FileBundle],
    budget: usize,
    oversize_files: &mut Vec<String>,
) -> Vec<ModulePart> {
    let mut groups: Vec<Vec<FileBundle>> = Vec::new();
    let mut current: Vec<FileBundle> = Vec::new();
    let mut current_tokens = 0usize;
    for file in files {
        let file_tokens = file.estimated_tokens();
        if file_tokens > budget {
            oversize_files.push(file.input.relative_path.clone());
        }
        if !current.is_empty() && current_tokens + file_tokens > budget {
            groups.push(std::mem::take(&mut current));
            current_tokens = 0;
        }
        current_tokens += file_tokens;
        current.push(file.clone());
    }
    if !current.is_empty() {
        groups.push(current);
    }
    let total_parts = groups.len();
    groups
        .into_iter()
        .enumerate()
        .map(|(i, files)| ModulePart {
            dir: dir.to_string(),
            part: i + 1,
            total_parts,
            files,
        })
        .collect()
}

fn citation_record_family(kind: CitedSourceKind) -> &'static str {
    match kind {
        CitedSourceKind::Source => "KnowledgeSource",
        CitedSourceKind::Entity => "KnowledgeEntity",
        CitedSourceKind::LoomBlock => "LoomBlock",
        CitedSourceKind::RichDocument => "KnowledgeRichDocument",
    }
}

/// Legacy `staleness_hash` over the stamp's cited set (deterministic).
pub(crate) fn stamp_staleness_hash(stamp: &WikiCompileStamp) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"project_wiki_stamp_v1");
    for cited in &stamp.cited_sources {
        hasher.update(b"|");
        hasher.update(cited.kind.as_str().as_bytes());
        hasher.update(b":");
        hasher.update(cited.id.as_bytes());
        hasher.update(b"@");
        hasher.update(cited.content_hash.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Sanitize one inline text fragment for deterministic markdown (strip
/// newlines/pipes so a hostile display name cannot forge page structure or
/// fake citation lines — render-injection guard).
fn inline(text: &str) -> String {
    let mut cleaned: String = text
        .chars()
        .map(|c| match c {
            '\n' | '\r' => ' ',
            '|' => '/',
            '[' => '(',
            ']' => ')',
            _ => c,
        })
        .collect();
    if cleaned.chars().count() > 160 {
        cleaned = cleaned.chars().take(160).collect::<String>() + "…";
    }
    cleaned
}

/// Render one module page. Returns (content, citations, links).
pub(crate) fn render_module_page(
    part: &ModulePart,
    dep_dirs: &BTreeSet<String>,
    has_concept_page: bool,
    all_parts: &[ModulePart],
) -> (String, Vec<CitedSource>, Value) {
    let mut content = String::new();
    let mut citations: Vec<CitedSource> = Vec::new();
    let mut links: Vec<Value> = Vec::new();

    content.push_str(&format!("# {}\n", part.title()));
    content.push_str("- type: module\n");
    content.push_str(&format!("- files: {}\n", part.files.len()));
    let symbol_count: usize = part.files.iter().map(|f| f.symbols.len()).sum();
    content.push_str(&format!("- symbols: {symbol_count}\n"));
    content.push_str(
        "\n_Compiled module page. Generated projection — regenerable, never authority._\n",
    );

    for file in &part.files {
        // File citation: the knowledge SOURCE record (id + content hash).
        citations.push(CitedSource {
            kind: CitedSourceKind::Source,
            id: file.input.source_id.clone(),
            content_hash: file.input.content_hash.clone(),
            span_id: None,
            source_id: None,
        });
        content.push_str(&format!("\n## {}\n", inline(&file.input.relative_path)));
        content.push_str(&format!(
            "- cite: source:{} hash:{}\n- language: {} | parse: {}\n",
            file.input.source_id,
            file.input.content_hash,
            file.input.language.as_str(),
            file.input.parse_status.as_str(),
        ));
        if !file.symbols.is_empty() {
            content.push_str("\n### Symbols\n");
            for symbol in &file.symbols {
                let lines = match (symbol.line_start, symbol.line_end) {
                    (Some(start), Some(end)) => format!(" L{start}-L{end}"),
                    _ => String::new(),
                };
                content.push_str(&format!(
                    "- `{}` — cite: entity:{} span:{}{}\n",
                    inline(&symbol.entity.display_name),
                    symbol.entity.entity_id,
                    symbol.span_id,
                    lines,
                ));
                citations.push(CitedSource {
                    kind: CitedSourceKind::Entity,
                    id: symbol.entity.entity_id.clone(),
                    content_hash: entity_content_hash(
                        &symbol.entity,
                        Some(&file.input.content_hash),
                    ),
                    span_id: Some(symbol.span_id.clone()),
                    source_id: Some(file.input.source_id.clone()),
                });
            }
        }
    }

    // Navigation: sibling parts, the cluster's concept page, dependencies,
    // and the index.
    for sibling in all_parts {
        if sibling.dir == part.dir && sibling.part != part.part {
            let title = sibling.title();
            content.push_str(&format!("\n- see: [[{title}]]\n"));
            links.push(json!({"title": title}));
        }
    }
    if has_concept_page {
        let title = concept_page_title(&part.dir);
        content.push_str(&format!("\n- concepts: [[{title}]]\n"));
        links.push(json!({"title": title}));
    }
    if !dep_dirs.is_empty() {
        content.push_str("\n### Depends on\n");
        for dep in dep_dirs {
            // Link to part 1 of the dependency cluster.
            if let Some(target) = all_parts.iter().find(|p| &p.dir == dep && p.part == 1) {
                let title = target.title();
                content.push_str(&format!("- [[{title}]]\n"));
                links.push(json!({"title": title}));
            }
        }
    }
    content.push_str(&format!("\n- index: [[{WIKI_INDEX_PAGE_TITLE}]]\n"));
    links.push(json!({"title": WIKI_INDEX_PAGE_TITLE}));

    (content, citations, Value::Array(links))
}

/// Render one concept page for a directory cluster (doc-passage concepts).
pub(crate) fn render_concept_page(
    dir: &str,
    files: &[FileBundle],
    all_parts: &[ModulePart],
) -> (String, Vec<CitedSource>, Value) {
    let mut content = String::new();
    let mut citations: Vec<CitedSource> = Vec::new();
    let mut links: Vec<Value> = Vec::new();

    content.push_str(&format!("# {}\n", concept_page_title(dir)));
    content.push_str("- type: concept\n");
    content.push_str(
        "\n_Doc-passage concepts of this module. Generated projection — regenerable, never authority._\n",
    );

    for file in files {
        if file.concepts.is_empty() {
            continue;
        }
        // The concept page compiles FROM this file's content: cite the source
        // record too (keeps the MT-243 per-source fan-out set equal to the
        // MT-242 drift set — a source edit reaches concept pages directly).
        citations.push(CitedSource {
            kind: CitedSourceKind::Source,
            id: file.input.source_id.clone(),
            content_hash: file.input.content_hash.clone(),
            span_id: None,
            source_id: None,
        });
        content.push_str(&format!("\n## {}\n", inline(&file.input.relative_path)));
        for concept in &file.concepts {
            content.push_str(&format!(
                "- {} — cite: entity:{} span:{}\n",
                inline(&concept.entity.display_name),
                concept.entity.entity_id,
                concept.span_id,
            ));
            citations.push(CitedSource {
                kind: CitedSourceKind::Entity,
                id: concept.entity.entity_id.clone(),
                content_hash: entity_content_hash(&concept.entity, Some(&file.input.content_hash)),
                span_id: Some(concept.span_id.clone()),
                source_id: Some(file.input.source_id.clone()),
            });
        }
    }

    if let Some(target) = all_parts.iter().find(|p| p.dir == dir && p.part == 1) {
        let title = target.title();
        content.push_str(&format!("\n- module: [[{title}]]\n"));
        links.push(json!({"title": title}));
    }
    content.push_str(&format!("\n- index: [[{WIKI_INDEX_PAGE_TITLE}]]\n"));
    links.push(json!({"title": WIKI_INDEX_PAGE_TITLE}));

    (content, citations, Value::Array(links))
}

/// Extract readable text from a ProseMirror/Tiptap document JSON (bounded).
fn prosemirror_excerpt(content_json: &Value, cap_chars: usize) -> String {
    fn walk(node: &Value, out: &mut String, cap: usize) {
        if out.chars().count() >= cap {
            return;
        }
        if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(text);
        }
        if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
            for child in children {
                walk(child, out, cap);
            }
        }
    }
    let mut out = String::new();
    walk(content_json, &mut out, cap_chars);
    if out.chars().count() > cap_chars {
        out = out.chars().take(cap_chars).collect::<String>() + "…";
    }
    out
}

/// Render one entity page for a rich document.
pub(crate) fn render_entity_page(
    document: &KnowledgeRichDocument,
) -> (String, String, Vec<CitedSource>, Value) {
    let suffix: String = document
        .rich_document_id
        .chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let title = format!("entity: {} ({suffix})", inline(&document.title));
    let mut content = String::new();
    content.push_str(&format!("# {title}\n"));
    content.push_str("- type: entity\n");
    content.push_str(&format!(
        "- cite: rich_document:{} hash:{}\n- doc_version: {}\n",
        document.rich_document_id, document.content_sha256, document.doc_version,
    ));
    content.push_str(
        "\n_Compiled entity page. Generated projection — regenerable, never authority._\n",
    );
    let excerpt = prosemirror_excerpt(&document.content_json, 600);
    if !excerpt.is_empty() {
        content.push_str(&format!("\n> {}\n", inline(&excerpt)));
    }
    content.push_str(&format!("\n- index: [[{WIKI_INDEX_PAGE_TITLE}]]\n"));
    let citations = vec![CitedSource {
        kind: CitedSourceKind::RichDocument,
        id: document.rich_document_id.clone(),
        content_hash: rich_document_content_hash(document),
        span_id: None,
        source_id: None,
    }];
    let links = Value::Array(vec![json!({"title": WIKI_INDEX_PAGE_TITLE})]);
    (title, content, citations, links)
}

/// Render one decision page for a work-packet / micro-task / taskboard-row
/// entity.
pub(crate) fn render_decision_page(
    entity: &crate::storage::knowledge::KnowledgeEntity,
    source_content_hash: Option<&str>,
) -> (String, String, Vec<CitedSource>, Value) {
    let suffix: String = entity
        .entity_id
        .chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let title = format!("decision: {} ({suffix})", inline(&entity.display_name));
    let content_hash = entity_content_hash(entity, source_content_hash);
    let mut content = String::new();
    content.push_str(&format!("# {title}\n"));
    content.push_str("- type: decision\n");
    content.push_str(&format!(
        "- cite: entity:{} hash:{}\n- entity_kind: {}\n- entity_key: {}\n",
        entity.entity_id,
        content_hash,
        entity.entity_kind.as_str(),
        inline(&entity.entity_key),
    ));
    content.push_str(
        "\n_Compiled decision page. Generated projection — regenerable, never authority._\n",
    );
    content.push_str(&format!("\n- index: [[{WIKI_INDEX_PAGE_TITLE}]]\n"));
    let citations = vec![CitedSource {
        kind: CitedSourceKind::Entity,
        id: entity.entity_id.clone(),
        content_hash,
        span_id: None,
        source_id: entity.primary_source_id.clone(),
    }];
    let links = Value::Array(vec![json!({"title": WIKI_INDEX_PAGE_TITLE})]);
    (title, content, citations, links)
}
