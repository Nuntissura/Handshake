import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  configureHandshakeCodeIntelligence,
  ensureHandshakeCodeIntelligenceProviders,
  refreshHandshakeCodeIntelligenceMarkers,
  resetHandshakeCodeIntelligenceForTests,
} from "./code_intelligence";
import { createDiagnostic, getCodeFileLens, lookupCodeSymbols } from "../api";

vi.mock("../api", () => ({
  lookupCodeSymbols: vi.fn(),
  getCodeSymbolReferences: vi.fn(),
  getCodeFileLens: vi.fn(),
  createDiagnostic: vi.fn(),
}));

type ProviderCallback = (...args: unknown[]) => unknown;
type CompletionResult = { suggestions: Array<{ label: string; documentation: { value: string } }> };
type CompletionProvider = {
  provideCompletionItems: (...args: unknown[]) => CompletionResult | Promise<CompletionResult>;
};

function fakeMonaco() {
  const providers = {
    completions: [] as CompletionProvider[],
    hovers: [] as Array<{ provideHover: ProviderCallback }>,
    definitions: [] as Array<{ provideDefinition: ProviderCallback }>,
    references: [] as Array<{ provideReferences: ProviderCallback }>,
    formatters: [] as Array<{ provideDocumentFormattingEdits: ProviderCallback }>,
    codeLenses: [] as Array<{ provideCodeLenses: ProviderCallback }>,
    inlayHints: [] as Array<{ provideInlayHints: ProviderCallback }>,
  };
  const markers: unknown[] = [];
  class Range {
    constructor(
      public startLineNumber: number,
      public startColumn: number,
      public endLineNumber: number,
      public endColumn: number,
    ) {}
  }
  const disposable = { dispose: vi.fn() };
  return {
    providers,
    markers,
    monaco: {
      Range,
      MarkerSeverity: { Warning: 4 },
      editor: {
        setModelMarkers: (_model: unknown, _owner: string, nextMarkers: unknown[]) => {
          markers.push(...nextMarkers);
        },
      },
      languages: {
        CompletionItemKind: {
          Class: 1,
          Enum: 2,
          Field: 3,
          Function: 4,
          Module: 5,
          Variable: 6,
        },
        InlayHintKind: { Type: 1 },
        registerCompletionItemProvider: (_language: string, provider: CompletionProvider) => {
          providers.completions.push(provider);
          return disposable;
        },
        registerHoverProvider: (_language: string, provider: { provideHover: ProviderCallback }) => {
          providers.hovers.push(provider);
          return disposable;
        },
        registerDefinitionProvider: (_language: string, provider: { provideDefinition: ProviderCallback }) => {
          providers.definitions.push(provider);
          return disposable;
        },
        registerReferenceProvider: (_language: string, provider: { provideReferences: ProviderCallback }) => {
          providers.references.push(provider);
          return disposable;
        },
        registerDocumentFormattingEditProvider: (
          _language: string,
          provider: { provideDocumentFormattingEdits: ProviderCallback },
        ) => {
          providers.formatters.push(provider);
          return disposable;
        },
        registerCodeLensProvider: (_language: string, provider: { provideCodeLenses: ProviderCallback }) => {
          providers.codeLenses.push(provider);
          return disposable;
        },
        registerInlayHintsProvider: (_language: string, provider: { provideInlayHints: ProviderCallback }) => {
          providers.inlayHints.push(provider);
          return disposable;
        },
      },
    },
  };
}

function fakeModel(value = "add(1, 2)", wordUntil = "ad", wholeWord = wordUntil) {
  return {
    uri: { toString: () => "inmemory://code" },
    getValue: () => value,
    getFullModelRange: () => ({ startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: value.length + 1 }),
    getWordUntilPosition: () => ({ word: wordUntil, startColumn: 1, endColumn: wordUntil.length + 1 }),
    getWordAtPosition: () => ({ word: wholeWord, startColumn: 1, endColumn: wholeWord.length + 1 }),
  };
}

describe("Handshake Monaco code intelligence bridge", () => {
  beforeEach(() => {
    resetHandshakeCodeIntelligenceForTests();
    vi.mocked(lookupCodeSymbols).mockReset();
    vi.mocked(getCodeFileLens).mockReset();
    vi.mocked(createDiagnostic).mockReset();
  });

  it("registers completion providers that query the backend by prefix and surface staleness", async () => {
    const env = fakeMonaco();
    vi.mocked(lookupCodeSymbols).mockResolvedValue({
      workspace_id: "w1",
      matches: [
        {
          symbol_entity_id: "KEN-add",
          symbol_key: "rust:src/lib.rs#add",
          display_name: "add",
          symbol_kind: "function",
          lifecycle_state: "active",
          primary_source_id: "source-lib",
          owning_wp: "WP-KERNEL-009",
          definition: { span_id: "span-add", source_id: "source-lib", line_start: 2, line_end: 2, range_start: 1, range_end: 10 },
          staleness: { state: "marked_stale", fresh: false },
        },
      ],
      nav_receipt_event_id: "EVT-lookup",
      quiet_background_work_receipt_id: "quiet-lookup",
    });
    configureHandshakeCodeIntelligence({ workspaceId: "w1" });

    ensureHandshakeCodeIntelligenceProviders(env.monaco as never);
    const result = await env.providers.completions[0].provideCompletionItems(fakeModel(), { lineNumber: 1, column: 3 });

    expect(lookupCodeSymbols).toHaveBeenCalledWith({ workspaceId: "w1", prefix: "ad", limit: 20 });
    expect(result.suggestions[0].label).toBe("add");
    expect(result.suggestions[0].documentation.value).toContain("marked_stale");
  });

  it("does not query code navigation when no active workspace exists", async () => {
    const env = fakeMonaco();

    ensureHandshakeCodeIntelligenceProviders(env.monaco as never);
    const result = await env.providers.completions[0].provideCompletionItems(fakeModel(), { lineNumber: 1, column: 3 });

    expect(result.suggestions).toEqual([]);
    expect(lookupCodeSymbols).not.toHaveBeenCalled();
  });

  it("enriches hover content with backend file-lens documentation when indexed staleness is available", async () => {
    const env = fakeMonaco();
    vi.mocked(lookupCodeSymbols).mockResolvedValue({
      workspace_id: "w1",
      matches: [
        {
          symbol_entity_id: "KEN-add",
          symbol_key: "rust:src/lib.rs#add",
          display_name: "add",
          symbol_kind: "function",
          lifecycle_state: "active",
          primary_source_id: "source-lib",
          owning_wp: "WP-KERNEL-009",
          definition: { span_id: "span-add", source_id: "source-lib", line_start: 2, line_end: 2, range_start: 1, range_end: 10 },
          staleness: {
            state: "fresh",
            fresh: true,
            indexed_content_hash: "hash-lib",
            indexed_parser_version: "tree-sitter-rust@1",
          },
        },
      ],
      nav_receipt_event_id: "EVT-lookup",
      quiet_background_work_receipt_id: "quiet-lookup",
    });
    vi.mocked(getCodeFileLens).mockResolvedValue({
      workspace_id: "w1",
      relative_path: "src/lib.rs",
      staleness: { state: "fresh", fresh: true },
      truncated: false,
      entries: [
        {
          symbol_entity_id: "KEN-add",
          symbol_key: "rust:src/lib.rs#add",
          display_name: "add",
          symbol_kind: "function",
          definition: { start_line: 2, end_line: 2 },
          references: [],
          doc: "Adds two numbers.",
          caller_count: 0,
        },
      ],
      nav_receipt_event_id: "EVT-lens",
      quiet_background_work_receipt_id: "quiet-lens",
    });
    configureHandshakeCodeIntelligence({ workspaceId: "w1" });

    ensureHandshakeCodeIntelligenceProviders(env.monaco as never);
    const hover = (await env.providers.hovers[0].provideHover(fakeModel("add(1, 2)", "a", "add"), {
      lineNumber: 1,
      column: 2,
    })) as { contents: Array<{ value: string }> };

    expect(lookupCodeSymbols).toHaveBeenCalledWith({ workspaceId: "w1", name: "add", limit: 5 });
    expect(getCodeFileLens).toHaveBeenCalledWith("w1", "src/lib.rs", "hash-lib", "tree-sitter-rust@1");
    expect(hover.contents[0].value).toContain("Adds two numbers.");
  });

  it("sets Monaco warning markers for stale backend symbol verdicts", async () => {
    const env = fakeMonaco();
    vi.mocked(lookupCodeSymbols).mockResolvedValue({
      workspace_id: "w1",
      matches: [
        {
          symbol_entity_id: "KEN-add",
          symbol_key: "rust:src/lib.rs#add",
          display_name: "add",
          symbol_kind: "function",
          lifecycle_state: "active",
          primary_source_id: "source-lib",
          owning_wp: "WP-KERNEL-009",
          definition: null,
          staleness: { state: "marked_stale", fresh: false },
        },
      ],
      nav_receipt_event_id: "EVT-lookup",
      quiet_background_work_receipt_id: "quiet-lookup",
    });
    configureHandshakeCodeIntelligence({ workspaceId: "w1" });
    const model = fakeModel();
    const editor = { getModel: () => model };

    await refreshHandshakeCodeIntelligenceMarkers(editor as never, env.monaco as never);

    expect(env.markers).toEqual([
      expect.objectContaining({
        message: expect.stringContaining("marked_stale"),
        severity: 4,
      }),
    ]);
    expect(createDiagnostic).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "Stale code intelligence result",
        severity: "warning",
        source: "lsp",
        surface: "monaco",
        code: "HSK-CODE-INTEL-STALE",
        wsid: "w1",
        locations: [
          expect.objectContaining({
            entity_id: "KEN-add",
            range: { startLine: 1, startColumn: 1, endLine: 1, endColumn: 4 },
          }),
        ],
      }),
    );
  });

  it("scans beyond early prose identifiers when emitting stale marker diagnostics", async () => {
    const env = fakeMonaco();
    vi.mocked(lookupCodeSymbols).mockImplementation(async (input: unknown) => {
      const name = (input as { name?: string }).name;
      return {
        workspace_id: "w1",
        matches:
          name === "add"
            ? [
                {
                  symbol_entity_id: "KEN-add",
                  symbol_key: "rust:src/lib.rs#add",
                  display_name: "add",
                  symbol_kind: "function",
                  lifecycle_state: "active",
                  primary_source_id: "source-lib",
                  owning_wp: "WP-KERNEL-009",
                  definition: null,
                  staleness: { state: "marked_stale", fresh: false },
                },
              ]
            : [],
        nav_receipt_event_id: "EVT-lookup",
        quiet_background_work_receipt_id: "quiet-lookup",
      };
    });
    configureHandshakeCodeIntelligence({ workspaceId: "w1" });
    const model = fakeModel("// mt249 runtime monaco proof filler words\npub fn scratch() {}\nadd(1, 2)");
    const editor = { getModel: () => model };

    await refreshHandshakeCodeIntelligenceMarkers(editor as never, env.monaco as never);

    expect(env.markers).toEqual([
      expect.objectContaining({
        message: expect.stringContaining("marked_stale"),
        startLineNumber: 3,
        startColumn: 1,
        endColumn: 4,
      }),
    ]);
    expect(createDiagnostic).toHaveBeenCalledWith(
      expect.objectContaining({
        code: "HSK-CODE-INTEL-STALE",
        locations: [
          expect.objectContaining({
            entity_id: "KEN-add",
            range: { startLine: 3, startColumn: 1, endLine: 3, endColumn: 4 },
          }),
        ],
      }),
    );
  });
});
