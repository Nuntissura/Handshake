// Prospective extension beside the closed 9150741f recover/verify helper.
// RAM ONLY. AccountOriginalController is REQUIRED future protected Account
// integration; there is deliberately no implementation or permissive default.
// Reuses PURPOSE, original_secret_reference, canonical_secret, purpose_digest,
// currently_valid from that helper. No change to its crypto or recipient ABI.
#[cfg(feature = "os-keychain")]
mod original_invitation_writer {
    use super::{
        canonical_secret, currently_valid, original_secret_reference,
        purpose_digest, PURPOSE,
    };
    use crate::model_runtime::cloud::secrets_vault::{
        OsKeychainSecretsVault, SecretsVault, SecretsVaultError,
    };
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use fs2::FileExt;
    use handshake_member_enrollment::command_plan::{
        EnrollmentDescriptor, IdentityScope, SourceRef, StagedIdentity, Timestamp,
    };
    use sha2::{Digest, Sha256};
    use std::{
        any::Any,
        collections::BTreeMap,
        error::Error,
        fs::{File, OpenOptions},
        future::Future,
        path::{Path, PathBuf},
        pin::Pin,
        sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex, MutexGuard},
        task::Poll,
    };
    use subtle::ConstantTimeEq;
    use tokio::{runtime::Handle, task::{JoinError, JoinHandle}, time::Instant};
    use uuid::Uuid;
    use zeroize::Zeroizing;

    // NEW prospective Account input. These exact immutable values, including
    // both source refs and every captured expiry, must exist in ONE durable
    // original row BEFORE start(). No placeholder descriptor/digest is used.
    #[derive(Clone, PartialEq, Eq)]
    pub(crate) struct OriginalIssueIntent {
        pub operation_id: Uuid,
        pub original_enroller: IdentityScope,
        pub staged: StagedIdentity,
        pub account_name: String,
        pub administrator_resource: String,
        pub enroll_source: SourceRef,
        pub assignment_source: SourceRef,
        pub issued_at: Timestamp,
        pub expires_at: Timestamp,
        pub session_expires_at: Timestamp,
        pub policy_expires_at: Option<Timestamp>,
        pub purpose: String,
        pub vault_namespace: String,
        pub vault_reference: String,
    }

    impl OriginalIssueIntent {
        fn descriptor(&self, digest: [u8; 32]) -> Result<EnrollmentDescriptor, Stop> {
            let value = EnrollmentDescriptor::new(
                self.operation_id, self.original_enroller.clone(), self.staged.clone(),
                self.account_name.clone(), self.administrator_resource.clone(),
                self.enroll_source.clone(), self.assignment_source.clone(),
                self.issued_at, self.expires_at, self.session_expires_at,
                self.policy_expires_at, digest,
            ).map_err(|_| Stop::InvalidOriginal)?;
            currently_valid(&value).map_err(|_| Stop::InvalidOriginal)?;
            Ok(value)
        }

        fn matches_descriptor(&self, value: &EnrollmentDescriptor) -> bool {
            // Rebuild with the ACTUAL bound digest, never a placeholder.
            self.descriptor(*value.token_digest()).as_ref() == Ok(value)
        }
    }

    // Opaque original-row concurrency revision chosen by Account, NOT a claimed
    // existing schema field or an OS-lock-derived authority token.
    #[derive(Clone, PartialEq, Eq)]
    pub(crate) struct AccountRevision(pub Vec<u8>);

    #[derive(Clone, PartialEq, Eq)]
    pub(crate) enum OriginalIssueState {
        Reserved,
        Preparing { claim_id: Uuid },
        WriteStarted(EnrollmentDescriptor),
        Bound(EnrollmentDescriptor),
    }

    #[derive(Clone, PartialEq, Eq)]
    pub(crate) struct OriginalIssueRow {
        pub intent: OriginalIssueIntent,
        pub revision: AccountRevision,
        pub state: OriginalIssueState,
    }

    pub(crate) type AccountError = Box<dyn Error + Send + Sync>;
    pub(crate) type AccountResult = Result<Option<OriginalIssueRow>, AccountError>;
    pub(crate) type AccountFuture<'a> =
        Pin<Box<dyn Future<Output = AccountResult> + Send + 'a>>;

    // Required supplier, not implemented here:
    // - reserve_original compares the FULL immutable original request and refs;
    //   never resets issued_at/expires_at, operation or a prior state.
    // - observe reads the SAME canonical original row; missing/denied/ambiguous
    //   observations never imply Reserved.
    // - claim_preparation atomically compares Reserved + expected revision and
    //   current authority, persists the exact unique claim_id before entropy.
    //   A different claim or any existing Preparing state cannot mint again.
    // - mark_write_started atomically compares expected original row+revision,
    //   current authorization/status and SAME Preparing claim; persists exact descriptor,
    //   token digest, namespace/reference BEFORE returning positive readback.
    // - bind_original atomically compares the SAME WriteStarted descriptor and
    //   revision, rereads current authority/status, and returns actual canonical
    //   Bound readback. A request acknowledgement is insufficient.
    // - errors preserve their actual original payload; Unknown is not rollback.
    // No methods grant recipient enrollment or replace its one-use transaction.
    pub(crate) trait AccountOriginalController: Send + Sync {
        fn reserve_original(&self, intent: OriginalIssueIntent) -> AccountFuture<'_>;
        fn observe(&self, operation: Uuid) -> AccountFuture<'_>;
        fn claim_preparation(
            &self, expected: OriginalIssueRow, claim_id: Uuid,
        ) -> AccountFuture<'_>;
        fn mark_write_started(
            &self, expected: OriginalIssueRow, descriptor: EnrollmentDescriptor,
        ) -> AccountFuture<'_>;
        fn bind_original(&self, expected: OriginalIssueRow) -> AccountFuture<'_>;
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Stop {
        InvalidOriginal, Conflict, Unknown, CancelledOrExpired,
        LockUnavailable, EntropyUnavailable,
    }

    enum WorkerOutcome {
        // Positive crypto/custody readback only. NOT current recipient authority.
        Recovered(EnrollmentDescriptor),
        Stopped(Stop),
    }

    #[derive(Default)]
    struct Attempt {
        account_results: Vec<AccountResult>,
        lookups: Vec<Result<Zeroizing<String>, SecretsVaultError>>,
        candidate: Option<Zeroizing<String>>,
        prepared_descriptor: Option<EnrollmentDescriptor>,
        preparation_claim_id: Option<Uuid>,
        entropy_error: Option<getrandom::Error>,
        put_result: Option<Result<(), SecretsVaultError>>,
        lock_error: Option<std::io::Error>,
        panic: Option<Box<dyn Any + Send>>,
        outcome: Option<WorkerOutcome>,
    }

    struct Completion {
        attempt: Option<Attempt>,
        worker_finished_at: Option<Instant>,
    }

    struct JoinSlot {
        task: Option<JoinHandle<()>>,
        result: Option<Result<(), JoinError>>,
    }

    struct Custody {
        intent: OriginalIssueIntent,
        original_deadline: Instant,
        cancel: AtomicBool,
        completion: Mutex<Completion>,
        join: Mutex<JoinSlot>,
        drain_gate: tokio::sync::Mutex<()>,
    }

    fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
        value.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // Each access refers to the SAME worker, candidate, errors and receipts.
    // Dropping an access or drain future does not abort or restart anything.
    #[derive(Clone)]
    pub(crate) struct IssueAccess(Arc<Custody>);

    #[derive(Clone, Copy)]
    pub(crate) struct DrainObservation {
        pub joined: bool,
        pub worker_finished_at: Option<Instant>,
        pub deadline_exceeded: bool,
    }

    impl IssueAccess {
        pub(crate) fn request_cancel(&self) {
            self.0.cancel.store(true, Ordering::SeqCst);
        }

        fn poll_join(&self, cx: &mut std::task::Context<'_>) -> Poll<()> {
            let mut slot = lock(&self.0.join);
            if slot.result.is_some() { return Poll::Ready(()); }
            let Some(task) = slot.task.as_mut() else { return Poll::Pending; };
            match Pin::new(task).poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(result) => {
                    slot.result = Some(result); // WHOLE original Result/JoinError.
                    slot.task = None;
                    Poll::Ready(())
                }
            }
        }

        pub(crate) async fn drain(&self) -> DrainObservation {
            // Poll the retained join even after the original deadline so an
            // already completed worker is observable; never allocate a budget.
            // Serialize competing drain pollers with an ASYNC mutex; no std
            // mutex or file guard is held by the awaiter across an await.
            if let Ok(_gate) = tokio::time::timeout_at(
                self.0.original_deadline, self.0.drain_gate.lock(),
            ).await {
                let wait = std::future::poll_fn(|cx| self.poll_join(cx));
                let _ = tokio::time::timeout_at(self.0.original_deadline, wait).await;
            }
            let _ = std::future::poll_fn(|cx| {
                let _ = self.poll_join(cx);
                Poll::Ready(())
            }).await;
            let slot = lock(&self.0.join);
            let completion = lock(&self.0.completion);
            let joined = matches!(slot.result.as_ref(), Some(Ok(())));
            let exceeded = completion.worker_finished_at
                .map(|time| time > self.0.original_deadline)
                .unwrap_or_else(|| Instant::now() >= self.0.original_deadline);
            DrainObservation {
                joined, worker_finished_at: completion.worker_finished_at,
                deadline_exceeded: exceeded,
            }
        }

        pub(crate) fn recovered_descriptor(&self) -> Option<EnrollmentDescriptor> {
            if !matches!(lock(&self.0.join).result.as_ref(), Some(Ok(()))) { return None; }
            let completion = lock(&self.0.completion);
            match completion.attempt.as_ref()?.outcome.as_ref()? {
                WorkerOutcome::Recovered(value) => Some(value.clone()),
                WorkerOutcome::Stopped(_) => None,
            }
        }

        pub(crate) fn stopped_reason(&self) -> Option<Stop> {
            let completion = lock(&self.0.completion);
            match completion.attempt.as_ref()?.outcome.as_ref()? {
                WorkerOutcome::Stopped(reason) => Some(*reason),
                WorkerOutcome::Recovered(_) => None,
            }
        }
    }

    // Dedicated production-vault namespace and stable existing lock directory
    // are operator configuration. ALL writers of this invitation namespace must
    // adopt the same directory/inode protocol. Do not delete/replace lock files
    // or the directory, including on success; no stale-file cleanup/ABA.
    pub(crate) struct OriginalInvitationWriter {
        root: PathBuf,
        vault: Arc<OsKeychainSecretsVault>,
        account: Arc<dyn AccountOriginalController>,
        // Host must retain this service through shutdown/reconciliation.
        // No auto-eviction of Unknown records and no global cancellation.
        retained: Mutex<BTreeMap<Uuid, Arc<Custody>>>,
    }

    impl OriginalInvitationWriter {
        pub(crate) fn new(
            configured_root: &Path,
            vault: Arc<OsKeychainSecretsVault>,
            account: Arc<dyn AccountOriginalController>,
        ) -> Result<Self, std::io::Error> {
            let root = configured_root.canonicalize()?;
            if !root.is_dir() || vault.service_namespace().trim().is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput, "invalid invitation writer configuration",
                ));
            }
            Ok(Self { root, vault, account, retained: Mutex::new(BTreeMap::new()) })
        }

        pub(crate) fn start(
            &self, intent: OriginalIssueIntent, original_deadline: Instant,
        ) -> Result<IssueAccess, Stop> {
            if intent.operation_id.is_nil() || intent.purpose != PURPOSE
                || intent.vault_namespace != self.vault.service_namespace()
                || intent.vault_reference != original_secret_reference(intent.operation_id)
            {
                return Err(Stop::InvalidOriginal);
            }
            let runtime = Handle::try_current().map_err(|_| Stop::Unknown)?;
            // A blocking worker's Handle::block_on cannot drive a current-thread
            // runtime's IO/timers after its caller drops. Require a live host
            // multithread runtime; shutdown errors/panics remain retained Unknown.
            if runtime.runtime_flavor() != tokio::runtime::RuntimeFlavor::MultiThread {
                return Err(Stop::Unknown);
            }
            let mut retained = lock(&self.retained);
            if let Some(existing) = retained.get(&intent.operation_id) {
                if existing.intent != intent
                    || existing.original_deadline != original_deadline
                {
                    return Err(Stop::Conflict);
                }
                return Ok(IssueAccess(Arc::clone(existing)));
            }
            let custody = Arc::new(Custody {
                intent, original_deadline, cancel: AtomicBool::new(false),
                completion: Mutex::new(Completion {
                    attempt: None, worker_finished_at: None,
                }),
                join: Mutex::new(JoinSlot { task: None, result: None }),
                drain_gate: tokio::sync::Mutex::new(()),
            });
            // Retain BEFORE scheduling, under the same registration lock.
            retained.insert(custody.intent.operation_id, Arc::clone(&custody));
            let root = self.root.clone();
            let vault = Arc::clone(&self.vault);
            let account = Arc::clone(&self.account);
            let worker = Arc::clone(&custody);
            let worker_runtime = runtime.clone();
            let scheduled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.spawn_blocking(move || {
                    let mut attempt = Attempt::default();
                    // The guard lives OUTSIDE catch_unwind and inside the real
                    // blocking worker; a caller cancellation cannot drop it.
                    let mut guard: Option<File> = None;
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        guard = match acquire(&root, &worker.intent) {
                            Ok(file) => Some(file),
                            Err(error) => {
                                attempt.lock_error = Some(error);
                                return Err(Stop::LockUnavailable);
                            }
                        };
                        run(&worker_runtime, vault.as_ref(), account.as_ref(), &worker, &mut attempt)
                    }));
                    attempt.outcome = Some(match result {
                        Ok(Ok(descriptor)) => WorkerOutcome::Recovered(descriptor),
                        Ok(Err(reason)) => WorkerOutcome::Stopped(reason),
                        Err(payload) => {
                            attempt.panic = Some(payload);
                            WorkerOutcome::Stopped(Stop::Unknown)
                        }
                    });
                    // Retain whole results, exact candidate and panic BEFORE
                    // releasing the OS lock; no secret rendering/logging.
                    lock(&worker.completion).attempt = Some(attempt);
                    drop(guard);
                    lock(&worker.completion).worker_finished_at = Some(Instant::now());
                })
            }));
            match scheduled {
                Ok(task) => lock(&custody.join).task = Some(task),
                Err(payload) => {
                    let mut attempt = Attempt::default();
                    attempt.panic = Some(payload);
                    attempt.outcome = Some(WorkerOutcome::Stopped(Stop::Unknown));
                    lock(&custody.completion).attempt = Some(attempt);
                    // No join/finished timestamp is fabricated.
                }
            }
            Ok(IssueAccess(custody))
        }
    }

    fn acquire(root: &Path, intent: &OriginalIssueIntent) -> std::io::Result<File> {
        let mut hash = Sha256::new();
        hash.update(b"handshake.invitation-vault-writer.v1\0");
        hash.update((intent.vault_namespace.len() as u64).to_be_bytes());
        hash.update(intent.vault_namespace.as_bytes());
        hash.update((intent.purpose.len() as u64).to_be_bytes());
        hash.update(intent.purpose.as_bytes());
        hash.update(intent.operation_id.as_bytes());
        let path = root.join(format!("{}.lock", hex::encode(hash.finalize())));
        let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
        FileExt::try_lock_exclusive(&file)?;
        // No unlink, rename, truncate, or independently cloned handle.
        Ok(file)
    }

    fn preflight(custody: &Custody) -> Result<(), Stop> {
        if custody.cancel.load(Ordering::SeqCst)
            || Instant::now() >= custody.original_deadline
        {
            return Err(Stop::CancelledOrExpired);
        }
        // Preparation has no token digest yet: enforce the exact Source030 caps
        // directly, without constructing any placeholder descriptor.
        let intent = &custody.intent;
        let mut limit = intent.issued_at.checked_add_seconds(60)
            .map_err(|_| Stop::InvalidOriginal)?.min(intent.session_expires_at);
        for source in [&intent.enroll_source, &intent.assignment_source] {
            if let Some(expiry) = source.grant_expires_at() { limit = limit.min(expiry); }
        }
        if let Some(expiry) = intent.policy_expires_at { limit = limit.min(expiry); }
        if intent.expires_at <= intent.issued_at || intent.expires_at > limit {
            return Err(Stop::InvalidOriginal);
        }
        let now = chrono::Utc::now();
        let now = Timestamp::new(now.timestamp(), now.timestamp_subsec_nanos())
            .map_err(|_| Stop::InvalidOriginal)?;
        if now < custody.intent.issued_at || now >= custody.intent.expires_at {
            return Err(Stop::CancelledOrExpired);
        }
        Ok(())
    }

    fn observe_account(
        result: AccountResult, attempt: &mut Attempt,
        intent: &OriginalIssueIntent,
    ) -> Result<OriginalIssueRow, Stop> {
        attempt.account_results.push(result);
        match attempt.account_results.last() {
            Some(Ok(Some(row))) if row.intent == *intent && !row.revision.0.is_empty() => {
                Ok(row.clone())
            }
            Some(Ok(Some(_))) => Err(Stop::Conflict),
            _ => Err(Stop::Unknown),
        }
    }

    enum Lookup { Missing, Present([u8; 32]) }

    fn lookup(
        vault: &OsKeychainSecretsVault, intent: &OriginalIssueIntent,
        attempt: &mut Attempt,
    ) -> Result<Lookup, Stop> {
        attempt.lookups.push(vault.get(&intent.vault_reference));
        match attempt.lookups.last() {
            Some(Ok(secret)) => {
                let bytes = canonical_secret(secret.as_str()).map_err(|_| Stop::Conflict)?;
                Ok(Lookup::Present(purpose_digest(
                    intent.operation_id, PURPOSE, bytes.as_slice(),
                )))
            }
            Some(Err(SecretsVaultError::NoSecretForLane(_))) => Ok(Lookup::Missing),
            _ => Err(Stop::Unknown),
        }
    }

    fn exact_digest(actual: [u8; 32], expected: &[u8; 32]) -> Result<(), Stop> {
        if actual.ct_eq(expected).unwrap_u8() == 1 { Ok(()) } else { Err(Stop::Conflict) }
    }

    fn finish_binding(
        runtime: &Handle, account: &dyn AccountOriginalController,
        custody: &Custody, attempt: &mut Attempt, row: OriginalIssueRow,
        descriptor: EnrollmentDescriptor,
    ) -> Result<EnrollmentDescriptor, Stop> {
        preflight(custody)?;
        currently_valid(&descriptor).map_err(|_| Stop::InvalidOriginal)?;
        match &row.state {
            OriginalIssueState::Bound(value) if value == &descriptor => {}
            OriginalIssueState::WriteStarted(value) if value == &descriptor => {
                let previous_revision = row.revision.clone();
                let committed = observe_account(
                    runtime.block_on(account.bind_original(row)), attempt, &custody.intent,
                )?;
                if committed.revision == previous_revision
                    || committed.state != OriginalIssueState::Bound(descriptor.clone())
                {
                    return Err(Stop::Conflict);
                }
            }
            _ => return Err(Stop::Conflict),
        }
        preflight(custody)?;
        currently_valid(&descriptor).map_err(|_| Stop::InvalidOriginal)?;
        Ok(descriptor)
    }

    fn run(
        runtime: &Handle, vault: &OsKeychainSecretsVault,
        account: &dyn AccountOriginalController, custody: &Custody,
        attempt: &mut Attempt,
    ) -> Result<EnrollmentDescriptor, Stop> {
        preflight(custody)?;
        let intent = &custody.intent;
        let row = observe_account(
            runtime.block_on(account.observe(intent.operation_id)), attempt, intent,
        )?;
        preflight(custody)?;
        // Read existing FIRST. Missing is first-write eligible ONLY together
        // with the canonical positively observed Reserved state below.
        let first = lookup(vault, intent, attempt)?;
        match (&row.state, first) {
            (OriginalIssueState::WriteStarted(descriptor)
                | OriginalIssueState::Bound(descriptor), Lookup::Present(actual)) => {
                if !intent.matches_descriptor(descriptor) { return Err(Stop::Conflict); }
                exact_digest(actual, descriptor.token_digest())?;
                let descriptor = descriptor.clone();
                return finish_binding(runtime, account, custody, attempt, row, descriptor);
            }
            (OriginalIssueState::Reserved, Lookup::Present(_)) => return Err(Stop::Conflict),
            (OriginalIssueState::Preparing { .. }, _) => return Err(Stop::Unknown),
            (OriginalIssueState::WriteStarted(_) | OriginalIssueState::Bound(_),
                Lookup::Missing) => return Err(Stop::Unknown),
            (OriginalIssueState::Reserved, Lookup::Missing) => {}
        }
        preflight(custody)?;
        // A distinct preparation CLAIM id, never a replacement operation id.
        // Durable claiming precedes entropy so crash recovery cannot remint.
        let claim_id = Uuid::now_v7();
        attempt.preparation_claim_id = Some(claim_id);
        let previous_revision = row.revision.clone();
        let preparing = observe_account(
            runtime.block_on(account.claim_preparation(row, claim_id)), attempt, intent,
        )?;
        if preparing.revision == previous_revision
            || preparing.state != (OriginalIssueState::Preparing { claim_id })
        {
            return Err(Stop::Conflict);
        }
        preflight(custody)?;
        let mut raw = Zeroizing::new([0u8; 32]);
        if let Err(error) = getrandom::getrandom(&mut raw[..]) {
            attempt.entropy_error = Some(error);
            return Err(Stop::EntropyUnavailable);
        }
        let digest = purpose_digest(intent.operation_id, PURPOSE, &raw[..]);
        attempt.candidate = Some(Zeroizing::new(URL_SAFE_NO_PAD.encode(&raw[..])));
        let descriptor = intent.descriptor(digest)?;
        attempt.prepared_descriptor = Some(descriptor.clone());
        preflight(custody)?;
        let previous_revision = preparing.revision.clone();
        let started = observe_account(
            runtime.block_on(account.mark_write_started(preparing, descriptor.clone())),
            attempt, intent,
        )?;
        if started.revision == previous_revision
            || started.state != OriginalIssueState::WriteStarted(descriptor.clone())
        {
            return Err(Stop::Conflict);
        }
        preflight(custody)?;
        // Re-probe after durable admission. Existing matching bytes are reused;
        // mismatch/backend uncertainty never invokes overwriting put.
        match lookup(vault, intent, attempt)? {
            Lookup::Present(actual) => exact_digest(actual, descriptor.token_digest())?,
            Lookup::Missing => {
                preflight(custody)?;
                let secret = attempt.candidate.as_ref().ok_or(Stop::Unknown)?;
                attempt.put_result = Some(vault.put(&intent.vault_reference, secret.as_str()));
                // Retain the WHOLE original put result even on error. Neither
                // error nor missing readback means NoEffect; never retry put.
                match lookup(vault, intent, attempt)? {
                    Lookup::Present(actual) => exact_digest(actual, descriptor.token_digest())?,
                    Lookup::Missing => return Err(Stop::Unknown),
                }
            }
        }
        finish_binding(runtime, account, custody, attempt, started, descriptor)
    }
}
// Required integration boundaries, NOT delivered authority:
// 1. Account implements actual one-row durable reservation/compare-update/readback,
//    current authority/status, original immutable request encoding and vault ref.
//    Preparing and WriteStarted are never reset, even after restart/missing
//    keychain entry. Preparing without its original retained candidate is Unknown.
//    State names/revision are proposed supplier ABI, NOT adopted Account schema.
// 2. The configured namespace is dedicated; every invitation writer uses this
//    SAME persistent lock-root inode and namespace/op lock. OS keychain has no
//    atomic create-only API. Nonparticipating writers/root replacement are not
//    excluded by fs2; configuration/ownership adoption is mandatory.
// 3. Host retains the service, all Unknown custody and original deadline, drains
//    its real handles at shutdown, and reconciles canonical Account/keychain
//    state after process loss. No claimed cross-store atomicity or durable secret
//    after process crash before put. No remint on an ambiguous started write.
// 4. Account still supplies recipient current authority/revocation/one-use CAS;
//    recovered_descriptor is crypto/custody data, never acceptance permission.
// 5. No source save, build, test, OS-vault call, lock-file creation or runtime
//    proof has been performed by preparing this candidate.
