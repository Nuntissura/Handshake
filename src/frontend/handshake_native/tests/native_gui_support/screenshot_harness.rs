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
const PROOF_MT_ID_ENV: &str = "HANDSHAKE_PROOF_MT_ID";
static OUTCOME_ORDINAL: AtomicU64 = AtomicU64::new(1);

fn default_proof_mt_id() -> String {
    std::env::var(PROOF_MT_ID_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| MT_ID.to_owned())
}

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
    proof_mt_id: String,
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
        ScreenshotHarnessBuilder {
            inner: builder,
            proof_mt_id: None,
        }
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
        let process_correlation_id = std::env::var("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID").ok();
        let process_identity =
            screenshot_process_identity(process_correlation_id.as_deref(), std::process::id());
        let outcome_id = format!(
            "{process_identity}:{}:{source}:{ordinal}",
            sanitize(&test_name)
        );
        let scenario_id = std::env::var("HANDSHAKE_PROOF_PROCESS_SCENARIO_ID")
            .ok()
            .filter(|scenario| !scenario.trim().is_empty())
            .map(|scenario| format!("matrix:{scenario}"))
            .unwrap_or_else(|| format!("runtime:{test_name}"));

        if !gpu_screenshot_enabled() {
            let marker = record_screenshot_outcome(
                &self.proof_mt_id,
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
            "{}-{process_identity}-{ordinal}.png",
            sanitize(&test_name),
        ));
        if let Err(error) = image.save(&frame_path) {
            let reason = format!(
                "central screenshot save failed at {}: {error}",
                frame_path.display()
            );
            let marker = write_blocked(&self.proof_mt_id, &scenario_id, &outcome_id, &reason)
                .map_err(|write| {
                    format!("{reason}; durable BLOCKED marker write failed: {write}")
                })?;
            self.last_screenshot_outcome = Some(marker.into());
            return Err(reason);
        }
        let marker = record_screenshot_outcome(
            &self.proof_mt_id,
            &scenario_id,
            &outcome_id,
            Ok(frame_path.display().to_string()),
        )
        .map_err(|error| format!("durable CAPTURED marker write failed: {error}"))?;
        self.last_screenshot_outcome = Some(marker.into());
        Ok(image)
    }

    /// Initialize the optional wgpu renderer without producing screenshot evidence.
    ///
    /// The real Argus screenshot route has a two-second production timeout. First-use shader and
    /// pipeline initialization is setup work rather than screenshot latency, so governed GPU proofs
    /// perform it before issuing the RPC request. The subsequent request still renders the terminal
    /// UI state through the real bounded route and is the only render that writes a proof marker.
    pub fn warm_gpu_renderer(&mut self) -> Result<(), String> {
        if !gpu_screenshot_enabled() {
            return Ok(());
        }
        catch_unwind(AssertUnwindSafe(|| self.inner.render()))
            .map_err(|_| "wgpu renderer warm-up panicked".to_owned())?
            .map(|_| ())
            .map_err(|error| format!("wgpu renderer warm-up failed: {error}"))
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

    /// Advance one ordinary application frame before capturing proof pixels. Canonical action
    /// receipts and AccessKit snapshots can terminalize in the dispatch frame while wgpu still owns
    /// the preceding painted frame; this seam makes the pixel artifact observe the same settled state.
    #[track_caller]
    pub fn render_settled_proof_frame(&mut self, expectation: &str) -> Option<image::RgbaImage> {
        self.inner.step();
        self.render_proof_frame(expectation)
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
        let marker = write_blocked(&self.proof_mt_id, scenario_id, outcome_id, &reason)
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

pub struct ScreenshotHarnessBuilder<State = ()> {
    inner: egui_kittest::HarnessBuilder<State>,
    proof_mt_id: Option<String>,
}

impl<State> ScreenshotHarnessBuilder<State> {
    pub fn with_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }

    pub fn with_step_dt(mut self, step_dt: f32) -> Self {
        self.inner = self.inner.with_step_dt(step_dt);
        self
    }

    /// Bind screenshot markers to the exact microtask without mutating process-global environment.
    pub fn proof_mt_id(mut self, mt_id: impl Into<String>) -> Self {
        let mt_id = mt_id.into();
        assert!(!mt_id.trim().is_empty(), "proof MT id cannot be blank");
        self.proof_mt_id = Some(mt_id);
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
            inner: self.inner.build_state(app, state),
            last_screenshot_outcome: None,
            proof_mt_id: self.proof_mt_id.unwrap_or_else(default_proof_mt_id),
        }
    }

    pub fn build_ui_state<'a>(
        self,
        app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
        state: State,
    ) -> ScreenshotHarness<'a, State> {
        ScreenshotHarness {
            inner: self.inner.build_ui_state(app, state),
            last_screenshot_outcome: None,
            proof_mt_id: self.proof_mt_id.unwrap_or_else(default_proof_mt_id),
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
            inner: self.inner.build_eframe(build),
            last_screenshot_outcome: None,
            proof_mt_id: self.proof_mt_id.unwrap_or_else(default_proof_mt_id),
        }
    }
}

impl ScreenshotHarnessBuilder {
    pub fn build_ui<'a>(self, app: impl FnMut(&mut egui::Ui) + 'a) -> ScreenshotHarness<'a> {
        ScreenshotHarness {
            inner: self.inner.build_ui(app),
            last_screenshot_outcome: None,
            proof_mt_id: self.proof_mt_id.unwrap_or_else(default_proof_mt_id),
        }
    }
}

fn write_blocked(
    mt_id: &str,
    scenario_id: &str,
    outcome_id: &str,
    reason: &str,
) -> std::io::Result<ScreenshotMarker> {
    let marker = ScreenshotMarker::blocked(mt_id, scenario_id, outcome_id, reason);
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

fn screenshot_process_identity(correlation_id: Option<&str>, pid: u32) -> String {
    correlation_id
        .filter(|identity| !identity.trim().is_empty())
        .map(sanitize)
        .filter(|identity| !identity.is_empty())
        .unwrap_or_else(|| format!("pid{pid}"))
}

#[cfg(test)]
mod tests {
    use super::screenshot_process_identity;

    #[test]
    fn process_identity_survives_pid_reuse_between_supervised_scenarios() {
        let first = screenshot_process_identity(Some("cargo-scenario-a-111"), 76348);
        let second = screenshot_process_identity(Some("cargo-scenario-b-222"), 76348);

        assert_ne!(first, second);
        assert_eq!(
            screenshot_process_identity(None, 76348),
            "pid76348",
            "standalone tests retain a deterministic process-local fallback"
        );
    }
}
