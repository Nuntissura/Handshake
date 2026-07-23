use std::{
    collections::{BTreeSet, HashSet},
    sync::{mpsc, Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

use handshake_core::model_runtime::{
    CancellationToken, ModelId, RuntimeActivityKind, RuntimeActivityRegistrationError,
    RuntimeActivityTracker, RuntimeQuiesceError,
};

fn release_all(release: &Arc<(Mutex<bool>, Condvar)>) {
    let (released, wake) = &**release;
    *released.lock().unwrap() = true;
    wake.notify_all();
}

fn wait_for_release(release: &Arc<(Mutex<bool>, Condvar)>) {
    let (released, wake) = &**release;
    let mut released = released.lock().unwrap();
    while !*released {
        released = wake.wait(released).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generation_guards_are_unique_cancelled_and_worker_owned_after_stream_drop() {
    let tracker = RuntimeActivityTracker::new();
    let first_model_id = ModelId::new_v7();
    let second_model_id = ModelId::new_v7();
    let first_cancel = CancellationToken::new();
    let second_cancel = CancellationToken::new();
    let first_guard = tracker
        .try_register(
            first_model_id,
            RuntimeActivityKind::Generate,
            Some(first_cancel.clone()),
        )
        .expect("first generation admission");
    let second_guard = tracker
        .try_register(
            second_model_id,
            RuntimeActivityKind::Generate,
            Some(second_cancel.clone()),
        )
        .expect("second generation admission");

    assert_ne!(first_guard.id(), second_guard.id());
    let ids = tracker
        .active_operations()
        .into_iter()
        .map(|activity| activity.id.get())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 2, "every generation has a unique work id");
    assert_eq!(
        tracker
            .active_operations()
            .into_iter()
            .map(|activity| activity.model_id)
            .collect::<HashSet<_>>(),
        HashSet::from([first_model_id, second_model_id])
    );

    // This receiver is the consumer side of the generated-token stream. The
    // worker owns both the sender and the activity guard, so dropping the
    // receiver cannot make either generation look quiescent.
    let (stream_sender, stream_receiver) = tokio::sync::mpsc::unbounded_channel::<()>();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let (cancelled_sender, cancelled_receiver) = mpsc::channel();

    let first_worker = {
        let release = Arc::clone(&release);
        let cancelled_sender = cancelled_sender.clone();
        let stream_sender = stream_sender.clone();
        thread::spawn(move || {
            let _guard = first_guard;
            while !first_cancel.is_cancelled() {
                thread::yield_now();
            }
            let _ = stream_sender.send(());
            cancelled_sender.send("first").unwrap();
            wait_for_release(&release);
        })
    };
    let second_worker = {
        let release = Arc::clone(&release);
        let cancelled_sender = cancelled_sender.clone();
        thread::spawn(move || {
            let _guard = second_guard;
            while !second_cancel.is_cancelled() {
                thread::yield_now();
            }
            let _ = stream_sender.send(());
            cancelled_sender.send("second").unwrap();
            wait_for_release(&release);
        })
    };
    drop(cancelled_sender);
    drop(stream_receiver);

    let quiesce = {
        let tracker = tracker.clone();
        tokio::spawn(async move { tracker.quiesce(Duration::from_secs(2)).await })
    };

    let mut cancelled = tokio::task::spawn_blocking(move || {
        [
            cancelled_receiver.recv_timeout(Duration::from_secs(1)),
            cancelled_receiver.recv_timeout(Duration::from_secs(1)),
        ]
    })
    .await
    .expect("cancellation observer joins")
    .into_iter()
    .map(|observed| observed.expect("each generation observes cancellation"))
    .collect::<Vec<_>>();
    cancelled.sort_unstable();
    assert_eq!(cancelled, vec!["first", "second"]);

    assert!(matches!(
        tracker.try_register(first_model_id, RuntimeActivityKind::Score, None),
        Err(RuntimeActivityRegistrationError::Quiescing {
            kind: RuntimeActivityKind::Score
        })
    ));
    assert!(
        !quiesce.is_finished(),
        "quiescence waits for workers, not the dropped stream"
    );

    release_all(&release);
    quiesce
        .await
        .expect("quiesce task joins")
        .expect("workers drain before the shared deadline");
    first_worker.join().expect("first worker joins");
    second_worker.join().expect("second worker joins");
    assert!(tracker.active_operations().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_worker_timeout_is_typed_and_never_false_success() {
    let tracker = RuntimeActivityTracker::new();
    let score_model_id = ModelId::new_v7();
    let embed_model_id = ModelId::new_v7();
    let score_guard = tracker
        .try_register(score_model_id, RuntimeActivityKind::Score, None)
        .expect("score admission");
    let embed_guard = tracker
        .try_register(embed_model_id, RuntimeActivityKind::Embed, None)
        .expect("embed admission");
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let (started_sender, started_receiver) = mpsc::channel();

    let score_worker = {
        let release = Arc::clone(&release);
        let started_sender = started_sender.clone();
        tokio::task::spawn_blocking(move || {
            let _guard = score_guard;
            started_sender.send(RuntimeActivityKind::Score).unwrap();
            wait_for_release(&release);
        })
    };
    let embed_worker = {
        let release = Arc::clone(&release);
        tokio::task::spawn_blocking(move || {
            let _guard = embed_guard;
            started_sender.send(RuntimeActivityKind::Embed).unwrap();
            wait_for_release(&release);
        })
    };

    let started = tokio::task::spawn_blocking(move || {
        [
            started_receiver.recv_timeout(Duration::from_secs(1)),
            started_receiver.recv_timeout(Duration::from_secs(1)),
        ]
    })
    .await
    .expect("start observer joins");
    assert!(started.into_iter().all(|result| result.is_ok()));

    // Aborting a spawn_blocking JoinHandle does not stop a running worker. The
    // guard is inside that worker, so the tracker must still report both jobs.
    score_worker.abort();
    embed_worker.abort();

    let error = tracker
        .quiesce(Duration::from_millis(50))
        .await
        .expect_err("live blocking workers cannot quiesce successfully");
    let RuntimeQuiesceError::TimedOut { timeout, remaining } = error else {
        panic!("expected typed timeout, got {error:?}");
    };
    assert_eq!(timeout, Duration::from_millis(50));
    assert_eq!(remaining.len(), 2);
    assert_eq!(
        remaining
            .iter()
            .map(|activity| activity.model_id)
            .collect::<HashSet<_>>(),
        HashSet::from([score_model_id, embed_model_id])
    );
    assert_eq!(
        remaining
            .into_iter()
            .map(|activity| activity.kind)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([RuntimeActivityKind::Score, RuntimeActivityKind::Embed])
    );
    assert!(!tracker.is_accepting());

    release_all(&release);
    tracker
        .quiesce(Duration::from_secs(1))
        .await
        .expect("a later bounded wait observes both worker guards drop");
    assert!(tracker.active_operations().is_empty());
}
