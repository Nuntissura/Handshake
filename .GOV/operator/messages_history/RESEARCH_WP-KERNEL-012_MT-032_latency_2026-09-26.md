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

The source reduction compiled on the warm D target in 552s with unchanged pre/post digest across 1,228 inputs, then was committed/pushed as `49eac54e03d2a94abf5aa3ced025e2947f9adc8b`. No new target, manifest, dependency, profile or expensive test was used for these checks. The single final union has now completed; no diagnostic dependency or synthetic fixture substitutes for its results.

Agents: resumed builder Astra/medium `01a0d677-2c57-7813-9075-c273d1c4637f`; independent WP Validator Sol/medium `01a0db0c-e4d4-73a0-be7d-33ebb18bff84`; read-only researcher Luna/medium `01a0dd58-85bd-7793-8004-583920f7b12d`. The D compile, push, cheap comparisons and one final union are complete. Task state, independent verdicts and authorization remain in MT JSON.

</topic>

<topic id="final-union-outcome" wp="WP-KERNEL-012" updated_at="2026-09-26">

## Result and unresolved boundaries

The clean-export `49eac54e` union completed at 2026-09-26T15:40:57Z: core 221/223 passed; native 2908/2952 passed, 44 failed, 309 skipped. Both completed invocations returned nextest 100. Native execution took 6207.400s; core took 4227.949s. This is completed failing proof, not a launch/configuration abort. MT-032's dedicated native target passed 22/24; both required live cases still failed. No second run or deadline increase was used.

Evidence under `../Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-c3x/`: native `junit-49eac54e03d2a94abf5aa3ced025e2947f9adc8b-native.xml`, SHA256 `B435DAD2C6647E7D50C7083D035D43B61B864B0AE7B09D42095263EAD08D3F9C`; core suffix `-core.xml`, SHA256 `CD420357013FF59A89EA4440C394A8951404DD3700A75D7D066C385D118B7897`. The native executable `test_loom_address-424a802415922a9d.exe` hash is `DD26BD0615FC6F21EA9E978F810F70339B3BAF089FA07502CC11520DCF3659D7`; its dependency metadata names the exact `export-49eac54e` manifest, whose marker equals the full candidate. Phase capture `logs/mt032-phase-watch-49eac54e03d2a94abf5aa3ced025e2947f9adc8b.jsonl` has SHA256 `607D863BFCC28E8F02260C08577D4A6A5E46A5E104099E84E2348C074CD94547` after the observer exited.

- Owned restart: PUT save failed at request send with `TimedOut`, not connect/body/decode failure. Request `e0c33158-8b97-4798-8347-3fcd1bb677d6` completed transaction in 5069ms and receipt in 2645ms; backlinks began at 7.750s with no terminal marker. The required restart/hash/backlink assertions were not reached.
- Self-seeded proof: creating A without a link failed at request send with `is_timeout=true`. Request `c58124c5-d656-4352-8383-43884e924ed4` completed transaction in 5362ms, receipt in 2636ms and embeds in 12ms; indexing began at 8.041s with no terminal marker. Preceding B creation completed in 9462ms. These observations do not constitute a controlled latency comparison with the prior candidate.

The independent validator reconciled 68 observer events with 68 events in the two retained source logs. The coder separately inspected both source logs and the current patch; the parent opened the captured events, final native JUnit, executable hash and dependency metadata. `MT-032.remediation_v12.union_observation` retains the detailed observation and source-log hashes; the final independent verdict belongs in the MT's validation record, not this document.

## Why the patch was insufficient; next source decision

`knowledge.rs:5353` still acquires the keyed mutation lock/retry before `replace_backlinks_attempt` (`:5861`). That attempt still reads the source and two prior-state surfaces, resolves targets, rewrites backlinks/edges and recomputes counts. The current save contains a KRD-ID link: empty-candidate skips do not cover its nonempty target; exact-record selection applies only if execution reaches that branch. The outer phase marker cannot identify which inner operation remains unfinished.

`knowledge_documents.rs:710` still awaits source lookup, possible source provisioning/stale marking and entity upsert. The None-path skip and exact source reread do not remove provisioning, authority-fence work or entity mutation. This is the unfinished boundary in create A, distinct from save backlinks.

Next investigation must distinguish these inner awaits and their lock/retry/authorization costs before selecting another repair. Existing evidence does not select a guard/query for removal, prove SIGNIN dominance, establish a deadlock or establish hardware causation. Do not remove live authorization, detach required postcommit work, increase deadlines or repeat the unchanged union. GP-064 records the two-boundary distinction before the next repair dispatch; it changes no acceptance criterion and grants no additional run.

The repair preserved compile/query-shape compatibility but did not deliver MT-032 acceptance. Cheap selector equality was useful negative-path feedback, not performance proof. Warm caches survived; no target wipe occurred. The parent separately missed MT-136's complete proof floor during prelaunch, and the validator's draft MT-153 failure-kind enum required correction before commit. During closeout, “no additional tests” was also incorrectly treated as excluding cheap source/diff/image inspections; the parent required those inspections or exact unavailable-artifact gaps and added GP-065. None of these workflow mistakes establishes a hardware explanation for MT-032's product timeouts. The [handoff](HANDOFF_WP-KERNEL-012_KB-orchestrator_2026-09-24_session7.md) records MT dispositions and remaining proof gaps. This cycle consumes one final union and ends with its outcome report; no further test is authorized by this research record.

</topic>
