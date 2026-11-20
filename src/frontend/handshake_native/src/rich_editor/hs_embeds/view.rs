use super::*;
use std::collections::HashMap;
use crate::rich_editor::document_owner::EditorSaveContext;
use crate::theme::HsPalette;

pub(crate) enum Action { Open(Reference),Remove(BlockNode) }
struct Painted { reference:Option<Reference>,node:BlockNode,occurrence:String,rect:egui::Rect,clip:egui::Rect,controls:egui::Rect,ready:bool }

pub(crate) struct Frame {
    pub(super) entries:HashMap<Key,(Reference,Status)>,
    pub(super) visible:Vec<Key>,
    pub(crate) context:Option<EditorSaveContext>,
    control_height:f32,
    painted:Vec<Painted>,
}
impl Frame {
    pub(super) fn new(entries:HashMap<Key,(Reference,Status)>,context:Option<EditorSaveContext>,control_height:f32)->Self {
        Self { entries,context,control_height,visible:Vec::new(),painted:Vec::new() }
    }
    pub(crate) fn unavailable(control_height:f32)->Self { Self::new(HashMap::new(),None,control_height) }

    pub(crate) fn paint(&mut self,painter:&egui::Painter,node:&BlockNode,origin:egui::Pos2,
        width:f32,palette:&HsPalette,font:f32)->f32 {
        let reference=Reference::from_block(node).ok();
        let entry=reference.as_ref().and_then(|reference|self.entries.get(&reference.key));
        let label=reference.as_ref().map(|reference|reference.label.as_str()).filter(|label|!label.is_empty())
            .unwrap_or("Note block");
        let caption=match entry.map(|(_,status)|status) {
            Some(Status::Ready { source_version,.. })=>format!("{label} (version {source_version})"),
            _=>label.to_owned(),
        };
        let body=match entry.map(|(_,status)|status) {
            Some(Status::Ready { text,.. }) if text.is_empty()=>"The selected block has no text preview",
            Some(Status::Ready { text,.. })=>text.as_str(),
            Some(Status::Idle)=>"Preview not loaded",
            Some(Status::Loading)=>"Loading preview...",
            Some(Status::Missing)=>"Referenced block or note is missing",
            Some(Status::Denied)=>"Preview access denied",
            Some(Status::Failed)=>"Preview could not be loaded",
            None=>"Embed identity or account is unavailable",
        };
        // Reserve the bounded preview rows before delivery so asynchronous text cannot move later targets.
        let row_height=painter.fonts_mut(|fonts|fonts.row_height(&egui::FontId::proportional(font)));
        let shape=|text:String,rows:usize,color| {
            let mut job=egui::text::LayoutJob::simple(text,egui::FontId::proportional(font),color,(width-16.0).max(1.0));
            job.wrap.max_rows=rows;
            for section in &mut job.sections { section.format.line_height=Some(row_height); }
            painter.layout_job(job)
        };
        let title=shape(caption,1,palette.text_subtle);
        let preview=shape(body.to_owned(),3,palette.text);
        let controls_y=origin.y+8.0+row_height+4.0+3.0*row_height+4.0;
        let height=controls_y-origin.y+self.control_height+8.0;
        let rect=egui::Rect::from_min_size(origin,egui::vec2(width.max(1.0),height));
        let clipped=painter.with_clip_rect(rect);
        clipped.rect_filled(rect,6.0,palette.surface);
        clipped.rect_stroke(rect,6.0,egui::Stroke::new(1.0,palette.border),egui::StrokeKind::Inside);
        clipped.galley(origin+egui::vec2(8.0,8.0),title.clone(),palette.text_subtle);
        let preview_origin=origin+egui::vec2(8.0,12.0+row_height);
        clipped.with_clip_rect(egui::Rect::from_min_size(preview_origin,
            egui::vec2((width-16.0).max(1.0),3.0*row_height))).galley(preview_origin,preview,palette.text);
        if rect.intersects(painter.clip_rect()) {
            if let Some((reference,_))=entry {
                if !self.visible.contains(&reference.key) { self.visible.push(reference.key.clone()); }
            }
            if let Some(occurrence) = node.attrs.get("block_id").and_then(|v|v.as_str())
                .filter(|id| !id.trim().is_empty() && self.context.is_some()) {
                self.painted.push(Painted { reference:reference.clone(),node:node.clone(),occurrence:occurrence.into(),
                    rect,clip:rect.intersect(painter.clip_rect()),
                    controls:egui::Rect::from_min_size(egui::pos2(origin.x+8.0,controls_y),egui::vec2((width-16.0).max(1.0),self.control_height)),
                    ready:entry.is_some_and(|(_,status)|matches!(status,Status::Ready { .. })) });
            }
        }
        height+crate::rich_editor::renderer::line_layout::BLOCK_GAP_PTS
    }

    pub(crate) fn interact(&mut self,ui:&mut egui::Ui,read_only:bool)->Vec<Action> {
        let mut actions=Vec::new();
        for painted in self.painted.drain(..) {
            let mut child=ui.new_child(egui::UiBuilder::new().max_rect(painted.rect)
                .id_salt(("note-block-embed",painted.occurrence.clone())));
            child.set_clip_rect(painted.clip);
            let width=((painted.controls.width()-4.0)*0.5).max(1.0);
            let open_rect=egui::Rect::from_min_size(painted.controls.min,egui::vec2(width,painted.controls.height()));
            let remove_rect=egui::Rect::from_min_size(egui::pos2(open_rect.max.x+4.0,open_rect.min.y),open_rect.size());
            let open=child.add_enabled_ui(painted.ready,|ui|ui.put(open_rect,egui::Button::new("Open").truncate())).inner;
            let remove=child.add_enabled_ui(!read_only,|ui|ui.put(remove_rect,egui::Button::new("Remove embed").truncate())).inner;
            for (id,role,suffix) in [(child.unique_id(),egui::accesskit::Role::Group,""),
                (open.id,egui::accesskit::Role::Button,"-open"),(remove.id,egui::accesskit::Role::Button,"-remove")] {
                let author=crate::rich_editor::scoped_author_id(format!("note-block-embed-{}{suffix}",painted.occurrence));
                child.ctx().accesskit_node_builder(id,|node|{ node.set_role(role);node.set_author_id(author); });
            }
            if open.clicked() { if let Some(reference)=painted.reference { actions.push(Action::Open(reference)); } }
            if remove.clicked() { actions.push(Action::Remove(painted.node)); }
        }
        actions
    }
}

#[cfg(test)]
mod tests;
