// WP-KERNEL-011 MT-004 build script.
//
// Two jobs, both deterministic and disk-agnostic:
//
// 1. Font asset change tracking: tell cargo to re-run (and thus re-embed via include_bytes! in
//    app.rs) whenever either bundled Inter face changes. Without this, editing a .ttf would not
//    trigger a rebuild of the binary that embeds it.
//
// 2. Version + build-date stamping: expose HANDSHAKE_NATIVE_VERSION and HANDSHAKE_BUILD_DATE to
//    the crate via env! so a later MT can render them in an about/title surface. The date comes
//    from the SOURCE_DATE_EPOCH env var when present (reproducible-build friendly) else falls back
//    to "unknown" — we deliberately do NOT shell out to `date` so the build stays hermetic and
//    portable ([GLOBAL-PORTABILITY]).

use std::path::Path;

fn main() {
    // (1) Re-embed fonts when they change. Reference the paths unconditionally so the rerun signal
    // is present regardless of the bundled-fonts feature flag (cargo only acts on the include when
    // the feature compiles the include_bytes!, but the rerun hint is harmless when off).
    println!("cargo:rerun-if-changed=assets/fonts/Inter-Regular.ttf");
    println!("cargo:rerun-if-changed=assets/fonts/Inter-Bold.ttf");
    println!("cargo:rerun-if-changed=build.rs");

    // (2) Version stamp. CARGO_PKG_VERSION is the [package] version (0.1.0); append -dev to mark
    // an unsigned/unreleased build (the installer scaffold in MT-004 is functional-but-unsigned).
    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    println!("cargo:rustc-env=HANDSHAKE_NATIVE_VERSION={pkg_version}-dev");

    // Build date: prefer SOURCE_DATE_EPOCH (seconds since epoch) for reproducibility; else unknown.
    let build_date = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(epoch) => epoch,
        Err(_) => "unknown".to_string(),
    };
    println!("cargo:rustc-env=HANDSHAKE_BUILD_DATE={build_date}");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    // Fail-fast guard: when bundled-fonts is enabled the include_bytes! in app.rs needs the assets
    // to exist. Emit a clear build error here (before the opaque include_bytes! error) so a missing
    // asset is diagnosable by a no-context model (RISK-6 / CONTROL-6).
    if std::env::var("CARGO_FEATURE_BUNDLED_FONTS").is_ok() {
        for face in ["assets/fonts/Inter-Regular.ttf", "assets/fonts/Inter-Bold.ttf"] {
            if !Path::new(face).exists() {
                panic!(
                    "bundled-fonts feature is enabled but {face} is missing. Run the MT-004 font \
                     fetch (see installer/windows/BUNDLED_DEPS_POLICY.md 'Font provenance') or build \
                     with --no-default-features to use eframe's default fonts."
                );
            }
        }
    }
}
