#!/usr/bin/env node
/**
 * COR-701 SHA helper
 * - Prints deterministic Pre/Post SHA1 values for a target file.
 * - Prefers Git blobs (HEAD/INDEX) and normalizes LF/CRLF variants for human convenience.
 */

import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';

const targetArg = process.argv[2];
if (!targetArg) {
  console.error('Usage: node scripts/validation/cor701-sha.mjs <path/to/file>');
  process.exit(1);
}

const normalizeLf = (text) => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
const normalizeCrlf = (text) => normalizeLf(text).replace(/\n/g, '\r\n');
const sha1Hex = (bufOrString) => crypto.createHash('sha1').update(bufOrString).digest('hex');
const isLikelyText = (buf) => !buf.includes(0);

const shaVariantsForText = (text) => {
  const lf = normalizeLf(text);
  return {
    lf: sha1Hex(lf),
    crlf: sha1Hex(normalizeCrlf(lf)),
  };
};

const shaVariantsForBlob = (buf) => {
  const lf = sha1Hex(buf);
  if (!isLikelyText(buf)) return { lf, crlf: lf };
  const { crlf } = shaVariantsForText(buf.toString('utf8'));
  return { lf, crlf };
};

const shaVariantsForWorktree = (filePath) => {
  const buf = fs.readFileSync(filePath);
  const raw = sha1Hex(buf);
  if (!isLikelyText(buf)) return { raw, lf: raw, crlf: raw };
  const { lf, crlf } = shaVariantsForText(buf.toString('utf8'));
  return { raw, lf, crlf };
};

const gitPath = path.normalize(targetArg).replace(/\\/g, '/');

const tryGitBuffer = (command) => {
  try {
    // Avoid noisy git stderr (e.g., "fatal: path ... not in HEAD") for expected lookups.
    return execSync(command, { stdio: ['ignore', 'pipe', 'pipe'] });
  } catch {
    return null;
  }
};

const tryGitTrim = (command) => {
  try {
    // Avoid noisy git stderr (e.g., "fatal: path ... not in HEAD") for expected lookups.
    return execSync(command, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  } catch {
    return '';
  }
};

const headBuf = tryGitBuffer(`git show HEAD:${gitPath}`);
const indexBuf = tryGitBuffer(`git show :${gitPath}`);
const stagedNameOnly = tryGitTrim(`git diff --name-only --cached -- "${gitPath}"`);
const isStaged = stagedNameOnly.split('\n').map((l) => l.trim()).filter(Boolean).includes(gitPath);

const worktreePath = path.normalize(targetArg);
const hasWorktree = fs.existsSync(worktreePath);
const worktree = hasWorktree ? shaVariantsForWorktree(worktreePath) : null;

const head = headBuf ? shaVariantsForBlob(headBuf) : null;
const index = indexBuf ? shaVariantsForBlob(indexBuf) : null;

const recommendedPre = head?.lf || '<untracked>';
const recommendedPost = isStaged ? (index?.lf || '<untracked>') : (worktree?.lf || '<missing>');

console.log(`\nCOR-701 SHA helper: ${gitPath}\n`);

console.log('SHA variants:');
if (head) {
  console.log(`- HEAD (LF blob):   ${head.lf}`);
  if (head.crlf !== head.lf) console.log(`- HEAD (CRLF alt):  ${head.crlf}`);
} else {
  console.log('- HEAD:             <untracked/new file>');
}

if (index) {
  console.log(`- INDEX (LF blob):  ${index.lf}${isStaged ? '' : ' (NOTE: file not staged; INDEX may not include your changes)'}`);
  if (index.crlf !== index.lf) console.log(`- INDEX (CRLF alt): ${index.crlf}`);
} else {
  console.log('- INDEX:            <untracked/new file>');
}

if (worktree) {
  console.log(`- WORKTREE (raw):   ${worktree.raw}`);
  if (worktree.lf !== worktree.raw) console.log(`- WORKTREE (LF):    ${worktree.lf}`);
  if (worktree.crlf !== worktree.raw && worktree.crlf !== worktree.lf) console.log(`- WORKTREE (CRLF):  ${worktree.crlf}`);
} else {
  console.log('- WORKTREE:         <missing on disk>');
}

console.log('\nRecommended for task packet manifest:');
console.log(`- **Pre-SHA1**: \`${recommendedPre}\``);
console.log(`- **Post-SHA1**: \`${recommendedPost}\``);

if (!isStaged) {
  console.log('\nNOTE: For deterministic Post-SHA1, stage the file before copying Post-SHA1 (so it comes from INDEX).');
}

