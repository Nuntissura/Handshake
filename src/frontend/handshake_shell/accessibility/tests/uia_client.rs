//! AC214-7 through the asset adapter and a real UI Automation client, on a real loop with quiet windows on a private
//! desktop: two windows, each with a tree of its own under the same keys, read by a client on another thread of the
//! same desktop; then the keyboard focus, a scale change, an Invoke and a SetFocus on a window.
//!
//! Reading is proven not to activate, focus, show or reorder either window: the window crate's guard refuses no
//! activation meanwhile, no quiet window breaks, no focus or display change is reported, the windows keep their place
//! in the Z order, and the watches of the support module (Operator's desktop, private desktop) see no window of the
//! process active or focused. One loop per process, so this is a target of its own.
#![cfg(windows)]
#![allow(unsafe_code)]

#[path = "../../window/tests/support/mod.rs"]
mod support;

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::thread::JoinHandle;
use std::time::Duration;

use handshake_shell_accessibility::accesskit::{Action, Node, Rect, Role};
use handshake_shell_accessibility::{
    text_run, AccessRequest, AccessTarget, Accessibility, AttachRefusal, NodeKey, SemanticFrame,
    SemanticNode,
};
use handshake_shell_window::{
    CommandSource, Control, DispatchedWake, Display, Event, LogicalSize, LoopConfig, Origin,
    PhysicalPosition, Placement, Refusal, RunReport, ScaleFactor, WakeLane, WindowClass,
    WindowCommand, WindowEvent, WindowKey, WindowSpec,
};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
    TreeScope_Children, UIA_ButtonControlTypeId, UIA_CONTROLTYPE_ID, UIA_EditControlTypeId,
    UIA_InvokePatternId, UIA_TitleBarControlTypeId,
};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindow, PostMessageW, GW_HWNDNEXT, GW_HWNDPREV, WM_KILLFOCUS, WM_NCACTIVATE, WM_SETFOCUS,
};

use support::{
    must_stay_inactive, on_private_desktop, run_script, spawn_on_this_desktop, stop_watching, Log,
};

const SAVE: NodeKey = NodeKey(1);
const NAME: NodeKey = NodeKey(2);
const RUN: NodeKey = NodeKey(3);
const SAVE_BOUNDS: Rect = Rect::new(10.0, 10.0, 90.0, 40.0);

fn button(key: NodeKey, label: &str, author: &str, bounds: Rect) -> SemanticNode {
    let mut node = Node::new(Role::Button);
    node.set_label(label);
    node.set_author_id(author);
    node.set_bounds(bounds);
    node.add_action(Action::Click);
    node.add_action(Action::Focus);
    SemanticNode::new(key, node)
}

/// The first window: a button and a text field with one run.
fn first(focus: Option<NodeKey>) -> SemanticFrame {
    let mut field = Node::new(Role::TextInput);
    field.set_label("Name");
    field.set_author_id("name");
    field.set_bounds(Rect::new(10.0, 50.0, 190.0, 80.0));
    field.add_action(Action::Focus);
    let mut field = SemanticNode::new(NAME, field);
    field.children = vec![RUN];
    let run = text_run("Ada", Rect::new(4.0, 4.0, 40.0, 26.0), None).unwrap();
    SemanticFrame {
        name: Some("first window".to_owned()),
        top: vec![SAVE, NAME],
        nodes: vec![
            button(SAVE, "Save", "save", SAVE_BOUNDS),
            field,
            SemanticNode::new(RUN, run),
        ],
        focus,
    }
}

/// The second window: another button under the same key.
fn second() -> SemanticFrame {
    SemanticFrame {
        name: Some("second window".to_owned()),
        top: vec![SAVE],
        nodes: vec![button(SAVE, "Open", "open", Rect::new(20.0, 20.0, 120.0, 60.0))],
        focus: None,
    }
}

/// One element as the client read it.
#[derive(Debug)]
struct Element {
    name: String,
    automation_id: String,
    control_type: UIA_CONTROLTYPE_ID,
    rect: RECT,
    focused: bool,
}

/// A window's root and its children as the client read them.
#[derive(Debug)]
struct Reading {
    name: String,
    children: Vec<Element>,
}

impl Reading {
    fn child(&self, name: &str) -> &Element {
        self.children
            .iter()
            .find(|child| child.name == name)
            .unwrap_or_else(|| panic!("{name} in {self:?}"))
    }
}

/// Run `work` with a UI Automation client of this thread (MTA, as UI Automation asks of clients).
fn with_automation<T>(
    work: impl FnOnce(&IUIAutomation) -> windows::core::Result<T>,
) -> Result<T, String> {
    // SAFETY: the thread's own COM, ended below.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .map_err(|error| format!("CoInitializeEx: {error}"))?;
    let result = {
        // SAFETY: the class and context of the system's UI Automation client.
        let automation: windows::core::Result<IUIAutomation> =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) };
        automation
            .and_then(|automation| work(&automation))
            .map_err(|error| error.to_string())
    };
    // SAFETY: after every interface of this thread was released.
    unsafe { CoUninitialize() };
    result
}

fn root_of(automation: &IUIAutomation, hwnd: isize) -> windows::core::Result<IUIAutomationElement> {
    // SAFETY: a window of the test's loop thread, alive while the client runs.
    unsafe { automation.ElementFromHandle(HWND(hwnd as *mut c_void)) }
}

/// Read a window's root and the provider's own children: name, automation id, control type, bounds, keyboard focus.
/// UI Automation puts the window's non-client title bar (control type 50037, no name) before them; it is the system's,
/// not the provider's, so it is left out (MT-214 E7-F1).
fn read(automation: &IUIAutomation, hwnd: isize) -> windows::core::Result<Reading> {
    let root = root_of(automation, hwnd)?;
    // SAFETY: calls on interfaces of this thread.
    unsafe {
        let all = automation.CreateTrueCondition()?;
        let found = root.FindAll(TreeScope_Children, &all)?;
        let mut children = Vec::new();
        for at in 0..found.Length()? {
            let element = found.GetElement(at)?;
            if element.CurrentControlType()? == UIA_TitleBarControlTypeId {
                continue;
            }
            children.push(Element {
                name: element.CurrentName()?.to_string(),
                automation_id: element.CurrentAutomationId()?.to_string(),
                control_type: element.CurrentControlType()?,
                rect: element.CurrentBoundingRectangle()?,
                focused: element.CurrentHasKeyboardFocus()?.as_bool(),
            });
        }
        Ok(Reading {
            name: root.CurrentName()?.to_string(),
            children,
        })
    }
}

/// Read the first window, then Invoke its Save button, SetFocus the second window's root, and SetFocus its Open button.
fn read_and_act(first: isize, second: isize) -> Result<Vec<Reading>, String> {
    with_automation(|automation| {
        let reading = read(automation, first)?;
        let root = root_of(automation, first)?;
        // SAFETY: calls on interfaces of this thread.
        unsafe {
            let all = automation.CreateTrueCondition()?;
            let found = root.FindAll(TreeScope_Children, &all)?;
            for at in 0..found.Length()? {
                let element = found.GetElement(at)?;
                if element.CurrentName()? == "Save" {
                    let invoke: IUIAutomationInvokePattern =
                        element.GetCurrentPatternAs(UIA_InvokePatternId)?;
                    invoke.Invoke()?;
                }
            }
            root_of(automation, second)?.SetFocus()?;
            let root = root_of(automation, second)?;
            let found = root.FindAll(TreeScope_Children, &all)?;
            for at in 0..found.Length()? {
                let element = found.GetElement(at)?;
                if element.CurrentName()? == "Open" {
                    element.SetFocus()?;
                }
            }
        }
        Ok(vec![reading])
    })
}

type Client = JoinHandle<Result<Vec<Reading>, String>>;

/// Each of the two windows' neighbours in the Z order: the one before and the one after.
type Places = [(isize, isize); 2];

#[derive(Default)]
struct State {
    hub: RefCell<Accessibility>,
    steps: RefCell<Option<WakeLane>>,
    notices: RefCell<Option<WakeLane>>,
    /// The two windows with trees, and their handles.
    windows: RefCell<Vec<(WindowKey, isize)>>,
    /// A window shown before anything was attached, then hidden again.
    spare: Cell<Option<(WindowKey, isize)>>,
    spare_hidden: Cell<bool>,
    spare_done: Cell<bool>,
    shown: Cell<u32>,
    destroyed: Cell<u32>,
    step: Cell<u32>,
    commands: Cell<u64>,
    client: RefCell<Option<Client>>,
    requests: RefCell<Vec<AccessRequest>>,
    /// Focus and display changes of the two windows, as the loop reported them.
    seen: Log<String>,
    /// The guard's refused activations, and each window's neighbours in the Z order, before the client read.
    before: Cell<Option<(u64, Places)>>,
}

impl State {
    fn source(&self) -> CommandSource {
        self.commands.set(self.commands.get() + 1);
        CommandSource::host(self.commands.get())
    }

    fn wake_in(&self, delay: Duration) {
        let lane = self.steps.borrow();
        lane.as_ref().unwrap().request(0, delay).unwrap();
    }

    fn next(&self) {
        self.step.set(self.step.get() + 1);
        self.wake_in(Duration::from_millis(30));
    }

    fn window(&self, at: usize) -> (WindowKey, isize) {
        self.windows.borrow()[at]
    }

    fn create(&self, control: &Control, x: i32, y: i32) -> WindowKey {
        let spec = WindowSpec {
            title: "test".to_owned(),
            inner: LogicalSize::new(220.0, 120.0),
            min_inner: None,
            placement: Placement::At(PhysicalPosition::new(x, y)),
            class: WindowClass::Quiet,
        };
        control.create_window(&self.source(), spec).unwrap()
    }

    fn take_requests(&self, control: &Control) {
        let taken = self.hub.borrow_mut().take_requests(control.now());
        self.requests.borrow_mut().extend(taken);
    }

    /// Each window's neighbours in the Z order.
    fn places(&self) -> Places {
        [0, 1].map(|at| {
            let hwnd = self.window(at).1;
            // SAFETY: reads of windows of this thread.
            unsafe { (GetWindow(hwnd, GW_HWNDPREV), GetWindow(hwnd, GW_HWNDNEXT)) }
        })
    }

    /// The client's result once it finished; `None` while it runs.
    fn finished(&self) -> Option<Vec<Reading>> {
        let mut client = self.client.borrow_mut();
        if !client.as_ref().is_some_and(JoinHandle::is_finished) {
            return None;
        }
        let result = client.take().unwrap().join().expect("the client did not panic");
        Some(result.unwrap_or_else(|error| panic!("the client failed: {error}")))
    }

    fn spawn(&self, work: impl FnOnce() -> Result<Vec<Reading>, String> + Send + 'static) {
        *self.client.borrow_mut() = Some(spawn_on_this_desktop(work));
    }

    fn event(&self, control: &Control, event: Event) {
        self.hub.borrow_mut().feed(&event);
        match event {
            Event::LoopStarted => {
                *self.steps.borrow_mut() = Some(control.open_wake_lane().unwrap());
                let notices = control.open_wake_lane().unwrap();
                let waker = notices.waker();
                self.hub.borrow_mut().set_notifier(move || {
                    let _ = waker.wake(1);
                });
                *self.notices.borrow_mut() = Some(notices);
                let a = self.create(control, 30, 30);
                let b = self.create(control, 320, 30);
                let spare = self.create(control, 30, 260);
                self.windows.borrow_mut().extend([(a, 0), (b, 0)]);
                self.spare.set(Some((spare, 0)));
            }
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => self.created(control, window),
            Event::Window {
                window,
                event: WindowEvent::FocusChanged { focused },
            } => self.seen.add(format!("{} focus {focused}", window.number())),
            Event::Window {
                window,
                event: WindowEvent::DisplayChanged { display },
            } if self.spare.get().is_some_and(|(spare, _)| spare == window) => {
                self.spare_displayed(control, window, display);
            }
            Event::Window {
                window,
                event: WindowEvent::DisplayChanged { display },
            } if display != Display::Normal => {
                self.seen.add(format!("{} display {display:?}", window.number()));
            }
            Event::Window {
                window,
                event: WindowEvent::Destroyed,
            } => {
                let ours = self.windows.borrow().iter().find(|(key, _)| *key == window).copied();
                if let Some((_, hwnd)) = ours {
                    stop_watching(hwnd);
                    self.destroyed.set(self.destroyed.get() + 1);
                    if self.destroyed.get() == 2 {
                        control.begin_shutdown().unwrap();
                    }
                }
            }
            Event::ShutdownBegan => control.finish(0).unwrap(),
            _ => {}
        }
    }

    fn created(&self, control: &Control, window: WindowKey) {
        let hwnd = control.native_window(window).unwrap().win32().hwnd.get();
        must_stay_inactive(hwnd);
        if let Some((spare, _)) = self.spare.get().filter(|(spare, _)| *spare == window) {
            // Shown first: the adapter must not take a shown window's procedure, and is refused before it panics.
            self.spare.set(Some((spare, hwnd)));
            control
                .command(&self.source(), window, WindowCommand::Show { activate: false })
                .unwrap();
            assert_eq!(self.hub.borrow_mut().attach(control, window), Err(AttachRefusal::AlreadyShown));
            assert!(!self.hub.borrow().windows().contains(&window), "nothing kept of a refused attach");
            // It is hidden again when the loop reports it shown (`Self::spare_displayed`).
            return;
        }
        let at = self.windows.borrow().iter().position(|(key, _)| *key == window).unwrap();
        self.windows.borrow_mut()[at].1 = hwnd;
        let mut hub = self.hub.borrow_mut();
        hub.attach(control, window).unwrap();
        assert_eq!(hub.attach(control, window), Err(AttachRefusal::AlreadyAttached));
        hub.update(window, if at == 0 { first(None) } else { second() }).unwrap();
        drop(hub);
        control
            .command(&self.source(), window, WindowCommand::Show { activate: false })
            .unwrap();
        self.shown.set(self.shown.get() + 1);
        if self.shown.get() == 2 {
            self.next();
        }
    }

    /// The spare window's display, as the loop reports it: shown, then hidden again, which no longer shows in its
    /// visibility; attaching it is refused all the same, as the window was shown once (MT-214 E7-F6).
    fn spare_displayed(&self, control: &Control, window: WindowKey, display: Display) {
        let (_, hwnd) = self.spare.get().unwrap();
        match display {
            Display::Normal if !self.spare_hidden.replace(true) => {
                control.command(&self.source(), window, WindowCommand::Hide).unwrap();
            }
            Display::Hidden => {
                assert_eq!(self.hub.borrow_mut().attach(control, window), Err(AttachRefusal::AlreadyShown));
                assert!(!self.hub.borrow().windows().contains(&window), "nothing kept of a refused attach");
                control.command(&self.source(), window, WindowCommand::Destroy).unwrap();
                stop_watching(hwnd);
                self.spare_done.set(true);
            }
            _ => {}
        }
    }

    fn wake(&self, control: &Control, wake: &DispatchedWake<'_>) {
        self.take_requests(control);
        let ours = self.steps.borrow().as_ref().is_some_and(|lane| lane.id() == wake.lane());
        if ours {
            self.step(control);
        }
    }

    fn step(&self, control: &Control) {
        let ((a, a_hwnd), (b, b_hwnd)) = (self.window(0), self.window(1));
        match self.step.get() {
            1 => {
                if !self.spare_done.get() {
                    return self.wake_in(Duration::from_millis(30));
                }
                assert_eq!(
                    control.command(&self.source(), a, WindowCommand::Destroy),
                    Err(Refusal::NativeTokensHeld),
                    "the adapter keeps the window until it is closed"
                );
                assert_eq!(self.hub.borrow_mut().attach(control, WindowKey::from_number(999)), Err(AttachRefusal::UnknownWindow));
                self.before.set(Some((control.report().activations_refused, self.places())));
                self.seen.clear();
                self.spawn(move || with_automation(|automation| [a_hwnd, b_hwnd].iter().map(|hwnd| read(automation, *hwnd)).collect()));
                self.next();
            }
            2 => {
                let Some(readings) = self.finished() else {
                    return self.wake_in(Duration::from_millis(30));
                };
                eprintln!("UIA readings: {readings:#?}");
                self.check_reading(control, &readings);
                self.no_activation(control, true);
                // The keyboard reaches the first window (posted messages, as the input crate's real loop test does): the
                // window library takes a window as focused only when it is active as well, so the activation comes
                // first (WM_NCACTIVATE, true; posting it activates nothing, the guard and the watches see to that), then
                // the adapter shows the frame's focus while the window has it (MT-214 E7-F1).
                // SAFETY: plain values, to a window of this thread.
                assert_ne!(unsafe { PostMessageW(a_hwnd, WM_NCACTIVATE, 1, 0) }, 0);
                // SAFETY: as above.
                assert_ne!(unsafe { PostMessageW(a_hwnd, WM_SETFOCUS, 0, 0) }, 0);
                self.hub.borrow_mut().update(a, first(Some(NAME))).unwrap();
                self.next();
            }
            3 => {
                if self.seen.count(&format!("{} focus true", a.number())) == 0 {
                    return self.wake_in(Duration::from_millis(30));
                }
                self.spawn(move || with_automation(|automation| Ok(vec![read(automation, a_hwnd)?])));
                self.next();
            }
            4 => {
                let Some(readings) = self.finished() else {
                    return self.wake_in(Duration::from_millis(30));
                };
                let reading = &readings[0];
                assert!(reading.child("Name").focused, "the frame's focus: {reading:?}");
                assert!(!reading.child("Save").focused);
                // SAFETY: plain values, to a window of this thread.
                assert_ne!(unsafe { PostMessageW(a_hwnd, WM_KILLFOCUS, 0, 0) }, 0);
                // The scale doubles for the tree only; the window crate's scale is untouched.
                let scale = control.window_state(a).unwrap().scale;
                let doubled = ScaleFactor::from_dpi(scale.dpi() * 2).unwrap();
                self.hub.borrow_mut().feed(&Event::Window {
                    window: a,
                    event: WindowEvent::ScaleChanged { from: scale, to: doubled },
                });
                self.spawn(move || read_and_act(a_hwnd, b_hwnd));
                self.next();
            }
            5 => {
                let Some(readings) = self.finished() else {
                    return self.wake_in(Duration::from_millis(30));
                };
                let scale = control.window_state(a).unwrap().scale;
                let doubled = ScaleFactor::from_dpi(scale.dpi() * 2).unwrap();
                close_to(readings[0].child("Save").rect, self.expected(a_hwnd, doubled, SAVE_BOUNDS));
                self.hub.borrow_mut().feed(&Event::Window {
                    window: a,
                    event: WindowEvent::ScaleChanged { from: doubled, to: scale },
                });
                self.take_requests(control);
                let requests: Vec<(WindowKey, AccessTarget, Action, Origin)> = self
                    .requests
                    .borrow()
                    .iter()
                    .map(|request| (request.window(), request.target(), request.action(), request.origin()))
                    .collect();
                // The Invoke is one typed Click, first. A SetFocus of the client reaches the provider as a Focus request
                // of the window or of the node, never as an activation: on a private desktop the root's SetFocus
                // reached the provider with no request at all (the system's window element answered it), a child's
                // with a request for the node and one for the window. So the child's request is required, and nothing
                // but a window or node request of the second window may follow the Invoke (MT-214 E7-F1).
                let focus_node = (b, AccessTarget::Node(SAVE), Action::Focus, Origin::OtherProgram);
                let focus_window = (b, AccessTarget::Window, Action::Focus, Origin::OtherProgram);
                assert_eq!(
                    requests.first(),
                    Some(&(a, AccessTarget::Node(SAVE), Action::Click, Origin::OtherProgram)),
                    "the client's Invoke, typed, with its window: {requests:?}"
                );
                assert!(requests.contains(&focus_node), "the child's SetFocus asked the provider: {requests:?}");
                assert!(
                    requests[1..].iter().all(|request| *request == focus_node || *request == focus_window),
                    "only focus requests of the second window followed: {requests:?}"
                );
                self.no_activation(control, false);
                let report = self.hub.borrow().report();
                assert!(report.initial_trees >= 2 && report.updates_sent >= 3, "{report:?}");
                assert_eq!((report.requests_stale, report.requests_unsupported, report.raise_failed), (0, 0, 0), "{report:?}");
                for window in [a, b] {
                    assert!(self.hub.borrow_mut().close(window));
                    control.command(&self.source(), window, WindowCommand::Destroy).unwrap();
                }
            }
            other => panic!("step {other}"),
        }
    }

    /// Where a node's logical bounds are on the screen, at `scale`.
    fn expected(&self, hwnd: isize, scale: ScaleFactor, bounds: Rect) -> RECT {
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: a window of this thread; the point outlives the call.
        assert_ne!(unsafe { ClientToScreen(hwnd, &mut origin) }, 0);
        let at = |logical: f64, from: i32| from + scale.to_physical(logical).round() as i32;
        RECT {
            left: at(bounds.x0, origin.x),
            top: at(bounds.y0, origin.y),
            right: at(bounds.x1, origin.x),
            bottom: at(bounds.y1, origin.y),
        }
    }

    fn check_reading(&self, control: &Control, readings: &[Reading]) {
        let ((a, a_hwnd), (b, b_hwnd)) = (self.window(0), self.window(1));
        assert_eq!(readings.len(), 2);
        let (first, second) = (&readings[0], &readings[1]);
        assert_eq!(first.name, "first window");
        assert_eq!(second.name, "second window", "each window its own tree");
        let names = |reading: &Reading| reading.children.iter().map(|child| child.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(first), ["Save", "Name"], "roles and names, in order; the run is no element");
        assert_eq!(names(second), ["Open"], "the same key, another window's node");
        let save = first.child("Save");
        assert_eq!((save.automation_id.as_str(), save.control_type), ("save", UIA_ButtonControlTypeId));
        let name = first.child("Name");
        assert_eq!((name.automation_id.as_str(), name.control_type), ("name", UIA_EditControlTypeId));
        assert!(!save.focused && !name.focused, "the window does not have the keyboard");
        let open = second.child("Open");
        assert_eq!(open.automation_id, "open");
        let scale = |window| control.window_state(window).unwrap().scale;
        close_to(save.rect, self.expected(a_hwnd, scale(a), SAVE_BOUNDS));
        close_to(open.rect, self.expected(b_hwnd, scale(b), Rect::new(20.0, 20.0, 120.0, 60.0)));
    }

    /// Nothing activated a window: the guard refused no activation since the client began (none was tried), no quiet
    /// window broke, and the windows kept their place in the Z order. With `reading`, also no focus or display change.
    fn no_activation(&self, control: &Control, reading: bool) {
        let report = control.report();
        let (refused, places) = self.before.get().unwrap();
        assert_eq!(report.activations_refused, refused, "an activation was tried: {report:?}");
        assert_eq!(report.quiet_broken, 0);
        assert_eq!(self.places(), places, "a window moved in the Z order");
        let changes = self.seen.all();
        let changes: Vec<&String> = changes
            .iter()
            .filter(|change| reading || change.contains("display"))
            .collect();
        assert!(changes.is_empty(), "{changes:?}");
    }
}

/// Within a pixel each way: UI Automation reports whole pixels of fractional bounds.
fn close_to(read: RECT, expected: RECT) {
    let edges = [
        (read.left, expected.left),
        (read.top, expected.top),
        (read.right, expected.right),
        (read.bottom, expected.bottom),
    ];
    assert!(edges.iter().all(|(read, expected)| (read - expected).abs() <= 1), "read {read:?}, expected {expected:?}");
}

#[test]
fn a_ui_automation_client_reads_each_windows_tree_and_acts_without_activating_it() {
    on_private_desktop(Duration::from_secs(90), || {
        let state = std::rc::Rc::new(State::default());
        let (events, wakes) = (std::rc::Rc::clone(&state), std::rc::Rc::clone(&state));
        let report: RunReport = run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            move |control, wake| wakes.wake(control, wake),
        )
        .unwrap();
        assert_eq!(state.step.get(), 5, "every step ran");
        assert_eq!(state.destroyed.get(), 2);
        assert!(state.spare_done.get(), "the window shown and hidden again was refused and destroyed");
        assert_eq!(report.quiet_broken, 0);
        assert_eq!(report.refusals, 1, "the one Destroy refused while the adapter held the window");
        assert_eq!(report.windows_left, 0, "every adapter let go of its window");
    });
}
