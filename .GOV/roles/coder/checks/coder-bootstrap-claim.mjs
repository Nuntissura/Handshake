#!/usr/bin/env node
/**
 * Helper: docs-only bootstrap claim checkpoint for a WP [CX-217, CX-212D]
 *
 * The packet claim fields live in the governance kernel through the .GOV junction,
 * so the WP branch records a phase-boundary marker commit instead of committing .GOV.
 *
 * Creates: "docs: bootstrap claim [WP-{ID}]" (allow-empty)
 */

import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { registerFailCaptureHook } from '../../../roles_shared/scripts/lib/fail-capture-lib.mjs';
import { GOV_ROOT_REPO_REL, repoPathAbs, resolveWorkPacketPath } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';

registerFailCaptureHook('coder-bootstrap-claim.mjs', { role: 'CODER' });

const wpId = process.argv[2];
if (!wpId) {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles/coder/checks/coder-bootstrap-claim.mjs WP-{ID}`);
  process.exit(1);
}

const resolved = resolveWorkPacketPath(wpId);
const packetRel = resolved?.packetPath || path.join(GOV_ROOT_REPO_REL, 'task_packets', `${wpId}.md`);
if (!fs.existsSync(repoPathAbs(packetRel))) {
  console.error(`FAIL: Work packet not found: ${packetRel}`);
  process.exit(1);
}

function git(args) {
  return execSync(`git ${args}`, { encoding: 'utf8' });
}

const subject = `docs: bootstrap claim [${wpId}]`;

function latestBootstrapSha() {
  const log = git('log -n 50 --format=%H%x09%s');
  for (const line of log.split(/\r?\n/)) {
    if (!line) continue;
    const [sha, message] = line.split('\t');
    if ((message || '').trim() === subject) {
      return (sha || '').trim();
    }
  }
  return '';
}

const currentBranch = git('rev-parse --abbrev-ref HEAD').trim();
if (currentBranch === 'main' || currentBranch.startsWith('role_') || currentBranch.startsWith('user_')) {
  console.error(`FAIL: Refusing to create a WP bootstrap claim on branch "${currentBranch}".`);
  console.error(`Expected a WP feature branch for ${wpId}.`);
  process.exit(1);
}

const existingSha = latestBootstrapSha();
if (existingSha) {
  console.log(`PASS: Bootstrap claim checkpoint already exists at ${existingSha}. Nothing new to commit.`);
  process.exit(0);
}

execSync(`git commit --allow-empty -m "${subject}"`, { stdio: 'inherit' });
console.log('Bootstrap claim marker created. Packet claim fields remain governed through the kernel junction.');
