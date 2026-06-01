import { OperatorChat } from "./OperatorChat";
import type { SwarmRoom } from "./SwarmControlRoom";

// SessionWorkbench (WP-KERNEL-004 governance glue #3): the operator's front-end
// wrapper over the CLI backend. For the currently-selected chat session it
// composes the three REAL surfaces that already exist — it does NOT rebuild any
// of them and adds NO new IPC:
//
//   (a) <OperatorChat>      — the real operator <-> model chat (any provider:
//                             local, cloud BYOK, official CLI).
//   (b) "Show captured terminal" — focuses the surface's SINGLE shared
//                             <TerminalPanel> on THIS session's captured stdout,
//                             bound by the swarm composite instance_id (the
//                             capture session for a model session is keyed by
//                             instance_id, surfaced on TerminalSession.instanceId).
//                             Where CLI toolcalls / thinking land.
//   (c) "Open full transcript" — reveals the surface's SessionReplayPanel
//                             preselected to this session's unified timeline.
//
// Decision (single shared TerminalPanel / SessionReplayPanel, no double xterm):
// the Workbench does NOT mount its own terminal or replay panel. It bumps the
// host surface's existing off-main-window drawers via the injected callbacks, so
// there is exactly one xterm instance and one replay panel in the surface. This
// honors GLOBAL-BUILD quiet/bounded and reuses both existing drawers.
//
// Honesty: the Workbench never fabricates a terminal or a transcript. If no
// session is selected, the terminal/transcript actions are disabled with an
// explanation. If the selected session has no captured terminal, the shared
// TerminalPanel renders its own honest empty state ("No captured terminal
// sessions…"). If transcript review is not wired (onReviewSession absent), the
// transcript button is honestly disabled.

export interface SessionWorkbenchProps {
  /** The shared swarm room state (chat selection + all sessions). */
  room: SwarmRoom;
  /**
   * Focus the host surface's shared TerminalPanel on the captured terminal of
   * the session with this composite instance_id. The Workbench calls this with
   * `room.chatInstanceId` when the operator clicks "Show captured terminal".
   * Absent => the action is honestly disabled.
   */
  onShowTerminal?: (instanceId: string) => void;
  /**
   * Reveal the host surface's shared SessionReplayPanel preselected to this
   * session's unified transcript. Reuses SwarmOperatorSurface.handleReviewSession.
   * Absent => the transcript button is honestly disabled ("Session Replay not
   * wired"), mirroring the board's honest-disable pattern.
   */
  onReviewSession?: (instanceId: string) => void;
  /**
   * ROI #3 STATE RECOVERY (edit-then-resume): open the host's Spawn form PREFILLED
   * from this session's recorded spawn template, so the operator can tweak it
   * (repoint a moved artifact, change worktree) before re-spawning through the
   * existing validated spawn path. The host reads the template and surfaces an
   * HONEST notice when the session is not resumable (no template). Absent => the
   * Resume button is honestly disabled ("Resume not wired").
   */
  onResumeSession?: (instanceId: string) => void;
}

export function SessionWorkbench({
  room,
  onShowTerminal,
  onReviewSession,
  onResumeSession,
}: SessionWorkbenchProps) {
  const selected = room.chatInstanceId;
  const hasSelection = selected !== null;

  return (
    <section
      className="session-workbench"
      data-testid="session-workbench"
      data-stable-id="session-workbench"
      data-selected-instance={selected ?? ""}
      aria-label="Session workbench"
    >
      {/* (a) Chat — accepts ALL spawned sessions (local, cloud, CLI). */}
      <OperatorChat
        selectedInstanceId={room.chatInstanceId}
        sessions={room.allSessions}
        onSelectInstance={room.setChatInstanceId}
      />

      {/* (b) + (c) the two off-main-window affordances for THIS session. They act
          on the surface's single shared TerminalPanel / SessionReplayPanel — the
          Workbench never mounts a second instance. */}
      <div
        className="session-workbench__surfaces"
        data-testid="session-workbench-surfaces"
        style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginTop: 8 }}
      >
        <button
          type="button"
          data-testid="session-workbench-show-terminal"
          disabled={!hasSelection || !onShowTerminal}
          title={
            !hasSelection
              ? "Select a session to view its captured terminal output"
              : !onShowTerminal
                ? "Captured terminal not wired"
                : "Reveal this session's captured terminal (its live stdout)"
          }
          onClick={() => {
            if (selected && onShowTerminal) onShowTerminal(selected);
          }}
        >
          Show captured terminal
        </button>

        <button
          type="button"
          data-testid="session-workbench-open-transcript"
          disabled={!hasSelection || !onReviewSession}
          title={
            !hasSelection
              ? "Select a session to open its full transcript"
              : !onReviewSession
                ? "Session Replay not wired"
                : "Open this session's full unified transcript"
          }
          onClick={() => {
            if (selected && onReviewSession) onReviewSession(selected);
          }}
        >
          Open full transcript
        </button>

        {/* ROI #3 STATE RECOVERY: edit-then-resume. Opens the Spawn form
            prefilled from this session's recorded config so the operator can
            tweak before re-spawning (the replay row's button is the one-click
            "as-is" path; this workbench button is the open-in-form path). Honest
            disable when no selection / not wired; the host surfaces the
            not-resumable case (no stored template) as a notice. */}
        <button
          type="button"
          data-testid="session-workbench-resume"
          disabled={!hasSelection || !onResumeSession}
          title={
            !hasSelection
              ? "Select a session to resume it"
              : !onResumeSession
                ? "Resume not wired"
                : "Open the Spawn form prefilled from this session's recorded config to re-spawn it"
          }
          onClick={() => {
            if (selected && onResumeSession) onResumeSession(selected);
          }}
        >
          Resume this session
        </button>

        {!hasSelection ? (
          <span
            className="muted"
            data-testid="session-workbench-no-selection"
            style={{ fontSize: 12 }}
          >
            Select a session to view its captured terminal output and transcript.
          </span>
        ) : null}
      </div>
    </section>
  );
}
