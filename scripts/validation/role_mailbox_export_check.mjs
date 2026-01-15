import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const EXPORT_ROOT = 'docs/ROLE_MAILBOX/';
const EXPORT_DIR = path.join(process.cwd(), 'docs', 'ROLE_MAILBOX');
const INDEX_PATH = path.join(EXPORT_DIR, 'index.json');
const MANIFEST_PATH = path.join(EXPORT_DIR, 'export_manifest.json');

const FORBIDDEN_FIELDS = new Set(['body', 'body_text', 'raw_body']);
const GOVERNANCE_MODES = new Set(['gov_strict', 'gov_standard', 'gov_light']);
const CRITICAL_MESSAGE_TYPES = new Set([
    'scope_change_approval',
    'waiver_approval',
    'validation_finding'
]);
const MESSAGE_TYPES = new Set([
    'clarification_request',
    'clarification_response',
    'scope_risk',
    'scope_change_proposal',
    'scope_change_approval',
    'waiver_proposal',
    'waiver_approval',
    'validation_finding',
    'handoff',
    'blocker',
    'tooling_request',
    'tooling_result',
    'fyi'
]);

function sha256Hex(buf) {
    return crypto.createHash('sha256').update(buf).digest('hex');
}

function isLowerHexSha256(value) {
    return typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
}

function isSafeId(value, maxLen = 128) {
    return typeof value === 'string' && value.length > 0 && value.length <= maxLen && /^[A-Za-z0-9_-]+$/.test(value);
}

function isSafeRoleId(value) {
    if (typeof value !== 'string' || value.trim().length === 0) return false;
    if (['operator', 'orchestrator', 'coder', 'validator'].includes(value)) return true;
    if (!value.startsWith('advisory:')) return false;
    return isSafeId(value.slice('advisory:'.length), 128);
}

function isRfc3339(value) {
    return typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/.test(value);
}

function boundedSingleLine(value, maxLen = 160) {
    return typeof value === 'string' && value.length > 0 && value.length <= maxLen && !value.includes('\n') && !value.includes('\r');
}

function escapeStringAscii(value) {
    let out = '"';
    for (const ch of value) {
        const codePoint = ch.codePointAt(0);
        switch (ch) {
            case '"':
                out += '\\"';
                continue;
            case '\\':
                out += '\\\\';
                continue;
            case '\b':
                out += '\\b';
                continue;
            case '\f':
                out += '\\f';
                continue;
            case '\n':
                out += '\\n';
                continue;
            case '\r':
                out += '\\r';
                continue;
            case '\t':
                out += '\\t';
                continue;
        }

        if (codePoint < 0x20) {
            out += `\\u${codePoint.toString(16).toUpperCase().padStart(4, '0')}`;
            continue;
        }
        if (codePoint <= 0x7f) {
            out += ch;
            continue;
        }
        if (codePoint <= 0xffff) {
            out += `\\u${codePoint.toString(16).toUpperCase().padStart(4, '0')}`;
            continue;
        }

        const adjusted = codePoint - 0x10000;
        const high = 0xd800 + ((adjusted >> 10) & 0x3ff);
        const low = 0xdc00 + (adjusted & 0x3ff);
        out += `\\u${high.toString(16).toUpperCase().padStart(4, '0')}\\u${low.toString(16).toUpperCase().padStart(4, '0')}`;
    }
    out += '"';
    return out;
}

function stableStringify(value) {
    if (value === null) return 'null';
    if (typeof value === 'boolean') return value ? 'true' : 'false';
    if (typeof value === 'number') return String(value);
    if (typeof value === 'string') return escapeStringAscii(value);
    if (Array.isArray(value)) {
        return `[${value.map(stableStringify).join(',')}]`;
    }
    if (typeof value === 'object') {
        const keys = Object.keys(value).sort();
        const inner = keys.map(k => `${escapeStringAscii(k)}:${stableStringify(value[k])}`).join(',');
        return `{${inner}}`;
    }
    throw new Error(`unsupported JSON type: ${typeof value}`);
}

function deepForbiddenScan(value, pathLabel, errors) {
    if (value === null) return;
    if (Array.isArray(value)) {
        value.forEach((v, idx) => deepForbiddenScan(v, `${pathLabel}[${idx}]`, errors));
        return;
    }
    if (typeof value === 'object') {
        for (const [k, v] of Object.entries(value)) {
            if (FORBIDDEN_FIELDS.has(k)) {
                errors.push(`forbidden field "${k}" found at ${pathLabel}`);
            }
            deepForbiddenScan(v, `${pathLabel}.${k}`, errors);
        }
    }
}

function assertCanonicalFile(label, bytes, parsed, errors) {
    if (bytes.includes(13)) {
        errors.push(`${label}: contains CR (must use \\n newlines only)`);
    }
    if (bytes.length > 0 && bytes[bytes.length - 1] !== 10) {
        errors.push(`${label}: must end with \\n`);
    }
    for (let i = 0; i < bytes.length; i++) {
        if (bytes[i] > 127) {
            errors.push(`${label}: contains non-ASCII byte (non-ASCII must be \\\\uXXXX escaped)`);
            break;
        }
    }

    const canonical = `${stableStringify(parsed)}\n`;
    const actual = bytes.toString('utf8');
    if (actual !== canonical) {
        errors.push(`${label}: not canonical JSON (stable key order + \\\\uXXXX escaping + no whitespace)`);
    }
}

function assertObject(value, label, errors) {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        errors.push(`${label}: must be an object`);
        return false;
    }
    return true;
}

function assertExactKeys(value, allowed, required, label, errors) {
    if (!assertObject(value, label, errors)) return;
    for (const k of Object.keys(value)) {
        if (!allowed.has(k)) errors.push(`${label}: unexpected key "${k}"`);
    }
    for (const k of required) {
        if (!(k in value)) errors.push(`${label}: missing required key "${k}"`);
    }
}

function loadJsonFile(filePath, label, errors) {
    if (!fs.existsSync(filePath)) {
        errors.push(`${label}: missing file ${path.relative(process.cwd(), filePath)}`);
        return null;
    }

    const bytes = fs.readFileSync(filePath);
    let parsed = null;
    try {
        parsed = JSON.parse(bytes.toString('utf8'));
    } catch (e) {
        errors.push(`${label}: invalid JSON (${e.message})`);
        return null;
    }
    return { bytes, parsed };
}

function validateIndex(index, errors) {
    const allowed = new Set(['schema_version', 'generated_at', 'threads']);
    assertExactKeys(index, allowed, ['schema_version', 'generated_at', 'threads'], 'index.json', errors);

    if (index.schema_version !== 'role_mailbox_export_v1') {
        errors.push('index.json: schema_version must equal "role_mailbox_export_v1"');
    }
    if (!isRfc3339(index.generated_at)) {
        errors.push('index.json: generated_at must be RFC3339');
    }
    if (!Array.isArray(index.threads)) {
        errors.push('index.json: threads must be an array');
        return;
    }

    let prev = null;
    for (let i = 0; i < index.threads.length; i++) {
        const t = index.threads[i];
        const label = `index.json.threads[${i}]`;

        const allowedThread = new Set([
            'thread_id',
            'created_at',
            'closed_at',
            'participants',
            'context',
            'subject_redacted',
            'subject_sha256',
            'message_count',
            'thread_file'
        ]);
        const requiredThread = [
            'thread_id',
            'created_at',
            'participants',
            'context',
            'subject_redacted',
            'subject_sha256',
            'message_count',
            'thread_file'
        ];
        assertExactKeys(t, allowedThread, requiredThread, label, errors);

        if (!isSafeId(t.thread_id)) errors.push(`${label}: thread_id must be a safe id`);
        if (!isRfc3339(t.created_at)) errors.push(`${label}: created_at must be RFC3339`);
        if (t.closed_at !== undefined && t.closed_at !== null && !isRfc3339(t.closed_at)) {
            errors.push(`${label}: closed_at must be RFC3339|null when present`);
        }
        if (!Array.isArray(t.participants) || t.participants.length === 0) {
            errors.push(`${label}: participants must be a non-empty array`);
        } else {
            for (let p = 0; p < t.participants.length; p++) {
                if (!isSafeRoleId(t.participants[p])) {
                    errors.push(`${label}: participants[${p}] invalid role id`);
                }
            }
        }
        if (!assertObject(t.context, `${label}.context`, errors)) continue;

        const allowedCtx = new Set([
            'spec_id',
            'work_packet_id',
            'task_board_id',
            'governance_mode',
            'project_id'
        ]);
        assertExactKeys(t.context, allowedCtx, ['governance_mode'], `${label}.context`, errors);
        if (!GOVERNANCE_MODES.has(t.context.governance_mode)) {
            errors.push(`${label}.context: governance_mode invalid`);
        }
        for (const k of ['spec_id', 'work_packet_id', 'task_board_id', 'project_id']) {
            if (k in t.context && t.context[k] !== null && typeof t.context[k] !== 'string') {
                errors.push(`${label}.context: ${k} must be string|null when present`);
            }
        }

        if (!boundedSingleLine(t.subject_redacted)) {
            errors.push(`${label}: subject_redacted must be single-line <=160 chars`);
        }
        if (!isLowerHexSha256(t.subject_sha256)) {
            errors.push(`${label}: subject_sha256 must be lowercase hex sha256`);
        }
        if (!Number.isInteger(t.message_count) || t.message_count < 0) {
            errors.push(`${label}: message_count must be an integer`);
        }
        const expectedFile = `threads/${t.thread_id}.jsonl`;
        if (t.thread_file !== expectedFile) {
            errors.push(`${label}: thread_file must equal "${expectedFile}"`);
        }

        const currentKey = `${t.created_at}\u0000${t.thread_id}`;
        if (prev !== null && currentKey < prev) {
            errors.push('index.json: threads must be sorted by (created_at, thread_id) asc');
        }
        prev = currentKey;
    }
}

function validateManifest(manifest, indexBytes, errors) {
    const allowed = new Set([
        'schema_version',
        'export_root',
        'generated_at',
        'index_sha256',
        'thread_files'
    ]);
    assertExactKeys(manifest, allowed, ['schema_version', 'export_root', 'generated_at', 'index_sha256', 'thread_files'], 'export_manifest.json', errors);

    if (manifest.schema_version !== 'role_mailbox_export_v1') {
        errors.push('export_manifest.json: schema_version must equal "role_mailbox_export_v1"');
    }
    if (manifest.export_root !== EXPORT_ROOT) {
        errors.push(`export_manifest.json: export_root must equal "${EXPORT_ROOT}"`);
    }
    if (!isRfc3339(manifest.generated_at)) {
        errors.push('export_manifest.json: generated_at must be RFC3339');
    }
    const actualIndexSha = sha256Hex(indexBytes);
    if (manifest.index_sha256 !== actualIndexSha) {
        errors.push(`export_manifest.json: index_sha256 mismatch (manifest=${manifest.index_sha256}, actual=${actualIndexSha})`);
    }
    if (!Array.isArray(manifest.thread_files)) {
        errors.push('export_manifest.json: thread_files must be an array');
        return [];
    }

    let prevPath = null;
    const entries = [];
    for (let i = 0; i < manifest.thread_files.length; i++) {
        const e = manifest.thread_files[i];
        const label = `export_manifest.json.thread_files[${i}]`;
        const allowedEntry = new Set(['path', 'sha256', 'message_count']);
        assertExactKeys(e, allowedEntry, ['path', 'sha256', 'message_count'], label, errors);

        if (typeof e.path !== 'string' || !e.path.startsWith('threads/')) {
            errors.push(`${label}: path must start with "threads/"`);
        }
        if (typeof e.path === 'string') {
            const base = path.basename(e.path);
            const threadId = base.endsWith('.jsonl') ? base.slice(0, -'.jsonl'.length) : '';
            if (!isSafeId(threadId)) {
                errors.push(`${label}: thread id in path must be a safe id`);
            }
            if (e.path !== `threads/${threadId}.jsonl`) {
                errors.push(`${label}: path must equal "threads/<thread_id>.jsonl"`);
            }
        }
        if (!isLowerHexSha256(e.sha256)) {
            errors.push(`${label}: sha256 must be lowercase hex sha256`);
        }
        if (!Number.isInteger(e.message_count) || e.message_count < 0) {
            errors.push(`${label}: message_count must be an integer`);
        }
        if (prevPath !== null && typeof e.path === 'string' && e.path < prevPath) {
            errors.push('export_manifest.json: thread_files must be sorted by path asc');
        }
        if (typeof e.path === 'string') prevPath = e.path;

        entries.push(e);
    }
    return entries;
}

function validateThreadFile(threadFileRel, expectedSha, expectedCount, errors) {
    const abs = path.join(EXPORT_DIR, threadFileRel);
    if (!fs.existsSync(abs)) {
        errors.push(`thread file missing: ${threadFileRel}`);
        return;
    }

    const bytes = fs.readFileSync(abs);
    if (bytes.includes(13)) {
        errors.push(`thread file ${threadFileRel}: contains CR (must use \\n newlines only)`);
    }
    for (let i = 0; i < bytes.length; i++) {
        if (bytes[i] > 127) {
            errors.push(`thread file ${threadFileRel}: contains non-ASCII byte (non-ASCII must be \\\\uXXXX escaped)`);
            break;
        }
    }
    if (bytes.length > 0 && bytes[bytes.length - 1] !== 10) {
        errors.push(`thread file ${threadFileRel}: must end with \\n when non-empty`);
    }

    const actualSha = sha256Hex(bytes);
    if (actualSha !== expectedSha) {
        errors.push(`thread file ${threadFileRel}: sha256 mismatch (manifest=${expectedSha}, actual=${actualSha})`);
    }

    const text = bytes.toString('utf8');
    const lines = text.length === 0 ? [] : text.split('\n').slice(0, -1); // drop trailing empty from final \n
    if (lines.length !== expectedCount) {
        errors.push(`thread file ${threadFileRel}: message_count mismatch (manifest=${expectedCount}, actual=${lines.length})`);
    }

    let prev = null;
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        let obj = null;
        try {
            obj = JSON.parse(line);
        } catch (e) {
            errors.push(`thread file ${threadFileRel}: line ${i} invalid JSON (${e.message})`);
            continue;
        }

        const label = `${threadFileRel}:${i}`;
        deepForbiddenScan(obj, label, errors);

        const allowed = new Set([
            'message_id',
            'thread_id',
            'created_at',
            'from_role',
            'to_roles',
            'message_type',
            'body_ref',
            'body_sha256',
            'attachments',
            'relates_to_message_id',
            'transcription_links',
            'idempotency_key'
        ]);
        const required = [
            'message_id',
            'thread_id',
            'created_at',
            'from_role',
            'to_roles',
            'message_type',
            'body_ref',
            'body_sha256',
            'attachments',
            'transcription_links',
            'idempotency_key'
        ];
        assertExactKeys(obj, allowed, required, `thread line ${label}`, errors);

        if (!isSafeId(obj.message_id)) errors.push(`thread line ${label}: message_id must be safe id`);
        if (obj.thread_id !== path.basename(threadFileRel, '.jsonl')) {
            errors.push(`thread line ${label}: thread_id must match thread file id`);
        }
        if (!isRfc3339(obj.created_at)) errors.push(`thread line ${label}: created_at must be RFC3339`);
        if (!isSafeRoleId(obj.from_role)) errors.push(`thread line ${label}: from_role invalid`);
        if (!Array.isArray(obj.to_roles) || obj.to_roles.length === 0) {
            errors.push(`thread line ${label}: to_roles must be non-empty array`);
        } else {
            obj.to_roles.forEach((r, ridx) => {
                if (!isSafeRoleId(r)) errors.push(`thread line ${label}: to_roles[${ridx}] invalid`);
            });
        }
        if (typeof obj.message_type !== 'string' || !MESSAGE_TYPES.has(obj.message_type)) {
            errors.push(`thread line ${label}: message_type invalid`);
        }
        if (typeof obj.body_ref !== 'string' || obj.body_ref.trim().length === 0) {
            errors.push(`thread line ${label}: body_ref must be non-empty string`);
        }
        if (!isLowerHexSha256(obj.body_sha256)) {
            errors.push(`thread line ${label}: body_sha256 invalid`);
        }
        if (!Array.isArray(obj.attachments)) {
            errors.push(`thread line ${label}: attachments must be array`);
        } else {
            obj.attachments.forEach((a, aidx) => {
                if (typeof a !== 'string') errors.push(`thread line ${label}: attachments[${aidx}] must be string`);
            });
        }
        if (obj.relates_to_message_id !== undefined && obj.relates_to_message_id !== null && !isSafeId(obj.relates_to_message_id)) {
            errors.push(`thread line ${label}: relates_to_message_id must be safe id|null when present`);
        }
        if (typeof obj.idempotency_key !== 'string' || obj.idempotency_key.trim().length === 0) {
            errors.push(`thread line ${label}: idempotency_key must be non-empty string`);
        }

        if (!Array.isArray(obj.transcription_links)) {
            errors.push(`thread line ${label}: transcription_links must be array`);
        } else {
            if (CRITICAL_MESSAGE_TYPES.has(obj.message_type) && obj.transcription_links.length === 0) {
                errors.push(`thread line ${label}: governance-critical message_type missing transcription_links`);
            }
            obj.transcription_links.forEach((l, lidx) => {
                const ll = obj.transcription_links[lidx];
                const llabel = `thread line ${label}: transcription_links[${lidx}]`;
                const allowedLink = new Set(['target_kind', 'target_ref', 'target_sha256', 'note_redacted', 'note_sha256']);
                assertExactKeys(ll, allowedLink, Array.from(allowedLink), llabel, errors);
                if (typeof ll.target_kind !== 'string' || ll.target_kind.trim().length === 0) errors.push(`${llabel}: target_kind invalid`);
                if (typeof ll.target_ref !== 'string' || ll.target_ref.trim().length === 0) errors.push(`${llabel}: target_ref invalid`);
                if (!isLowerHexSha256(ll.target_sha256)) errors.push(`${llabel}: target_sha256 invalid`);
                if (!boundedSingleLine(ll.note_redacted)) errors.push(`${llabel}: note_redacted must be single-line <=160 chars`);
                if (!isLowerHexSha256(ll.note_sha256)) errors.push(`${llabel}: note_sha256 invalid`);
            });
        }

        const canonicalLine = stableStringify(obj);
        if (canonicalLine !== line) {
            errors.push(`thread file ${threadFileRel}: line ${i} not canonical JSON`);
        }

        const key = `${obj.created_at}\u0000${obj.message_id}`;
        if (prev !== null && key < prev) {
            errors.push(`thread file ${threadFileRel}: messages must be sorted by (created_at, message_id) asc`);
        }
        prev = key;
    }
}

function main() {
    const errors = [];

    const index = loadJsonFile(INDEX_PATH, 'index.json', errors);
    const manifest = loadJsonFile(MANIFEST_PATH, 'export_manifest.json', errors);

    if (!index || !manifest) {
        console.error(errors.join('\n'));
        process.exit(1);
    }

    deepForbiddenScan(index.parsed, 'index.json', errors);
    deepForbiddenScan(manifest.parsed, 'export_manifest.json', errors);

    validateIndex(index.parsed, errors);

    assertCanonicalFile('index.json', index.bytes, index.parsed, errors);
    assertCanonicalFile('export_manifest.json', manifest.bytes, manifest.parsed, errors);

    if (manifest.parsed.generated_at !== index.parsed.generated_at) {
        errors.push('generated_at mismatch between index.json and export_manifest.json');
    }

    const entries = validateManifest(manifest.parsed, index.bytes, errors);
    for (const entry of entries) {
        validateThreadFile(entry.path, entry.sha256, entry.message_count, errors);
    }

    if (errors.length > 0) {
        console.error('? ROLE_MAILBOX_EXPORT_GATE FAIL');
        console.error(errors.join('\n'));
        process.exit(1);
    }

    console.log('? ROLE_MAILBOX_EXPORT_GATE PASS');
}

main();
