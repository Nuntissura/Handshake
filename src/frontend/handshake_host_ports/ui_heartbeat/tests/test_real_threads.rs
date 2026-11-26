//! Real threads, the real clock and a real blocked loop: another thread cannot stand in for the UI thread. And the
//! tests' own limits, in processes of their own: a ring file is removed however its test ends (MT-215 E5, E7).
//!
//! Every assertion holds by construction or waits for an event with a bound; none depends on how fast the machine is.

mod support;

use std::cell::Cell;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagRingReader, DiagRingWriter, DEFAULT_CAPACITY};
use support::limits::TIMED_OUT;
use support::{RingFile, TimeLimit};
use ui_heartbeat::{
    EventLoopWake, HeartbeatCounters, ProcessClock, StartRefused, UiHeartbeat, WakeOutcome,
    WakeRefused, WakeTicket, MAX_BEAT_GAP,
};

const PATIENCE: Duration = Duration::from_secs(30);

/// The time limit of a test here: every wait in it gives up after [`PATIENCE`], and none waits more than four times.
const LIMIT: Duration = Duration::from_secs(120);

/// The timer half of a loop that owns its thread.
struct ThreadLoop {
    owner: ThreadId,
    timer: Rc<Cell<Option<(Instant, WakeTicket)>>>,
    last_ticket: Arc<AtomicU64>,
}

impl EventLoopWake for ThreadLoop {
    type Dispatched<'a> = WakeTicket;

    fn dispatched_ticket(&self, wake: &WakeTicket) -> Option<WakeTicket> {
        Some(*wake)
    }

    fn on_loop_thread(&self) -> bool {
        thread::current().id() == self.owner
    }

    fn request_wake(&self, ticket: WakeTicket, delay: Duration) -> Result<(), WakeRefused> {
        self.timer.set(Some((Instant::now() + delay, ticket)));
        self.last_ticket.store(ticket.to_raw(), Ordering::SeqCst);
        Ok(())
    }

    fn cancel_wakes(&self) {
        self.timer.set(None);
    }
}

enum Message {
    /// A wake posted by some other thread.
    Posted(WakeTicket),
    /// A handler that says it has begun and then blocks until released.
    Block {
        begun: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    },
    Quit,
}

/// A UI thread: a message queue with a timer, like a window loop.
fn ui_thread(
    writer: DiagRingWriter,
    queue: mpsc::Receiver<Message>,
    last_ticket: Arc<AtomicU64>,
) -> HeartbeatCounters {
    let timer = Rc::new(Cell::new(None));
    let heartbeat = UiHeartbeat::new(
        ProcessClock::since(Instant::now()),
        ThreadLoop {
            owner: thread::current().id(),
            timer: Rc::clone(&timer),
            last_ticket,
        },
        writer,
    );
    heartbeat.start().expect("the loop thread may start it");
    loop {
        let wait = timer.get().map_or(PATIENCE, |(due, _)| {
            due.saturating_duration_since(Instant::now())
        });
        match queue.recv_timeout(wait) {
            Ok(Message::Posted(ticket)) => {
                heartbeat.on_wake(&ticket);
            }
            Ok(Message::Block { begun, release }) => {
                heartbeat.on_loop_turn();
                let _ = begun.send(());
                let _ = release.recv();
            }
            Ok(Message::Quit) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                if let Some((_, ticket)) = timer.take() {
                    heartbeat.on_wake(&ticket);
                }
            }
        }
    }
    heartbeat.stop();
    heartbeat.counters()
}

fn counter(reader: &DiagRingReader) -> u64 {
    reader.read_heartbeat().expect("readable").counter
}

fn wait_for_counter_above(reader: &DiagRingReader, floor: u64) {
    let give_up = Instant::now() + PATIENCE;
    while counter(reader) <= floor {
        assert!(Instant::now() < give_up, "no beat above {floor}");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_blocked_ui_thread_stops_the_beat_whatever_other_threads_post() {
    let _limit = TimeLimit::new(LIMIT);
    let ring = RingFile::new("threads");
    let writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
    let reader = DiagRingReader::open(ring.path()).expect("reader");
    let (post, queue) = mpsc::channel();
    let last_ticket = Arc::new(AtomicU64::new(0));
    let ui = {
        let last_ticket = Arc::clone(&last_ticket);
        thread::spawn(move || ui_thread(writer, queue, last_ticket))
    };
    wait_for_counter_above(&reader, 2);

    let (begun_tx, begun) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    post.send(Message::Block {
        begun: begun_tx,
        release: release_rx,
    })
    .expect("the loop runs");
    begun.recv_timeout(PATIENCE).expect("the handler began");

    // The UI thread is now inside a handler. Another thread posts the very wake the port is waiting for, over and
    // over, and wakes it never asked for.
    let stuck_at = counter(&reader);
    let poster = {
        let post = post.clone();
        let awaited = WakeTicket::from_raw(last_ticket.load(Ordering::SeqCst));
        thread::spawn(move || {
            for _ in 0..5_000 {
                let _ = post.send(Message::Posted(awaited));
                let _ = post.send(Message::Posted(WakeTicket::from_raw(u64::MAX)));
            }
        })
    };
    let until = Instant::now() + MAX_BEAT_GAP * 4;
    while Instant::now() < until {
        assert_eq!(counter(&reader), stuck_at, "a blocked UI thread beat");
        thread::sleep(Duration::from_millis(10));
    }
    poster.join().expect("poster");
    assert_eq!(counter(&reader), stuck_at);

    // Released: the thread takes the posts out of its own queue, and the beat returns.
    release.send(()).expect("released");
    wait_for_counter_above(&reader, stuck_at + 1);
    post.send(Message::Quit).expect("the loop runs");
    let counters = ui.join().expect("ui thread");
    assert_eq!(
        counters.ignored_wakes, 9_999,
        "10 000 posts, of which one was the wake the port asked for"
    );
    drop(reader);
}

#[test]
fn another_thread_cannot_start_a_heartbeat_for_this_threads_loop() {
    let _limit = TimeLimit::new(LIMIT);
    let ring = RingFile::new("foreign");
    let writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
    let reader = DiagRingReader::open(ring.path()).expect("reader");
    let owner = thread::current().id();

    let refused = thread::spawn(move || {
        let heartbeat = UiHeartbeat::new(
            ProcessClock::since(Instant::now()),
            ThreadLoop {
                owner,
                timer: Rc::new(Cell::new(None)),
                last_ticket: Arc::new(AtomicU64::new(0)),
            },
            writer,
        );
        let started = heartbeat.start();
        for raw in 0..100 {
            assert!(matches!(
                heartbeat.on_wake(&WakeTicket::from_raw(raw)),
                WakeOutcome::Ignored(_)
            ));
        }
        started
    })
    .join()
    .expect("thread");

    assert_eq!(refused, Err(StartRefused::NotLoopThread));
    assert_eq!(counter(&reader), 0, "it never beat");
    drop(reader);
}

/// How [`a_test_that_holds_a_mapped_ring_and_ends_badly`] ends: `panic` or `overrun`.
const ENDING: &str = "HSK_UI_HEARTBEAT_TEST_ENDING";

/// Never run on its own: [`a_ring_file_is_removed_however_its_test_ends`] runs it in a process of its own. It maps a
/// ring file with a writer and a reader, then fails, or outlives its time limit.
#[test]
#[ignore = "run in a process of its own by a_ring_file_is_removed_however_its_test_ends"]
fn a_test_that_holds_a_mapped_ring_and_ends_badly() {
    let ending = std::env::var(ENDING).unwrap_or_default();
    let _limit = TimeLimit::new(if ending == "overrun" {
        Duration::from_secs(1)
    } else {
        LIMIT
    });
    let ring = RingFile::new("ends-badly");
    let _writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
    let _reader = DiagRingReader::open(ring.path()).expect("reader");
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "RING {}", ring.path().display());
    let _ = out.flush();
    drop(out);
    match ending.as_str() {
        "panic" => panic!("the test fails while its ring is mapped"),
        "overrun" => thread::sleep(PATIENCE),
        _ => {}
    }
}

/// What a test process left behind.
struct Ended {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs [`a_test_that_holds_a_mapped_ring_and_ends_badly`] in a process of its own, with no console and so no window
/// (HBR-QUIET), and ends it after `2 * PATIENCE` at the latest.
fn run_alone(ending: &str) -> Ended {
    let mut command = Command::new(std::env::current_exe().expect("this test binary"));
    command
        .args([
            "a_test_that_holds_a_mapped_ring_and_ends_badly",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ENDING, ending)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().expect("the test process starts");
    let read_all = |mut pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut text = String::new();
            let _ = pipe.read_to_string(&mut text);
            text
        })
    };
    let stdout = read_all(Box::new(child.stdout.take().expect("piped")));
    let stderr = read_all(Box::new(child.stderr.take().expect("piped")));
    let give_up = Instant::now() + PATIENCE * 2;
    let code = loop {
        if let Some(status) = child.try_wait().expect("our child") {
            break status.code();
        }
        if Instant::now() >= give_up {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ended {
        code,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    }
}

/// MT-215 E5, E7, CX-GIT-003: a ring file that a writer and a reader still map is gone however its test ends: when it
/// fails, and when it outlives its time limit, which ends the process as timed out, not as failed, and says so.
#[test]
fn a_ring_file_is_removed_however_its_test_ends() {
    let _limit = TimeLimit::new(LIMIT);
    for (ending, expected) in [("panic", 101), ("overrun", TIMED_OUT)] {
        let ended = run_alone(ending);
        let ring = ended
            .stdout
            .lines()
            .find_map(|line| line.strip_prefix("RING "))
            .unwrap_or_else(|| panic!("{ending}: no ring named; {}", ended.stderr))
            .to_owned();
        let left = Path::new(&ring).exists();
        // Whatever the verdict, nothing stays behind.
        let _ = std::fs::remove_file(&ring);
        assert_eq!(ended.code, Some(expected), "{ending}: {}", ended.stderr);
        assert!(!left, "{ending} left {ring}");
        if ending == "overrun" {
            assert!(
                ended.stderr.contains("TIMEOUT "),
                "an overrun says so: {}",
                ended.stderr
            );
        }
    }
}
