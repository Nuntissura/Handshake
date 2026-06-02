import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";

// OperatorChat unit tests (governance glue #3): the chat box accepts ALL spawned
// sessions (local + cloud BYOK + official CLI), labels each by provider + model
// + worktree, disables non-live sessions honestly, sends a REAL generate through
// the provider-agnostic backend path, and surfaces backend errors verbatim. We
// mock only the chatGenerate IPC CALL; the rest of the component is real.

const chatGenerateMock = vi.fn();
const chatGenerateWithCloudEscalationMock = vi.fn();

vi.mock("../../lib/ipc/swarm_runtime", async () => {
  const actual = await vi.importActual<typeof import("../../lib/ipc/swarm_runtime")>(
    "../../lib/ipc/swarm_runtime",
  );
  return {
    ...actual,
    chatGenerate: (instanceId: string, prompt: string) => chatGenerateMock(instanceId, prompt),
    chatGenerateWithCloudEscalation: (request: unknown) =>
      chatGenerateWithCloudEscalationMock(request),
  };
});

import { OperatorChat, canChat, sessionOptionLabel } from "./OperatorChat";
import type { SwarmSession } from "../../lib/ipc/swarm_runtime";

function makeSession(over: Partial<SwarmSession> & { modelId?: string; instance?: number }): SwarmSession {
  const modelId = over.modelId ?? "m";
  const instance = over.instance ?? 0;
  return {
    instanceId: { modelId, instance, composite: `${modelId}#${instance}` },
    state: "READY",
    provider: "local",
    runtimeBinding: "candle",
    artifactPath: null,
    cloudModelName: null,
    worktreeId: null,
    workingDir: null,
    ...over,
  } as SwarmSession;
}

const LOCAL = makeSession({
  modelId: "alpha-model",
  provider: "local",
  artifactPath: "D:/models/alpha/model.safetensors",
  worktreeId: "wt-feature-x",
});
const CLOUD = makeSession({
  modelId: "beta-cloud",
  provider: "byok_cloud",
  cloudModelName: "claude-sonnet-4",
});
const CLI = makeSession({
  modelId: "gamma-cli",
  provider: "official_cli",
  cloudModelName: "claude-code",
});
const DEAD = makeSession({
  modelId: "delta-dead",
  provider: "byok_cloud",
  cloudModelName: "gpt-4o",
  state: "CANCELLED",
});

beforeEach(() => {
  chatGenerateMock.mockReset();
  chatGenerateWithCloudEscalationMock.mockReset();
});

describe("canChat (pure helper)", () => {
  test("READY and GENERATING are chattable; everything else is not", () => {
    expect(canChat(makeSession({ state: "READY" }))).toBe(true);
    expect(canChat(makeSession({ state: "GENERATING" }))).toBe(true);
    expect(canChat(makeSession({ state: "QUEUED" }))).toBe(false);
    expect(canChat(makeSession({ state: "LOADING" }))).toBe(false);
    expect(canChat(makeSession({ state: "CANCELLED" }))).toBe(false);
    expect(canChat(makeSession({ state: "FAILED" }))).toBe(false);
    expect(canChat(makeSession({ state: "COMPLETED" }))).toBe(false);
  });
});

describe("sessionOptionLabel (provider + model + worktree)", () => {
  test("local: provider tag + artifact basename + worktree", () => {
    expect(sessionOptionLabel(LOCAL)).toBe("local · model.safetensors · wt:wt-feature-x (#0, READY)");
  });
  test("cloud: provider tag + cloud model name, no worktree", () => {
    expect(sessionOptionLabel(CLOUD)).toBe("cloud · claude-sonnet-4 (#0, READY)");
  });
  test("CLI: provider tag maps official_cli -> CLI", () => {
    expect(sessionOptionLabel(CLI)).toBe("CLI · claude-code (#0, READY)");
  });
});

describe("OperatorChat picker", () => {
  test("lists local + cloud + CLI sessions, each tagged with its provider", () => {
    render(
      <OperatorChat
        selectedInstanceId={null}
        sessions={[LOCAL, CLOUD, CLI]}
        onSelectInstance={() => {}}
      />,
    );
    const picker = screen.getByTestId("operator-chat-session") as HTMLSelectElement;
    expect(within(picker).getByText(/local · model.safetensors/)).toBeInTheDocument();
    expect(within(picker).getByText(/cloud · claude-sonnet-4/)).toBeInTheDocument();
    expect(within(picker).getByText(/CLI · claude-code/)).toBeInTheDocument();

    // data-provider tags every option for the visual/test matrix.
    expect(screen.getByTestId("operator-chat-option-alpha-model#0")).toHaveAttribute("data-provider", "local");
    expect(screen.getByTestId("operator-chat-option-beta-cloud#0")).toHaveAttribute("data-provider", "byok_cloud");
    expect(screen.getByTestId("operator-chat-option-gamma-cli#0")).toHaveAttribute("data-provider", "official_cli");
  });

  test("a non-live (CANCELLED) session renders as a DISABLED option", () => {
    render(
      <OperatorChat
        selectedInstanceId={null}
        sessions={[CLOUD, DEAD]}
        onSelectInstance={() => {}}
      />,
    );
    expect(screen.getByTestId("operator-chat-option-delta-dead#0")).toBeDisabled();
    expect(screen.getByTestId("operator-chat-option-beta-cloud#0")).not.toBeDisabled();
  });
});

describe("OperatorChat honest non-chattable state", () => {
  test("selecting a non-live session disables the composer and shows the honest note", () => {
    render(
      <OperatorChat
        selectedInstanceId="delta-dead#0"
        sessions={[DEAD]}
        onSelectInstance={() => {}}
      />,
    );
    expect(screen.getByTestId("operator-chat-unsupported")).toHaveTextContent(
      /Session is CANCELLED; not ready for a chat turn/i,
    );
    expect(screen.getByTestId("operator-chat-input")).toBeDisabled();
    expect(screen.getByTestId("operator-chat-send")).toBeDisabled();
  });
});

describe("OperatorChat cloud generate (provider-agnostic path)", () => {
  test("selecting a cloud session and sending calls chatGenerate(composite, text) and renders the model turn", async () => {
    chatGenerateMock.mockResolvedValue({ text: "Hello from the cloud session.", tokenCount: 6, finishReason: "stop" });
    render(
      <OperatorChat
        selectedInstanceId="beta-cloud#0"
        sessions={[CLOUD]}
        onSelectInstance={() => {}}
      />,
    );

    const input = screen.getByTestId("operator-chat-input");
    expect(input).toBeEnabled();
    fireEvent.change(input, { target: { value: "Hi cloud" } });
    fireEvent.click(screen.getByTestId("operator-chat-send"));

    await waitFor(() => expect(chatGenerateMock).toHaveBeenCalledWith("beta-cloud#0", "Hi cloud"));
    expect(await screen.findByText("Hello from the cloud session.")).toBeInTheDocument();
  });

  test("a rejected chatGenerate surfaces the backend error verbatim (honesty)", async () => {
    chatGenerateMock.mockRejectedValue(new Error("session no longer live"));
    render(
      <OperatorChat
        selectedInstanceId="beta-cloud#0"
        sessions={[CLOUD]}
        onSelectInstance={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("operator-chat-input"), { target: { value: "Hi" } });
    fireEvent.click(screen.getByTestId("operator-chat-send"));

    expect(await screen.findByTestId("operator-chat-error")).toHaveTextContent("session no longer live");
  });

  test("local session can opt into cloud escalation and renders the cloud result", async () => {
    chatGenerateWithCloudEscalationMock.mockResolvedValueOnce({
      selected: "cloud",
      escalated: true,
      escalationReason: "local runtime overloaded",
      localError: null,
      local: null,
      cloudInstance: { modelId: "cloud-fallback", instance: 0, composite: "cloud-fallback#0" },
      cloud: { text: "Recovered from cloud.", tokenCount: 4, finishReason: "stop" },
      cloudError: null,
    });

    render(
      <OperatorChat
        selectedInstanceId="alpha-model#0"
        sessions={[LOCAL]}
        onSelectInstance={() => {}}
        cloudEscalation={{
          label: "openai · gpt-4o",
          request: {
            provider: "byok_cloud",
            byokCloudProvider: "openai",
            cloudModelName: "gpt-4o",
            swarmId: "wt-feature-x",
            worktreeId: "wt-feature-x",
          },
        }}
      />,
    );

    fireEvent.click(screen.getByTestId("operator-chat-escalation-enabled"));
    fireEvent.change(screen.getByTestId("operator-chat-escalation-task-class"), {
      target: { value: "hard_reasoning" },
    });
    fireEvent.change(screen.getByTestId("operator-chat-input"), {
      target: { value: "try local first" },
    });
    fireEvent.click(screen.getByTestId("operator-chat-send"));

    await waitFor(() => {
      expect(chatGenerateWithCloudEscalationMock).toHaveBeenCalledWith({
        localInstanceId: "alpha-model#0",
        prompt: "try local first",
        cloud: {
          provider: "byok_cloud",
          byokCloudProvider: "openai",
          cloudModelName: "gpt-4o",
          swarmId: "wt-feature-x",
          worktreeId: "wt-feature-x",
        },
        taskClass: "hard_reasoning",
      });
    });
    expect(chatGenerateMock).not.toHaveBeenCalled();
    expect(await screen.findByText("Escalated to cloud: local runtime overloaded")).toBeInTheDocument();
    expect(await screen.findByText("Recovered from cloud.")).toBeInTheDocument();
  });

  test("cloud fallback is disabled for non-local sessions", async () => {
    render(
      <OperatorChat
        selectedInstanceId="beta-cloud#0"
        sessions={[CLOUD]}
        onSelectInstance={() => {}}
        cloudEscalation={{
          label: "openai · gpt-4o",
          request: {
            provider: "byok_cloud",
            byokCloudProvider: "openai",
            cloudModelName: "gpt-4o",
          },
        }}
      />,
    );

    expect(screen.getByTestId("operator-chat-escalation-enabled")).toBeDisabled();
    expect(screen.getByTestId("operator-chat-escalation-note")).toHaveTextContent(
      /Select a READY local session/i,
    );
  });
});
