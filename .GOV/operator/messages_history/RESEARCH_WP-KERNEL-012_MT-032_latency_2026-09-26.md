---
file_id: RESEARCH-WP-KERNEL-012-MT-032-LATENCY-2026-09-26
file_kind: operator_research
updated_at: 2026-09-26
---

<topic id="evidence-and-hypotheses" wp="WP-KERNEL-012" updated_at="2026-09-26">

## Observed failure

The independent diagnostic on pushed product `d9277d04` ran the two former native failures once. Both timed out. Save spent 4.840s in its transaction, 2.692s in its receipt and 7.253s in backlinks before indexing began, against a 15s request deadline. Create A spent 5.390s in its transaction and 2.626s in its receipt before indexing, against a 10s deadline. Neither index await has a captured terminal event. These observations prove deadline exhaustion; they do not prove an index deadlock, hardware fault or the internal source of each operation's cost.

Canonical evidence: `MT-032.json:remediation_v11.diagnostic_result`; phase capture `../Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-c3x/logs/mt032-phase-watch-d9277d04f03490f9ceac1c9f4684e02757d8d43a.jsonl`, SHA256 `219BB11650E4FBEE99DD79D9AB3B531EA6D59474593680B8C688DA3E9AEE54B6`. All 72 unique events were reconciled with retained source logs. The original incomplete-stream flag remains explained in the MT record.

## Source findings

- `storage/surreal.rs::with_data_operation` obtains a shared lifecycle lease, clones an SDK session, selects namespace/database and performs record SIGNIN before dispatching its callback. No global exclusive-query lock was found.
- `schema.surql:6002` selects the authenticated session through its indexed SHA-256 token hash. It checks channel binding, expiry, revocation, account/principal/Space state and policy epochs. It performs no password comparison or Argon2 work. Repeated sign-in cost is unmeasured; middleware authorization took about 30ms in the retained run.
- `knowledge.rs::replace_backlinks_attempt` performs six serial datastore calls: source read, two prior-state reads, two candidate-target reads, then the guarded mutation. `fn::mt120_document_access` uses three grant queries per evaluation. Query amplification is a source-backed hypothesis, not a measured dominant cause.
- Receipt append reauthorizes the exact RichDocument and writes the EventLedger through record-user scope. Index provisioning writes authority fences also consumed by receipt authorization. Concurrent postcommit writes need conflict, revocation and cancellation analysis.
- Both core and native tests use authenticated record-user routes. The passing core Owner client lacks the native clients' explicit 10s/15s deadlines, so core PASS does not establish native deadline compliance.

</topic>

<topic id="research-and-options" wp="WP-KERNEL-012" updated_at="2026-09-26">

## Primary sources checked

1. [SurrealDB query optimisation](https://surrealdb.com/docs/learn/querying/concepts-and-guides/query-optimisation): inspect plans, align indexes with predicates, prefer exact record selection; synchronous events execute in the triggering write transaction. Current documentation includes features newer than pinned 3.2.0, so syntax must be checked with the pinned engine.
2. [Pinned SDK SIGNIN](https://github.com/surrealdb/surrealdb/blob/v3.2.0/surrealdb/src/method/signin.rs): dispatches a Signin command against the SDK session.
3. [Pinned core SIGNIN](https://github.com/surrealdb/surrealdb/blob/v3.2.0/surrealdb/core/src/iam/signin.rs) and [token verification](https://github.com/surrealdb/surrealdb/blob/v3.2.0/surrealdb/core/src/iam/verify.rs): custom record sign-in and JWT authentication are distinct paths.
4. [Rust SDK multi-tenancy](https://surrealdb.com/docs/reference/rust/concepts/multi-tenancy) and [record access](https://surrealdb.com/docs/reference/query-language/statements/define/access/record): session isolation and record permissions are part of the authority boundary.
5. [CLI SQL](https://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/sql) and [official 3.2.0 release](https://github.com/surrealdb/surrealdb/releases/tag/v3.2.0): a standalone pinned tool allows syntax and small query-plan diagnostics without Cargo rebuilds.

Diagnostic dependency: `../gov_runtime/tools/surrealdb/3.2.0/surreal.exe`, version 3.2.0, 114,430,464 bytes, official asset SHA256 `9382A851A54DF09AAF39C74C0B010A14B09F2565651F620E83C21DC1F1BCA8D3`. It is not substitute product acceptance proof.

## Options and debate

- Reject cross-operation cached JWT/session authentication: it skips current SIGNIN predicates unless equivalent live checks are preserved.
- Reject asynchronous security events, detached index work and deadline increases: they change required guarantees.
- Do not accept concurrent receipt/index/backlink mutations merely because their return values are independent; they share authority-fence writes.
- Investigate document-local redundant reads and exact record selectors. Preserve live permissions, mutation guards, row counts, errors and durable projections. A skipped `relative_path=None` lookup is provably empty but alone does not establish repair of both failures.
- Use the pinned SDK's existing statistics to separate query execution from enclosing operation costs where needed. Exclude SQL, content and credentials from diagnostics.

The selected patch is confined to `storage/surreal/knowledge.rs`: skip the existing-source lookup when normalized `relative_path` is None; skip empty candidate lookups; select known document/source/Loom record IDs directly, retaining original predicates and ordering. The title-based branch remains unchanged. Schema assertions at lines 499, 3511 and 3771 enforce record-key/external-ID equality. Independently inspected wiki import binds the document and Loom block to that same document ID. No mutation statement, authentication, authority fence, deadline or transaction boundary changes.

Counterfactuals: removing the candidate document workspace predicate would admit a foreign workspace target; removing the downstream foreign-Loom split would lose cross-workspace rejection. Retained predicates and schema assertions were inspected independently of the coder. Existing query-count assertions cover unchanged methods. Actual parameterized record-array execution and denied cases still require the pinned-engine advisory; source review is not runtime proof. The reduction is not yet a complete measured explanation of the native deadlines.

The CLI's local memory endpoint disables authentication; root-only results cannot establish record-user permission equivalence. WPV therefore ran both a root-only selector comparison (702ms) and a small owned loopback server with simplified record permissions (1365ms). Old/new outputs matched for ordering, empty/None/missing/deleted selectors, owned records, cross-workspace denial and revoked records. The candidate preserved foreign Loom rows for the existing caller to reject. These fixtures do not reproduce the full product grant/fence model or Rust binding transport. The tiny candidate plan's SourceExpr was not faster than the original workspace IndexScan; no latency benefit is claimed. Actual inputs/results/hashes and failed fixture setup corrections are in `MT-032.remediation_v12.cheap_checks.result`. All owned diagnostic processes were reaped; no foreign process stopped.

The source reduction compiled on the warm D target in 552s with unchanged pre/post digest across 1,228 inputs, then was committed/pushed as `49eac54e03d2a94abf5aa3ced025e2947f9adc8b`. No new target, manifest, dependency, profile or expensive test was used for these checks. One final union remains pending; no diagnostic dependency or synthetic fixture substitutes for it.

Agents: resumed builder Astra/medium `01a0d677-2c57-7813-9075-c273d1c4637f`; independent WP Validator Sol/medium `01a0db0c-e4d4-73a0-be7d-33ebb18bff84`; read-only researcher Luna/medium `01a0dd58-85bd-7793-8004-583920f7b12d`. Next: finish the one D compile, push, obtain independent cheap comparison, then one final union and outcome report regardless of result. Task state and authorization remain in MT JSON.

</topic>
