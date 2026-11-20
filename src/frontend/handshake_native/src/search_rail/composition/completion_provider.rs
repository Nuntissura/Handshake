//! Authorized Notes/Loom reads for native Handpick completion.
use std::sync::Arc;
use sha2::{Digest, Sha256};
use handpick::*;
use unicode_normalization::UnicodeNormalization;
use crate::backend::knowledge_documents::{HskDocumentHeaders, KnowledgeDocumentsClient};
use crate::local_account::AuthenticatedContext;
use crate::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, TextLeaf};

pub(super) const PAGE_SIZE: usize = 40;
const MAX_OWNER_ROWS: usize = 5_000;
const FILTER_DETAIL: &str = "Complete filter";
const TAG_DETAIL: &str = "Insert existing tag";

pub(super) fn writing_limits() -> Limits {
    Limits {
        max_text_bytes: 0,
        max_pins: 8,
        max_items: 100,
        max_item_bytes: (MAX_COMPLETION_FILTER_BYTES + FILTER_DETAIL.len())
            .max(1 + MAX_COMPLETION_TAG_NAME_BYTES + TAG_DETAIL.len()),
        max_actions_per_item: 16,
    }
}

#[derive(Clone)]
pub(super) enum Target {
    Filter(String),
    Tag { id: String, name: String },
    Note { id: String, updated_at: String, template: bool },
    Block { note_id: String, updated_at: String, doc_version: u64, target: crate::rich_editor::block_identity::references::Target, template: bool },
}

#[derive(Clone)]
pub(super) struct Row {
    pub item: SearchItem,
    pub choice: NativeCompletionChoice,
    pub target: Target,
}

pub(super) struct Page { pub rows: Vec<Row>, pub status: DeliveryStatus }
pub(super) enum Material {
    Inline(Child),
    Template(BlockNode),
    NoteEmbed { note_id: String, block_id: String },
    BlockEmbed { note_id: String, target: crate::rich_editor::block_identity::references::Target, label: String },
}

#[derive(Clone)]
pub(super) struct Provider {
    pub workspace: String,
    pub account: Arc<AuthenticatedContext>,
    pub base: String,
    pub runtime: tokio::runtime::Handle,
}

impl Provider {
    pub(super) fn declaration(&self) -> Result<WritingProvider, String> {
        Ok(WritingProvider {
            provider: ProviderId::new("native.notes-completion").map_err(str::to_owned)?,
            canonical_set: RevisionId::new(format!("{}:{}", self.account.session_id, self.workspace)).map_err(str::to_owned)?,
            capabilities: vec![CapabilityDeclaration { capability: WritingCapability::Search,
                support: CapabilitySupport::Supported { contract: RevisionId::new("notes-owner-v1").map_err(str::to_owned)? } }],
            max_query_bytes: MAX_COMPLETION_QUERY_BYTES, max_page_items: PAGE_SIZE,
        })
    }

    fn documents(&self) -> KnowledgeDocumentsClient {
        KnowledgeDocumentsClient::with_base_url(&self.base)
            .with_authenticated_context(self.account.clone())
    }

    fn active(&self) -> Result<(), String> {
        self.account.is_active().then_some(()).ok_or_else(|| "Completion authorization expired".into())
    }

    async fn tags(&self) -> Result<Vec<(String, String)>, String> {
        self.active()?;
        let client = crate::backend_client::LoomTagClient::new(&self.base, self.runtime.clone())
            .with_authenticated_context(Some(self.account.clone()));
        let rows = client.completion_tags(&self.workspace).await
            .map_err(|_| "Authorized tags could not be loaded")?;
        self.active()?;
        if rows.len() > MAX_OWNER_ROWS { return Err("Tag owner returned more rows than this completion can safely inspect".into()); }
        rows.into_iter().map(|row| {
            let name = row.title;
            if name.is_empty() || name.len() > MAX_COMPLETION_TAG_NAME_BYTES { return Err("Tag owner returned no usable name".into()); }
            Ok((row.block_id, name))
        }).collect()
    }

    pub(super) async fn lookup(&self, trigger: &NativeCompletionTrigger) -> Result<Page, String> {
        self.active()?;
        let mut rows = match trigger.lookup() {
            NativeCompletionLookup::QueryFilter { prefix } => {
                let tags = if prefix.starts_with("tag:") { self.tags().await? } else { Vec::new() };
                let names: Vec<_> = tags.iter().map(|(_, name)| name.as_str()).collect();
                let mut seen = std::collections::HashSet::new();
                trigger.operator_completions(&names).map_err(|error| error.to_string())?
                    .into_iter().filter(|text| seen.insert(text.clone())).map(|text| {
                        let key = key("native.query", &opaque_id(&text))?;
                        let action = Action { id: ActionId::new(format!("complete:{}", key.result.as_str())).map_err(str::to_owned)?, kind: ActionKind::ApplyCompletion };
                        Ok(Row { item: SearchItem { key: key.clone(), label: text.clone(), detail: Some(FILTER_DETAIL.into()), actions: vec![action.clone()] },
                            choice: NativeCompletionChoice { key, value: NativeCompletionValue::Text { action, text: text.clone() } }, target: Target::Filter(text) })
                    }).collect::<Result<Vec<_>, String>>()?
            }
            NativeCompletionLookup::Tags { name } => {
                self.tags().await?.into_iter().filter(|(_, label)| normalized(label).contains(&name))
                    .map(|(id, name)| {
                        let key = key("native.loom.tags", &id)?;
                        let action = Action { id: ActionId::new(format!("tag:{}", opaque_id(&id))).map_err(str::to_owned)?, kind: ActionKind::ApplyCompletion };
                        Ok(Row { item: SearchItem { key: key.clone(), label: format!("#{name}"), detail: Some(TAG_DETAIL.into()), actions: vec![action.clone()] },
                            choice: NativeCompletionChoice { key, value: NativeCompletionValue::Tag { action, label: name.clone() } }, target: Target::Tag { id, name } })
                    }).collect::<Result<Vec<_>, String>>()?
            }
            NativeCompletionLookup::Notes { title }
            | NativeCompletionLookup::Templates { source: NativeTemplateSource::Note { title } } => {
                let headers = HskDocumentHeaders::for_read(&self.account.session_id, "handpick-completion");
                let documents = self.documents().list_documents(&headers, &self.workspace).await
                    .map_err(|_| "Authorized Notes could not be loaded")?;
                self.active()?;
                if documents.len() > MAX_OWNER_ROWS { return Err("Notes owner returned more rows than this completion can safely inspect".into()); }
                let template = trigger.kind() == NativeCompletionKind::Template;
                documents.into_iter().filter(|document| normalized(&document.title).contains(&title))
                    .map(|document| {
                        let key = key("native.notes", &document.rich_document_id)?;
                        let value = if template { NativeCompletionValue::Template } else {
                            NativeCompletionValue::Reference { target: NoteAddress::Note {
                                note: NoteId::new(&document.rich_document_id).map_err(str::to_owned)? },
                                label: document.title.clone(), embed: trigger.embed() }
                        };
                        Ok(Row { item: SearchItem { key: key.clone(), label: document.title, detail: Some(if template { "Insert note content" } else if trigger.embed() { "Embed note" } else { "Insert note reference" }.into()), actions: Vec::new() },
                            choice: NativeCompletionChoice { key, value }, target: Target::Note { id: document.rich_document_id, updated_at: document.updated_at, template } })
                    }).collect::<Result<Vec<_>, String>>()?
            }
            NativeCompletionLookup::Blocks { note_title, text, headings_only } => {
                return self.block_rows(note_title.as_deref(), &text, headings_only, false, trigger.embed()).await;
            }
            NativeCompletionLookup::Templates { source: NativeTemplateSource::Blocks { note_title, text, headings_only } } => {
                return self.block_rows(Some(&note_title), &text, headings_only, true, false).await;
            }
        };
        self.active()?;
        rows.sort_by(|left, right| left.item.label.cmp(&right.item.label).then_with(|| left.item.key.result.as_str().cmp(right.item.key.result.as_str())));
        let status = if rows.len() > PAGE_SIZE { DeliveryStatus::Partial } else { DeliveryStatus::Complete };
        rows.truncate(PAGE_SIZE);
        Ok(Page { rows, status })
    }

    async fn source_document(&self, id: &str, updated_at: &str) -> Result<crate::backend_client::RichDocBody, String> {
        self.active()?;
        let client = crate::backend_client::RichDocClient::new(&self.base, self.runtime.clone())
            .with_authenticated_context(Some(self.account.clone()));
        let document = client.load_document(id).await.map_err(|_| "The selected note could not be resolved")?;
        self.active()?;
        if document.document_id != id || document.workspace_id != self.workspace || document.updated_at != updated_at {
            return Err("The selected note changed; invoke completion again".into());
        }
        Ok(document)
    }

    pub(super) async fn resolve(&self, row: &Row, plan: &NativeCompletionPlan) -> Result<Material, String> {
        self.active()?;
        if plan.choice().key != row.choice.key { return Err("Completion result changed".into()); }
        let material = match (&row.target, &plan.choice().value) {
            (Target::Filter(text), NativeCompletionValue::Text { text: selected, .. }) if text == selected => {
                Material::Inline(Child::Text(TextLeaf::new(text)))
            }
            (Target::Tag { id, name }, NativeCompletionValue::Tag { label, .. }) if name == label => {
                if !self.tags().await?.iter().any(|(current_id, current_name)| current_id == id && current_name == name) {
                    return Err("The selected tag changed; invoke completion again".into());
                }
                Material::Inline(Child::HsLink(HsLinkNode::tag_hub_id(id, name)))
            }
            (Target::Block { note_id, updated_at, doc_version, target, template }, value) => {
                let document = self.source_document(note_id, updated_at).await?;
                if document.doc_version != *doc_version {
                    return Err("The selected block source version changed; invoke completion again".into());
                }
                let content = crate::rich_editor::document_model::doc_json::from_json_value(&document.content_json)
                    .map_err(|_| "The referenced note is incompatible with Notes")?;
                target.locate(&content)?;
                match (template, value) {
                    (true, NativeCompletionValue::Template) => Material::Template(content),
                    (false, NativeCompletionValue::Reference { target: address, label, embed: false })
                        if blocks::matches_address(target, note_id, address) =>
                            Material::Inline(Child::HsLink(target.link(note_id, label))),
                    (false, NativeCompletionValue::Reference { target: address, label, embed: true })
                        if blocks::matches_address(target, note_id, address) =>
                            Material::BlockEmbed { note_id: note_id.clone(), target: target.clone(), label: label.clone() },
                    _ => return Err("Completion action no longer matches the selected block".into()),
                }
            }
            (Target::Note { id, updated_at, template }, value) => {
                let document = self.source_document(id, updated_at).await?;
                match (template, value) {
                    (true, NativeCompletionValue::Template) => Material::Template(
                        crate::rich_editor::document_model::doc_json::from_json_value(&document.content_json)
                            .map_err(|_| "Template content is incompatible with Notes")?),
                    (false, NativeCompletionValue::Reference { target: NoteAddress::Note { note }, label, embed: false }) if note.as_str() == id => {
                        Material::Inline(Child::HsLink(HsLinkNode::new("note", id, label)))
                    }
                    (false, NativeCompletionValue::Reference { target: NoteAddress::Note { note }, embed: true, .. })
                        if note.as_str() == id && plan.choice().key.provider.as_str() == "native.notes"
                            && plan.choice().key.result.as_str() == id => {
                        let block_id = document.block_id.as_deref().filter(|id| !id.trim().is_empty())
                            .ok_or("The selected note has no canonical Loom identity; writing retained")?;
                        let loom = crate::backend::loom::LoomClient::with_client(
                            crate::backend_client::shared_http_client(), &self.base)
                            .with_authenticated_context(Some(self.account.clone()));
                        let source = loom.get_loom_block_transclusion(&self.workspace, block_id).await
                            .map_err(|_| "The selected note embed could not be resolved; writing retained")?;
                        self.active()?;
                        if source.block_id != block_id || source.workspace_id != self.workspace
                            || source.source_document_id.as_deref() != Some(id.as_str()) {
                            return Err("The selected note embed belongs to a different source; writing retained".into());
                        }
                        if !source.resolved || source.content_json.is_none() {
                            return Err("The selected note embed is unavailable; writing retained".into());
                        }
                        if source.source_doc_version.and_then(|version| u64::try_from(version).ok()) != Some(document.doc_version) {
                            return Err("The selected note changed; invoke completion again".into());
                        }
                        Material::NoteEmbed { note_id: id.clone(), block_id: block_id.to_owned() }
                    }
                    _ => return Err("Completion action no longer matches the selected note".into()),
                }
            }
            _ => return Err("Completion action changed".into()),
        };
        self.active()?;
        Ok(material)
    }
}

fn normalized(value: &str) -> String { value.nfc().collect::<String>().to_lowercase() }
fn key(provider: &str, result: &str) -> Result<ResultKey, String> {
    Ok(ResultKey { provider: ProviderId::new(provider).map_err(str::to_owned)?, result: ResultId::new(result).map_err(str::to_owned)? })
}

fn opaque_id(value: &str) -> String { format!("{:x}", Sha256::digest(value.as_bytes())) }

mod blocks;

#[cfg(test)]
mod tests;
