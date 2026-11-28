//! Source scans with the window crate's scanner (comments and literals blanked, so naming a call is not making one):
//! over every member of the shell's and the ports' workspaces, read from their manifests so a new member is scanned
//! without editing this file (MT-215 E3), one publisher of the ring's heartbeat slot, one heartbeat port and one ring
//! writer in product code (HB-7, E3); no window library, graphics binding, Win32 type or unsafe code in this crate's
//! code, and only its listed dependencies (AC214-1, AC215-1); nothing here moves focus or the cursor or injects input
//! (HBR-QUIET); every loop of a test on a private desktop; every ring file of a test removed by its guard (E7); and the
//! shell program's rules (MT-214 S5): the device opens in one place, the product binary declares no test authority and
//! the test binary says it is one, nothing here installs a panic or exception handler, and no widget or pane logic.

#[path = "../../window/tests/source/mod.rs"]
mod source;

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use limits::TimeLimit;
use source::{below, code_only, lines_with, Source};

/// The time limit of a scan: it reads a few hundred files.
const LIMIT: Duration = Duration::from_secs(60);

/// The five crates of the shell, as folders relative to this crate.
const CRATE_FOLDERS: &[&str] = &["../window", "../surface", "../input", "../accessibility", "../shell"];

/// The workspaces whose members are scanned, as folders relative to this crate.
const WORKSPACES: &[&str] = &["..", "../../handshake_host_ports"];

/// Members that must be among them, so a manifest the parser misreads cannot shrink the scan unnoticed.
const KNOWN_MEMBERS: &[&str] = &[
    "../window",
    "../surface",
    "../input",
    "../accessibility",
    "../shell",
    "../../handshake_host_ports/ui_heartbeat",
    "../../handshake_host_ports/palmistry_launch",
];

/// The port's own code, which defines what the rules below forbid elsewhere.
const PORT_SRC: &str = "../../handshake_host_ports/ui_heartbeat/src/";

/// The shell's adapter: the one place in product code that makes the port and names its sink.
const ADAPTER: &str = "../shell/src/heartbeat.rs";

/// What writes the heartbeat slot or stands in for the port's beat: the ring's writer call, the native host's recorder
/// call and its per-frame bump.
const PUBLISHERS: &[&str] = &["write_heartbeat", "bump_heartbeat", "RecorderHeartbeat"];

/// What makes a ring writer: the ring's only writer constructors (`handshake_native/diag_ring/src/ring.rs`).
const RING_WRITERS: &[&str] = &["DiagRingWriter::create", "DiagRingWriterV2::create"];

/// Not in this crate's code at all: they come through the Handshake crates, privately.
const LIBRARIES: &[&str] = &[
    "winit",
    "ash",
    "vk",
    "windows_sys",
    "raw_window_handle",
    "accesskit",
    "egui",
    "eframe",
    "wgpu",
    "glow",
    "kumihimo",
    "HWND",
    "unsafe",
];

/// The window crate's ban (its `tests/test_focus_ban.rs`).
const NEVER: &[&str] = &[
    "SetForegroundWindow",
    "BringWindowToTop",
    "SetActiveWindow",
    "SwitchToThisWindow",
    "AllowSetForegroundWindow",
    "LockSetForegroundWindow",
    "SetFocus",
    "AttachThreadInput",
    "keybd_event",
    "mouse_event",
    "SendInput",
    "InjectTouchInput",
    "InjectSyntheticPointerInput",
    "SetCursorPos",
    "ClipCursor",
    "SetWindowsHookExW",
    "SetWindowsHookExA",
    "SetWinEventHook",
    "FlashWindow",
    "FlashWindowEx",
    "SwitchDesktop",
    "OpenInputDesktop",
    "ShowWindow",
    "SetWindowPos",
    "focus_window",
    "request_user_attention",
    "set_cursor_position",
];

fn lines(sources: &[Source], words: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        for word in words {
            for number in lines_with(&source.code, word) {
                found.push(format!("{}:{number}: {word}", source.path));
            }
        }
    }
    found
}

fn sample(path: &str, code: &str) -> Source {
    Source {
        path: path.to_owned(),
        code: code_only(code),
    }
}

/// The `members` of a manifest's `[workspace]` table.
fn workspace_members(manifest: &str) -> Vec<String> {
    let code = manifest
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    let table = code
        .split("[workspace]")
        .nth(1)
        .expect("a [workspace] table");
    let table = table.split("\n[").next().unwrap_or(table);
    let list = table
        .split_once("members")
        .and_then(|(_, rest)| rest.split_once('['))
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(list, _)| list)
        .expect("a members list");
    list.split(',')
        .map(|member| member.trim().trim_matches('"').to_owned())
        .filter(|member| !member.is_empty())
        .collect()
}

/// Every member of the scanned workspaces, as a folder relative to this crate.
fn members() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut folders = Vec::new();
    for workspace in WORKSPACES {
        let manifest = std::fs::read_to_string(root.join(workspace).join("Cargo.toml"))
            .expect("the workspace's manifest");
        for member in workspace_members(&manifest) {
            assert!(
                !member.contains('*'),
                "{workspace}: the glob member {member} is not expanded by this scan"
            );
            folders.push(format!("{workspace}/{member}"));
        }
    }
    folders
}

/// Path crates of the closure that are not product code: the ring's own crate defines the writer, and the watcher's
/// crates are a dev-dependency of the port's tests that read it and write rings in their own tests (MT-215 N4). Every
/// other path crate of either closure is scanned, wherever it lives.
const NOT_SCANNED_CRATES: &[&str] = &["handshake-diag-ring", "palmistry", "palmistry-control-protocol"];

/// What makes the port or its sink, which no rename may hide (MT-215 N4: `use ... as`).
const RENAME_WATCHED: &[&str] = &[
    "DiagRingWriter",
    "DiagRingWriterV2",
    "UiHeartbeat",
    "HeartbeatSink",
    "PortToken",
    "RecorderHeartbeat",
    "ShellHeartbeat",
];

/// The direct children of the JSON container that starts `text`: key (empty in an array) and raw value.
fn json_children(text: &str) -> Vec<(String, &str)> {
    let bytes = text.as_bytes();
    let is_object = bytes.first() == Some(&b'{');
    let mut found = Vec::new();
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    let mut string_from = 0;
    let mut key = String::new();
    let mut value_from: Option<usize> = None;
    for (at, &c) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
                if depth == 1 && is_object && value_from.is_none() {
                    key = text[string_from + 1..at].to_owned();
                }
            }
            continue;
        }
        match c {
            b'"' => {
                in_string = true;
                string_from = at;
            }
            b'{' | b'[' => {
                depth += 1;
                if depth == 1 && !is_object {
                    value_from = Some(at + 1);
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(from) = value_from.take() {
                        let raw = text[from..at].trim();
                        if !raw.is_empty() {
                            found.push((std::mem::take(&mut key), raw));
                        }
                    }
                    break;
                }
            }
            b':' if depth == 1 && is_object && value_from.is_none() => value_from = Some(at + 1),
            b',' if depth == 1 => {
                if let Some(from) = value_from.take() {
                    found.push((std::mem::take(&mut key), text[from..at].trim()));
                }
                if !is_object {
                    value_from = Some(at + 1);
                }
            }
            _ => {}
        }
    }
    found
}

fn unquote(raw: &str) -> String {
    raw.trim_matches('"').replace("\\\\", "\\").replace("\\/", "/")
}

/// Name and manifest of every path crate (`"source": null` of the package itself, not of a dependency entry in it) in
/// the output of `cargo metadata`.
fn path_crates(metadata: &str) -> Vec<(String, String)> {
    let root = json_children(metadata.trim());
    let packages = root
        .iter()
        .find(|(key, _)| key == "packages")
        .expect("a packages list in the metadata")
        .1;
    json_children(packages)
        .into_iter()
        .filter_map(|(_, package)| {
            let fields = json_children(package);
            let get = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, value)| *value);
            if get("source")? != "null" {
                return None;
            }
            Some((unquote(get("name")?), unquote(get("manifest_path")?)))
        })
        .collect()
}

/// `cargo metadata --offline` of a workspace, for the platform this test runs on, with no console window.
fn metadata(workspace: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let target = if cfg!(target_arch = "aarch64") {
        "aarch64-pc-windows-msvc"
    } else {
        "x86_64-pc-windows-msvc"
    };
    let mut command = Command::new(cargo);
    command
        .args(["metadata", "--format-version", "1", "--offline", "--locked", "--filter-platform", target])
        .arg("--manifest-path")
        .arg(root.join(workspace).join("Cargo.toml"))
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().expect("cargo metadata starts");
    assert!(
        output.status.success(),
        "cargo metadata of {workspace}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("metadata is UTF-8")
}

/// The folders of every path crate of both closures, besides the members and [`NOT_SCANNED_CRATES`]: what a path
/// dependency outside both workspaces adds (MT-215 N4, the eval's mutation S4).
fn closure_folders() -> &'static Vec<(String, PathBuf)> {
    static FOLDERS: OnceLock<Vec<(String, PathBuf)>> = OnceLock::new();
    FOLDERS.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let members: Vec<PathBuf> = members()
            .iter()
            .map(|member| root.join(member).canonicalize().expect("a member folder"))
            .collect();
        let mut folders: Vec<(String, PathBuf)> = Vec::new();
        for workspace in WORKSPACES {
            for (name, manifest) in path_crates(&metadata(workspace)) {
                let folder = Path::new(&manifest)
                    .parent()
                    .and_then(|folder| folder.canonicalize().ok())
                    .unwrap_or_else(|| panic!("{name}: no folder for {manifest}"));
                let known = folders.iter().any(|(_, held)| *held == folder);
                if !members.contains(&folder) && !known && !NOT_SCANNED_CRATES.contains(&name.as_str()) {
                    folders.push((name, folder));
                }
            }
        }
        folders
    })
}

fn read_below(folder: &Path, found: &mut Vec<Source>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            read_below(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path).expect("a source file");
            found.push(Source {
                path: path.to_string_lossy().replace('\\', "/"),
                code: code_only(&text),
            });
        }
    }
}

/// The product code of every path crate of the closures that is no member of either workspace.
fn closure_sources() -> Vec<Source> {
    let mut found = Vec::new();
    for (_, folder) in closure_folders() {
        read_below(&folder.join("src"), &mut found);
    }
    found
}

/// Every `use` that renames what the rules look for by name: a rename hides the name from them (MT-215 N4, the eval's
/// mutation S2).
fn renamed_imports(sources: &[Source]) -> Vec<String> {
    let part_of_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    for source in sources {
        for (at, _) in source.code.match_indices("use") {
            let before = source.code[..at].chars().next_back();
            let after = source.code[at + 3..].chars().next();
            if before.is_some_and(part_of_word) || after.is_some_and(part_of_word) {
                continue;
            }
            let statement = source.code[at..].split(';').next().unwrap_or_default();
            let words: Vec<&str> = statement
                .split(|c: char| !part_of_word(c))
                .filter(|word| !word.is_empty())
                .collect();
            let renames = words
                .windows(2)
                .any(|pair| RENAME_WATCHED.contains(&pair[0]) && pair[1] == "as");
            if renames {
                let line = source.code[..at].matches('\n').count() + 1;
                found.push(format!("{}:{line}: a renamed import", source.path));
            }
        }
    }
    found
}

/// The product code of every member: its `src`, and of every other path crate of the closures.
fn product_sources() -> Vec<Source> {
    let members = members();
    for known in KNOWN_MEMBERS {
        assert!(
            members.iter().any(|member| member == known),
            "{known} is not among the members read: {members:#?}"
        );
    }
    members
        .iter()
        .flat_map(|member| {
            let sources = below(&format!("{member}/src"));
            assert!(!sources.is_empty(), "{member}/src holds no Rust code");
            sources
        })
        .chain(closure_sources())
        .collect()
}

fn outside_the_port(sources: &[Source]) -> Vec<Source> {
    sources
        .iter()
        .filter(|source| !source.path.starts_with(PORT_SRC))
        .map(|source| Source {
            path: source.path.clone(),
            code: source.code.clone(),
        })
        .collect()
}

/// What writes the heartbeat slot past the port.
fn slot_writers(sources: &[Source]) -> Vec<String> {
    lines(&outside_the_port(sources), PUBLISHERS)
}

/// What names the port or its sink outside the port and the shell's adapter: a second port, or a second publisher
/// through a sink (the eval's mutation S4).
fn port_users_besides_the_adapter(sources: &[Source]) -> Vec<String> {
    let besides: Vec<Source> = outside_the_port(sources)
        .into_iter()
        .filter(|source| source.path != ADAPTER)
        .collect();
    lines(&besides, &["UiHeartbeat", "HeartbeatSink", "PortToken"])
}

/// Where the port is made.
fn ports_made(sources: &[Source]) -> Vec<String> {
    lines(
        &outside_the_port(sources),
        &["UiHeartbeat::new", "UiHeartbeat::with_frames"],
    )
}

/// Where a shell heartbeat is opened.
fn shell_heartbeats_opened(sources: &[Source]) -> Vec<String> {
    lines(sources, &["ShellHeartbeat::open"])
}

/// Where a ring writer is made.
fn ring_writers_made(sources: &[Source]) -> Vec<String> {
    lines(sources, RING_WRITERS)
}

#[test]
fn every_member_of_both_workspaces_is_read_from_the_manifests() {
    let _limit = TimeLimit::new(LIMIT);
    let sources = product_sources();
    assert!(sources.len() >= 50, "the scan reads the members: {}", sources.len());
    for known in KNOWN_MEMBERS {
        assert!(
            sources
                .iter()
                .any(|source| source.path.starts_with(&format!("{known}/src/"))),
            "{known} is scanned"
        );
    }

    let manifest = "# a comment naming [workspace]\n[package]\nname = \"x\"\n\n[workspace]\nresolver = \"2\"\n\
                    members = [\n    \"one\", # the first\n    \"two\",\n]\n\n[profile.dev]\ndebug = 0\n";
    assert_eq!(workspace_members(manifest), ["one", "two"]);
}

/// MT-215 N4 (the eval's mutation S4): a path dependency outside both workspaces is read from `cargo metadata` and
/// scanned with the rest, and the reader of the metadata takes a package's own `source`, not a dependency entry's.
#[test]
fn the_path_crate_closure_is_read_from_cargo_metadata_and_scanned() {
    let _limit = TimeLimit::new(LIMIT);
    let sample_metadata = r#"{"packages":[
        {"name":"local","version":"0.1.0","id":"path+file:///C:/a/local#0.1.0","source":null,
         "dependencies":[{"name":"x","source":null,"req":"*"}],"manifest_path":"C:\\a\\local\\Cargo.toml"},
        {"name":"registry","id":"r","source":"registry+https://x","dependencies":[{"name":"y","source":null}],
         "manifest_path":"C:\\c\\registry\\Cargo.toml", "description":"has \"source\":null, {braces} inside"},
        {"name":"other","source":null,"manifest_path":"C:/b/other/Cargo.toml"}],
        "workspace_members":["x"],"resolve":null}"#;
    assert_eq!(
        path_crates(sample_metadata),
        [
            ("local".to_owned(), "C:\\a\\local\\Cargo.toml".to_owned()),
            ("other".to_owned(), "C:/b/other/Cargo.toml".to_owned()),
        ]
    );

    let folders = closure_folders();
    let renderer = folders
        .iter()
        .find(|(name, _)| name == "handshake-vulkan-renderer")
        .unwrap_or_else(|| panic!("the path dependency outside both workspaces is in the closure: {folders:#?}"));
    let members: Vec<String> = members();
    assert!(
        !members.iter().any(|member| member.ends_with("vulkan-renderer")),
        "it is no member of either workspace"
    );
    for (name, _) in folders {
        assert!(!NOT_SCANNED_CRATES.contains(&name.as_str()), "{name}");
    }
    let closure = closure_sources();
    let prefix = renderer.1.to_string_lossy().replace('\\', "/");
    assert!(
        closure.iter().filter(|source| source.path.starts_with(&prefix)).count() >= 3,
        "its code is scanned"
    );
    let forbidden = sample(&format!("{prefix}/src/sample.rs"), "writer.write_heartbeat(1, 2);");
    assert_eq!(slot_writers(&[forbidden]).len(), 1, "a ring writer there is found");
}

/// MT-215 N4 (the eval's mutation S2): a rename of the ring writer or a port type is refused; others are not.
#[test]
fn a_renamed_import_of_the_ring_writer_or_the_port_is_refused() {
    let _limit = TimeLimit::new(LIMIT);
    let renamed = [
        "use handshake_diag_ring::ring::DiagRingWriterV2 as W;\nlet w = W::create(p, 1, n);",
        "use ui_heartbeat::{Heartbeat, UiHeartbeat as Port};",
        "pub use ui_heartbeat::{\n    HeartbeatSink\n        as Sink,\n};",
        "use ui_heartbeat::PortToken as T;",
    ];
    for code in renamed {
        let hidden = sample("../shell/src/sample.rs", code);
        assert_eq!(renamed_imports(std::slice::from_ref(&hidden)).len(), 1, "{code}");
    }
    let hidden = sample(
        "../shell/src/sample.rs",
        "use handshake_diag_ring::ring::DiagRingWriterV2 as W;\nlet w = W::create(p, 1, n);",
    );
    assert!(ring_writers_made(std::slice::from_ref(&hidden)).is_empty(), "the old rule does not see it");
    let fine = [
        "use handshake_shell_window::WakeRefused as LaneRefused;",
        "use ui_heartbeat::{UiHeartbeat, WakeRefused as Refused};",
        "let reuse = 1; // an old use of the word, UiHeartbeat as nothing",
        "let alias = UiHeartbeat as_ref;",
    ];
    for code in fine {
        let source = sample("../shell/src/sample.rs", code);
        assert!(renamed_imports(std::slice::from_ref(&source)).is_empty(), "{code}");
    }
}

#[test]
fn the_heartbeat_slot_has_one_publisher_one_port_and_one_ring_writer() {
    let _limit = TimeLimit::new(LIMIT);
    let sources = product_sources();

    let found = slot_writers(&sources);
    assert!(found.is_empty(), "the heartbeat slot is written past the port: {found:#?}");
    let port: Vec<Source> = below("../../handshake_host_ports/ui_heartbeat/src");
    assert!(port.len() >= 6, "the scan reads the port");
    let writes = lines(&port, &["write_heartbeat"]);
    assert_eq!(writes.len(), 2, "{writes:#?}");
    assert!(
        writes
            .iter()
            .all(|line| line.starts_with(&format!("{PORT_SRC}sink.rs:"))),
        "only the port's sinks write the slot: {writes:#?}"
    );

    let found = port_users_besides_the_adapter(&sources);
    assert!(found.is_empty(), "the port or its sink used past the adapter: {found:#?}");
    let made = ports_made(&sources);
    assert_eq!(made.len(), 1, "one port, made by the adapter: {made:#?}");
    assert!(made[0].starts_with(&format!("{ADAPTER}:")), "{made:#?}");
    let opened = shell_heartbeats_opened(&sources);
    assert!(opened.len() <= 1, "a second shell heartbeat in product code: {opened:#?}");
    let writers = ring_writers_made(&sources);
    assert!(writers.len() <= 1, "a second ring writer in product code: {writers:#?}");
    let renamed = renamed_imports(&outside_the_port(&sources));
    assert!(renamed.is_empty(), "a ring writer or port type imported under another name: {renamed:#?}");

    // Each rule finds what it looks for, and only calls.
    let mention = sample(
        "../surface/src/sample.rs",
        "writer.write_heartbeat(1, 2); // write_heartbeat",
    );
    assert_eq!(slot_writers(&[mention]).len(), 1, "a call, not a mention");
    let second_publisher = sample(
        "../shell/src/lib.rs",
        "pub fn second_publisher<S: ui_heartbeat::HeartbeatSink>(sink: &S) {}",
    );
    assert_eq!(port_users_besides_the_adapter(&[second_publisher]).len(), 1);
    let second_port = sample(
        ADAPTER,
        "let a = UiHeartbeat::with_frames(c, l, s, f);\nlet b = UiHeartbeat::new(c, l, s);",
    );
    assert_eq!(ports_made(&[second_port]).len(), 2);
    let elsewhere = sample(
        "../window/src/sample.rs",
        "let port = ui_heartbeat::UiHeartbeat::new(c, l, s);",
    );
    assert_eq!(port_users_besides_the_adapter(&[elsewhere]).len(), 1);
    let two_shells = sample(
        "../surface/src/sample.rs",
        "let a = ShellHeartbeat::open(control, at, x);\nlet b = ShellHeartbeat::open(control, at, y);",
    );
    assert_eq!(shell_heartbeats_opened(&[two_shells]).len(), 2);
    let two_writers = sample(
        "../shell/src/sample.rs",
        "let a = DiagRingWriterV2::create(p, n, k);\nlet b = handshake_diag_ring::DiagRingWriter::create(p, n);",
    );
    assert_eq!(ring_writers_made(&[two_writers]).len(), 2);
}

/// MT-214 S7 (P2): the program's ring is the launcher's version 2 ring, and nothing in the shell's code names the version 1
/// writer. Whole words: `DiagRingWriterV2` contains `DiagRingWriter`, which is not the same name.
#[test]
fn the_program_ring_is_version_two_and_no_version_one_writer_is_named() {
    let _limit = TimeLimit::new(LIMIT);
    let sources = below("src");
    assert!(sources.len() >= 7, "the scan reads the crate");
    let found = lines(&sources, &["DiagRingWriter", "DiagRingReader", "RING_VERSION"]);
    assert!(found.is_empty(), "the version 1 ring in the shell's code: {found:#?}");
    let program = source_in(&sources, "src/program.rs");
    assert!(!lines_with(&program.code, "DiagRingWriterV2").is_empty(), "the program holds the version 2 writer");
    assert!(!lines_with(&program.code, "DiagEventV2").is_empty(), "its marker is a version 2 event");

    let v1 = sample("src/sample.rs", "pub ring: Option<DiagRingWriter>,");
    assert_eq!(lines(&[v1], &["DiagRingWriter"]).len(), 1);
    let v2 = sample("src/sample.rs", "pub ring: Option<DiagRingWriterV2>,");
    assert!(lines(&[v2], &["DiagRingWriter"]).is_empty(), "the version 2 name is another word");
}

/// MT-214 S7 (P2, AC214-11, MT-215 HB-3): `ended` marks the ring and asks the watcher only inside the settled check, the
/// marker first, and a failed ring write is not reported as a marked ring.
#[test]
fn the_watcher_is_asked_only_inside_the_settled_check_after_the_ring_marker() {
    let _limit = TimeLimit::new(LIMIT);
    let program = source_in(&below("src"), "src/program.rs");
    let code = squeezed(&program);
    let ended = block_of(&code, "fnended(&self");
    let settled = ended.find("ifmay_acknowledge(ended.settled)").expect("the settled check");
    let marker = ended.find("ring.write(").expect("the ring marker");
    let asked = ended.find("acknowledge_shutdown()").expect("the watcher is asked");
    assert!(settled < marker && marker < asked, "settled check, then ring marker, then the watcher: {ended}");
    assert_eq!(ended.matches("acknowledge_shutdown()").count(), 1, "the watcher is asked once");
    let settled_block = block_of(ended, "ifmay_acknowledge(");
    assert!(
        settled_block.contains("acknowledge_shutdown()") && settled_block.contains("ring.write("),
        "the marker and the question are inside the settled check: {ended}"
    );
    assert!(
        !ended.contains("ring_marked=true"),
        "the ring is marked only when the write says it was: {ended}"
    );
    assert!(ended.contains("ring_marked=") && ended.contains("ring.write("), "{ended}");
    assert!(
        ended.contains("ended.ring_marked=DiagEventV2::ordinary(marker).is_some_and(|event|ring.write(event));"),
        "the ring is marked exactly when the write returned true, nothing else decides it: {ended}"
    );

    // Each rule finds what it looks for.
    let early = sample(
        "src/sample.rs",
        "fn ended(&self, shell: &Shell<'_>) { self.watcher.acknowledge_shutdown(); if may_acknowledge(s) { ring.write(e); } }",
    );
    let code = squeezed(&early);
    let sample_ended = block_of(&code, "fnended(&self");
    assert!(sample_ended.find("acknowledge_shutdown()") < sample_ended.find("ifmay_acknowledge(s)"));
}

/// MT-214 S7 (P3, AC214-11): the adapter never lets an unasked watcher drop (the launcher's handle sends a clean Shutdown
/// when dropped), tells the watcher only in the one place that is asked, and takes `confirmed` from the handle's own report.
#[test]
fn the_adapter_never_drops_an_unasked_watcher_and_takes_confirmation_from_the_handle() {
    let _limit = TimeLimit::new(LIMIT);
    let adapter = source_in(&below("src"), "src/palmistry.rs");
    let code = squeezed(&adapter);
    assert!(code.contains("RefCell<Option<ManuallyDrop<Box<dynOwnedWatcher>>>>"), "the held watcher is never dropped");
    assert!(code.contains("ManuallyDrop::into_inner(watcher).shut_down()"), "the one way out is the question");
    assert!(!code.contains("implDropfor"), "no Drop of the adapter can send what the question must");
    assert_eq!(code.matches(".shutdown(").count(), 1, "the launcher's shutdown is called in one place");
    assert_eq!(code.matches("request_shutdown_and_wait").count(), 0, "no other way to send the Shutdown");
    // The confirmation is the handle's report and nothing else.
    let from_handle = block_of(&code, "pubfnfrom_handle(");
    assert!(from_handle.contains("letconfirmed=handle.handshake_acked();"), "{from_handle}");
    assert_eq!(code.matches("handshake_acked()").count(), 1);
    // Each rule finds what it looks for.
    let dropping = sample("src/sample.rs", "impl Drop for Watcher { fn drop(&mut self) { self.0.take(); } }");
    assert!(squeezed(&dropping).contains("implDropfor"));
}

/// Whether `code` (white space removed) binds something to `_`, which drops it at once.
fn binds_to_underscore(code: &str) -> bool {
    code.contains("let_=") || code.contains("let_:")
}

/// MT-214 S7 (P4a, AC214-11, MT-199 AC199-8): the session makes the shell's one ring, takes its lease before the ring, makes
/// the ring with the launch nonce of the launch inputs, holds the lease in a named binding, and records no marker through
/// the launcher's function-pointer sink (the ring writer is the heartbeat's).
#[test]
fn the_session_takes_the_lease_then_makes_the_one_ring_with_the_launch_nonce() {
    let _limit = TimeLimit::new(LIMIT);
    let sources = product_sources();
    let writers = ring_writers_made(&sources);
    assert_eq!(writers.len(), 1, "one ring writer, made by the session: {writers:#?}");
    assert!(writers[0].starts_with("../shell/src/palmistry.rs:"), "{writers:#?}");

    let session = source_in(&below("src"), "src/palmistry.rs");
    let code = squeezed(&session);
    let make = block_of(&code, "pubfnmake_ring(");
    assert!(make.contains("DiagRingWriterV2::create("), "{make}");
    assert_eq!(code.matches("DiagRingWriterV2::create(").count(), 1, "the create call is in make_ring only");
    let prepare = block_of(&code, "pubfnprepare(");
    let lease = prepare.find("acquire_embedded_runtime_instance_lease(").expect("the lease is acquired");
    let ring = prepare.find("make_ring(").expect("the ring is made");
    assert!(lease < ring, "the lease is taken before the ring is made: {prepare}");
    assert!(
        prepare.contains("make_ring(&inputs.session.ring_path,inputs.launch_nonce)"),
        "the ring is made with the launch nonce and the ring path of the launch inputs: {prepare}"
    );
    assert_eq!(prepare.matches("make_ring(").count(), 1, "one ring per start");
    assert_eq!(prepare.matches("Uuid::now_v7()").count(), 1, "one session id per start");
    assert!(
        prepare.contains("letlease=match(seams.host_scope)()"),
        "the lease is bound to a name the start hands on: {prepare}"
    );
    assert!(!binds_to_underscore(&code), "a binding to `_` drops its value at once: the lease would not be held");
    assert!(code.contains("marker_sink:None,"), "the launcher's marker sink stays empty");
    assert!(!code.contains("marker_sink:Some"), "a function pointer cannot reach the heartbeat's ring writer");

    // main and the example make their ring through the session's maker and name no writer.
    for path in ["src/main.rs", "examples/shell_program.rs"] {
        let text = squeezed(&sample(path, &text_of(path)));
        assert!(text.contains("make_ring("), "{path} makes its ring through the session's maker");
        assert!(!text.contains("DiagRingWriter"), "{path} names a ring writer");
    }

    // Each rule finds what it looks for.
    assert!(binds_to_underscore(&squeezed(&sample("src/sample.rs", "let _ = acquire_embedded_runtime_instance_lease(id, scope);"))));
    assert!(binds_to_underscore(&squeezed(&sample("src/sample.rs", "let _lease: u8 = 1; let _: u8 = 2;"))));
    assert!(!binds_to_underscore(&squeezed(&sample("src/sample.rs", "let lease = acquire(id, scope);"))));
    let late = sample(
        "src/sample.rs",
        "pub fn prepare() { let r = make_ring(a, b); let lease = acquire_embedded_runtime_instance_lease(i, s); }",
    );
    let late = squeezed(&late);
    assert!(late.find("make_ring(") < late.find("acquire_embedded_runtime_instance_lease("));
}

#[test]
fn no_window_library_or_graphics_binding_in_the_crates_code() {
    let _limit = TimeLimit::new(LIMIT);
    let sources: Vec<Source> = below("src")
        .into_iter()
        .map(|source| Source {
            code: source.code.replace("cfg(windows)", "").replace("not(windows)", ""),
            path: source.path,
        })
        .collect();
    assert!(sources.len() >= 3, "the scan reads the crate");
    let found = lines(&sources, LIBRARIES);
    assert!(found.is_empty(), "{found:#?}");
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a dependency table");
    let named: Vec<&str> = dependencies
        .lines()
        .filter_map(|line| line.split_once(" = ").map(|(name, _)| name.trim()))
        .collect();
    assert_eq!(
        named,
        [
            "handshake_shell_window",
            "handshake_shell_surface",
            "handshake_shell_input",
            "handshake_shell_accessibility",
            "ui_heartbeat",
            "handshake-diag-ring",
            "handshake-vulkan-renderer",
            "palmistry_launch",
            "handshake_runtime_instance",
            "uuid",
            "zeroize",
            "rand"
        ]
    );
}

#[test]
fn nothing_moves_focus_or_cursor_or_injects_input() {
    let _limit = TimeLimit::new(LIMIT);
    let sources: Vec<Source> = below("src").into_iter().chain(below("tests")).collect();
    let found = lines(&sources, NEVER);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn every_loop_of_a_test_runs_on_a_private_desktop() {
    let _limit = TimeLimit::new(LIMIT);
    let mut loops = 0;
    let mut program_loops = 0;
    for test in below("tests") {
        let code = &test.code;
        let calls: Vec<usize> = code
            .match_indices("run_script(")
            .map(|(at, _)| at)
            .filter(|at| {
                let before = &code[..*at];
                !before
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_')
                    && !before.trim_end().ends_with("fn")
            })
            .collect();
        let inside = code.find("on_private_desktop(");
        for at in &calls {
            assert!(
                inside.is_some_and(|start| start < *at),
                "{}: a loop before or without a private desktop",
                test.path
            );
        }
        loops += calls.len();
        // The shell program's loop (MT-214 S5) runs inside `on_private_desktop` as well.
        let programs: Vec<usize> = code
            .match_indices("run_program(")
            .map(|(at, _)| at)
            .filter(|at| !code[..*at].trim_end().ends_with("fn"))
            .collect();
        for at in &programs {
            assert!(
                inside.is_some_and(|start| start < *at),
                "{}: a program loop before or without a private desktop",
                test.path
            );
        }
        program_loops += programs.len();
        let direct = lines(
            std::slice::from_ref(&test),
            &["handshake_shell_window::run", "SetThreadDesktop", "CreateDesktopW"],
        );
        assert!(direct.is_empty(), "{direct:#?}");
    }
    assert_eq!(loops, 2, "the scan sees both loops");
    assert_eq!(program_loops, 3, "the scan sees the three program loops: two of the program, one of several windows");
}

/// What leaves a test's ring file to chance: removing it by hand, which a failure skips, or making a ring without the
/// guard that removes it however the test ends (MT-215 E7).
fn rings_left_to_chance(tests: &[Source]) -> Vec<String> {
    let mut found = lines(tests, &["remove_file"]);
    for test in tests {
        let makes_a_ring = !lines(std::slice::from_ref(test), RING_WRITERS).is_empty();
        let guarded = !lines_with(&test.code, "RingFile::new").is_empty();
        if makes_a_ring && !guarded {
            found.push(format!("{}: a ring without a RingFile", test.path));
        }
    }
    found
}

#[test]
fn every_ring_file_of_a_test_is_removed_by_its_guard() {
    let _limit = TimeLimit::new(LIMIT);
    let tests = below("tests");
    let found = rings_left_to_chance(&tests);
    assert!(found.is_empty(), "{found:#?}");
    let rings = tests
        .iter()
        .filter(|test| !lines(std::slice::from_ref(test), RING_WRITERS).is_empty())
        .count();
    // Three: the two heartbeat tests and tests/shell_program.rs, which keeps its own version 2 create (MT-214 S7 P2); the
    // Palmistry tests make no ring by a create call here, they go through the shell's session (P4a).
    assert_eq!(rings, 3, "the scan sees the three tests that make a ring");

    let by_hand = sample(
        "tests/sample.rs",
        "let ring = RingFile::new(\"x\");\nlet w = DiagRingWriterV2::create(ring.path(), 1, n);\nlet _ = std::fs::remove_file(&path);",
    );
    assert_eq!(rings_left_to_chance(&[by_hand]).len(), 1);
    let unguarded = sample(
        "tests/sample.rs",
        "let w = DiagRingWriterV2::create(&path, 1, n);",
    );
    assert_eq!(rings_left_to_chance(&[unguarded]).len(), 1);
}

/// A file of this crate as written, comments and literals included.
fn text_of(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).expect(path)
}

/// The files of `sources` that name `word` as a call or a type.
fn files_naming(sources: &[Source], word: &str) -> Vec<String> {
    let mut files: Vec<String> = sources
        .iter()
        .filter(|source| !lines_with(&source.code, word).is_empty())
        .map(|source| source.path.clone())
        .collect();
    files.sort();
    files
}

#[test]
fn the_device_opens_in_one_place_and_only_the_gate_reaches_it_for_the_product() {
    let _limit = TimeLimit::new(LIMIT);
    let sources = product_sources();
    let opened = files_naming(&sources, "PresentHost::open");
    assert_eq!(opened, ["../shell/src/program.rs"], "the presenter, and so the device, opens in one place");
    let programs = files_naming(&below("src"), "run_program");
    assert_eq!(
        programs,
        ["src/lib.rs", "src/main.rs", "src/program.rs"],
        "the program is defined, re-exported and started by the product binary, nowhere else"
    );

    // The product binary asks the gate, declares no test authority and prints what it runs under; it has no flag that
    // shows a window on a desktop.
    let main = code_only(&text_of("src/main.rs"));
    assert!(!lines_with(&main, "run_product").is_empty());
    assert!(!lines_with(&main, "receipt").is_empty());
    for word in ["TestBinary", "resolve_product", "IssuedAuthority"] {
        assert!(lines_with(&main, word).is_empty(), "the product binary names {word}");
    }
    assert!(!text_of("src/main.rs").contains("show-on-my-desktop"), "the product binary has a desktop flag");

    // A test declaration is made in no product code of this crate, only in its examples and tests.
    let declared = files_naming(&below("src"), "TestBinary::declare");
    assert!(declared.is_empty(), "a test authority is declared in product code: {declared:#?}");

    // The example is a test binary: it needs its flag, declares itself and prints its receipt before the program runs.
    let example_text = text_of("examples/shell_program.rs");
    assert!(example_text.contains("--show-on-my-desktop"));
    let example = code_only(&example_text);
    assert!(lines_with(&example, "run_product").is_empty(), "the example takes the product's gate");
    let flag = example.find("any(|argument| argument == SHOW_FLAG)").expect("the flag is required");
    let declaration = example.find("TestBinary::declare").expect("the example declares itself");
    let receipt = example.find("authority.receipt()").expect("the example prints its receipt");
    let program = example.find("run_program(").expect("the example runs the program");
    assert!(flag < declaration, "the flag is checked before anything is declared or opened");
    assert!(declaration < receipt && receipt < program, "the receipt comes before the program runs");

    // Each rule finds what it looks for.
    let second_door = sample("../shell/src/sample.rs", "let host = PresentHost::open(c, d, a);");
    assert_eq!(files_naming(&[second_door], "PresentHost::open").len(), 1);
    let declared_in_src = sample("src/sample.rs", "let t = TestBinary::declare(\"x\");");
    assert_eq!(files_naming(&[declared_in_src], "TestBinary::declare").len(), 1);
}

/// A source's code with every white space removed, so `activate : true` and `activate:` with a line break `true` read alike.
fn squeezed(source: &Source) -> String {
    source.code.split_whitespace().collect()
}

/// The sources of every test and example of the crates other than the window crate (whose own Operator-class tests
/// run on private desktops under its own scans): a test binary has no Operator window and no activation.
fn test_binary_sources() -> Vec<Source> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    ["tests", "examples", "../surface/tests", "../surface/examples", "../input/tests", "../accessibility/tests"]
        .into_iter()
        .filter(|folder| manifest.join(folder).is_dir())
        .flat_map(below)
        .filter(|source| source.path != "tests/source_scan.rs")
        .collect()
}

/// What an Operator window or a focus-taking show is, in code with its white space removed.
const OPERATOR_WINDOW_WORDS: &[&str] = &["WindowClass::Operator", "operator_root(", "launched_by_operator:true", "activate:true"];

fn operator_windows(sources: &[Source]) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        let code = squeezed(source);
        for word in OPERATOR_WINDOW_WORDS {
            if code.contains(word) {
                found.push(format!("{}: {word}", source.path));
            }
        }
    }
    found
}

/// WP012-SHELL-ROOT-WINDOW-20261127 ("Operator window, closes app"): the product's root is an Operator window that
/// takes the focus once, at the Operator's launch, with the launch authority; a test binary or example never has an
/// Operator window or a focus-taking show. Mutations caught: the product binary built from the quiet config; the
/// Operator config without the launch; a second `activate: true` show or a second `take_launch_source`; a
/// test, example or the test binaries' own path (`run_program` for a `TestBinary`) with an Operator window or a launch.
#[test]
fn the_product_root_is_an_operator_window_activated_once_and_no_test_binary_has_one() {
    let _limit = TimeLimit::new(LIMIT);
    let main = code_only(&text_of("src/main.rs"));
    assert!(!lines_with(&main, "ProgramConfig::operator_root(").is_empty(), "the product's root is the Operator's");
    assert!(lines_with(&main, "ProgramConfig::new(").is_empty(), "the product's root is not the quiet one");

    let program = source_in(&below("src"), "src/program.rs");
    let code = squeezed(&program);
    let root = block_of(&code, "pubfnoperator_root(");
    assert!(root.contains("class:WindowClass::Operator"), "{root}");
    assert!(root.contains("launched_by_operator:true"), "{root}");
    let quiet = block_of(&code, "pubfnnew(ring:Option<DiagRingWriterV2>)");
    assert!(quiet.contains("class:WindowClass::Quiet") && quiet.contains("launched_by_operator:false"), "{quiet}");
    // Only the loop's own start asks the window crate for the launch, and only when the root is an Operator window.
    assert!(code.contains("launched_by_operator=config.launched_by_operator&&config.class==WindowClass::Operator"));
    // A test authority is made quiet before anything opens.
    let run = block_of(&code, "pubfnrun_program(");
    let forced = run.find("DeviceAuthority::Test(_)").expect("a test authority is checked");
    assert!(run[forced..].starts_with("DeviceAuthority::Test(_))"), "{run}");
    assert!(run[forced..].contains("config.class=WindowClass::Quiet;config.launched_by_operator=false;"));
    assert!(forced < run.find("PresentConfig::new(").expect("the presenter's config"), "before the device opens");
    // Detached panes and Settings stay quiet.
    assert!(block_of(&code, "fnopen_detached(").contains("class:WindowClass::Quiet"));

    // Once: one show that activates, one launch taken, in the whole crate; it is the root's, with the launch source.
    let sources = below("src");
    let activating: usize = sources.iter().map(|source| squeezed(source).matches("activate:true").count()).sum();
    assert_eq!(activating, 1, "one show activates, in one place");
    let launches: usize = sources.iter().map(|source| squeezed(source).matches("take_launch_source(").count()).sum();
    assert_eq!(launches, 1, "the launch is taken in one place");
    // The exact guarded statement (MT-214 E9 R5): `launched_by_operator && true` or a dropped root test must fail.
    assert!(
        code.contains("letlaunch=(self.config.launched_by_operator&&self.root.get()==Some(window)).then(||control.take_launch_source(self.next_id())).flatten();"),
        "only the Operator-launched root takes the launch"
    );
    assert!(code.contains("Some(launch)=>control.command(&launch,window,WindowCommand::Show{activate:true})"));
    assert!(code.contains("None=>control.command(&self.source(),window,WindowCommand::Show{activate:false})"));

    // No test, example or test binary has an Operator window or a show that takes the focus.
    let tests = test_binary_sources();
    assert!(tests.len() >= 10, "the scan reads the tests and examples: {}", tests.len());
    let found = operator_windows(&tests);
    assert!(found.is_empty(), "a test binary has an Operator window or an activation: {found:#?}");
    let example = text_of("examples/shell_program.rs");
    assert!(example.contains("TestBinary::declare") && !example.contains("operator_root"));

    // Each rule finds what it looks for.
    let operator = sample("tests/sample.rs", "let spec = WindowSpec { class: WindowClass::Operator, ..s };");
    assert_eq!(operator_windows(&[operator]).len(), 1);
    let launch = sample("tests/sample.rs", "ProgramConfig { launched_by_operator : true, ..c }");
    assert_eq!(operator_windows(&[launch]).len(), 1);
    let show = sample("examples/sample.rs", "WindowCommand::Show {
 activate : true }");
    assert_eq!(operator_windows(&[show]).len(), 1);
    let door = sample("examples/sample.rs", "let c = ProgramConfig::operator_root(ring);");
    assert_eq!(operator_windows(&[door]).len(), 1);
}

fn source_in(sources: &[Source], path: &str) -> Source {
    sources
        .iter()
        .find(|source| source.path == path)
        .map(|source| Source { path: source.path.clone(), code: source.code.clone() })
        .unwrap_or_else(|| panic!("{path} is read"))
}

/// The text of the brace block that follows `head` in `code` (white space removed), through its closing brace.
fn block_of<'a>(code: &'a str, head: &str) -> &'a str {
    let at = code.find(head).unwrap_or_else(|| panic!("{head} is in the code"));
    let open = at + code[at..].find('{').expect("a block");
    let mut depth = 0;
    for (offset, character) in code[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &code[at..open + offset + 1];
                }
            }
            _ => {}
        }
    }
    panic!("{head} has no end");
}

#[test]
fn nothing_here_installs_a_panic_or_exception_handler_or_ends_the_process_itself() {
    let _limit = TimeLimit::new(LIMIT);
    let sources = below("src");
    assert!(sources.len() >= 7, "the scan reads the crate");
    let words = [
        "set_hook",
        "take_hook",
        "catch_unwind",
        "resume_unwind",
        "SetUnhandledExceptionFilter",
        "AddVectoredExceptionHandler",
        "SetErrorMode",
        "abort",
        "exit",
        "ExitProcess",
        "TerminateProcess",
        "crash_handler",
        "minidumper",
    ];
    let found = lines(&sources, &words);
    assert!(found.is_empty(), "{found:#?}");
    let hook = sample("src/sample.rs", "std::panic::set_hook(Box::new(|_| {}));");
    assert_eq!(lines(&[hook], &words).len(), 1);
    let crash_client = sample("src/sample.rs", "let _crash_handler = crash_handler::install(); minidumper::Client::with_name(n);");
    assert_eq!(lines(&[crash_client], &words).len(), 2, "one finding for each word");
    let minidump = sample("src/sample.rs", "use minidumper::Client;");
    assert_eq!(lines(&[minidump], &words).len(), 1);
}

#[test]
fn the_shell_holds_no_widget_pane_or_layout_logic() {
    let _limit = TimeLimit::new(LIMIT);
    let words = ["Widget", "Pane", "Splitter", "Layout", "Theme", "Palette", "Button", "TextEdit", "Role"];
    let found = lines(&below("src"), &words);
    assert!(found.is_empty(), "{found:#?}");
    let widget = sample("src/sample.rs", "struct Pane;");
    assert_eq!(lines(&[widget], &words).len(), 1);
}

/// What a second way to draw would be named: Direct3D, OpenGL and the operating system's software drawing, or a crate for
/// them (MT-214 AC214-9: "no D3D, OpenGL or software fallback exists in the code"). Substrings, case-insensitive, over
/// code with comments and literals blanked, so naming what is refused in a comment or a message is not having it.
const FALLBACK_RENDERERS: &[&str] = &[
    "d3d",
    "direct3d",
    "dxgi",
    "directx",
    "opengl",
    "wgl",
    "glutin",
    "softbuffer",
    "tiny_skia",
    "swiftshader",
    "llvmpipe",
    "bitblt",
    "stretchdibits",
    "dibsection",
    "software_render",
    "softwarerender",
    "software_raster",
    "cpu_render",
    "cpu_raster",
    "swrast",
    "fallback_render",
    "fallbackrender",
    "render_fallback",
];

/// The same names in a manifest's dependency names and features, with `-` as `_`.
const FALLBACK_CRATES: &[&str] = &["d3d", "direct3d", "dxgi", "directx", "opengl", "wgl", "glutin", "softbuffer", "tiny_skia"];

fn fallback_renderers(sources: &[Source]) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        for (number, line) in source.code.lines().enumerate() {
            let lower = line.to_lowercase();
            for word in FALLBACK_RENDERERS {
                if lower.contains(word) {
                    found.push(format!("{}:{}: {word}", source.path, number + 1));
                }
            }
        }
    }
    found
}

fn fallback_crates(path: &str, manifest: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (number, line) in manifest.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").to_lowercase().replace('-', "_");
        for word in FALLBACK_CRATES {
            if line.contains(word) {
                found.push(format!("{path}:{}: {word}", number + 1));
            }
        }
    }
    found
}

#[test]
fn no_direct3d_opengl_or_software_fallback_renderer_exists_in_the_shell_crates() {
    let _limit = TimeLimit::new(LIMIT);
    let mut sources = Vec::new();
    for folder in CRATE_FOLDERS {
        sources.extend(below(&format!("{folder}/src")));
    }
    assert!(sources.len() >= 60, "the scan reads the five crates: {}", sources.len());
    for known in ["window", "surface", "input", "accessibility", "shell"] {
        assert!(
            sources.iter().any(|source| source.path.starts_with(&format!("../{known}/src/"))),
            "{known} is scanned"
        );
    }
    let found = fallback_renderers(&sources);
    assert!(found.is_empty(), "a second way to draw: {found:#?}");

    // The manifests of the five crates and of the GPU owner name no such crate or feature either.
    let mut manifests = 0;
    for folder in CRATE_FOLDERS.iter().chain(&["../../handshake_native/vulkan-renderer"]) {
        let path = format!("{folder}/Cargo.toml");
        let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(&path)).expect("a manifest");
        let found = fallback_crates(&path, &text);
        assert!(found.is_empty(), "a crate or feature for a second way to draw: {found:#?}");
        manifests += 1;
    }
    assert_eq!(manifests, 6);

    // The one way to draw is the GPU owner's device, and the owner has no way to fall back either: it refuses software and
    // layered drivers, by typed reason.
    let reasons: Vec<&str> = handshake_vulkan_renderer::Unavailable::ALL.iter().map(|reason| reason.key()).collect();
    assert!(reasons.contains(&"software_device_refused") && reasons.contains(&"layered_driver_refused"), "{reasons:?}");

    // Each rule finds what it looks for, in code, and not in a comment or a message.
    let drawn = sample("src/sample.rs", "let device = D3D11CreateDevice(None, 0);");
    assert_eq!(fallback_renderers(&[drawn]).len(), 1);
    let software = sample("src/sample.rs", "fn software_rasterize(frame: &Frame) {}");
    assert_eq!(fallback_renderers(&[software]).len(), 1);
    let named = sample(
        "src/sample.rs",
        "// no Direct3D, OpenGL or software fallback
let reason = \"software device refused\";",
    );
    assert!(fallback_renderers(&[named]).is_empty(), "a comment and a message are not a renderer");
    assert_eq!(fallback_crates("Cargo.toml", "tiny-skia = \"0.11\"").len(), 1);
    assert_eq!(fallback_crates("Cargo.toml", "features = [\"Win32_Graphics_Direct3D11\"]").len(), 1, "a feature too");
    assert!(fallback_crates("Cargo.toml", "features = [\"Win32_Graphics_Gdi\"] # no Direct3D here").is_empty());
}
