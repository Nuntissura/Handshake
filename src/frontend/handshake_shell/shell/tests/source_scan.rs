//! Source scans with the window crate's scanner (comments and literals blanked, so naming a call is not making one):
//! over every member of the shell's and the ports' workspaces, read from their manifests so a new member is scanned
//! without editing this file (MT-215 E3), one publisher of the ring's heartbeat slot, one heartbeat port and one ring
//! writer in product code (HB-7, E3); no window library, graphics binding, Win32 type or unsafe code in this crate's
//! code, and only its three dependencies (AC214-1, AC215-1); nothing here moves focus or the cursor or injects input
//! (HBR-QUIET); every loop of a test on a private desktop; and every ring file of a test removed by its guard (E7).

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

/// The workspaces whose members are scanned, as folders relative to this crate.
const WORKSPACES: &[&str] = &["..", "../../handshake_host_ports"];

/// Members that must be among them, so a manifest the parser misreads cannot shrink the scan unnoticed.
const KNOWN_MEMBERS: &[&str] = &[
    "../window",
    "../surface",
    "../shell",
    "../../handshake_host_ports/ui_heartbeat",
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
        ["handshake_shell_window", "handshake_shell_surface", "ui_heartbeat"]
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
        let direct = lines(
            std::slice::from_ref(&test),
            &["handshake_shell_window::run", "SetThreadDesktop", "CreateDesktopW"],
        );
        assert!(direct.is_empty(), "{direct:#?}");
    }
    assert_eq!(loops, 2, "the scan sees both loops");
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
    assert_eq!(rings, 2, "the scan sees both tests that make a ring");

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
