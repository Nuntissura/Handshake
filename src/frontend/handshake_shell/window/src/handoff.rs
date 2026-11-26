//! What the loop's thread leaves for the thread that holds a lane's slot, so that it never waits for it (MT-215 N2,
//! AC215-6: the beat path is wait-free).
//!
//! A lane's slot is held by pool threads (the timer's callback, a waker) for one decision at a time: pure arithmetic on
//! the slot, never an operating-system call. The loop's thread takes it with `try_lock`; where that fails it puts what it
//! wanted here and goes on. Whoever holds the slot works through it before it lets go and looks again after (the fences
//! pair a put with that look: either the putter's `try_lock` finds the slot free, or the holder's look finds the put).
//! There is nothing here that waits: a put is a few stores, a take a few loads.
//!
//! The mail is a seqlock with one writer, the loop's thread, and one reader at a time, whoever holds the slot. The other
//! hand-offs are single flags and numbers, which need no more.

use std::sync::atomic::{fence, AtomicBool, AtomicU64, Ordering};

/// What the loop's thread asked for while it could not take the slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mail {
    /// Arm this value, due at this time (the lane's own clock).
    Request { value: u64, due: u64 },
    /// Withdraw the wake.
    Cancel,
}

const REQUEST: u64 = 0;
const CANCEL: u64 = 1;

/// Hand-offs from the loop's thread (and, for a refused post, from any thread that posts) to the slot's holder.
#[derive(Debug, Default)]
pub(crate) struct Handoff {
    /// Even: the mail is stable; odd: being written. The newest mail has this number.
    seq: AtomicU64,
    /// The number of the mail the holder took last.
    applied: AtomicU64,
    kind: AtomicU64,
    value: AtomicU64,
    due: AtomicU64,
    /// A dispatched post the slot could not be asked about: post it again, so it comes round once more.
    retake: AtomicBool,
    retake_token: AtomicU64,
    /// The highest number of a post the queue did not take; 0 for none.
    refused: AtomicU64,
    /// Something the slot decides on changed (the loop answered a probe): look again.
    look: AtomicBool,
}

impl Handoff {
    /// Leave a request or a cancel; the newest one wins. From the loop's thread only: there is one writer.
    pub(crate) fn put(&self, mail: Mail) {
        let (kind, value, due) = match mail {
            Mail::Request { value, due } => (REQUEST, value, due),
            Mail::Cancel => (CANCEL, 0, 0),
        };
        let seq = self.seq.load(Ordering::Relaxed);
        self.seq.store(seq.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        self.kind.store(kind, Ordering::Relaxed);
        self.value.store(value, Ordering::Relaxed);
        self.due.store(due, Ordering::Relaxed);
        self.seq.store(seq.wrapping_add(2), Ordering::Release);
        fence(Ordering::SeqCst);
    }

    /// The mail left since the last take, if it is whole. Only the holder of the slot calls it. A mail that is being
    /// written now is not taken: its writer takes the slot itself, or leaves it to the holder it finds, after the write.
    pub(crate) fn take_mail(&self) -> Option<Mail> {
        let seq = self.seq.load(Ordering::Acquire);
        if seq & 1 == 1 || seq == self.applied.load(Ordering::Relaxed) {
            return None;
        }
        let kind = self.kind.load(Ordering::Relaxed);
        let value = self.value.load(Ordering::Relaxed);
        let due = self.due.load(Ordering::Relaxed);
        fence(Ordering::Acquire);
        if self.seq.load(Ordering::Relaxed) != seq {
            return None;
        }
        self.applied.store(seq, Ordering::Relaxed);
        Some(if kind == CANCEL {
            Mail::Cancel
        } else {
            Mail::Request { value, due }
        })
    }

    /// A dispatched post whose slot was held: it comes round again, so the dispatch is not lost.
    pub(crate) fn retake(&self, token: u64) {
        self.retake_token.store(token, Ordering::Relaxed);
        self.retake.store(true, Ordering::Release);
        fence(Ordering::SeqCst);
    }

    pub(crate) fn take_retake(&self) -> Option<u64> {
        self.retake
            .swap(false, Ordering::AcqRel)
            .then(|| self.retake_token.load(Ordering::Relaxed))
    }

    /// The queue did not take the post with this number.
    pub(crate) fn refuse(&self, number: u64) {
        self.refused.fetch_max(number, Ordering::AcqRel);
        fence(Ordering::SeqCst);
    }

    pub(crate) fn take_refused(&self) -> Option<u64> {
        Some(self.refused.swap(0, Ordering::AcqRel)).filter(|number| *number != 0)
    }

    /// The slot's decision may have changed: look again.
    pub(crate) fn look(&self) {
        self.look.store(true, Ordering::Release);
        fence(Ordering::SeqCst);
    }

    pub(crate) fn take_look(&self) -> bool {
        self.look.swap(false, Ordering::AcqRel)
    }

    /// Whether something waits for the holder: for the one that has just let go of the slot, and for the loop's thread
    /// after a put.
    pub(crate) fn pending_after_release(&self) -> bool {
        fence(Ordering::SeqCst);
        let seq = self.seq.load(Ordering::Acquire);
        (seq & 1 == 0 && seq != self.applied.load(Ordering::Relaxed))
            || self.retake.load(Ordering::Acquire)
            || self.refused.load(Ordering::Acquire) != 0
            || self.look.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::thread;

    use super::*;

    #[test]
    fn nothing_is_pending_until_something_is_put() {
        let hand = Handoff::default();
        assert!(!hand.pending_after_release());
        assert_eq!(hand.take_mail(), None);
        assert_eq!(hand.take_retake(), None);
        assert_eq!(hand.take_refused(), None);
        assert!(!hand.take_look());
    }

    #[test]
    fn the_newest_mail_wins_and_is_taken_once() {
        let hand = Handoff::default();
        hand.put(Mail::Request { value: 1, due: 10 });
        hand.put(Mail::Request { value: 2, due: 20 });
        assert!(hand.pending_after_release());
        assert_eq!(hand.take_mail(), Some(Mail::Request { value: 2, due: 20 }));
        assert_eq!(hand.take_mail(), None, "taken once");
        assert!(!hand.pending_after_release());
        hand.put(Mail::Cancel);
        assert_eq!(hand.take_mail(), Some(Mail::Cancel), "a cancel replaces a request, and the other way round");
        hand.put(Mail::Request { value: u64::MAX, due: u64::MAX });
        assert_eq!(hand.take_mail(), Some(Mail::Request { value: u64::MAX, due: u64::MAX }));
    }

    #[test]
    fn a_mail_being_written_is_not_taken_and_is_not_pending() {
        let hand = Handoff::default();
        hand.put(Mail::Request { value: 1, due: 1 });
        // As if a put were halfway: the writer will look after it finishes.
        hand.seq.store(hand.seq.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
        assert_eq!(hand.take_mail(), None);
        assert!(!hand.pending_after_release());
    }

    #[test]
    fn a_retake_a_refusal_and_a_look_are_pending_until_taken() {
        let hand = Handoff::default();
        hand.retake(0xfeed);
        assert!(hand.pending_after_release());
        assert_eq!(hand.take_retake(), Some(0xfeed));
        assert_eq!(hand.take_retake(), None);
        hand.refuse(3);
        hand.refuse(7);
        hand.refuse(5);
        assert!(hand.pending_after_release());
        assert_eq!(hand.take_refused(), Some(7), "the highest number");
        assert_eq!(hand.take_refused(), None);
        hand.look();
        assert!(hand.pending_after_release());
        assert!(hand.take_look());
        assert!(!hand.take_look());
        assert!(!hand.pending_after_release());
    }

    /// The writer never waits and the reader never sees half a mail: the value and the due time of every mail taken
    /// belong together, and the newest mail is taken at the end.
    #[test]
    fn a_reader_on_another_thread_never_sees_a_torn_mail() {
        const PUTS: u64 = 50_000;
        let hand = Arc::new(Handoff::default());
        let done = Arc::new(AtomicBool::new(false));
        let reader = {
            let (hand, done) = (Arc::clone(&hand), Arc::clone(&done));
            thread::spawn(move || {
                let (mut taken, mut last) = (0u64, 0u64);
                loop {
                    let finished = done.load(Ordering::Acquire);
                    if let Some(Mail::Request { value, due }) = hand.take_mail() {
                        assert_eq!(due, value.wrapping_mul(3), "a torn mail");
                        assert!(value > last, "mail goes back in time: {value} after {last}");
                        last = value;
                        taken += 1;
                    }
                    if finished && !hand.pending_after_release() {
                        return (taken, last);
                    }
                }
            })
        };
        for value in 1..=PUTS {
            hand.put(Mail::Request {
                value,
                due: value.wrapping_mul(3),
            });
        }
        done.store(true, Ordering::Release);
        let (taken, last) = reader.join().expect("the reader");
        assert!(taken >= 1);
        assert_eq!(last, PUTS, "the newest mail was taken");
    }
}
