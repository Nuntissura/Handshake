import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { readSourceBinding, deriveMechanicalReady, canonicalJson, verifyBuild3ReconciliationRows, verifyGithubBuild, verifyProseTargets, verifyComponentRun } from '../scripts/mechanical-ready-evidence.mjs';

const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function fixture(run) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'mechanical-ready-'));
  const product = path.join(root, 'product');
  fs.mkdirSync(product);
  const git = args => execFileSync('git', ['-C', product, ...args], { encoding: 'utf8', windowsHide: true });
  const file = (name, value) => {
    const filename = path.join(root, name);
    fs.writeFileSync(filename, typeof value === 'string' ? value : JSON.stringify(value));
    return { path: filename, sha256: digest(fs.readFileSync(filename)) };
  };
  try {
    git(['init', '-q']);
    fs.writeFileSync(path.join(product, 'wire.rs'), 'pub fn wire() {}\n');
    fs.mkdirSync(path.join(product, '.GOV'));
    fs.writeFileSync(path.join(product, '.GOV', 'state.json'), '{"phase":"runtime"}');
    git(['add', 'wire.rs', '.GOV/state.json']);
    git(['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture']);
    git(['update-index', '--skip-worktree', '.GOV/state.json']);
    const binding = readSourceBinding(product);
    const manifest = file('source.txt', binding.manifest);
    const command = 'cargo test --locked --manifest-path Cargo.toml --features surreal-test-support --test wire wire_real -- --exact';
    const args = ['test', '--locked', '--manifest-path', 'Cargo.toml', '--features', 'surreal-test-support', '--test', 'wire', '--', 'wire_real', '--exact'];
    const log = file('run.log', 'test wire_real ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\nCARGO_EXIT=0\n');
    const original = file('original.json', { schema: 'handshake.local_cargo_run@1', exit_code: 0, cwd: product, arguments: args });
    const receiptData = { schema: 'handshake.bound_command_run@1', head: binding.head, tree: binding.tree, product,
      exit_code: 0, proof_valid: true, before_manifest: manifest, compiled_source_manifest: manifest, after_manifest: manifest,
      original_receipt: original, cargo_arguments: args, log, platform: 'native-platform' };
    const receipt = file('bound.json', receiptData);
    const reviewData = { schema_id: 'hsk.source_review_findings@1', reviewer_session: 'independent-reviewer', product_commit: binding.head,
      product_tree: binding.tree, reviewed_files: [{ path: 'wire.rs', sha256: digest(fs.readFileSync(path.join(product, 'wire.rs'))) }],
      coverage: ['reason_strings', 'public_exports', 'test_gate_intent'], findings: [],
      lifecycle_sha256: digest(canonicalJson({ claimed_by: 'implementer', completed_by: null })),
      real_resource_tests: [{ target: 'wire', test_name: 'wire_real', source_path: 'wire.rs' }] };
    const review = file('review.json', reviewData);
    const contract = { mt_id: 'MT-001', lifecycle: { claimed_by: 'implementer', completed_by: null }, owned_files: ['wire.rs'],
      scope: { proof_targets: [command] }, handoff: { ready_evidence_refs: { source_reviews: [review], command_results: [{ receipt, log }] } } };
    run({ root, product, file, git, binding, contract, receiptData, reviewData, log });
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}
const derive = f => deriveMechanicalReady({ contract: f.contract, evidenceBase: f.root, productRoot: f.product, nativePlatform: 'native-platform' });

test('source-binding CLI preserves JSON output and captures failures in scoped runtime', () => fixture(f => {
  const script = fileURLToPath(new URL('../scripts/mechanical-ready-evidence.mjs', import.meta.url));
  const options = { encoding: 'utf8', windowsHide: true, env: { ...process.env, HANDSHAKE_GOV_RUNTIME_ROOT: f.root } };
  const result = JSON.parse(execFileSync(process.execPath, [script, '--source-binding', f.product], options));
  assert.equal(result.manifest_sha256, f.binding.manifest_sha256);
  assert.throws(() => execFileSync(process.execPath, [script, '--wrong-option'], { ...options, stdio: 'pipe' }));
  const capture = fs.readFileSync(path.join(f.root, 'roles_shared', 'fail_capture.jsonl'), 'utf8');
  assert.match(capture, /only --source-binding/);
}));

test('component proof binds original build, source components, binary, inventory and runtime log', () => {
  for (const fault of [null, 'lib', 'bin', 'source', 'during-run', 'binary', 'inventory', 'build-exit', 'log', 'empty']) fixture(f => {
    const kind = ['lib', 'bin'].includes(fault) ? fault : 'test';
    const snapshot = { product_root: f.product, selection: { roots: ['.'], exclude: ['.GOV/'], enumerator: 'git ls-files --cached --others --exclude-standard' },
      files: [`wire.rs=${digest(fs.readFileSync(path.join(f.product, 'wire.rs')))}`], environment: [], fingerprint: 'same' };
    const before = f.file('components.json', snapshot);
    const after = fault === 'during-run' ? f.file('after.json', { ...snapshot, fingerprint: 'different' }) : before;
    const binary = f.file('wire.exe', 'compiled executable');
    const artifact = { reason: 'compiler-artifact', package_id: 'fixture', profile: { test: true }, executable: binary.path, features: ['surreal-test-support'], target: { name: 'wire', kind: [kind] } };
    const buildLog = f.file('build.jsonl', [artifact, { reason: 'build-finished', success: true }].map(row => JSON.stringify(row)).join('\n'));
    const build = f.file('build.json', { schema_id: 'hsk.build_command@1', program: 'cargo', cwd: f.product, exit_code: fault === 'build-exit' ? 1 : 0, source_binding: f.binding,
      input_before: before, input_after: before, log: buildLog, args: ['test', '--locked', '--manifest-path', 'Cargo.toml', '--test', 'wire', '--no-run'] });
    const data = { schema_id: 'hsk.component_test_run@1', status: 'GREEN', exit: 0, build_command: build, runtime_input_before: before, runtime_input_after: after,
      input_fingerprint: 'same', inventory: f.file('inventory.json', fault === 'inventory' ? [] : [artifact]), name: 'wire', kind, binary: binary.path,
      binary_sha256_before: binary.sha256, binary_sha256_after: fault === 'binary' ? '0'.repeat(64) : binary.sha256,
      args: ['--test-threads=1', '--nocapture'], features: 'surreal-test-support', log: f.log.path, passed: fault === 'empty' ? 0 : 1, failed: 0, ignored: 0, measured: 0, filtered: 0, executed: fault === 'empty' ? 0 : 1 };
    if (fault === 'source') fs.writeFileSync(path.join(f.product, 'wire.rs'), 'changed');
    const log = fault === 'log' ? 'test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out;' : fs.readFileSync(f.log.path, 'utf8');
    const action = () => verifyComponentRun({ receipt: { path: 'component.json' }, log: f.log }, data, log, f.root, f.binding);
    if (fault && !['lib', 'bin'].includes(fault)) assert.throws(action, undefined, fault);
    else {
      const result = action();
      assert.equal(result.executed, 1);
      assert.deepEqual(result.args, ['test', '--locked', '--manifest-path', 'Cargo.toml', ...(kind === 'lib' ? ['--lib'] : [`--${kind}`, 'wire']), '--', '--test-threads=1', '--nocapture']);
    }
  });
});

test('CI alternative checks provider commit, successful build steps, and non-native runner', () => {
  const ref = { provider: 'github_actions', repository: 'owner/repo', run_id: 12, job_id: 34, required_steps: ['Linux build check'] };
  const binding = { head: 'abc' };
  const originalRun = { id: 12, head_sha: 'abc' };
  const originalJob = { run_id: 12, head_sha: 'abc', status: 'completed', conclusion: 'success', labels: ['ubuntu-latest'],
    html_url: 'https://github.com/owner/repo/actions/runs/12/job/34', steps: [{ name: 'Linux build check', status: 'completed', conclusion: 'success' }] };
  for (const mutation of ['none', 'head', 'run', 'failed', 'step', 'native', 'unknown-platform', 'metadata-only', 'wrong-checkout']) {
    const job = structuredClone(originalJob), run = structuredClone(originalRun);
    if (mutation === 'head') run.head_sha = 'other';
    if (mutation === 'run') job.run_id = 13;
    if (mutation === 'failed') job.conclusion = 'failure';
    if (mutation === 'step') job.steps[0].conclusion = 'skipped';
    if (mutation === 'native') job.labels = ['windows-latest'];
    if (mutation === 'unknown-platform') job.labels = ['self-hosted'];
    const log = `[command]/usr/bin/git log -1 --format=%H\n${mutation === 'wrong-checkout' ? 'other' : 'abc'}\n`
      + (mutation === 'metadata-only' ? '##[group]Run echo check metadata\nFinished checks\n' : '##[group]Run cargo check --manifest-path src/backend/handshake_core/Cargo.toml\nFinished `dev` profile [unoptimized]\n');
    const action = () => verifyGithubBuild(ref, binding, 'x86_64-pc-windows-msvc', endpoint => endpoint.endsWith('/logs') ? log : endpoint.includes('/jobs/') ? job : run);
    if (mutation === 'none') assert.equal(action().platform, 'linux'); else assert.throws(action, undefined, mutation);
  }
});

test('prose targets require exact independent mappings and intact artifacts', () => fixture(f => {
  const target = 'PT-001: Inspect retained behavior.';
  const artifact = f.file('inspection.json', { inspected: 'wire.rs' });
  const mapping = { target, kind: 'inspection', source_review: 'review.json', artifact_refs: [artifact] };
  const reviews = [{ ref: 'review.json', proof_targets: [{ target, kind: 'inspection', artifact_refs: [artifact] }] }];
  assert.doesNotThrow(() => verifyProseTargets([target], [mapping], reviews, [], f.root));
  assert.throws(() => verifyProseTargets([target], [], reviews, [], f.root), /unmapped/);
  assert.throws(() => verifyProseTargets([target], [mapping, mapping], reviews, [], f.root), /duplicate/);
  assert.throws(() => verifyProseTargets([target], [mapping], [], [], f.root), /independent/);
  fs.appendFileSync(artifact.path, 'changed');
  assert.throws(() => verifyProseTargets([target], [mapping], reviews, [], f.root), /hash mismatch/);
}));

test('suite proof rejects missing identities and jointly cropped receipts/inventory', () => fixture(f => {
  const target = 'PT-002: Run the whole backend suite.';
  const artifacts = ['one', 'two'].map(name => ({ reason: 'compiler-artifact', package_id: 'fixture-package', features: [], profile: { test: true }, executable: name, target: { name, kind: ['test'], test: true, src_path: path.join(f.product, `${name}.rs`) } }));
  const metadata = { packages: [{ id: 'fixture-package', targets: artifacts.map(value => value.target) }] };
  const inventory = f.file('inventory.json', artifacts);
  const components = f.file('components.json', { product_root: f.product, selection: { roots: ['.'], exclude: ['.GOV/'], enumerator: 'git ls-files --cached --others --exclude-standard' }, files: [`wire.rs=${digest(fs.readFileSync(path.join(f.product, 'wire.rs')))}`], environment: [], fingerprint: 'same' });
  const buildLog = f.file('build.jsonl', [...artifacts, { reason: 'build-finished', success: true }].map(value => JSON.stringify(value)).join('\n'));
  const build = f.file('build.json', { schema_id: 'hsk.build_command@1', program: 'cargo', cwd: f.product, exit_code: 0, source_binding: f.binding, input_before: components, input_after: components, log: buildLog });
  const mapping = { target, kind: 'runtime_suite', source_review: 'review.json', artifact_refs: [inventory], suite_inventory: inventory, suite_build_command: build, suite_metadata: f.file('metadata.json', metadata), command_receipts: ['one', 'wrong'] };
  const reviews = [{ ref: 'review.json', proof_targets: [{ target, kind: 'runtime_suite', artifact_refs: [inventory] }] }];
  const binding = { ...f.binding, cargo_metadata: metadata };
  const commands = ['one', 'wrong'].map((name, i) => ({ ref: name, kind: 'test', name, executed: 1, filtered: 0, runtime_args: ['--test-threads=1'], artifact: artifacts[i] }));
  const action = () => verifyProseTargets([target], [mapping], reviews, commands, f.root, binding);
  assert.throws(action, /identity set/);
  commands[1].name = 'two'; commands[1].ref = 'two'; mapping.command_receipts[1] = 'two';
  assert.doesNotThrow(action); // Original focused inventory need not equal aggregate inventory.
  commands[0].filtered = 5;
  assert.throws(action, /target is filtered/);
  commands[0].filtered = 0; commands[0].runtime_args.push('only_one_case');
  assert.throws(action, /target is filtered/);
  commands[0].runtime_args.pop();
  mapping.suite_inventory = f.file('cropped.json', artifacts.slice(0, 1));
  commands.pop(); mapping.command_receipts.pop();
  assert.throws(action, /omits Cargo metadata/);
}));

test('v2 source binding independently matches raw git object identity and rejects untracked source', () => fixture(f => {
  const object = f.git(['hash-object', '--no-filters', 'wire.rs']).trim();
  assert.ok(f.binding.manifest.endsWith(`wire.rs\t17\t${object}\n`));
  fs.writeFileSync(path.join(f.product, 'new.rs'), 'new source');
  assert.throws(() => readSourceBinding(f.product), /dirty/);
}));

test('mechanical derivation ignores authored rubric answers and requires independent code evidence', () => fixture(f => {
  f.contract.handoff.kb_ready_checklist_evidence = { rubric_items: Array(6).fill({ answer: 'yes' }) };
  delete f.contract.handoff.ready_evidence_refs;
  const result = derive(f);
  assert.equal(result.items.get('RC-006-IMPLEMENTER-NOT-SELF-CERTIFYING').answer, 'yes');
  for (const [id, item] of result.items) if (!id.startsWith('RC-006')) assert.equal(item.answer, 'no');
}));

test('independent source review and real executed command derive supported items; native check cannot stand in for cross platform', () => fixture(f => {
  const result = derive(f);
  for (const id of ['RC-001-NO-STALE-REASONS', 'RC-002-NO-DEAD-CODE', 'RC-003-CFG-GATED-TESTS', 'RC-005-PROOF-COMMANDS']) {
    assert.equal(result.items.get(id).answer, 'yes', result.items.get(id).explanation);
  }
  assert.equal(result.items.get('RC-004-CROSS-PLATFORM-CI').answer, 'no');
}));

test('stale source, modified log, failed exit, zero tests, missing command, and self review cannot clear proof', () => {
  for (const mutation of ['head', 'log', 'exit', 'zero', 'command', 'self-review', 'completed']) fixture(f => {
    if (mutation === 'head') f.receiptData.head = '0'.repeat(40);
    if (mutation === 'exit') f.receiptData.exit_code = 1;
    if (mutation === 'log') fs.appendFileSync(f.log.path, 'tamper');
    if (mutation === 'zero') {
      f.log = f.file('run.log', 'test result: ok. 0 passed; 0 failed;\nCARGO_EXIT=0\n');
      f.receiptData.log = f.log;
      f.contract.handoff.ready_evidence_refs.command_results[0].log = f.log;
    }
    if (mutation === 'command') f.contract.scope.proof_targets.push('cargo test --manifest-path Cargo.toml --test different --');
    if (mutation === 'self-review') {
      f.reviewData.reviewer_session = 'implementer';
      f.contract.handoff.ready_evidence_refs.source_reviews = [f.file('review.json', f.reviewData)];
    }
    if (mutation === 'completed') f.contract.lifecycle.completed_by = 'implementer';
    f.contract.handoff.ready_evidence_refs.command_results[0].receipt = f.file('bound.json', f.receiptData);
    const result = derive(f);
    const id = mutation === 'completed' ? 'RC-006-IMPLEMENTER-NOT-SELF-CERTIFYING' : 'RC-005-PROOF-COMMANDS';
    assert.equal(result.items.get(id).answer, 'no', mutation);
  });
});

test('Cargo positional filters, full-target coverage, and default feature mode cannot be substituted', () => {
  for (const mode of ['wrong-filter', 'filtered-as-full', 'stripped-defaults', 'stale-lifecycle']) fixture(f => {
    const required = 'cargo test --locked --manifest-path Cargo.toml --features surreal-test-support --test wire';
    f.contract.scope.proof_targets = [mode === 'filtered-as-full' ? `${required} --` : `${required} wire_required -- --exact`];
    const args = ['test', '--locked', '--manifest-path', 'Cargo.toml', '--features', 'surreal-test-support', '--test', 'wire',
      mode === 'wrong-filter' ? 'wire_other' : 'wire_required', '--', '--exact'];
    if (mode === 'stripped-defaults') args.splice(1, 0, '--no-default-features');
    const testName = mode === 'wrong-filter' ? 'wire_other' : 'wire_required';
    const log = f.file('run.log', `test ${testName} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;\nCARGO_EXIT=0\n`);
    f.reviewData.real_resource_tests[0].test_name = testName;
    f.contract.handoff.ready_evidence_refs.source_reviews = [f.file('review.json', f.reviewData)];
    f.receiptData.original_receipt = f.file('original.json', { schema: 'handshake.local_cargo_run@1', exit_code: 0, cwd: f.product, arguments: args });
    Object.assign(f.receiptData, { cargo_arguments: args, log });
    f.contract.handoff.ready_evidence_refs.command_results = [{ receipt: f.file('bound.json', f.receiptData), log }];
    if (mode === 'stale-lifecycle') f.contract.lifecycle.blocked_reason = 'changed after source review';
    const result = derive(f);
    const id = mode === 'stale-lifecycle' ? 'RC-001-NO-STALE-REASONS' : 'RC-005-PROOF-COMMANDS';
    assert.equal(result.items.get(id).answer, 'no', mode);
  });
});

test('direct receipt cannot be paired with a different sibling runtime log', () => fixture(f => {
  const data = { schema: 'handshake.direct_test_run@1', head: f.binding.head, exit_code: 0, proof_valid: true };
  const receipt = f.file('correct.direct.command.json', data);
  const wrongLog = f.file('other.direct.log', 'test wire_real ... ok\ntest result: ok. 1 passed; 0 failed;\n');
  f.contract.handoff.ready_evidence_refs.command_results = [{ receipt, log: wrongLog }];
  const result = derive(f).items.get('RC-005-PROOF-COMMANDS');
  assert.equal(result.answer, 'no');
  assert.match(result.explanation, /not the original sibling runtime log/);
}));


test('post-runtime governance handoff delta is explicit while hidden product drift still blocks', () => {
  for (const productDrift of [false, true]) fixture(f => {
    fs.writeFileSync(path.join(f.product, '.GOV', 'state.json'), '{"phase":"handoff"}');
    if (productDrift) {
      f.git(['update-index', '--skip-worktree', 'wire.rs']);
      fs.writeFileSync(path.join(f.product, 'wire.rs'), 'pub fn changed() {}\n');
    }
    const result = derive(f);
    const proof = result.items.get('RC-005-PROOF-COMMANDS');
    assert.equal(proof.answer, productDrift ? 'no' : 'yes', proof.explanation);
    if (!productDrift) {
      assert.deepEqual(result.runtime_bindings[0].governance_delta.map(row => row.path), ['.GOV/state.json']);
      assert.match(result.runtime_bindings[0].governance_delta_sha256, /^[a-f0-9]{64}$/);
      assert.equal(result.runtime_bindings[0].historical_runtime_manifest_sha256, f.binding.manifest_sha256);
      assert.notEqual(result.binding.manifest_sha256, f.binding.manifest_sha256);
    }
  });
});


test('retained exact evidence copies survive original cleanup and reject changed retained bytes', () => {
  for (const tamper of [false, true]) fixture(f => {
    const refs = f.contract.handoff.ready_evidence_refs;
    const retained = path.join(f.root, 'retained');
    fs.mkdirSync(retained);
    const originals = [refs.source_reviews[0], refs.command_results[0].receipt, f.log,
      f.receiptData.before_manifest, f.receiptData.original_receipt];
    const artifacts = originals.map(original => {
      const filename = path.join(retained, path.basename(original.path));
      fs.copyFileSync(original.path, filename);
      return { original, retained: { path: filename, sha256: original.sha256 } };
    });
    refs.retained_artifacts = f.file('retained-map.json', { schema_id: 'hsk.retained_evidence_map@1', artifacts });
    for (const original of originals) fs.unlinkSync(original.path);
    if (tamper) fs.appendFileSync(artifacts[0].retained.path, 'changed');
    const result = derive(f);
    assert.equal(result.items.get('RC-005-PROOF-COMMANDS').answer, tamper ? 'no' : 'yes');
    if (!tamper) assert.deepEqual(result.retained_artifacts.artifacts, artifacts);
  });
});

test('direct original joins survive executable cleanup and reject detached manifests, owner, cwd and prepare', () => {
  for (const fault of [null, 'manifest', 'owner', 'cwd', 'build-cwd', 'prepare']) fixture(f => {
    const args = ['test', '--locked', '--manifest-path', 'Cargo.toml', '--features', 'surreal-test-support', '--test', 'wire', '--no-run'];
    const owner = 'fixture-owner';
    const shared = { owner, head: f.binding.head, product: f.product, target: path.join(f.root, 'target'),
      source_manifest: f.receiptData.before_manifest, tools: { rustc_version: 'host: native-platform' }, selected_targets: ['wire'] };
    const prepare = f.file('prepare.json', { schema: 'handshake.direct_test_prepare@2', ...shared,
      ...(fault === 'prepare' ? { head: '0'.repeat(40) } : {}) });
    const build = f.file('build.command.json', { schema: 'handshake.local_cargo_run@1', exit_code: 0, arguments: args, owner,
      cwd: fault === 'build-cwd' ? f.root : f.product });
    const executable = { path: path.join(f.root, 'removed-test.exe'), sha256: '1'.repeat(64) };
    const registry = f.file('registry.json', { schema: 'handshake.direct_test_registry@2', ...shared,
      compiled_source_manifest: shared.source_manifest, runtime_source_manifest: shared.source_manifest,
      build_provenance: [prepare, build], tests: { wire: executable }, bins: [], dlls: [] });
    const log = f.file('real.direct.log', 'test wire_real ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n');
    const receipt = f.file('real.direct.command.json', { schema: 'handshake.direct_test_run@1', head: f.binding.head,
      owner: fault === 'owner' ? 'other-owner' : owner, cwd: fault === 'cwd' ? f.product : path.join(f.product, 'src/backend/handshake_core'),
      exit_code: 0, proof_valid: true, registry, source_manifest: fault === 'manifest' ? f.file('detached-source.txt', f.binding.manifest) : shared.source_manifest,
      compiled_source_manifest: shared.source_manifest, runtime_source_manifest: shared.source_manifest,
      build_provenance: [prepare, build], command: [executable.path, 'wire_real', '--exact'], expected_passed: 1,
      result: { status: 'ok', passed: 1, failed: 0, ignored: 0, measured: 0, filtered: 0 } });
    f.contract.handoff.ready_evidence_refs.command_results = [{ receipt, log }];
    const result = derive(f);
    assert.equal(result.items.get('RC-005-PROOF-COMMANDS').answer, fault ? 'no' : 'yes', fault || result.items.get('RC-005-PROOF-COMMANDS').explanation);
    if (!fault) assert.deepEqual(result.runtime_bindings[0].historical_runtime_dependencies, [executable]);
  });
});

test('locked mode, restrictive build selectors and scope proof commands stay mandatory', () => {
  for (const fault of ['unlocked', 'wrong-bin', 'lib-for-default', 'bin-for-bins', 'lib-for-tests', 'scope-command']) fixture(f => {
    const args = ['check', '--locked', '--manifest-path', 'Cargo.toml'];
    let required = 'cargo check --locked --manifest-path Cargo.toml';
    if (fault === 'unlocked') args.splice(1, 1);
    if (fault === 'wrong-bin') { required += ' --bin required'; args.push('--bin', 'other'); }
    if (fault === 'lib-for-default') args.push('--lib');
    if (fault === 'bin-for-bins') { required += ' --bins'; args.push('--bin', 'single'); }
    if (fault === 'lib-for-tests') { required += ' --tests'; args.push('--lib'); }
    const log = f.file('check.log', 'CARGO_EXIT=0\n');
    const original = f.file('check-original.json', { schema: 'handshake.local_cargo_run@1', exit_code: 0, cwd: f.product, arguments: args });
    const receipt = f.file('check-bound.json', { ...f.receiptData, cargo_arguments: args, original_receipt: original, log });
    f.contract.handoff.ready_evidence_refs.command_results.push({ receipt, log });
    if (fault === 'scope-command') f.contract.scope.proof_commands = ['cargo check --locked --manifest-path Cargo.toml --all-targets'];
    else f.contract.scope.proof_targets.push(required);
    const result = derive(f).items.get('RC-005-PROOF-COMMANDS');
    assert.equal(result.answer, 'no', fault);
  });
});


test('only exact reviewed build3 governance rows reconcile; product, target and compiled drift fail', () => {
  const names = ['.GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs', '.GOV/task_packets/stubs/WP-1-Handshake-Stage-MVP-v1.contract.json'];
  const row = (name, hash) => `P\ttracked\tS\t100644\t${'1'.repeat(40)}\t${name}\t1\t${hash.repeat(40)}`;
  const manifest = rows => `hsk.mt013_compiled_worktree_manifest@2\n${rows.join('\n')}\n`;
  const before = names.map(name => row(name, '2')), after = names.map(name => row(name, '3'));
  const prepared = manifest([...before, row('wire.rs', '4')]);
  const runtime = manifest([...after, row('wire.rs', '4')]);
  const record = { schema: 'handshake.exact_build_input_reconciliation@1', batch: 'promotion-repaired-build-3',
    selected_targets: ['model_lane_promotion_surreal_tests'], exact_rows: names.map((name, i) => ({ path: name, prepared: before[i], compiled: before[i], current: after[i] })) };
  const result = verifyBuild3ReconciliationRows(record, prepared, prepared, runtime);
  assert.notEqual(result.historical_build_manifest_sha256, result.historical_runtime_manifest_sha256);
  assert.equal(result.compiled_runtime_delta.length, 2);
  assert.throws(() => verifyBuild3ReconciliationRows(record, prepared, runtime, runtime), /compiled source/);
  assert.throws(() => verifyBuild3ReconciliationRows(record, prepared, prepared, manifest([...after, row('wire.rs', '5')])), /two-row delta/);
  for (const target of ['llama_cpp_e2e_smoke', 'kernel_mechanical_contract_generation_tests']) {
    assert.throws(() => verifyBuild3ReconciliationRows({ ...record, selected_targets: [target] }, prepared, prepared, runtime), /selected targets/);
  }
  assert.throws(() => verifyBuild3ReconciliationRows({ ...record, exact_rows: record.exact_rows.slice(0, 1) }, prepared, prepared, runtime), /two-row delta/);
});


test('nocapture split verdict is attributed only to the explicitly serial test invocation', () => {
  for (const serial of [false, true]) fixture(f => {
    const args = [...f.receiptData.cargo_arguments, ...(serial ? ['--test-threads=1'] : [])];
    const log = f.file('run.log', 'test wire_real ... [REAL_RESOURCE_PROOF] receipt.json\nok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\nCARGO_EXIT=0\n');
    const original = f.file('original.json', { schema: 'handshake.local_cargo_run@1', exit_code: 0, cwd: f.product, arguments: args });
    const receipt = f.file('bound.json', { ...f.receiptData, cargo_arguments: args, original_receipt: original, log });
    f.contract.handoff.ready_evidence_refs.command_results = [{ receipt, log }];
    const result = derive(f).items.get('RC-005-PROOF-COMMANDS');
    assert.equal(result.answer, serial ? 'yes' : 'no', result.explanation);
  });
});


test('retained manifest generations use exact original path and requested hash even when current path exists', () => fixture(f => {
  const old = f.receiptData.before_manifest;
  const preservedOld = f.file('preserved-old.txt', f.binding.manifest);
  const newText = f.binding.manifest.replace('wire.rs', 'different.rs');
  const preservedNew = f.file('preserved-new.txt', newText);
  fs.writeFileSync(old.path, newText);
  const artifacts = [{ original: old, retained: preservedOld },
    { original: { path: old.path, sha256: preservedNew.sha256 }, retained: preservedNew }];
  f.contract.handoff.ready_evidence_refs.retained_artifacts = f.file('generations.json', { schema_id: 'hsk.retained_evidence_map@1', artifacts });
  assert.equal(derive(f).items.get('RC-005-PROOF-COMMANDS').answer, 'yes');
  f.contract.handoff.ready_evidence_refs.retained_artifacts = f.file('generations.json', { schema_id: 'hsk.retained_evidence_map@1', artifacts: artifacts.slice(1) });
  const missing = derive(f).items.get('RC-005-PROOF-COMMANDS');
  assert.equal(missing.answer, 'no');
  assert.match(missing.explanation, /hash mismatch/);
}));
