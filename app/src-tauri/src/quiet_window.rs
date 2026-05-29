use std::path::PathBuf;

use tauri::webview::{WebviewWindow, WebviewWindowBuilder};
use tauri::{AppHandle, Runtime, Url, WebviewUrl};

pub struct QuietWindowBuilder<'a, R: Runtime> {
    builder: WebviewWindowBuilder<'a, R, AppHandle<R>>,
}

impl<'a, R: Runtime> QuietWindowBuilder<'a, R> {
    #[allow(clippy::disallowed_methods)]
    pub fn new<L: Into<String>>(app: &'a AppHandle<R>, label: L, url: WebviewUrl) -> Self {
        let builder = WebviewWindowBuilder::new(app, label, url)
            .visible(false)
            .focused(false)
            .focusable(false)
            .skip_taskbar(true)
            .always_on_bottom(true)
            .decorations(false);
        Self { builder }
    }

    pub fn title<S: Into<String>>(self, title: S) -> Self {
        Self {
            builder: self.builder.title(title),
        }
    }

    pub fn data_directory(self, data_directory: PathBuf) -> Self {
        Self {
            builder: self.builder.data_directory(data_directory),
        }
    }

    pub fn on_navigation<F>(self, handler: F) -> Self
    where
        F: Fn(&Url) -> bool + Send + 'static,
    {
        Self {
            builder: self.builder.on_navigation(handler),
        }
    }

    pub fn build(self) -> tauri::Result<WebviewWindow<R>> {
        self.builder.build()
    }
}
