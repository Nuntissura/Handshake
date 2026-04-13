#!/usr/bin/env node
/**
 * repomem — Governance Conversation Memory CLI
 *
 * Conversational checkpoint system that captures what was discussed, decided,
 * and discovered across sessions. Mechanical triggers ensure checkpoints are
 * written; content comes from the model/operator.
 *
 * Quality gates:
 *   - open/close/insight/research-close/decision/abandon/concern require >=80 character content
 *   - close requires --decisions flag (non-empty)
 *   - pre/error/escalation require >=40 characters (fast capture under pressure)
 *
 * Usage:
 *   just repomem open  "<what this session is about>"  [--role ROLE] [--wp WP-ID]
 *   just repomem pre   "<about to do X because Y>"     [--wp WP-ID] [--trigger "cmd"]
 *   just repomem insight "<key realization>"            [--wp WP-ID] [--files "a,b"] [--decisions "x"]
 *   just repomem decision "<what was chosen and why>"   [--wp WP-ID] [--alternatives "rejected options"]
 *   just repomem error "<what went wrong>"              [--wp WP-ID] [--trigger "cmd"] [--files "a,b"]
 *   just repomem abandon "<what was abandoned and why>" [--wp WP-ID] [--files "a,b"]
 *   just repomem concern "<risk or issue flagged>"      [--wp WP-ID] [--files "a,b"]
 *   just repomem escalation "<what was escalated>"      [--wp WP-ID]
 *   just repomem research-close "<what was found>"      [--wp WP-ID] [--files "a,b"] [--decisions "x"]
 *   just repomem close "<session summary>"              --decisions "key decisions made"
 *   just repomem log                                    [--session last] [--week] [--search "q"] [--wp WP-ID] [--limit N]
 *   just repomem gate                                   (exit 1 if no SESSION_OPEN)
 *   just repomem context "<why>" --trigger "just cmd"   (piggybacked context for mutation commands)
 */

import {
  openGovernanceMemoryDb,
  closeDb,
  addConversationCheckpoint,
  getConversationLog,
  getLastSession,
  getCurrentSession,
  writeSessionMarker,
  clearSessionMarker,
  generateSessionId,
  checkSessionGate,
  VALID_CHECKPOINT_TYPES,
} from "./governance-memory-lib.mjs";

// ---------------------------------------------------------------------------
// Argument parsing — consumes all remaining text after a --flag as its value
// until the next --flag, to work around justfile *FLAGS quoting limitations
// ---------------------------------------------------------------------------

function parseFlags(args) {
  const flags = {};
  const positional = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i].startsWith("--")) {
      const key = args[i].slice(2);
      // Collect all following non-flag tokens as the value
      const valueParts = [];
      while (i + 1 < args.length && !args[i + 1].startsWith("--")) {
        i++;
        valueParts.push(args[i]);
      }
      flags[key] = valueParts.length > 0 ? valueParts.join(" ") : true;
    } else {
      positional.push(args[i]);
    }
  }
  return { flags, positional };
}

// ---------------------------------------------------------------------------
// Content quality gates
// ---------------------------------------------------------------------------

const MIN_CONTENT_LENGTH = 80;
const MIN_PRE_CONTENT_LENGTH = 40;

function enforceContentLength(content, command, minLength = MIN_CONTENT_LENGTH) {
  if (!content || content.length < minLength) {
    console.error(`REPOMEM_QUALITY_GATE_FAIL: "${command}" content must be at least ${minLength} characters.`);
    console.error(`  Received ${content ? content.length : 0} characters: "${(content || "").slice(0, 60)}"`);
    console.error(`  Write something substantive — this becomes cross-session memory.`);
    process.exit(1);
  }
}

function enforceDecisions(decisions, command) {
  if (!decisions || decisions.trim().length === 0) {
    console.error(`REPOMEM_QUALITY_GATE_FAIL: "${command}" requires --decisions flag with non-empty content.`);
    console.error(`  What decisions were made? What was concluded?`);
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------

function usage() {
  console.error(`Usage: repomem <open|pre|insight|decision|error|abandon|concern|escalation|research-close|close|log|gate|context>

  open  "<intent>"           Start session checkpoint (>=80 chars, required before work)
  pre   "<about to do>"      Pre-task checkpoint (>=40 chars, before mutation commands)
  insight "<realization>"    Insight checkpoint (>=80 chars, key discovery)
  decision "<choice+why>"    Decision checkpoint (>=80 chars, deliberate choice between alternatives)
  error "<what broke>"       Error checkpoint (>=40 chars, what went wrong)
  abandon "<path+why>"       Abandon checkpoint (>=80 chars, approach/path abandoned)
  concern "<risk flagged>"   Concern checkpoint (>=80 chars, risk or issue identified)
  escalation "<what+to>"     Escalation checkpoint (>=40 chars, escalated to operator/role)
  research-close "<found>"   Research conclusion checkpoint (>=80 chars)
  close "<summary>"          Session end checkpoint (>=80 chars, --decisions required)
  log                        Show conversation history
  gate                       Check if session is open (exit 1 if not)
  context "<why>"            Piggybacked context for mutation commands (>=40 chars)`);
  process.exit(1);
}

const [command, ...rawArgs] = process.argv.slice(2);
const { flags, positional } = parseFlags(rawArgs);

if (!command) usage();

// ---------------------------------------------------------------------------
// gate — lightweight check, no DB open needed
// ---------------------------------------------------------------------------
if (command === "gate") {
  const result = checkSessionGate();
  if (!result.open) {
    console.error(result.message);
    process.exit(1);
  }
  console.log(`REPOMEM_GATE_OK: session=${result.session.session_id} role=${result.session.role} opened=${result.session.opened_at}`);
  process.exit(0);
}

// All other commands need the DB
const { db } = openGovernanceMemoryDb();

try {
  if (command === "open") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem open "<what this session is about>" [--role ROLE] [--wp WP-ID]');
      process.exit(1);
    }
    enforceContentLength(content, "open");
    const role = flags.role || "ORCHESTRATOR";
    const wpId = flags.wp || "";

    // Close any stale session first
    const existingSession = getCurrentSession();
    if (existingSession) {
      addConversationCheckpoint(db, {
        sessionId: existingSession.session_id,
        role: existingSession.role,
        checkpointType: "SESSION_CLOSE",
        topic: "(auto-closed by new session open)",
        content: "Previous session was not explicitly closed. Auto-closed when new session started.",
        decisions: "(none — auto-closed)",
      });
      console.log(`[repomem] Auto-closed stale session ${existingSession.session_id}`);
    }

    const sessionId = generateSessionId(role);
    const now = new Date().toISOString();

    writeSessionMarker({
      session_id: sessionId,
      role,
      opened_at: now,
      topic: content.slice(0, 200),
    });

    const id = addConversationCheckpoint(db, {
      sessionId,
      role,
      checkpointType: "SESSION_OPEN",
      wpId,
      topic: content.slice(0, 120),
      content,
    });

    console.log(`REPOMEM_SESSION_OPEN`);
    console.log(`  session_id: ${sessionId}`);
    console.log(`  role: ${role}`);
    console.log(`  checkpoint: #${id}`);
    console.log(`  topic: ${content.slice(0, 120)}`);

    // Show last session context
    const lastSession = getLastSession(db);
    if (lastSession.length > 0) {
      const sessionDate = lastSession[0].timestamp_utc?.slice(0, 10) || "unknown";
      console.log(`\nPRIOR_SESSION (${sessionDate}):`);
      for (const entry of lastSession.slice(-6)) {
        console.log(`  [${entry.checkpoint_type}] ${entry.topic.slice(0, 100)}`);
        if (entry.decisions) console.log(`    decisions: ${entry.decisions.slice(0, 150)}`);
      }
    }

  } else if (command === "pre") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem pre "<about to do X because Y>" [--wp WP-ID] [--trigger "just cmd"]');
      process.exit(1);
    }
    enforceContentLength(content, "pre", MIN_PRE_CONTENT_LENGTH);
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "PRE_TASK",
      triggerRef: flags.trigger || "",
      wpId: flags.wp || "",
      topic: content.slice(0, 120),
      content,
      filesReferenced: flags.files || "",
    });
    console.log(`[repomem] PRE_TASK #${id}: ${content.slice(0, 100)}`);

  } else if (command === "insight") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem insight "<key realization>" [--wp WP-ID] [--files "a,b"] [--decisions "x"]');
      process.exit(1);
    }
    enforceContentLength(content, "insight");
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "INSIGHT",
      wpId: flags.wp || "",
      topic: content.slice(0, 120),
      content,
      filesReferenced: flags.files || "",
      decisions: flags.decisions || "",
    });
    console.log(`[repomem] INSIGHT #${id}: ${content.slice(0, 100)}`);

  } else if (command === "decision") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem decision "<what was chosen and why>" [--wp WP-ID] [--alternatives "rejected options"]');
      process.exit(1);
    }
    enforceContentLength(content, "decision");
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "DECISION",
      wpId: flags.wp || "",
      topic: content.slice(0, 120),
      content,
      filesReferenced: flags.files || "",
      decisions: flags.alternatives ? `Alternatives rejected: ${flags.alternatives}` : "",
    });
    console.log(`[repomem] DECISION #${id}: ${content.slice(0, 100)}`);

  } else if (command === "error") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem error "<what went wrong>" [--wp WP-ID] [--trigger "cmd"] [--files "a,b"]');
      process.exit(1);
    }
    enforceContentLength(content, "error", MIN_PRE_CONTENT_LENGTH);
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "ERROR",
      triggerRef: flags.trigger || "",
      wpId: flags.wp || "",
      topic: `[error] ${content.slice(0, 112)}`,
      content,
      filesReferenced: flags.files || "",
    });
    console.log(`[repomem] ERROR #${id}: ${content.slice(0, 100)}`);

  } else if (command === "abandon") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem abandon "<what was abandoned and why>" [--wp WP-ID] [--files "a,b"]');
      process.exit(1);
    }
    enforceContentLength(content, "abandon");
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "ABANDON",
      wpId: flags.wp || "",
      topic: `[abandon] ${content.slice(0, 110)}`,
      content,
      filesReferenced: flags.files || "",
    });
    console.log(`[repomem] ABANDON #${id}: ${content.slice(0, 100)}`);

  } else if (command === "concern") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem concern "<risk or issue flagged>" [--wp WP-ID] [--files "a,b"]');
      process.exit(1);
    }
    enforceContentLength(content, "concern");
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "CONCERN",
      wpId: flags.wp || "",
      topic: `[concern] ${content.slice(0, 109)}`,
      content,
      filesReferenced: flags.files || "",
    });
    console.log(`[repomem] CONCERN #${id}: ${content.slice(0, 100)}`);

  } else if (command === "escalation") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem escalation "<what was escalated>" [--wp WP-ID]');
      process.exit(1);
    }
    enforceContentLength(content, "escalation", MIN_PRE_CONTENT_LENGTH);
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "ESCALATION",
      wpId: flags.wp || "",
      topic: `[escalation] ${content.slice(0, 107)}`,
      content,
    });
    console.log(`[repomem] ESCALATION #${id}: ${content.slice(0, 100)}`);

  } else if (command === "research-close") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem research-close "<what was found>" [--wp WP-ID] [--files "a,b"] [--decisions "x"]');
      process.exit(1);
    }
    enforceContentLength(content, "research-close");
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "RESEARCH_CLOSE",
      wpId: flags.wp || "",
      topic: content.slice(0, 120),
      content,
      filesReferenced: flags.files || "",
      decisions: flags.decisions || "",
    });
    console.log(`[repomem] RESEARCH_CLOSE #${id}: ${content.slice(0, 100)}`);

  } else if (command === "close") {
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem close "<session summary>" --decisions "key decisions made"');
      process.exit(1);
    }
    enforceContentLength(content, "close");
    enforceDecisions(flags.decisions, "close");
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session to close.");
      process.exit(1);
    }

    // Validate: session should have at least the SESSION_OPEN checkpoint
    const priorCheckpoints = db.prepare(
      "SELECT checkpoint_type, topic FROM conversation_log WHERE session_id = ? AND checkpoint_type != 'SESSION_CLOSE' ORDER BY timestamp_utc ASC"
    ).all(session.session_id);

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "SESSION_CLOSE",
      wpId: flags.wp || "",
      topic: content.slice(0, 120),
      content,
      decisions: flags.decisions || "",
    });

    console.log(`REPOMEM_SESSION_CLOSE`);
    console.log(`  session_id: ${session.session_id}`);
    console.log(`  checkpoints: ${priorCheckpoints.length + 1}`);
    for (const e of priorCheckpoints) {
      console.log(`  [${e.checkpoint_type}] ${e.topic.slice(0, 100)}`);
    }
    console.log(`  [SESSION_CLOSE] ${content.slice(0, 100)}`);
    console.log(`  decisions: ${(flags.decisions || "").slice(0, 200)}`);

    if (priorCheckpoints.length <= 1) {
      console.log(`\n  WARNING: Only ${priorCheckpoints.length} checkpoint(s) before close. Consider writing more insights/pre-task notes during sessions.`);
    }

    clearSessionMarker();

  } else if (command === "context") {
    // Piggybacked context capture — for mutation commands that require --context
    const [content] = positional;
    if (!content) {
      console.error('Usage: repomem context "<why this action>" --trigger "just some-cmd"');
      process.exit(1);
    }
    enforceContentLength(content, "context", MIN_PRE_CONTENT_LENGTH);
    const session = getCurrentSession();
    if (!session) {
      console.error("REPOMEM_ERROR: No active session. Run `just repomem open` first.");
      process.exit(1);
    }

    const id = addConversationCheckpoint(db, {
      sessionId: session.session_id,
      role: session.role,
      checkpointType: "PRE_TASK",
      triggerRef: flags.trigger || "",
      wpId: flags.wp || "",
      topic: `[ctx] ${(content).slice(0, 110)}`,
      content,
      filesReferenced: flags.files || "",
    });
    console.log(`[repomem] CONTEXT #${id}: ${content.slice(0, 100)}`);

  } else if (command === "log") {
    const limit = Number(flags.limit) || 20;

    if (flags.session === "last") {
      const entries = getLastSession(db);
      if (entries.length === 0) {
        console.log("[repomem] No prior session found.");
      } else {
        console.log(`REPOMEM_LOG (last session: ${entries[0].session_id}):\n`);
        for (const e of entries) {
          const time = e.timestamp_utc?.slice(11, 16) || "?";
          console.log(`  ${time}Z [${e.checkpoint_type}]${e.wp_id ? ` (${e.wp_id})` : ""} ${e.topic}`);
          if (e.content && e.content !== e.topic) {
            const preview = e.content.slice(0, 200);
            if (preview.length > e.topic.length + 20) console.log(`         ${preview}`);
          }
          if (e.decisions) console.log(`         decisions: ${e.decisions.slice(0, 150)}`);
          if (e.files_referenced) console.log(`         files: ${e.files_referenced.slice(0, 150)}`);
        }
      }
    } else if (flags.session === "current") {
      const session = getCurrentSession();
      if (!session) {
        console.log("[repomem] No active session.");
      } else {
        const entries = db.prepare(
          "SELECT * FROM conversation_log WHERE session_id = ? ORDER BY timestamp_utc ASC"
        ).all(session.session_id);
        console.log(`REPOMEM_LOG (current session: ${session.session_id}):\n`);
        for (const e of entries) {
          const time = e.timestamp_utc?.slice(11, 16) || "?";
          console.log(`  ${time}Z [${e.checkpoint_type}]${e.wp_id ? ` (${e.wp_id})` : ""} ${e.topic}`);
          if (e.decisions) console.log(`         decisions: ${e.decisions.slice(0, 150)}`);
        }
      }
    } else {
      // Default: recent entries with optional filters
      let sinceDate = "";
      if (flags.week) {
        sinceDate = new Date(Date.now() - 7 * 86400000).toISOString();
      } else if (flags.month) {
        sinceDate = new Date(Date.now() - 30 * 86400000).toISOString();
      }

      const entries = getConversationLog(db, {
        lastN: limit,
        sinceDate,
        search: flags.search || "",
        wpId: flags.wp || "",
      });

      if (entries.length === 0) {
        console.log("[repomem] No conversation entries found.");
      } else {
        console.log(`REPOMEM_LOG (${entries.length} entries):\n`);
        let lastDate = "";
        for (const e of entries) {
          const date = e.timestamp_utc?.slice(0, 10) || "?";
          const time = e.timestamp_utc?.slice(11, 16) || "?";
          if (date !== lastDate) {
            console.log(`--- ${date} ---`);
            lastDate = date;
          }
          console.log(`  ${time}Z [${e.checkpoint_type}] ${e.role || "?"} ${e.topic}`);
          if (e.content && e.content !== e.topic && e.content.length > e.topic.length + 20) {
            console.log(`         ${e.content.slice(0, 200)}`);
          }
          if (e.decisions) console.log(`         decisions: ${e.decisions.slice(0, 150)}`);
          if (e.files_referenced) console.log(`         files: ${e.files_referenced}`);
        }
      }
    }

  } else {
    console.error(`Unknown command: ${command}`);
    usage();
  }
} finally {
  closeDb(db);
}
