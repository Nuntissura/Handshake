import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const canonicalJson = value => JSON.stringify(value, (_, item) => item && typeof item === 'object' && !Array.isArray(item) ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)) : item);
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const requireThat = (condition, reason) => { if (!condition) throw new Error(reason); };
const git = (root, args, input) => execFileSync('git', ['-C', root, ...args], { input, encoding: 'utf8', windowsHide: true, maxBuffer: 32 * 1024 * 1024 });

// Same v2 member/index/raw-byte closure as handshake_core/build.rs. No generated
// checklist or asserted current-HEAD field substitutes for reading these bytes.
export function readSourceBinding(root) {
  root = fs.realpathSync(root);
  requireThat(!git(root, ['status', '--porcelain', '--untracked-files=all']).trim(), 'product worktree is dirty');
  const head = git(root, ['rev-parse', 'HEAD']).trim();
  const tree = git(root, ['rev-parse', 'HEAD^{tree}']).trim();
  const members = new Map();
  for (const entry of git(root, ['ls-files', '--cached', '--stage', '-v', '-z']).split('\0').filter(Boolean)) {
    const tab = entry.indexOf('\t');
    const fields = entry.slice(0, tab).split(/\s+/);
    const name = entry.slice(tab + 1);
    requireThat(tab >= 0 && fields.length === 4 && fields[3] === '0' && !members.has(name), 'invalid tracked source entry');
    members.set(name, fields.slice(0, 3));
  }
  for (const name of git(root, ['ls-files', '--others', '--exclude-standard', '-z']).split('\0').filter(Boolean)) {
    requireThat(!members.has(name), 'duplicate source entry');
    members.set(name, null);
  }
  requireThat(members.size > 0, 'empty source closure');
  const names = [...members.keys()].sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b)));
  const present = [], absent = new Map();
  for (const name of names) {
    requireThat(!/[\r\n\t]/.test(name), 'source path contains control separators');
    const tracked = members.get(name);
    let stat;
    try { stat = fs.lstatSync(path.join(root, name)); } catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (tracked?.[1] === '160000') absent.set(name, 'gitlink');
    else if (!stat && ['S', 's'].includes(tracked?.[0])) absent.set(name, 'skip-worktree');
    else {
      requireThat(stat && (stat.isFile() || stat.isSymbolicLink()), `unavailable source member: ${name}`);
      present.push(name);
    }
  }
  const hashes = git(root, ['hash-object', '--no-filters', '--stdin-paths'], present.map(name => `${name}\n`).join('')).trim().split('\n');
  requireThat(hashes.length === present.length, 'source object count mismatch');
  const objects = new Map(present.map((name, index) => [name, hashes[index]]));
  const lines = ['hsk.mt013_compiled_worktree_manifest@2'];
  for (const name of names) {
    const tracked = members.get(name);
    lines.push(absent.has(name)
      ? ['A', absent.get(name), ...tracked, name].join('\t')
      : ['P', ...(tracked ? ['tracked', ...tracked] : ['untracked', '-', '-', '-']), name, fs.lstatSync(path.join(root, name)).size, objects.get(name)].join('\t'));
  }
  const manifest = `${lines.join('\n')}\n`;
  return { head, tree, product: root, manifest_sha256: sha(manifest), manifest };
}

export function verifiedFile(ref, base) {
  requireThat(ref && typeof ref.path === 'string' && /^[a-f0-9]{64}$/i.test(ref.sha256), 'hashed evidence reference required');
  const cacheKey = canonicalJson(ref);
  if (typeof base !== 'string' && base.files?.has(cacheKey)) return base.files.get(cacheKey);
  const root = typeof base === 'string' ? base : base.root;
  const originalFilename = path.resolve(root, ref.path);
  const retained = typeof base === 'string' ? null : base.retained.get(`${originalFilename.toLowerCase()}\0${ref.sha256}`);
  if (retained) requireThat(retained.original.sha256 === ref.sha256, 'retained reference changed original hash');
  const filename = fs.realpathSync(retained ? path.resolve(root, retained.retained.path) : originalFilename);
  const bytes = fs.readFileSync(filename);
  requireThat(sha(bytes) === ref.sha256.toLowerCase(), `evidence hash mismatch: ${ref.path}`);
  const result = { filename, originalFilename, bytes, text: bytes.toString('utf8').replace(/^\uFEFF/, '') };
  if (typeof base !== 'string') base.files?.set(cacheKey, result);
  return result;
}
function evidenceContext(input, root) {
  const context = { root, retained: new Map(), relocation: null, files: new Map(), json: new Map(), pairs: new Map(), jsonl: new Map() };
  if (!input?.retained_artifacts) return context;
  const source = verifiedFile(input.retained_artifacts, root);
  const map = JSON.parse(source.text);
  requireThat(map.schema_id === 'hsk.retained_evidence_map@1' && Array.isArray(map.artifacts), 'unsupported retained evidence map');
  for (const entry of map.artifacts) {
    requireThat(entry.original?.path && path.isAbsolute(entry.original.path) && entry.original.sha256 === entry.retained?.sha256, 'retention must preserve exact original path/hash');
    const key = `${path.resolve(entry.original.path).toLowerCase()}\0${entry.original.sha256}`;
    requireThat(!context.retained.has(key), 'duplicate retained original path/hash');
    verifiedFile(entry.retained, root);
    context.retained.set(key, entry);
  }
  context.relocation = { reference: input.retained_artifacts, artifacts: map.artifacts };
  return context;
}
const jsonRef = (ref, base) => {
  const key = canonicalJson(ref);
  if (typeof base !== 'string' && base.json?.has(key)) return base.json.get(key);
  const value = JSON.parse(verifiedFile(ref, base).text);
  if (typeof base !== 'string') base.json?.set(key, value);
  return value;
};
const samePath = (a, b) => path.resolve(a).toLowerCase() === path.resolve(b).toLowerCase();
function jsonLines(ref, base) {
  const key = canonicalJson(ref);
  if (typeof base !== 'string' && base.jsonl?.has(key)) return base.jsonl.get(key);
  const rows = verifiedFile(ref, base).text.trim().split(/\r?\n/).map(line => JSON.parse(line));
  if (typeof base !== 'string') base.jsonl?.set(key, rows);
  return rows;
}

function tokens(command) {
  // Parse declarative Cargo commands only; never execute contract text. Preserve
  // quoted paths. An environment prefix may precede the Cargo token.
  const source = String(command).slice(String(command).search(/\bcargo\s/));
  requireThat(source.startsWith('cargo ') && !/[;&|`\r\n]/.test(source), 'unsupported proof command syntax');
  return source.match(/"[^"]*"|'[^']*'|\S+/g).map(token => token.replace(/^(['"])(.*)\1$/, '$2'));
}
function optionValues(args, option) {
  const values = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i] === option) values.push(args[++i]);
    else if (args[i].startsWith(`${option}=`)) values.push(args[i].slice(option.length + 1));
  }
  return values;
}
function commandShape(args) {
  const split = args.indexOf('--');
  const before = split < 0 ? args : args.slice(0, split);
  const after = split < 0 ? [] : args.slice(split + 1);
  const subcommand = before.find(token => ['test', 'check', 'build'].includes(token));
  const positional = [];
  const valueOptions = new Set(['--target-dir', '--manifest-path', '--features', '--test', '--bin', '--example', '--bench', '--target', '--jobs', '-j', '--profile', '--message-format', '--package', '-p', '--exclude', '--config', '--color']);
  for (let i = before.indexOf(subcommand) + 1; i < before.length; i++) {
    if (valueOptions.has(before[i])) { i++; continue; }
    if (!before[i].startsWith('-')) positional.push(before[i]);
  }
  return { subcommand, before, after, targets: optionValues(before, '--test'),
    features: optionValues(before, '--features').flatMap(value => value.split(',')),
    manifest: optionValues(before, '--manifest-path')[0],
    filters: [...positional, ...after.filter(token => !token.startsWith('-') && !/^\d+$/.test(token))] };
}
function covers(required, actual, passedTests, requiredTarget = null) {
  if (String(required).trim() === 'git diff --check') return JSON.stringify(actual) === JSON.stringify(['git', 'diff', '--check']);
  const need = commandShape(tokens(required).slice(1));
  const got = commandShape(actual);
  if (need.subcommand !== got.subcommand || need.manifest !== got.manifest) return false;
  if (need.before.includes('--locked') && !got.before.includes('--locked')) return false;
  for (const option of ['--package', '-p', '--exclude']) {
    if (optionValues(need.before, option).join() !== optionValues(got.before, option).join()) return false;
  }
  const selectionFlags = ['--lib', '--bins', '--tests', '--examples', '--benches', '--all-targets'];
  const selectionOptions = ['--bin', '--test', '--example', '--bench'];
  const selected = shape => selectionFlags.some(flag => shape.before.includes(flag)) || selectionOptions.some(option => optionValues(shape.before, option).length);
  if (!selected(need) && selected(got) && !got.before.includes('--all-targets')) return false;
  for (const flag of selectionFlags) {
    if (need.before.includes(flag) && !got.before.includes(flag) && !got.before.includes('--all-targets')) return false;
  }
  for (const option of ['--bin', '--example', '--bench']) {
    if (!got.before.includes('--all-targets') && !optionValues(need.before, option).every(name => optionValues(got.before, option).includes(name))) return false;
  }
  if (!need.features.every(value => got.features.includes(value))) return false;
  const providerProofFeatures = new Set(['test-utils', 'surreal-test-support']);
  if (got.features.some(value => !need.features.includes(value) && !providerProofFeatures.has(value))) return false;
  if (need.before.includes('--no-default-features') !== got.before.includes('--no-default-features')) return false;
  if (need.before.includes('--all-features') !== got.before.includes('--all-features')) return false;
  if (optionValues(need.before, '--profile').join() !== optionValues(got.before, '--profile').join() || need.before.includes('--release') !== got.before.includes('--release')) return false;
  if (optionValues(need.before, '--target').join() !== optionValues(got.before, '--target').join()) return false;
  if (!(requiredTarget ? [requiredTarget] : need.targets).every(value => got.targets.includes(value))) return false;
  for (const flag of ['--all-targets', '--workspace', '--all-features', '--no-default-features', '--lib']) {
    if (need.before.includes(flag) && !got.before.includes(flag)) return false;
  }
  if (need.subcommand !== 'test') return true;
  if (need.after.includes('--ignored') !== got.after.includes('--ignored')) return false;
  if (!need.filters.length) return !got.filters.length;
  return need.filters.every(name => passedTests.has(name));
}

function manifestRows(text) {
    const lines = text.trimEnd().split('\n');
    requireThat(lines.shift() === 'hsk.mt013_compiled_worktree_manifest@2', 'unsupported runtime manifest');
    const result = new Map();
    for (const line of lines) {
      const cells = line.split('\t');
      requireThat((cells[0] === 'P' && cells.length === 8) || (cells[0] === 'A' && cells.length === 6), 'malformed runtime manifest row');
      requireThat(!result.has(cells[5]), 'duplicate runtime manifest member');
      result.set(cells[5], line);
    }
    return result;
  }

function runtimeManifestComparison(refs, base, binding) {
  const saved = refs.map(ref => verifiedFile(ref, base).text);
  requireThat(saved.length >= 2 && saved.every(text => text === saved[0]), 'runtime pre/compiled/post full manifests differ');
  const historical = manifestRows(saved[0]), current = manifestRows(binding.manifest), delta = [];
  for (const name of [...new Set([...historical.keys(), ...current.keys()])].sort()) {
    const before = historical.get(name) ?? null, after = current.get(name) ?? null;
    if (before === after) continue;
    requireThat(name.startsWith('.GOV/') && !binding.owned_files.includes(name), `product source changed since runtime: ${name}`);
    delta.push({ path: name, historical_row: before, current_row: after });
  }
  return { historical_runtime_manifest_sha256: sha(saved[0]), current_product_head: binding.head,
    current_product_tree: binding.tree, governance_delta_sha256: sha(canonicalJson(delta)), governance_delta: delta };
}

// Only the separately reviewed build3 record is accepted. This function checks
// its exact row semantics; the caller also verifies the immutable record pin.
export function verifyBuild3ReconciliationRows(record, prepared, compiled, runtime) {
  requireThat(record.schema === 'handshake.exact_build_input_reconciliation@1' && record.batch === 'promotion-repaired-build-3', 'unsupported reconciliation schema/batch');
  requireThat(canonicalJson(record.selected_targets) === canonicalJson(['model_lane_promotion_surreal_tests']), 'reconciliation does not cover selected targets');
  requireThat(prepared === compiled, 'reconciliation changed compiled source');
  const old = manifestRows(prepared), current = manifestRows(runtime);
  const delta = [...new Set([...old.keys(), ...current.keys()])].sort().filter(name => old.get(name) !== current.get(name))
    .map(name => ({ path: name, prepared: old.get(name) ?? null, compiled: old.get(name) ?? null, current: current.get(name) ?? null }));
  const names = ['.GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs', '.GOV/task_packets/stubs/WP-1-Handshake-Stage-MVP-v1.contract.json'];
  requireThat(canonicalJson(delta.map(row => row.path)) === canonicalJson(names) && canonicalJson(delta) === canonicalJson(record.exact_rows), 'reconciliation differs from exact reviewed two-row delta');
  return { historical_build_manifest_sha256: sha(prepared), historical_runtime_manifest_sha256: sha(runtime),
    compiled_runtime_delta_sha256: sha(canonicalJson(delta)), compiled_runtime_delta: delta };
}
function reconciledRuntimeBinding(receipt, registry, base, binding) {
  const ref = receipt.reconciliation;
  requireThat(canonicalJson(ref) === canonicalJson(registry.reconciliation) && ref.sha256 === 'ba2c3b5cd6cac65f521b222c7256c7d0a61db03281f6a90f49afd22af2dff378', 'unsupported reconciliation reference');
  const record = jsonRef(ref, base);
  requireThat(record.owner === registry.owner && record.product_head === registry.head && canonicalJson(record.selected_targets) === canonicalJson(registry.selected_targets), 'reconciliation identity mismatch');
  for (const [left, right] of [['original_prepared_manifest', 'source_manifest'], ['original_compiled_manifest', 'compiled_source_manifest'], ['runtime_manifest', 'runtime_source_manifest'], ['build_provenance', 'build_provenance']]) {
    requireThat(canonicalJson(record[left]) === canonicalJson(registry[right]), 'reconciliation changed original provenance');
  }
  const rows = verifyBuild3ReconciliationRows(record, ...[receipt.source_manifest, receipt.compiled_source_manifest, receipt.runtime_source_manifest].map(value => verifiedFile(value, base).text));
  return { ...runtimeManifestComparison([receipt.runtime_source_manifest, receipt.runtime_source_manifest], base, binding),
    ...rows, reconciliation: ref };
}

function verifyCommand(row, base, binding) {
  const receipt = jsonRef(row.receipt, base);
  const log = verifiedFile(row.log, base).text;
  if (receipt.schema_id === 'hsk.component_test_run@1') return verifyComponentRun(row, receipt, log, base, binding);
  let args, platform, temporalBinding, runtimeDependencies = [];
  requireThat(receipt.exit_code === 0 && receipt.proof_valid === true && receipt.head === binding.head, 'command failed, unbound, or stale');
  if (receipt.schema === 'handshake.direct_test_run@1') {
    const receiptPath = verifiedFile(row.receipt, base).originalFilename;
    requireThat(receiptPath.endsWith('.direct.command.json'), 'unrecognized direct receipt filename');
    const expectedLog = receiptPath.slice(0, -'.direct.command.json'.length) + '.direct.log';
    requireThat(samePath(verifiedFile(row.log, base).originalFilename, expectedLog), 'direct log is not the original sibling runtime log');
    if (receipt.log) requireThat(receipt.log.sha256 === row.log.sha256 && samePath(verifiedFile(receipt.log, base).originalFilename, expectedLog), 'direct log binding mismatch');
    const result = receipt.result;
    const final = [...log.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/gm)].at(-1);
    requireThat(final && result?.status === 'ok' && final[1] === 'ok' && Number.isInteger(receipt.expected_passed) && receipt.expected_passed > 0 && +final[2] === receipt.expected_passed && result.passed === receipt.expected_passed && result.failed === 0 && +final[3] === 0 && +final[4] === result.ignored && +final[5] === result.measured && +final[6] === result.filtered, 'direct parent summary differs from original receipt');
    const registry = jsonRef(receipt.registry, base);
    requireThat(registry.schema === 'handshake.direct_test_registry@2' && registry.head === binding.head && samePath(registry.product, binding.product), 'direct registry source mismatch');
    requireThat(typeof registry.owner === 'string' && registry.owner && receipt.owner === registry.owner && samePath(receipt.cwd, path.join(binding.product, 'src/backend/handshake_core')), 'direct owner or package cwd mismatch');
    for (const key of ['source_manifest', 'compiled_source_manifest', 'runtime_source_manifest']) {
      requireThat(canonicalJson(receipt[key]) === canonicalJson(registry[key]), 'direct manifest differs from registered manifest');
    }
    temporalBinding = receipt.reconciliation ? reconciledRuntimeBinding(receipt, registry, base, binding)
      : runtimeManifestComparison([receipt.source_manifest, receipt.compiled_source_manifest, receipt.runtime_source_manifest], base, binding);
    requireThat(JSON.stringify(registry.build_provenance) === JSON.stringify(receipt.build_provenance), 'direct build provenance differs from registered provenance');
    const build = receipt.build_provenance.map(ref => ({ ref, value: verifiedFile(ref, base) }));
    const records = build.map(({ value }) => { try { return JSON.parse(value.text); } catch { return null; } });
    const commandReceipts = records.filter(value => value?.schema === 'handshake.local_cargo_run@1');
    requireThat(commandReceipts.length === 1 && commandReceipts[0].exit_code === 0 && commandReceipts[0].owner === registry.owner && samePath(commandReceipts[0].cwd, binding.product), 'missing successful original Cargo build provenance');
    const prepares = records.filter(value => value?.schema === 'handshake.direct_test_prepare@2');
    requireThat(prepares.length === 1, 'missing unique prospective prepare');
    for (const key of ['owner', 'head', 'product', 'target', 'source_manifest', 'tools', 'selected_targets']) {
      requireThat(canonicalJson(prepares[0][key]) === canonicalJson(registry[key]), `prepared ${key} differs from registry`);
    }
    const test = Object.entries(registry.tests).find(([, ref]) => samePath(ref.path, receipt.command[0]));
    requireThat(test, 'direct executable was not registered');
    // The direct runner verified these exact hashes before/after execution.
    // User-requested build cleanup does not require retaining executable bytes.
    runtimeDependencies = [test[1], ...registry.bins, ...registry.dlls];
    for (const ref of runtimeDependencies) {
      requireThat(typeof ref.path === 'string' && /^[a-f0-9]{64}$/.test(ref.sha256), 'invalid historical runtime dependency hash');
      if (fs.existsSync(ref.path)) verifiedFile(ref, base);
    }
    args = [...commandReceipts[0].arguments.slice(0, commandReceipts[0].arguments.indexOf('--') < 0 ? undefined : commandReceipts[0].arguments.indexOf('--'))];
    args = args.filter(token => token !== '--no-run');
    // Registration may bundle targets; runtime executes only this exact target.
    for (let i = args.length - 1; i >= 0; i--) if (args[i] === '--test') args.splice(i, 2);
    args.push('--test', test[0], '--', ...receipt.command.slice(1));
    platform = /host: (\S+)/.exec(registry.tools.rustc_version)?.[1];
  } else if (receipt.schema === 'handshake.bound_command_run@1') {
    requireThat(receipt.tree === binding.tree && samePath(receipt.product, binding.product), 'bound command tree mismatch');
    temporalBinding = runtimeManifestComparison(receipt.cargo_arguments?.[0] === 'git'
      ? [receipt.before_manifest, receipt.after_manifest]
      : [receipt.before_manifest, receipt.compiled_source_manifest, receipt.after_manifest], base, binding);
    const original = jsonRef(receipt.original_receipt, base);
    requireThat(['handshake.local_cargo_run@1', 'handshake.local_command_run@1'].includes(original.schema) && original.exit_code === 0 && samePath(original.cwd, binding.product), 'original Cargo receipt failed or wrong cwd');
    requireThat(JSON.stringify(original.arguments) === JSON.stringify(receipt.cargo_arguments), 'bound command changed original arguments');
    requireThat(receipt.log.sha256 === row.log.sha256, 'bound command log mismatch');
    const markers = [...log.matchAll(/^(?:CARGO|COMMAND)_EXIT=(-?\d+)\s*$/gm)];
    requireThat(markers.length > 0 && markers.at(-1)[1] === '0', 'original log has no successful terminal marker');
    args = original.arguments;
    platform = receipt.platform;
  } else throw new Error('unsupported source-bound command receipt');
  const summaries = [...log.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;/gm)];
  if (commandShape(args).subcommand === 'test') {
    requireThat(summaries.length > 0 && summaries.every(match => match[1] === 'ok' && +match[2] > 0 && +match[3] === 0), 'missing, zero-test, or failed runtime result');
  }
  const passedTests = new Set([...log.matchAll(/^test (\S+) \.\.\. ok\s*$/gm)].map(match => match[1]));
  // --nocapture diagnostics may interrupt the harness line. Attribute a later
  // standalone verdict only for the original explicitly serial invocation.
  if (optionValues(commandShape(args).after, '--test-threads').join() === '1') {
    let pending = null;
    for (const line of log.split(/\r?\n/)) {
      const start = /^test (\S+) \.\.\. (.*)$/.exec(line);
      if (start) pending = /^(ok|FAILED|ignored)\b/.test(start[2]) ? null : start[1];
      else if (pending && /^(ok|FAILED|ignored)\s*$/.test(line)) {
        if (line.trim() === 'ok') passedTests.add(pending);
        pending = null;
      }
    }
  }
  if (row.external_resource) verifiedFile(row.external_resource, base);
  return { args, platform, passedTests, temporalBinding, runtimeDependencies, ref: row.receipt.path };
}

function componentRows(snapshot) {
  requireThat(Array.isArray(snapshot.files) && snapshot.files.length > 0 && Array.isArray(snapshot.environment), 'missing original input components');
  const files = new Map();
  for (const line of snapshot.files) {
    const split = line.lastIndexOf('='), name = line.slice(0, split), hash = line.slice(split + 1);
    requireThat(split > 0 && !files.has(name) && (hash === 'MISSING' || /^[a-f0-9]{64}$/i.test(hash)), 'invalid/duplicate input component');
    files.set(name, hash.toLowerCase());
  }
  requireThat(new Set(snapshot.environment.map(row => row.name)).size === snapshot.environment.length && snapshot.environment.every(row => row.name && /^[a-f0-9]{64}$/i.test(row.value_sha256)), 'invalid environment component');
  return files;
}

export function verifyComponentPair(beforeRef, afterRef, base, binding, { compile = false } = {}) {
  const cacheKey = canonicalJson([beforeRef, afterRef, binding.head, binding.tree, compile]);
  if (typeof base !== 'string' && base.pairs?.has(cacheKey)) return base.pairs.get(cacheKey);
  const before = jsonRef(beforeRef, base), after = jsonRef(afterRef, base);
  requireThat(samePath(before.product_root, binding.product) && samePath(after.product_root, binding.product), 'component product root mismatch');
  const files = componentRows(before), afterFiles = componentRows(after);
  requireThat(canonicalJson([...files].sort()) === canonicalJson([...afterFiles].sort()) && canonicalJson(before.environment) === canonicalJson(after.environment) && before.fingerprint === after.fingerprint, 'inputs changed during proof');
  if (compile) requireThat(canonicalJson(before.selection) === canonicalJson({ roots: ['.'], exclude: ['.GOV/'], enumerator: 'git ls-files --cached --others --exclude-standard' }), 'unsupported compile input selection');
  const current = [...new Set(git(binding.product, ['ls-files', '--cached', '--others', '--exclude-standard', '-z']).split('\0').filter(name => name && !name.startsWith('.GOV/')))];
  const productNames = [...files.keys()].filter(name => !path.isAbsolute(name) && !name.startsWith('.GOV/'));
  requireThat(canonicalJson(current.sort()) === canonicalJson(productNames.sort()), 'component product membership changed or incomplete');
  for (const [name, hash] of files) {
    const filename = path.resolve(binding.product, name);
    if (!path.isAbsolute(name)) requireThat(!path.relative(binding.product, filename).startsWith('..'), 'component escapes product');
    else requireThat(!compile && samePath(filename, path.join(binding.product, '.GOV', path.relative(path.join(binding.product, '.GOV'), filename))) && !path.relative(path.join(binding.product, '.GOV'), filename).startsWith('..'), 'external runtime component is not governance input');
    if (hash === 'missing') requireThat(!fs.existsSync(filename), `missing component appeared: ${name}`);
    else requireThat(fs.existsSync(filename) && sha(fs.readFileSync(filename)) === hash, `current input differs: ${name}`);
  }
  const result = { before, after, files };
  if (typeof base !== 'string') base.pairs?.set(cacheKey, result);
  return result;
}

function verifyHistoricalComponentPair(beforeRef, afterRef, base, binding, original, changed, recovered, { compile = false } = {}) {
  const cacheKey = canonicalJson(['historical', beforeRef, afterRef, binding.head, original.head, [...changed].sort(), [...recovered], compile]);
  if (typeof base !== 'string' && base.pairs?.has(cacheKey)) return base.pairs.get(cacheKey);
  const before = jsonRef(beforeRef, base), after = jsonRef(afterRef, base);
  requireThat(samePath(before.product_root, binding.product) && samePath(after.product_root, binding.product), 'historical component root mismatch');
  const files = componentRows(before), afterFiles = componentRows(after);
  requireThat(canonicalJson([...files].sort()) === canonicalJson([...afterFiles].sort()) && canonicalJson(before.environment) === canonicalJson(after.environment) && before.fingerprint === after.fingerprint, 'historical inputs changed during proof');
  if (compile) requireThat(canonicalJson(before.selection) === canonicalJson({ roots: ['.'], exclude: ['.GOV/'], enumerator: 'git ls-files --cached --others --exclude-standard' }), 'unsupported historical input selection');
  const historicalNames = git(binding.product, ['ls-tree', '-r', '--name-only', '-z', original.head]).split('\0').filter(name => name && !name.startsWith('.GOV/'));
  const productNames = [...files.keys()].filter(name => !path.isAbsolute(name) && !name.startsWith('.GOV/'));
  requireThat(canonicalJson(historicalNames.sort()) === canonicalJson(productNames.sort()), 'historical product membership incomplete');
  for (const [name, hash] of files) {
    const filename = path.resolve(binding.product, name);
    if (path.isAbsolute(name)) requireThat(!compile && !path.relative(path.join(binding.product, '.GOV'), filename).startsWith('..'), 'historical external component is not governance input');
    else requireThat(!path.relative(binding.product, filename).startsWith('..'), 'historical component escapes product');
    if (!path.isAbsolute(name) && changed.has(name)) {
      if (recovered.has(name)) {
        const bytes = verifiedFile(recovered.get(name), base).bytes;
        requireThat(sha(bytes) === hash, `recovered historical source hash differs: ${name}`);
        const cleanBlob = git(binding.product, ['hash-object', `--path=${name}`, '--stdin'], bytes).trim();
        requireThat(cleanBlob === git(binding.product, ['rev-parse', `${original.head}:${name}`]).trim(), `recovered historical Git blob differs: ${name}`);
        continue;
      }
      // A committed working copy may retain LF bytes until its next checkout.
      // Accept only an exact original hash of the blob or checkout-filtered blob.
      const readBlob = mode => execFileSync('git', ['-C', binding.product, 'cat-file', mode, `${original.head}:${name}`], { windowsHide: true, maxBuffer: 32 * 1024 * 1024 });
      requireThat(sha(readBlob('blob')) === hash || sha(readBlob('--filters')) === hash, `historical source differs: ${name}`);
    } else if (hash === 'missing') requireThat(!fs.existsSync(filename), `historical missing input appeared: ${name}`);
    else requireThat(fs.existsSync(filename) && sha(fs.readFileSync(filename)) === hash, `historical unchanged input differs: ${name}`);
  }
  const result = { before, after, files };
  if (typeof base !== 'string') base.pairs?.set(cacheKey, result);
  return result;
}

function verifyComponentReuse(row, receipt, build, artifact, base, binding) {
  const review = jsonRef(row.reuse_review, base);
  requireThat(review.schema_id === 'hsk.component_reuse_review@1' && binding.claimant && review.reviewer_session && review.reviewer_session !== binding.claimant, 'component reuse needs independent reviewer');
  requireThat(review.original_commit === build.source_binding.head && review.original_tree === build.source_binding.tree && review.product_commit === binding.head && review.product_tree === binding.tree, 'reuse review source binding differs');
  requireThat(git(binding.product, ['rev-parse', `${review.original_commit}^{tree}`]).trim() === review.original_tree, 'historical tree differs');
  requireThat(canonicalJson(review.original_receipt) === canonicalJson(row.receipt) && review.target?.kind === receipt.kind && review.target?.name === receipt.name, 'reuse review target or original receipt differs');
  verifiedFile(review.original_receipt, base);
  const delta = git(binding.product, ['diff', '--no-ext-diff', '--no-renames', '--binary', review.original_commit, binding.head, '--']);
  const changed = git(binding.product, ['diff', '--no-renames', '--name-only', '-z', review.original_commit, binding.head, '--']).split('\0').filter(Boolean).sort();
  requireThat(changed.length > 0 && sha(delta) === review.source_delta_sha256, 'reuse source delta changed');
  requireThat(Array.isArray(review.source_findings) && canonicalJson(review.source_findings.map(value => value.path).sort()) === canonicalJson(changed)
    && review.source_findings.every(value => value.status === 'not_a_dependency' && typeof value.reason === 'string' && value.reason.trim()), 'reuse contains unreviewed source delta');
  requireThat(review.runtime_relevance === 'unchanged' && typeof review.runtime_reason === 'string' && review.runtime_reason.trim(), 'runtime dependency relevance not reviewed');
  const recovered = new Map();
  const originalFiles = componentRows(jsonRef(build.input_before, base));
  for (const value of review.historical_source_artifacts || []) {
    requireThat(changed.includes(value.source_path) && originalFiles.has(value.source_path) && !recovered.has(value.source_path) && typeof value.derivation === 'string' && value.derivation.trim(), 'invalid recovered historical source reference');
    verifiedFile(value.artifact, base);
    recovered.set(value.source_path, value.artifact);
  }
  const finalBuild = jsonRef(review.final_build_command, base);
  requireThat(finalBuild.schema_id === 'hsk.build_command@1' && finalBuild.program === 'cargo' && finalBuild.exit_code === 0 && samePath(finalBuild.cwd, binding.product)
    && finalBuild.source_binding?.head === binding.head && finalBuild.source_binding?.tree === binding.tree, 'reuse final build is not current');
  const versionLine = value => typeof value === 'string' ? value.split(/\r?\n/)[0].trim() : '';
  for (const tool of ['cargo_version', 'rustc_version']) requireThat(versionLine(build[tool]) && versionLine(build[tool]) === versionLine(finalBuild[tool]), `reuse ${tool} changed or missing`);
  const finalPair = verifyComponentPair(finalBuild.input_before, finalBuild.input_after, base, binding, { compile: true });
  const records = jsonLines(finalBuild.log, base);
  requireThat(records.at(-1)?.reason === 'build-finished' && records.at(-1).success === true, 'reuse final build did not finish');
  const inventory = jsonRef(review.final_inventory, base);
  requireThat(canonicalJson(inventory) === canonicalJson(records.filter(value => value.reason === 'compiler-artifact' && value.profile?.test && value.executable)), 'reuse final inventory differs');
  const current = inventory.find(value => value.target?.name === receipt.name && value.target.kind.join('-') === receipt.kind);
  const identity = value => ({ package_id: value.package_id, target: value.target, features: value.features, profile: value.profile, executable: value.executable });
  requireThat(current?.fresh === true && canonicalJson(identity(current)) === canonicalJson(identity(artifact)), 'reuse artifact is not fresh with identical compiler configuration');
  requireThat(fs.existsSync(current.executable) && sha(fs.readFileSync(current.executable)) === receipt.binary_sha256_after.toLowerCase(), 'reuse executable hash changed');
  const historicalCompile = verifyHistoricalComponentPair(build.input_before, build.input_after, base, binding, build.source_binding, new Set(changed), recovered, { compile: true });
  const historicalRuntime = verifyHistoricalComponentPair(receipt.runtime_input_before, receipt.runtime_input_after, base, binding, build.source_binding, new Set(changed), recovered);
  const finalRuntime = verifyComponentPair(review.final_runtime_input_before, review.final_runtime_input_after, base, binding);
  for (const [oldSnapshot, newSnapshot, findings] of [
    [historicalCompile.before, finalPair.before, review.compile_environment_findings],
    [historicalRuntime.before, finalRuntime.before, review.environment_findings],
  ]) {
    const oldEnvironment = new Map(oldSnapshot.environment.map(value => [value.name, value.value_sha256]));
    const newEnvironment = new Map(newSnapshot.environment.map(value => [value.name, value.value_sha256]));
    const environmentDelta = [...new Set([...oldEnvironment.keys(), ...newEnvironment.keys()])].sort().filter(name => oldEnvironment.get(name) !== newEnvironment.get(name))
      .map(name => ({ name, before: oldEnvironment.get(name) ?? null, after: newEnvironment.get(name) ?? null }));
    requireThat(Array.isArray(findings) && canonicalJson(findings.map(({ reason, status, ...value }) => value)) === canonicalJson(environmentDelta)
      && findings.every(value => value.status === 'not_a_dependency' && typeof value.reason === 'string' && value.reason.trim()), 'reuse contains unreviewed environment delta');
  }
  return { runtime: historicalRuntime, review: row.reuse_review, original_commit: review.original_commit, product_commit: binding.head };
}

export function verifyComponentRun(row, receipt, log, base, binding) {
  requireThat(receipt.exit === 0 && ['GREEN', 'COMPILED_EMPTY_HARNESS'].includes(receipt.status), 'component test failed');
  requireThat(/^[a-f0-9]{64}$/i.test(receipt.binary_sha256_before) && receipt.binary_sha256_before.toLowerCase() === String(receipt.binary_sha256_after).toLowerCase(), 'runtime binary changed or hash missing');
  const build = jsonRef(receipt.build_command, base);
  requireThat(build.schema_id === 'hsk.build_command@1' && build.program === 'cargo' && build.exit_code === 0 && samePath(build.cwd, binding.product), 'invalid original component build');
  let runtime, reuse;
  if (!row.reuse_review) {
    requireThat(build.source_binding?.head === binding.head && build.source_binding?.tree === binding.tree, 'component build commit is stale');
    verifyComponentPair(build.input_before, build.input_after, base, binding, { compile: true });
    runtime = verifyComponentPair(receipt.runtime_input_before, receipt.runtime_input_after, base, binding);
  }
  const records = jsonLines(build.log, base);
  requireThat(records.at(-1)?.reason === 'build-finished' && records.at(-1).success === true, 'original build did not finish successfully');
  const inventory = jsonRef(receipt.inventory, base);
  const artifacts = records.filter(value => value.reason === 'compiler-artifact' && value.profile?.test && value.executable);
  requireThat(canonicalJson(inventory) === canonicalJson(artifacts), 'inventory differs from original Cargo output');
  const artifact = artifacts.find(value => value.target.name === receipt.name && value.target.kind.join('-') === receipt.kind && samePath(value.executable, receipt.binary));
  requireThat(artifact && Array.isArray(receipt.args), 'runtime executable is not in original compile inventory');
  if (row.reuse_review) { reuse = verifyComponentReuse(row, receipt, build, artifact, base, binding); runtime = reuse.runtime; }
  requireThat(receipt.input_fingerprint === runtime.before.fingerprint, 'runtime fingerprint does not match original components');
  requireThat(receipt.features.split(',').every(feature => artifact.features.includes(feature)), 'runtime features differ from compile artifact');
  if (fs.existsSync(receipt.binary)) requireThat(sha(fs.readFileSync(receipt.binary)) === receipt.binary_sha256_after.toLowerCase(), 'retained runtime binary changed');
  requireThat(samePath(receipt.log, verifiedFile(row.log, base).originalFilename), 'runtime log path differs');
  const summaries = [...log.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/gm)];
  requireThat(summaries.length === 1, 'runtime log must identify exactly one harness result');
  const result = summaries[0];
  requireThat(result[1] === 'ok' && +result[2] === receipt.passed && +result[3] === 0 && receipt.failed === 0 && +result[4] === receipt.ignored && +result[5] === receipt.measured && +result[6] === receipt.filtered && receipt.executed === receipt.passed, 'runtime counts differ from original log');
  const compiledEmpty = receipt.status === 'COMPILED_EMPTY_HARNESS' && receipt.kind === 'bin' && receipt.executed === 0 && +result[4] === 0 && +result[5] === 0 && +result[6] === 0;
  requireThat(receipt.executed > 0 || compiledEmpty, 'component runtime executed zero tests');
  requireThat(!receipt.args.includes('--list'), 'test inventory is not runtime proof');
  const args = build.args.filter(value => value !== '--no-run');
  const split = args.indexOf('--');
  if (split >= 0) args.splice(split);
  for (let index = args.length - 1; index >= 0; index--) {
    if (['--test', '--bin', '--bench', '--example'].includes(args[index])) args.splice(index, 2);
    else if (['--lib', '--tests', '--bins', '--benches', '--examples', '--all-targets'].includes(args[index])) args.splice(index, 1);
  }
  requireThat(['test', 'lib', 'bin'].includes(receipt.kind), 'unsupported runtime harness kind');
  args.push(...(receipt.kind === 'lib' ? ['--lib'] : [`--${receipt.kind}`, receipt.name]), '--', ...receipt.args);
  const passedTests = new Set([...log.matchAll(/^test (\S+) \.\.\. ok\s*$/gm)].map(match => match[1]));
  if (receipt.args.includes('--test-threads=1')) {
    let pending;
    for (const line of log.split(/\r?\n/)) {
      const start = /^test (\S+) \.\.\. (.*)$/.exec(line);
      if (start) pending = /^(ok|FAILED|ignored)\b/.test(start[2]) ? null : start[1];
      else if (pending && /^(ok|FAILED|ignored)\s*$/.test(line)) { if (line.trim() === 'ok') passedTests.add(pending); pending = null; }
    }
  }
  return { args, passedTests, platform: /host: (\S+)/.exec(build.rustc_version || '')?.[1], ref: row.receipt.path,
    name: receipt.name, kind: receipt.kind, executed: receipt.executed, filtered: receipt.filtered, runtime_args: receipt.args, compiledEmpty, inventory_ref: receipt.inventory, artifact,
    temporalBinding: { build_command: receipt.build_command, runtime_input_before: receipt.runtime_input_before, runtime_input_after: receipt.runtime_input_after, ...(reuse ? { historical_runtime_reuse: { review: reuse.review, original_commit: reuse.original_commit, product_commit: reuse.product_commit } } : {}) }, runtimeDependencies: [{ path: receipt.binary, sha256: receipt.binary_sha256_after.toLowerCase() }] };
}

// CI is a protocol-supported alternative to a local cross compilation. Query
// the provider at the handoff boundary; a supplied URL or saved success string
// alone is not evidence that this commit's build succeeded.
export function verifyGithubBuild(ref, binding, nativePlatform, readApi = endpoint => {
  const value = execFileSync('gh', ['api', endpoint], { encoding: 'utf8', windowsHide: true, timeout: 60000, maxBuffer: 32 * 1024 * 1024 });
  return endpoint.endsWith('/logs') ? value : JSON.parse(value);
}) {
  requireThat(ref?.provider === 'github_actions' && /^[\w.-]+\/[\w.-]+$/.test(ref.repository), 'unsupported CI provider/repository');
  requireThat(Number.isSafeInteger(ref.run_id) && ref.run_id > 0 && Number.isSafeInteger(ref.job_id) && ref.job_id > 0, 'invalid CI run/job identity');
  const run = readApi(`repos/${ref.repository}/actions/runs/${ref.run_id}`);
  const job = readApi(`repos/${ref.repository}/actions/jobs/${ref.job_id}`);
  requireThat(run.id === ref.run_id && run.head_sha === binding.head && job.run_id === run.id && job.head_sha === binding.head, 'CI build is for a different commit/run');
  requireThat(job.status === 'completed' && job.conclusion === 'success', 'CI job did not succeed');
  const labels = (job.labels || []).join(' ').toLowerCase();
  const platform = /ubuntu|linux/.test(labels) ? 'linux' : /windows/.test(labels) ? 'windows' : /macos/.test(labels) ? 'darwin' : '';
  const native = /windows/.test(nativePlatform) ? 'windows' : /linux/.test(nativePlatform) ? 'linux' : /darwin|apple/.test(nativePlatform) ? 'darwin' : '';
  requireThat(platform && native && platform !== native, 'CI job does not establish a non-native platform');
  requireThat(Array.isArray(ref.required_steps) && ref.required_steps.length > 0 && ref.required_steps.some(name => /\b(build|compile|check)\b/i.test(name)), 'CI build steps must be identified');
  for (const name of ref.required_steps) requireThat(job.steps?.some(step => step.name === name && step.status === 'completed' && step.conclusion === 'success'), `CI build step did not succeed: ${name}`);
  requireThat(job.html_url === `https://github.com/${ref.repository}/actions/runs/${ref.run_id}/job/${ref.job_id}`, 'CI job URL does not match identity');
  const log = readApi(`repos/${ref.repository}/actions/jobs/${ref.job_id}/logs`);
  requireThat(typeof log === 'string', 'CI original job log unavailable');
  const plain = log.replace(/\x1b\[[0-9;]*m/g, '').split(/\r?\n/).map(line => line.replace(/^\S+Z\s+/, '')).join('\n');
  const checkout = `git log -1 --format=%H\n${binding.head}\n`;
  requireThat(plain.includes(checkout), 'CI checkout does not establish the exact product commit');
  const blocks = plain.split('##[group]Run ').slice(1);
  const build = blocks.find(block => {
    const command = block.split('\n')[0];
    if (!/^cargo\s/.test(command)) return false;
    let shape; try { shape = commandShape(tokens(command).slice(1)); } catch { return false; }
    return shape.manifest === 'src/backend/handshake_core/Cargo.toml'
      && (['check', 'build'].includes(shape.subcommand) || shape.subcommand === 'test' && shape.before.includes('--no-run'))
      && /Finished [`'][^\n]+profile/.test(block);
  });
  requireThat(build, 'CI job has no successful product Cargo build in original log');
  return { platform, ref: job.html_url, head: run.head_sha, steps: ref.required_steps, original_log_sha256: sha(log) };
}

export function verifyProseTargets(targets, mappings, reviews, commands, base, binding) {
  const mapped = new Map();
  requireThat(Array.isArray(mappings), 'typed prose proof-target mappings are missing');
  for (const row of mappings) {
    requireThat(targets.includes(row.target) && !mapped.has(row.target), 'unknown or duplicate prose proof target');
    mapped.set(row.target, row);
  }
  for (const target of targets) {
    const row = mapped.get(target);
    requireThat(row, `unmapped proof target: ${target}`);
    const review = reviews.find(value => value.ref === row.source_review);
    const reviewed = review?.proof_targets?.find(value => value.target === target);
    requireThat(reviewed && canonicalJson(reviewed.artifact_refs) === canonicalJson(row.artifact_refs) && row.artifact_refs?.length > 0, `proof target lacks independent artifact review: ${target}`);
    for (const ref of row.artifact_refs) verifiedFile(ref, base);
    if (row.kind === 'inspection') {
      requireThat(reviewed.kind === 'inspection', 'inspection proof-kind mismatch');
    } else {
      requireThat(row.kind === 'runtime_suite' && reviewed.kind === 'runtime_suite', 'unsupported proof-target kind');
      requireThat(Array.isArray(row.command_receipts) && new Set(row.command_receipts).size === row.command_receipts.length, 'duplicate/missing suite command references');
      const inventory = jsonRef(row.suite_inventory, base);
      requireThat(Array.isArray(inventory) && inventory.length > 0, 'empty suite inventory');
      const expected = inventory.filter(value => value.reason === 'compiler-artifact' && value.profile?.test && value.executable);
      requireThat(expected.length === inventory.length && expected.some(value => value.target.kind.includes('test')), 'invalid compiled suite inventory');
      const key = value => `${value.kind}:${value.name}`;
      const expectedKeys = expected.map(value => key({ kind: value.target.kind.join('-'), name: value.target.name }));
      requireThat(new Set(expectedKeys).size === expectedKeys.length, 'duplicate compiled suite target');
      const metadata = jsonRef(row.suite_metadata, base);
      const packageId = expected[0].package_id;
      const pkg = metadata.packages?.find(value => value.id === packageId);
      requireThat(pkg && expected.every(value => value.package_id === packageId), 'suite package is not in Cargo metadata');
      requireThat(binding, 'suite source binding missing');
      if (!binding.cargo_metadata) {
        binding.cargo_metadata = JSON.parse(execFileSync('cargo', ['metadata', '--locked', '--offline', '--no-deps', '--format-version', '1', '--manifest-path', 'src/backend/handshake_core/Cargo.toml'],
          { cwd: binding.product, encoding: 'utf8', windowsHide: true, maxBuffer: 16 * 1024 * 1024 }));
      }
      const currentPackage = binding.cargo_metadata.packages?.find(value => value.id === packageId);
      requireThat(currentPackage && canonicalJson(currentPackage.targets) === canonicalJson(pkg.targets), 'suite metadata differs from current Cargo targets');
      const metadataKeys = pkg.targets.filter(value => value.test && value.kind.some(kind => ['test', 'lib', 'bin'].includes(kind)))
        .map(value => key({ kind: value.kind.join('-'), name: value.name }));
      requireThat(canonicalJson(metadataKeys.sort()) === canonicalJson([...expectedKeys].sort()), 'suite inventory omits Cargo metadata targets');
      const suiteBuild = jsonRef(row.suite_build_command, base);
      requireThat(binding && suiteBuild.schema_id === 'hsk.build_command@1' && suiteBuild.exit_code === 0 && suiteBuild.program === 'cargo' && samePath(suiteBuild.cwd, binding.product)
        && suiteBuild.source_binding?.head === binding.head && suiteBuild.source_binding?.tree === binding.tree, 'suite build is not source bound');
      verifyComponentPair(suiteBuild.input_before, suiteBuild.input_after, base, binding, { compile: true });
      const buildRecords = jsonLines(suiteBuild.log, base);
      requireThat(buildRecords.at(-1)?.reason === 'build-finished' && buildRecords.at(-1).success === true, 'suite build did not finish');
      requireThat(canonicalJson(buildRecords.filter(value => value.reason === 'compiler-artifact' && value.profile?.test && value.executable)) === canonicalJson(inventory), 'suite inventory differs from original Cargo output');
      const observed = row.command_receipts.map(ref => commands.find(value => value.ref === ref));
      requireThat(observed.every(Boolean) && observed.length === expected.length, 'suite command set is incomplete');
      requireThat(canonicalJson(observed.map(key).sort()) === canonicalJson(expectedKeys.sort()), 'suite target identity set differs from compiled inventory');
      for (const result of observed) {
        requireThat(result.executed > 0 || (result.kind === 'bin' && result.compiledEmpty === true), `suite target did not execute: ${result.name}`);
        requireThat(result.filtered === 0 && Array.isArray(result.runtime_args) && result.runtime_args.every(arg => ['--test-threads=1', '--nocapture'].includes(arg)
          || arg === '--ignored' && ['candle_e2e_smoke', 'llama_cpp_e2e_smoke'].includes(result.name)), `suite target is filtered: ${result.name}`);
        const compiled = expected.find(value => key({ kind: value.target.kind.join('-'), name: value.target.name }) === key(result));
        requireThat(result.artifact && canonicalJson(result.artifact.features) === canonicalJson(compiled.features)
          && canonicalJson(result.artifact.profile) === canonicalJson(compiled.profile)
          && result.artifact.package_id === compiled.package_id && samePath(result.artifact.target.src_path, compiled.target.src_path), 'suite result compile configuration differs');
      }
    }
  }
}

export function deriveMechanicalReady({ contract, evidenceBase, productRoot, nativePlatform }) {
  const errors = [], items = new Map();
  let binding;
  try { binding = { ...readSourceBinding(productRoot), owned_files: contract.owned_files || [] }; } catch (error) { errors.push(error.message); }
  const input = contract?.handoff?.ready_evidence_refs;
  let context;
  try { context = evidenceContext(input, evidenceBase); } catch (error) { errors.push(error.message); }
  const set = (id, ok, explanation, refs = []) => items.set(id, { answer: ok ? 'yes' : 'no', explanation, evidence_refs: refs });
  const claimed = String(contract?.lifecycle?.claimed_by || '').trim();
  if (binding) binding.claimant = claimed;
  const completed = String(contract?.lifecycle?.completed_by || '').trim();
  set('RC-006-IMPLEMENTER-NOT-SELF-CERTIFYING', Boolean(claimed && !completed), `Canonical lifecycle: claimed_by=${claimed || '<empty>'}; completed_by=${completed || '<unset>'}.`);
  const reviews = [], reviewErrors = [];
  if (binding && context) for (const ref of input?.source_reviews || []) {
    try {
      const review = jsonRef(ref, context);
      requireThat(review.schema_id === 'hsk.source_review_findings@1', 'unsupported source review');
      requireThat(review.reviewer_session && review.reviewer_session !== claimed, 'source review must be independent of claimant');
      requireThat(review.product_commit === binding.head && review.product_tree === binding.tree, 'source review is stale');
      requireThat(Array.isArray(review.findings) && review.findings.every(finding => finding.status === 'resolved' || finding.status === 'not_a_defect'), 'unresolved source finding');
      const files = new Map((review.reviewed_files || []).map(file => [file.path, file]));
      requireThat(files.size > 0, 'empty source review');
      for (const [name, file] of files) {
        requireThat(!path.isAbsolute(name) && !path.relative(productRoot, path.resolve(productRoot, name)).startsWith('..'), 'reviewed file escapes product');
        verifiedFile(file, productRoot);
      }
      requireThat(Array.isArray(review.coverage), 'source review coverage missing');
      reviews.push({ ...review, ref: ref.path });
    } catch (error) { reviewErrors.push(`${ref.path}: ${error.message}`); }
  }
  for (const [id, lens] of [['RC-001-NO-STALE-REASONS', 'reason_strings'], ['RC-002-NO-DEAD-CODE', 'public_exports'], ['RC-003-CFG-GATED-TESTS', 'test_gate_intent']]) {
    const matching = reviews.filter(review => review.coverage.includes(lens) && (lens !== 'reason_strings' || review.lifecycle_sha256 === sha(canonicalJson(contract.lifecycle || {}))));
    const covered = new Set(matching.flatMap(review => review.reviewed_files.map(file => file.path)));
    const missingFiles = (contract.owned_files || []).filter(file => !covered.has(file));
    const ok = matching.length > 0 && !reviewErrors.length && Boolean(binding) && !missingFiles.length;
    set(id, ok, ok ? `Independent source-hashed review covers ${lens}; HEAD ${binding.head}, tree ${binding.tree}.` : `Missing/invalid independent ${lens} evidence: ${[...reviewErrors, ...missingFiles.map(file => `unreviewed owned file ${file}`)].join('; ') || 'no matching review'}`, matching.map(review => review.ref));
  }
  const commands = [], commandErrors = [];
  if (binding && context) for (const row of input?.command_results || []) {
    try { commands.push(verifyCommand(row, context, binding)); }
    catch (error) { commandErrors.push(`${row.receipt?.path || '<missing>'}: ${error.message}`); }
  }
  const declaredTargets = contract.scope?.proof_targets || [];
  const isCommand = value => /^\s*(?:cargo\s|git diff --check\s*$)/.test(value) || /\bcargo\s/.test(value) && !/^PT-[\w-]+:/.test(value);
  const proseTargets = declaredTargets.filter(value => !isCommand(value));
  const required = [...new Set([...declaredTargets.filter(isCommand), ...(contract.scope?.proof_commands || []), ...(contract.handoff?.proof_commands || []), ...(contract.proof_commands || [])])];
  const targetErrors = [];
  if (proseTargets.length) {
    try { verifyProseTargets(proseTargets, input?.proof_targets, reviews, commands, context, binding); }
    catch (error) { targetErrors.push(error.message); }
  }
  const missing = required.filter(command => {
    try {
      const targets = String(command).trim() === 'git diff --check' ? [] : commandShape(tokens(command).slice(1)).targets;
      return !(targets.length > 1 ? targets : [null]).every(target => commands.some(result => covers(command, result.args, result.passedTests, target)));
    } catch { return true; }
  });
  const realResource = reviews.some(review => (review.real_resource_tests || []).some(test => {
    const source = review.reviewed_files.find(file => file.path === test.source_path);
    if (!source || typeof test.test_name !== 'string' || !test.test_name) return false;
    return commands.some(result => commandShape(result.args).targets.includes(test.target) && result.passedTests.has(test.test_name));
  }));
  const proofOK = required.length + proseTargets.length > 0 && !missing.length && !targetErrors.length && !commandErrors.length && realResource && !reviewErrors.length;
  set('RC-005-PROOF-COMMANDS', proofOK, proofOK ? `Verified ${required.length} declared commands and ${proseTargets.length} mapped proof targets against runtime/source evidence at ${binding.head}.` : `Missing/failed proof: ${[...commandErrors, ...missing, ...targetErrors, ...(!realResource ? ['no executed real-resource test identified by independent source review'] : [])].join('; ')}`, commands.map(result => result.ref));
  const cross = commands.filter(result => commandShape(result.args).subcommand === 'check' && result.platform && nativePlatform && result.platform !== nativePlatform);
  const ciErrors = [];
  if (binding) for (const ref of input?.ci_builds || []) {
    try { cross.push(verifyGithubBuild(ref, binding, nativePlatform)); }
    catch (error) { ciErrors.push(error.message); }
  }
  set('RC-004-CROSS-PLATFORM-CI', cross.length > 0 && !commandErrors.length && !ciErrors.length, cross.length && !ciErrors.length ? `Verified non-target build: ${cross.map(result => result.platform).join(', ')}; native=${nativePlatform}.` : `Missing source-bound successful non-target build or CI job. ${ciErrors.join('; ')}`, cross.map(result => result.ref));
  if (!input) errors.push('handoff.ready_evidence_refs is missing; no questionnaire answers are accepted');
  if (!binding) for (const [id, item] of items) if (id !== 'RC-006-IMPLEMENTER-NOT-SELF-CERTIFYING') item.explanation = errors.join('; ');
  return { binding, items, errors, retained_artifacts: context?.relocation, runtime_bindings: commands.map(result => ({ receipt: result.ref, ...result.temporalBinding, historical_runtime_dependencies: result.runtimeDependencies })) };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { registerFailCaptureHook, failWithMemory } = await import('../../../roles_shared/scripts/lib/fail-capture-lib.mjs');
  registerFailCaptureHook('mechanical-ready-evidence.mjs', { role: 'KERNEL_BUILDER' });
  try {
    requireThat(process.argv[2] === '--source-binding' && process.argv[3], 'only --source-binding <product> is supported');
    process.stdout.write(`${JSON.stringify(readSourceBinding(process.argv[3]))}\n`);
  } catch (error) {
    failWithMemory('mechanical-ready-evidence.mjs', error.message, { role: 'KERNEL_BUILDER' });
  }
}
