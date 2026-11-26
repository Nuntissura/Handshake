//! When a wake is posted, and when it is posted again. Decisions only; `platform::wake` carries them out.
//!
//! A wake that was accepted is dispatched, unless it is cancelled, replaced or the loop ends. Posting it can fail (a
//! full message queue), and a posted message can be taken out and thrown away by somebody else's loop. So every post
//! carries a number. A dispatch settles its own post and every earlier one, because the posted messages of one queue
//! come out in order. A post that no dispatch settled is posted again, after an interval that doubles with each post
//! nobody answered, up to [`REPEAT_MAX`], for as long as the wake is wanted (MT-214 W2, MT-215 F1).
//!
//! Bounded (MT-214 R2-N3, MT-215 F4). A refused post is tried again after [`RETRY`], doubled for each refusal in a row
//! up to [`REPEAT_MAX`]. After [`BLIND_POSTS`] posts nobody answered, the next one waits for a sign that the loop runs:
//! a dispatched wake of any lane, or a probe it answered. A loop that hangs keeps the posts it has in its queue, so
//! more would only fill it; one that runs and threw them away gets the next post as soon as it answers.

use std::time::Duration;

/// How long after a refused post the next one follows, the first time.
pub(crate) const RETRY: Duration = Duration::from_millis(10);

/// How many posts may go unanswered before the next one waits for a sign that the loop runs.
pub(crate) const BLIND_POSTS: u32 = 4;

/// How long a post may go unanswered before it is posted again.
pub(crate) const REPEAT: Duration = Duration::from_millis(500);

/// The longest interval between posts nobody answered.
pub(crate) const REPEAT_MAX: Duration = Duration::from_secs(8);

/// How long a lane waits after a dispatch while the Operator's input waits in the queue, so a lane that asks again at
/// once does not keep input out: a posted message comes out before input.
pub(crate) const YIELD: Duration = Duration::from_millis(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Armed {
    value: u64,
    due: u64,
}

/// What to do now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Nothing is wanted: no timer.
    Idle,
    /// Look again at this time.
    LookAt(u64),
    /// Post now, with this number, then report with [`WakeSlot::posted`].
    Post(u64),
    /// Ask the loop whether it runs, and look again at this time, or when it answers.
    Ask(u64),
}

/// A wake that the loop dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Taken {
    pub(crate) value: u64,
    /// How long after it was due.
    pub(crate) late: u64,
}

/// One wake at a time: the newest.
#[derive(Debug, Default)]
pub(crate) struct WakeSlot {
    armed: Option<Armed>,
    /// The number of the newest post the queue took.
    posted: u64,
    /// Every post up to this number was dispatched, or thrown away in front of one that was.
    settled: u64,
    /// When the newest post was taken.
    posted_at: u64,
    /// Posts taken since the last dispatch.
    unanswered: u32,
    /// No post before this time.
    not_before: u64,
    /// Posts refused in a row.
    refused: u32,
    /// No post before this time, after a refused one.
    retry_at: u64,
}

impl WakeSlot {
    /// Ask for a wake. One that has not been dispatched yet is replaced. `life` as for [`WakeSlot::step`].
    pub(crate) fn request(&mut self, now: u64, value: u64, delay: u64, life: u64) -> Step {
        self.armed = Some(Armed {
            value,
            due: now.saturating_add(delay),
        });
        self.step(now, life)
    }

    /// What is needed at this time. `life` is when the loop last showed that it runs; 0 for never.
    pub(crate) fn step(&self, now: u64, life: u64) -> Step {
        let Some(armed) = self.armed else {
            return Step::Idle;
        };
        let start = armed.due.max(self.not_before).max(self.retry_at);
        if now < start {
            return Step::LookAt(start);
        }
        if self.settled == self.posted {
            return Step::Post(self.next_number());
        }
        let repeat_at = self.posted_at.saturating_add(self.interval());
        if now < repeat_at {
            Step::LookAt(repeat_at)
        } else if self.unanswered < BLIND_POSTS || life > self.posted_at {
            Step::Post(self.next_number())
        } else {
            Step::Ask(now.saturating_add(nanos(REPEAT_MAX)))
        }
    }

    /// The outcome of the post with this number. Returns when to look again.
    pub(crate) fn posted(&mut self, now: u64, number: u64, taken: bool) -> u64 {
        if !taken {
            let wait = nanos(RETRY)
                .saturating_mul(1 << self.refused.min(16))
                .min(nanos(REPEAT_MAX));
            self.refused = self.refused.saturating_add(1);
            self.retry_at = now.saturating_add(wait);
            return self.retry_at;
        }
        self.refused = 0;
        self.retry_at = 0;
        self.posted = self.posted.max(number);
        self.posted_at = now;
        self.unanswered = self.unanswered.saturating_add(1);
        now.saturating_add(self.interval())
    }

    /// The loop dispatched the post with this number. Returns the wake, if one is due. A number that is not one of an
    /// open post (forged, replayed, or settled before) changes nothing. While `input_waits`, no post follows for
    /// [`YIELD`].
    pub(crate) fn dispatched(&mut self, now: u64, number: u64, input_waits: bool) -> Option<Taken> {
        if number <= self.settled || number > self.posted {
            return None;
        }
        self.settled = number;
        self.unanswered = 0;
        self.not_before = if input_waits {
            now.saturating_add(nanos(YIELD))
        } else {
            0
        };
        let armed = self.armed.filter(|armed| now >= armed.due)?;
        self.armed = None;
        Some(Taken {
            value: armed.value,
            late: now.saturating_sub(armed.due),
        })
    }

    /// Withdraw the wake. A post that is under way then dispatches nothing.
    pub(crate) fn cancel(&mut self) {
        self.armed = None;
    }

    fn next_number(&self) -> u64 {
        self.posted.wrapping_add(1).max(1)
    }

    /// How long the newest post may go unanswered: [`REPEAT`], doubled for each further post nobody answered.
    fn interval(&self) -> u64 {
        let doublings = self.unanswered.saturating_sub(1).min(16);
        nanos(REPEAT)
            .saturating_mul(1 << doublings)
            .min(nanos(REPEAT_MAX))
    }
}

/// Whole nanoseconds, as far as they fit.
pub(crate) fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    /// A queue for the slot: posts are refused while `refuses` says so, thrown away while `loses` says so (by the
    /// number of the accepted post), and otherwise dispatched `DISPATCH_AFTER` later. A loop that `runs` answers a
    /// probe at once. Runs from `from` until a wake is dispatched or `until`; returns its time.
    struct Queue {
        refuses: fn(u64) -> bool,
        loses: Box<dyn Fn(u64) -> bool>,
        runs: bool,
        accepted: u64,
        /// When a post was tried, taken or not.
        attempts: Vec<u64>,
        asked: u32,
        life: u64,
    }

    const DISPATCH_AFTER: u64 = MS;

    impl Queue {
        fn new(loses: impl Fn(u64) -> bool + 'static) -> Self {
            Self {
                refuses: |_| false,
                loses: Box::new(loses),
                runs: true,
                accepted: 0,
                attempts: Vec::new(),
                asked: 0,
                life: 0,
            }
        }

        fn serve(&mut self, slot: &mut WakeSlot, step: Step, from: u64, until: u64) -> Option<u64> {
            let (mut step, mut now) = (step, from);
            loop {
                match step {
                    Step::Idle => return None,
                    Step::Ask(at) => {
                        self.asked += 1;
                        step = if self.runs {
                            // The answer nudges the lane.
                            self.life = now;
                            slot.step(now, self.life)
                        } else {
                            Step::LookAt(at)
                        };
                    }
                    Step::LookAt(at) if at > until => return None,
                    Step::LookAt(at) => {
                        now = at;
                        step = slot.step(now, self.life);
                    }
                    Step::Post(number) => {
                        self.attempts.push(now);
                        let taken = !(self.refuses)(now);
                        let look = slot.posted(now, number, taken);
                        if taken {
                            self.accepted += 1;
                            if !(self.loses)(self.accepted) {
                                let at = now + DISPATCH_AFTER;
                                self.life = self.life.max(at);
                                if slot.dispatched(at, number, false).is_some() {
                                    return Some(at);
                                }
                            }
                        }
                        step = Step::LookAt(look);
                    }
                }
            }
        }
    }

    #[test]
    fn a_wake_is_posted_when_due_and_not_before() {
        let mut slot = WakeSlot::default();
        assert_eq!(slot.request(10 * MS, 7, 200 * MS, 0), Step::LookAt(210 * MS));
        assert_eq!(slot.step(209 * MS, 0), Step::LookAt(210 * MS));
        assert_eq!(slot.step(210 * MS, 0), Step::Post(1));
        slot.posted(210 * MS, 1, true);
        assert_eq!(
            slot.dispatched(225 * MS, 1, false),
            Some(Taken {
                value: 7,
                late: 15 * MS
            })
        );
        assert_eq!(slot.step(226 * MS, 0), Step::Idle, "nothing keeps ticking");
    }

    #[test]
    fn a_refused_post_is_tried_again_at_growing_intervals_until_one_is_taken() {
        let mut slot = WakeSlot::default();
        let step = slot.request(0, 1, 0, 0);
        let mut queue = Queue::new(|_| false);
        queue.refuses = |now| now < 300 * MS;
        let at = queue.serve(&mut slot, step, 0, 400 * MS);
        assert_eq!(
            queue.attempts,
            [0, 10 * MS, 30 * MS, 70 * MS, 150 * MS, 310 * MS],
            "RETRY, doubled for each refusal"
        );
        assert_eq!(at, Some(310 * MS + DISPATCH_AFTER), "the first try after the queue had room");
        assert_eq!(queue.accepted, 1);
    }

    /// MT-214 R2-N3: a queue that stays full is not tried every 10 ms.
    #[test]
    fn a_queue_that_stays_full_is_tried_once_per_repeat_max_and_no_more() {
        let mut slot = WakeSlot::default();
        let step = slot.request(0, 1, 0, 0);
        let mut queue = Queue::new(|_| false);
        queue.refuses = |_| true;
        assert_eq!(queue.serve(&mut slot, step, 0, 600_000 * MS), None);
        let gaps: Vec<u64> = queue.attempts.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert_eq!(&gaps[..4], &[10 * MS, 20 * MS, 40 * MS, 80 * MS]);
        assert!(gaps.windows(2).all(|pair| pair[0] <= pair[1]), "never sooner again");
        assert_eq!(gaps.last(), Some(&nanos(REPEAT_MAX)));
        let tries = queue.attempts.len();
        assert!((80..90).contains(&tries), "{tries} tries in ten minutes");
        assert_eq!(queue.accepted, 0);
    }

    #[test]
    fn posts_thrown_away_again_and_again_do_not_end_the_lane() {
        // The probe of the evaluation: three posts thrown away ended the lane for good.
        for lost in [1, 3, 4, 10, 40] {
            let mut slot = WakeSlot::default();
            let mut queue = Queue::new(move |accepted| accepted <= lost);
            let step = slot.request(0, 1, 0, 0);
            let at = queue.serve(&mut slot, step, 0, 3_600_000 * MS);
            assert!(at.is_some(), "{lost} thrown away");
            assert_eq!(queue.accepted, lost + 1);
            let blind = u64::from(BLIND_POSTS);
            assert_eq!(u64::from(queue.asked), lost.saturating_sub(blind - 1), "{lost}");
            // Afterwards, a heartbeat asked for 200 ms after each dispatch keeps its cadence.
            let mut now = at.unwrap();
            for beat in 0..50 {
                let step = slot.request(now, beat, 200 * MS, queue.life);
                let at = queue.serve(&mut slot, step, now, now + 1_000 * MS).unwrap();
                assert_eq!(at - now, 200 * MS + DISPATCH_AFTER, "{lost} thrown away, beat {beat}");
                now = at;
            }
        }
    }

    #[test]
    fn losses_spread_over_a_long_life_delay_one_beat_each() {
        let mut slot = WakeSlot::default();
        let mut queue = Queue::new(|accepted| matches!(accepted, 100 | 2_000 | 5_000 | 9_999));
        let mut now = 0;
        let mut late_beats = 0;
        for beat in 0..20_000 {
            let step = slot.request(now, beat, 200 * MS, queue.life);
            let at = queue.serve(&mut slot, step, now, now + 60_000 * MS);
            let at = at.unwrap_or_else(|| panic!("the heartbeat ended at beat {beat}"));
            let gap = at - now;
            if gap != 200 * MS + DISPATCH_AFTER {
                late_beats += 1;
                assert_eq!(gap, 200 * MS + nanos(REPEAT) + DISPATCH_AFTER, "beat {beat}");
            }
            now = at;
        }
        assert_eq!(late_beats, 4);
        assert_eq!(queue.asked, 0, "a single loss needs no probe");
    }

    #[test]
    fn wakes_asked_for_long_after_losses_are_dispatched() {
        let mut slot = WakeSlot::default();
        let mut queue = Queue::new(|accepted| accepted <= 3);
        let step = slot.request(0, 1, 0, 0);
        assert!(queue.serve(&mut slot, step, 0, 600_000 * MS).is_some());
        for (value, at) in [(10, 700_000 * MS), (11, 800_000 * MS), (12, 900_000 * MS)] {
            let step = slot.request(at, value, 0, queue.life);
            assert_eq!(queue.serve(&mut slot, step, at, at + MS), Some(at + DISPATCH_AFTER));
        }
    }

    /// MT-214 R2-N3, MT-215 F4: a hung loop keeps its posts, so it gets no more than [`BLIND_POSTS`], however long it
    /// hangs; once it shows that it runs, the next post follows at once.
    #[test]
    fn a_loop_that_does_not_run_gets_a_bounded_number_of_posts_and_the_wake_once_it_answers() {
        let mut slot = WakeSlot::default();
        let mut queue = Queue::new(|_| true);
        queue.runs = false;
        let step = slot.request(0, 1, 0, 0);
        assert_eq!(queue.serve(&mut slot, step, 0, 3_600_000 * MS), None);
        assert_eq!(queue.attempts, [0, 500 * MS, 1_500 * MS, 3_500 * MS], "in an hour");
        assert_eq!(queue.attempts.len(), BLIND_POSTS as usize);
        assert!(queue.asked > 400, "it keeps asking, once per {REPEAT_MAX:?}: {}", queue.asked);
        // It runs again, and threw the posts away meanwhile.
        let back = 3_600_000 * MS;
        assert!(matches!(slot.step(back, 0), Step::Ask(_)));
        queue.runs = true;
        queue.loses = Box::new(|_| false);
        assert_eq!(
            queue.serve(&mut slot, Step::Ask(back + nanos(REPEAT_MAX)), back, back + MS),
            Some(back + DISPATCH_AFTER)
        );
        // A sign of life from elsewhere, a dispatch of another lane, does as much, but only one after the newest post.
        let mut other = WakeSlot::default();
        other.request(0, 2, 0, 0);
        for (number, at) in (1..=4).zip([0, 500, 1_500, 3_500]) {
            other.posted(at * MS, number, true);
        }
        assert!(matches!(other.step(8_000 * MS, 0), Step::Ask(_)));
        assert!(matches!(other.step(8_000 * MS, 3_500 * MS), Step::Ask(_)));
        assert_eq!(other.step(8_000 * MS, 7_000 * MS), Step::Post(5));
    }

    #[test]
    fn the_loop_coming_back_dispatches_the_wake_once() {
        let mut slot = WakeSlot::default();
        slot.request(0, 9, 0, 0);
        let mut numbers = Vec::new();
        let mut now = 0;
        while now < 5_000 * MS {
            now = match slot.step(now, 0) {
                Step::Post(number) => {
                    numbers.push(number);
                    slot.posted(now, number, true)
                }
                Step::LookAt(at) | Step::Ask(at) => at,
                Step::Idle => break,
            };
        }
        assert_eq!(numbers, [1, 2, 3, 4]);
        // The queue gives them back in order.
        let got: Vec<_> = numbers
            .iter()
            .map(|number| slot.dispatched(5_001 * MS, *number, false).map(|taken| taken.value))
            .collect();
        assert_eq!(got, [Some(9), None, None, None]);
        assert_eq!(slot.request(5_004 * MS, 10, 0, 0), Step::Post(5), "no post is open");
    }

    #[test]
    fn a_dispatch_of_a_post_that_is_not_open_changes_nothing() {
        let mut slot = WakeSlot::default();
        assert_eq!(slot.dispatched(5, 1, false), None, "nothing was posted");
        assert_eq!(slot.request(6, 1, 0, 0), Step::Post(1));
        slot.posted(6, 1, true);
        for forged in [0, 2, 77, u64::MAX] {
            assert_eq!(slot.dispatched(7, forged, false), None, "{forged}");
        }
        assert_eq!(slot.dispatched(8, 1, false).map(|taken| taken.value), Some(1));
        assert_eq!(slot.dispatched(9, 1, false), None, "replayed");
        slot.request(10, 2, 0, 0);
        assert_eq!(slot.dispatched(11, 1, false), None, "settled before");
    }

    #[test]
    fn a_post_of_an_earlier_wake_does_not_bring_the_next_one_early() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 0, 0);
        slot.posted(0, 1, true);
        slot.request(MS, 2, 200 * MS, 0);
        assert_eq!(slot.dispatched(2 * MS, 1, false), None);
        assert_eq!(slot.step(201 * MS, 0), Step::Post(2));
        slot.posted(201 * MS, 2, true);
        assert_eq!(slot.dispatched(202 * MS, 2, false).map(|taken| taken.value), Some(2));
    }

    #[test]
    fn the_newest_request_wins() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 500 * MS, 0);
        assert_eq!(slot.request(MS, 2, 0, 0), Step::Post(1));
        slot.posted(MS, 1, true);
        assert_eq!(slot.dispatched(2 * MS, 1, false).map(|taken| taken.value), Some(2));
        assert_eq!(slot.step(600 * MS, 0), Step::Idle);
    }

    #[test]
    fn a_cancelled_wake_is_not_dispatched() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 0, 0);
        slot.posted(0, 1, true);
        slot.cancel();
        assert_eq!(slot.dispatched(MS, 1, false), None);
        assert_eq!(slot.step(MS, 0), Step::Idle);
    }

    #[test]
    fn a_wake_asked_for_at_once_waits_a_moment_while_input_waits() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 0, 0);
        slot.posted(0, 1, true);
        slot.dispatched(MS, 1, true);
        assert_eq!(slot.request(MS, 2, 0, 0), Step::LookAt(MS + nanos(YIELD)));
        assert_eq!(slot.step(MS + nanos(YIELD), 0), Step::Post(2));
        slot.posted(2 * MS, 2, true);
        slot.dispatched(3 * MS, 2, false);
        assert_eq!(slot.request(3 * MS, 3, 0, 0), Step::Post(3), "no input, no wait");
    }
}
