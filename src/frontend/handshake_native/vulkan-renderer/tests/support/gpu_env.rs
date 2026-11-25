//! The surroundings of a test on the machine's real Vulkan device: a process of its own, a switch that turns a skip
//! into a failure, and nothing left behind.
//!
//! `HANDSHAKE_REQUIRE_VULKAN_GPU` set to anything but `0`: a machine without an acceptable device fails these tests
//! instead of skipping them with one `HSK_GPU_SKIP` line.
//!
//! A process of its own, because the owner slot is the process's: a case that leaves a device alive, on purpose or by
//! failing, takes nothing from the next one. And because what a driver leaves behind can only be removed once its
//! process is gone. That is `%LOCALAPPDATA%\D3DSCache\<hash>`, the shader cache of the Direct3D 12 runtime, whose
//! files name the executable they belong to. The owner creates no Direct3D device, but the Vulkan loader loads every
//! installed driver into the process, and Microsoft's Direct3D mapping driver (Dozen, manifest `dzn_icd.*.json`)
//! creates one for each adapter it enumerates. The owner refuses that driver anyway, so these processes have the
//! loader leave it out with its documented `VK_LOADER_DRIVERS_DISABLE` filter. A cache of this executable that
//! appears all the same is removed and reported in one `HSK_GPU_LEFTOVER` line, and fails the test when it stays.

use super::EventLog;
use handshake_vulkan_renderer::{GpuOwner, QuotaPolicy, Unavailable};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

const REQUIRE_GPU: &str = "HANDSHAKE_REQUIRE_VULKAN_GPU";
const DRIVER_FILTER: &str = "VK_LOADER_DRIVERS_DISABLE";
const IN_CHILD: &str = "HANDSHAKE_VULKAN_TEST_PROCESS";
/// A test process that takes longer hangs.
const DEADLINE: Duration = Duration::from_secs(120);
/// The executable path is in one of the first pages of a cache file.
const CACHE_FILE_HEAD: u64 = 256 * 1024;

static TURN: Mutex<()> = Mutex::new(());
static BEFORE: OnceLock<Before> = OnceLock::new();

/// Run the calling test in a process of its own: in the process that cargo started this returns from the test.
macro_rules! in_own_process {
    () => {{
        fn here() {}
        if ran_in_its_own_process(std::any::type_name_of_val(&here)) {
            return;
        }
    }};
}
pub(crate) use in_own_process;

/// `true` after the test `here` is in passed in a process of its own; `false` in that process, which then runs it.
pub fn ran_in_its_own_process(here: &str) -> bool {
    if std::env::var_os(IN_CHILD).is_some() {
        return false;
    }
    let _one_at_a_time = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let before = BEFORE.get_or_init(prepare);
    // `crate::module::test::here` is `module::test` to the test harness.
    let path = here.strip_suffix("::here").unwrap_or(here);
    let name = path.split_once("::").map_or(path, |(_, name)| name);
    let (passed, output) = run_child(name);
    print!("{output}");
    let kept = remove_new_cache(before);
    assert!(passed, "{name} failed in its own process");
    assert!(output.contains("1 passed"), "{name} did not run");
    assert!(kept.is_empty(), "left behind: {kept:?}");
    true
}

/// Run this executable for the one test `name`. Whether it succeeded, and what it printed.
fn run_child(name: &str) -> (bool, String) {
    let exe = std::env::current_exe().expect("this executable");
    let mut command = Command::new(exe);
    command
        .args([name, "--exact", "--nocapture", "--test-threads=1"])
        .env(IN_CHILD, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped());
    if std::env::var_os(DRIVER_FILTER).is_none() {
        command.env(DRIVER_FILTER, "dzn_icd*");
    }
    let mut child = command.spawn().expect("starts");
    let mut stdout = child.stdout.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut output = String::new();
        let _ = stdout.read_to_string(&mut output);
        output
    });
    let end = Instant::now() + DEADLINE;
    let passed = loop {
        match child.try_wait().expect("own child") {
            Some(status) => break status.success(),
            None if Instant::now() >= end => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    (passed, reader.join().unwrap_or_default())
}

/// Whether this machine must have an acceptable device.
fn gpu_required() -> bool {
    std::env::var_os(REQUIRE_GPU).is_some_and(|value| !value.is_empty() && value != "0")
}

/// The owner on this machine's device, or `None` after reporting why there is none.
pub fn open(policy: QuotaPolicy) -> Option<(GpuOwner, Arc<EventLog>)> {
    let log = Arc::new(EventLog::default());
    match GpuOwner::open(policy, log.clone()) {
        Ok(owner) => Some((owner, log)),
        Err(
            reason @ (Unavailable::LoaderMissing
            | Unavailable::InstanceRefused
            | Unavailable::ApiBelowBaseline
            | Unavailable::NoPhysicalDevice
            | Unavailable::SoftwareDeviceRefused
            | Unavailable::LayeredDriverRefused
            | Unavailable::NoSuitableDevice
            | Unavailable::FeatureMissing),
        ) => {
            assert!(
                !gpu_required(),
                "{REQUIRE_GPU} is set, but there is no acceptable device: {}",
                reason.key()
            );
            println!(
                "HSK_GPU_SKIP reason={} number={}",
                reason.key(),
                reason.number()
            );
            None
        }
        Err(reason) => panic!("a device exists but did not open: {}", reason.key()),
    }
}

/// What was there before the first test process.
struct Before {
    cache: Option<PathBuf>,
    cache_existed: bool,
    folders: BTreeSet<OsString>,
}

fn prepare() -> Before {
    let cache = std::env::var_os("LOCALAPPDATA").map(|base| Path::new(&base).join("D3DSCache"));
    let cache_existed = cache.as_deref().is_some_and(Path::is_dir);
    for folder in folders_of(cache.as_deref()) {
        // An earlier run of this very executable.
        if belongs_to_this_executable(&folder) {
            let _ = std::fs::remove_dir_all(&folder);
        }
    }
    let folders = folders_of(cache.as_deref())
        .iter()
        .filter_map(|folder| folder.file_name().map(OsString::from))
        .collect();
    Before {
        cache,
        cache_existed,
        folders,
    }
}

fn folders_of(cache: Option<&Path>) -> Vec<PathBuf> {
    let Some(entries) = cache.and_then(|cache| std::fs::read_dir(cache).ok()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

/// Whether a cache file in `folder` names this executable.
fn belongs_to_this_executable(folder: &Path) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(name) = exe
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
    else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(folder) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let mut head = Vec::new();
        std::fs::File::open(entry.path())
            .and_then(|file| file.take(CACHE_FILE_HEAD).read_to_end(&mut head))
            .is_ok()
            && head
                .windows(name.len())
                .any(|window| window == name.as_bytes())
    })
}

/// Remove what test processes of this executable left in the cache since `before`. Returns what stays.
fn remove_new_cache(before: &Before) -> Vec<PathBuf> {
    let mut kept = Vec::new();
    for folder in folders_of(before.cache.as_deref()) {
        let is_new = folder
            .file_name()
            .is_some_and(|name| !before.folders.contains(name));
        if !is_new || !belongs_to_this_executable(&folder) {
            continue;
        }
        // The files are released shortly after their process ended.
        let removed = (0..40).any(|_| {
            std::fs::remove_dir_all(&folder).is_ok() || {
                std::thread::sleep(Duration::from_millis(50));
                false
            }
        });
        println!(
            "HSK_GPU_LEFTOVER removed={removed} path={}",
            folder.display()
        );
        if !removed {
            kept.push(folder);
        }
    }
    if let (Some(cache), false) = (before.cache.as_deref(), before.cache_existed) {
        // Only when it is empty.
        let _ = std::fs::remove_dir(cache);
    }
    kept
}
