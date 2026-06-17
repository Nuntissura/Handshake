import { loadViewModeFromStorage } from "./viewMode";

const BASE_URL = "http://127.0.0.1:37501";

/**
 * The Handshake REST base, exported for surfaces that build typed backend URLs
 * outside this module (e.g. media embed asset resolution, MT-244) so the base
 * stays single-sourced here.
 */
export const API_BASE_URL = BASE_URL;

type FetchOptions = {
  method?: string;
  body?: unknown;
  headers?: Record<string, string>;
};

export type WriteActorKind = "HUMAN" | "AI" | "SYSTEM";

export type WriteContext = {
  actor_kind?: WriteActorKind;
  actor_id?: string;
  job_id?: string;
  workflow_id?: string;
};

function writeContextHeaders(ctx?: WriteContext): Record<string, string> | undefined {
  if (!ctx) return undefined;
  const headers: Record<string, string> = {};
  if (ctx.actor_kind) headers["x-hsk-actor-kind"] = ctx.actor_kind;
  if (ctx.actor_id) headers["x-hsk-actor-id"] = ctx.actor_id;
  if (ctx.job_id) headers["x-hsk-job-id"] = ctx.job_id;
  if (ctx.workflow_id) headers["x-hsk-workflow-id"] = ctx.workflow_id;
  return headers;
}

function codeNavHeaders(label: string): Record<string, string> {
  const safeLabel = label.replace(/[^a-zA-Z0-9_.:-]+/g, "-").slice(0, 80) || "code-nav";
  return {
    "x-hsk-actor-kind": "model_adapter",
    "x-hsk-actor-id": "handshake-monaco-code-intelligence",
    "x-hsk-kernel-task-run-id": `MT-249-${safeLabel}`,
    "x-hsk-session-run-id": "frontend-code-intelligence",
    "x-hsk-correlation-id": `frontend-code-intelligence-${safeLabel}`,
  };
}

/**
 * Typed backend request failure (iteration-3 M18): carries the HTTP status and
 * raw body so consumers (EditorBackendErrorStates and friends) can classify
 * failures STRUCTURALLY instead of substring-matching free-text messages.
 * The message keeps the legacy `Request failed: <status> ...` shape.
 */
export class ApiRequestError extends Error {
  constructor(
    public readonly status: number,
    public readonly statusText: string,
    public readonly body: string,
  ) {
    super(`Request failed: ${status} ${statusText} - ${body}`);
    this.name = "ApiRequestError";
  }
}

async function request<T>(path: string, options: FetchOptions = {}): Promise<T> {
  const response = await fetch(`${BASE_URL}${path}`, {
    method: options.method ?? "GET",
    headers: {
      "Content-Type": "application/json",
      ...(options.headers ?? {}),
    },
    body: options.body ? JSON.stringify(options.body) : undefined,
  });

  if (!response.ok) {
    const text = await response.text();
    throw new ApiRequestError(response.status, response.statusText, text);
  }

  // Handle empty responses (e.g., 204 No Content or DELETE with no body)
  const contentLength = response.headers.get("content-length");
  if (response.status === 204 || contentLength === "0") {
    return undefined as T;
  }

  // Check if response has content before parsing JSON
  const text = await response.text();
  if (!text || text.trim().length === 0) {
    return undefined as T;
  }

  return JSON.parse(text) as T;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export async function sha256HexUtf8(text: string): Promise<string> {
  if (typeof crypto === "undefined" || !crypto.subtle) {
    throw new Error("crypto.subtle is not available in this environment");
  }
  const bytes = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return bytesToHex(new Uint8Array(digest));
}

export type Workspace = {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
};

export type DocumentSummary = {
  id: string;
  workspace_id: string;
  title: string;
  created_at: string;
  updated_at: string;
};

export type Block = {
  id: string;
  kind: string;
  sequence: number;
  raw_content: string;
  display_content: string;
  derived_content: unknown;
};

export type SelectionRangeV1 = {
  schema_version: "hsk.selection_range@v1";
  surface: "docs";
  coordinate_space: "doc_text_utf8_v1";
  start_utf8: number;
  end_utf8: number;
  doc_preimage_sha256: string;
  selection_preimage_sha256: string;
};

export type DocPatchOpV1 = {
  op: "replace_range";
  range_utf8: { start: number; end: number };
  insert_text: string;
};

export type DocPatchsetV1 = {
  schema_version: "hsk.doc_patchset@v1";
  doc_id: string;
  selection: SelectionRangeV1;
  boundary_normalization: "disabled";
  ops: DocPatchOpV1[];
  summary?: string | null;
};

export type AtelierApplySuggestionV1 = {
  role_id: string;
  suggestion_id: string;
  source_job_id: string;
  patchset: DocPatchsetV1;
};

export type AtelierApplyRequestV1 = {
  doc_id: string;
  selection: SelectionRangeV1;
  suggestions_to_apply: AtelierApplySuggestionV1[];
};

export type AtelierRoleSummary = {
  role_id: string;
  display_name: string;
};

export type AtelierRolesResponse = {
  roles: AtelierRoleSummary[];
};

export type BlockInput = {
  id?: string;
  kind: string;
  sequence: number;
  raw_content: string;
  display_content?: string;
  derived_content?: unknown;
};

export type DocumentWithBlocks = DocumentSummary & {
  blocks: Block[];
};

export type CanvasSummary = {
  id: string;
  workspace_id: string;
  title: string;
  created_at: string;
  updated_at: string;
};

export type CanvasNode = {
  id: string;
  kind: string;
  position_x: number;
  position_y: number;
  data: unknown;
};

export type CanvasEdge = {
  id: string;
  from_node_id: string;
  to_node_id: string;
  kind: string;
};

export type CanvasWithGraph = CanvasSummary & {
  nodes: CanvasNode[];
  edges: CanvasEdge[];
};

export type CanvasNodeInput = {
  id?: string;
  kind: string;
  position_x: number;
  position_y: number;
  data?: unknown;
};

export type CanvasEdgeInput = {
  id?: string;
  from_node_id: string;
  to_node_id: string;
  kind: string;
};

export type LogTailResponse = {
  lines: string[];
};

export type HealthResponse = {
  status?: string;
  component?: string;
  version?: string;
  db_status?: string;
};

export type DccPanelKind =
  | "WorkSelection"
  | "WorktreeState"
  | "SessionState"
  | "ActionCatalog"
  | "WriteBoxQueue"
  | "DirectEditDenialView"
  | "PromotionPreview"
  | "FreshnessBadges"
  | "ProposalState"
  | "DiffEvidence"
  | "ApprovalPreview"
  | "Timeline";

export type DccEvidenceKind =
  | "DiffPatch"
  | "FlightRecorderEvent"
  | "Receipt"
  | "Screenshot"
  | "ValidationOutput";

export type DccProposalStatus = "Draft" | "AwaitingApproval" | "Approved" | "Denied" | "Promoted";
export type DccApprovalScope = "Once" | "Job" | "Workspace";

export type DccRuntimePanelV1 = {
  panel_id: string;
  kind: DccPanelKind;
  projection_only: boolean;
  source_refs: string[];
  visible_state_fields: string[];
};

export type DccWorkItemV1 = {
  work_id: string;
  wp_id: string;
  mt_id: string | null;
  status: string;
  worktree_id: string;
  session_ids: string[];
  proposal_ids: string[];
  evidence_ids: string[];
  allowed_action_ids: string[];
};

export type DccWorktreeStateV1 = {
  worktree_id: string;
  path_ref: string;
  branch: string;
  dirty: boolean;
  diff_ref: string | null;
  linked_work_ids: string[];
};

export type DccSessionRuntimeStateV1 = {
  session_id: string;
  role: string;
  model_id: string;
  backend: string;
  worktree_id: string;
  wp_id: string;
  mt_id: string | null;
  state: string;
};

export type DccProposalStateV1 = {
  proposal_id: string;
  work_id: string;
  action_id: string;
  status: DccProposalStatus;
  evidence_ids: string[];
  approval_preview_id: string | null;
};

export type DccEvidenceItemV1 = {
  evidence_id: string;
  kind: DccEvidenceKind;
  evidence_ref: string;
  work_id: string;
};

export type DccApprovalPreviewV1 = {
  preview_id: string;
  action_id: string;
  scope_options: DccApprovalScope[];
  requires_same_turn_approval: boolean;
  denied_failure_code: string;
};

export type WriteBoxKind =
  | "Draft"
  | "CrdtWorkspace"
  | "Proposal"
  | "Patch"
  | "Artifact"
  | "MirrorAdvisory"
  | "Memory"
  | "Execution"
  | "Promotion";
export type WriteBoxLifecycleState =
  | "Open"
  | "ReadyForValidation"
  | "ValidationFailed"
  | "Validated"
  | "PromotionQueued"
  | "Promoted"
  | "Denied"
  | "Archived";
export type WriteBoxValidationState = "Pending" | "Valid" | "Invalid" | "Denied";

export type DccCatalogActionRowV1 = {
  action_id: string;
  target_authority_class: string;
  input_schema_id: string;
  result_schema_id: string;
  role_eligibility: string[];
  capability_requirements: string[];
  approval_posture: string;
  preview_behavior_summary: string;
  preview_panel_id: string;
};

export type DccWriteBoxQueueRowV1 = {
  row_id: string;
  write_box_id: string;
  work_id: string;
  kind: WriteBoxKind;
  lifecycle_state: WriteBoxLifecycleState;
  actor_id: string;
  target_refs: string[];
  validation_state: WriteBoxValidationState;
  denial_receipt_refs: string[];
  promotion_receipt_refs: string[];
  event_ledger_event_refs: string[];
  stale_state_vector: boolean;
  stable_element_id: string;
};

export type DccDirectEditDenialRowV1 = {
  row_id: string;
  denial_id: string;
  work_id: string;
  actor_id: string;
  target_ref: string;
  attempted_action: string;
  recovery_instruction: string;
  ui_response_ref: string;
  api_response_ref: string;
  stable_element_id: string;
};

export type DccPromotionPreviewStaleRisk =
  | "None"
  | "StaleStateVector"
  | "DuplicateIdempotency"
  | "Both";

export type DccPromotionPreviewRowV1 = {
  row_id: string;
  preview_id: string;
  work_id: string;
  write_box_id: string;
  promotion_target_ref: string;
  request_event_ref: string | null;
  accepted_event_ref: string | null;
  rejected_event_ref: string | null;
  state_vector: string;
  validation_check_summaries: string[];
  idempotency_key: string;
  expected_event_kinds: string[];
  stale_risk: DccPromotionPreviewStaleRisk;
  freshness_badge_id: string;
  stable_element_id: string;
};

export type DccFreshnessBadgeV1 = {
  badge_id: string;
  source_projection_id: string;
  source_ref: string;
  state_vector: string;
  updated_at_ref: string;
  stale: boolean;
  stable_element_id: string;
};

export type DccStableElementIdV1 = {
  element_id: string;
  surface_id: string;
  element_kind: string;
  source_ref: string;
};

export type SessionSpawnTreeNodeProjectionV1 = {
  session_id: string;
  parent_session_id: string | null;
  role_id: string;
  depth: number;
  child_count: number;
  active_child_count: number;
  spawn_mode: string;
  runtime_state: string;
  cascade_cancel_available: boolean;
  announce_back_badges: string[];
};

export type SessionSpawnTreeDccProjectionV1 = {
  schema_id: string;
  tree_id: string;
  panel_id: string;
  visible_fields: string[];
  nodes: SessionSpawnTreeNodeProjectionV1[];
  max_depth: number;
  cascade_cancel_session_ids: string[];
  announce_back_badge_count: number;
  runtime_record_refs: string[];
  mutates_runtime_records: boolean;
};

export type SessionAnnounceBackBadgeV1 = {
  badge_id: string;
  session_id: string;
  label: string;
  mailbox_route: string;
};

export type SessionSpawnRuntimeRecordV1 = {
  session_id: string;
  parent_session_id: string | null;
  role_id: string;
  spawn_mode: string;
  runtime_state: string;
  cascade_cancel_supported: boolean;
  announce_back_badges: SessionAnnounceBackBadgeV1[];
  runtime_record_ref: string;
  flight_recorder_ref: string;
};

export type KernelDccProjectionSurfaceV1 = {
  schema_id: string;
  surface_id: string;
  folded_stub_id: string;
  panels: DccRuntimePanelV1[];
  work_items: DccWorkItemV1[];
  worktrees: DccWorktreeStateV1[];
  sessions: DccSessionRuntimeStateV1[];
  proposals: DccProposalStateV1[];
  evidence: DccEvidenceItemV1[];
  approval_previews: DccApprovalPreviewV1[];
  write_box_queue_rows: DccWriteBoxQueueRowV1[];
  direct_edit_denials: DccDirectEditDenialRowV1[];
  promotion_previews: DccPromotionPreviewRowV1[];
  freshness_badges: DccFreshnessBadgeV1[];
  stable_element_ids: DccStableElementIdV1[];
  catalog_action_refs: string[];
  catalog_action_rows: DccCatalogActionRowV1[];
  direct_authority_mutation_allowed: boolean;
  ungoverned_tool_execution_allowed: boolean;
  destructive_git_ops_require_same_turn_approval: boolean;
  flight_recorder_event_types: string[];
  product_authority_refs: string[];
  folded_source_refs: string[];
  session_spawn_runtime_records?: SessionSpawnRuntimeRecordV1[] | null;
  spawn_tree_projection?: SessionSpawnTreeDccProjectionV1 | null;
};

export type KernelDccActionTriggerRequestV1 = {
  work_id: string;
  action_id: string;
  approval_preview_id?: string | null;
  same_turn_approval?: boolean;
};

export type KernelDccActionTriggerResponseV1 = {
  schema_id: "hsk.kernel.dcc_governed_action_trigger_result@1";
  work_id: string;
  action_id: string;
  triggered: boolean;
  catalog_checked: boolean;
  preview_checked: boolean;
  gate_enforced: boolean;
  approval_preview_id: string | null;
  authority_effect: string;
  approval_posture: string;
  expected_write_box_kinds: string[];
  receipt_ref: string;
};

export type SessionSpawnTreeDccRequestV1 = {
  schema_id: string;
  tree_id: string;
  folded_stub_ids: string[];
  panel_id: string;
  visible_fields: string[];
  runtime_records: SessionSpawnRuntimeRecordV1[];
  product_authority_refs: string[];
  folded_source_refs: string[];
};

export type WorkflowRun = {
  id: string;
  job_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type FemsProtocolId =
  | "memory_extract_v0.1"
  | "memory_consolidate_v0.1"
  | "memory_forget_v0.1";

export const FEMS_PROTOCOLS: readonly FemsProtocolId[] = [
  "memory_extract_v0.1",
  "memory_consolidate_v0.1",
  "memory_forget_v0.1",
];

export function isFemsProtocolId(value: string): value is FemsProtocolId {
  return (FEMS_PROTOCOLS as readonly string[]).includes(value);
}

export type FlightEvent = {
  event_id: string;
  trace_id: string;
  timestamp: string;
  actor: "human" | "agent" | "system";
  actor_id: string;
  event_type:
    | "system"
    | "llm_inference"
    | "diagnostic"
    | "capability_action"
    | "security_violation"
    | "workflow_recovery"
    | "debug_bundle_export"
    | "governance_pack_export"
    | "memory_write_proposed"
    | "memory_write_reviewed"
    | "memory_write_committed"
    | "memory_pack_built"
    | "memory_item_status_changed"
    | "runtime_chat_message_appended"
    | "runtime_chat_ans001_validation"
    | "runtime_chat_session_closed";
  job_id?: string;
  workflow_id?: string;
  model_id?: string;
  wsids: string[];
  activity_span_id?: string;
  session_span_id?: string;
  capability_id?: string;
  policy_decision_id?: string;
  payload: unknown;
};

export type RuntimeChatEventType =
  | "runtime_chat_message_appended"
  | "runtime_chat_ans001_validation"
  | "runtime_chat_session_closed";

export type RuntimeChatEventV0_1 = {
  schema_version: "hsk.fr.runtime_chat@0.1";
  event_id: string; // FR-EVT-RUNTIME-CHAT-1xx
  ts_utc: string; // RFC3339
  session_id: string;

  job_id?: string;
  work_packet_id?: string;
  spec_id?: string;
  wsid?: string;

  type: RuntimeChatEventType;

  message_id?: string;
  role?: "user" | "assistant";
  model_role?: "frontend" | "orchestrator" | "worker" | "validator";

  body_sha256?: string;
  ans001_sha256?: string;

  ans001_compliant?: boolean;
  violation_clauses?: string[];
};

export type SecurityViolationPayload = {
  violation_type: string;
  description: string;
  trigger: string;
  guard_name: string;
  offset?: number;
  context?: string;
  action_taken: string;
};

export type DiagnosticSeverity = "fatal" | "error" | "warning" | "info" | "hint";
export type DiagnosticSource =
  | "lsp"
  | "terminal"
  | "validator"
  | "engine"
  | "connector"
  | "system"
  | `plugin:${string}`
  | `matcher:${string}`;
export type DiagnosticSurface = "monaco" | "canvas" | "sheet" | "terminal" | "connector" | "system";
export type LinkConfidence = "direct" | "inferred" | "ambiguous" | "unlinked";
export type DiagnosticStatus = "open" | "acknowledged" | "muted" | "resolved";
export type DiagnosticActor = "human" | "agent" | "system";

export type DiagnosticRange = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type DiagnosticLocation = {
  path?: string;
  uri?: string;
  wsid?: string;
  entity_id?: string;
  range?: DiagnosticRange;
};

export type ArtifactHashes = {
  input_hash?: string;
  output_hash?: string;
  diff_hash?: string;
};

export type EvidenceRefs = {
  fr_event_ids?: string[];
  related_job_ids?: string[];
  related_activity_span_ids?: string[];
  related_session_span_ids?: string[];
  artifact_hashes?: ArtifactHashes;
};

export type Diagnostic = {
  id: string;
  fingerprint: string;
  title: string;
  message: string;
  severity: DiagnosticSeverity;
  source: DiagnosticSource;
  surface: DiagnosticSurface;
  tool?: string | null;
  code?: string | null;
  tags?: string[] | null;
  wsid?: string | null;
  job_id?: string | null;
  model_id?: string | null;
  actor?: DiagnosticActor | null;
  capability_id?: string | null;
  policy_decision_id?: string | null;
  locations?: DiagnosticLocation[] | null;
  evidence_refs?: EvidenceRefs | null;
  link_confidence: LinkConfidence;
  status?: DiagnosticStatus | null;
  count?: number | null;
  first_seen?: string | null;
  last_seen?: string | null;
  timestamp: string;
  updated_at?: string | null;
};

export type ProblemGroup = {
  fingerprint: string;
  count: number;
  first_seen: string;
  last_seen: string;
  sample: Diagnostic;
};

export type DiagnosticFilters = {
  severity?: DiagnosticSeverity;
  source?: string;
  surface?: DiagnosticSurface;
  wsid?: string;
  job_id?: string;
  from?: string;
  to?: string;
  fingerprint?: string;
  limit?: number;
};

export async function listWorkspaces(): Promise<Workspace[]> {
  return request("/workspaces");
}

export async function createWorkspace(name: string): Promise<Workspace> {
  return request("/workspaces", { method: "POST", body: { name } });
}

export async function deleteWorkspace(id: string): Promise<void> {
  await request(`/workspaces/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export async function listDocuments(workspaceId: string): Promise<DocumentSummary[]> {
  return request(`/workspaces/${workspaceId}/documents`);
}

export async function createDocument(workspaceId: string, title: string): Promise<DocumentSummary> {
  return request(`/workspaces/${workspaceId}/documents`, { method: "POST", body: { title } });
}

export async function deleteDocument(documentId: string): Promise<void> {
  await request(`/documents/${encodeURIComponent(documentId)}`, { method: "DELETE" });
}

export async function listCanvases(workspaceId: string): Promise<CanvasSummary[]> {
  return request(`/workspaces/${workspaceId}/canvases`);
}

export async function createCanvas(workspaceId: string, title: string): Promise<CanvasSummary> {
  return request(`/workspaces/${workspaceId}/canvases`, { method: "POST", body: { title } });
}

export async function deleteCanvas(canvasId: string): Promise<void> {
  await request(`/canvases/${encodeURIComponent(canvasId)}`, { method: "DELETE" });
}

export async function getDocument(docId: string): Promise<DocumentWithBlocks> {
  return request(`/documents/${docId}`);
}

export async function getCanvas(canvasId: string): Promise<CanvasWithGraph> {
  return request(`/canvases/${canvasId}`);
}

export type DiagnosticInput = {
  title: string;
  message: string;
  severity: DiagnosticSeverity;
  source: DiagnosticSource;
  surface: DiagnosticSurface;
  tool?: string | null;
  code?: string | null;
  tags?: string[] | null;
  wsid?: string | null;
  job_id?: string | null;
  model_id?: string | null;
  actor?: DiagnosticActor | null;
  capability_id?: string | null;
  policy_decision_id?: string | null;
  locations?: DiagnosticLocation[] | null;
  evidence_refs?: EvidenceRefs | null;
  link_confidence: LinkConfidence;
  status?: DiagnosticStatus | null;
  count?: number | null;
  first_seen?: string | null;
  last_seen?: string | null;
  timestamp?: string | null;
  updated_at?: string | null;
};

export async function listDiagnostics(filters?: DiagnosticFilters): Promise<Diagnostic[]> {
  const params = new URLSearchParams();
  if (filters?.severity) params.append("severity", filters.severity);
  if (filters?.source) params.append("source", filters.source);
  if (filters?.surface) params.append("surface", filters.surface);
  if (filters?.wsid) params.append("wsid", filters.wsid);
  if (filters?.job_id) params.append("job_id", filters.job_id);
  if (filters?.from) params.append("from", new Date(filters.from).toISOString());
  if (filters?.to) params.append("to", new Date(filters.to).toISOString());
  if (filters?.fingerprint) params.append("fingerprint", filters.fingerprint);
  if (filters?.limit) params.append("limit", filters.limit.toString());

  const query = params.toString();
  const path = query.length > 0 ? `/api/diagnostics?${query}` : "/api/diagnostics";
  return request(path);
}

export async function listProblemGroups(filters?: DiagnosticFilters): Promise<ProblemGroup[]> {
  const params = new URLSearchParams();
  if (filters?.severity) params.append("severity", filters.severity);
  if (filters?.source) params.append("source", filters.source);
  if (filters?.surface) params.append("surface", filters.surface);
  if (filters?.wsid) params.append("wsid", filters.wsid);
  if (filters?.job_id) params.append("job_id", filters.job_id);
  if (filters?.from) params.append("from", new Date(filters.from).toISOString());
  if (filters?.to) params.append("to", new Date(filters.to).toISOString());
  if (filters?.fingerprint) params.append("fingerprint", filters.fingerprint);
  if (filters?.limit) params.append("limit", filters.limit.toString());

  const query = params.toString();
  const path = query.length > 0 ? `/api/diagnostics/problems?${query}` : "/api/diagnostics/problems";
  return request(path);
}

export async function getDiagnostic(id: string): Promise<Diagnostic> {
  return request(`/api/diagnostics/${encodeURIComponent(id)}`);
}

export async function createDiagnostic(input: DiagnosticInput): Promise<Diagnostic> {
  return request("/api/diagnostics", { method: "POST", body: input });
}

export type UserManualPageSummary = {
  slug: string;
  title: string;
  page_kind: string;
  audience: string;
  manual_version: string;
  content_hash: string;
  status: string;
  updated_at: string;
};

export type UserManualPage = UserManualPageSummary & {
  page_id?: string;
  body?: unknown;
  source_kind?: string;
  spec_anchors?: string[];
  superseded_by_slug?: string | null;
  ledger_event_id?: string | null;
  created_at?: string;
};

export type UserManualSection = {
  section_id: string;
  page_id?: string;
  position?: number;
  section_kind?: string;
  title: string;
  body_md: string;
  body_json?: unknown | null;
};

export type UserManualAnchor = {
  anchor_id: string;
  page_id?: string;
  anchor_kind: string;
  anchor_value: string;
  http_method?: string;
};

export type UserManualPagesResponse = {
  manual_version: string;
  route_namespace: string;
  count: number;
  pages: UserManualPageSummary[];
};

export type UserManualPageResponse = {
  page: UserManualPage;
  sections: UserManualSection[];
  anchors: UserManualAnchor[];
  bootstrap_receipt_event_id: string;
  bootstrap_identity_used: boolean;
};

export type UserManualSearchHit = {
  kind?: "page" | "section" | "tool" | string;
  result_kind?: "page" | "section" | "tool" | string;
  result_ref: string;
  page_slug: string | null;
  title: string;
  excerpt: string;
};

export type UserManualSearchResponse = {
  query: string;
  count: number;
  results: UserManualSearchHit[];
};

export type UserManualAccessPoint = {
  access_point_id: string;
  host_surface: string;
  entry_kind: string;
  target_page_slug: string;
  ui_wiring_route: string;
  stable_element_id: string;
  note: string;
  target_resolves: boolean;
};

export type UserManualAccessPointsResponse = {
  count: number;
  access_points: UserManualAccessPoint[];
};

export type LoomGraphSearchSourceKind =
  | "loom_block"
  | "file"
  | "tag_hub"
  | "document"
  | "symbol"
  | "work_packet"
  | "micro_task"
  | "user_manual_page"
  | "wiki_page";

export type LoomGraphSearchResultKind = "loom_block" | "knowledge_entity" | "user_manual_page" | "wiki_page";

export type LoomGraphSearchHit = {
  result_kind: LoomGraphSearchResultKind;
  source_kind: LoomGraphSearchSourceKind;
  ref_id: string;
  title: string;
  excerpt: string;
  block?: unknown | null;
  score: number;
  metadata: unknown;
};

export type QuickSwitcherRecent = {
  workspace_id: string;
  hit_key: string;
  source_kind: LoomGraphSearchSourceKind;
  ref_id: string;
  result_kind: LoomGraphSearchResultKind;
  title: string;
  excerpt: string;
  metadata: unknown;
  selected_count: number;
  selected_at: string;
  event_ledger_event_id: string;
};

export type RecordQuickSwitcherRecentRequest = {
  result_kind: LoomGraphSearchResultKind;
  source_kind: LoomGraphSearchSourceKind;
  ref_id: string;
  title: string;
  excerpt: string;
  metadata: unknown;
};

export type WorkbenchLayoutStateResponse = {
  workspace_id: string;
  layout_state: Record<string, unknown> | null;
  updated_at: string | null;
  event_ledger_event_id: string | null;
};

export type WorkspaceSettingsStateResponse = {
  workspace_id: string;
  settings_state: Record<string, unknown> | null;
  updated_at: string | null;
  event_ledger_event_id: string | null;
};

export type LoomBlockContentType = "note" | "file" | "annotated_file" | "tag_hub" | "journal" | string;

export type LoomBlockDerived = {
  full_text_index?: string | null;
  embedding_id?: string | null;
  auto_tags?: string[] | null;
  auto_caption?: string | null;
  quality_score?: number | null;
  backlink_count: number;
  mention_count: number;
  tag_count: number;
  thumbnail_asset_id?: string | null;
  proxy_asset_id?: string | null;
  preview_status: string;
  generated_by?: unknown | null;
};

export type LoomBlock = {
  block_id: string;
  workspace_id: string;
  content_type: LoomBlockContentType;
  document_id?: string | null;
  asset_id?: string | null;
  title?: string | null;
  original_filename?: string | null;
  content_hash?: string | null;
  pinned: boolean;
  favorite: boolean;
  pin_order?: number | null;
  journal_date?: string | null;
  created_at: string;
  updated_at: string;
  imported_at?: string | null;
  derived: LoomBlockDerived;
};

export type LoomBlockUpdateRequest = {
  title?: string | null;
  pinned?: boolean;
  favorite?: boolean;
  journal_date?: string | null;
  /**
   * MT-258 properties-panel tag editing. Tags are NOT a column: each entry is a
   * TagHub block id, applied as a `tag` loom_edge from this block. `add_tags`
   * creates the tag edge; `remove_tags` deletes it. The backend recomputes
   * `derived.tag_count` after mutating edges.
   */
  add_tags?: string[];
  remove_tags?: string[];
};

/**
 * MT-258 note-transclusion read-through. The host doc embeds a LoomBlock by
 * reference (a single `loomTransclusion` atom node) and renders the SOURCE
 * document content this resolves to. The host never persists the body; edits
 * route to `source_document_id`.
 */
export type LoomBlockTransclusion = {
  block_id: string;
  workspace_id: string;
  source_document_id: string | null;
  source_doc_version: number | null;
  content_json: JSONContentLike | null;
  resolved: boolean;
  unresolved_reason?: string | null;
};

export type LoomViewType = "all" | "unlinked" | "sorted" | "pins" | "favorites";

export type LoomViewQuery = {
  contentType?: LoomBlockContentType;
  mime?: string;
  tagIds?: string[];
  mentionIds?: string[];
  limit?: number;
  offset?: number;
};

export type LoomViewGroup = {
  edge_type: string;
  target_block_id: string;
  blocks: LoomBlock[];
};

export type LoomViewResponse =
  | { view_type: "all"; blocks: LoomBlock[] }
  | { view_type: "unlinked"; blocks: LoomBlock[] }
  | { view_type: "pins"; blocks: LoomBlock[] }
  | { view_type: "favorites"; blocks: LoomBlock[] }
  | { view_type: "sorted"; groups: LoomViewGroup[] };

// MT-262 BlockCollectionViews ------------------------------------------------
export type BlockViewKind = "table" | "kanban" | "calendar";

export type BlockViewField =
  | "title"
  | "created"
  | "updated"
  | "journal_date"
  | "content_type"
  | "pinned"
  | "favorite"
  | "backlink_count"
  | "mention_count"
  | "tag_count";

export type BlockViewSortDirection = "asc" | "desc";

export type BlockViewSort = {
  field: BlockViewField;
  direction?: BlockViewSortDirection;
};

export type BlockViewQuery = {
  content_type?: LoomBlockContentType;
  mime?: string;
  date_from?: string;
  date_to?: string;
  tag_ids?: string[];
  mention_ids?: string[];
};

export type BlockViewGroupBy = { kind: "tag" } | { kind: "field"; field: BlockViewField };

export type BlockViewDefinition = {
  kind: BlockViewKind;
  query?: BlockViewQuery;
  columns?: BlockViewField[];
  group_by?: BlockViewGroupBy | null;
  sort?: BlockViewSort | null;
  calendar_date_field?: BlockViewField | null;
};

export type BlockViewRecord = {
  block: LoomBlock;
  definition: BlockViewDefinition;
};

export type BlockViewLane = {
  key: string;
  blocks: LoomBlock[];
};

export type BlockViewResults = {
  kind: BlockViewKind;
  blocks: LoomBlock[];
  groups?: BlockViewLane[];
  total_returned: number;
};

/** The sentinel Kanban lane key for blocks with no tag in a tag-grouped view. */
export const BLOCK_VIEW_UNTAGGED_LANE = "__untagged__";

export type LoomWikiProjection = {
  projection_id: string;
  workspace_id: string;
  title: string;
  source_block_ids: string[];
  rendered_content: string;
  staleness_hash: string;
  rebuild_status: string;
  page_type?: string | null;
  compile_stamp?: unknown | null;
  page_links: unknown;
  created_at: string;
  updated_at: string;
};

export type LoomWikiPageResponse = LoomWikiProjection & {
  staleness_verdict: unknown;
};

export type LoomGraphSearchQuery = {
  q: string;
  limit?: number;
  offset?: number;
  sourceKinds?: LoomGraphSearchSourceKind[];
  contentType?: string;
  mime?: string;
  tagIds?: string[];
  mentionIds?: string[];
  backlinkDepth?: number;
  caseSensitive?: boolean;
  wholeWord?: boolean;
  isRegex?: boolean;
  path?: string;
};

export type CodeSymbolDefinition = {
  span_id: string;
  source_id: string;
  line_start: number;
  line_end: number;
  range_start: number;
  range_end: number;
  section_path?: string | null;
};

export type CodeSymbolNavProjection = {
  symbol_entity_id: string;
  symbol_key: string;
  display_name: string;
  symbol_kind: string;
  owning_wp?: string | null;
  primary_source_id?: string | null;
  lifecycle_state: string;
  definition: CodeSymbolDefinition | null;
  staleness: unknown;
};

export type CodeSymbolResponse = {
  symbol: CodeSymbolNavProjection;
  nav_receipt_event_id: string;
  quiet_background_work_receipt_id: string;
};

export type CodeSymbolLookupQuery = {
  workspaceId: string;
  name?: string;
  prefix?: string;
  path?: string;
  limit?: number;
};

export type CodeSymbolLookupResponse = {
  workspace_id: string;
  matches: CodeSymbolNavProjection[];
  nav_receipt_event_id: string;
  quiet_background_work_receipt_id: string;
};

export type CodeSymbolEvidenceSpan = {
  span_id: string;
  source_id: string;
  span_kind?: string;
  line_start?: number | null;
  line_end?: number | null;
  range_start?: number | null;
  range_end?: number | null;
  section_path?: string | null;
  content_sha256?: string | null;
  parser_version?: string | null;
};

export type CodeSymbolReference = {
  symbol_entity_id: string;
  symbol_key: string;
  display_name: string;
  confidence: number;
  evidence_spans: CodeSymbolEvidenceSpan[];
  staleness: unknown;
};

export type CodeSymbolReferencesResponse = {
  symbol_entity_id: string;
  staleness: unknown;
  callers: CodeSymbolReference[];
  callees: CodeSymbolReference[];
  nav_receipt_event_id: string;
  quiet_background_work_receipt_id: string;
};

export type CodeLensLineRange = {
  start_line: number;
  end_line: number;
};

export type CodeFileLensEntry = {
  symbol_entity_id: string;
  symbol_key: string;
  display_name: string;
  symbol_kind: string;
  definition: CodeLensLineRange;
  references: CodeLensLineRange[];
  doc?: string | null;
  caller_count: number;
};

export type CodeFileLensResponse = {
  workspace_id: string;
  relative_path: string;
  staleness: unknown;
  truncated: boolean;
  entries: CodeFileLensEntry[];
  nav_receipt_event_id: string;
  quiet_background_work_receipt_id: string;
};

export type UserManualPagesQuery = {
  kind?: string;
  audience?: string;
  limit?: number;
};

function userManualQuery(params: Record<string, string | number | undefined>): string {
  const query = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined) query.append(key, String(value));
  });
  return query.toString();
}

export async function listUserManualPages(
  params: UserManualPagesQuery = {},
): Promise<UserManualPagesResponse> {
  const query = userManualQuery({
    kind: params.kind,
    audience: params.audience,
    limit: params.limit,
  });
  return request(query ? `/api/usermanual/pages?${query}` : "/api/usermanual/pages");
}

export async function getUserManualPage(slug: string): Promise<UserManualPageResponse> {
  return request(`/api/usermanual/pages/${encodeURIComponent(slug)}`);
}

export async function searchUserManual(
  q: string,
  limit = 25,
): Promise<UserManualSearchResponse> {
  const query = userManualQuery({ q, limit });
  return request(`/api/usermanual/search?${query}`);
}

export async function listUserManualAccessPoints(): Promise<UserManualAccessPointsResponse> {
  return request("/api/usermanual/access-points");
}

export async function searchLoomGraph(
  workspaceId: string,
  params: LoomGraphSearchQuery,
): Promise<LoomGraphSearchHit[]> {
  const query = new URLSearchParams();
  query.append("q", params.q);
  if (params.sourceKinds && params.sourceKinds.length > 0) {
    query.append("source_kinds", params.sourceKinds.join(","));
  }
  if (params.contentType) query.append("content_type", params.contentType);
  if (params.mime) query.append("mime", params.mime);
  if (params.tagIds && params.tagIds.length > 0) query.append("tag_ids", params.tagIds.join(","));
  if (params.mentionIds && params.mentionIds.length > 0) query.append("mention_ids", params.mentionIds.join(","));
  if (params.backlinkDepth !== undefined) query.append("backlink_depth", String(params.backlinkDepth));
  if (params.caseSensitive !== undefined) query.append("case_sensitive", String(params.caseSensitive));
  if (params.wholeWord !== undefined) query.append("whole_word", String(params.wholeWord));
  if (params.isRegex !== undefined) query.append("regex", String(params.isRegex));
  if (params.path) query.append("path", params.path);
  if (params.limit !== undefined) query.append("limit", String(params.limit));
  if (params.offset !== undefined) query.append("offset", String(params.offset));
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/graph-search?${query.toString()}`,
  );
}

// ===========================================================================
// MT-264: LoomSearchV2 -- Postgres-native, graph-blended hybrid search
// (FTS + pg_trgm + pgvector kNN). Results are references into LoomBlocks.
// ===========================================================================

export type LoomSearchV2Hit = {
  block: LoomBlock;
  score: number;
  fts_rank: number;
  trgm_sim: number;
  vector_sim: number;
  edge_degree: number;
  highlight: string;
};

export type LoomSearchV2Response = {
  hits: LoomSearchV2Hit[];
  content_type_facets: Record<string, number>;
  semantic_available: boolean;
  total: number;
};

export type LoomSearchV2Request = {
  query: string;
  contentType?: LoomBlockContentType;
  tagIds?: string[];
  graphBoost?: number;
  limit?: number;
  offset?: number;
};

export async function loomSearchV2(
  workspaceId: string,
  params: LoomSearchV2Request,
): Promise<LoomSearchV2Response> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/loom/search-v2`, {
    method: "POST",
    body: {
      query: params.query,
      content_type: params.contentType,
      tag_ids: params.tagIds ?? [],
      graph_boost: params.graphBoost ?? 0,
      limit: params.limit ?? 25,
      offset: params.offset ?? 0,
    },
  });
}

export async function listQuickSwitcherRecents(
  workspaceId: string,
  limit = 20,
): Promise<QuickSwitcherRecent[]> {
  const query = new URLSearchParams();
  query.append("limit", String(limit));
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/quick-switcher/recents?${query.toString()}`,
  );
}

export async function recordQuickSwitcherRecent(
  workspaceId: string,
  hit: RecordQuickSwitcherRecentRequest,
): Promise<QuickSwitcherRecent> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/loom/quick-switcher/recents`, {
    method: "POST",
    body: hit,
  });
}

// ===========================================================================
// MT-260: AI Loom jobs (auto-tag / auto-caption / link-suggest). Suggestions
// are PENDING proposals requiring confirm-to-promote.
// ===========================================================================

export type LoomAiJobKind = "auto_tag" | "auto_caption" | "link_suggest";

export type LoomAiReviewState = "pending" | "accepted" | "rejected" | "promoted";

export type LoomAiSuggestion = {
  suggestion_id: string;
  job_id: string;
  workspace_id: string;
  kind: LoomAiJobKind;
  block_id: string;
  target_block_id: string | null;
  suggested_value: Record<string, unknown>;
  model_attribution: Record<string, unknown>;
  prompt_sha256: string;
  output_sha256: string;
  review_state: LoomAiReviewState;
  decided_by: string | null;
  decision_reason: string | null;
  recorded_event_id: string;
  decided_event_id: string | null;
  promotion_requested_event_id: string | null;
  promotion_accepted_event_id: string | null;
  promoted_artifact_ref: string | null;
  value_hash: string;
  created_at_utc: string;
};

export type LoomAiJobResponse = {
  job_id: string;
  kind: string;
  suggestions: LoomAiSuggestion[];
};

export type LoomAiAcceptAllResponse = {
  promoted: string[];
  denied: string[];
  skipped: string[];
};

export type RunLoomAiJobRequest = {
  kind: LoomAiJobKind;
  block_ids: string[];
  tag_candidates?: string[];
};

/**
 * The reviewer identity for accept/reject. Defaults to operator. A model actor
 * here causes the backend to write a durable promotion-denial receipt and the
 * request 403s (per-item authority is enforced server-side, not just in UI).
 */
export type LoomAiReviewerContext = {
  actorKind?: "operator" | "validator" | "local_model" | "cloud_model" | "system";
  actorId?: string;
};

function loomAiReviewerHeaders(ctx?: LoomAiReviewerContext): Record<string, string> {
  return {
    "x-hsk-actor-kind": ctx?.actorKind ?? "operator",
    "x-hsk-actor-id": ctx?.actorId ?? "operator",
  };
}

export async function runLoomAiJob(
  workspaceId: string,
  req: RunLoomAiJobRequest,
): Promise<LoomAiJobResponse> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/loom/ai-jobs`, {
    method: "POST",
    body: req,
  });
}

export async function listLoomAiSuggestions(
  workspaceId: string,
  filters: { jobId?: string; state?: LoomAiReviewState } = {},
): Promise<LoomAiSuggestion[]> {
  const query = new URLSearchParams();
  if (filters.jobId) query.append("job_id", filters.jobId);
  if (filters.state) query.append("state", filters.state);
  const suffix = query.toString() ? `?${query.toString()}` : "";
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/loom/ai-suggestions${suffix}`);
}

export async function acceptLoomAiSuggestion(
  workspaceId: string,
  suggestionId: string,
  ctx?: LoomAiReviewerContext,
  reason?: string,
): Promise<LoomAiSuggestion> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/ai-suggestions/${encodeURIComponent(
      suggestionId,
    )}/accept`,
    { method: "POST", body: { reason }, headers: loomAiReviewerHeaders(ctx) },
  );
}

export async function rejectLoomAiSuggestion(
  workspaceId: string,
  suggestionId: string,
  ctx?: LoomAiReviewerContext,
  reason?: string,
): Promise<LoomAiSuggestion> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/ai-suggestions/${encodeURIComponent(
      suggestionId,
    )}/reject`,
    { method: "POST", body: { reason }, headers: loomAiReviewerHeaders(ctx) },
  );
}

export async function acceptAllLoomAiSuggestions(
  workspaceId: string,
  jobId: string,
  kind?: LoomAiJobKind,
  ctx?: LoomAiReviewerContext,
): Promise<LoomAiAcceptAllResponse> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/ai-jobs/${encodeURIComponent(
      jobId,
    )}/accept-all`,
    { method: "POST", body: { kind }, headers: loomAiReviewerHeaders(ctx) },
  );
}

export async function getLoomWikiProjection(
  workspaceId: string,
  projectionId: string,
): Promise<LoomWikiPageResponse> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/wiki/${encodeURIComponent(projectionId)}`,
  );
}

export async function getWorkbenchLayoutState(
  workspaceId: string,
): Promise<WorkbenchLayoutStateResponse> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/workbench/layout`);
}

export async function saveWorkbenchLayoutState(
  workspaceId: string,
  layoutState: Record<string, unknown>,
): Promise<WorkbenchLayoutStateResponse> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/workbench/layout`, {
    method: "PUT",
    body: { layout_state: layoutState },
  });
}

export async function getWorkspaceSettingsState(
  workspaceId: string,
): Promise<WorkspaceSettingsStateResponse> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/settings`);
}

export async function saveWorkspaceSettingsState(
  workspaceId: string,
  settingsState: Record<string, unknown>,
): Promise<WorkspaceSettingsStateResponse> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/settings`, {
    method: "PUT",
    body: { settings_state: settingsState },
  });
}

// MT-258 durable saved searches. Search bookmarks are workspace-scoped support
// state persisted to PostgreSQL + EventLedger (NOT localStorage); the UI is a
// projection that re-hydrates from this canonical blob.
export type WorkspaceSearchBookmarkStateResponse = {
  workspace_id: string;
  bookmark_state: Record<string, unknown> | null;
  updated_at: string | null;
  event_ledger_event_id: string | null;
};

export async function getWorkspaceSearchBookmarkState(
  workspaceId: string,
): Promise<WorkspaceSearchBookmarkStateResponse> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/search-bookmarks`);
}

export async function saveWorkspaceSearchBookmarkState(
  workspaceId: string,
  bookmarkState: Record<string, unknown>,
): Promise<WorkspaceSearchBookmarkStateResponse> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/search-bookmarks`, {
    method: "PUT",
    body: { bookmark_state: bookmarkState },
  });
}

export async function getLoomBlock(workspaceId: string, blockId: string): Promise<LoomBlock> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/blocks/${encodeURIComponent(blockId)}`,
  );
}

// -- MT-261 CanvasBoard (Loom canvas-class) -------------------------------

export const LOOM_CANVAS_BOARD_SCHEMA_ID = "hsk.loom_canvas_board@1";

export type LoomCanvasBoardState = {
  schema_id: string;
  pan_x: number;
  pan_y: number;
  zoom: number;
};

export type LoomCanvasBoard = {
  block_id: string;
  workspace_id: string;
  board_state: LoomCanvasBoardState;
  created_at: string;
  updated_at: string;
  event_ledger_event_id: string;
};

export type LoomCanvasPlacement = {
  placement_id: string;
  canvas_block_id: string;
  workspace_id: string;
  /** FK to a LoomBlock — the live source of truth, NEVER a content copy. */
  placed_block_id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  z_index: number;
  group_id?: string | null;
  created_at: string;
  updated_at: string;
};

/** A board-local visual-only edge. EXPLICITLY NOT a Loom (semantic) edge. */
export type LoomCanvasVisualEdge = {
  visual_edge_id: string;
  canvas_block_id: string;
  workspace_id: string;
  from_placement_id: string;
  to_placement_id: string;
  label?: string | null;
  created_at: string;
};

export type LoomCanvasBoardView = {
  board: LoomCanvasBoard;
  placements: LoomCanvasPlacement[];
  visual_edges: LoomCanvasVisualEdge[];
};

export function defaultLoomCanvasBoardState(): LoomCanvasBoardState {
  return { schema_id: LOOM_CANVAS_BOARD_SCHEMA_ID, pan_x: 0, pan_y: 0, zoom: 1 };
}

export async function createCanvasBoard(
  workspaceId: string,
  title?: string,
  boardState?: LoomCanvasBoardState,
): Promise<LoomCanvasBoard> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-boards`, {
    method: "POST",
    body: { title, board_state: boardState },
  });
}

export async function getCanvasBoard(
  workspaceId: string,
  blockId: string,
): Promise<LoomCanvasBoardView> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-boards/${encodeURIComponent(blockId)}`,
  );
}

export async function updateCanvasBoardViewport(
  workspaceId: string,
  blockId: string,
  boardState: LoomCanvasBoardState,
): Promise<LoomCanvasBoard> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-boards/${encodeURIComponent(
      blockId,
    )}/viewport`,
    { method: "PUT", body: { board_state: boardState } },
  );
}

export async function placeBlockOnCanvas(
  workspaceId: string,
  blockId: string,
  placed: { placedBlockId: string; x: number; y: number; w: number; h: number; zIndex?: number },
): Promise<LoomCanvasPlacement> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-boards/${encodeURIComponent(
      blockId,
    )}/placements`,
    {
      method: "POST",
      body: {
        placed_block_id: placed.placedBlockId,
        x: placed.x,
        y: placed.y,
        w: placed.w,
        h: placed.h,
        z_index: placed.zIndex,
      },
    },
  );
}

export type CreateCanvasCardResponse = {
  block: LoomBlock;
  rich_document_id: string;
  placement: LoomCanvasPlacement;
};

export async function createCanvasCard(
  workspaceId: string,
  blockId: string,
  card: { title: string; body?: string; x: number; y: number; w: number; h: number },
): Promise<CreateCanvasCardResponse> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-boards/${encodeURIComponent(
      blockId,
    )}/cards`,
    { method: "POST", body: card },
  );
}

export async function updateCanvasPlacement(
  workspaceId: string,
  placementId: string,
  update: {
    x?: number;
    y?: number;
    w?: number;
    h?: number;
    zIndex?: number;
    groupId?: string;
    clearGroup?: boolean;
  },
): Promise<LoomCanvasPlacement> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-placements/${encodeURIComponent(
      placementId,
    )}`,
    {
      method: "PATCH",
      body: {
        x: update.x,
        y: update.y,
        w: update.w,
        h: update.h,
        z_index: update.zIndex,
        group_id: update.groupId,
        clear_group: update.clearGroup,
      },
    },
  );
}

export async function removeCanvasPlacement(
  workspaceId: string,
  placementId: string,
): Promise<void> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-placements/${encodeURIComponent(
      placementId,
    )}`,
    { method: "DELETE" },
  );
}

export async function addCanvasVisualEdge(
  workspaceId: string,
  blockId: string,
  edge: { fromPlacementId: string; toPlacementId: string; label?: string },
): Promise<LoomCanvasVisualEdge> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-boards/${encodeURIComponent(
      blockId,
    )}/visual-edges`,
    {
      method: "POST",
      body: {
        from_placement_id: edge.fromPlacementId,
        to_placement_id: edge.toPlacementId,
        label: edge.label,
      },
    },
  );
}

export async function removeCanvasVisualEdge(
  workspaceId: string,
  visualEdgeId: string,
): Promise<void> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/canvas-visual-edges/${encodeURIComponent(
      visualEdgeId,
    )}`,
    { method: "DELETE" },
  );
}

export type LoomEdge = {
  edge_id: string;
  workspace_id: string;
  source_block_id: string;
  target_block_id: string;
  edge_type: string;
  created_by: string;
  created_at: string;
};

/**
 * Create a SEMANTIC Loom edge (real graph authority). The canvas board delegates
 * to this for semantic connections; visual-only connections use
 * `addCanvasVisualEdge` instead.
 */
export async function createLoomEdge(
  workspaceId: string,
  edge: {
    sourceBlockId: string;
    targetBlockId: string;
    edgeType?: "mention" | "tag" | "sub_tag" | "parent" | "ai_suggested";
  },
): Promise<LoomEdge> {
  return request(`/workspaces/${encodeURIComponent(workspaceId)}/loom/edges`, {
    method: "POST",
    body: {
      source_block_id: edge.sourceBlockId,
      target_block_id: edge.targetBlockId,
      edge_type: edge.edgeType ?? "mention",
      created_by: "user",
    },
  });
}

export async function getLoomBlockTransclusion(
  workspaceId: string,
  blockId: string,
): Promise<LoomBlockTransclusion> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/blocks/${encodeURIComponent(blockId)}/transclusion`,
  );
}

export async function queryLoomView(
  workspaceId: string,
  viewType: LoomViewType,
  params: LoomViewQuery = {},
): Promise<LoomViewResponse> {
  const query = new URLSearchParams();
  if (params.contentType) query.append("content_type", params.contentType);
  if (params.mime) query.append("mime", params.mime);
  if (params.tagIds && params.tagIds.length > 0) query.append("tag_ids", params.tagIds.join(","));
  if (params.mentionIds && params.mentionIds.length > 0) {
    query.append("mention_ids", params.mentionIds.join(","));
  }
  if (params.limit !== undefined) query.append("limit", String(params.limit));
  if (params.offset !== undefined) query.append("offset", String(params.offset));
  const suffix = query.toString();
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/views/${encodeURIComponent(viewType)}${
      suffix ? `?${suffix}` : ""
    }`,
  );
}

export async function updateLoomBlock(
  workspaceId: string,
  blockId: string,
  update: LoomBlockUpdateRequest,
): Promise<LoomBlock> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/blocks/${encodeURIComponent(blockId)}`,
    { method: "PATCH", body: update },
  );
}

// MT-262 BlockCollectionViews API --------------------------------------------

/** Create a saved view (a typed view_def LoomBlock + bridge + receipt). */
export async function createBlockView(
  workspaceId: string,
  definition: BlockViewDefinition,
  options: { title?: string; blockId?: string } = {},
): Promise<BlockViewRecord> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/views/definitions`,
    {
      method: "POST",
      body: {
        block_id: options.blockId,
        title: options.title,
        definition,
      },
    },
  );
}

export async function getBlockView(
  workspaceId: string,
  blockId: string,
): Promise<BlockViewRecord> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/views/definitions/${encodeURIComponent(blockId)}`,
  );
}

/** Persist a new definition for a saved view (e.g. a re-sort), to PostgreSQL. */
export async function updateBlockView(
  workspaceId: string,
  blockId: string,
  definition: BlockViewDefinition,
): Promise<BlockViewRecord> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/views/definitions/${encodeURIComponent(blockId)}`,
    { method: "PATCH", body: { definition } },
  );
}

/** Execute a saved view against the REAL Loom query backend (SQL-side). */
export async function queryBlockViewResults(
  workspaceId: string,
  blockId: string,
  params: { limit?: number; offset?: number } = {},
): Promise<BlockViewResults> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/views/definitions/${encodeURIComponent(blockId)}/results`,
    { method: "POST", body: { limit: params.limit, offset: params.offset } },
  );
}

export async function setLoomBlockPinOrder(
  workspaceId: string,
  blockId: string,
  pinOrder: number | null,
): Promise<LoomBlock> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/blocks/${encodeURIComponent(blockId)}/pin-order`,
    { method: "PUT", body: { pin_order: pinOrder } },
  );
}

export async function openDailyJournal(
  workspaceId: string,
  journalDate: string,
): Promise<LoomBlock> {
  return request(
    `/workspaces/${encodeURIComponent(workspaceId)}/loom/journals/${encodeURIComponent(journalDate)}`,
    { method: "PUT" },
  );
}

export type SourceControlStatusCode =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "copied"
  | "untracked"
  | "ignored"
  | "unmerged"
  | "unknown";

export type SourceControlDiffScope = "worktree" | "staged";

export type SourceControlStatusEntry = {
  path: string;
  index: SourceControlStatusCode | null;
  worktree: SourceControlStatusCode | null;
};

export type SourceControlStatus = {
  repo_root: string;
  branch: string | null;
  entries: SourceControlStatusEntry[];
};

export type SourceControlDiff = {
  path: string;
  scope: SourceControlDiffScope;
  patch: string;
};

export type SourceControlReceipt = {
  operation: string;
  paths: string[];
  event_ledger_event_id?: string | null;
};

export type SourceControlCommit = {
  id: string;
  message: string;
  event_ledger_event_id?: string | null;
};

export type SourceControlLogEntry = {
  id: string;
  author: string;
  timestamp: number;
  message: string;
};

export type SourceControlLog = {
  entries: SourceControlLogEntry[];
};

export type SourceControlBranch = {
  name: string;
  current: boolean;
  commit_id: string;
};

export type SourceControlBlameLine = {
  line_number: number;
  commit_id: string;
  author: string;
  content: string;
};

export type SourceControlBlame = {
  path: string;
  lines: SourceControlBlameLine[];
};

function sourceControlRepoQuery(repoPath: string): URLSearchParams {
  return new URLSearchParams({ repo_path: repoPath });
}

export async function getSourceControlStatus(repoPath: string): Promise<SourceControlStatus> {
  const query = sourceControlRepoQuery(repoPath);
  return request(`/source-control/status?${query.toString()}`);
}

export async function getSourceControlDiff(
  repoPath: string,
  path: string,
  scope: SourceControlDiffScope,
): Promise<SourceControlDiff> {
  const query = sourceControlRepoQuery(repoPath);
  query.append("path", path);
  query.append("scope", scope);
  return request(`/source-control/diff?${query.toString()}`);
}

export async function stageSourceControlPaths(
  repoPath: string,
  paths: string[],
): Promise<SourceControlReceipt> {
  return request("/source-control/stage", {
    method: "POST",
    body: { repo_path: repoPath, paths },
  });
}

export async function unstageSourceControlPaths(
  repoPath: string,
  paths: string[],
): Promise<SourceControlReceipt> {
  return request("/source-control/unstage", {
    method: "POST",
    body: { repo_path: repoPath, paths },
  });
}

export async function discardSourceControlPaths(
  repoPath: string,
  paths: string[],
  confirmed: boolean,
): Promise<SourceControlReceipt> {
  return request("/source-control/discard", {
    method: "POST",
    body: { repo_path: repoPath, paths, confirmed },
  });
}

export async function commitSourceControl(
  repoPath: string,
  message: string,
): Promise<SourceControlCommit> {
  return request("/source-control/commit", {
    method: "POST",
    body: { repo_path: repoPath, message },
  });
}

export async function listSourceControlBranches(repoPath: string): Promise<SourceControlBranch[]> {
  const query = sourceControlRepoQuery(repoPath);
  return request(`/source-control/branches?${query.toString()}`);
}

export async function createSourceControlBranch(
  repoPath: string,
  name: string,
): Promise<SourceControlReceipt> {
  return request("/source-control/branches", {
    method: "POST",
    body: { repo_path: repoPath, name },
  });
}

export async function switchSourceControlBranch(
  repoPath: string,
  name: string,
): Promise<SourceControlReceipt> {
  return request("/source-control/switch", {
    method: "POST",
    body: { repo_path: repoPath, name },
  });
}

export async function getSourceControlLog(
  repoPath: string,
  limit?: number,
): Promise<SourceControlLog> {
  const query = sourceControlRepoQuery(repoPath);
  if (limit !== undefined) query.append("limit", String(limit));
  return request(`/source-control/log?${query.toString()}`);
}

export async function getSourceControlBlame(
  repoPath: string,
  path: string,
): Promise<SourceControlBlame> {
  const query = sourceControlRepoQuery(repoPath);
  query.append("path", path);
  return request(`/source-control/blame?${query.toString()}`);
}

export async function getCodeSymbol(symbolEntityId: string): Promise<CodeSymbolResponse> {
  return request(`/knowledge/code/symbols/${encodeURIComponent(symbolEntityId)}`, {
    headers: codeNavHeaders(`symbol-${symbolEntityId}`),
  });
}

export async function lookupCodeSymbols(query: CodeSymbolLookupQuery): Promise<CodeSymbolLookupResponse> {
  const params = new URLSearchParams({ workspace_id: query.workspaceId });
  if (query.name) params.append("name", query.name);
  if (query.prefix) params.append("prefix", query.prefix);
  if (query.path) params.append("path", query.path);
  if (query.limit !== undefined) params.append("limit", String(query.limit));
  return request(`/knowledge/code/symbols?${params.toString()}`, {
    headers: codeNavHeaders(`lookup-${query.workspaceId}`),
  });
}

export async function getCodeSymbolReferences(symbolEntityId: string): Promise<CodeSymbolReferencesResponse> {
  return request(`/knowledge/code/symbols/${encodeURIComponent(symbolEntityId)}/references`, {
    headers: codeNavHeaders(`references-${symbolEntityId}`),
  });
}

export async function getCodeFileLens(
  workspaceId: string,
  relativePath: string,
  contentHash: string,
  parserVersion: string,
): Promise<CodeFileLensResponse> {
  const params = new URLSearchParams({
    workspace_id: workspaceId,
    content_hash: contentHash,
    parser_version: parserVersion,
  });
  return request(`/knowledge/code/files/${encodeURIComponent(relativePath)}/lens?${params.toString()}`, {
    headers: codeNavHeaders(`lens-${workspaceId}`),
  });
}

export async function getAtelierRoles(): Promise<AtelierRolesResponse> {
  return request("/api/atelier/roles");
}

export async function applyAtelierPatchsets(
  documentId: string,
  body: AtelierApplyRequestV1,
  ctx?: WriteContext,
): Promise<Block[]> {
  return request(`/documents/${encodeURIComponent(documentId)}/atelier/apply`, {
    method: "POST",
    body,
    headers: writeContextHeaders(ctx),
  });
}

export async function updateDocumentBlocks(
  documentId: string,
  blocks: BlockInput[],
  ctx?: WriteContext,
): Promise<Block[]> {
  return request(`/documents/${encodeURIComponent(documentId)}/blocks`, {
    method: "PUT",
    body: { blocks },
    headers: writeContextHeaders(ctx),
  });
}

export async function updateCanvasGraph(
  canvasId: string,
  nodes: CanvasNodeInput[],
  edges: CanvasEdgeInput[],
  ctx?: WriteContext,
): Promise<CanvasWithGraph> {
  return request(`/canvases/${encodeURIComponent(canvasId)}`, {
    method: "PUT",
    body: { nodes, edges },
    headers: writeContextHeaders(ctx),
  });
}

export async function getLogTail(limit = 200): Promise<LogTailResponse> {
  const url = `/logs/tail?limit=${limit}`;
  return request(url);
}

export async function getHealth(): Promise<HealthResponse> {
  return request("/health");
}

export async function getKernelDccProjection(): Promise<KernelDccProjectionSurfaceV1> {
  const surface = await request<KernelDccProjectionSurfaceV1>("/api/kernel/dcc_projection");
  if (surface.spawn_tree_projection || surface.sessions.length === 0) {
    return surface;
  }
  const runtimeRecords = surface.session_spawn_runtime_records ?? [];
  if (runtimeRecords.length === 0) {
    return {
      ...surface,
      spawn_tree_projection: null,
    };
  }

  return {
    ...surface,
    spawn_tree_projection: await projectKernelSessionSpawnTreeDcc(
      buildSessionSpawnTreeDccRequest(surface, runtimeRecords),
    ),
  };
}

export async function triggerKernelDccAction(
  input: KernelDccActionTriggerRequestV1,
): Promise<KernelDccActionTriggerResponseV1> {
  return request("/api/kernel/dcc_actions/trigger", { method: "POST", body: input });
}

export async function projectKernelSessionSpawnTreeDcc(
  input: SessionSpawnTreeDccRequestV1,
): Promise<SessionSpawnTreeDccProjectionV1> {
  return request("/api/kernel/session_spawn_tree_dcc_projection", { method: "POST", body: input });
}

function buildSessionSpawnTreeDccRequest(
  surface: KernelDccProjectionSurfaceV1,
  runtimeRecords: SessionSpawnRuntimeRecordV1[],
): SessionSpawnTreeDccRequestV1 {
  return {
    schema_id: "hsk.kernel.session_spawn_tree_dcc@1",
    tree_id: `${surface.surface_id}.session-spawn-tree`,
    folded_stub_ids: ["WP-1-Session-Spawn-Tree-DCC-Visualization-v1"],
    panel_id: "session-spawn-tree",
    visible_fields: [
      "SpawnHierarchy",
      "ChildCounts",
      "SpawnDepth",
      "CascadeCancel",
      "SpawnMode",
      "AnnounceBackBadges",
    ],
    runtime_records: runtimeRecords,
    product_authority_refs: [
      "kernel.dcc_mvp_runtime_surface",
      "kernel.role_mailbox_inbox_evidence_bridge",
      "kernel.session_anti_pattern_registry",
      "flight_recorder.session_spawn",
    ],
    folded_source_refs: [
      ".GOV/task_packets/stubs/WP-1-Session-Spawn-Tree-DCC-Visualization-v1.contract.json",
    ],
  };
}

export type FlightEventFilters = {
  eventId?: string;
  jobId?: string;
  traceId?: string;
  from?: string;
  to?: string;
  actor?: "human" | "agent" | "system";
  surface?: string;
  eventType?: FlightEvent["event_type"];
  wsid?: string;
};

export async function getEvents(filters?: FlightEventFilters): Promise<FlightEvent[]> {
  const toIso = (value: string) => {
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toISOString();
  };

  const params = new URLSearchParams();
  if (filters?.eventId) params.append("event_id", filters.eventId);
  if (filters?.jobId) params.append("job_id", filters.jobId);
  if (filters?.traceId) params.append("trace_id", filters.traceId);
  if (filters?.from) params.append("from", toIso(filters.from));
  if (filters?.to) params.append("to", toIso(filters.to));
  if (filters?.actor) params.append("actor", filters.actor);
  if (filters?.surface) params.append("surface", filters.surface);
  if (filters?.eventType) params.append("event_type", filters.eventType);
  if (filters?.wsid) params.append("wsid", filters.wsid);

  const query = params.toString();
  const path = query.length > 0 ? `/api/flight_recorder?${query}` : "/api/flight_recorder";
  return request(path);
}

export async function recordRuntimeChatEvent(event: RuntimeChatEventV0_1): Promise<void> {
  await request("/api/flight_recorder/runtime_chat_event", { method: "POST", body: event });
}

export type AiJob = {
  job_id: string;
  trace_id: string;
  workflow_run_id?: string | null;
  job_kind: string;
  state: string;
  error_message?: string | null;
  protocol_id: string;
  profile_id: string;
  capability_profile_id: string;
  access_mode: string;
  safety_mode: string;
  entity_refs: { entity_id: string; entity_kind: string }[];
  planned_operations: {
    op_type: string;
    target: { entity_id: string; entity_kind: string };
    description?: string | null;
  }[];
  metrics: {
    duration_ms: number;
    total_tokens: number;
    input_tokens: number;
    output_tokens: number;
    tokens_planner: number;
    tokens_executor: number;
    entities_read: number;
    entities_written: number;
    validators_run_count: number;
  };
  status_reason: string;
  job_inputs?: unknown;
  job_outputs?: unknown;
  created_at: string;
  updated_at: string;
};

export type FemsJobOutput = {
  schema_version: "hsk.fems.result@0.1";
  protocol_id: FemsProtocolId;
  memory_policy: "EPHEMERAL" | "SESSION_SCOPED" | "WORKSPACE_SCOPED";
  memory_policy_requested?: "EPHEMERAL" | "SESSION_SCOPED" | "WORKSPACE_SCOPED";
  memory_state_ref?: string | null;
  memory_session?: {
    memory_policy_requested: "EPHEMERAL" | "SESSION_SCOPED" | "WORKSPACE_SCOPED";
    memory_policy_effective: "EPHEMERAL" | "SESSION_SCOPED" | "WORKSPACE_SCOPED";
    memory_state_ref?: string | null;
    server_enforced: boolean;
    cloud_consent_granted: boolean;
  };
  proposal?: Record<string, unknown>;
  proposal_hash?: string;
  commit_report?: Record<string, unknown>;
  commit_report_hash?: string;
  memory_pack?: Record<string, unknown> | null;
  memory_pack_hash?: string | null;
  review?: {
    status: string;
    required_ops: number;
    reviewer_kind?: "user" | "policy";
    disable_memory?: boolean;
    disable_memory_allowed?: boolean;
  };
  memory_browser?: {
    items: Array<Record<string, unknown>>;
  };
  warning?: string;
};

export function asFemsJobOutput(value: unknown): FemsJobOutput | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (record.schema_version !== "hsk.fems.result@0.1") return null;
  const protocol_id = record.protocol_id;
  if (typeof protocol_id !== "string" || !isFemsProtocolId(protocol_id)) return null;
  return record as FemsJobOutput;
}

export async function createJob(
  jobKind: string,
  protocolId: string,
  docId?: string,
  jobInputs?: unknown,
): Promise<WorkflowRun> {
  const body: Record<string, unknown> = {
    job_kind: jobKind,
    protocol_id: protocolId,
  };

  if (docId) body.doc_id = docId;
  const shouldAttachViewMode =
    jobKind === "doc_edit" || jobKind === "doc_summarize" || jobKind === "doc_rewrite";
  const viewMode = shouldAttachViewMode ? loadViewModeFromStorage() : undefined;

  let effectiveJobInputs = jobInputs;
  if (shouldAttachViewMode && viewMode) {
    if (jobInputs == null) {
      effectiveJobInputs = { view_mode: viewMode };
    } else if (typeof jobInputs === "object" && !Array.isArray(jobInputs)) {
      const record = jobInputs as Record<string, unknown>;
      if (record.view_mode === undefined) {
        effectiveJobInputs = { ...record, view_mode: viewMode };
      }
    }
  }

  if (effectiveJobInputs !== undefined) body.job_inputs = effectiveJobInputs;

  return request("/api/jobs", {
    method: "POST",
    body,
  });
}

export async function createFemsJob(
  protocolId: FemsProtocolId,
  jobInputs?: unknown,
): Promise<WorkflowRun> {
  return createJob(protocolId, protocolId, undefined, jobInputs);
}

export async function getJob(jobId: string): Promise<AiJob> {
  return request(`/api/jobs/${encodeURIComponent(jobId)}`);
}

export type ListJobsFilters = {
  status?: string;
  job_kind?: string;
  wsid?: string;
  from?: string;
  to?: string;
};

export async function listJobs(filters?: ListJobsFilters): Promise<AiJob[]> {
  const params = new URLSearchParams();
  if (filters?.status) params.append("status", filters.status);
  if (filters?.job_kind) params.append("job_kind", filters.job_kind);
  if (filters?.wsid) params.append("wsid", filters.wsid);
  if (filters?.from) params.append("from", new Date(filters.from).toISOString());
  if (filters?.to) params.append("to", new Date(filters.to).toISOString());

  const query = params.toString();
  const path = query.length > 0 ? `/api/jobs?${query}` : "/api/jobs";
  return request(path);
}

export type CloudEscalationUiSurface = "cloud_escalation_modal" | "settings" | "operator_console";

export type CloudEscalationConsentInput = {
  request_id: string;
  approved: boolean;
  user_id: string;
  ui_surface?: CloudEscalationUiSurface;
  notes?: string;
};

export async function submitCloudEscalationConsent(
  jobId: string,
  input: CloudEscalationConsentInput,
): Promise<{ status: string }> {
  return request(`/api/jobs/${encodeURIComponent(jobId)}/cloud_escalation/consent`, {
    method: "POST",
    body: input,
  });
}

export async function resumeJob(jobId: string): Promise<WorkflowRun> {
  return request(`/api/jobs/${encodeURIComponent(jobId)}/resume`, { method: "POST" });
}

// Debug Bundle types
export type BundleScopeInput =
  | { kind: "problem"; problem_id: string }
  | { kind: "job"; job_id: string }
  | { kind: "time_window"; time_range: { start: string; end: string }; wsid?: string }
  | { kind: "workspace"; wsid: string };

export type BundleExportRequest = {
  scope: BundleScopeInput;
  redaction_mode: "SAFE_DEFAULT" | "WORKSPACE" | "FULL_LOCAL";
};

export type BundleExportResponse = {
  export_job_id: string;
  status: "queued" | "running";
  estimated_size_bytes?: number | null;
};

export type BundleStatus = {
  bundle_id: string;
  status: "pending" | "ready" | "expired" | "failed";
  manifest?: unknown;
  error?: string | null;
  expires_at?: string | null;
};

export type BundleValidationFinding = {
  severity: "Error" | "Warning" | "Info";
  code: string;
  message: string;
  file?: string | null;
  path?: string | null;
};

export type BundleValidationResponse = {
  valid: boolean;
  findings: BundleValidationFinding[];
};

export async function exportDebugBundle(input: BundleExportRequest): Promise<BundleExportResponse> {
  return request("/api/bundles/debug/export", { method: "POST", body: input });
}

export async function getBundleStatus(bundleId: string): Promise<BundleStatus> {
  return request(`/api/bundles/debug/${encodeURIComponent(bundleId)}`);
}

export async function validateBundle(bundleId: string): Promise<BundleValidationResponse> {
  return request(`/api/bundles/debug/${encodeURIComponent(bundleId)}/validate`, { method: "POST" });
}

export async function downloadBundle(bundleId: string): Promise<Blob> {
  const response = await fetch(`${BASE_URL}/api/bundles/debug/${encodeURIComponent(bundleId)}/download`);
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Download failed: ${text}`);
  }
  return response.blob();
}

// Governance Pack export types
export type ExportTarget = { type: "local_file"; path: string };

export type GovernancePackInvariants = {
  project_code: string;
  project_display_name: string;
  project_prefix?: string;
  issue_prefix: string;
  language_layout_profile_id: string;
  frontend_root_dir: string;
  frontend_src_dir: string;
  backend_root_dir: string;
  backend_crate_name: string;
  codex_version?: string;
  master_spec_filename?: string;
  cargo_target_dir_name?: string;
  node_package_manager?: string;
  default_base_branch?: string;
  additional_placeholders?: Record<string, string>;
};

export type GovernancePackExportRequest = {
  export_target: ExportTarget;
  overwrite?: boolean;
  invariants: GovernancePackInvariants;
};

export type GovernancePackExportResponse = {
  export_job_id: string;
  status: "queued" | "running";
};

export async function exportGovernancePack(
  input: GovernancePackExportRequest,
): Promise<GovernancePackExportResponse> {
  return request("/api/governance_pack/export", { method: "POST", body: input });
}

// WP-KERNEL-005 Atelier domain
export type AtelierOverview = {
  tables: { name: string; rows: number }[];
  event_families: { family: string; count: number }[];
};

export type AtelierIntakeBatchMode = "manual" | "folder_scan" | "sourcing_run";
export type AtelierIntakeProfileMode = "loose_profile" | "character_linked";

export type AtelierIntakeBatch = {
  batch_id: string;
  idempotency_key: string;
  source_label: string;
  source_ref: string;
  mode: AtelierIntakeBatchMode;
  profile_mode: AtelierIntakeProfileMode;
  target_character_id: string | null;
  target_sheet_version_id: string | null;
  target_collection_id: string | null;
  status: string;
  resume_cursor: string | null;
  resumed_at_utc: string | null;
  created_at_utc: string;
};

export type OpenAtelierIntakeBatchRequest = {
  idempotency_key: string;
  source_label: string;
  source_ref?: string | null;
  mode?: AtelierIntakeBatchMode;
  profile_mode?: AtelierIntakeProfileMode;
  target_character_id?: string | null;
  target_sheet_version_id?: string | null;
  target_collection_id?: string | null;
  resume_cursor?: string | null;
};

export type AtelierIntakeItems = {
  lane_counts: {
    new: number;
    accepted: number;
    rejected: number;
    deferred: number;
  };
  items: {
    item_id: string;
    source_path: string;
    file_name: string;
    lane: string;
    byte_len: number;
  }[];
};

export type AtelierCommandCorpusEntry = {
  entry_id: string;
  action_id: string;
  owner: string;
  execution_class: string;
  foreground_flag: boolean;
  manual_anchor: string;
};

export type AtelierStealthWindow = {
  window_ref_id: string;
  owner_actor: string;
  title: string;
  visibility: string;
  status: string;
  revision: number;
};

export type AtelierAiTagSuggestion = {
  suggestion_id: string;
  character_internal_id: string;
  asset_id: string | null;
  tag_text: string;
  confidence: number | null;
  model_receipt_ref: string;
  tool_receipt_ref: string;
  suggested_by: string;
  status: "proposed" | "accepted" | "rejected" | "applied";
  decided_by: string | null;
  decision_reason: string | null;
  applied_tag_id: string | null;
  created_at_utc: string;
  updated_at_utc: string;
};

export type RecordAtelierAiTagSuggestionRequest = {
  character_internal_id: string;
  asset_id?: string | null;
  tag_text: string;
  confidence?: number | null;
  model_receipt_ref: string;
  tool_receipt_ref: string;
  suggested_by: string;
};

export type DecideAtelierAiTagSuggestionRequest = {
  reason?: string | null;
};

export type AtelierFilesystemHealthFinding = {
  finding_id: string;
  check_id: string;
  finding_kind:
    | "missing_original"
    | "missing_thumbnail"
    | "inbox_pending"
    | "untracked_original"
    | "sidecar_visibility_anomaly";
  target_type: string;
  target_id: string;
  details: Record<string, unknown>;
  created_at_utc: string;
};

export type AtelierFilesystemHealthReport = {
  check: {
    check_id: string;
    requested_by: string;
    scope_label: string | null;
    summary: Record<string, unknown>;
    created_at_utc: string;
  };
  findings: AtelierFilesystemHealthFinding[];
};

export type AtelierDeletionTargetKind = "media_asset" | "sheet_version";

export type AtelierDeletionTarget = {
  target_type: AtelierDeletionTargetKind;
  target_id: string;
};

export type AtelierDeletionControlsRequest = {
  targets: AtelierDeletionTarget[];
  reason: string;
};

export type AtelierDeletionImpactTarget = AtelierDeletionTarget & {
  currently_archived: boolean;
  would_archive: boolean;
};

export type AtelierDeletionImpactPreview = {
  requested_by: string;
  reason: string;
  target_count: number;
  would_archive_count: number;
  already_archived_count: number;
  targets: AtelierDeletionImpactTarget[];
};

export type AtelierBulkOperationReceipt = {
  receipt_id: string;
  operation: string;
  requested_by: string;
  target_count: number;
  mutation_count: number;
  status: string;
  payload: Record<string, unknown>;
  created_at_utc: string;
};

export type AtelierClipboardImageImportRequest = {
  idempotency_key: string;
  mime: "image/png" | "image/jpeg" | "image/webp";
  content_hash: string;
  byte_len: number;
  artifact_ref: string;
  source_application?: string | null;
};

export type AtelierUrlImageImportRequest = {
  idempotency_key: string;
  source_url: string;
  expected_mime?: "image/png" | "image/jpeg" | "image/webp" | null;
  source_label?: string | null;
  capability_profile_id: string;
  capability_grant_ref: string;
};

export type AtelierImageImportRecord = {
  import_id: string;
  idempotency_key: string;
  source_kind: "clipboard" | "url";
  status: "materialized" | "queued";
  requested_by: string;
  normalized_url?: string | null;
  source_url_hash: string;
  source_host?: string | null;
  source_label?: string | null;
  expected_mime?: string | null;
  capability_profile_id?: string | null;
  capability_grant_ref?: string | null;
  required_capabilities: unknown;
  asset_id?: string | null;
  artifact_ref?: string | null;
  source_provenance: string;
  preflight: Record<string, unknown>;
  created_at_utc: string;
  updated_at_utc: string;
};

export async function getAtelierOverview(): Promise<AtelierOverview> {
  return request("/atelier/overview");
}

export async function listAtelierIntakeBatches(): Promise<AtelierIntakeBatch[]> {
  return request("/atelier/intake/batches");
}

export async function openAtelierIntakeBatch(
  input: OpenAtelierIntakeBatchRequest,
): Promise<AtelierIntakeBatch>;
// eslint-disable-next-line no-redeclare
export async function openAtelierIntakeBatch(
  idempotencyKey: string,
  sourceLabel: string,
): Promise<AtelierIntakeBatch>;
// eslint-disable-next-line no-redeclare
export async function openAtelierIntakeBatch(
  inputOrKey: OpenAtelierIntakeBatchRequest | string,
  sourceLabel?: string,
): Promise<AtelierIntakeBatch> {
  const body =
    typeof inputOrKey === "string"
      ? { idempotency_key: inputOrKey, source_label: sourceLabel }
      : inputOrKey;
  return request("/atelier/intake/batches", {
    method: "POST",
    body,
  });
}

export async function getAtelierIntakeItems(batchId: string): Promise<AtelierIntakeItems> {
  return request(`/atelier/intake/batches/${encodeURIComponent(batchId)}/items`);
}

export async function listAtelierCommandCorpus(): Promise<AtelierCommandCorpusEntry[]> {
  return request("/atelier/command-corpus");
}

export async function runAtelierFilesystemHealthCheck(
  ctx: WriteContext,
  input: { scope_label?: string | null } = {},
): Promise<AtelierFilesystemHealthReport> {
  return request("/atelier/filesystem-health/checks", {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function listAtelierFilesystemHealthFindings(
  checkId: string,
): Promise<AtelierFilesystemHealthFinding[]> {
  return request(
    `/atelier/filesystem-health/checks/${encodeURIComponent(checkId)}/findings`,
  );
}

export async function previewAtelierDeletionImpact(
  ctx: WriteContext,
  input: AtelierDeletionControlsRequest,
): Promise<AtelierDeletionImpactPreview> {
  return request("/atelier/deletion/impact-preview", {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function archiveAtelierDeletionTargets(
  ctx: WriteContext,
  input: AtelierDeletionControlsRequest,
): Promise<AtelierBulkOperationReceipt> {
  return request("/atelier/deletion/archive", {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function restoreAtelierDeletionTargets(
  ctx: WriteContext,
  input: AtelierDeletionControlsRequest,
): Promise<AtelierBulkOperationReceipt> {
  return request("/atelier/deletion/restore", {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function importAtelierClipboardImage(
  ctx: WriteContext,
  input: AtelierClipboardImageImportRequest,
): Promise<AtelierImageImportRecord> {
  return request("/atelier/image-import/clipboard", {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function recordAtelierUrlImageImport(
  ctx: WriteContext,
  input: AtelierUrlImageImportRequest,
): Promise<AtelierImageImportRecord> {
  return request("/atelier/image-import/url", {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function recordAtelierAiTagSuggestion(
  input: RecordAtelierAiTagSuggestionRequest,
): Promise<AtelierAiTagSuggestion> {
  return request("/atelier/ai-tag-suggestions", { method: "POST", body: input });
}

export async function listAtelierAiTagSuggestionsForCharacter(
  characterInternalId: string,
): Promise<AtelierAiTagSuggestion[]> {
  return request(
    `/atelier/ai-tag-suggestions/characters/${encodeURIComponent(characterInternalId)}`,
  );
}

export async function acceptAtelierAiTagSuggestion(
  suggestionId: string,
  ctx: WriteContext,
  input: DecideAtelierAiTagSuggestionRequest = {},
): Promise<AtelierAiTagSuggestion> {
  return request(`/atelier/ai-tag-suggestions/${encodeURIComponent(suggestionId)}/accept`, {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function rejectAtelierAiTagSuggestion(
  suggestionId: string,
  ctx: WriteContext,
  input: DecideAtelierAiTagSuggestionRequest = {},
): Promise<AtelierAiTagSuggestion> {
  return request(`/atelier/ai-tag-suggestions/${encodeURIComponent(suggestionId)}/reject`, {
    method: "POST",
    headers: writeContextHeaders(ctx),
    body: input,
  });
}

export async function applyAtelierAiTagSuggestion(
  suggestionId: string,
  ctx: WriteContext,
): Promise<AtelierAiTagSuggestion> {
  return request(`/atelier/ai-tag-suggestions/${encodeURIComponent(suggestionId)}/apply`, {
    method: "POST",
    headers: writeContextHeaders(ctx),
  });
}

export async function listAtelierStealthWindows(
  ctx: WriteContext,
): Promise<AtelierStealthWindow[]> {
  return request("/atelier/stealth/windows", {
    headers: writeContextHeaders(ctx),
  });
}

// ---------------------------------------------------------------------------
// WP-KERNEL-009 RichDocumentCore (MT-145..MT-160): the rich-document authority
// API client. These call the REAL backend authority (knowledge_rich_documents
// via /knowledge/documents) — NOT the legacy /documents+/blocks surface — so
// the editor's document MODEL works end-to-end with no mocks.
//
// Every rich-document request requires the backend-navigation identity headers
// (actor + kernel/session run ids); the actor-kind drives the server-enforced
// MT-158 permission boundary.
// ---------------------------------------------------------------------------

export type RichDocActorKind =
  | "operator"
  | "local_model"
  | "cloud_model"
  | "validator"
  | "system";

export type RichDocContext = {
  actor_id: string;
  kernel_task_run_id: string;
  session_run_id: string;
  actor_kind?: RichDocActorKind;
  correlation_id?: string;
};

/** A stable, default operator identity for editor-driven document calls. */
export const DEFAULT_RICH_DOC_CONTEXT: RichDocContext = {
  actor_id: "operator",
  kernel_task_run_id: "KTR-EDITOR-UI",
  session_run_id: "SR-EDITOR-UI",
  actor_kind: "operator",
};

function richDocHeaders(ctx: RichDocContext): Record<string, string> {
  const headers: Record<string, string> = {
    "x-hsk-actor-id": ctx.actor_id,
    "x-hsk-kernel-task-run-id": ctx.kernel_task_run_id,
    "x-hsk-session-run-id": ctx.session_run_id,
  };
  if (ctx.actor_kind) headers["x-hsk-actor-kind"] = ctx.actor_kind;
  if (ctx.correlation_id) headers["x-hsk-correlation-id"] = ctx.correlation_id;
  return headers;
}

export type RichDocument = {
  rich_document_id: string;
  workspace_id: string;
  document_id: string | null;
  title: string;
  schema_version: string;
  doc_version: number;
  content_json: JSONContentLike;
  content_sha256: string;
  crdt_document_id: string | null;
  crdt_snapshot_id: string | null;
  promotion_receipt_event_id: string | null;
  projection_refs: unknown;
  project_ref: string | null;
  folder_ref: string | null;
  authority_label: string;
  owner_actor_kind: string | null;
  owner_actor_id: string | null;
  created_at: string;
  updated_at: string;
};

/** A loosely typed ProseMirror doc-node JSON (mirrors @tiptap/core JSONContent). */
export type JSONContentLike = {
  type?: string;
  attrs?: Record<string, unknown>;
  content?: JSONContentLike[];
  text?: string;
  [key: string]: unknown;
};

export type RichDocBlock = {
  block_id: string;
  kind: string;
  heading_level: number | null;
  sequence: number;
  content: {
    raw: JSONContentLike;
    derived: { plain_text: string; word_count: number; preview: string };
    display: unknown;
  };
};

export type RichDocTree = {
  schema_version: string;
  schema_matches: boolean;
  block_ids: string[];
  blocks: RichDocBlock[];
};

export type RichDocLoad = {
  document: RichDocument;
  tree: RichDocTree;
  code_nodes: unknown[];
};

export type RichDocSaveResult = {
  document: RichDocument;
  save_receipt_event_id: string;
  backlinks_persisted: number;
  backlinks_skipped_reason: string | null;
};

export type RichDocSaveMetadata = {
  crdt_document_id?: string | null;
  crdt_snapshot_id?: string | null;
  promotion_receipt_event_id?: string | null;
};

export type RichDocumentDraft = {
  rich_document_id: string;
  workspace_id: string;
  base_doc_version: number;
  base_content_sha256: string;
  draft_content_json: JSONContentLike;
  draft_content_sha256: string;
  actor_kind: string;
  actor_id: string;
  kernel_task_run_id: string;
  session_run_id: string;
  created_at: string;
  updated_at: string;
};

export type RichDocumentDraftLoad = {
  rich_document_id: string;
  current_doc_version: number;
  current_content_sha256: string;
  draft: RichDocumentDraft | null;
};

export type RichDocumentDraftWriteResult = {
  rich_document_id: string;
  draft: RichDocumentDraft | null;
  cleared: boolean;
  draft_receipt_event_id: string | null;
  receipt_error: string | null;
};

export type RichDocumentDraftClearResult = {
  rich_document_id: string;
  cleared: boolean;
  clear_receipt_event_id: string | null;
  receipt_error: string | null;
};

export type RichDocVersion = {
  rich_document_id: string;
  doc_version: number;
  schema_version: string;
  content_sha256: string;
  crdt_snapshot_id?: string | null;
  promotion_receipt_event_id: string | null;
  created_at: string;
};

export type RichDocVersionBody = RichDocVersion & {
  content_json: JSONContentLike;
  crdt_snapshot_id: string | null;
};

export type RichDocHistory = {
  rich_document_id: string;
  current_version: number;
  authority_label: string;
  owner_actor_kind: string | null;
  owner_actor_id: string | null;
  versions: RichDocVersion[];
};

export type RichDocEmbed = {
  embed_id: string;
  rich_document_id: string;
  block_id: string;
  ref_kind: string;
  ref_value: string;
  caption: string | null;
  repair_state: string;
  repair_reason: string | null;
};

export type RichDocBacklink = {
  backlink_id: string;
  workspace_id: string;
  relationship_id: string;
  source_document_id: string;
  link_kind: string;
  target: string;
  block_id: string;
};

export type RichDocProjectionFormat =
  | "markdown"
  | "html"
  | "plain_text"
  | "wiki_loom"
  | "context_bundle";

export async function createRichDocument(
  workspaceId: string,
  title: string,
  contentJson?: JSONContentLike,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<{ document: RichDocument; save_receipt_event_id: string }> {
  return request("/knowledge/documents", {
    method: "POST",
    body: { workspace_id: workspaceId, title, content_json: contentJson ?? null },
    headers: richDocHeaders(ctx),
  });
}

export async function loadRichDocument(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<RichDocLoad> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}`, {
    headers: richDocHeaders(ctx),
  });
}

export async function loadRichDocumentDraft(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<RichDocumentDraftLoad> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/draft`, {
    headers: richDocHeaders(ctx),
  });
}

export async function upsertRichDocumentDraft(
  documentId: string,
  baseDocVersion: number,
  baseContentSha256: string,
  contentJson: JSONContentLike,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<RichDocumentDraftWriteResult> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/draft`, {
    method: "PUT",
    body: {
      base_doc_version: baseDocVersion,
      base_content_sha256: baseContentSha256,
      content_json: contentJson,
    },
    headers: richDocHeaders(ctx),
  });
}

export async function clearRichDocumentDraft(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<RichDocumentDraftClearResult> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/draft`, {
    method: "DELETE",
    headers: richDocHeaders(ctx),
  });
}

export async function saveRichDocument(
  documentId: string,
  expectedVersion: number,
  contentJson: JSONContentLike,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
  metadata: RichDocSaveMetadata = {},
): Promise<RichDocSaveResult> {
  const body: Record<string, unknown> = { expected_version: expectedVersion, content_json: contentJson };
  if (metadata.crdt_document_id !== undefined) body.crdt_document_id = metadata.crdt_document_id;
  if (metadata.crdt_snapshot_id !== undefined) body.crdt_snapshot_id = metadata.crdt_snapshot_id;
  if (metadata.promotion_receipt_event_id !== undefined) {
    body.promotion_receipt_event_id = metadata.promotion_receipt_event_id;
  }
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/save`, {
    method: "PUT",
    body,
    headers: richDocHeaders(ctx),
  });
}

export async function loadRichDocumentBlocks(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<RichDocTree> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/blocks`, {
    headers: richDocHeaders(ctx),
  });
}

export async function loadRichDocumentHistory(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<RichDocHistory> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/history`, {
    headers: richDocHeaders(ctx),
  });
}

export async function loadRichDocumentVersion(
  documentId: string,
  docVersion: number,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<{ rich_document_id: string; version: RichDocVersionBody }> {
  return request(
    `/knowledge/documents/${encodeURIComponent(documentId)}/history/${encodeURIComponent(String(docVersion))}`,
    { headers: richDocHeaders(ctx) },
  );
}

export async function exportRichDocumentProjection(
  documentId: string,
  format: RichDocProjectionFormat,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<{ rich_document_id: string; projection: { format: string; content: string } }> {
  return request(
    `/knowledge/documents/${encodeURIComponent(documentId)}/projection?format=${format}`,
    { headers: richDocHeaders(ctx) },
  );
}

export async function listRichDocumentEmbeds(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<{ rich_document_id: string; embeds: RichDocEmbed[] }> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/embeds`, {
    headers: richDocHeaders(ctx),
  });
}

export async function listRichDocumentBrokenEmbeds(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<{ rich_document_id: string; broken_embeds: RichDocEmbed[]; available_actions: string[] }> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/embeds/broken`, {
    headers: richDocHeaders(ctx),
  });
}

export async function listRichDocumentBacklinks(
  documentId: string,
  ctx: RichDocContext = DEFAULT_RICH_DOC_CONTEXT,
): Promise<{ source_document_id: string; backlinks: RichDocBacklink[] }> {
  return request(`/knowledge/documents/${encodeURIComponent(documentId)}/backlinks`, {
    headers: richDocHeaders(ctx),
  });
}

// ---------------------------------------------------------------------------
// MT-254 DebugAdapterCore — adapter registry + durable breakpoints.
// ---------------------------------------------------------------------------

/** The kind of a runnable debug adapter. Only `node` ships today. */
export type DebugAdapterKind = "node";

/** A listable debug adapter descriptor (the honesty-gate list). */
export type DebugAdapterDescriptor = {
  kind: DebugAdapterKind;
  id: string;
  display_name: string;
  /** Always true for listed adapters: a listed adapter is a runnable adapter. */
  runnable: boolean;
};

/** A persisted per-document breakpoint. */
export type DebugBreakpointRecord = {
  breakpoint_id: string;
  rich_document_id: string;
  workspace_id: string;
  source_url: string;
  line: number;
  condition: string | null;
  verified: boolean;
  updated_at: string;
  event_ledger_event_id: string;
};

/** A breakpoint to persist (PUT replaces the full set for the document). */
export type DebugBreakpointInput = {
  source_url: string;
  line: number;
  condition?: string | null;
  verified?: boolean;
};

/**
 * The runnable debug adapters. The list IS the honesty gate: it contains ONLY
 * adapters that drive a real process (Node today), never disabled/stub entries.
 */
export async function getDebugAdapters(): Promise<{ adapters: DebugAdapterDescriptor[] }> {
  return request(`/debug/adapters`);
}

/** Durable breakpoints for a RichDocument. */
export async function getDebugBreakpoints(
  richDocumentId: string,
): Promise<{ rich_document_id: string; breakpoints: DebugBreakpointRecord[] }> {
  return request(`/debug/documents/${encodeURIComponent(richDocumentId)}/breakpoints`);
}

/** Replace the full breakpoint set for a RichDocument (PUT semantics). */
export async function setDebugBreakpoints(
  richDocumentId: string,
  workspaceId: string,
  breakpoints: DebugBreakpointInput[],
): Promise<{ rich_document_id: string; breakpoints: DebugBreakpointRecord[] }> {
  return request(`/debug/documents/${encodeURIComponent(richDocumentId)}/breakpoints`, {
    method: "PUT",
    body: { workspace_id: workspaceId, breakpoints },
  });
}

// ---------------------------------------------------------------------------
// MT-254 DebugAdapterCore — LIVE debug session HTTP surface (real process).
// These drive the backend `crate::debug_adapter` session over the SAME axum
// transport the rest of the product UI speaks. No Tauri IPC bridge.
// ---------------------------------------------------------------------------

/** A breakpoint request against a source (1-based line). */
export type SessionSourceBreakpoint = {
  line: number;
  column?: number;
  condition?: string;
};

/** The adapter's verdict on a requested breakpoint (verified is never faked). */
export type SessionBreakpoint = {
  id: string;
  verified: boolean;
  line?: number;
  message?: string;
};

/** One frame of a paused call stack. */
export type SessionStackFrame = {
  id: string;
  name: string;
  source?: string;
  line: number;
  column: number;
};

/** A variable scope within a paused frame. */
export type SessionScope = {
  name: string;
  variables_reference: string;
  expensive: boolean;
};

/** One variable (real runtime value). */
export type SessionVariable = {
  name: string;
  value: string;
  type_name?: string;
  variables_reference: string;
};

/** A streamed dap:// lifecycle/output event (raw payload shape). */
export type SessionDebugEvent =
  | { kind: "stopped"; reason: string; top_frame_line?: number; top_frame_source?: string }
  | { kind: "output"; category: string; output: string }
  | { kind: "continued" }
  | { kind: "terminated"; exit_code?: number };

/** Launch a real debuggee; returns the session id + initial (entry) pause. */
export async function launchDebugSession(input: {
  adapter: DebugAdapterKind;
  program: string;
  cwd?: string;
  runtime_path?: string;
}): Promise<{ session_id: string; adapter: string; paused: boolean; top_frame_line?: number }> {
  return request(`/debug/sessions`, { method: "POST", body: input });
}

/** Bind breakpoints on the live session (REAL CDP binding). */
export async function setSessionBreakpoints(
  sessionId: string,
  source: string,
  breakpoints: SessionSourceBreakpoint[],
): Promise<{ breakpoints: SessionBreakpoint[] }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}/breakpoints`, {
    method: "POST",
    body: { source, breakpoints },
  });
}

/** The paused call stack. */
export async function getSessionStack(
  sessionId: string,
): Promise<{ frames: SessionStackFrame[] }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}/stack`);
}

/** A paused frame's variable scopes. */
export async function getSessionScopes(
  sessionId: string,
  frameId: string,
): Promise<{ scopes: SessionScope[] }> {
  return request(
    `/debug/sessions/${encodeURIComponent(sessionId)}/frames/${encodeURIComponent(frameId)}/scopes`,
  );
}

/** Real runtime variables behind a scope/object reference. */
export async function getSessionVariables(
  sessionId: string,
  reference: string,
): Promise<{ variables: SessionVariable[] }> {
  return request(
    `/debug/sessions/${encodeURIComponent(sessionId)}/variables/${encodeURIComponent(reference)}`,
  );
}

/** Debug-console eval in the paused frame. */
export async function evaluateInSession(
  sessionId: string,
  frameId: string,
  expression: string,
): Promise<{ result: string }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}/evaluate`, {
    method: "POST",
    body: { frame_id: frameId, expression },
  });
}

/** Step over/into/out; resolves once paused again. */
export async function stepSession(
  sessionId: string,
  kind: "over" | "into" | "out",
): Promise<{ paused: boolean; top_frame_line?: number }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}/step`, {
    method: "POST",
    body: { kind },
  });
}

/** Resume execution. */
export async function continueSession(sessionId: string): Promise<{ resumed: boolean }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}/continue`, { method: "POST" });
}

/** Pause a running debuggee. */
export async function pauseSession(sessionId: string): Promise<{ paused: boolean }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}/pause`, { method: "POST" });
}

/** Drain the buffered dap events that arrived since the last poll. */
export async function pollSessionEvents(
  sessionId: string,
): Promise<{ events: SessionDebugEvent[] }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}/events`);
}

/** Terminate the session; returns the real process exit code if known. */
export async function terminateDebugSession(
  sessionId: string,
): Promise<{ terminated: boolean; exit_code?: number }> {
  return request(`/debug/sessions/${encodeURIComponent(sessionId)}`, { method: "DELETE" });
}
