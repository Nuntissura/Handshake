#!/usr/bin/env node
/**
 * Helper: skeleton checkpoint marker commit for a WP [CX-GATE-001, CX-625]
 *
 * [CX-212D] Coders do not commit .GOV/ files on feature branches.
 * The skeleton content lives in the work packet (governance kernel, via junction).
 * This script creates an empty commit marker on the feature branch as a phase gate
 * signal so post-work can verify the interface-first checkpoint happened.
 *
 * Creates: "docs: skeleton checkpoint [WP-{ID}]" (allow-empty)
 */

import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { GOV_ROOT_REPO_REL } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';

const wpId = process.argv[2];
if (!wpId) {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles/coder/checks/coder-skeleton-checkpoint.mjs WP-{ID}`);
  process.exit(1);
}

const packetRel = path.join(GOV_ROOT_REPO_REL, 'task_packets', `${wpId}.md`);
if (!fs.existsSync(packetRel)) {
  console.error(`FAIL: Work packet not found: ${packetRel}`);
  process.exit(1);
}

// Verify ## SKELETON section exists and has content.
const packetContent = fs.readFileSync(packetRel, 'utf8');
const skeletonMatch = packetContent.match(/^##\s+SKELETON\s*$/mi);
if (!skeletonMatch) {
  console.error(`FAIL: Work packet missing ## SKELETON section. Fill it before creating the checkpoint.`);
  process.exit(1);
}

function git(args) {
  return execSync(`git ${args}`, { encoding: 'utf8' });
}

const checkpointSubject = `docs: skeleton checkpoint [${wpId}]`;

function latestCheckpointSha() {
  const log = git('log -n 50 --format=%H%x09%s');
  for (const line of log.split(/\r?\n/)) {
    if (!line) continue;
    const [sha, subject] = line.split('\t');
    if ((subject || '').trim() === checkpointSubject) {
      return (sha || '').trim();
    }
  }
  return '';
}

const currentBranch = git('rev-parse --abbrev-ref HEAD').trim();

if (currentBranch === 'main' || currentBranch.startsWith('role_') || currentBranch.startsWith('user_')) {
  console.error(`FAIL: Refusing to create a WP skeleton commit on branch "${currentBranch}".`);
  console.error(`Expected a WP feature branch for ${wpId}.`);
  process.exit(1);
}

const checkpointSha = latestCheckpointSha();
if (checkpointSha) {
  console.log(`PASS: Skeleton checkpoint already exists at ${checkpointSha}. Nothing new to commit.`);
  process.exit(0);
}

// Create empty commit marker — no .GOV/ files staged [CX-212D].
execSync(`git commit --allow-empty -m "${checkpointSubject}"`, { stdio: 'inherit' });
console.log(`Skeleton checkpoint marker created. The ## SKELETON content lives in the governance kernel.`);
