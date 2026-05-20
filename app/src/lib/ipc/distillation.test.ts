import { describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  extractDistillCorpus,
  listDistillCandidates,
  listDistillJobs,
  listDistillSessions,
  promoteDistillCandidate,
  rejectDistillCandidate,
} from "./distillation";

describe("distillation IPC bindings", () => {
  it("list_distill_sessions invokes the camelCase Tauri channel", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        sessionId: "s-1",
        modelId: "unknown",
        closedAtUtc: "2026-05-20T04:00:00Z",
        turnCount: 0,
      },
    ]);

    const result = await listDistillSessions();

    expect(invokeMock).toHaveBeenCalledWith("list_distill_sessions");
    expect(result).toHaveLength(1);
    expect(result[0].sessionId).toBe("s-1");
    expect(result[0].closedAtUtc).toBe("2026-05-20T04:00:00Z");
  });

  it("list_distill_candidates returns Pending entries with provenance", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        loraId: "lora-1",
        teacherModelPath: "teacher.gguf",
        studentBaseModelPath: "student.gguf",
        corpusTurnCount: 12,
        trainedAtUtc: "2026-05-20T05:00:00Z",
        licenseTag: "MIT",
        status: "Pending",
        rejectionReason: null,
      },
    ]);

    const result = await listDistillCandidates();

    expect(invokeMock).toHaveBeenCalledWith("list_distill_candidates");
    expect(result[0].status).toBe("Pending");
    expect(result[0].licenseTag).toBe("MIT");
    expect(result[0].corpusTurnCount).toBe(12);
  });

  it("list_distill_jobs surfaces queued/running/done/error states", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        jobId: "job-1",
        sessionId: "s-1",
        status: "done",
        queuedAtUtc: "2026-05-20T05:00:00Z",
        startedAtUtc: "2026-05-20T05:01:00Z",
        finishedAtUtc: "2026-05-20T05:30:00Z",
        errorMessage: null,
      },
    ]);

    const result = await listDistillJobs();

    expect(invokeMock).toHaveBeenCalledWith("list_distill_jobs");
    expect(result[0].status).toBe("done");
    expect(result[0].startedAtUtc).toBe("2026-05-20T05:01:00Z");
    expect(result[0].errorMessage).toBeNull();
  });

  it("extract_distill_corpus passes the request payload through invoke()", async () => {
    invokeMock.mockResolvedValueOnce({
      sessionId: "s-1",
      status: "live_runtime_unavailable",
      eventType: "FR-EVT-DISTILL-EXTRACT-REQUESTED",
    });

    const result = await extractDistillCorpus({ sessionId: "s-1" });

    expect(invokeMock).toHaveBeenCalledWith("extract_distill_corpus", {
      request: { sessionId: "s-1" },
    });
    expect(result.status).toBe("live_runtime_unavailable");
    expect(result.eventType).toBe("FR-EVT-DISTILL-EXTRACT-REQUESTED");
  });

  it("promote_distill_candidate forwards loraId + operatorSignature", async () => {
    invokeMock.mockResolvedValueOnce({
      loraId: "lora-1",
      newStatus: "Promoted",
      eventType: "FR-EVT-DISTILL-CANDIDATE-PROMOTE",
    });

    const result = await promoteDistillCandidate({
      loraId: "lora-1",
      operatorSignature: "op-ilja",
    });

    expect(invokeMock).toHaveBeenCalledWith("promote_distill_candidate", {
      request: { loraId: "lora-1", operatorSignature: "op-ilja" },
    });
    expect(result.newStatus).toBe("Promoted");
  });

  it("reject_distill_candidate forwards loraId + signature + reason", async () => {
    invokeMock.mockResolvedValueOnce({
      loraId: "lora-1",
      newStatus: "Rejected",
      eventType: "FR-EVT-DISTILL-CANDIDATE-REJECT",
    });

    const result = await rejectDistillCandidate({
      loraId: "lora-1",
      operatorSignature: "op-ilja",
      reason: "eval regressed",
    });

    expect(invokeMock).toHaveBeenCalledWith("reject_distill_candidate", {
      request: {
        loraId: "lora-1",
        operatorSignature: "op-ilja",
        reason: "eval regressed",
      },
    });
    expect(result.newStatus).toBe("Rejected");
  });
});
