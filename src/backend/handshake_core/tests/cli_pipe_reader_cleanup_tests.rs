use std::time::Duration;

use handshake_core::model_runtime::cloud::official_cli_bridge::hostile_never_eof_reader_cleanup_probe;

#[test]
fn hostile_never_eof_reader_cleanup_is_bounded() {
    let (completed, elapsed) = hostile_never_eof_reader_cleanup_probe();

    assert!(
        !completed,
        "a never-EOF reader must report incomplete cleanup"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "reader cleanup exceeded its bounded deadline: {elapsed:?}"
    );
}
