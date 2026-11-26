//! AC215-6, MT-215 N2: the beat path takes no lock, so it is wait-free.
//!
//! The beat path is every function a dispatched wake runs on the UI thread, from the window crate's message handler to
//! the ring: the lane's `take` and the lane's `request` for the next wake, the port's `on_wake` and what it calls, the
//! shell's frame watches, and each watched window's `frame_wanted_for`. This scans exactly those, with the window
//! crate's scanner (comments and literals blanked), for anything that takes or waits for a lock, a wait or a join. A
//! lock that creeps back into any of them fails here, with the function named. Blocking calls live in the pool-thread
//! helpers only, which the scan finds and requires (a scan that finds nothing to refuse proves nothing).

#[path = "../../window/tests/source/mod.rs"]
mod source;

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

use std::time::Duration;

use limits::TimeLimit;
use source::{below, code_only, lines_with, Source};

const LIMIT: Duration = Duration::from_secs(60);

/// The port's crate: every function a beat runs, `on_wake` and what it calls, is in these files.
const PORT_SRC: &str = "../../handshake_host_ports/ui_heartbeat/src";

/// What takes or waits for a lock, or waits otherwise, as a word of its own.
const WAITS: &[&str] = &[
    "Mutex",
    "RwLock",
    "Condvar",
    "Barrier",
    "lock",
    "sleep",
    "park",
    "join",
    "recv",
    "WaitForSingleObject",
    "WaitForThreadpoolTimerCallbacks",
    "SendMessageW",
];

/// The beat path's functions in the window crate's wake lanes: what the loop's thread runs for a dispatched wake and for
/// the port's request of the next one. `try_locked` is the one place that takes the slot, and only with `try_lock`.
const LANE_BEAT_FUNCTIONS: &[&str] = &[
    "take",
    "request",
    "cancel",
    "nudge",
    "try_locked",
    "settle",
    "drain",
    "decide",
    "perform",
    "arm",
    "pump_loop",
    "leave_to_timer",
    "number",
];

/// What the beat path must not name: the blocking accessor and the pool-thread pump. (`request` and `cancel` name their
/// pool-thread helpers, behind `if !self.on_loop_thread()`; that guard is checked below.)
const LANE_BLOCKING: &[&str] = &["locked", "pump_pool"];

/// The beat's order in the port's `on_wake`: the beat out first, then the frames, then the next wake.
fn beat_comes_first(on_wake: &str) -> bool {
    let beat = on_wake.find("self.publish(");
    let stall = on_wake.find("self.track_present_stall(");
    let request = on_wake.find("self.request_wake(");
    matches!((beat, stall, request), (Some(beat), Some(stall), Some(request)) if beat < stall && beat < request)
}

/// The code of `source` before its test module.
fn product_part(code: &str) -> &str {
    code.split("#[cfg(test)]").next().unwrap_or(code)
}

/// The bodies of every function called `name` in `code`: from its opening brace to the matching one.
fn bodies<'a>(code: &'a str, name: &str) -> Vec<&'a str> {
    let part_of_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    for (at, _) in code.match_indices(&format!("fn {name}")) {
        let after = code[at + 3 + name.len()..].chars().next();
        let before = code[..at].chars().next_back();
        if before.is_some_and(part_of_word) || after.is_some_and(part_of_word) {
            continue;
        }
        let Some(open) = code[at..].find('{').map(|open| at + open) else {
            continue;
        };
        let mut depth = 0usize;
        for (offset, c) in code[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        found.push(&code[open..open + offset + 1]);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    found
}

fn named_in(path: &str, sources: &[Source]) -> String {
    sources
        .iter()
        .find(|source| source.path == path)
        .unwrap_or_else(|| panic!("{path} is read"))
        .code
        .clone()
}

/// Every wait and blocking name found in the bodies of `names` in `code`, as `file: function: word`.
fn refused_in(file: &str, code: &str, names: &[&str], words: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for name in names {
        for body in bodies(code, name) {
            for word in words {
                if !lines_with(body, word).is_empty() {
                    found.push(format!("{file}: fn {name}: {word}"));
                }
            }
        }
    }
    found
}

/// The lock calls (`.lock(`, `.slot()`) in `body` that are not `try_lock`.
fn blocking_calls(body: &str) -> bool {
    body.contains(".lock(") || body.contains(".slot()")
}

fn lane_file(sources: &[Source]) -> String {
    named_in("../window/src/platform/wake.rs", sources)
}

fn all_lane_findings(sources: &[Source]) -> Vec<String> {
    let code = lane_file(sources);
    let code = product_part(&code);
    let mut found = refused_in("wake.rs", code, LANE_BEAT_FUNCTIONS, WAITS);
    found.extend(refused_in("wake.rs", code, LANE_BEAT_FUNCTIONS, LANE_BLOCKING));
    for name in LANE_BEAT_FUNCTIONS {
        for body in bodies(code, name) {
            if blocking_calls(body) {
                found.push(format!("wake.rs: fn {name}: a blocking lock call"));
            }
        }
    }
    found
}

#[test]
fn the_port_takes_no_lock_before_or_after_the_beat() {
    let _limit = TimeLimit::new(LIMIT);
    let port = below(PORT_SRC);
    assert!(port.len() >= 6, "the port's code is read: {}", port.len());
    let mut found = Vec::new();
    for source in &port {
        let code = product_part(&source.code);
        for word in WAITS {
            for line in lines_with(code, word) {
                found.push(format!("{}:{line}: {word}", source.path));
            }
        }
    }
    assert!(found.is_empty(), "the port waits: {found:#?}");
    let port_code = named_in(&format!("{PORT_SRC}/port.rs"), &port);
    assert_eq!(
        bodies(&port_code, "on_wake").len(),
        1,
        "the beat's entry is found, so the scan has something to refuse"
    );
}

#[test]
fn the_port_publishes_the_beat_before_it_reads_the_frames() {
    let _limit = TimeLimit::new(LIMIT);
    let port = below(PORT_SRC);
    let code = named_in(&format!("{PORT_SRC}/port.rs"), &port);
    let on_wake = bodies(&code, "on_wake").pop().expect("on_wake");
    assert!(
        beat_comes_first(on_wake),
        "the beat is out before the frames are read and before the next wake is asked for"
    );
    // The controls: the order the eval found (frames first), and a request first, are what this refuses.
    assert!(!beat_comes_first(
        "{ self.track_present_stall(now); let b = self.publish(now, r); self.request_wake(now, d); }"
    ));
    assert!(!beat_comes_first(
        "{ self.request_wake(now, d); let b = self.publish(now, r); self.track_present_stall(now); }"
    ));
    assert!(beat_comes_first(
        "{ let b = self.publish(now, r); self.track_present_stall(now); self.request_wake(now, d); }"
    ));
}

#[test]
fn the_shell_reads_each_present_wait_without_a_lock() {
    let _limit = TimeLimit::new(LIMIT);
    let shell = below("src");
    let watches = named_in("src/watches.rs", &shell);
    let heartbeat = named_in("src/heartbeat.rs", &shell);
    for (file, code) in [("watches.rs", &watches), ("heartbeat.rs", &heartbeat)] {
        let code = product_part(code);
        for word in WAITS.iter().chain(&["read"]) {
            let lines = lines_with(code, word);
            assert!(lines.is_empty(), "{file}:{lines:?}: {word}");
        }
    }
    assert!(
        product_part(&watches).contains("PresentWatch::frame_wanted_for(self)"),
        "the watch is read with the lock-free reading"
    );

    let surface = below("../surface/src");
    let watch = named_in("../surface/src/watch.rs", &surface);
    let bodies = bodies(product_part(&watch), "frame_wanted_for");
    assert_eq!(bodies.len(), 1, "the surface's lock-free reading is found");
    for word in WAITS.iter().chain(&["probe", "state", "read"]) {
        assert!(lines_with(bodies[0], word).is_empty(), "frame_wanted_for: {word}");
    }
    assert!(!blocking_calls(bodies[0]));
    assert!(bodies[0].contains("wanted_since.load("), "one atomic load");
}

#[test]
fn the_loop_threads_wake_lane_calls_take_the_slot_only_with_try_lock() {
    let _limit = TimeLimit::new(LIMIT);
    let window = below("../window/src");
    let found = all_lane_findings(&window);
    assert!(found.is_empty(), "the beat path waits for the lane's slot: {found:#?}");

    // The scan finds what it refuses: the pool-thread helpers do take the slot, waiting.
    let code = lane_file(&window);
    let code = product_part(&code);
    for name in ["take", "request", "try_locked", "pump_loop"] {
        assert_eq!(bodies(code, name).len(), 1, "{name} is found");
    }
    let try_locked = bodies(code, "try_locked").pop().unwrap();
    assert!(try_locked.contains(".try_lock()"), "the slot is taken with try_lock");
    for (name, helper) in [("request", "request_from_a_pool_thread"), ("cancel", "cancel_from_a_pool_thread")] {
        let body = bodies(code, name).pop().unwrap_or_else(|| panic!("{name} is found"));
        let guard = body.find("if !self.on_loop_thread()").unwrap_or_else(|| panic!("{name} has its guard"));
        let call = body.find(helper).unwrap_or_else(|| panic!("{name} calls {helper}"));
        assert!(guard < call, "{name}: only a thread that is not the loop's calls {helper}");
    }
    for helper in ["request_from_a_pool_thread", "cancel_from_a_pool_thread", "pump_pool", "on_timer"] {
        let body = bodies(code, helper).pop().unwrap_or_else(|| panic!("{helper} is found"));
        assert!(
            body.contains("locked(") || body.contains("pump_pool("),
            "{helper} is where a pool thread may wait"
        );
    }
    assert!(
        bodies(code, "locked").pop().expect("locked").contains("self.slot()"),
        "the waiting accessor exists, and is named only by pool threads"
    );
}

/// Each way a lock gets back onto the beat path is found, and `try_lock` is not one.
#[test]
fn each_lock_that_could_come_back_is_found_and_try_lock_is_not() {
    let _limit = TimeLimit::new(LIMIT);
    let sample = |code: &str| Source {
        path: "../window/src/platform/wake.rs".to_owned(),
        code: code_only(code),
    };
    let fine = sample("fn take() { let g = match self.slot.try_lock() { Ok(g) => g, Err(_) => return }; self.try_locked(op); }");
    assert!(all_lane_findings(&[fine]).is_empty());
    for (what, code) in [
        ("a blocking lock", "fn take() { let g = self.slot.lock().unwrap(); }"),
        ("the waiting accessor", "fn request() { let g = self.state.slot(); }"),
        ("the pool helper", "fn cancel() { self.state.locked(op); }"),
        ("a pump that waits", "fn nudge() { self.state.pump_pool(t, a); }"),
        ("a condvar", "fn perform() { let c: Condvar; }"),
        ("a join", "fn arm() { thread.join(); }"),
        ("a sent message", "fn decide() { SendMessageW(w, m, 0, 0); }"),
    ] {
        let found = all_lane_findings(&[sample(code)]);
        assert!(!found.is_empty(), "{what} is not found");
    }
    // A lock outside the beat path's functions is not the scan's business.
    let elsewhere = sample("fn on_timer() { let g = self.slot.lock(); } fn locked() { self.slot(); }");
    assert!(all_lane_findings(&[elsewhere]).is_empty());

    // The port and the shell: a lock in any word of the list is found by the same rule.
    for word in WAITS {
        let code = code_only(&format!("fn on_wake() {{ self.{word}(); }}"));
        assert!(!lines_with(&code, word).is_empty(), "{word}");
    }
}
