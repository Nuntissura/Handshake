// WP-KERNEL-011 MT-004 single-binary proof test.
//
// Asserts that the release-native build of handshake-native.exe is a TRUE single binary: every DLL
// in its PE import table is a Windows system/OS DLL or a CRT apiset — no bundled third-party .dll
// sits next to the .exe — AND that the MSVC CRT is statically linked (VCRUNTIME140.dll absent),
// which proves the crate-local .cargo/config.toml `target-feature=+crt-static` actually took effect
// (CONTROL-1 / RISK-1).
//
// It also proves the font-bundling contract surface: the Inter-Regular/Bold .ttf assets exist, are
// non-trivially sized, match the canonical Inter 3.19 release checksums (Font provenance), and the
// OFL.txt license file is present and is the SIL Open Font License (CONTROL-3 / RISK-3).
//
// DEVIATIONS (recorded here for the reviewer):
//   * PATH: the contract names this file tests/native_gui/test_single_binary.rs. cargo derives the
//     integration-test target name from a file directly in the crate `tests/` dir; a tests/native_gui/
//     subdir would NOT register a `test_single_binary` target (`cargo test --test test_single_binary`
//     would fail to find it). This file therefore lives at tests/test_single_binary.rs, matching the
//     same decision already documented in tests/test_theme.rs (MT-003).
//   * PE PARSER: the contract suggests the `pelite` crate. We use the `object` crate (pin 0.36)
//     instead — it is the MT-001 spike-proven parser on this host (dumpbin is absent;
//     toolkit_spike_verdict.json probe d), reads the same PE import table, and adds NO new
//     dependency family. CONTROL-6 (RISK-6: parse failure on large binaries) is honored: a parse
//     failure on a >50 MB binary degrades to a non-fatal WARN + skip rather than a hard failure.
//   * TARGET DIR: this crate's .cargo/config.toml redirects the cargo target dir to the external
//     artifacts root (CX-212E), so the binary is NOT at ./target/release-native/. The test resolves
//     the real location from CARGO_TARGET_DIR or the known crate-relative artifacts path.

use std::path::{Path, PathBuf};

/// "System DLL" = a Windows apiset (api-ms-win-* / ext-ms-*), a CRT redistributable
/// (vcruntime*/ucrtbase/msvcp*/msvcrt/concrt), or a DLL that resolves in %WINDIR%\System32.
/// This mirrors the MT-001 spike probe d allowlist (binary_probe.rs) so the single-binary
/// definition is consistent across the WP.
fn is_system_dll(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("api-ms-win-") || lower.starts_with("ext-ms-") {
        return true;
    }
    for p in ["vcruntime", "ucrtbase", "msvcp", "msvcrt", "concrt"] {
        if lower.starts_with(p) {
            return true;
        }
    }
    if let Ok(windir) = std::env::var("WINDIR") {
        if Path::new(&windir).join("System32").join(name).exists() {
            return true;
        }
    }
    false
}

/// Resolve the cargo target directory honoring CARGO_TARGET_DIR, then the crate-local
/// .cargo/config.toml override (the external artifacts root), then the default ./target.
fn target_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // .cargo/config.toml: target-dir = "../../../../Handshake_Artifacts/handshake-native-target"
    let external = manifest.join("../../../../Handshake_Artifacts/handshake-native-target");
    if external.exists() {
        return external;
    }
    manifest.join("target")
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts")
}

/// CONTROL-3 / RISK-3: the OFL license file must ship with the bundled fonts.
#[test]
fn ofl_license_present_and_is_sil_ofl() {
    let ofl = fonts_dir().join("OFL.txt");
    let body = std::fs::read_to_string(&ofl)
        .unwrap_or_else(|e| panic!("OFL.txt missing at {}: {e}", ofl.display()));
    assert!(
        body.contains("SIL OPEN FONT LICENSE"),
        "OFL.txt does not contain 'SIL OPEN FONT LICENSE' — wrong/placeholder license file",
    );
}

/// Acceptance: fonts exist and are non-empty (>50 KB each), and match the canonical Inter 3.19
/// release checksums (Font provenance — proof_target). Checksums are SHA-256 of the
/// "Inter Hinted for Windows/Desktop/Inter-{Regular,Bold}.ttf" entries in Inter-3.19.zip.
#[test]
fn fonts_present_sized_and_provenance_matches() {
    // Expected SHA-256 of the canonical Inter 3.19 Desktop faces (documented in
    // installer/windows/BUNDLED_DEPS_POLICY.md "Font provenance").
    const EXPECT_REGULAR: &str =
        "529be850e06f62f8904f22bda77e45bde4834498fdbec4ff4201fa3177447a3a";
    const EXPECT_BOLD: &str = "e6c172fd8a2f957414a7a63ec8deb7f2aa239182394cfa5ee2ea6927c6194389";

    for (file, expect) in [
        ("Inter-Regular.ttf", EXPECT_REGULAR),
        ("Inter-Bold.ttf", EXPECT_BOLD),
    ] {
        let path = fonts_dir().join(file);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("font missing at {}: {e}", path.display()));
        assert!(
            bytes.len() > 50 * 1024,
            "{file} is only {} bytes (<50 KB) — looks truncated/placeholder",
            bytes.len()
        );
        let got = sha256_hex(&bytes);
        assert_eq!(
            got, expect,
            "{file} checksum mismatch — not the canonical Inter 3.19 face.\n  got:    {got}\n  expect: {expect}",
        );
    }
}

/// Acceptance: the bundled Inter font is actually the active proportional font (not the eframe
/// fallback). Drives the real font-install path on a headless egui context and asserts the
/// "Inter-Bold" NAMED family is registered — that family only exists because the bundled-fonts code
/// in `HandshakeApp::install_fonts` ran; eframe's defaults never create an "Inter-Bold" family.
/// This is the kittest font-family probe the acceptance criteria allow in lieu of a pixel readback.
#[cfg(feature = "bundled-fonts")]
#[test]
fn bundled_inter_font_is_registered_in_context() {
    use egui::FontFamily;
    use handshake_native::app::HandshakeApp;

    let ctx = egui::Context::default();
    HandshakeApp::install_fonts(&ctx);
    // Force the font set to materialize for this frame so `families()` reflects it.
    let _ = ctx.run(Default::default(), |_| {});

    let families = ctx.fonts(|f| f.families());
    assert!(
        families.contains(&FontFamily::Name("Inter-Bold".into())),
        "Inter-Bold named family not registered — bundled-fonts install path did not run. \
         families present: {families:?}",
    );
    // The Proportional family must be able to render basic Latin text via the embedded Inter face.
    let has_glyphs = ctx.fonts_mut(|f| {
        f.has_glyphs(&egui::FontId::proportional(14.0), "Handshake")
    });
    assert!(has_glyphs, "proportional font cannot render 'Handshake' after Inter install");
    println!("PASS: bundled Inter font registered (Proportional + Inter-Bold named family)");
}

/// The single-binary DLL audit. Skips cleanly (Ok) if the release-native binary has not been built,
/// so `cargo test` is green on a fresh checkout; the installer/CI pipeline builds it first.
#[test]
fn release_native_binary_has_no_non_system_dlls() {
    let exe = target_dir().join("release-native").join("handshake-native.exe");
    if !exe.exists() {
        eprintln!(
            "SKIP: release-native binary not found at {}; run \
             `cargo build --profile release-native -p handshake-native` first",
            exe.display()
        );
        return;
    }

    let data = std::fs::read(&exe).expect("read release-native exe");

    // CONTROL-6 / RISK-6: degrade gracefully if the parser cannot read a very large binary.
    let file = match object::File::parse(&*data) {
        Ok(f) => f,
        Err(e) => {
            if data.len() > 50 * 1024 * 1024 {
                eprintln!(
                    "WARN: object parse failed on large binary ({} MB); manual `dumpbin /dependents` \
                     verification required: {e}",
                    data.len() / (1024 * 1024)
                );
                return;
            }
            panic!("object parse of {} failed: {e}", exe.display());
        }
    };

    use object::Object;
    let imports = file.imports().expect("read PE import table");
    let mut dlls: Vec<String> = imports
        .iter()
        .map(|i| String::from_utf8_lossy(i.library()).to_string())
        .collect();
    dlls.sort();
    dlls.dedup();

    let non_system: Vec<&String> = dlls.iter().filter(|d| !is_system_dll(d)).collect();
    assert!(
        non_system.is_empty(),
        "non-system DLL dependencies found (single-binary constraint violated): {non_system:?}\nall imports: {dlls:?}",
    );

    // CONTROL-1 / RISK-1: VCRUNTIME140.dll present => CRT is dynamic => crt-static flag did not
    // apply. This is the canonical signal that the crate-local .cargo/config.toml took effect.
    let dynamic_crt = dlls
        .iter()
        .any(|d| d.eq_ignore_ascii_case("VCRUNTIME140.dll") || d.eq_ignore_ascii_case("MSVCP140.dll"));
    assert!(
        !dynamic_crt,
        "CRT is dynamic (VCRUNTIME140.dll/MSVCP140.dll imported) — crt-static flag not applied. \
         Check src/frontend/handshake_native/.cargo/config.toml [target.x86_64-pc-windows-msvc] rustflags.",
    );

    println!(
        "PASS: no non-system DLLs found in handshake-native.exe ({} system DLLs imported)",
        dlls.len()
    );
}

// --- minimal pure-Rust SHA-256 (no new dependency; used only for font provenance) ---
// Standard FIPS 180-4 implementation. Self-checked against the known Inter checksums by the
// fonts_present_sized_and_provenance_matches test above, which is itself the regression guard.

fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let bitlen = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}
