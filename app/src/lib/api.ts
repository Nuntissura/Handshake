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
  actor: string;
  actor_id?: string;
  event_type: string;
  job_id?: string;
  workflow_id?: string;
  payload: unknown;
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

  const query = params.toString();
  const path = query.length > 0 ? `/api/flight_recorder?${query}` : "/api/flight_recorder";
  return request(path);
}

export type AiJob = {
  id: string;
  job_kind: string;
  status: string;
  error_message?: string;
  protocol_id: string;
  profile_id: string;
  capability_profile_id: string;
  access_mode: string;
  safety_mode: string;
  job_inputs?: string;
  job_outputs?: string;
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
