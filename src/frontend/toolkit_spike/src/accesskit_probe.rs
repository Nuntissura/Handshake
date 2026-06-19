// Probe (a): AccessKit tree readable OUT-OF-PROCESS + action dispatch.
// REAL test: a child process (`toolkit_spike --accesskit-app`) opens a real eframe window that
// exposes (via eframe's accesskit_winit -> accesskit_windows UIA provider) a button named
// "btn_test" and a label "akcount: N". This parent process uses the Windows UIAutomation client
// (uiautomation crate) to find that button OUT-OF-PROCESS by name, read the counter label, fire
// the Invoke pattern (a real UIA action across the process boundary), then re-read the counter and
// assert it incremented. This is the true "a model sees + steers the running app's a11y tree from
// outside the process" proof. CONTROL-2: a 10s timeout; if the window never appears the probe
// records an honest TIMEOUT (false), it never hangs.

use std::process::Command;
use std::time::{Duration, Instant};

pub struct ProbeResult {
    pub pass: bool,
    pub notes: String,
}

// ---- child mode: the app under test ----

#[derive(Default)]
struct AkApp {
    count: u32,
    start: Option<Instant>,
}

impl eframe::App for AkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let start = *self.start.get_or_insert_with(Instant::now);
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("btn_test").clicked() {
                self.count += 1;
            }
            ui.label(format!("akcount: {}", self.count));
        });
        // keep repainting so AccessKit updates flow promptly to UIA clients
        ctx.request_repaint_after(Duration::from_millis(100));
        // safety self-close in case the parent fails to kill us
        if start.elapsed() > Duration::from_secs(30) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Runs the eframe window (child process entrypoint). Blocks until the window closes.
pub fn run_app() {
    eprintln!("[child] --accesskit-app: starting eframe window 'akspike'");
    let options = eframe::NativeOptions::default();
    let r = eframe::run_native(
        "akspike",
        options,
        Box::new(|_cc| Ok(Box::new(AkApp::default()))),
    );
    eprintln!("[child] run_native returned ok={}", r.is_ok());
}

// ---- parent mode: the out-of-process reader ----

fn parse_count(name: &str) -> Option<i64> {
    name.rsplit(':').next()?.trim().parse::<i64>().ok()
}

pub fn run() -> ProbeResult {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return ProbeResult { pass: false, notes: format!("current_exe failed: {e}") },
    };
    let mut child = match Command::new(&exe).arg("--accesskit-app").spawn() {
        Ok(c) => c,
        Err(e) => return ProbeResult { pass: false, notes: format!("spawn child app failed: {e}") },
    };

    let result = (|| -> ProbeResult {
        let auto = match uiautomation::UIAutomation::new() {
            Ok(a) => a,
            Err(e) => return ProbeResult { pass: false, notes: format!("UIAutomation::new failed: {e}") },
        };

        // Diagnostic: can UIA see the app's top-level window at all?
        match auto
            .create_matcher()
            .match_name("akspike")
            .timeout(8_000)
            .interval(300)
            .find_first()
        {
            Ok(_) => eprintln!("[probe a] diagnostic: UIA FOUND window 'akspike'"),
            Err(ex) => eprintln!("[probe a] diagnostic: UIA did NOT find window 'akspike' ({ex})"),
        }

        // Find the button OUT-OF-PROCESS by name, retrying up to 25s (CONTROL-2).
        // 25s (not 10s) absorbs first-run UIA/window cold-start: the probe was observed to time
        // out once at 10s on a cold start, then pass; the mechanism is sound, it just needs time.
        let btn = auto
            .create_matcher()
            .match_name("btn_test")
            .timeout(25_000)
            .interval(300)
            .find_first();
        let btn = match btn {
            Ok(b) => b,
            Err(e) => {
                return ProbeResult {
                    pass: false,
                    notes: format!(
                        "TIMEOUT: btn_test not found out-of-process within 25s ({e}) - window may not have appeared in this environment"
                    ),
                }
            }
        };

        let read_count = || -> Option<i64> {
            auto.create_matcher()
                .contains_name("akcount:")
                .timeout(2_000)
                .interval(200)
                .find_first()
                .ok()
                .and_then(|el| el.get_name().ok())
                .as_deref()
                .and_then(parse_count)
        };

        let before = read_count();

        // Fire the Invoke pattern across the process boundary (the model action).
        let invoke = match btn.get_pattern::<uiautomation::patterns::UIInvokePattern>() {
            Ok(p) => p,
            Err(e) => return ProbeResult { pass: false, notes: format!("get Invoke pattern failed: {e}") },
        };
        if let Err(e) = invoke.invoke() {
            return ProbeResult { pass: false, notes: format!("invoke() failed: {e}") };
        }

        // Re-read the counter, retrying for the child's repaint to propagate.
        let mut after = None;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(250));
            after = read_count();
            if let (Some(b), Some(a)) = (before, after) {
                if a > b {
                    break;
                }
            }
        }

        let pass = matches!((before, after), (Some(b), Some(a)) if a == b + 1);
        ProbeResult {
            pass,
            notes: format!(
                "out-of-process UIA: found btn_test by name; Invoke dispatched; counter before={before:?} after={after:?}"
            ),
        }
    })();

    let _ = child.kill();
    let _ = child.wait();
    result
}
