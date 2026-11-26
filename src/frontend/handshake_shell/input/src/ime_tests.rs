//! The input method's stream, against event sequences of winit 0.30.13's IMM32 handling (`event_loop.rs`,
//! `WM_IME_STARTCOMPOSITION`, `WM_IME_COMPOSITION`, `WM_IME_ENDCOMPOSITION`) as its source writes them, with focus
//! changes at every step; and the candidate area. The Operator's origin is used directly: only the loop makes an
//! Operator stamp.

use super::*;
use handshake_shell_window::Attribution;

const W: WindowKey = WindowKey::from_number(1);
const A: TextTarget = TextTarget(10);
const B: TextTarget = TextTarget(20);
const MODEL: Attribution = Attribution {
    actor: 7,
    session: 8,
    correlation: 9,
};

fn at(nanos: u64) -> EventTime {
    EventTime::from_nanos(nanos)
}

fn operator(nanos: u64) -> Source {
    Source {
        origin: Origin::OperatorDevice,
        at: at(nanos),
        stamp: None,
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> LogicalRect {
    LogicalRect {
        x,
        y,
        width,
        height,
    }
}

fn place() -> CaretPlace {
    CaretPlace {
        viewport: rect(10.0, 20.0, 300.0, 200.0),
        caret: rect(40.0, 50.0, 1.0, 16.0),
        scroll_x: 0.0,
        scroll_y: 0.0,
    }
}

fn focus(target: TextTarget, accepts_ime: bool) -> Option<TextFocus> {
    Some(TextFocus {
        target,
        accepts_ime,
        caret: place(),
    })
}

fn preedit(text: &str, cursor: Option<(usize, usize)>) -> ImeChange {
    ImeChange::Preedit {
        text: TypedText::new(text),
        cursor,
    }
}

fn commit(text: &str) -> ImeChange {
    ImeChange::Commit(TypedText::new(text))
}

/// One event as a line: target, composition, what, and why when synthesized.
fn line(event: &InputEvent) -> String {
    let Input::Ime(ime) = event.input() else {
        panic!("not an input method event: {event:?}");
    };
    let what = match &ime.change {
        Composition::Enabled => "enabled".to_owned(),
        Composition::Preedit { text, cursor } => format!("preedit {:?} {cursor:?}", text.as_str()),
        Composition::Commit(text) => format!("commit {:?}", text.as_str()),
        Composition::Cancel => "cancel".to_owned(),
        Composition::Disabled => "disabled".to_owned(),
    };
    let why = event.synthetic().map(Synthetic::code).unwrap_or_default();
    format!(
        "{} #{} {what} {why}",
        ime.target.0,
        ime.composition.number()
    )
    .trim_end()
    .to_owned()
}

/// A router with one window that has the keyboard, and target A focused.
struct Run {
    router: ImeRouter,
    out: Vec<InputEvent>,
    clock: u64,
}

impl Run {
    fn new() -> Self {
        let mut router = ImeRouter::default();
        router.created(W, ScaleFactor::ONE, true);
        let mut out = Vec::new();
        router
            .focus(W, focus(A, true), false, at(0), &mut out)
            .unwrap();
        assert!(out.is_empty());
        Self {
            router,
            out,
            clock: 1,
        }
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    fn os(&mut self, changes: &[ImeChange]) -> &mut Self {
        for change in changes {
            let from = operator(self.tick());
            self.router.change(W, change, from, &mut self.out);
        }
        self
    }

    fn keyboard(&mut self, has: bool) -> &mut Self {
        let now = at(self.tick());
        self.router.keyboard(W, has, now, &mut self.out);
        self
    }

    fn focus(&mut self, focus: Option<TextFocus>, model: bool) -> Result<(), FocusRefusal> {
        let now = at(self.tick());
        self.router.focus(W, focus, model, now, &mut self.out)
    }

    fn lines(&mut self) -> Vec<String> {
        self.out.drain(..).map(|event| line(&event)).collect()
    }
}

/// winit: `WM_IME_STARTCOMPOSITION`, preedit updates (`GCS_COMPSTR`), then the result (`GCS_RESULTSTR`: an empty preedit
/// and the commit), then `WM_IME_ENDCOMPOSITION` in the enabled state.
#[test]
fn a_composition_that_commits_ends_with_the_commit_and_no_cancel() {
    let mut run = Run::new();
    run.os(&[
        ImeChange::Started,
        preedit("に", Some((3, 3))),
        preedit("日本", Some((0, 6))),
        preedit("", None),
        commit("日本"),
        ImeChange::Ended,
    ]);
    assert_eq!(
        run.lines(),
        [
            "10 #1 enabled",
            "10 #1 preedit \"に\" Some(3..3)",
            "10 #1 preedit \"日本\" Some(0..6)",
            "10 #1 preedit \"\" None",
            "10 #1 commit \"日本\"",
            "10 #1 disabled",
        ]
    );
}

/// Esc: the composition string emptied (`GCS_COMPSTR` with no text, or lparam 0, which "indicates that the current
/// composition has been canceled"), then `WM_IME_ENDCOMPOSITION` while winit is in the preedit state, where it commits
/// the empty result string.
#[test]
fn a_composition_cleared_without_a_result_ends_with_one_cancel() {
    for clear in [preedit("", Some((0, 0))), preedit("", None)] {
        let mut run = Run::new();
        run.os(&[
            ImeChange::Started,
            preedit("か", Some((3, 3))),
            clear,
            preedit("", None),
            commit(""),
            ImeChange::Ended,
        ]);
        let lines = run.lines();
        assert_eq!(
            lines.iter().filter(|line| line.contains("cancel")).count(),
            1,
            "{lines:#?}"
        );
        assert!(
            !lines.iter().any(|line| line.contains("commit")),
            "{lines:#?}"
        );
        assert_eq!(lines.last().unwrap(), "10 #1 disabled");
        assert_eq!(lines[lines.len() - 2], "10 #1 cancel");
        assert_eq!(run.router.counts.empty_commits, 1);
    }
}

/// The Korean input method sends the result after `WM_IME_ENDCOMPOSITION`; winit reads it there while in the preedit
/// state and commits it before disabled.
#[test]
fn a_result_read_at_the_end_is_the_one_commit() {
    let mut run = Run::new();
    run.os(&[
        ImeChange::Started,
        preedit("한", Some((0, 3))),
        preedit("", None),
        commit("한"),
        ImeChange::Ended,
    ]);
    let lines = run.lines();
    assert_eq!(lines[3], "10 #1 commit \"한\"");
    assert_eq!(lines[4], "10 #1 disabled");
    assert!(!lines.iter().any(|line| line.contains("cancel")));
}

#[test]
fn clause_by_clause_commits_are_all_passed_on_and_need_no_cancel() {
    let mut run = Run::new();
    run.os(&[
        ImeChange::Started,
        preedit("にほんご", None),
        preedit("", None),
        commit("日本"),
        preedit("ご", None),
        preedit("", None),
        commit("語"),
        ImeChange::Ended,
    ]);
    let lines = run.lines();
    let commits: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("commit"))
        .collect();
    assert_eq!(commits, ["10 #1 commit \"日本\"", "10 #1 commit \"語\""]);
    assert!(!lines.iter().any(|line| line.contains("cancel")));
}

/// AC214-6: focus loss mid-composition sends one cancel, never also a commit: what the input method sends after it does
/// not reach the target.
#[test]
fn focus_lost_mid_composition_cancels_once_and_a_late_commit_reaches_nobody() {
    let mut run = Run::new();
    run.os(&[ImeChange::Started, preedit("か", Some((3, 3)))]);
    run.lines();
    run.keyboard(false);
    let ended = std::mem::take(&mut run.out);
    let lines: Vec<String> = ended.iter().map(line).collect();
    assert_eq!(
        lines,
        [
            "10 #1 cancel input.synthetic.focus_lost",
            "10 #1 disabled input.synthetic.focus_lost"
        ]
    );
    for event in &ended {
        assert_eq!(
            event.origin(),
            Origin::OperatorDevice,
            "the composition's origin"
        );
        assert!(event.stamp().is_none(), "a synthesized end grants nothing");
    }
    run.os(&[preedit("", None), commit("か"), ImeChange::Ended]);
    assert_eq!(
        run.lines(),
        Vec::<String>::new(),
        "the late result goes nowhere"
    );
    assert_eq!(run.router.counts.dropped, 2, "the preedit and the commit");
    assert_eq!(run.router.counts.settled, 1);
}

#[test]
fn focus_lost_after_a_commit_ends_without_a_cancel() {
    let mut run = Run::new();
    run.os(&[
        ImeChange::Started,
        preedit("か", None),
        preedit("", None),
        commit("家"),
    ]);
    run.lines();
    run.keyboard(false);
    assert_eq!(run.lines(), ["10 #1 disabled input.synthetic.focus_lost"]);
    run.os(&[ImeChange::Ended]);
    assert_eq!(
        run.lines(),
        Vec::<String>::new(),
        "the input method's own end comes after"
    );
}

/// The host moves the text focus mid-composition: the old target's composition ends there; what the input method goes
/// on with begins again at the new target, as a composition of its own.
#[test]
fn a_composition_the_focus_left_ends_at_its_target_and_goes_on_only_at_the_new_one() {
    let mut run = Run::new();
    run.os(&[ImeChange::Started, preedit("か", None)]);
    run.lines();
    run.focus(focus(B, true), false).unwrap();
    assert_eq!(
        run.lines(),
        [
            "10 #1 cancel input.synthetic.focus_moved",
            "10 #1 disabled input.synthetic.focus_moved"
        ]
    );
    run.os(&[
        preedit("かな", None),
        preedit("", None),
        commit("仮名"),
        ImeChange::Ended,
    ]);
    assert_eq!(
        run.lines(),
        [
            "20 #2 enabled input.synthetic.rebound",
            "20 #2 preedit \"かな\" None",
            "20 #2 preedit \"\" None",
            "20 #2 commit \"仮名\"",
            "20 #2 disabled",
        ]
    );
}

#[test]
fn a_target_that_stops_taking_composed_text_ends_its_composition_and_turns_the_input_method_off() {
    let mut run = Run::new();
    run.router.take_requests();
    run.os(&[ImeChange::Started, preedit("か", None)]);
    run.lines();
    run.router.take_requests();
    run.focus(focus(A, false), false).unwrap();
    assert_eq!(
        run.lines(),
        [
            "10 #1 cancel input.synthetic.ime_turned_off",
            "10 #1 disabled input.synthetic.ime_turned_off"
        ]
    );
    assert_eq!(run.router.take_requests(), [(W, ImeRequest::Allow(false))]);
    run.os(&[preedit("かな", None), commit("仮名")]);
    assert_eq!(
        run.lines(),
        Vec::<String>::new(),
        "no target takes composed text"
    );
}

#[test]
fn events_lost_and_a_window_gone_end_the_composition_once() {
    let mut run = Run::new();
    run.os(&[ImeChange::Started, preedit("か", None)]);
    run.lines();
    let now = at(run.tick());
    run.router.events_lost(now, &mut run.out);
    run.router.events_lost(now, &mut run.out);
    assert_eq!(
        run.lines(),
        [
            "10 #1 cancel input.synthetic.events_lost",
            "10 #1 disabled input.synthetic.events_lost"
        ]
    );
    run.os(&[ImeChange::Started, preedit("と", None)]);
    run.lines();
    let now = at(run.tick());
    run.router.gone(W, now, &mut run.out);
    assert_eq!(
        run.lines(),
        [
            "10 #2 cancel input.synthetic.window_gone",
            "10 #2 disabled input.synthetic.window_gone"
        ]
    );
    assert_eq!(
        run.focus(focus(A, true), false),
        Err(FocusRefusal::UnknownWindow)
    );
    run.os(&[preedit("x", None)]);
    assert!(run.lines().is_empty());
}

#[test]
fn a_composition_that_begins_over_an_open_one_ends_the_open_one_first() {
    let mut run = Run::new();
    run.os(&[ImeChange::Started, preedit("か", None), ImeChange::Started]);
    assert_eq!(
        run.lines(),
        [
            "10 #1 enabled",
            "10 #1 preedit \"か\" None",
            "10 #1 cancel input.synthetic.replaced",
            "10 #1 disabled input.synthetic.replaced",
            "10 #2 enabled",
        ]
    );
}

#[test]
fn without_the_keyboard_or_a_target_that_takes_it_nothing_is_composed() {
    let mut run = Run::new();
    run.keyboard(false);
    run.os(&[ImeChange::Started, preedit("か", None), commit("家")]);
    assert!(run.lines().is_empty());
    run.keyboard(true);
    run.focus(None, false).unwrap();
    run.os(&[ImeChange::Started]);
    assert!(run.lines().is_empty());
    assert_eq!(run.router.counts.dropped, 4);
}

/// GUI-DESKTOP-041: a model may not take the focus or composed text away from the Operator's composition; the
/// Operator's own and the shell's changes end it.
#[test]
fn a_model_cannot_take_the_focus_from_the_operators_composition() {
    let mut run = Run::new();
    run.os(&[ImeChange::Started, preedit("か", None)]);
    run.lines();
    assert!(run.router.operator_composing(W));
    assert_eq!(
        run.focus(focus(B, true), true),
        Err(FocusRefusal::OperatorComposing)
    );
    assert_eq!(run.focus(None, true), Err(FocusRefusal::OperatorComposing));
    assert_eq!(
        run.focus(focus(A, false), true),
        Err(FocusRefusal::OperatorComposing)
    );
    assert!(run.lines().is_empty(), "nothing ended");
    assert_eq!(run.router.counts.refused, 3);
    // The same target with its caret elsewhere takes nothing away.
    let mut moved = focus(A, true);
    if let Some(focus) = moved.as_mut() {
        focus.caret.caret.y += 16.0;
    }
    assert_eq!(run.focus(moved, true), Ok(()));
    assert!(run.lines().is_empty());
    // The shell may, and so may the Operator.
    assert_eq!(run.focus(focus(B, true), false), Ok(()));
    assert_eq!(run.lines().len(), 2, "cancel and disabled");
    assert!(!run.router.operator_composing(W));
}

#[test]
fn a_model_may_move_the_focus_from_a_composition_that_is_not_the_operators() {
    let mut router = ImeRouter::default();
    router.created(W, ScaleFactor::ONE, true);
    let mut out = Vec::new();
    router
        .focus(W, focus(A, true), true, at(1), &mut out)
        .unwrap();
    let from = Source {
        origin: Origin::ModelAction(MODEL),
        at: at(2),
        stamp: None,
    };
    router.change(W, &ImeChange::Started, from, &mut out);
    assert!(!router.operator_composing(W));
    assert_eq!(
        router.focus(W, focus(B, true), true, at(3), &mut out),
        Ok(())
    );
    assert_eq!(
        out.last().map(InputEvent::origin),
        Some(Origin::ModelAction(MODEL))
    );
}

#[test]
fn a_cursor_off_the_preedit_text_is_passed_on_as_none_and_counted() {
    let mut run = Run::new();
    run.os(&[
        ImeChange::Started,
        preedit("日本", Some((1, 3))),
        preedit("日本", Some((3, 9))),
        preedit("日本", Some((6, 3))),
        preedit("日本", Some((3, 6))),
    ]);
    let lines = run.lines();
    assert_eq!(
        lines[1], "10 #1 preedit \"日本\" None",
        "inside a character"
    );
    assert_eq!(lines[2], "10 #1 preedit \"日本\" None", "past the end");
    assert_eq!(lines[3], "10 #1 preedit \"日本\" None", "backwards");
    assert_eq!(lines[4], "10 #1 preedit \"日本\" Some(3..6)");
    assert_eq!(run.router.counts.bad_cursors, 3);
}

// The candidate area.

#[test]
fn the_candidate_area_is_the_caret_where_it_is_shown_in_physical_pixels() {
    let place = place();
    assert_eq!(
        place.candidate_area(ScaleFactor::ONE),
        Some(ClientRect {
            x: 50,
            y: 70,
            width: 1,
            height: 16
        })
    );
    // 150 %: edges rounded outward.
    assert_eq!(
        place.candidate_area(ScaleFactor::from_dpi(144).unwrap()),
        Some(ClientRect {
            x: 75,
            y: 105,
            width: 2,
            height: 24
        })
    );
    let unusable = CaretPlace {
        scroll_y: f64::NAN,
        ..place
    };
    assert_eq!(unusable.candidate_area(ScaleFactor::ONE), None);
    let negative = CaretPlace {
        caret: rect(0.0, 0.0, -1.0, 16.0),
        ..place
    };
    assert_eq!(negative.candidate_area(ScaleFactor::ONE), None);
}

#[test]
fn a_caret_scrolled_out_of_view_keeps_the_candidates_at_the_viewports_edge() {
    let above = CaretPlace {
        scroll_y: 500.0,
        ..place()
    };
    assert_eq!(
        above.candidate_area(ScaleFactor::ONE).unwrap().y,
        20,
        "the viewport's top"
    );
    let right = CaretPlace {
        scroll_x: -1000.0,
        ..place()
    };
    let area = right.candidate_area(ScaleFactor::ONE).unwrap();
    assert_eq!(
        area.x + area.width as i32,
        310,
        "inside the viewport's right edge"
    );
    let huge = CaretPlace {
        caret: rect(1e300, 1e300, 1e300, 1e300),
        ..place()
    };
    assert!(
        huge.candidate_area(ScaleFactor::ONE).is_some(),
        "no overflow"
    );
}

/// AC214-6: the candidate area follows the caret on scroll and on a scale change; nothing is asked again when nothing
/// moved.
#[test]
fn the_window_is_asked_for_the_area_again_on_scroll_and_scale_change_only() {
    let mut run = Run::new();
    assert_eq!(
        run.router.take_requests(),
        [
            (W, ImeRequest::Allow(true)),
            (
                W,
                ImeRequest::Area(ClientRect {
                    x: 50,
                    y: 70,
                    width: 1,
                    height: 16
                })
            )
        ]
    );
    run.router
        .place(W, A, |place| place.scroll_y = 30.0)
        .unwrap();
    assert_eq!(
        run.router.take_requests(),
        [(
            W,
            ImeRequest::Area(ClientRect {
                x: 50,
                y: 40,
                width: 1,
                height: 16
            })
        )]
    );
    run.router
        .place(W, A, |place| place.scroll_y = 30.0)
        .unwrap();
    run.router.scale(W, ScaleFactor::ONE);
    assert!(run.router.take_requests().is_empty(), "nothing moved");
    run.router.scale(W, ScaleFactor::from_dpi(192).unwrap());
    assert_eq!(
        run.router.take_requests(),
        [(
            W,
            ImeRequest::Area(ClientRect {
                x: 100,
                y: 80,
                width: 2,
                height: 32
            })
        )]
    );
    assert_eq!(
        run.router.place(W, B, |place| place.scroll_y = 0.0),
        Err(FocusRefusal::NotFocused),
        "a target without the focus is out of date"
    );
    assert!(run.router.take_requests().is_empty());
}

#[test]
fn a_composition_that_begins_asks_for_the_area_again_and_turning_off_forgets_it() {
    let mut run = Run::new();
    run.router.take_requests();
    run.os(&[ImeChange::Started]);
    let requests = run.router.take_requests();
    assert!(
        matches!(requests.as_slice(), [(W, ImeRequest::Area(_))]),
        "{requests:?}"
    );
    run.focus(focus(A, false), false).unwrap();
    assert_eq!(run.router.take_requests(), [(W, ImeRequest::Allow(false))]);
    run.focus(focus(A, true), false).unwrap();
    let requests = run.router.take_requests();
    assert!(
        matches!(
            requests.as_slice(),
            [(W, ImeRequest::Allow(true)), (W, ImeRequest::Area(_))]
        ),
        "allowed before placed: {requests:?}"
    );
    assert_eq!(
        ImeRequest::Allow(true).command(),
        WindowCommand::AllowIme(true)
    );
}

// Every interleaving.

/// A small deterministic generator, so a failure names its seed.
struct Steps(u64);

impl Steps {
    fn next(&mut self, below: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % below
    }
}

/// What a composition showed and ended with, to check every rule at once.
#[derive(Default)]
struct Seen {
    target: Option<TextTarget>,
    enabled: u32,
    pending: bool,
    cancels: u32,
    disabled: u32,
}

/// AC214-6 over generated sequences of input method changes, focus changes, keyboard loss, lost events and windows that
/// go, from every state: each composition goes to one target, begins once, shows a cancel only for preedit no commit
/// took and then never a commit, ends once, never commits empty text, and reaches a target only while that target has
/// the focus of a window with the keyboard.
#[test]
fn every_interleaving_keeps_one_target_one_end_and_never_a_commit_and_a_cancel() {
    let texts = ["か", "かな", "", "日本"];
    for seed in 1..=400_u64 {
        let mut steps = Steps(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut router = ImeRouter::default();
        router.created(W, ScaleFactor::ONE, true);
        let mut keyboard = true;
        let mut focused = Some((A, true));
        let mut out = Vec::new();
        router
            .focus(W, focus(A, true), false, at(0), &mut out)
            .unwrap();
        let mut seen: BTreeMap<u64, Seen> = BTreeMap::new();
        let mut clock = 0;
        for step in 0..60 {
            clock += 1;
            let from = operator(clock);
            out.clear();
            match steps.next(12) {
                0 => router.change(W, &ImeChange::Started, from, &mut out),
                1..=3 => {
                    let text = texts[steps.next(4) as usize];
                    router.change(W, &preedit(text, None), from, &mut out);
                }
                4 | 5 => {
                    let text = texts[steps.next(4) as usize];
                    router.change(W, &commit(text), from, &mut out);
                }
                6 => router.change(W, &ImeChange::Ended, from, &mut out),
                7 => {
                    keyboard = !keyboard;
                    router.keyboard(W, keyboard, at(clock), &mut out);
                }
                8 | 9 => {
                    let next = match steps.next(4) {
                        0 => None,
                        1 => Some((A, true)),
                        2 => Some((B, true)),
                        _ => Some((B, false)),
                    };
                    let target = next.map(|(target, accepts)| TextFocus {
                        target,
                        accepts_ime: accepts,
                        caret: place(),
                    });
                    router.focus(W, target, false, at(clock), &mut out).unwrap();
                    focused = next;
                }
                10 => router.events_lost(at(clock), &mut out),
                _ => router.change(W, &preedit("か", Some((0, 3))), from, &mut out),
            }
            for event in &out {
                let Input::Ime(ime) = event.input() else {
                    panic!("seed {seed} step {step}: {event:?}");
                };
                let state = seen.entry(ime.composition.number()).or_default();
                let context = format!("seed {seed} step {step}: {}", line(event));
                assert!(state.disabled == 0, "{context}: after its end");
                assert_eq!(
                    *state.target.get_or_insert(ime.target),
                    ime.target,
                    "{context}: one target"
                );
                let synthesized_end = matches!(
                    event.synthetic(),
                    Some(cause) if cause != Synthetic::Rebound
                );
                if !synthesized_end {
                    assert!(keyboard, "{context}: the window has no keyboard");
                    assert_eq!(
                        focused,
                        Some((ime.target, true)),
                        "{context}: not the focused target"
                    );
                }
                match &ime.change {
                    Composition::Enabled => {
                        assert_eq!(state.enabled, 0, "{context}: begun twice");
                        state.enabled = 1;
                    }
                    Composition::Preedit { text, .. } => {
                        assert_eq!(state.enabled, 1, "{context}: not begun");
                        state.pending |= !text.as_str().is_empty();
                    }
                    Composition::Commit(text) => {
                        assert!(!text.as_str().is_empty(), "{context}: empty commit");
                        assert_eq!(state.cancels, 0, "{context}: a commit after a cancel");
                        state.pending = false;
                    }
                    Composition::Cancel => {
                        assert!(state.pending, "{context}: a cancel with nothing to cancel");
                        state.cancels += 1;
                    }
                    Composition::Disabled => {
                        assert!(
                            !state.pending || state.cancels == 1,
                            "{context}: preedit left without an end"
                        );
                        state.disabled += 1;
                    }
                }
            }
        }
        out.clear();
        router.events_lost(at(clock + 1), &mut out);
        for event in &out {
            if let Input::Ime(ime) = event.input() {
                let state = seen.entry(ime.composition.number()).or_default();
                match ime.change {
                    Composition::Cancel => state.cancels += 1,
                    Composition::Disabled => state.disabled += 1,
                    _ => panic!("seed {seed}: the final loss ends only"),
                }
            }
        }
        for (number, state) in &seen {
            assert_eq!(state.enabled, 1, "seed {seed} #{number}: begun once");
            assert_eq!(state.disabled, 1, "seed {seed} #{number}: ended once");
            assert!(
                state.cancels <= 1,
                "seed {seed} #{number}: one cancel at most"
            );
        }
    }
}

#[test]
fn the_state_kept_holds_no_composed_text() {
    // The composition is Kumihimo's (KG-03): what is kept per window is who and which, not what.
    let mut run = Run::new();
    run.os(&[ImeChange::Started, preedit("geheim", Some((0, 6)))]);
    let kept = format!("{:?}", run.router);
    assert!(!kept.contains("geheim"), "{kept}");
    assert!(
        kept.contains("pending: true"),
        "the state is printed: {kept}"
    );
}
