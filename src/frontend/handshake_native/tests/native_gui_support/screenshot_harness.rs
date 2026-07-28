//! Runtime-enforced screenshot harness for native GUI integration tests.
//!
//! Every call to `render()` records one exact, durable outcome. Headless runs do not probe wgpu (the
//! probe can terminate the process on affected Windows adapters); they return a typed DEFERRED result.
//! GPU runs catch Rust panics, save the rendered pixels centrally, validate the saved file, and only
//! then emit CAPTURED. Adapter/device creation may block below Rust, so GPU proof must be launched by
//! `run_mt108_argus_proof.ps1`, which supervises each test process with a hard wall-clock timeout.

#![allow(dead_code)]

use std::ops::{Deref, DerefMut};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "screenshot_marker.rs"]
pub(crate) mod screenshot_marker;

use screenshot_marker::{
    gpu_screenshot_enabled, marker_dir, record_screenshot_outcome, ScreenshotMarker,
};

const MT_ID: &str = "MT-108";
static OUTCOME_ORDINAL: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenshotOutcomeEvidence {
    pub run_id: String,
    pub outcome_id: String,
    pub scenario_id: String,
    pub status: String,
    pub frame_path: Option<String>,
    pub gpu_screenshot_enabled: bool,
}

impl From<ScreenshotMarker> for ScreenshotOutcomeEvidence {
    fn from(marker: ScreenshotMarker) -> Self {
        Self {
            run_id: marker.run_id,
            outcome_id: marker.outcome_id,
            scenario_id: marker.scenario_id,
            status: format!("{:?}", marker.status).to_uppercase(),
            frame_path: marker.frame_path,
            gpu_screenshot_enabled: marker.gpu_screenshot_enabled,
        }
    }
}

pub struct ScreenshotHarness<'a, State = ()> {
    inner: egui_kittest::Harness<'a, State>,
    last_screenshot_outcome: Option<ScreenshotOutcomeEvidence>,
}

impl<State> ScreenshotHarness<'_, State> {
    pub fn builder() -> ScreenshotHarnessBuilder<State> {
        let builder = egui_kittest::HarnessBuilder::default();
        // Render-only sites and explicit `.wgpu()` sites converge here. Never initialize wgpu on a
        // declared headless run; initialize it centrally on a GPU run so render-only sites are covered.
        let builder = if gpu_screenshot_enabled() {
            builder.wgpu()
        } else {
            builder
        };
        ScreenshotHarnessBuilder(builder)
    }

    /// Render through the systemic proof seam. A caller cannot receive pixels without a durable
    /// CAPTURED row whose frame path exists, and cannot receive a headless error without a durable
    /// DEFERRED row. Durable marker failure is returned as a test error (fail closed).
    #[track_caller]
    pub fn render(&mut self) -> Result<image::RgbaImage, String> {
        let caller = std::panic::Location::caller();
        let ordinal = OUTCOME_ORDINAL.fetch_add(1, Ordering::Relaxed);
        let test_name = std::thread::current()
            .name()
            .unwrap_or("unnamed-test")
            .to_owned();
        let source = format!("{}:{}", caller.file(), caller.line());
        let outcome_id = format!(
            "pid{}:{}:{source}:{ordinal}",
            std::process::id(),
            sanitize(&test_name)
        );
        let scenario_id = std::env::var("HANDSHAKE_PROOF_PROCESS_SCENARIO_ID")
            .ok()
            .filter(|scenario| !scenario.trim().is_empty())
            .map(|scenario| format!("matrix:{scenario}"))
            .unwrap_or_else(|| format!("runtime:{test_name}"));

        if !gpu_screenshot_enabled() {
            let marker = record_screenshot_outcome(
                MT_ID,
                &scenario_id,
                &outcome_id,
                Err(format!(
                    "{source}: {0} is unset; render was safely deferred before wgpu initialization",
                    screenshot_marker::GPU_SCREENSHOT_ENV
                )),
            )
            .map_err(|error| format!("durable DEFERRED marker write failed: {error}"))?;
            self.last_screenshot_outcome = Some(marker.into());
            return Err(format!(
                "typed DEFERRED screenshot outcome recorded for {test_name}; set {}=1 on a real-GPU host",
                screenshot_marker::GPU_SCREENSHOT_ENV
            ));
        }

        let rendered = catch_unwind(AssertUnwindSafe(|| self.inner.render()));
        let image = match rendered {
            Ok(Ok(image)) => image,
            Ok(Err(error)) => {
                return self.record_blocked(
                    &scenario_id,
                    &outcome_id,
                    format!("{source}: wgpu render failed: {error}"),
                );
            }
            Err(_) => {
                return self.record_blocked(
                    &scenario_id,
                    &outcome_id,
                    format!("{source}: wgpu render panicked"),
                );
            }
        };

        let frame_dir = marker_dir().join("frames");
        if let Err(error) = std::fs::create_dir_all(&frame_dir) {
            return self.record_blocked(
                &scenario_id,
                &outcome_id,
                format!("create screenshot frame dir failed: {error}"),
            );
        }
        let frame_path = frame_dir.join(format!(
            "{}-pid{}-{ordinal}.png",
            sanitize(&test_name),
            std::process::id()
        ));
        if let Err(error) = image.save(&frame_path) {
            let reason = format!(
                "central screenshot save failed at {}: {error}",
                frame_path.display()
            );
            let marker = write_blocked(&scenario_id, &outcome_id, &reason).map_err(|write| {
                format!("{reason}; durable BLOCKED marker write failed: {write}")
            })?;
            self.last_screenshot_outcome = Some(marker.into());
            return Err(reason);
        }
        let marker = record_screenshot_outcome(
            MT_ID,
            &scenario_id,
            &outcome_id,
            Ok(frame_path.display().to_string()),
        )
        .map_err(|error| format!("durable CAPTURED marker write failed: {error}"))?;
        self.last_screenshot_outcome = Some(marker.into());
        Ok(image)
    }

    /// Require a material frame on a declared GPU run while accepting only a durably recorded
    /// `DEFERRED` outcome on a headless run. This is the canonical matrix call path: marker-write
    /// failures and GPU render failures remain hard test failures instead of being mistaken for an
    /// environment deferral.
    #[track_caller]
    pub fn render_proof_frame(&mut self, expectation: &str) -> Option<image::RgbaImage> {
        let gpu_expected = gpu_screenshot_enabled();
        match self.render() {
            Ok(image) if gpu_expected => Some(image),
            Ok(_) => panic!(
                "{expectation}: render unexpectedly returned pixels while GPU screenshots were disabled"
            ),
            Err(_)
                if !gpu_expected
                    && self
                        .last_screenshot_outcome
                        .as_ref()
                        .is_some_and(|outcome| outcome.status == "DEFERRED") =>
            {
                None
            }
            Err(error) => panic!("{expectation}: {error}"),
        }
    }

    pub fn last_screenshot_outcome(&self) -> Option<&ScreenshotOutcomeEvidence> {
        self.last_screenshot_outcome.as_ref()
    }

    fn record_blocked<T>(
        &mut self,
        scenario_id: &str,
        outcome_id: &str,
        reason: String,
    ) -> Result<T, String> {
        let marker = write_blocked(scenario_id, outcome_id, &reason)
            .map_err(|error| format!("{reason}; durable BLOCKED marker write failed: {error}"))?;
        self.last_screenshot_outcome = Some(marker.into());
        Err(reason)
    }
}

impl<'a> ScreenshotHarness<'a> {
    pub fn new_ui(app: impl FnMut(&mut egui::Ui) + 'a) -> Self {
        Self::builder().build_ui(app)
    }
}

impl<'a, State> Deref for ScreenshotHarness<'a, State> {
    type Target = egui_kittest::Harness<'a, State>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, State> DerefMut for ScreenshotHarness<'a, State> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct ScreenshotHarnessBuilder<State = ()>(egui_kittest::HarnessBuilder<State>);

impl<State> ScreenshotHarnessBuilder<State> {
    pub fn with_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.0 = self.0.with_size(size);
        self
    }

    pub fn with_step_dt(mut self, step_dt: f32) -> Self {
        self.0 = self.0.with_step_dt(step_dt);
        self
    }

    pub fn wgpu(self) -> Self {
        // `ScreenshotHarness::builder` already selects wgpu when explicitly enabled. This retained
        // method keeps existing test builders source-compatible without probing a headless adapter.
        self
    }

    pub fn build_state<'a>(
        self,
        app: impl FnMut(&egui::Context, &mut State) + 'a,
        state: State,
    ) -> ScreenshotHarness<'a, State> {
        ScreenshotHarness {
            inner: self.0.build_state(app, state),
            last_screenshot_outcome: None,
        }
    }

    pub fn build_ui_state<'a>(
        self,
        app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
        state: State,
    ) -> ScreenshotHarness<'a, State> {
        ScreenshotHarness {
            inner: self.0.build_ui_state(app, state),
            last_screenshot_outcome: None,
        }
    }

    pub fn build_eframe<'a>(
        self,
        build: impl FnOnce(&mut eframe::CreationContext<'a>) -> State,
    ) -> ScreenshotHarness<'a, State>
    where
        State: eframe::App,
    {
        ScreenshotHarness {
            inner: self.0.build_eframe(build),
            last_screenshot_outcome: None,
        }
    }
}

impl ScreenshotHarnessBuilder {
    pub fn build_ui<'a>(self, app: impl FnMut(&mut egui::Ui) + 'a) -> ScreenshotHarness<'a> {
        ScreenshotHarness {
            inner: self.0.build_ui(app),
            last_screenshot_outcome: None,
        }
    }
}

fn write_blocked(
    scenario_id: &str,
    outcome_id: &str,
    reason: &str,
) -> std::io::Result<ScreenshotMarker> {
    let marker = ScreenshotMarker::blocked(MT_ID, scenario_id, outcome_id, reason);
    marker.write_jsonl(&marker_dir())?;
    Ok(marker)
}

fn sanitize(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('-').to_owned()
}
