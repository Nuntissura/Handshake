const BASE_URL = "http://127.0.0.1:37501";

type FetchOptions = {
  method?: string;
  body?: unknown;
};

async function request<T>(path: string, options: FetchOptions = {}): Promise<T> {
  const response = await fetch(`${BASE_URL}${path}`, {
    method: options.method ?? "GET",
    headers: {
      "Content-Type": "application/json",
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
    | "debug_bundle_export";
  job_id?: string;
  workflow_id?: string;
  model_id?: string;
  wsids: string[];
  payload: unknown;
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
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
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

export async function updateDocumentBlocks(documentId: string, blocks: BlockInput[]): Promise<Block[]> {
  return request(`/documents/${encodeURIComponent(documentId)}/blocks`, {
    method: "PUT",
    body: { blocks },
  });
}

export async function updateCanvasGraph(
  canvasId: string,
  nodes: CanvasNodeInput[],
  edges: CanvasEdgeInput[],
): Promise<CanvasWithGraph> {
  return request(`/canvases/${encodeURIComponent(canvasId)}`, {
    method: "PUT",
    body: { nodes, edges },
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
  jobId?: string;
  traceId?: string;
  from?: string;
  to?: string;
  actor?: "human" | "agent" | "system";
  eventType?: FlightEvent["event_type"];
  wsid?: string;
};

export async function getEvents(filters?: FlightEventFilters): Promise<FlightEvent[]> {
  const toIso = (value: string) => {
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toISOString();
  };

  const params = new URLSearchParams();
  if (filters?.jobId) params.append("job_id", filters.jobId);
  if (filters?.traceId) params.append("trace_id", filters.traceId);
  if (filters?.from) params.append("from", toIso(filters.from));
  if (filters?.to) params.append("to", toIso(filters.to));
  if (filters?.actor) params.append("actor", filters.actor);
  if (filters?.eventType) params.append("event_type", filters.eventType);
  if (filters?.wsid) params.append("wsid", filters.wsid);

  const query = params.toString();
  const path = query.length > 0 ? `/api/flight_recorder?${query}` : "/api/flight_recorder";
  return request(path);
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
): Promise<WorkflowRun> {
  return request("/api/jobs", {
    method: "POST",
    body: { job_kind: jobKind, protocol_id: protocolId, doc_id: docId },
  });
}

export async function getJob(jobId: string): Promise<AiJob> {
  return request(`/api/jobs/${encodeURIComponent(jobId)}`);
}

export type ListJobsFilters = {
  status?: string;
  job_kind?: string;
  from?: string;
  to?: string;
};

export async function listJobs(filters?: ListJobsFilters): Promise<AiJob[]> {
  const params = new URLSearchParams();
  if (filters?.status) params.append("status", filters.status);
  if (filters?.job_kind) params.append("job_kind", filters.job_kind);
  if (filters?.from) params.append("from", new Date(filters.from).toISOString());
  if (filters?.to) params.append("to", new Date(filters.to).toISOString());

  const query = params.toString();
  const path = query.length > 0 ? `/api/jobs?${query}` : "/api/jobs";
  return request(path);
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
  status: "queued" | "running" | "ready" | "failed";
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
  return request(`/api/bundles/debug/${encodeURIComponent(bundleId)}`, { method: "POST" });
}

export async function downloadBundle(bundleId: string): Promise<Blob> {
  const response = await fetch(`${BASE_URL}/api/bundles/debug/${encodeURIComponent(bundleId)}/download`);
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Download failed: ${text}`);
  }
  return response.blob();
}
