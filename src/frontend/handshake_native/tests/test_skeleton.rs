// MT-002 smoke test: render HandshakeApp headlessly (egui_kittest, no live backend via with_health)
// for a frame and assert the title label is present in the AccessKit tree.
// (Lives in the crate's tests/ dir since handshake_native is a standalone crate — see lib.rs note.)

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;

#[test]
fn skeleton_renders_title_and_status() {
    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();

    // AccessKit query: the title label must exist (get_by_label panics if not found).
    let _title = harness.get_by_label("Handshake");
    println!("PASS: handshake_title widget found");
}
