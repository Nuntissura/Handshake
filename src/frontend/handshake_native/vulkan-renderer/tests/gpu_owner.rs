//! The owner on the machine's real Vulkan device, through its public surface: open, attach, lease, real work with a
//! readback, observe, settle, reclaim and close.
//!
//! No window and no surface. A machine without an acceptable device skips with one `HSK_GPU_SKIP` line, which is not a
//! pass of these cases, and fails where `HANDSHAKE_REQUIRE_VULKAN_GPU` is set. Faults a healthy machine does not
//! produce are in the crate's unit tests (`src/owner/fault_tests.rs`).

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AttachRefusal, AuthorizeRefusal, ByteQuantity, ClientId, DeviceClass,
    DeviceFate, DeviceHealth, DeviceRebindRefusal, GpuClient, GpuOwner, MemoryPlacement, ModuleId,
    NullDiagnostics, OwnerEvent, QuotaPolicy, RecordAborted, SettleOutcome, SettleRefusal,
    Settlement, SubmissionObservation, SubmissionRefusal, SubmissionState, SubmitRefusal,
    Unavailable, DESCRIPTOR, MAX_WAIT,
};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};
use support::gpu_env::{in_own_process, open, ran_in_its_own_process};
use support::gpu_work::Worker;
use support::{bounded_policy, policy, request};

const COMPLETED: SubmissionState = SubmissionState::Finished(SubmissionObservation::Completed);
const QUOTA: u64 = 1 << 30;

fn attach(owner: &GpuOwner, module: u32, client: u64) -> GpuClient {
    owner
        .attach_client(ModuleId(module), ClientId(client))
        .expect("free")
}

fn worker(client: &GpuClient) -> Worker {
    let claim = client.admit(&request(Worker::RESERVE)).expect("fits");
    Worker::new(client, claim)
}

#[test]
fn the_process_has_one_owner_on_a_hardware_device() {
    in_own_process!();
    let Some((owner, log)) = open(policy(QUOTA)) else {
        return;
    };
    let identity = owner.identity();
    assert!(!matches!(
        identity.class,
        DeviceClass::Cpu | DeviceClass::Other
    ));
    assert_eq!(identity.api_version[0], 1);
    assert!(identity.api_version[1] >= 3);
    for feature in &identity.features {
        assert!(DESCRIPTOR.contains(&format!("\"{feature}\"")), "{feature}");
    }
    let device = owner.device().expect("bound");
    assert_eq!(
        (device.key, device.generation, device.lost),
        (identity.key, 1, false)
    );
    assert_eq!(
        GpuOwner::open(policy(1), Arc::new(NullDiagnostics)).err(),
        Some(Unavailable::OwnerAlreadyOpen)
    );
    let client = attach(&owner, 1, 1);
    assert_eq!(client.identity().key, identity.key);
    assert_eq!(
        owner.attach_client(ModuleId(1), ClientId(1)).err(),
        Some(AttachRefusal::AlreadyAttached)
    );

    let memory = owner.memory_snapshot().expect("bound");
    assert!(!memory.heaps.is_empty());
    for heap in &memory.heaps {
        assert!(heap.size_bytes > 0);
        assert_eq!(
            heap.placement,
            MemoryPlacement::of(identity.class, heap.device_local)
        );
        // What the device does not say is never called dedicated.
        assert!(
            heap.placement != MemoryPlacement::Dedicated || identity.class == DeviceClass::Discrete
        );
        assert!(!matches!(heap.budget, ByteQuantity::Measured(_)));
    }

    assert_eq!(owner.probe(MAX_WAIT), Ok(DeviceHealth::Idle));
    assert_eq!(owner.poll(), Ok(()));
    assert_eq!(client.poll(), Ok(()));
    assert_eq!(owner.rebind_device(), Err(DeviceRebindRefusal::NotLost));
    let report = owner.close();
    assert!(report.is_settled(), "{report:?}");
    assert_eq!(owner.close(), report);
    assert_eq!(owner.rebind_device(), Err(DeviceRebindRefusal::Closed));
    assert!(owner.memory_snapshot().is_err());
    assert!(owner.probe(MAX_WAIT).is_err() && client.poll().is_err());
    assert_eq!(
        owner.attach_client(ModuleId(2), ClientId(1)).err(),
        Some(AttachRefusal::Closed)
    );
    assert_eq!(
        log.count(OwnerEvent::GenerationRetired {
            generation: 1,
            resolved: 0
        }),
        1
    );
    // Closed is enough: the slot is free although the handles still exist.
    let Some((again, _)) = open(policy(1)) else {
        panic!("the same device opens again")
    };
    assert!(again.close().is_settled());
}

#[test]
fn a_device_without_a_quota_admits_nothing() {
    in_own_process!();
    let Some((owner, _)) = open(QuotaPolicy::Unavailable) else {
        return;
    };
    assert_eq!(
        attach(&owner, 1, 1)
            .admit(&request(0))
            .expect_err("no quota"),
        AdmissionDenial::HostQuotaUnavailable
    );
    assert!(owner.close().is_settled());
}

#[test]
fn real_work_is_observed_complete_only_when_the_gpu_did_it() {
    in_own_process!();
    let Some((owner, log)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1, 1);
    let worker = worker(&client);
    assert_eq!(owner.usage().committed_bytes, worker.work.allocated_bytes());
    let events = log.events().len();
    for round in 1..=8u32 {
        let value = 0xA5A5_0000 | round;
        assert!(!worker.work.holds(value));
        let submission = client
            .submit(&worker.claim, |recording| {
                assert_eq!(recording.generation(), 1);
                worker.work.record(recording, value);
                Ok(())
            })
            .expect("submits");
        assert_eq!(
            (submission.generation(), submission.sequence()),
            (1, u64::from(round))
        );
        assert_eq!(submission.claim_info(), worker.claim.info());
        assert_eq!(client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
        assert!(
            worker.work.holds(value),
            "completed before the GPU was done"
        );
        assert_eq!(client.observe(&submission), Ok(COMPLETED));
    }
    assert_eq!(owner.usage().in_flight, 0);
    assert_eq!(log.events().len(), events, "ordinary work emits no event");
    let done = Settlement::clean(SettleOutcome::Completed);
    assert_eq!(
        client.settle(&worker.claim, done),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    let claim = worker.let_go(&client);
    assert!(matches!(
        client.settle(&claim, Settlement::clean(SettleOutcome::NoEffect)),
        Err(SettleRefusal::OutcomeContradicted { .. })
    ));
    let receipt = client.settle(&claim, done).expect("settles");
    assert_eq!(
        (receipt.released_bytes, receipt.observed.completed),
        (Worker::RESERVE, 8)
    );
    assert_eq!(
        client.settle(&claim, done),
        Err(SettleRefusal::DoubleRelease)
    );
    assert!(matches!(
        client.submit(&claim, |_| Ok(())),
        Err(SubmitRefusal::NotAuthorized(AuthorizeRefusal::Settled))
    ));
    assert!(owner.close().is_settled());
}

#[test]
fn a_recording_that_ends_without_work_submits_nothing() {
    in_own_process!();
    let Some((owner, _)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1, 1);
    let claim = client.admit(&request(16)).expect("fits");
    assert_eq!(
        client.submit(&claim, |_| Err(RecordAborted)).err(),
        Some(SubmitRefusal::RecordAborted)
    );
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.submit(&claim, |_| -> Result<(), RecordAborted> {
            panic!("recording failed")
        })
    }));
    assert!(panicked.is_err());
    // Revoked while recording: the second authorization refuses it.
    let revoked = client.submit(&claim, |_| {
        owner.revoke_client(claim.info().module, claim.info().client);
        Ok(())
    });
    assert_eq!(
        revoked.err(),
        Some(SubmitRefusal::NotAuthorized(AuthorizeRefusal::Revoked))
    );
    assert_eq!(
        (owner.usage().in_flight, owner.counters().submitted),
        (0, 0)
    );
    let canceled = Settlement::clean(SettleOutcome::Canceled);
    assert!(client.settle(&claim, canceled).is_ok());
    assert_eq!(
        client.admit(&request(1)).expect_err("revoked"),
        AdmissionDenial::ClientDetached
    );
    // The owner is usable after all of it, by the client itself with a new handle.
    let again = attach(&owner, 1, 1);
    let worker = worker(&again);
    let submission = again
        .submit(&worker.claim, |recording| {
            worker.work.record(recording, 5);
            Ok(())
        })
        .expect("submits");
    assert_eq!(again.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert!(worker.work.holds(5));
    worker.settle(&again);
    assert!(owner.close().is_settled());
}

/// Four clients submit from four threads without waiting between submissions, so that they meet at the queue. Values
/// that reach the queue in another order than they were handed out fail the submission.
#[test]
fn concurrent_clients_share_one_queue_in_fence_order() {
    const CLIENTS: u32 = 4;
    const ROUNDS: u32 = 16;
    in_own_process!();
    let Some((owner, _)) = open(bounded_policy(QUOTA, 8, ROUNDS)) else {
        return;
    };
    let clients: Vec<_> = (1..=CLIENTS)
        .map(|module| attach(&owner, module, 1))
        .collect();
    let workers: Vec<_> = clients.iter().map(worker).collect();
    let barrier = Barrier::new(clients.len());
    let mut submissions: Vec<_> = std::thread::scope(|scope| {
        let threads: Vec<_> = clients
            .iter()
            .zip(&workers)
            .map(|(client, worker)| {
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    (1..=ROUNDS)
                        .map(|round| {
                            let value = (client.module().0 << 16) | round;
                            client.submit(&worker.claim, |recording| {
                                worker.work.record(recording, value);
                                Ok(())
                            })
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        threads
            .into_iter()
            .flat_map(|thread| thread.join().expect("no panic"))
            .map(|submission| submission.expect("submits"))
            .collect()
    });
    submissions.sort_by_key(|submission| submission.sequence());
    let values: Vec<_> = submissions.iter().map(|one| one.sequence()).collect();
    assert_eq!(
        values,
        (1..=u64::from(CLIENTS * ROUNDS)).collect::<Vec<u64>>()
    );
    let last = submissions.last().expect("some");
    let its_client = &clients[(last.claim_info().module.0 - 1) as usize];
    assert_eq!(its_client.wait(last, MAX_WAIT), Ok(COMPLETED));
    // One fence for all of them: its value ends every submission before it, whoever asked.
    assert!(submissions.iter().all(|one| one.state() == COMPLETED));
    for (client, worker) in clients.iter().zip(workers) {
        assert!(worker.work.holds((client.module().0 << 16) | ROUNDS));
        worker.settle(client);
    }
    assert!(owner.close().is_settled());
}

#[test]
fn a_dropped_claim_and_submission_are_reclaimed_after_the_gpu_is_done() {
    in_own_process!();
    // One submission per claim, so that the second one needs the first to be seen finished.
    let Some((owner, log)) = open(bounded_policy(QUOTA, 8, 1)) else {
        return;
    };
    let client = attach(&owner, 1, 1);
    let worker = worker(&client);
    let dropped = client.admit(&request(4096)).expect("fits");
    let subject = support::claim_subject(dropped.info());
    let submission = client
        .submit(&dropped, |recording| {
            worker.work.record(recording, 21);
            Ok(())
        })
        .expect("submits");
    drop(submission);
    drop(dropped);
    let end = Instant::now() + MAX_WAIT;
    while owner.counters().reclaimed == 0 {
        assert!(Instant::now() < end, "not reclaimed");
        assert_eq!(owner.usage().abandoned_claims, 1);
        owner.poll().expect("healthy");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(worker.work.holds(21), "reclaimed before the GPU was done");
    assert_eq!(
        log.count(OwnerEvent::ClaimReclaimed {
            subject,
            released_bytes: 4096
        }),
        1
    );
    // A dropped submission alone blocks nobody for ever either: nobody asks for it, and it clears.
    for value in [22, 23, 24] {
        let end = Instant::now() + MAX_WAIT;
        loop {
            match client.submit(&worker.claim, |recording| {
                worker.work.record(recording, value);
                Ok(())
            }) {
                Ok(_) => break,
                Err(SubmitRefusal::NotAuthorized(AuthorizeRefusal::InFlightLimit { .. })) => {
                    assert!(Instant::now() < end, "stays full");
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(other) => panic!("{other:?}"),
            }
        }
    }
    assert_eq!(owner.probe(MAX_WAIT), Ok(DeviceHealth::Idle));
    assert!(worker.work.holds(24));
    worker.settle(&client);
    assert!(owner.close().is_settled());
}

#[test]
fn a_leased_device_outlives_the_close_of_its_owner() {
    in_own_process!();
    let Some((owner, log)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1, 1);
    let claim = client.admit(&request(16)).expect("fits");
    let lease = client.lease(&claim).expect("authorized");
    assert_eq!((lease.generation(), lease.claim()), (1, claim.id()));
    let handles = lease.handles().expect("may use its device").clone();
    assert!(handles.get_instance_proc_addr != 0 && handles.instance != 0);
    assert!(handles.physical_device != 0 && handles.device != 0);
    assert!(handles.features.contains(&"dynamicRendering"));
    let canceled = Settlement::clean(SettleOutcome::Canceled);
    assert_eq!(
        client.settle(&claim, canceled),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    let report = owner.close();
    assert_eq!(
        (report.device, report.leases),
        (DeviceFate::Held { holders: 1 }, 1)
    );
    assert!(!report.is_settled());
    assert_eq!(
        log.count(OwnerEvent::DeviceRetainedAtClose {
            generation: 1,
            holders: 1
        }),
        1
    );
    // The leased device is still the process's one device.
    assert_eq!(
        GpuOwner::open(policy(1), Arc::new(NullDiagnostics)).err(),
        Some(Unavailable::OwnerAlreadyOpen)
    );
    // Its holder is told on its next call, and can still clean up.
    assert_eq!(
        lease.handles().err(),
        Some(AuthorizeRefusal::DeviceUnavailable)
    );
    assert_eq!(lease.handles_to_release(), &handles);
    let ended = OwnerEvent::RetainedDeviceDestroyed {
        generation: 1,
        resolved: 0,
    };
    assert_eq!(log.count(ended), 0);
    drop(lease);
    assert_eq!(log.count(ended), 1);
    assert!(client.settle(&claim, canceled).is_ok());
    let Some((again, _)) = open(policy(1)) else {
        panic!("the same device opens again")
    };
    assert!(again.close().is_settled());
}

#[test]
fn a_close_drains_the_queue_and_another_owner_knows_nothing_of_it() {
    in_own_process!();
    let Some((first, _)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&first, 1, 1);
    let worker = worker(&client);
    let other = attach(&first, 1, 2);
    let submission = client
        .submit(&worker.claim, |recording| {
            worker.work.record(recording, 77);
            Ok(())
        })
        .expect("submits");
    assert_eq!(other.observe(&submission), Err(SubmissionRefusal::Unknown));
    assert_eq!(
        other.lease(&worker.claim).err(),
        Some(AuthorizeRefusal::Unknown)
    );
    // Closing drains the queue, so the submission ends as completed and not as lost.
    let report = first.close();
    assert_eq!((report.in_flight, report.open_claims), (0, 1));
    assert_eq!(report.device, DeviceFate::Held { holders: 1 });
    assert_eq!(submission.state(), COMPLETED);
    assert!(worker.work.holds(77), "drained before the GPU was done");
    assert_eq!(client.observe(&submission), Ok(COMPLETED));
    let claim = worker.let_go(&client);
    let done = Settlement::clean(SettleOutcome::Completed);
    let Some((second, _)) = open(policy(QUOTA)) else {
        panic!("the same device opens again")
    };
    let stranger = attach(&second, 1, 1);
    assert_eq!(
        stranger.observe(&submission),
        Err(SubmissionRefusal::Unknown)
    );
    assert_eq!(
        stranger.lease(&claim).err(),
        Some(AuthorizeRefusal::Unknown)
    );
    assert_eq!(
        stranger.settle(&claim, done),
        Err(SettleRefusal::StaleClaim)
    );
    assert!(client.settle(&claim, done).is_ok());
    assert!(second.close().is_settled());
}

/// Windows looks beside the executable before it looks in the system directory, unless it is told not to. A file there
/// that is no library makes the difference visible: it is either never tried, or no loader is found.
#[cfg(windows)]
#[test]
fn a_vulkan_library_beside_the_executable_is_never_tried() {
    const CONTENT: &[u8] = b"not a library";
    struct Planted(std::path::PathBuf);
    impl Drop for Planted {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    in_own_process!();
    let path = std::env::current_exe()
        .expect("this executable")
        .with_file_name("vulkan-1.dll");
    // Nobody's library is overwritten; an earlier run of this test may have been killed.
    let free = !path.exists() || std::fs::read(&path).is_ok_and(|bytes| bytes == CONTENT);
    assert!(free, "not from this test: {}", path.display());
    std::fs::write(&path, CONTENT).expect("the build directory is writable");
    let planted = Planted(path);
    let opened = GpuOwner::open(policy(1), Arc::new(NullDiagnostics));
    drop(planted);
    match opened {
        Ok(owner) => assert!(owner.close().is_settled()),
        // Without the file there is none either, and the test skips or fails as the others do.
        Err(Unavailable::LoaderMissing) => {
            let found = open(policy(1));
            assert!(found.is_none(), "the file beside the executable was tried");
        }
        Err(_) => {
            assert!(open(policy(1)).is_none());
        }
    }
}
