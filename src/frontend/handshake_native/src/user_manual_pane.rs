//! Rust-native UserManual reader. The pane consumes the canonical backend
//! `/usermanual/pages` routes and never treats frontend state as manual authority.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use egui::accesskit;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub const SURFACE_AUTHOR_ID: &str = "user-manual.surface";
pub const NAVIGATION_AUTHOR_ID: &str = "user-manual.navigation";
pub const PAGE_AUTHOR_ID: &str = "user-manual.page";
pub const READ_RECEIPT_AUTHOR_ID: &str = "user-manual.status.read-receipt";
pub const RETRY_AUTHOR_ID: &str = "user-manual.action.retry";
pub const LOADING_AUTHOR_ID: &str = "user-manual.status.loading";
pub const UNAVAILABLE_AUTHOR_ID: &str = "user-manual.status.unavailable";
pub const ERROR_AUTHOR_ID: &str = "user-manual.status.error";
pub const SEARCH_INPUT_AUTHOR_ID: &str = "user-manual.search.input";
pub const SEARCH_ACTION_AUTHOR_ID: &str = "user-manual.search.action";
pub const SEARCH_STATUS_AUTHOR_ID: &str = "user-manual.search.status";
pub const SEARCH_RESULTS_AUTHOR_ID: &str = "user-manual.search.results";

pub type UserManualResultCell<T> = Arc<Mutex<Option<Result<T, String>>>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserManualPageSummary {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub page_kind: String,
    #[serde(default)]
    pub audience: String,
    #[serde(default)]
    pub manual_version: String,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserManualNavigation {
    pub manual_version: String,
    pub route_namespace: String,
    #[serde(default)]
    pub pages: Vec<UserManualPageSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserManualPageContent {
    pub page: Value,
    #[serde(default)]
    pub sections: Vec<Value>,
    #[serde(default)]
    pub anchors: Vec<Value>,
    #[serde(default)]
    pub bootstrap_receipt_event_id: String,
    #[serde(default)]
    pub bootstrap_identity_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserManualSearchHit {
    pub result_kind: String,
    pub result_ref: String,
    pub page_slug: Option<String>,
    pub title: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserManualSearchResponse {
    pub query: String,
    pub count: usize,
    #[serde(default)]
    pub results: Vec<UserManualSearchHit>,
}

#[derive(Clone, Default)]
pub struct UserManualPaneController {
    search_focus_requested: Arc<AtomicBool>,
}

impl UserManualPaneController {
    pub fn request_search_focus(&self) {
        self.search_focus_requested.store(true, Ordering::Release);
    }

    fn take_search_focus_request(&self) -> bool {
        self.search_focus_requested.swap(false, Ordering::AcqRel)
    }
}

pub trait UserManualTransport: Send + Sync {
    fn fetch_navigation(&self, cell: UserManualResultCell<UserManualNavigation>);
    fn fetch_page(&self, slug: &str, cell: UserManualResultCell<UserManualPageContent>);
    fn fetch_search(&self, query: &str, cell: UserManualResultCell<UserManualSearchResponse>);
}

#[derive(Clone)]
enum RetryRequest {
    Navigation,
    Page(String),
    Search(String),
}

#[derive(Clone)]
struct UserManualUiError {
    message: String,
    retry: RetryRequest,
}

#[derive(Default)]
struct UserManualUiState {
    navigation: Option<UserManualNavigation>,
    page: Option<UserManualPageContent>,
    selected_slug: Option<String>,
    navigation_pending: bool,
    page_pending: bool,
    search_query: String,
    search: Option<UserManualSearchResponse>,
    search_pending: bool,
    error: Option<UserManualUiError>,
}

pub struct UserManualPaneFactory {
    state: Arc<Mutex<UserManualUiState>>,
    transport: Option<Arc<dyn UserManualTransport>>,
    controller: UserManualPaneController,
    navigation_delivery: UserManualResultCell<UserManualNavigation>,
    page_delivery: UserManualResultCell<UserManualPageContent>,
    search_delivery: UserManualResultCell<UserManualSearchResponse>,
}

impl UserManualPaneFactory {
    pub fn offline() -> Self {
        Self::offline_with_controller(UserManualPaneController::default())
    }

    pub fn offline_with_controller(controller: UserManualPaneController) -> Self {
        Self {
            state: Arc::new(Mutex::new(UserManualUiState::default())),
            transport: None,
            controller,
            navigation_delivery: Arc::new(Mutex::new(None)),
            page_delivery: Arc::new(Mutex::new(None)),
            search_delivery: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_transport(transport: Arc<dyn UserManualTransport>) -> Self {
        Self::with_transport_and_controller(transport, UserManualPaneController::default())
    }

    pub fn with_transport_and_controller(
        transport: Arc<dyn UserManualTransport>,
        controller: UserManualPaneController,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(UserManualUiState::default())),
            transport: Some(transport),
            controller,
            navigation_delivery: Arc::new(Mutex::new(None)),
            page_delivery: Arc::new(Mutex::new(None)),
            search_delivery: Arc::new(Mutex::new(None)),
        }
    }

    fn start_navigation_fetch(&self) {
        let Some(transport) = &self.transport else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            if state.navigation_pending {
                return;
            }
            state.navigation_pending = true;
            state.error = None;
        }
        transport.fetch_navigation(self.navigation_delivery.clone());
    }

    fn start_page_fetch(&self, slug: &str) {
        let Some(transport) = &self.transport else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            if state.page_pending {
                return;
            }
            state.page_pending = true;
            state.page = None;
            state.selected_slug = Some(slug.to_string());
            state.error = None;
        }
        transport.fetch_page(slug, self.page_delivery.clone());
    }

    fn drain_deliveries(&self) {
        let navigation = self
            .navigation_delivery
            .lock()
            .ok()
            .and_then(|mut delivery| delivery.take());
        if let Some(result) = navigation {
            if let Ok(mut state) = self.state.lock() {
                state.navigation_pending = false;
                match result {
                    Ok(navigation) => {
                        state.navigation = Some(navigation);
                        state.error = None;
                    }
                    Err(message) => {
                        state.error = Some(UserManualUiError {
                            message,
                            retry: RetryRequest::Navigation,
                        })
                    }
                }
            }
        }
        let page = self
            .page_delivery
            .lock()
            .ok()
            .and_then(|mut delivery| delivery.take());
        if let Some(result) = page {
            if let Ok(mut state) = self.state.lock() {
                state.page_pending = false;
                match result {
                    Ok(page) => {
                        state.page = Some(page);
                        state.error = None;
                    }
                    Err(message) => {
                        let slug = state.selected_slug.clone().unwrap_or_default();
                        state.error = Some(UserManualUiError {
                            message,
                            retry: RetryRequest::Page(slug),
                        });
                    }
                }
            }
        }
        let search = self
            .search_delivery
            .lock()
            .ok()
            .and_then(|mut delivery| delivery.take());
        if let Some(result) = search {
            if let Ok(mut state) = self.state.lock() {
                state.search_pending = false;
                match result {
                    Ok(search) => {
                        state.search = Some(search);
                        state.error = None;
                    }
                    Err(message) => {
                        let query = state.search_query.clone();
                        state.error = Some(UserManualUiError {
                            message,
                            retry: RetryRequest::Search(query),
                        });
                    }
                }
            }
        }
    }

    fn start_search_fetch(&self, query: &str) {
        let query = query.trim();
        let Some(transport) = &self.transport else {
            return;
        };
        if query.is_empty() {
            if let Ok(mut state) = self.state.lock() {
                state.search = None;
            }
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            if state.search_pending {
                return;
            }
            state.search_pending = true;
            state.search = None;
            state.error = None;
            state.search_query = query.to_string();
        }
        transport.fetch_search(query, self.search_delivery.clone());
    }

    fn retry(&self, request: RetryRequest) {
        match request {
            RetryRequest::Navigation => self.start_navigation_fetch(),
            RetryRequest::Page(slug) => self.start_page_fetch(&slug),
            RetryRequest::Search(query) => self.start_search_fetch(&query),
        }
    }
}

impl PaneFactory for UserManualPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::UserManual
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        self.drain_deliveries();
        let initial_slug = ctx
            .record
            .content_id
            .as_deref()
            .filter(|slug| !slug.trim().is_empty());
        if self.transport.is_some()
            && self.state.lock().ok().is_some_and(|state| {
                state.navigation.is_none() && !state.navigation_pending && state.error.is_none()
            })
        {
            self.start_navigation_fetch();
        }

        let pane_id = ctx.record.pane_id.as_ref();
        tagged_group(
            ui,
            ctx.egui_id.with("user-manual-surface"),
            pane_id,
            SURFACE_AUTHOR_ID,
            "UserManual",
        );
        let Ok(mut state) = self.state.lock() else {
            tagged_label(
                ui,
                pane_id,
                ERROR_AUTHOR_ID,
                "UserManual state lock unavailable",
            );
            return;
        };

        ui.heading("UserManual");
        if self.transport.is_none() {
            tagged_label(
                ui,
                pane_id,
                UNAVAILABLE_AUTHOR_ID,
                "UserManual backend unavailable. Start handshake_core, then retry.",
            );
            return;
        }
        if state.navigation_pending || state.page_pending || state.search_pending {
            tagged_label(
                ui,
                pane_id,
                LOADING_AUTHOR_ID,
                "Loading UserManual content...",
            );
        }
        if let Some(error) = state.error.clone() {
            tagged_label(
                ui,
                pane_id,
                ERROR_AUTHOR_ID,
                &format!("UserManual request failed: {}", error.message),
            );
            let retry = ui.button("Retry");
            tag_button(
                ui,
                &retry,
                pane_id,
                RETRY_AUTHOR_ID,
                "Retry UserManual request",
            );
            if retry.clicked() {
                state.error = None;
                drop(state);
                self.retry(error.retry);
            }
            return;
        }

        let Some(navigation) = state.navigation.clone() else {
            return;
        };
        if state.selected_slug.is_none() {
            state.selected_slug = initial_slug
                .map(str::to_owned)
                .or_else(|| navigation.pages.first().map(|page| page.slug.clone()));
        }
        let selected_slug = state.selected_slug.clone();
        let mut next_slug = None;
        let mut search_requested = false;
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search UserManual")
                    .desired_width(300.0),
            );
            tag_textbox(
                ui,
                &response,
                pane_id,
                SEARCH_INPUT_AUTHOR_ID,
                "Search UserManual",
            );
            if self.controller.take_search_focus_request() {
                response.request_focus();
            }
            let search = ui.button("Search");
            tag_button(
                ui,
                &search,
                pane_id,
                SEARCH_ACTION_AUTHOR_ID,
                "Search UserManual",
            );
            search_requested = search.clicked()
                || (response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));
        });
        if let Some(search) = state.search.as_ref() {
            tagged_label(
                ui,
                pane_id,
                SEARCH_STATUS_AUTHOR_ID,
                &format!(
                    "{} UserManual result(s) for '{}'",
                    search.count, search.query
                ),
            );
            tagged_group(
                ui,
                ctx.egui_id.with("user-manual-search-results"),
                pane_id,
                SEARCH_RESULTS_AUTHOR_ID,
                "UserManual search results",
            );
            egui::ScrollArea::vertical()
                .id_salt(ctx.egui_id.with("user-manual-search-results-scroll"))
                .max_height(180.0)
                .show(ui, |ui| {
                    for hit in &search.results {
                        let response = ui.button(format!("{} — {}", hit.title, hit.excerpt));
                        let author_id = search_hit_author_id(&hit.result_kind, &hit.result_ref);
                        tag_button(ui, &response, pane_id, &author_id, &hit.title);
                        if response.clicked() {
                            if let Some(slug) = hit.page_slug.as_ref() {
                                next_slug = Some(slug.clone());
                            }
                        }
                    }
                });
        }
        ui.columns(2, |columns| {
            tagged_group(
                &mut columns[0],
                ctx.egui_id.with("user-manual-navigation"),
                pane_id,
                NAVIGATION_AUTHOR_ID,
                "UserManual navigation",
            );
            columns[0].small(format!("Version {}", navigation.manual_version));
            egui::ScrollArea::vertical()
                .id_salt(ctx.egui_id.with("user-manual-navigation-scroll"))
                .show(&mut columns[0], |ui| {
                    for page in &navigation.pages {
                        let selected = selected_slug.as_deref() == Some(page.slug.as_str());
                        let response = ui.selectable_label(selected, &page.title);
                        let author_id = page_author_id(&page.slug);
                        tag_button(ui, &response, pane_id, &author_id, &page.title);
                        if response.clicked() && !selected {
                            next_slug = Some(page.slug.clone());
                        }
                    }
                });

            tagged_group(
                &mut columns[1],
                ctx.egui_id.with("user-manual-page"),
                pane_id,
                PAGE_AUTHOR_ID,
                "UserManual page content",
            );
            egui::ScrollArea::vertical()
                .id_salt(ctx.egui_id.with("user-manual-page-scroll"))
                .show(&mut columns[1], |ui| {
                    if let Some(page) = state.page.as_ref() {
                        render_page(ui, pane_id, page);
                    } else if !state.page_pending {
                        ui.label("Select a manual page.");
                    }
                });
        });
        let should_fetch_selected = state.page.is_none() && !state.page_pending;
        let selected_to_fetch = selected_slug.filter(|_| should_fetch_selected);
        let query = search_requested.then(|| state.search_query.clone());
        drop(state);
        if let Some(query) = query {
            self.start_search_fetch(&query);
        }
        if let Some(slug) = next_slug.or(selected_to_fetch) {
            self.start_page_fetch(&slug);
        }
    }
}

fn render_page(ui: &mut egui::Ui, pane_id: &str, content: &UserManualPageContent) {
    let title = string_field(&content.page, &["title", "slug"]).unwrap_or("UserManual page");
    ui.heading(title);
    if let Some(version) = string_field(&content.page, &["manual_version"]) {
        ui.small(format!("Manual version {version}"));
    }
    for section in &content.sections {
        if let Some(heading) = string_field(section, &["heading", "title", "section_key"]) {
            ui.separator();
            ui.strong(heading);
        }
        if let Some(body) = string_field(section, &["body_markdown", "markdown", "body", "content"])
        {
            ui.label(body);
        }
    }
    if !content.bootstrap_receipt_event_id.is_empty() {
        let label = format!("Read receipt: {}", content.bootstrap_receipt_event_id);
        tagged_label(ui, pane_id, READ_RECEIPT_AUTHOR_ID, &label);
    }
}

fn string_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
}

pub fn page_author_id(slug: &str) -> String {
    format!("user-manual.nav.page.{}", sanitize_author_segment(slug))
}

pub fn search_hit_author_id(result_kind: &str, result_ref: &str) -> String {
    format!(
        "user-manual.search.result.{}.{}",
        sanitize_author_segment(result_kind),
        sanitize_author_segment(result_ref)
    )
}

fn sanitize_author_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn scoped_author_id(pane_id: &str, author_id: &str) -> String {
    if pane_id == "pane-a" {
        author_id.to_string()
    } else {
        format!("{pane_id}.{author_id}")
    }
}

fn tagged_group(ui: &mut egui::Ui, id: egui::Id, pane_id: &str, author_id: &str, label: &str) {
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(scoped_author_id(pane_id, author_id));
        node.set_label(label.to_string());
    });
}

fn tagged_label(ui: &mut egui::Ui, pane_id: &str, author_id: &str, label: &str) {
    let response = ui.label(label);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_author_id(scoped_author_id(pane_id, author_id));
        node.set_label(label.to_string());
    });
}

fn tag_button(
    ui: &egui::Ui,
    response: &egui::Response,
    pane_id: &str,
    author_id: &str,
    label: &str,
) {
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::Button);
        node.add_action(accesskit::Action::Click);
        node.set_author_id(scoped_author_id(pane_id, author_id));
        node.set_label(label.to_string());
    });
}

fn tag_textbox(
    ui: &egui::Ui,
    response: &egui::Response,
    pane_id: &str,
    author_id: &str,
    label: &str,
) {
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::TextInput);
        node.add_action(accesskit::Action::Focus);
        node.set_author_id(scoped_author_id(pane_id, author_id));
        node.set_label(label.to_string());
    });
}
