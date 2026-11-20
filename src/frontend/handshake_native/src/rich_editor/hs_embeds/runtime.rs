use super::*;
use crate::diagnostics::gui_events::GuiDiagnosticPhase;
use std::collections::{HashMap,VecDeque};
use std::sync::{Arc,Mutex};
use crate::backend::knowledge_documents::{HskDocumentHeaders,KnowledgeDocumentsClient,KnowledgeDocumentsError};
use crate::local_account::AuthenticatedContext;
use crate::rich_editor::document_owner::{DocumentEditOwner,EditorSaveContext};

const MAX_IN_FLIGHT_PREVIEWS:usize=4;

#[derive(Clone)]
struct Binding {
    owner:DocumentEditOwner,
    context:EditorSaveContext,
    workspace:String,
    endpoint:String,
    account:Arc<AuthenticatedContext>,
}
impl Binding {
    fn current(&self)->bool {
        self.owner.current_context().ok()==Some(self.context)
            && self.owner.has_active_binding(&self.workspace,&self.endpoint,&self.account)
    }
    fn same(&self,other:&Self)->bool {
        self.owner.same_owner(&other.owner) && self.context==other.context
            && self.workspace==other.workspace && self.endpoint==other.endpoint
            && Arc::ptr_eq(&self.account,&other.account)
    }
}

pub(crate) struct Cache {
    slots: Arc<tokio::sync::Semaphore>,
    binding:Option<Binding>,
    entries:HashMap<Key,Status>,
    references:Vec<Reference>,
    deliveries:Arc<Mutex<VecDeque<(Key,Status)>>>,
}
impl Default for Cache {
    fn default() -> Self {
        Self { binding:None,entries:HashMap::new(),references:Vec::new(),deliveries:Default::default(),
            slots:Arc::new(tokio::sync::Semaphore::new(MAX_IN_FLIGHT_PREVIEWS)) }
    }
}

impl Cache {
    pub(crate) fn clear(&mut self) {
        self.binding=None;
        self.entries.clear();
        self.references.clear();
        self.deliveries=Arc::new(Mutex::new(VecDeque::new()));
    }

    pub(crate) fn frame(&mut self,owner:&DocumentEditOwner,account:Option<Arc<AuthenticatedContext>>,
        endpoint:&str,document:&BlockNode,control_height:f32)->Frame {
        let binding=(|| {
            let account=account?;
            let identity=owner.document_identity()?;
            let context=owner.current_context().ok()?;
            let binding=Binding { owner:owner.clone(),context,workspace:identity.workspace_id().into(),
                endpoint:endpoint.into(),account };
            binding.current().then_some(binding)
        })();
        let Some(binding)=binding else { self.clear();return Frame::unavailable(control_height) };
        if !self.binding.as_ref().is_some_and(|old|old.same(&binding)) {
            self.clear();
            let Ok(index)=references::index(document) else { return Frame::unavailable(control_height) };
            self.references=index.into_iter().filter_map(|entry|at_path(document,&entry.path))
                .filter(|node|node.kind==NodeKind::HsEmbed)
                .filter_map(|node|Reference::from_block(node).ok()).collect();
            self.binding=Some(binding.clone());
        }
        if let Ok(mut deliveries)=self.deliveries.lock() {
            while let Some((key,status))=deliveries.pop_front() {
                if self.entries.contains_key(&key) {
                    observe_preview(match &status {
                        Status::Ready { .. } => GuiDiagnosticPhase::Completed,
                        Status::Denied => GuiDiagnosticPhase::Rejected,
                        Status::Missing => GuiDiagnosticPhase::Unavailable,
                        _ => GuiDiagnosticPhase::Failed,
                    });
                    self.entries.insert(key,status);
                }
            }
        }
        let entries=self.references.iter().cloned().map(|reference| {
            let status=self.entries.entry(reference.key.clone()).or_insert(Status::Idle).clone();
            (reference.key.clone(),(reference,status))
        }).collect();
        Frame::new(entries,Some(binding.context),control_height)
    }

    pub(crate) fn dispatch_visible(&mut self,frame:&Frame,runtime:Option<&tokio::runtime::Handle>,ctx:&egui::Context) {
        let Some(binding)=self.binding.clone().filter(Binding::current) else { self.clear();return };
        if frame.context!=Some(binding.context) { return; }
        let Some(runtime)=runtime else { return };
        for key in &frame.visible {
            if !matches!(self.entries.get(key),Some(Status::Idle)) { continue; }
            let Some((reference,_))=frame.entries.get(key) else { continue };
            let Ok(permit)=self.slots.clone().try_acquire_owned() else { break };
            let reference=reference.clone();
            let binding=binding.clone();
            let cell=self.deliveries.clone();
            let ctx=ctx.clone();
            self.entries.insert(key.clone(),Status::Loading);
            observe_preview(GuiDiagnosticPhase::Pending);
            runtime.spawn(async move {
                let _permit=permit;
                let delivered=async {
                    if !binding.current() { return None; }
                    let client=KnowledgeDocumentsClient::with_base_url(&binding.endpoint)
                        .with_authenticated_context(binding.account.clone());
                    let headers=HskDocumentHeaders::for_read(&binding.account.session_id,"note-block-preview");
                    let result=client.load_document(&headers,&reference.key.note).await;
                    if !binding.current() { return None; }
                    let status=match result {
                        Ok(response)=>source_preview(&reference,&binding.workspace,&response.document),
                        Err(KnowledgeDocumentsError::NotFound(_))=>Status::Missing,
                        Err(KnowledgeDocumentsError::Forbidden(_))
                        | Err(KnowledgeDocumentsError::UnexpectedStatus { status:401,.. })=>Status::Denied,
                        Err(_)=>Status::Failed,
                    };
                    Some((reference.key,status))
                }.await;
                if let Some(delivered)=delivered {
                    if let Ok(mut queue)=cell.lock() { queue.push_back(delivered); }
                } else { observe_preview(GuiDiagnosticPhase::Stale); }
                drop(_permit);
                ctx.request_repaint();
            });
        }
    }
}

#[cfg(test)]
mod tests;
