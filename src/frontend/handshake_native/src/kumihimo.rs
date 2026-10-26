//! Native pane/window arrangement adapter. Kumihimo owns plans and projected geometry;
//! Handshake retains content, authentication, public IDs and persisted layout transport.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use kumihimo_content::{Controller, ContentLimits};
use kumihimo_core::{self as core, Command, EntityId, FrameId, WorkspaceId};
use crate::pane_registry::PaneId;
use crate::split_layout::{SplitWeights, DIVIDER_HIT_THICKNESS};
use crate::tab_bar::TabBarState;

type Result<T> = std::result::Result<T, String>;
const CONTEXT_KEY: &str = "handshake-kumihimo-owner";

pub struct PaneProjection {
    pub panes: Vec<(PaneId, egui::Rect)>,
    pub horizontal: Option<egui::Rect>,
    pub vertical: Option<egui::Rect>,
}

struct Window {
    identity:handshake_layout_snapshot::KumihimoWindowId,
    root: WorkspaceId,
    retained: WorkspaceId,
    frames: BTreeMap<PaneId, FrameId>,
    active: Vec<PaneId>,
    imported: Option<SplitWeights>,
    chrome: kumihimo_egui::chrome::ChromeState,
    input: kumihimo_egui::input::InputState,
    epoch: u64,
}

#[derive(Debug)]
pub struct CleanupObservation {
    pub controller_retired: bool,
    pub logical_window_bindings_retired: usize,
    pub view_bindings_retired: usize,
    pub native_window_slots: Option<usize>,
    pub sdk_capture_registry_entries: Option<usize>,
    pub os_windows_destroyed: Option<bool>,
}

pub struct NativeHost {
    controller: Option<Controller>,
    windows: BTreeMap<egui::ViewportId, Window>,
    operation: u64,
    scope: Option<String>,
    provider: core::ProviderId,
    resources: BTreeMap<String, core::ResourceId>,
    views: BTreeMap<(egui::ViewportId, PaneId, String), core::ViewId>,
    // Settings is an existing ephemeral native window, not a persisted PaneRegistry entity.
    // Its distinct Core root reuses this adapter and is retired with the actual host owner.
    settings_surface: Option<Box<NativeHost>>,
}

impl NativeHost {
    pub fn new() -> Result<Self> {
        let session = uuid::Uuid::new_v4().to_string();
        let core = core::Core::new(core::Identity::new("handshake-native", &session)
            .map_err(|error| error.to_string())?, Default::default(), 256).map_err(|error| error.to_string())?;
        let mut controller = Controller::new(core, &session, ContentLimits::default()).map_err(|error| error.to_string())?;
        let response = controller.execute(core::Envelope {
            identity: controller.snapshot().identity.clone(), actor: "handshake-native-host".into(),
            operation: core::OperationId::new(1), expected_generation: controller.snapshot().generation,
            command: Command::RegisterProvider { key: "handshake-pane-factories".into(), supports_shared_views: true },
        }).map_err(|error| error.to_string())?;
        let core::Change::Created(EntityId::Provider(provider)) = response.outcome.result.map_err(|error| error.to_string())?
            else { return Err("Kumihimo provider identity was not returned".into()); };
        observe(HostEvent::Initialized);
        Ok(Self { controller: Some(controller), windows: BTreeMap::new(), operation: 1,
            scope: None, provider, resources: BTreeMap::new(), views: BTreeMap::new(),settings_surface:None })
    }

    pub fn rebind_scope(&mut self, scope: String) -> Result<()> {
        if self.scope.as_ref() == Some(&scope) { return Ok(()); }
        self.shutdown();
        let mut next = Self::new()?;
        next.scope = Some(scope);
        *self = next;
        Ok(())
    }

    fn command(&mut self, command: Command) -> Result<core::Change> {
        let controller = self.controller.as_mut().ok_or("Kumihimo owner has shut down")?;
        let operation = self.operation.checked_add(1).ok_or("Kumihimo operation sequence exhausted")?;
        let response = controller.execute(core::Envelope {
            identity: controller.snapshot().identity.clone(), actor: "handshake-native-host".into(),
            operation: core::OperationId::new(operation), expected_generation: controller.snapshot().generation, command,
        }).map_err(|error| error.to_string())?;
        self.operation = operation;
        response.outcome.result.map_err(|error| error.to_string())
    }

    fn workspace(&mut self) -> Result<WorkspaceId> {
        match self.command(Command::CreateWorkspace { owner: None })? {
            core::Change::Created(EntityId::Workspace(id)) => Ok(id),
            _ => Err("Kumihimo workspace identity was not returned".into()),
        }
    }

    fn reconcile_window(&mut self, viewport: egui::ViewportId, panes: &[PaneId]) -> Result<()> {
        if !self.windows.contains_key(&viewport) {
            let root = self.workspace()?;
            let retained = self.workspace()?;
            let identity=if viewport==egui::ViewportId::ROOT { handshake_layout_snapshot::KumihimoWindowId::Root }
                else if let [pane_id]=panes { handshake_layout_snapshot::KumihimoWindowId::PopOut { pane_id:pane_id.clone() } }
                else { return Err("Native window has no stable host content identity".into()); };
            self.windows.insert(viewport, Window { identity,root, retained, frames: BTreeMap::new(), active: vec![], imported: None,
                chrome: Default::default(), input: Default::default(),
                // Logical GUI incarnation before an actual SDK target is available; no native
                // readback, window lease or renderer ownership is inferred from this local fence.
                epoch: 1 });
        }
        let root = self.windows[&viewport].root;
        let retained = self.windows[&viewport].retained;
        let removed: Vec<_> = self.windows[&viewport].active.iter().filter(|pane| !panes.contains(pane)).cloned().collect();
        for pane in removed {
            let frame = self.windows[&viewport].frames[&pane];
            let count = self.controller.as_ref().unwrap().snapshot().workspaces[&retained].frames.len();
            self.command(Command::MoveFrame { frame, destination: retained, index: count })?;
        }
        for (index, pane) in panes.iter().enumerate() {
            match self.windows[&viewport].frames.get(pane).copied() {
                Some(frame) if self.controller.as_ref().unwrap().snapshot().frames[&frame].workspace != root => {
                    self.command(Command::MoveFrame { frame, destination: root, index })?;
                }
                Some(_) => (),
                None => {
                    let core::Change::Created(EntityId::Frame(frame)) = self.command(Command::CreateFrame { workspace: root, index })?
                        else { return Err("Kumihimo frame identity was not returned".into()); };
                    self.windows.get_mut(&viewport).unwrap().frames.insert(pane.clone(), frame);
                }
            }
        }
        if self.windows[&viewport].active != panes {
            let window = self.windows.get_mut(&viewport).unwrap();
            let was_empty=window.active.is_empty();
            window.active = panes.to_vec();
            if was_empty { window.imported = None; }
        }
        Ok(())
    }

    fn legacy_shape(&self,viewport:egui::ViewportId)->Option<LegacyShape> {
        let window=self.windows.get(&viewport)?;
        let snapshot=self.controller.as_ref()?.snapshot();
        if snapshot.maximized.contains_key(&window.root)
            || window.active.iter().any(|pane|snapshot.floats.contains_key(&window.frames[pane])) {return None;}
        legacy_shape(snapshot.layouts.get(&window.root)?)
    }

    fn import_weights(&mut self,viewport:egui::ViewportId,weights:SplitWeights)->Result<()> {
        if self.windows[&viewport].imported.is_none() || self.legacy_shape(viewport).is_some() {
            self.resize(viewport,weights)
        } else {
            // An unrepresentable legacy hint cannot replace a canonical dock tree.
            self.windows.get_mut(&viewport).unwrap().imported=Some(weights);Ok(())
        }
    }

    /// Stable legacy controls adjust only an eligible canonical root, preserving every node/id.
    pub fn resize(&mut self, viewport: egui::ViewportId, weights: SplitWeights) -> Result<()> {
        let window = self.windows.get(&viewport).ok_or("Kumihimo window is not mounted")?;
        if window.imported == Some(weights) { return Ok(()); }
        let root=window.root;
        let plan=if window.imported.is_none() {
            plan(&window.active.iter().map(|pane|window.frames[pane]).collect::<Vec<_>>(),weights)?
        } else {
            let shape=self.legacy_shape(viewport).ok_or("Legacy divider does not represent this canonical dock layout")?;
            let mut plan=self.controller.as_ref().unwrap().snapshot().layouts[&root].clone();
            update_legacy_weights(&mut plan,shape,weights)?;plan
        };
        observe(HostEvent::LayoutPending);
        if let Err(error)=self.command(Command::SetLayout { workspace: root, plan }) {
            observe(HostEvent::LayoutFailed);return Err(error);
        }
        self.windows.get_mut(&viewport).unwrap().imported = Some(weights);
        observe(HostEvent::Layout);Ok(())
    }

    pub fn project(&mut self, viewport: egui::ViewportId, panes: &[PaneId], area: egui::Rect,
                   weights: SplitWeights) -> Result<PaneProjection> {
        self.reconcile_window(viewport, panes)?;
        self.import_weights(viewport, weights)?;
        let root = self.windows[&viewport].root;
        let mut context = core::GeometryContext::new(core::Rect::new(area.left() as f64, area.top() as f64,
            area.width() as f64, area.height() as f64).map_err(|error| error.to_string())?);
        context.allow_floating = true;
        context.allowed_docks.push(core::DockLocation::Append { workspace: root });
        for pane in panes {
            let frame = self.windows[&viewport].frames[pane];
            context.minimums.insert(frame,core::Size::new(80.0,48.0).map_err(|error|error.to_string())?);
            for edge in [core::Edge::Top,core::Edge::Bottom,core::Edge::Left,core::Edge::Right] {
                context.allowed_docks.push(core::DockLocation::Beside { frame,edge });
            }
        }
        // Policy changes keep the previous viewport until the asset performs its checked
        // viewport transition, including canonical floating-frame moves.
        if let Some(previous)=self.controller.as_ref().unwrap().snapshot().contexts.get(&root) {
            context.viewport=previous.viewport;
        }
        if self.controller.as_ref().unwrap().snapshot().contexts.get(&root) != Some(&context) {
            self.command(Command::SetGeometryContext { workspace: root, context })?;
        }
        let window=&self.windows[&viewport];
        let surface=kumihimo_egui::SurfaceBinding {root,viewport,epoch:window.epoch,
            bounds:kumihimo_egui::from_egui(area).map_err(|error|error.to_string())?,pixels_per_point:1.0};
        let mut no_paint=|_:&mut egui::Ui,_:&PaneId,_:egui::Rect|{};
        let mut bridge=PaintBridge {operation:&mut self.operation,frames:BTreeMap::new(),paint:&mut no_paint,
            policy:kumihimo_egui::HostPolicy {settings:kumihimo_egui::AdapterSettings {
                profile:kumihimo_egui::ChromeProfile::Hidden,..Default::default()},..Default::default()}};
        let projected=kumihimo_egui::workspace::project_with_chrome(self.controller.as_mut().unwrap(),surface,&mut bridge,&window.chrome)
            .map_err(|error|error.to_string())?;
        if let Some(error)=projected.rejected.first() {observe(HostEvent::Rejected);return Err(error.clone());}
        let geometry = self.controller.as_ref().unwrap().snapshot().geometry(root).map_err(|error| error.to_string())?;
        let snapshot=self.controller.as_ref().unwrap().snapshot();
        let window=&self.windows[&viewport];
        let mut slots=Vec::new();
        for pane in panes {
            let frame=window.frames[pane];
            if snapshot.frames.get(&frame).is_none_or(|record|record.workspace!=root) {
                return Err("Kumihimo admitted pane has an invalid frame binding".into());
            }
            if let Some(rect)=geometry.frames.get(&frame) {slots.push((pane.clone(),kumihimo_egui::to_egui(*rect)));}
            else if !snapshot.maximized.get(&root).is_some_and(|visible|*visible!=frame) {
                return Err("Kumihimo omitted a visible admitted pane".into());
            }
        }
        let (horizontal,vertical)=self.legacy_shape(viewport)
            .map(|shape|legacy_hits(&snapshot.layouts[&root],&geometry,shape)).transpose()?.unwrap_or((None,None));
        Ok(PaneProjection { panes: slots, horizontal, vertical })
    }

    /// Bind real tab metadata/order to Kumihimo views after the owning host accepts a tab action.
    pub fn reconcile_tabs(&mut self, viewport: egui::ViewportId, bars: &HashMap<PaneId, TabBarState>) -> Result<()> {
        let Some(window) = self.windows.get(&viewport) else { return Ok(()); };
        let active = window.active.clone();
        let mut wanted = Vec::new();
        for pane in active {
            let frame = self.windows[&viewport].frames[&pane];
            if let Some(bar) = bars.get(&pane) {
                for (index, tab) in bar.tabs.iter().enumerate() {
                    let key = serde_json::to_string(&(&tab.pane_type, &tab.content_id)).map_err(|error| error.to_string())?;
                    wanted.push((pane.clone(), frame, index, key, tab.clone(), index == bar.active_index));
                }
            }
        }
        let desired: BTreeSet<_> = wanted.iter().map(|(pane,_,_,key,_,_)| (viewport,pane.clone(),key.clone())).collect();
        for (pane, frame, index, key, tab, selected) in wanted {
            let occurrence = (viewport, pane, key.clone());
            let resource = if let Some(resource) = self.resources.get(&key) { *resource } else {
                let core::Change::Created(EntityId::Resource(resource)) = self.command(Command::RegisterResource {
                    provider: self.provider, key: key.clone(), restore_reference: key.clone() })?
                    else { return Err("Kumihimo resource identity was not returned".into()); };
                self.resources.insert(key, resource); resource
            };
            let view = if let Some(view) = self.views.get(&occurrence) { *view } else {
                let movable = self.views.iter().find(|(old, view)| old.0 == viewport && !desired.contains(*old)
                    && self.controller.as_ref().unwrap().snapshot().views.get(*view).is_some_and(|record|record.resource == resource))
                    .map(|(old, view)| (old.clone(), *view));
                if let Some((old, view)) = movable {
                    self.views.remove(&old); self.views.insert(occurrence.clone(), view); view
                } else {
                    let core::Change::Created(EntityId::View(view)) = self.command(Command::CreateView { frame, resource, index })?
                        else { return Err("Kumihimo view identity was not returned".into()); };
                    self.views.insert(occurrence.clone(), view); view
                }
            };
            let current = self.controller.as_ref().unwrap().snapshot().views[&view].clone();
            if current.frame != frame || self.controller.as_ref().unwrap().snapshot().frames[&frame].views.get(index) != Some(&view) {
                self.command(Command::MoveView { view, destination: frame, index })?;
            }
            if current.label != tab.label() || current.pinned != tab.pinned || current.unsaved != tab.dirty {
                self.command(Command::SetViewMetadata { view, label: tab.label(), pinned: tab.pinned, unsaved: tab.dirty })?;
            }
            if selected && self.controller.as_ref().unwrap().snapshot().frames[&frame].selected_view != Some(view) {
                self.command(Command::SelectView { view })?;
            }
        }
        let removed: Vec<_> = self.views.keys().filter(|key| key.0 == viewport && !desired.contains(*key)).cloned().collect();
        for key in removed {
            let view = self.views[&key];
            // This follows an already accepted host close (including its dirty/pin confirmation).
            self.command(Command::SetViewMetadata { view, label: self.controller.as_ref().unwrap().snapshot().views[&view].label.clone(), pinned: false, unsaved: false })?;
            self.command(Command::CloseView { view, approved: true })?;
            self.views.remove(&key);
        }
        Ok(())
    }

    pub fn dock(&mut self, viewport: egui::ViewportId, pane: &PaneId, target: &PaneId, edge: core::Edge) -> Result<()> {
        let window = self.windows.get(&viewport).ok_or("Kumihimo window is not mounted")?;
        let frame = *window.frames.get(pane).ok_or("Kumihimo source pane is missing")?;
        let target = *window.frames.get(target).ok_or("Kumihimo target pane is missing")?;
        let intent = core::DockIntent::Frame { frame, location: core::DockLocation::Beside { frame: target, edge } };
        let preview = self.controller.as_ref().ok_or("Kumihimo owner has shut down")?.core().preview(intent).map_err(|error| error.to_string())?;
        self.command(Command::CommitPreview(preview))?;
        observe(HostEvent::Interaction);
        Ok(())
    }

    /// Adopt accepted asset selection only when the current native occurrence/order still matches.
    pub fn apply_asset_selection(&self,viewport:egui::ViewportId,bars:&mut HashMap<PaneId,TabBarState>)->Result<()> {
        let window=self.windows.get(&viewport).ok_or("Kumihimo selection window is not mounted")?;
        let snapshot=self.controller.as_ref().ok_or("Kumihimo owner has shut down")?.snapshot();
        let mut changes=Vec::new();
        for pane in &window.active {
            let Some(bar)=bars.get(pane) else {return Err("Native tab owner disappeared during asset input".into());};
            let frame=&snapshot.frames[&window.frames[pane]];
            let mut expected=Vec::new();
            for tab in &bar.tabs {
                let key=serde_json::to_string(&(&tab.pane_type,&tab.content_id)).map_err(|error|error.to_string())?;
                let view=*self.views.get(&(viewport,pane.clone(),key)).ok_or("Native tab occurrence changed during asset input")?;
                let record=snapshot.views.get(&view).ok_or("Selected asset view disappeared")?;
                if record.closed.is_some() || record.frame!=frame.id || record.label!=tab.label()
                    || record.pinned!=tab.pinned || record.unsaved!=tab.dirty {return Err("Native tab mapping is stale".into());}
                expected.push(view);
            }
            if expected!=frame.views {return Err("Native tab order changed during asset input".into());}
            if let Some(selected)=frame.selected_view {
                let index=expected.iter().position(|view|*view==selected).ok_or("Selected view has no current native occurrence")?;
                changes.push((pane.clone(),index));
            }
        }
        for (pane,index) in changes {bars.get_mut(&pane).unwrap().activate(index);}
        Ok(())
    }

    pub fn synchronize_snapshot(&mut self,panes:&[PaneId],bars:&HashMap<PaneId,TabBarState>,weights:SplitWeights)->Result<()> {
        if !self.windows.contains_key(&egui::ViewportId::ROOT) { return Ok(()); }
        self.reconcile_window(egui::ViewportId::ROOT,panes)?;
        self.import_weights(egui::ViewportId::ROOT,weights)?;
        let viewports:Vec<_>=self.windows.iter().filter(|(_,window)|!window.active.is_empty()).map(|(id,_)|*id).collect();
        for viewport in viewports {self.reconcile_tabs(viewport,bars)?;}
        Ok(())
    }

    pub fn capture(&self)->Result<Option<handshake_layout_snapshot::KumihimoLayout>> {
        use handshake_layout_snapshot::{KumihimoLayout,KumihimoWindow,KumihimoView};
        let controller=self.controller.as_ref().ok_or("Kumihimo owner has shut down")?;
        if self.windows.is_empty() { return Ok(None); }
        let saved=kumihimo_layout_json::encode_core(controller.core(),Default::default()).map_err(|error|error.to_string())?;
        let windows=self.windows.values().map(|window|KumihimoWindow { window:window.identity.clone(),root:window.root,
            retained:window.retained,frames:window.frames.clone(),active:window.active.clone() }).collect();
        let views=self.views.iter().map(|((viewport,pane_id,resource_key),view)| {
            let window=self.windows.get(viewport).ok_or("Kumihimo view window has no logical binding")?;
            Ok(KumihimoView {window:window.identity.clone(),pane_id:pane_id.clone(),resource_key:resource_key.clone(),view:*view})
        }).collect::<Result<_>>()?;
        if !saved.is_current(controller.core()) { return Err("Kumihimo changed during layout capture".into()); }
        let captured=KumihimoLayout { model_json:String::from_utf8(saved.bytes().to_vec()).map_err(|error|error.to_string())?,windows,views };
        captured.restore_core().map_err(|error|error.to_string())?;
        Ok(Some(captured))
    }

    pub fn from_saved(saved:&handshake_layout_snapshot::KumihimoLayout,scope:String,weights:SplitWeights)->Result<Self> {
        use handshake_layout_snapshot::KumihimoWindowId;
        let core=saved.restore_core().map_err(|error|error.to_string())?;
        let provider=core.snapshot().providers.values().find(|provider|provider.key=="handshake-pane-factories")
            .map(|provider|provider.id).ok_or("Kumihimo host provider was not restored")?;
        let resources=core.snapshot().resources.values().map(|resource|(resource.key.clone(),resource.id)).collect();
        let session=core.snapshot().identity.incarnation.clone();
        let controller=Controller::new(core,session,ContentLimits::default()).map_err(|error|error.to_string())?;
        let viewport=|id:&KumihimoWindowId|match id {
            KumihimoWindowId::Root=>egui::ViewportId::ROOT,
            KumihimoWindowId::PopOut {pane_id}=>egui::ViewportId::from_hash_of(pane_id.as_ref()),
        };
        let windows=saved.windows.iter().map(|window|(viewport(&window.window),Window {identity:window.window.clone(),
            root:window.root,retained:window.retained,frames:window.frames.clone(),active:window.active.clone(),
            imported:Some(weights),chrome:Default::default(),input:Default::default(),epoch:1})).collect();
        let views=saved.views.iter().map(|view|((viewport(&view.window),view.pane_id.clone(),view.resource_key.clone()),view.view)).collect();
        Ok(Self {controller:Some(controller),windows,operation:0,scope:Some(scope),provider,resources,views,settings_surface:None})
    }

    pub fn viewport_ids(&self) -> Vec<egui::ViewportId> {
        let mut ids:Vec<_>=self.windows.iter().filter(|(_,window)|!window.active.is_empty()).map(|(id,_)|*id).collect();
        if self.settings_surface.is_some() {ids.push(settings_viewport());}ids
    }

    pub fn retire_viewport(&mut self, viewport: egui::ViewportId) -> Result<()> {
        if !self.windows.contains_key(&viewport) { return Ok(()); }
        self.reconcile_window(viewport,&[])?;
        if let Some(window)=self.windows.get_mut(&viewport) {
            window.input.cancel("native viewport removed by host");
            window.chrome=Default::default();
        }
        observe(HostEvent::WindowRetired);
        Ok(())
    }

    pub fn bind_observed_surface(&mut self, viewport: egui::ViewportId, epoch: u64) -> Result<()> {
        if epoch == 0 { return Err("Kumihimo rejected a zero observed surface incarnation".into()); }
        if viewport==settings_viewport() {
            if let Some(settings)=self.settings_surface.as_mut() {return settings.bind_observed_surface(egui::ViewportId::ROOT,epoch);}
        }
        if let Some(window) = self.windows.get_mut(&viewport) {
            if window.epoch != epoch { window.input.cancel("native surface incarnation changed"); }
            window.epoch = epoch;
        }
        Ok(())
    }

    pub fn render(&mut self, ui: &mut egui::Ui, panes: &[PaneId], weights: &mut SplitWeights,
                  palette: &crate::theme::HsPalette,
                  mut paint: impl FnMut(&mut egui::Ui, &PaneId, egui::Rect)) -> Result<PaneProjection> {
        let viewport=ui.ctx().viewport_id();
        self.render_at(viewport,viewport,true,ui,panes,weights,palette,&mut paint)
    }

    fn render_at(&mut self,viewport:egui::ViewportId,binding_viewport:egui::ViewportId,interactive:bool,
        ui:&mut egui::Ui,panes:&[PaneId],weights:&mut SplitWeights,palette:&crate::theme::HsPalette,
        mut paint:impl FnMut(&mut egui::Ui,&PaneId,egui::Rect))->Result<PaneProjection> {
        let area = ui.available_rect_before_wrap();
        let projection = self.project(viewport, panes, area, *weights)?;
        let window = self.windows.get_mut(&viewport).ok_or("Kumihimo window disappeared")?;
        let surface = kumihimo_egui::SurfaceBinding { root: window.root, viewport:binding_viewport, epoch: window.epoch,
            bounds: kumihimo_egui::from_egui(area).map_err(|error| error.to_string())?,
            pixels_per_point: ui.ctx().pixels_per_point() };
        let frame_panes: BTreeMap<_,_> = window.frames.iter().map(|(pane,frame)| (*frame,pane.clone())).collect();
        let mut chrome = std::mem::take(&mut window.chrome);
        let mut input = std::mem::take(&mut window.input);
        let mut controller = self.controller.take().ok_or("Kumihimo owner has shut down")?;
        let generation = controller.snapshot().generation;
        let mut bridge = PaintBridge { operation: &mut self.operation, frames: frame_panes,
            paint: &mut paint, policy: kumihimo_egui::HostPolicy {
                theme_tokens: Some(kumihimo_egui::chrome::ThemeTokens {
                    preview_stroke: palette.accent, resize_handle_stroke: palette.accent,
                    divider_stroke: palette.border, rail_fill: palette.surface, rail_thumb_fill: palette.accent_soft,
                }),
                settings: kumihimo_egui::AdapterSettings { profile: kumihimo_egui::ChromeProfile::Hidden,
                    ..Default::default() }, ..Default::default()
            } };
        let result = (|| -> Result<()> {
            let projected = kumihimo_egui::workspace::project_with_chrome(&mut controller, surface, &mut bridge, &chrome)
                .map_err(|error| error.to_string())?;
            let mut report = kumihimo_egui::FrameReport { generation: controller.snapshot().generation,
                epoch: surface.epoch, control_limit: bridge.policy.max_regions * bridge.policy.tab_page_size,
                ..Default::default() };
            kumihimo_egui::chrome::draw(ui.ctx(), &mut controller, &projected, surface, &mut bridge, &mut chrome, &mut report);
            kumihimo_egui::content::draw(ui.ctx(), &mut controller, &projected, surface, &mut bridge, &mut report);
            for frame in &projected.frames {
                if controller.snapshot().frames[&frame.frame].selected_view.is_none() {
                    let pane = bridge.frames.get(&frame.frame).ok_or("Kumihimo empty pane binding missing")?;
                    let rect = kumihimo_egui::to_egui(frame.usable);
                    let mut child = ui.new_child(egui::UiBuilder::new().id_salt(("kum-empty",frame.frame.raw()))
                        .max_rect(rect).layer_id(frame.layer()));
                    child.set_clip_rect(kumihimo_egui::to_egui(frame.clip));
                    (bridge.paint)(&mut child,pane,rect);
                }
            }
            // A host-provided grip retains the existing header widgets while using the asset's
            // canonical frame-drag operation and shared resize/docking implementation.
            if interactive {for frame in &projected.frames {
                let rect = kumihimo_egui::to_egui(frame.bounds);
                let grip = egui::Rect::from_min_size(rect.min, egui::vec2(12.0, crate::pane_header::PANE_HEADER_HEIGHT));
                let id = format!("frame/{}/drag", frame.frame.raw());
                let grip_ui = egui::Ui::new(ui.ctx().clone(), egui::Id::new((viewport,&id)),
                    egui::UiBuilder::new().layer_id(frame.layer()).max_rect(grip));
                let response = grip_ui.interact(grip, egui::Id::new(&id), egui::Sense::click_and_drag());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Move pane"));
                ui.ctx().accesskit_node_builder(response.id, |node| { node.set_author_id(id.clone()); node.set_label("Move pane"); });
                grip_ui.painter().rect_filled(grip,0.0,palette.surface);
                grip_ui.painter().text(grip.center(), egui::Align2::CENTER_CENTER, "⠿",
                    egui::FontId::proportional(11.0), palette.text_subtle);
                report.controls.push(kumihimo_egui::ControlRecord { id, label:"Move pane".into(),
                    bounds:kumihimo_egui::from_egui(grip).map_err(|error|error.to_string())?,target:None,enabled:true });
            }
            }
            if interactive {kumihimo_egui::input::process(ui.ctx(), &mut controller, &projected, surface, &mut bridge, &mut input, &mut report);}
            if let Some(error) = projected.rejected.first().or_else(||report.rejected.first()) { return Err(error.clone()); }
            Ok(())
        })();
        self.controller = Some(controller);
        let window = self.windows.get_mut(&viewport).unwrap();
        window.chrome = chrome; window.input = input;
        if self.controller.as_ref().unwrap().snapshot().generation != generation {
            observe(HostEvent::Interaction);
            ui.ctx().request_repaint();
        }
        // Export only axes actually represented by the current canonical topology.
        if let Some(shape)=self.legacy_shape(viewport) {
            let window=&self.windows[&viewport];
            export_legacy_weights(&self.controller.as_ref().unwrap().snapshot().layouts[&window.root],shape,weights);
        }
        self.windows.get_mut(&viewport).unwrap().imported = Some(*weights);
        if result.is_err() {observe(HostEvent::Rejected);}
        result?;
        Ok(projection)
    }

    /// Mount the real Settings body in its own ephemeral canonical root. No native pane record,
    /// document, Settings open flag, draft or credential is added to the workbench snapshot.
    pub fn render_settings(&mut self,ui:&mut egui::Ui,palette:&crate::theme::HsPalette,
        mut paint:impl FnMut(&mut egui::Ui))->Result<()> {
        if self.controller.is_none() {return Err("Kumihimo owner has shut down".into());}
        if self.settings_surface.is_none() {
            let mut owner=Self::new()?;owner.scope=self.scope.clone();self.settings_surface=Some(Box::new(owner));
        }
        let owner=self.settings_surface.as_mut().unwrap();
        let id:PaneId=Arc::from(crate::settings_dialog::SETTINGS_POPOUT_KEY);
        let mut weights=SplitWeights::default();
        owner.project(egui::ViewportId::ROOT,&[id.clone()],ui.available_rect_before_wrap(),weights)?;
        let occurrence=(egui::ViewportId::ROOT,id.clone(),crate::settings_dialog::SETTINGS_POPOUT_KEY.to_owned());
        if !owner.views.contains_key(&occurrence) {
            let core::Change::Created(EntityId::Resource(resource))=owner.command(Command::RegisterResource {
                provider:owner.provider,key:occurrence.2.clone(),restore_reference:occurrence.2.clone()})?
                else{return Err("Settings content resource identity was not returned".into());};
            let frame=owner.windows[&egui::ViewportId::ROOT].frames[&id];
            let core::Change::Created(EntityId::View(view))=owner.command(Command::CreateView {frame,resource,index:0})?
                else{return Err("Settings content view identity was not returned".into());};
            owner.resources.insert(occurrence.2.clone(),resource);owner.views.insert(occurrence,view);
            owner.command(Command::SetViewMetadata {view,label:crate::settings_dialog::SETTINGS_WINDOW_LABEL.into(),
                pinned:false,unsaved:false})?;
            owner.command(Command::SelectView {view})?;
        }
        owner.render_at(egui::ViewportId::ROOT,settings_viewport(),false,ui,&[id],&mut weights,palette,
            |ui,_,_|paint(ui))?;
        Ok(())
    }

    pub fn retire_settings(&mut self) {
        if let Some(mut settings)=self.settings_surface.take() {settings.shutdown();observe(HostEvent::WindowRetired);}
    }

    pub fn shutdown(&mut self)->CleanupObservation {
        let settings=self.settings_surface.take().map(|mut owner|owner.shutdown());
        let result=CleanupObservation {controller_retired:self.controller.take().is_some()
                ||settings.as_ref().is_some_and(|result|result.controller_retired),
            logical_window_bindings_retired:self.windows.len()+settings.as_ref().map_or(0,|result|result.logical_window_bindings_retired),
            view_bindings_retired:self.views.len()+settings.as_ref().map_or(0,|result|result.view_bindings_retired),
            native_window_slots:None,sdk_capture_registry_entries:None,os_windows_destroyed:None};
        self.windows.clear();self.views.clear();self.resources.clear();self.scope=None;
        if result.controller_retired {observe(HostEvent::Shutdown);}
        result
    }
}

struct PaintBridge<'a,F> {
    operation: &'a mut u64,
    frames: BTreeMap<FrameId,PaneId>,
    paint: &'a mut F,
    policy: kumihimo_egui::HostPolicy,
}
impl<F: FnMut(&mut egui::Ui,&PaneId,egui::Rect)> kumihimo_egui::DesktopHost for PaintBridge<'_,F> {
    fn policy(&self)->kumihimo_egui::HostPolicy { self.policy.clone() }
    fn dispatch(&mut self,controller:&mut Controller,command:Command)->kumihimo_egui::Result<core::Response> {
        let operation=self.operation.checked_add(1).ok_or(kumihimo_egui::AdapterError::Budget("host operation sequence"))?;
        let response=controller.execute(core::Envelope { identity:controller.snapshot().identity.clone(),
            actor:"handshake-native-host".into(),operation:core::OperationId::new(operation),
            expected_generation:controller.snapshot().generation,command })?;
        *self.operation=operation;
        Ok(response)
    }
    fn paint_content(&mut self,ui:&mut egui::Ui,slot:&kumihimo_egui::RenderSlot,_:&mut Controller)->kumihimo_egui::Result<()> {
        let pane=self.frames.get(&slot.frame).ok_or(kumihimo_egui::AdapterError::Stale("host pane frame"))?;
        (self.paint)(ui,pane,kumihimo_egui::to_egui(slot.bounds));
        Ok(())
    }
    fn content_input(&mut self,_:&mut Controller,_:&kumihimo_egui::RenderSlot,_:kumihimo_content::Input)->kumihimo_egui::Result<()> {
        // The existing egui editors consumed the original Context input during paint_content.
        // Replaying the adapter's serialized notification would duplicate native editor commands.
        Ok(())
    }
}

#[derive(Clone,Copy)]
enum LegacyShape { Split {axis:core::Axis,count:usize}, Grid, PrimaryAndStack {stack:core::NodeId} }
fn legacy_shape(plan:&core::LayoutPlan)->Option<LegacyShape> {
    let root=plan.root?;
    let leaf=|node:&core::NodeId|matches!(plan.nodes.get(node),Some(core::LayoutNode::Leaf(core::LayoutItem::Frame(_))));
    match plan.nodes.get(&root)? {
        core::LayoutNode::Split {axis,children} if (children.len()==2||children.len()==3)
            &&children.iter().all(|child|leaf(&child.node))=>Some(LegacyShape::Split {axis:*axis,count:children.len()}),
        core::LayoutNode::Split {axis:core::Axis::Horizontal,children} if children.len()==2&&leaf(&children[0].node)=>{
            let stack=children[1].node;
            match plan.nodes.get(&stack)? {
                core::LayoutNode::Split {axis:core::Axis::Vertical,children} if children.len()==2
                    &&children.iter().all(|child|leaf(&child.node))=>Some(LegacyShape::PrimaryAndStack {stack}),
                _=>None,
            }
        }
        core::LayoutNode::Grid {rows,columns,cells} if rows.len()==2&&columns.len()==2&&cells.len()==4
            &&cells.iter().all(|cell|leaf(&cell.node))=>Some(LegacyShape::Grid),
        _=>None,
    }
}
fn update_legacy_weights(plan:&mut core::LayoutPlan,shape:LegacyShape,weights:SplitWeights)->Result<()> {
    let scalar=|value|core::Scalar::new(value).map_err(|error|error.to_string());
    if let LegacyShape::PrimaryAndStack {stack}=shape {
        for (node,axis,fraction) in [(plan.root.ok_or("Canonical root is missing")?,core::Axis::Horizontal,weights.vertical),
            (stack,core::Axis::Vertical,weights.horizontal)] {
            match plan.nodes.get_mut(&node) {
                Some(core::LayoutNode::Split {axis:actual,children}) if *actual==axis&&children.len()==2=>{
                    children[0].weight=scalar(fraction as f64)?;
                    children[1].weight=scalar(1.0-fraction as f64)?;
                }
                _=>return Err("Canonical topology changed before legacy resize".into()),
            }
        }
        return Ok(());
    }
    match (plan.nodes.get_mut(&plan.root.ok_or("Canonical root is missing")?).ok_or("Canonical root node is missing")?,shape) {
        (core::LayoutNode::Split {children,..},LegacyShape::Split {axis,count})=>{
            let fraction=match axis {core::Axis::Horizontal=>weights.vertical,core::Axis::Vertical=>weights.horizontal} as f64;
            let total:f64=children[..count-1].iter().map(|child|child.weight.get()).sum();
            for child in &mut children[..count-1] {child.weight=scalar(fraction*child.weight.get()/total)?;}
            children[count-1].weight=scalar(1.0-fraction)?;
        }
        (core::LayoutNode::Grid {rows,columns,..},LegacyShape::Grid)=>{
            rows[0]=scalar(weights.horizontal as f64)?;rows[1]=scalar(1.0-weights.horizontal as f64)?;
            columns[0]=scalar(weights.vertical as f64)?;columns[1]=scalar(1.0-weights.vertical as f64)?;
        }
        _=>return Err("Canonical topology changed before legacy resize".into()),
    }
    Ok(())
}
fn export_legacy_weights(plan:&core::LayoutPlan,shape:LegacyShape,weights:&mut SplitWeights) {
    match (&plan.nodes[&plan.root.unwrap()],shape) {
        (core::LayoutNode::Split {children,..},LegacyShape::Split {axis,count})=>{
            let total:f64=children.iter().map(|child|child.weight.get()).sum();
            let first:f64=children[..count-1].iter().map(|child|child.weight.get()).sum();
            match axis {core::Axis::Horizontal=>weights.vertical=(first/total) as f32,
                core::Axis::Vertical=>weights.horizontal=(first/total) as f32}
        }
        (core::LayoutNode::Grid {rows,columns,..},LegacyShape::Grid)=>{
            weights.horizontal=(rows[0].get()/(rows[0].get()+rows[1].get())) as f32;
            weights.vertical=(columns[0].get()/(columns[0].get()+columns[1].get())) as f32;
        }
        (core::LayoutNode::Split {children,..},LegacyShape::PrimaryAndStack {stack})=>{
            weights.vertical=(children[0].weight.get()/(children[0].weight.get()+children[1].weight.get())) as f32;
            if let core::LayoutNode::Split {children,..}=&plan.nodes[&stack] {
                weights.horizontal=(children[0].weight.get()/(children[0].weight.get()+children[1].weight.get())) as f32;
            }
        }
        _=>(),
    }
}
fn legacy_hits(plan:&core::LayoutPlan,geometry:&core::GeometryOutput,shape:LegacyShape)
    ->Result<(Option<egui::Rect>,Option<egui::Rect>)> {
    let root=plan.root.ok_or("Canonical divider root is missing")?;
    let area=kumihimo_egui::to_egui(*geometry.nodes.get(&root).ok_or("Canonical divider bounds are missing")?);
    let half=DIVIDER_HIT_THICKNESS/2.0;
    let horizontal=|y|egui::Rect::from_min_max(egui::pos2(area.left(),y-half),egui::pos2(area.right(),y+half));
    let vertical=|x|egui::Rect::from_min_max(egui::pos2(x-half,area.top()),egui::pos2(x+half,area.bottom()));
    match (&plan.nodes[&root],shape) {
        (core::LayoutNode::Split {children,..},LegacyShape::Split {axis,count})=>{
            let before=geometry.nodes.get(&children[count-2].node).ok_or("Canonical split boundary is missing")?;
            let after=geometry.nodes.get(&children[count-1].node).ok_or("Canonical split boundary is missing")?;
            Ok(match axis {core::Axis::Horizontal=>(None,Some(vertical(((before.right()+after.x())/2.0) as f32))),
                core::Axis::Vertical=>(Some(horizontal(((before.bottom()+after.y())/2.0) as f32)),None)})
        }
        (core::LayoutNode::Grid {..},LegacyShape::Grid)=>{
            let first=geometry.cells.get(&(root,0,0)).ok_or("Canonical grid cell is missing")?;
            let right=geometry.cells.get(&(root,0,1)).ok_or("Canonical grid cell is missing")?;
            let bottom=geometry.cells.get(&(root,1,0)).ok_or("Canonical grid cell is missing")?;
            Ok((Some(horizontal(((first.bottom()+bottom.y())/2.0) as f32)),
                Some(vertical(((first.right()+right.x())/2.0) as f32))))
        }
        (core::LayoutNode::Split {children,..},LegacyShape::PrimaryAndStack {stack})=>{
            let left=geometry.nodes.get(&children[0].node).ok_or("Canonical primary pane bounds are missing")?;
            let right=geometry.nodes.get(&stack).ok_or("Canonical stacked pane bounds are missing")?;
            let core::LayoutNode::Split {children,..}=&plan.nodes[&stack]
                else {return Err("Canonical stacked divider topology changed".into());};
            let upper=geometry.nodes.get(&children[0].node).ok_or("Canonical upper pane bounds are missing")?;
            let lower=geometry.nodes.get(&children[1].node).ok_or("Canonical lower pane bounds are missing")?;
            let y=((upper.bottom()+lower.y())/2.0) as f32;
            let stacked_horizontal=egui::Rect::from_min_max(egui::pos2(right.x() as f32,y-half),
                egui::pos2(right.right() as f32,y+half));
            Ok((Some(stacked_horizontal),Some(vertical(((left.right()+right.x())/2.0) as f32))))
        }
        _=>Err("Canonical divider topology changed".into()),
    }
}

fn plan(frames: &[FrameId], weights: SplitWeights) -> Result<core::LayoutPlan> {
    let mut nodes = BTreeMap::new();
    for (index, frame) in frames.iter().enumerate() {
        nodes.insert(core::NodeId(index as u64 + 1), core::LayoutNode::Leaf(core::LayoutItem::Frame(*frame)));
    }
    if frames.len() <= 1 { return Ok(core::LayoutPlan { root: (!frames.is_empty()).then_some(core::NodeId(1)), nodes }); }
    let root = core::NodeId(frames.len() as u64 + 1);
    let v = weights.vertical as f64;
    let h = weights.horizontal as f64;
    let scalar = |value| core::Scalar::new(value).map_err(|error| error.to_string());
    let layout = if frames.len() == 3 {
        let stack=core::NodeId(5);
        nodes.insert(stack,core::LayoutNode::Split {axis:core::Axis::Vertical,children:vec![
            core::WeightedNode {node:core::NodeId(2),weight:scalar(h)?},
            core::WeightedNode {node:core::NodeId(3),weight:scalar(1.0-h)?},
        ]});
        core::LayoutNode::Split {axis:core::Axis::Horizontal,children:vec![
            core::WeightedNode {node:core::NodeId(1),weight:scalar(v)?},
            core::WeightedNode {node:stack,weight:scalar(1.0-v)?},
        ]}
    } else if frames.len() == 4 {
        core::LayoutNode::Grid { rows: vec![scalar(h)?,scalar(1.0-h)?], columns: vec![scalar(v)?,scalar(1.0-v)?],
            cells: (0..4).map(|index| core::Cell { row:index/2, column:index%2, node:core::NodeId(index as u64+1) }).collect() }
    } else {
        let shares = if frames.len()==2 { vec![v,1.0-v] } else { vec![1.0;frames.len()] };
        core::LayoutNode::Split { axis:core::Axis::Horizontal, children: shares.into_iter().enumerate()
            .map(|(index,weight)| Ok(core::WeightedNode { node:core::NodeId(index as u64+1), weight:scalar(weight)? }))
            .collect::<Result<_>>()? }
    };
    nodes.insert(root,layout);
    Ok(core::LayoutPlan { root:Some(root), nodes })
}

pub fn settings_viewport()->egui::ViewportId {
    egui::ViewportId::from_hash_of(crate::settings_dialog::SETTINGS_POPOUT_KEY)
}

pub fn owner(ctx: &egui::Context) -> Result<Arc<Mutex<NativeHost>>> {
    let id = egui::Id::new(CONTEXT_KEY);
    if let Some(owner) = ctx.data(|data| data.get_temp::<Arc<Mutex<NativeHost>>>(id)) { return Ok(owner); }
    let owner = Arc::new(Mutex::new(NativeHost::new()?));
    ctx.data_mut(|data| data.insert_temp(id,owner.clone()));
    Ok(owner)
}

pub fn install(ctx: &egui::Context) -> Result<Arc<Mutex<NativeHost>>> { owner(ctx) }

pub fn bind_context_owner(ctx:&egui::Context,owner:Arc<Mutex<NativeHost>>) {
    ctx.data_mut(|data|data.insert_temp(egui::Id::new(CONTEXT_KEY),owner));
}


#[derive(Clone,Copy)]
#[repr(u64)]
enum HostEvent { Initialized=1, Layout=2, Interaction=3, Shutdown=4, WindowRetired=5, Rejected=6, LayoutPending=7, LayoutFailed=8 }
fn observe(event: HostEvent) {
    use crate::diagnostics::gui_events::{GuiDiagnosticOperation as Operation,GuiDiagnosticPhase as Phase};
    let (operation,phase)=match event {
        HostEvent::Initialized=>(Operation::KumihimoOwner,Phase::Accepted),
        HostEvent::Layout=>(Operation::KumihimoLayout,Phase::Completed),
        HostEvent::Interaction=>(Operation::KumihimoInteraction,Phase::Completed),
        HostEvent::Shutdown=>(Operation::KumihimoCleanup,Phase::Completed),
        HostEvent::WindowRetired=>(Operation::KumihimoWindowRetired,Phase::Completed),
        HostEvent::Rejected=>(Operation::KumihimoInteraction,Phase::Rejected),
        HostEvent::LayoutPending=>(Operation::KumihimoLayout,Phase::Pending),
        HostEvent::LayoutFailed=>(Operation::KumihimoLayout,Phase::Failed),
    };
    crate::diagnostics::gui_events::record_gui_observation(operation,phase);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_registry::PaneType;
    use crate::tab_bar::TabState;
    fn area()->egui::Rect { egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1000.0,600.0)) }
    #[test]
    fn native_panes_use_kumihimo_geometry_and_keep_live_identity() {
        let mut owner=NativeHost::new().unwrap();
        let panes=vec![Arc::from("pane-a"),Arc::from("pane-b"),Arc::from("pane-c")];
        let first=owner.project(egui::ViewportId::ROOT,&panes,area(),SplitWeights::default()).unwrap();
        assert_eq!(first.panes.len(),3);
        assert_eq!(first.panes[0].1.right(),first.panes[1].1.left());
        assert_eq!(first.panes[0].1.right(),first.panes[2].1.left());
        assert_eq!(first.panes[1].1.bottom(),first.panes[2].1.top());
        assert_eq!(first.panes[0].1.top(),area().top());
        assert_eq!(first.panes[0].1.bottom(),area().bottom());
        assert_eq!(first.panes[1].1.top(),area().top());
        assert_eq!(first.panes[2].1.bottom(),area().bottom());
        assert_eq!(first.panes[1].1.right(),area().right());
        assert_eq!(first.panes[2].1.right(),area().right());
        assert!(first.horizontal.unwrap().left()>=first.panes[1].1.left());
        let frame=owner.windows[&egui::ViewportId::ROOT].frames[&panes[0]];
        owner.resize(egui::ViewportId::ROOT,SplitWeights{vertical:0.7,horizontal:0.65}).unwrap();
        let resized=owner.project(egui::ViewportId::ROOT,&panes,area(),SplitWeights{vertical:0.7,horizontal:0.65}).unwrap();
        assert!(resized.panes[2].1.width()<first.panes[2].1.width());
        assert!(resized.panes[1].1.height()>first.panes[1].1.height());
        assert_eq!(resized.panes[0].1.height(),area().height());
        assert_eq!(resized.panes[1].1.bottom(),resized.panes[2].1.top());
        assert_eq!(owner.windows[&egui::ViewportId::ROOT].frames[&panes[0]],frame);
    }
    #[test]
    fn accepted_tab_transfer_moves_the_existing_kumihimo_view() {
        let mut owner=NativeHost::new().unwrap();
        let a:PaneId=Arc::from("pane-a");let b:PaneId=Arc::from("pane-b");
        owner.project(egui::ViewportId::ROOT,&[a.clone(),b.clone()],area(),SplitWeights::default()).unwrap();
        let mut tab=TabState::new(PaneType::Workspace);tab.content_id=Some("ordinary-document".into());
        let mut bars=HashMap::from([(a.clone(),TabBarState::new(a.clone(),vec![tab])),(b.clone(),TabBarState::new(b.clone(),vec![]))]);
        owner.reconcile_tabs(egui::ViewportId::ROOT,&bars).unwrap();
        let view=*owner.views.values().next().unwrap();
        let moved=bars.get_mut(&a).unwrap().take_tab(0).unwrap();
        bars.get_mut(&b).unwrap().insert_tab(moved);
        owner.reconcile_tabs(egui::ViewportId::ROOT,&bars).unwrap();
        assert_eq!(owner.views.len(),1);
        assert_eq!(*owner.views.values().next().unwrap(),view);
        assert_eq!(owner.controller.as_ref().unwrap().snapshot().views[&view].frame,
            owner.windows[&egui::ViewportId::ROOT].frames[&b]);
    }
    #[test]
    fn docking_remains_canonical_until_a_new_host_layout_command() {
        let mut owner=NativeHost::new().unwrap();
        let panes=vec![Arc::from("pane-a"),Arc::from("pane-b")];
        owner.project(egui::ViewportId::ROOT,&panes,area(),SplitWeights::default()).unwrap();
        owner.dock(egui::ViewportId::ROOT,&panes[0],&panes[1],core::Edge::Bottom).unwrap();
        let root=owner.windows[&egui::ViewportId::ROOT].root;
        let docked=owner.controller.as_ref().unwrap().snapshot().layouts[&root].clone();
        let after=owner.project(egui::ViewportId::ROOT,&panes,area(),SplitWeights::default()).unwrap();
        assert_eq!(owner.controller.as_ref().unwrap().snapshot().layouts[&root],docked);
        assert_eq!(after.panes[0].1.width(),area().width());
        assert!(after.panes[0].1.top()>=after.panes[1].1.bottom());
    }
    #[test]
    fn account_workspace_rebind_retires_old_content_and_interactions() {
        let mut owner=NativeHost::new().unwrap();owner.rebind_scope("account-a/workspace-a".into()).unwrap();
        owner.project(egui::ViewportId::ROOT,&[Arc::from("pane-a")],area(),SplitWeights::default()).unwrap();
        let identity=owner.controller.as_ref().unwrap().snapshot().identity.clone();
        owner.rebind_scope("account-b/workspace-b".into()).unwrap();
        assert_ne!(owner.controller.as_ref().unwrap().snapshot().identity,identity);
        assert!(owner.windows.is_empty());assert!(owner.views.is_empty());assert!(owner.resources.is_empty());
        owner.shutdown();assert!(owner.controller.is_none());
    }
    #[test]
    fn canonical_dock_plan_and_tab_identity_survive_asset_codec_restore() {
        let mut owner=NativeHost::new().unwrap();
        let a:PaneId=Arc::from("pane-a");let b:PaneId=Arc::from("pane-b");
        let panes=vec![a.clone(),b.clone()];
        owner.project(egui::ViewportId::ROOT,&panes,area(),SplitWeights::default()).unwrap();
        let mut note=TabState::new(PaneType::Workspace);note.content_id=Some("ordinary-note".into());
        note.pinned=true;note.dirty=true;
        let bars=HashMap::from([(a.clone(),TabBarState::new(a.clone(),vec![note])),
            (b.clone(),TabBarState::new(b.clone(),vec![]))]);
        owner.reconcile_tabs(egui::ViewportId::ROOT,&bars).unwrap();
        owner.dock(egui::ViewportId::ROOT,&a,&b,core::Edge::Bottom).unwrap();
        let original=owner.controller.as_ref().unwrap().snapshot();
        let root=owner.windows[&egui::ViewportId::ROOT].root;
        let before_plan=original.layouts[&root].clone();
        let before_views=original.views.clone();let before_identity=original.identity.clone();
        let saved=owner.capture().unwrap().unwrap();
        let mut restored=NativeHost::from_saved(&saved,"same-account/workspace".into(),SplitWeights::default()).unwrap();
        let projected=restored.project(egui::ViewportId::ROOT,&panes,area(),SplitWeights::default()).unwrap();
        let after=restored.controller.as_ref().unwrap().snapshot();
        assert_ne!(after.identity.incarnation,before_identity.incarnation);
        assert_eq!(after.identity.lineage,before_identity.lineage);
        assert_eq!(after.layouts[&root],before_plan);
        assert_eq!(after.views,before_views);
        assert!(projected.panes[0].1.top()>=projected.panes[1].1.bottom());
        let mut corrupt=saved.clone();corrupt.windows[0].frames.insert(a,core::FrameId::from_raw(999_999));
        assert!(NativeHost::from_saved(&corrupt,"same-account/workspace".into(),SplitWeights::default()).is_err());
        assert_eq!(owner.controller.as_ref().unwrap().snapshot().layouts[&root],before_plan);
    }

    #[test]
    fn cleanup_reports_only_retired_host_state_and_keeps_native_counts_unknown() {
        let mut owner=NativeHost::new().unwrap();
        owner.project(egui::ViewportId::ROOT,&[Arc::from("pane-a")],area(),SplitWeights::default()).unwrap();
        let cleanup=owner.shutdown();
        assert!(cleanup.controller_retired);assert_eq!(cleanup.logical_window_bindings_retired,1);
        assert_eq!(cleanup.native_window_slots,None);assert_eq!(cleanup.sdk_capture_registry_entries,None);
        assert_eq!(cleanup.os_windows_destroyed,None);
        assert!(owner.capture().is_err());assert!(!owner.shutdown().controller_retired);
    }

    #[test]
    fn asset_viewport_transition_keeps_floating_pane_reachable_after_resize() {
        let mut owner=NativeHost::new().unwrap();let pane:PaneId=Arc::from("pane-a");
        owner.project(egui::ViewportId::ROOT,&[pane.clone()],area(),SplitWeights::default()).unwrap();
        let frame=owner.windows[&egui::ViewportId::ROOT].frames[&pane];
        let preview=owner.controller.as_ref().unwrap().core().preview(core::DockIntent::Float {
            frame,rect:core::Rect::new(700.0,300.0,250.0,250.0).unwrap()}).unwrap();
        owner.command(Command::CommitPreview(preview)).unwrap();
        let smaller=egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(600.0,400.0));
        let projection=owner.project(egui::ViewportId::ROOT,&[pane.clone()],smaller,SplitWeights::default()).unwrap();
        assert_eq!(owner.windows[&egui::ViewportId::ROOT].frames[&pane],frame);
        assert!(smaller.contains_rect(projection.panes[0].1));
        assert!(owner.capture().unwrap().is_some());
    }

    fn render_input(owner:&mut NativeHost,ctx:&egui::Context,panes:&[PaneId],weights:&mut SplitWeights,
        key:Option<egui::Key>,mut paint:impl FnMut(&mut egui::Ui,&PaneId,egui::Rect)) {
        let events=key.map(|key|vec![egui::Event::Key {key,physical_key:Some(key),pressed:false,repeat:false,modifiers:egui::Modifiers::CTRL},
            egui::Event::Key {key,physical_key:Some(key),pressed:true,repeat:false,modifiers:egui::Modifiers::CTRL}]).unwrap_or_default();
        let _=ctx.run(egui::RawInput {screen_rect:Some(area()),focused:true,modifiers:egui::Modifiers::CTRL,
            events,..Default::default()},|ctx| {egui::CentralPanel::default().show(ctx,|ui| {
                owner.render(ui,panes,weights,&crate::theme::HsPalette::dark(),&mut paint).unwrap();
            });});
    }

    #[test]
    fn maximize_keeps_hidden_owners_and_live_input_can_restore() {
        let mut owner=NativeHost::new().unwrap();let ctx=egui::Context::default();
        let a:PaneId=Arc::from("pane-a");let b:PaneId=Arc::from("pane-b");let panes=vec![a.clone(),b.clone()];
        let mut weights=SplitWeights::default();
        owner.project(egui::ViewportId::ROOT,&panes,area(),weights).unwrap();
        let bars=HashMap::from([(a.clone(),TabBarState::new(a.clone(),vec![TabState::new(PaneType::Workspace)])),
            (b.clone(),TabBarState::new(b.clone(),vec![TabState::new(PaneType::Problems)]))]);
        owner.reconcile_tabs(egui::ViewportId::ROOT,&bars).unwrap();
        let root=owner.windows[&egui::ViewportId::ROOT].root;
        let frames=owner.windows[&egui::ViewportId::ROOT].frames.clone();let views=owner.views.clone();
        owner.windows.get_mut(&egui::ViewportId::ROOT).unwrap().input.focused_view=Some(*owner.views.values().next().unwrap());
        render_input(&mut owner,&ctx,&panes,&mut weights,Some(egui::Key::F10),|_,_,_|{});
        assert!(owner.controller.as_ref().unwrap().snapshot().maximized.contains_key(&root));
        let mut visible=Vec::new();
        render_input(&mut owner,&ctx,&panes,&mut weights,None,|_,pane,_|visible.push(pane.clone()));
        assert_eq!(visible.len(),1);assert_eq!(owner.windows[&egui::ViewportId::ROOT].frames,frames);assert_eq!(owner.views,views);
        render_input(&mut owner,&ctx,&panes,&mut weights,Some(egui::Key::F10),|_,_,_|{});
        assert!(!owner.controller.as_ref().unwrap().snapshot().maximized.contains_key(&root));
        assert_eq!(owner.project(egui::ViewportId::ROOT,&panes,area(),weights).unwrap().panes.len(),2);
        assert!(!ctx.input_mut(|input|input.consume_key(egui::Modifiers::CTRL,egui::Key::F10)));
    }

    #[test]
    fn asset_ctrl_tab_reaches_the_native_tab_and_next_factory_content() {
        let mut owner=NativeHost::new().unwrap();let ctx=egui::Context::default();
        let pane:PaneId=Arc::from("pane-a");let panes=vec![pane.clone()];let mut weights=SplitWeights::default();
        owner.project(egui::ViewportId::ROOT,&panes,area(),weights).unwrap();
        let mut bars=HashMap::from([(pane.clone(),TabBarState::new(pane.clone(),
            vec![TabState::new(PaneType::Workspace),TabState::new(PaneType::Problems)]))]);
        owner.reconcile_tabs(egui::ViewportId::ROOT,&bars).unwrap();
        render_input(&mut owner,&ctx,&panes,&mut weights,Some(egui::Key::Tab),|_,_,_|{});
        owner.apply_asset_selection(egui::ViewportId::ROOT,&mut bars).unwrap();
        assert_eq!(bars[&pane].active_index,1);
        owner.reconcile_tabs(egui::ViewportId::ROOT,&bars).unwrap();
        let mut factories=Vec::new();
        render_input(&mut owner,&ctx,&panes,&mut weights,None,|_,id,_|factories.push(bars[id].active().unwrap().pane_type.clone()));
        assert_eq!(factories,vec![PaneType::Problems]);
        let frame=owner.windows[&egui::ViewportId::ROOT].frames[&pane];
        let selected=owner.controller.as_ref().unwrap().snapshot().frames[&frame].selected_view.unwrap();
        assert_eq!(owner.controller.as_ref().unwrap().snapshot().views[&selected].label,bars[&pane].active().unwrap().label());
        assert!(!ctx.input_mut(|input|input.consume_key(egui::Modifiers::CTRL,egui::Key::Tab)));
    }

    #[test]
    fn arbitrary_nested_dock_plan_cannot_be_reset_by_a_legacy_fraction() {
        let mut owner=NativeHost::new().unwrap();let panes:Vec<PaneId>=vec![Arc::from("pane-a"),Arc::from("pane-b"),Arc::from("pane-c")];
        let weights=SplitWeights::default();owner.project(egui::ViewportId::ROOT,&panes,area(),weights).unwrap();
        owner.dock(egui::ViewportId::ROOT,&panes[0],&panes[1],core::Edge::Bottom).unwrap();
        let root=owner.windows[&egui::ViewportId::ROOT].root;let before=owner.controller.as_ref().unwrap().snapshot().layouts[&root].clone();
        let changed=SplitWeights {vertical:0.7,horizontal:0.7};
        assert!(owner.resize(egui::ViewportId::ROOT,changed).is_err());
        let projected=owner.project(egui::ViewportId::ROOT,&panes,area(),changed).unwrap();
        assert!(projected.horizontal.is_none()&&projected.vertical.is_none());
        assert_eq!(owner.controller.as_ref().unwrap().snapshot().layouts[&root],before);
    }

    #[test]
    fn detached_settings_has_real_content_identity_and_retires_without_persisted_panes() {
        let mut owner=NativeHost::new().unwrap();let ctx=egui::Context::default();
        let mut painted=0;
        let _=ctx.run(egui::RawInput {screen_rect:Some(area()),..Default::default()},|ctx| {
            egui::CentralPanel::default().show(ctx,|ui|owner.render_settings(ui,&crate::theme::HsPalette::dark(),|ui| {
                painted+=1;ui.label("Settings body owned by the existing settings dialog");
            }).unwrap());
        });
        assert_eq!(painted,1);
        assert!(owner.windows.is_empty()&&owner.views.is_empty()&&owner.resources.is_empty());
        assert!(owner.capture().unwrap().is_none());
        let settings=owner.settings_surface.as_ref().unwrap();
        let identity=settings.controller.as_ref().unwrap().snapshot().identity.clone();
        let view=*settings.views.values().next().unwrap();
        assert_eq!(settings.controller.as_ref().unwrap().snapshot().views[&view].label,crate::settings_dialog::SETTINGS_WINDOW_LABEL);
        assert!(owner.viewport_ids().contains(&settings_viewport()));
        owner.bind_observed_surface(settings_viewport(),7).unwrap();
        assert_eq!(owner.settings_surface.as_ref().unwrap().windows[&egui::ViewportId::ROOT].epoch,7);
        let _=ctx.run(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(800.0,500.0))),
            ..Default::default()},|ctx| {egui::CentralPanel::default().show(ctx,|ui|owner.render_settings(ui,&crate::theme::HsPalette::dark(),|_|{}).unwrap());});
        let settings=owner.settings_surface.as_ref().unwrap();
        assert_eq!(*settings.views.values().next().unwrap(),view);
        assert_eq!(settings.controller.as_ref().unwrap().snapshot().identity,identity);
        owner.retire_settings();assert!(owner.settings_surface.is_none());
        assert!(!owner.viewport_ids().contains(&settings_viewport()));
        let _=ctx.run(egui::RawInput {screen_rect:Some(area()),..Default::default()},|ctx| {
            egui::CentralPanel::default().show(ctx,|ui|owner.render_settings(ui,&crate::theme::HsPalette::dark(),|_|{}).unwrap());
        });
        assert_ne!(owner.settings_surface.as_ref().unwrap().controller.as_ref().unwrap().snapshot().identity,identity);
        let cleanup=owner.shutdown();assert_eq!(cleanup.logical_window_bindings_retired,1);assert_eq!(cleanup.view_bindings_retired,1);
        assert!(owner.settings_surface.is_none());assert_eq!(cleanup.os_windows_destroyed,None);
    }

}
