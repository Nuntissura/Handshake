//! Main-window geometry must come from Handshake's explicit native startup contract, not eframe's
//! machine-local persistence. A stale framework record can otherwise restore a sub-minimum HWND and
//! clip every operator/model work surface before product layout persistence can run.

#[test]
fn native_entrypoint_disables_competing_eframe_window_persistence() {
    let main_rs = include_str!("../src/main.rs");
    assert!(
        main_rs.contains("persist_window: false"),
        "the root window must ignore stale machine-local eframe geometry"
    );
    assert!(main_rs.contains(".with_inner_size([1280.0, 800.0])"));
    assert!(main_rs.contains(".with_min_inner_size([640.0, 480.0])"));

    let app_rs = include_str!("../src/app.rs");
    assert!(app_rs.contains("ViewportCommand::MinInnerSize(egui::vec2(640.0, 480.0))"));
    assert!(app_rs.contains("ViewportCommand::InnerSize(egui::vec2(1280.0, 800.0))"));
}
