//! Wakes of a real loop: on its thread, in order, when due, through a full queue, through more posts another loop
//! throws away than go out before a sign that the loop takes posts out of its queue, not from a message anybody else
//! posts or sends (a genuine post sent back from another thread, and the posts of dropped lanes whose number is used
//! again, included), with no more than four posts while the loop waits in a SendMessage to a hung thread, inside loops
//! that run inside the host, and never after `finish`. One test, because a process has one loop.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use handshake_shell_window::{
    Control, Event, LaneWaker, LoopConfig, LoopError, WakeLane, WakeRefused,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DialogBoxIndirectParamW, DispatchMessageW, EndDialog, FindWindowW, GetMessageW, KillTimer,
    PeekMessageW, PostMessageW, SendMessageW, SetTimer, TranslateMessage, DLGTEMPLATE, MSG,
    PM_REMOVE, WM_APP, WM_DISPLAYCHANGE, WM_INITDIALOG, WM_NULL, WM_TIMER, WS_POPUP,
};

use support::{
    on_private_desktop, run_script, send_from_another_thread, send_to_a_thread_that_hangs, wide,
    Decoy, Log,
};

const LIMIT: Duration = Duration::from_secs(120);
const DIALOG_RESULT: isize = 7;

/// The crate's wake message (`src/platform/wake.rs`), as another program would post it.
const WM_WAKE: u32 = WM_APP + 1;

/// When an unanswered post is posted again, the first time (`wake_slot::REPEAT`).
const REPOSTED_AFTER: Duration = Duration::from_millis(500);

/// Posts that go out unanswered before the next waits for a sign that the loop takes posts out (`BLIND_POSTS`).
const BLIND_POSTS: usize = 4;

/// How long a foreign loop throws the posts away: five of them, at 0, 0.5, 1.5, 3.5 and 7.5 s, the last after the
/// loop answered a probe while no post waited (MT-214 E3 named test: three were fewer than BLIND_POSTS).
const THROWN_AWAY_FOR: Duration = Duration::from_millis(8_000);

/// How long the loop waits in a SendMessage to a hung thread: past the probe after the fourth post, at 7.5 s.
const HUNG_FOR: Duration = Duration::from_millis(9_000);

fn service_window() -> HWND {
    let service = wide("hsk_shell_service");
    // SAFETY: the name outlives the call; read only.
    let hwnd = unsafe { FindWindowW(service.as_ptr(), ptr::null()) };
    assert_ne!(hwnd, 0);
    hwnd
}

thread_local! {
    static DIALOG: Cell<HWND> = const { Cell::new(0) };
}

unsafe extern "system" fn dialog(hwnd: HWND, message: u32, _: WPARAM, _: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        DIALOG.set(hwnd);
    }
    0
}

/// A dialog without controls, as the memory layout the operating system reads: template, no menu, no class, no title.
#[repr(C, align(4))]
struct EmptyDialog {
    template: DLGTEMPLATE,
    ends: [u16; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nested {
    No,
    PlainLoop,
    Dialog,
}

struct State {
    thread: ThreadId,
    /// The number lane a and the dropped lanes before it were opened under, as their posts carry it.
    lane_number: Cell<WPARAM>,
    a: RefCell<Option<WakeLane>>,
    b: RefCell<Option<WakeLane>>,
    waker: RefCell<Option<LaneWaker>>,
    requested_at: Cell<Instant>,
    decoy: RefCell<Option<Decoy>>,
    wakes: Log<u64>,
    events: Log<&'static str>,
    in_event: Cell<u32>,
    nested: Cell<Nested>,
    nested_wakes: Cell<u32>,
    plain_loop_ends: Cell<bool>,
}

impl State {
    fn request(&self, value: u64, delay_ms: u64) {
        self.requested_at.set(Instant::now());
        let lane = self.a.borrow();
        lane.as_ref()
            .unwrap()
            .request(value, Duration::from_millis(delay_ms))
            .unwrap();
    }

    /// Fill this thread's message queue until the operating system takes no more.
    fn fill_queue(&self) {
        let decoy = self.decoy.borrow().as_ref().unwrap().0;
        for _ in 0..200_000 {
            // SAFETY: a message that does nothing, to a window of this thread.
            if unsafe { PostMessageW(decoy, WM_NULL, 0, 0) } == 0 {
                return;
            }
        }
        panic!("the queue never filled, so the refused post is not tested");
    }

    /// Three wakes have to arrive inside each nested loop.
    fn nested_wake(&self) -> bool {
        let count = self.nested_wakes.get() + 1;
        self.nested_wakes.set(count);
        count >= 3
    }

    fn plain_loop(&self) {
        self.nested.set(Nested::PlainLoop);
        self.nested_wakes.set(0);
        self.request(30, 5);
        while !self.plain_loop_ends.get() {
            // SAFETY: an ordinary message loop of this thread.
            unsafe {
                let mut message: MSG = std::mem::zeroed();
                assert!(GetMessageW(&mut message, 0, 0, 0) > 0);
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        self.nested.set(Nested::No);
        assert_eq!(self.nested_wakes.get(), 3);
    }

    /// Every wake post in the queue, taken out.
    fn take_wake_posts(&self) -> Vec<(WPARAM, LPARAM)> {
        let service = service_window();
        let mut posts = Vec::new();
        // SAFETY: a message structure of this function; only wake messages of the loop's own window are taken.
        unsafe {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, service, WM_WAKE, WM_WAKE, PM_REMOVE) != 0 {
                posts.push((message.wParam, message.lParam));
            }
        }
        posts
    }

    /// MT-214 E3 named test: the posts the 200 dropped lanes left, under the number lane a has now, stay in the queue,
    /// and lane a's own first post goes. A stale post that could settle a post of lane a would dispatch wake 1 at once;
    /// without one, lane a posts again after [`REPOSTED_AFTER`] (`wake`, value 1).
    fn only_stale_posts_left(&self) {
        let mut posts = self.take_wake_posts();
        let of = |posts: &[(WPARAM, LPARAM)], number: WPARAM| {
            posts.iter().filter(|(lane, _)| *lane == number).count()
        };
        let number = posts
            .iter()
            .map(|(lane, _)| *lane)
            .find(|number| of(&posts, *number) > 200)
            .expect("the posts of the 200 dropped lanes and lane a's, under one number");
        self.lane_number.set(number);
        let own = posts
            .iter()
            .rposition(|(lane, _)| *lane == number)
            .expect("lane a's post, the newest of its number");
        posts.remove(own);
        assert_eq!(of(&posts, number), 200, "the stale posts");
        let service = service_window();
        for (lane, token) in posts {
            // SAFETY: plain values, to a window of this thread: the posts as they were, in their order.
            assert_ne!(unsafe { PostMessageW(service, WM_WAKE, lane, token) }, 0);
        }
    }

    /// The posts of lane a in the queue, taken out.
    fn take_posts_of_lane_a(&self) -> usize {
        let number = self.lane_number.get();
        self.take_wake_posts()
            .iter()
            .filter(|(lane, _)| *lane == number)
            .count()
    }

    /// A loop that takes the wake posts out of the queue and throws them away, as somebody else's loop may.
    fn throw_posts_away(&self) -> usize {
        let service = service_window();
        let until = Instant::now() + THROWN_AWAY_FOR;
        let mut thrown = 0;
        while Instant::now() < until {
            // SAFETY: a message structure of this function; only wake messages of the loop's own window are taken.
            unsafe {
                let mut message: MSG = std::mem::zeroed();
                while PeekMessageW(&mut message, service, WM_WAKE, WM_WAKE, PM_REMOVE) != 0 {
                    thrown += 1;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        thrown
    }

    /// A loop the host runs after `finish`, ended by a timer of the thread. A wake that is posted meanwhile must not
    /// reach the host.
    fn loop_after_finish(&self) {
        let lane = self.b.borrow();
        lane.as_ref().unwrap().request(60, Duration::ZERO).unwrap();
        // SAFETY: a timer of this thread, ended below; an ordinary message loop of this thread.
        unsafe {
            let timer = SetTimer(0, 0, 300, None);
            assert_ne!(timer, 0);
            loop {
                let mut message: MSG = std::mem::zeroed();
                assert!(GetMessageW(&mut message, 0, 0, 0) > 0);
                if message.message == WM_TIMER && message.hwnd == 0 {
                    KillTimer(0, timer);
                    break;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn modal_dialog(&self) {
        self.nested.set(Nested::Dialog);
        self.nested_wakes.set(0);
        self.request(40, 5);
        let empty = EmptyDialog {
            template: DLGTEMPLATE {
                style: WS_POPUP,
                dwExtendedStyle: 0,
                cdit: 0,
                x: 0,
                y: 0,
                cx: 20,
                cy: 20,
            },
            ends: [0; 3],
        };
        // SAFETY: a complete template that outlives the call, which returns when the dialog ends.
        let result =
            unsafe { DialogBoxIndirectParamW(0, ptr::addr_of!(empty).cast(), 0, Some(dialog), 0) };
        self.nested.set(Nested::No);
        assert_eq!(result, DIALOG_RESULT, "ended by the wake");
        assert_eq!(self.nested_wakes.get(), 3);
    }

    fn event(&self, control: &Control, event: Event) {
        assert_eq!(
            self.in_event.replace(1),
            0,
            "an event handler was entered twice"
        );
        match event {
            Event::LoopStarted => {
                *self.decoy.borrow_mut() = Some(Decoy::create());
                // More lanes than may be open at once, each dropped with its wake posted: the numbers are used again
                // (MT-214 R2-N4), and none of the posts left in the queue dispatches a wake of the lane that has the
                // number now.
                let mut first = None;
                for _ in 0..200 {
                    let lane = control.open_wake_lane().unwrap();
                    first.get_or_insert(lane.id());
                    lane.request(99, Duration::ZERO).unwrap();
                }
                let a = control.open_wake_lane().unwrap();
                let b = control.open_wake_lane().unwrap();
                assert_eq!(Some(a.id()), first, "the number of a dropped lane");
                assert!(a.on_loop_thread());
                assert_ne!(a.id(), b.id());
                *self.waker.borrow_mut() = Some(a.waker());
                *self.a.borrow_mut() = Some(a);
                *self.b.borrow_mut() = Some(b);
                self.request(1, 0);
                self.only_stale_posts_left();
                // Wake messages nobody posted from a lane, sent at once (from this thread they run here) and posted
                // for later, for every lane there is and one more. None of them may dispatch a wake: the wake of
                // lane a is due, and would come from inside this handler.
                let service = service_window();
                for lane in 0..4 {
                    for forged in [0, 1, 2, 3, isize::MAX, -1] {
                        // SAFETY: plain values, to a window of this thread.
                        unsafe {
                            SendMessageW(service, WM_WAKE, lane, forged);
                            assert_ne!(PostMessageW(service, WM_WAKE, lane, forged), 0);
                        }
                    }
                }
                assert!(self.wakes.all().is_empty(), "never from inside the request");
                self.events.add("started");
            }
            Event::ShutdownBegan => {
                self.plain_loop();
                // An event that comes up now has to wait for this handler.
                // SAFETY: plain values, to a window of this thread.
                assert_ne!(
                    unsafe { PostMessageW(service_window(), WM_DISPLAYCHANGE, 0, 0) },
                    0
                );
                self.modal_dialog();
                self.events.add("shutdown");
                control.finish(5).unwrap();
                self.loop_after_finish();
            }
            Event::MonitorsChanged => self.events.add("monitors"),
            Event::LoopEnding => {
                self.events.add("ending");
                self.decoy.borrow_mut().take();
            }
            other => panic!("unexpected {other:?}"),
        }
        self.in_event.set(0);
    }

    fn wake(&self, control: &Control, value: u64, inside_host: bool) {
        assert_eq!(std::thread::current().id(), self.thread);
        assert_eq!(inside_host, self.nested.get() != Nested::No, "wake {value}");
        self.wakes.add(value);
        match value {
            1 => {
                assert!(
                    self.requested_at.get().elapsed() >= REPOSTED_AFTER,
                    "posted again, not dispatched by a post of a dropped lane"
                );
                self.request(2, 60);
            }
            2 => {
                assert!(
                    self.requested_at.get().elapsed() >= Duration::from_millis(60),
                    "not before it is due"
                );
                let waker = self.waker.borrow().clone().unwrap();
                // The loop is in here meanwhile, so all three are under way at once.
                std::thread::spawn(move || {
                    for value in [10, 11, 12] {
                        waker.wake(value).unwrap();
                    }
                })
                .join()
                .unwrap();
            }
            12 => {
                self.request(13, 30);
                self.a.borrow().as_ref().unwrap().cancel();
                let lane = self.b.borrow();
                lane.as_ref()
                    .unwrap()
                    .request(14, Duration::from_millis(150))
                    .unwrap();
            }
            14 => {
                // Every post of the next wake is thrown away for a while, more than go out without a sign that the
                // loop takes posts out; one after that still dispatches it.
                self.request(15, 0);
                let thrown = self.throw_posts_away();
                assert!(thrown > BLIND_POSTS, "{thrown} posts thrown away");
            }
            15 => {
                assert!(
                    self.requested_at.get().elapsed() >= THROWN_AWAY_FOR,
                    "posted again after the posts were thrown away"
                );
                // The genuine post of the next wake, taken out of the queue and sent back from another thread,
                // dispatches nothing: a wake comes out of the loop's own queue (InSendMessageEx).
                self.request(16, 0);
                let refused = control.report().sent_wakes_refused;
                let posts = self.take_wake_posts();
                assert!(!posts.is_empty(), "the post of wake 16");
                let sent = posts
                    .iter()
                    .map(|(wparam, lparam)| (WM_WAKE, *wparam, *lparam))
                    .collect();
                send_from_another_thread(service_window(), sent);
                assert_eq!(
                    control.report().sent_wakes_refused,
                    refused + posts.len() as u64
                );
                assert!(!self.wakes.all().contains(&16), "not from the sent copy");
            }
            16 => {
                // Posted again, since its post was taken out. MT-214 E3-F4: while the loop waits in a SendMessage to a
                // hung thread it answers probes, and takes nothing out of its queue: the next wake gets no post more
                // than four meanwhile. They are thrown away afterwards; a probe then brings the wake.
                self.request(17, 0);
                send_to_a_thread_that_hangs(HUNG_FOR);
                assert_eq!(self.take_posts_of_lane_a(), BLIND_POSTS, "posts while the loop waited");
            }
            17 => {
                // The post of the next wake is refused, and so is every other until the loop has caught up.
                self.fill_queue();
                self.request(20, 0);
            }
            20 => {
                assert_eq!(
                    control.finish(1).unwrap_err().code(),
                    "window.shutdown_not_begun"
                );
                control.begin_shutdown().unwrap();
                assert_eq!(
                    control.begin_shutdown().unwrap_err().code(),
                    "window.shutting_down"
                );
                // Dispatched during shutdown, by the loop that the shutdown handler runs.
                let lane = self.b.borrow();
                lane.as_ref().unwrap().request(21, Duration::ZERO).unwrap();
            }
            21 => {}
            30 => {
                if self.nested_wake() {
                    self.plain_loop_ends.set(true);
                } else {
                    self.request(30, 5);
                }
            }
            40 => {
                if self.nested_wake() {
                    // SAFETY: the dialog that is open on this thread.
                    unsafe { EndDialog(DIALOG.get(), DIALOG_RESULT) };
                } else {
                    self.request(40, 5);
                }
            }
            60 => panic!("a wake reached the host after finish"),
            other => panic!("wake {other} was replaced or cancelled"),
        }
    }
}

#[test]
fn a_wake_is_dispatched_by_every_loop_of_its_thread_and_by_nothing_else() {
    on_private_desktop(LIMIT, || {
        let state = Rc::new(State {
            thread: std::thread::current().id(),
            lane_number: Cell::new(0),
            a: RefCell::default(),
            b: RefCell::default(),
            waker: RefCell::default(),
            requested_at: Cell::new(Instant::now()),
            decoy: RefCell::default(),
            wakes: Log::new(),
            events: Log::new(),
            in_event: Cell::new(0),
            nested: Cell::new(Nested::No),
            nested_wakes: Cell::new(0),
            plain_loop_ends: Cell::new(false),
        });
        let (events, wakes) = (Rc::clone(&state), Rc::clone(&state));
        let report = run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            move |control, wake| wakes.wake(control, wake.value(), wake.inside_host_call()),
        )
        .unwrap();

        let seen = state.wakes.all();
        assert_eq!(&seen[..9], &[1, 2, 12, 14, 15, 16, 17, 20, 21], "{seen:?}");
        assert!(!seen.contains(&60));
        assert_eq!(state.wakes.count(&30), 3);
        assert_eq!(state.wakes.count(&40), 3);
        let events = state.events.all();
        let own: Vec<_> = events
            .iter()
            .filter(|event| **event != "monitors")
            .collect();
        assert_eq!(own, [&"started", &"shutdown", &"ending"]);
        let waited = events
            .iter()
            .skip_while(|event| **event != "shutdown")
            .any(|event| *event == "monitors");
        assert!(
            waited,
            "the event that came up in the handler arrived after it: {events:?}"
        );
        assert_eq!(report.exit_code, 5);
        assert_eq!(report.wakes, seen.len() as u64);

        // After the loop, from this thread and from another.
        let waker = state.waker.borrow().clone().unwrap();
        assert_eq!(waker.wake(1), Err(WakeRefused::Closed));
        assert_eq!(
            std::thread::spawn(move || waker.wake(1)).join().unwrap(),
            Err(WakeRefused::Closed)
        );
        let lane = state.a.borrow_mut().take().unwrap();
        assert_eq!(lane.request(1, Duration::ZERO), Err(WakeRefused::Closed));
        assert_eq!(
            run_script(LoopConfig::default(), |_, _| {}, |_, _| {}),
            Err(LoopError::AlreadyCreated)
        );
    });
}
