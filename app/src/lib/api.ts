import { loadViewModeFromStorage } from "./viewMode";

const BASE_URL = "http://127.0.0.1:37501";

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
    throw new Error(`Request failed: ${response.status} ${response.statusText} - ${text}`);
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

export type WorkflowRun = {
  id: string;
  job_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

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
