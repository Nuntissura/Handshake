import { useState } from "react";
import type {
  KernelDccActionTriggerResponseV1,
  KernelDccProjectionSurfaceV1,
} from "../lib/api";

type Props = {
  surface: KernelDccProjectionSurfaceV1;
  onTriggerCatalogAction?: (input: {
    work_id: string;
    action_id: string;
    approval_preview_id?: string | null;
    same_turn_approval?: boolean;
  }) => Promise<KernelDccActionTriggerResponseV1>;
};

function joinRefs(refs: string[]): string {
  return refs.length > 0 ? refs.join(", ") : "none";
}

function optionalRef(ref: string | null): string {
  return ref ?? "pending";
}

function firstWorkForAction(surface: KernelDccProjectionSurfaceV1, actionId: string): string | null {
  return surface.work_items.find((work) => work.allowed_action_ids.includes(actionId))?.work_id ?? null;
}

function approvalPreviewForAction(surface: KernelDccProjectionSurfaceV1, actionId: string): string | null {
  return surface.approval_previews.find((preview) => preview.action_id === actionId)?.preview_id ?? null;
}

export function KernelDccProjectionView({ surface, onTriggerCatalogAction }: Props) {
  const [triggerResult, setTriggerResult] = useState<KernelDccActionTriggerResponseV1 | null>(null);
  const [triggerError, setTriggerError] = useState<string | null>(null);
  const [triggeringActionId, setTriggeringActionId] = useState<string | null>(null);

  const triggerAction = async (actionId: string) => {
    if (!onTriggerCatalogAction) return;
    const workId = firstWorkForAction(surface, actionId);
    if (!workId) {
      setTriggerError("No selected DCC work item can trigger this catalog action");
      return;
    }
    const approvalPreviewId = approvalPreviewForAction(surface, actionId);
    setTriggeringActionId(actionId);
    setTriggerError(null);
    setTriggerResult(null);
    try {
      const result = await onTriggerCatalogAction({
        work_id: workId,
        action_id: actionId,
        approval_preview_id: approvalPreviewId,
        same_turn_approval: approvalPreviewId !== null,
      });
      setTriggerResult(result);
    } catch (err) {
      setTriggerError(err instanceof Error ? err.message : "DCC governed action trigger failed");
    } finally {
      setTriggeringActionId(null);
    }
  };

  return (
    <section className="kernel-dcc" data-testid="kernel-dcc-projection" data-surface-id={surface.surface_id}>
      <header className="kernel-dcc__header">
        <div>
          <p className="app-eyebrow">Kernel DCC</p>
          <h2>Action Catalog + Write Boxes</h2>
          <p className="muted">
            {surface.schema_id} / {surface.folded_stub_id}
          </p>
        </div>
        <span className={surface.direct_authority_mutation_allowed ? "kernel-dcc__badge stale" : "kernel-dcc__badge"}>
          {surface.direct_authority_mutation_allowed ? "Authority mutation enabled" : "Projection only"}
        </span>
      </header>

      <div className="kernel-dcc__grid">
        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-panels">
          <h3 id="kernel-dcc-panels">Runtime Panels</h3>
          <table>
            <thead>
              <tr>
                <th>Panel</th>
                <th>Kind</th>
                <th>Sources</th>
                <th>Fields</th>
              </tr>
            </thead>
            <tbody>
              {surface.panels.map((panel) => (
                <tr key={panel.panel_id}>
                  <td>{panel.panel_id}</td>
                  <td>{panel.kind}</td>
                  <td>{joinRefs(panel.source_refs)}</td>
                  <td>{joinRefs(panel.visible_state_fields)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-work">
          <h3 id="kernel-dcc-work">Work Selection</h3>
          <table>
            <thead>
              <tr>
                <th>Work</th>
                <th>WP</th>
                <th>MT</th>
                <th>Status</th>
                <th>Worktree</th>
              </tr>
            </thead>
            <tbody>
              {surface.work_items.map((work) => (
                <tr key={work.work_id}>
                  <td>{work.work_id}</td>
                  <td>{work.wp_id}</td>
                  <td>{work.mt_id ?? "none"}</td>
                  <td>{work.status}</td>
                  <td>{work.worktree_id}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-worktrees">
          <h3 id="kernel-dcc-worktrees">Worktrees</h3>
          <table>
            <thead>
              <tr>
                <th>Worktree</th>
                <th>Branch</th>
                <th>Dirty</th>
                <th>Diff</th>
              </tr>
            </thead>
            <tbody>
              {surface.worktrees.map((worktree) => (
                <tr key={worktree.worktree_id}>
                  <td>{worktree.worktree_id}</td>
                  <td>{worktree.branch}</td>
                  <td>{worktree.dirty ? "dirty" : "clean"}</td>
                  <td>{optionalRef(worktree.diff_ref)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-sessions">
          <h3 id="kernel-dcc-sessions">Sessions</h3>
          <table>
            <thead>
              <tr>
                <th>Session</th>
                <th>Role</th>
                <th>Model</th>
                <th>State</th>
              </tr>
            </thead>
            <tbody>
              {surface.sessions.map((session) => (
                <tr key={session.session_id}>
                  <td>{session.session_id}</td>
                  <td>{session.role}</td>
                  <td>{session.model_id}</td>
                  <td>{session.state}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-actions">
          <h3 id="kernel-dcc-actions">Catalog Actions</h3>
          <ul className="kernel-dcc__list">
            {surface.catalog_action_refs.map((actionId) => (
              <li key={actionId} data-stable-id={`dcc.action_catalog.row.${actionId}`}>
                <code>{actionId}</code>
                {onTriggerCatalogAction ? (
                  <button
                    type="button"
                    onClick={() => void triggerAction(actionId)}
                    disabled={triggeringActionId === actionId}
                  >
                    {triggeringActionId === actionId ? "Triggering..." : "Trigger governed action"}
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
          {triggerResult ? (
            <div role="status">
              Governed trigger accepted: {triggerResult.action_id} / gate enforced:{" "}
              {triggerResult.gate_enforced ? "yes" : "no"} / receipt: {triggerResult.receipt_ref}
            </div>
          ) : null}
          {triggerError ? <div role="alert">{triggerError}</div> : null}
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-proposals">
          <h3 id="kernel-dcc-proposals">Proposals</h3>
          <table>
            <thead>
              <tr>
                <th>Proposal</th>
                <th>Work</th>
                <th>Action</th>
                <th>Status</th>
                <th>Approval</th>
              </tr>
            </thead>
            <tbody>
              {surface.proposals.map((proposal) => (
                <tr key={proposal.proposal_id}>
                  <td>{proposal.proposal_id}</td>
                  <td>{proposal.work_id}</td>
                  <td>{proposal.action_id}</td>
                  <td>{proposal.status}</td>
                  <td>{optionalRef(proposal.approval_preview_id)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-evidence">
          <h3 id="kernel-dcc-evidence">Evidence</h3>
          <table>
            <thead>
              <tr>
                <th>Evidence</th>
                <th>Work</th>
                <th>Kind</th>
                <th>Reference</th>
              </tr>
            </thead>
            <tbody>
              {surface.evidence.map((item) => (
                <tr key={item.evidence_id}>
                  <td>{item.evidence_id}</td>
                  <td>{item.work_id}</td>
                  <td>{item.kind}</td>
                  <td>{item.evidence_ref}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-approval">
          <h3 id="kernel-dcc-approval">Approval Previews</h3>
          <table>
            <thead>
              <tr>
                <th>Preview</th>
                <th>Action</th>
                <th>Scopes</th>
                <th>Same-turn</th>
                <th>Denied code</th>
              </tr>
            </thead>
            <tbody>
              {surface.approval_previews.map((preview) => (
                <tr key={preview.preview_id}>
                  <td>{preview.preview_id}</td>
                  <td>{preview.action_id}</td>
                  <td>{joinRefs(preview.scope_options)}</td>
                  <td>{preview.requires_same_turn_approval ? "required" : "not required"}</td>
                  <td>{preview.denied_failure_code}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-write-boxes">
          <h3 id="kernel-dcc-write-boxes">Write Box Queue</h3>
          <table>
            <thead>
              <tr>
                <th>Write box</th>
                <th>Work</th>
                <th>Kind</th>
                <th>State</th>
                <th>Target</th>
              </tr>
            </thead>
            <tbody>
              {surface.write_box_queue_rows.map((row) => (
                <tr key={row.row_id} data-stable-id={row.stable_element_id}>
                  <td>{row.write_box_id}</td>
                  <td>{row.work_id}</td>
                  <td>{row.kind}</td>
                  <td>{row.lifecycle_state}</td>
                  <td>{row.target_refs[0] ?? "none"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-denials">
          <h3 id="kernel-dcc-denials">Direct Edit Denials</h3>
          <table>
            <thead>
              <tr>
                <th>Denial</th>
                <th>Work</th>
                <th>Actor</th>
                <th>Attempt</th>
                <th>Recovery</th>
              </tr>
            </thead>
            <tbody>
              {surface.direct_edit_denials.map((row) => (
                <tr key={row.row_id} data-stable-id={row.stable_element_id}>
                  <td>{row.denial_id}</td>
                  <td>{row.work_id}</td>
                  <td>{row.actor_id}</td>
                  <td>{row.attempted_action}</td>
                  <td>{row.recovery_instruction}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-promotion">
          <h3 id="kernel-dcc-promotion">Promotion Preview</h3>
          <table>
            <thead>
              <tr>
                <th>Preview</th>
                <th>Work</th>
                <th>Write box</th>
                <th>Target</th>
                <th>Request event</th>
              </tr>
            </thead>
            <tbody>
              {surface.promotion_previews.map((row) => (
                <tr key={row.row_id} data-stable-id={row.stable_element_id}>
                  <td>{row.preview_id}</td>
                  <td>{row.work_id}</td>
                  <td>{row.write_box_id}</td>
                  <td>{row.promotion_target_ref}</td>
                  <td>{optionalRef(row.request_event_ref)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-freshness">
          <h3 id="kernel-dcc-freshness">Freshness Badges</h3>
          <ul className="kernel-dcc__list">
            {surface.freshness_badges.map((badge) => (
              <li key={badge.badge_id} data-stable-id={badge.stable_element_id}>
                <strong>{badge.source_projection_id}</strong>
                <span>{badge.state_vector}</span>
                <span className={badge.stale ? "kernel-dcc__badge stale" : "kernel-dcc__badge"}>
                  {badge.stale ? "stale" : "fresh"}
                </span>
              </li>
            ))}
          </ul>
        </section>

        <section className="kernel-dcc__panel" aria-labelledby="kernel-dcc-stable-ids">
          <h3 id="kernel-dcc-stable-ids">Stable Element Ids</h3>
          <ul className="kernel-dcc__list">
            {surface.stable_element_ids.map((element) => (
              <li key={element.element_id}>
                <code>{element.element_id}</code>
                <span>{element.element_kind}</span>
              </li>
            ))}
          </ul>
        </section>

        {surface.spawn_tree_projection ? (
          <section
            className="kernel-dcc__panel"
            aria-labelledby="kernel-dcc-spawn-tree"
            data-testid="kernel-dcc-session-spawn-tree"
          >
            <h3 id="kernel-dcc-spawn-tree">Session Spawn Tree</h3>
            <p className="muted">
              {surface.spawn_tree_projection.schema_id} / max depth {surface.spawn_tree_projection.max_depth} /
              announce-back badges {surface.spawn_tree_projection.announce_back_badge_count}
            </p>
            <table>
              <thead>
                <tr>
                  <th>Session</th>
                  <th>Parent</th>
                  <th>Role</th>
                  <th>Depth</th>
                  <th>Children</th>
                  <th>Active children</th>
                  <th>Spawn mode</th>
                  <th>Cascade cancel</th>
                  <th>Announce-back</th>
                </tr>
              </thead>
              <tbody>
                {surface.spawn_tree_projection.nodes.map((node) => (
                  <tr key={node.session_id} data-stable-id={`dcc.session_spawn_tree.node.${node.session_id}`}>
                    <td>{node.session_id}</td>
                    <td>{node.parent_session_id ?? "root"}</td>
                    <td>{node.role_id}</td>
                    <td>{node.depth}</td>
                    <td>{node.child_count}</td>
                    <td>{node.active_child_count}</td>
                    <td>{node.spawn_mode}</td>
                    <td>{node.cascade_cancel_available ? "available" : "unavailable"}</td>
                    <td>{joinRefs(node.announce_back_badges)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p className="muted">Runtime records: {joinRefs(surface.spawn_tree_projection.runtime_record_refs)}</p>
            <p className="muted">
              Cascade cancel sessions: {joinRefs(surface.spawn_tree_projection.cascade_cancel_session_ids)}
            </p>
          </section>
        ) : null}
      </div>
    </section>
  );
}
