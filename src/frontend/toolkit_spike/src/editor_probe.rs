// Probe (c): editor text surface.
// REAL test: render a multiline egui::TextEdit in a headless egui_kittest harness, give it
// focus, push 10 synthetic Text input events (one per character), run a frame, then read the
// app's text buffer back via harness.state() and assert it contains exactly the typed chars.
// Headless (no OS window); the synthetic events go through egui's real input pipeline into the
// real TextEdit widget — not a mocked buffer.

use egui_kittest::Harness;

pub struct ProbeResult {
    pub pass: bool,
    pub notes: String,
}

const TARGET: &str = "abcdefghij"; // exactly 10 characters

pub fn run() -> ProbeResult {
    let mut harness = Harness::builder().build_ui_state(
        |ui, text: &mut String| {
            let resp = ui.add(egui::TextEdit::multiline(text).id_salt("spike_editor"));
            // Keep the editor focused so synthetic Text events are routed into it.
            resp.request_focus();
        },
        String::new(),
    );

    // Lay out + establish focus.
    harness.run();

    // Synthetic typing: one egui Text event per character (real input pipeline).
    for ch in TARGET.chars() {
        harness.event(egui::Event::Text(ch.to_string()));
    }
    harness.run();

    let buf: &String = harness.state();
    let pass = buf.as_str() == TARGET;

    ProbeResult {
        pass,
        notes: format!(
            "typed {} synthetic chars; buffer={:?} (len {})",
            TARGET.chars().count(),
            buf,
            buf.chars().count()
        ),
    }
}
