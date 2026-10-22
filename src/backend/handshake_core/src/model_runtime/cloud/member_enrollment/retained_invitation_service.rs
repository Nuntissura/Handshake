// Prospective PRIVATE Core host adapter beside the reviewed writer.
// Reuses EXACT 26906B/ced000c53465c04f57285c3aa1e6336b90c724522907ea7ffabde25d7b892920
// original_invitation_writer API. No change to that body or closed 915074 crypto.
// RAM ONLY; no AppState/lifecycle mount, DB controller, files or runtime proof.
#[cfg(feature = "os-keychain")]
mod retained_invitation_service {
    use super::original_invitation_writer::{
        DrainObservation, IssueAccess, OriginalInvitationWriter,
        OriginalIssueIntent, Stop,
    };
    use handshake_member_enrollment::command_plan::EnrollmentDescriptor;
    use std::{
        any::Any,
        collections::BTreeMap,
        sync::{Arc, Mutex, MutexGuard, Weak},
    };
    use tokio::time::Instant;
    use uuid::Uuid;

    // Complete original input, including the ORIGINAL monotonic deadline.
    // No Debug/Serialize: caller contexts and source references stay protected.
    #[derive(Clone, PartialEq, Eq)]
    pub(crate) struct OriginalSubmission {
        pub intent: OriginalIssueIntent,
        pub original_deadline: Instant,
    }

    // A panic's ACTUAL original payload is retained once, behind its own mutex.
    // Shared rejection views point to this SAME object, never formatted copies.
    pub(crate) enum RawAdmissionFailure {
        Writer(Stop),
        Panic(Mutex<Box<dyn Any + Send>>),
    }

    enum Admission {
        Reserved,
        Accepted(IssueAccess),
        Rejected(Arc<RawAdmissionFailure>),
    }

    struct EntryHistory {
        drains: Vec<DrainObservation>,
        completed: Option<Arc<CompletedCustody>>,
        acknowledged_at: Option<Instant>,
    }

    struct Entry {
        original: OriginalSubmission,
        admission: Mutex<Admission>,
        history: Mutex<EntryHistory>,
    }

    struct Registry {
        accepting: bool,
        entries: BTreeMap<Uuid, Arc<Entry>>,
        shutdown: Option<Arc<ShutdownSet>>,
    }

    struct Service {
        // The service owns the reviewed writer itself, not just an await future.
        // That writer owns the actual custody map, handles, candidate, get/put
        // results, Account readbacks, panic and JoinError for every attempt.
        writer: OriginalInvitationWriter,
        registry: Mutex<Registry>,
    }

    // Host lifecycle owner keeps this strong handle until reconciliation and
    // shutdown are complete. A client receives only a Ticket, never this owner.
    // There is NO Drop implementation pretending to perform async shutdown.
    pub(crate) struct InvitationServiceOwner {
        service: Arc<Service>,
    }

    #[derive(Clone)]
    pub(crate) struct Ticket {
        owner: Weak<Service>,
        entry: Arc<Entry>,
    }

    // Data receipt only: actual successful worker JOIN plus the writer's
    // recovered canonical Bound descriptor. Not current recipient authority,
    // a one-use receipt, OS create-only proof or cross-store atomicity proof.
    // Private fields prevent construction by consumers; weak entry avoids cycles.
    pub(crate) struct CompletedCustody {
        owner: Weak<Service>,
        entry: Weak<Entry>,
        operation_id: Uuid,
        descriptor: EnrollmentDescriptor,
        worker_finished_at: Instant,
    }

    pub(crate) enum RejectionReason {
        AdmissionClosed,
        OriginalNotRegistered,
        DifferentOriginal { retained: Ticket },
        RegistryPoisoned,
        // The original returned Stop or caught Box<Any> is shared intact.
        Writer { retained: Ticket, failure: Arc<RawAdmissionFailure> },
        AdmissionUnobserved { retained: Ticket },
    }

    // Even rejection returns the FULL owned rejected input. No caller has to
    // reconstruct the original tuple/deadline from a flattened error string.
    pub(crate) struct RejectedSubmission {
        pub original: OriginalSubmission,
        pub reason: RejectionReason,
    }

    pub(crate) enum CustodyObservation {
        Completed(Arc<CompletedCustody>),
        Unknown {
            worker: Option<DrainObservation>,
            admission_failure: Option<Arc<RawAdmissionFailure>>,
        },
    }

    pub(crate) struct EntryDrain {
        pub retained: Ticket,
        pub observation: CustodyObservation,
    }

    pub(crate) enum AcknowledgeError {
        ForeignOwner,
        DifferentOriginal,
        CompletionUnobserved,
        RegistryPoisoned,
    }

    pub(crate) struct CustodyAcknowledgement {
        pub operation_id: Uuid,
        pub first_acknowledged_at: Instant,
    }

    struct ShutdownSet {
        entries: Vec<Ticket>,
        // Cache complete reports, including Unknown and their retained Tickets.
        // No replacing the original shutdown set or reconstructing inputs.
        reports: Mutex<Vec<Arc<ShutdownReport>>>,
    }

    #[derive(Clone)]
    pub(crate) struct ShutdownAccess {
        retained: Arc<ShutdownSet>,
    }

    pub(crate) struct ShutdownReport {
        pub entries: Vec<EntryDrain>,
    }

    fn retained_lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
        value.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    impl InvitationServiceOwner {
        pub(crate) fn new(writer: OriginalInvitationWriter) -> Self {
            Self {
                service: Arc::new(Service {
                    writer,
                    registry: Mutex::new(Registry {
                        accepting: true, entries: BTreeMap::new(), shutdown: None,
                    }),
                }),
            }
        }

        // Synchronous reservation + admission. There is NO cancellable await
        // before the full original input and its unique Entry are retained.
        pub(crate) fn reserve(
            &self, original: OriginalSubmission,
        ) -> Result<Ticket, RejectedSubmission> {
            let mut registry = match self.service.registry.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    // Preserve the inventory; fail closed for new admission.
                    let mut retained = poisoned.into_inner();
                    retained.accepting = false;
                    return Err(RejectedSubmission {
                        original, reason: RejectionReason::RegistryPoisoned,
                    });
                }
            };
            let operation = original.intent.operation_id;
            if let Some(entry) = registry.entries.get(&operation) {
                let ticket = Ticket {
                    owner: Arc::downgrade(&self.service), entry: Arc::clone(entry),
                };
                if entry.original != original {
                    return Err(RejectedSubmission {
                        original,
                        reason: RejectionReason::DifferentOriginal { retained: ticket },
                    });
                }
                // Exact redelivery observes the SAME Entry even while closing.
                // Rejected/Pending entries never call writer.start again.
                return reservation_result(ticket, original);
            }
            if !registry.accepting {
                return Err(RejectedSubmission {
                    original, reason: RejectionReason::AdmissionClosed,
                });
            }
            let entry = Arc::new(Entry {
                original: original.clone(),
                admission: Mutex::new(Admission::Reserved),
                history: Mutex::new(EntryHistory {
                    drains: Vec::new(), completed: None, acknowledged_at: None,
                }),
            });
            registry.entries.insert(operation, Arc::clone(&entry));
            let ticket = Ticket {
                owner: Arc::downgrade(&self.service), entry: Arc::clone(&entry),
            };
            // SAME registry gate covers reservation, writer admission and the
            // shutdown boundary. start() is synchronous; no std lock crosses an
            // await. The reviewed writer retains its real custody BEFORE spawn.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.service.writer.start(
                    original.intent.clone(), original.original_deadline,
                )
            }));
            *retained_lock(&entry.admission) = match result {
                Ok(Ok(access)) => Admission::Accepted(access),
                Ok(Err(error)) => Admission::Rejected(Arc::new(
                    RawAdmissionFailure::Writer(error),
                )),
                Err(payload) => Admission::Rejected(Arc::new(
                    RawAdmissionFailure::Panic(Mutex::new(payload)),
                )),
            };
            // No operation-specific result is discarded on rejection.
            reservation_result(ticket, original)
        }

        pub(crate) fn reobserve(
            &self, original: OriginalSubmission,
        ) -> Result<Ticket, RejectedSubmission> {
            // Observation only: never creates an entry or starts a worker.
            let registry = match self.service.registry.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    let mut retained = poisoned.into_inner();
                    retained.accepting = false;
                    return Err(RejectedSubmission {
                        original, reason: RejectionReason::RegistryPoisoned,
                    });
                }
            };
            let Some(entry) = registry.entries.get(&original.intent.operation_id) else {
                return Err(RejectedSubmission {
                    original, reason: RejectionReason::OriginalNotRegistered,
                });
            };
            let ticket = Ticket {
                owner: Arc::downgrade(&self.service), entry: Arc::clone(entry),
            };
            if entry.original != original {
                return Err(RejectedSubmission {
                    original,
                    reason: RejectionReason::DifferentOriginal { retained: ticket },
                });
            }
            reservation_result(ticket, original)
        }

        pub(crate) fn begin_shutdown(&self) -> ShutdownAccess {
            // A poisoned gate also closes admission. Existing custody survives.
            let mut registry = retained_lock(&self.service.registry);
            registry.accepting = false;
            if let Some(existing) = &registry.shutdown {
                return ShutdownAccess { retained: Arc::clone(existing) };
            }
            let entries = registry.entries.values().map(|entry| Ticket {
                owner: Arc::downgrade(&self.service), entry: Arc::clone(entry),
            }).collect();
            let set = Arc::new(ShutdownSet {
                entries, reports: Mutex::new(Vec::new()),
            });
            registry.shutdown = Some(Arc::clone(&set));
            ShutdownAccess { retained: set }
        }

        pub(crate) fn acknowledge(
            &self, ticket: &Ticket, completed: &Arc<CompletedCustody>,
        ) -> Result<CustodyAcknowledgement, AcknowledgeError> {
            let owner = Arc::downgrade(&self.service);
            if !Weak::ptr_eq(&owner, &ticket.owner)
                || !Weak::ptr_eq(&owner, &completed.owner)
            {
                return Err(AcknowledgeError::ForeignOwner);
            }
            let registry = match self.service.registry.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    let mut retained = poisoned.into_inner();
                    retained.accepting = false;
                    return Err(AcknowledgeError::RegistryPoisoned);
                }
            };
            let operation = ticket.entry.original.intent.operation_id;
            let Some(entry) = registry.entries.get(&operation) else {
                return Err(AcknowledgeError::DifferentOriginal);
            };
            if !Arc::ptr_eq(entry, &ticket.entry)
                || completed.operation_id != operation
                || !Weak::ptr_eq(&Arc::downgrade(entry), &completed.entry)
            {
                return Err(AcknowledgeError::DifferentOriginal);
            }
            let mut history = retained_lock(&entry.history);
            if !history.completed.as_ref().is_some_and(|retained| {
                Arc::ptr_eq(retained, completed)
            }) {
                return Err(AcknowledgeError::CompletionUnobserved);
            }
            let first = *history.acknowledged_at.get_or_insert_with(Instant::now);
            // Custody acknowledgement ONLY. Keep original/raw records; no
            // eviction, lock-file deletion, secret deletion or DB transition.
            Ok(CustodyAcknowledgement {
                operation_id: operation, first_acknowledged_at: first,
            })
        }
    }

    fn reservation_result(
        ticket: Ticket, original: OriginalSubmission,
    ) -> Result<Ticket, RejectedSubmission> {
        // Clone just a shared raw-failure reference, never flatten its payload.
        let rejection = match &*retained_lock(&ticket.entry.admission) {
            Admission::Accepted(_) => None,
            Admission::Rejected(failure) => Some(RejectionReason::Writer {
                retained: ticket.clone(), failure: Arc::clone(failure),
            }),
            Admission::Reserved => Some(RejectionReason::AdmissionUnobserved {
                retained: ticket.clone(),
            }),
        };
        match rejection {
            None => Ok(ticket),
            Some(reason) => Err(RejectedSubmission { original, reason }),
        }
    }

    impl Ticket {
        pub(crate) fn original(&self) -> OriginalSubmission {
            self.entry.original.clone()
        }

        pub(crate) fn same_entry(&self, other: &Self) -> bool {
            Weak::ptr_eq(&self.owner, &other.owner)
                && Arc::ptr_eq(&self.entry, &other.entry)
        }

        pub(crate) fn admission_failure(&self) -> Option<Arc<RawAdmissionFailure>> {
            match &*retained_lock(&self.entry.admission) {
                Admission::Rejected(failure) => Some(Arc::clone(failure)),
                _ => None,
            }
        }

        pub(crate) fn request_cancel(&self) {
            let access = match &*retained_lock(&self.entry.admission) {
                Admission::Accepted(access) => Some(access.clone()),
                _ => None,
            };
            if let Some(access) = access { access.request_cancel(); }
            // No abort, worker replacement, deadline extension or Drop action.
        }

        pub(crate) async fn drain(&self) -> EntryDrain {
            let access = match &*retained_lock(&self.entry.admission) {
                Admission::Accepted(access) => Some(access.clone()),
                _ => None,
            };
            let Some(access) = access else {
                return EntryDrain {
                    retained: self.clone(),
                    observation: CustodyObservation::Unknown {
                        worker: None, admission_failure: self.admission_failure(),
                    },
                };
            };
            // Only a cloned access crosses await. The Entry and reviewed
            // writer's registry retain the SAME real worker and whole results.
            let worker = access.drain().await;
            let descriptor = if worker.joined {
                access.recovered_descriptor()
            } else {
                None
            };
            let mut history = retained_lock(&self.entry.history);
            history.drains.push(worker); // Cache BEFORE any later await.
            if history.completed.is_none() {
                if let (Some(descriptor), Some(finished)) =
                    (descriptor, worker.worker_finished_at)
                {
                    if descriptor.operation_id() == self.entry.original.intent.operation_id {
                        history.completed = Some(Arc::new(CompletedCustody {
                            owner: self.owner.clone(), entry: Arc::downgrade(&self.entry),
                            operation_id: descriptor.operation_id(),
                            descriptor, worker_finished_at: finished,
                        }));
                    }
                }
            }
            let observation = match &history.completed {
                Some(completed) => CustodyObservation::Completed(Arc::clone(completed)),
                None => CustodyObservation::Unknown {
                    worker: Some(worker), admission_failure: None,
                },
            };
            EntryDrain { retained: self.clone(), observation }
        }
    }

    impl CompletedCustody {
        pub(crate) fn descriptor(&self) -> &EnrollmentDescriptor {
            &self.descriptor
        }

        pub(crate) fn worker_finished_at(&self) -> Instant {
            self.worker_finished_at
        }
    }

    impl ShutdownAccess {
        pub(crate) fn entries(&self) -> &[Ticket] {
            &self.retained.entries
        }

        pub(crate) fn request_cancel_all(&self) {
            // Explicit selected-service shutdown only; no global/process cancel.
            for entry in &self.retained.entries { entry.request_cancel(); }
        }

        pub(crate) async fn drain(&self) -> Arc<ShutdownReport> {
            // Each reviewed access uses its ORIGINAL retained deadline. No new
            // duration or fresh global budget. Drop of join_all drops awaiters,
            // never any retained real worker/handle/candidate/error.
            let entries = futures::future::join_all(
                self.retained.entries.iter().map(Ticket::drain),
            ).await;
            let report = Arc::new(ShutdownReport { entries });
            retained_lock(&self.retained.reports).push(Arc::clone(&report));
            report
        }
    }
    pub(crate) enum InvitationAdmissionError<E> {
        Closed { original: OriginalSubmission },
        RegistryPoisoned { original: OriginalSubmission },
        Writer { original: OriginalSubmission, failure: E },
        Service(RejectedSubmission),
    }


    struct InvitationRegistry {
        accepting: bool,
        owners: BTreeMap<uuid::Uuid, Arc<InvitationServiceOwner>>,
        shutdown: Option<Arc<InvitationShutdown>>,
    }

    pub(crate) struct InvitationLifecycle {
        registry: std::sync::Mutex<InvitationRegistry>,
    }

    struct InvitationShutdownEntry {
        owner: Arc<InvitationServiceOwner>,
        access: ShutdownAccess,
    }

    struct InvitationDrainReport {
        _services: Vec<Arc<ShutdownReport>>,
        _acknowledgements: Vec<Result<CustodyAcknowledgement, AcknowledgeError>>,
    }

    struct InvitationShutdown {
        entries: Vec<InvitationShutdownEntry>,
        reports: tokio::sync::Mutex<Vec<InvitationDrainReport>>,
    }

    impl InvitationLifecycle {
        pub(crate) fn new() -> Self {
            Self {
                registry: std::sync::Mutex::new(InvitationRegistry {
                    accepting: true,
                    owners: BTreeMap::new(),
                    shutdown: None,
                }),
            }
        }

        pub(crate) fn admit_original<E>(
            &self,
            original: OriginalSubmission,
            make_writer: impl FnOnce() -> Result<OriginalInvitationWriter, E>,
        ) -> Result<Ticket, InvitationAdmissionError<E>> {
            let mut registry = match self.registry.lock() {
                Ok(registry) => registry,
                Err(error) => {
                    let mut registry = error.into_inner();
                    registry.accepting = false;
                    return Err(InvitationAdmissionError::RegistryPoisoned { original });
                }
            };
            let operation = original.intent.operation_id;
            if let Some(owner) = registry.owners.get(&operation) {
                return owner.reserve(original).map_err(InvitationAdmissionError::Service);
            }
            if !registry.accepting {
                return Err(InvitationAdmissionError::Closed { original });
            }
            let writer = match make_writer() {
                Ok(writer) => writer,
                Err(failure) => return Err(InvitationAdmissionError::Writer { original, failure }),
            };
            let owner = Arc::new(InvitationServiceOwner::new(writer));
            registry.owners.insert(operation, Arc::clone(&owner));
            owner.reserve(original).map_err(InvitationAdmissionError::Service)
        }

        pub(crate) fn reobserve_original(
            &self,
            original: OriginalSubmission,
        ) -> Result<Option<Ticket>, InvitationAdmissionError<std::convert::Infallible>> {
            let registry = match self.registry.lock() {
                Ok(registry) => registry,
                Err(error) => {
                    let mut registry = error.into_inner();
                    registry.accepting = false;
                    return Err(InvitationAdmissionError::RegistryPoisoned { original });
                }
            };
            match registry.owners.get(&original.intent.operation_id) {
                Some(owner) => owner.reobserve(original).map(Some).map_err(InvitationAdmissionError::Service),
                None if registry.accepting => Ok(None),
                None => Err(InvitationAdmissionError::Closed { original }),
            }
        }

        fn shutdown_set(&self) -> Arc<InvitationShutdown> {
            let mut registry = self.registry.lock().unwrap_or_else(|error| error.into_inner());
            registry.accepting = false;
            if let Some(shutdown) = &registry.shutdown {
                return Arc::clone(shutdown);
            }
            let shutdown = Arc::new(InvitationShutdown {
                entries: registry.owners.values().map(|owner| InvitationShutdownEntry {
                    owner: Arc::clone(owner),
                    access: owner.begin_shutdown(),
                }).collect(),
                reports: tokio::sync::Mutex::new(Vec::new()),
            });
            registry.shutdown = Some(Arc::clone(&shutdown));
            shutdown
        }

        pub(crate) fn begin_shutdown(&self) {
            let shutdown = self.shutdown_set();
            for entry in &shutdown.entries {
                entry.access.request_cancel_all();
            }
        }

        pub(crate) async fn drain(&self) -> bool {
            self.begin_shutdown();
            let shutdown = self.shutdown_set();
            let reports = futures::future::join_all(
                shutdown.entries.iter().map(|entry| entry.access.drain()),
            ).await;
            let mut acknowledgements = Vec::new();
            let mut settled = true;
            for (entry, report) in shutdown.entries.iter().zip(&reports) {
                for item in &report.entries {
                    match &item.observation {
                        CustodyObservation::Completed(completed) => {
                            let acknowledgement = entry.owner.acknowledge(&item.retained, completed);
                            settled &= acknowledgement.is_ok();
                            settled &= completed.worker_finished_at() <= item.retained.original().original_deadline;
                            acknowledgements.push(acknowledgement);
                        }
                        CustodyObservation::Unknown { .. } => settled = false,
                    }
                }
            }
            shutdown.reports.lock().await.push(InvitationDrainReport {
                _services: reports,
                _acknowledgements: acknowledgements,
            });
            settled
        }
    }
}
// Mandatory mounting inputs/dependencies:
// - Core lifecycle owns InvitationServiceOwner strongly, constructs it with the
//   reviewed concrete writer/config/Account supplier, and routes ALL original
//   issue submissions through reserve(). Tickets cannot instantiate workers.
// - Actual shutdown owner calls begin_shutdown() synchronously before teardown,
//   retains that SAME shutdown set and service, drains/reobserves without
//   destroying them when Unknown remains, and preserves the runtime needed by
//   outstanding Account/vault workers. Service Drop itself does not drain.
// - Unknown and rejected records are retained; there is intentionally no eviction
//   API. Durable archival/reconciliation and any later safe reclamation require
//   a separately implemented protected host policy, not receipt-count heuristics.
// - CompletedCustody/acknowledge report worker/custody completion only. Account
//   still owns durable original-row CAS, credential fencing/current authority,
//   revocation and one-use recipient acceptance. No acceptance permission added.
// - No global leak, mem::forget, process-global cancellation, replacement deadline,
//   resubmitted worker, DB table, public tool or aggregate crate is introduced.
