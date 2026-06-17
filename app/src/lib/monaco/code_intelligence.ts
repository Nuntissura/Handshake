import type * as Monaco from "monaco-editor";
import {
  createDiagnostic,
  getCodeFileLens,
  getCodeSymbolReferences,
  lookupCodeSymbols,
  type CodeFileLensEntry,
  type CodeSymbolDefinition,
  type CodeSymbolNavProjection,
  type CodeSymbolReference,
} from "../api";
import { HANDSHAKE_CODE_LANGUAGES } from "./language_registry";

type MonacoApi = typeof Monaco;
type TextModel = Monaco.editor.ITextModel;
type Position = Monaco.Position;
type IDisposable = Monaco.IDisposable;
type StandaloneEditor = Monaco.editor.IStandaloneCodeEditor;

export const HANDSHAKE_OPEN_CODE_SYMBOL_COMMAND = "handshake.codeIntelligence.openSymbol";
const MARKER_OWNER = "handshake-code-intelligence";
const SYMBOL_LOOKUP_LIMIT = 20;
const LENS_IDENTIFIER_LIMIT = 12;
const STALE_DIAGNOSTIC_CODE = "HSK-CODE-INTEL-STALE";

type CodeIntelligenceConfig = {
  workspaceId: string | null;
  openCodeSymbol?: (symbolEntityId: string) => void;
};

let config: CodeIntelligenceConfig = { workspaceId: null };
let providerDisposables: IDisposable[] = [];
let emittedStaleDiagnosticKeys = new Set<string>();

export function configureHandshakeCodeIntelligence(next: CodeIntelligenceConfig): void {
  config = next;
}

export function resetHandshakeCodeIntelligenceForTests(): void {
  for (const disposable of providerDisposables) disposable.dispose();
  providerDisposables = [];
  config = { workspaceId: null };
  emittedStaleDiagnosticKeys = new Set();
}

export function codeSymbolStalenessLabel(staleness: unknown): string {
  if (!staleness || typeof staleness !== "object") return "unknown";
  const record = staleness as Record<string, unknown>;
  const state = typeof record.state === "string" ? record.state : "unknown";
  const fresh = record.fresh === true ? "fresh" : "not fresh";
  return `${state} (${fresh})`;
}

function activeWorkspaceId(): string | null {
  return config.workspaceId?.trim() || null;
}

function supportedLanguageIds(): string[] {
  return HANDSHAKE_CODE_LANGUAGES.map((language) => language.id);
}

function wordUntilPosition(
  model: TextModel,
  position: Position,
): { word: string; startColumn: number; endColumn: number } | null {
  const word = model.getWordUntilPosition(position);
  if (!word.word.trim()) return null;
  return {
    word: word.word,
    startColumn: word.startColumn,
    endColumn: word.endColumn,
  };
}

function wordAtPosition(
  model: TextModel,
  position: Position,
): { word: string; startColumn: number; endColumn: number } | null {
  const word = model.getWordAtPosition(position);
  if (!word?.word.trim()) return null;
  return {
    word: word.word,
    startColumn: word.startColumn,
    endColumn: word.endColumn,
  };
}

function rangeForDefinition(monacoApi: MonacoApi, definition: CodeSymbolDefinition): Monaco.Range {
  const start = Math.max(1, definition.line_start ?? 1);
  const end = Math.max(start, definition.line_end ?? start);
  return new monacoApi.Range(start, 1, end, 1);
}

function rangeForSpan(monacoApi: MonacoApi, span: CodeSymbolReference["evidence_spans"][number]): Monaco.Range | null {
  if (typeof span.line_start !== "number") return null;
  const start = Math.max(1, span.line_start);
  const end = Math.max(start, typeof span.line_end === "number" ? span.line_end : start);
  return new monacoApi.Range(start, 1, end, 1);
}

async function lookupFirstSymbol(workspaceId: string, name: string): Promise<CodeSymbolNavProjection | null> {
  const response = await lookupCodeSymbols({ workspaceId, name, limit: 5 });
  return response.matches[0] ?? null;
}

function completionKind(monacoApi: MonacoApi, kind: string): Monaco.languages.CompletionItemKind {
  const completionKinds = monacoApi.languages.CompletionItemKind;
  switch (kind) {
    case "class":
    case "struct":
      return completionKinds.Class;
    case "enum":
      return completionKinds.Enum;
    case "field":
    case "property":
      return completionKinds.Field;
    case "module":
    case "namespace":
      return completionKinds.Module;
    case "variable":
      return completionKinds.Variable;
    default:
      return completionKinds.Function;
  }
}

function symbolFilePath(symbolKey: string): string | null {
  const beforeHash = symbolKey.split("#")[0];
  const separator = beforeHash.indexOf(":");
  if (separator < 0) return null;
  const path = beforeHash.slice(separator + 1).trim();
  return path.length > 0 ? path : null;
}

function fileLensInputs(symbol: CodeSymbolNavProjection): {
  relativePath: string;
  contentHash: string;
  parserVersion: string;
} | null {
  if (!symbol.staleness || typeof symbol.staleness !== "object") return null;
  const staleness = symbol.staleness as Record<string, unknown>;
  const contentHash = staleness.indexed_content_hash;
  const parserVersion = staleness.indexed_parser_version;
  const relativePath = symbolFilePath(symbol.symbol_key);
  if (typeof contentHash !== "string" || typeof parserVersion !== "string" || !relativePath) return null;
  return { relativePath, contentHash, parserVersion };
}

function stalenessState(staleness: unknown): string {
  if (!staleness || typeof staleness !== "object") return "unknown";
  const state = (staleness as Record<string, unknown>).state;
  return typeof state === "string" ? state : "unknown";
}

function staleDiagnosticKey(
  workspaceId: string,
  model: TextModel,
  identifier: { word: string; line: number; column: number },
  symbol: CodeSymbolNavProjection,
): string {
  const state = stalenessState(symbol.staleness);
  return [
    workspaceId,
    model.uri.toString(),
    symbol.symbol_entity_id,
    state,
    identifier.line,
    identifier.column,
    identifier.word,
  ].join("|");
}

function emitStaleCodeIntelligenceDiagnostic(
  workspaceId: string,
  model: TextModel,
  identifier: { word: string; line: number; column: number },
  symbol: CodeSymbolNavProjection,
): void {
  const key = staleDiagnosticKey(workspaceId, model, identifier, symbol);
  if (emittedStaleDiagnosticKeys.has(key)) return;
  emittedStaleDiagnosticKeys.add(key);

  const stalenessLabel = codeSymbolStalenessLabel(symbol.staleness);
  const state = stalenessState(symbol.staleness);
  const path = symbolFilePath(symbol.symbol_key);

  void Promise.resolve(
    createDiagnostic({
      title: "Stale code intelligence result",
      message: `Code index is ${stalenessLabel} for ${symbol.display_name} (${symbol.symbol_key}).`,
      severity: "warning",
      source: "lsp",
      surface: "monaco",
      code: STALE_DIAGNOSTIC_CODE,
      tags: ["code-intelligence", "staleness", state, "MT-249"],
      wsid: workspaceId,
      actor: "system",
      capability_id: "MT-249",
      locations: [
        {
          entity_id: symbol.symbol_entity_id,
          path: path ?? undefined,
          uri: model.uri.toString(),
          wsid: workspaceId,
          range: {
            startLine: identifier.line,
            startColumn: identifier.column,
            endLine: identifier.line,
            endColumn: identifier.column + identifier.word.length,
          },
        },
      ],
      link_confidence: "direct",
    }),
  ).catch(() => {
    emittedStaleDiagnosticKeys.delete(key);
  });
}

async function fileLensEntryForSymbol(
  workspaceId: string,
  symbol: CodeSymbolNavProjection,
): Promise<CodeFileLensEntry | null> {
  const inputs = fileLensInputs(symbol);
  if (!inputs) return null;
  try {
    const lens = await getCodeFileLens(workspaceId, inputs.relativePath, inputs.contentHash, inputs.parserVersion);
    return lens.entries.find((entry) => entry.symbol_entity_id === symbol.symbol_entity_id) ?? null;
  } catch {
    return null;
  }
}

function markdownForSymbol(symbol: CodeSymbolNavProjection, doc: string | null = null): Monaco.IMarkdownString {
  const lines = [
    `**${symbol.display_name}**`,
    "",
    `Kind: \`${symbol.symbol_kind}\``,
    `Symbol: \`${symbol.symbol_key}\``,
    `Staleness: \`${codeSymbolStalenessLabel(symbol.staleness)}\``,
  ];
  if (doc) {
    lines.push("", doc);
  }
  return {
    value: lines.join("\n"),
  };
}

function collectIdentifiers(model: TextModel, limit = LENS_IDENTIFIER_LIMIT): Array<{ word: string; line: number; column: number }> {
  const identifiers = new Map<string, { word: string; line: number; column: number }>();
  const lines = model.getValue().split(/\r?\n/);
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    const matcher = /\b[A-Za-z_][A-Za-z0-9_]*\b/g;
    let match: RegExpExecArray | null;
    while ((match = matcher.exec(line)) !== null) {
      const word = match[0];
      if (!identifiers.has(word)) {
        identifiers.set(word, { word, line: lineIndex + 1, column: match.index + 1 });
      }
      if (identifiers.size >= limit) return Array.from(identifiers.values());
    }
  }
  return Array.from(identifiers.values());
}

export function ensureHandshakeCodeIntelligenceProviders(monacoApi: MonacoApi): void {
  if (providerDisposables.length > 0) return;

  for (const languageId of supportedLanguageIds()) {
    providerDisposables.push(
      monacoApi.languages.registerCompletionItemProvider(languageId, {
        triggerCharacters: [".", ":", "_"],
        async provideCompletionItems(model, position) {
          const workspaceId = activeWorkspaceId();
          const currentWord = wordUntilPosition(model, position);
          if (!workspaceId || !currentWord || currentWord.word.length < 2) {
            return { suggestions: [] };
          }
          const response = await lookupCodeSymbols({
            workspaceId,
            prefix: currentWord.word,
            limit: SYMBOL_LOOKUP_LIMIT,
          });
          const range = new monacoApi.Range(
            position.lineNumber,
            currentWord.startColumn,
            position.lineNumber,
            currentWord.endColumn,
          );
          return {
            suggestions: response.matches.map((symbol) => ({
              label: symbol.display_name,
              kind: completionKind(monacoApi, symbol.symbol_kind),
              insertText: symbol.display_name,
              detail: symbol.symbol_kind,
              documentation: markdownForSymbol(symbol),
              range,
            })),
          };
        },
      }),
      monacoApi.languages.registerHoverProvider(languageId, {
        async provideHover(model, position) {
          const workspaceId = activeWorkspaceId();
          const currentWord = wordAtPosition(model, position);
          if (!workspaceId || !currentWord) return null;
          const symbol = await lookupFirstSymbol(workspaceId, currentWord.word);
          if (!symbol) return null;
          const fileLensEntry = await fileLensEntryForSymbol(workspaceId, symbol);
          return {
            contents: [markdownForSymbol(symbol, fileLensEntry?.doc ?? null)],
            range: new monacoApi.Range(
              position.lineNumber,
              currentWord.startColumn,
              position.lineNumber,
              currentWord.endColumn,
            ),
          };
        },
      }),
      monacoApi.languages.registerDefinitionProvider(languageId, {
        async provideDefinition(model, position) {
          const workspaceId = activeWorkspaceId();
          const currentWord = wordAtPosition(model, position);
          if (!workspaceId || !currentWord) return null;
          const symbol = await lookupFirstSymbol(workspaceId, currentWord.word);
          if (!symbol?.definition) return null;
          return {
            uri: model.uri,
            range: rangeForDefinition(monacoApi, symbol.definition),
          };
        },
      }),
      monacoApi.languages.registerReferenceProvider(languageId, {
        async provideReferences(model, position) {
          const workspaceId = activeWorkspaceId();
          const currentWord = wordAtPosition(model, position);
          if (!workspaceId || !currentWord) return [];
          const symbol = await lookupFirstSymbol(workspaceId, currentWord.word);
          if (!symbol) return [];
          const references = await getCodeSymbolReferences(symbol.symbol_entity_id);
          return [...references.callers, ...references.callees]
            .flatMap((reference) => reference.evidence_spans)
            .map((span) => rangeForSpan(monacoApi, span))
            .filter((range): range is Monaco.Range => range !== null)
            .map((range) => ({ uri: model.uri, range }));
        },
      }),
      monacoApi.languages.registerDocumentFormattingEditProvider(languageId, {
        provideDocumentFormattingEdits(model) {
          const current = model.getValue();
          const formatted = current
            .split(/\r?\n/)
            .map((line) => line.replace(/[ \t]+$/g, ""))
            .join("\n");
          if (formatted === current) return [];
          return [{ range: model.getFullModelRange(), text: formatted }];
        },
      }),
      monacoApi.languages.registerCodeLensProvider(languageId, {
        async provideCodeLenses(model) {
          const workspaceId = activeWorkspaceId();
          if (!workspaceId) return { lenses: [], dispose() {} };
          const identifiers = collectIdentifiers(model);
          const results = await Promise.all(
            identifiers.map(async (identifier) => ({
              identifier,
              symbol: await lookupFirstSymbol(workspaceId, identifier.word),
            })),
          );
          return {
            lenses: results
              .filter((item): item is { identifier: { word: string; line: number; column: number }; symbol: CodeSymbolNavProjection } =>
                item.symbol !== null,
              )
              .map(({ identifier, symbol }) => ({
                range: new monacoApi.Range(
                  identifier.line,
                  identifier.column,
                  identifier.line,
                  identifier.column + identifier.word.length,
                ),
                id: symbol.symbol_entity_id,
                command: {
                  id: HANDSHAKE_OPEN_CODE_SYMBOL_COMMAND,
                  title: `Open code symbol: ${symbol.display_name} (${codeSymbolStalenessLabel(symbol.staleness)})`,
                  arguments: [symbol.symbol_entity_id],
                },
              })),
            dispose() {},
          };
        },
      }),
    );

    if (monacoApi.languages.registerInlayHintsProvider) {
      providerDisposables.push(
        monacoApi.languages.registerInlayHintsProvider(languageId, {
          async provideInlayHints(model) {
            const workspaceId = activeWorkspaceId();
            if (!workspaceId) return { hints: [], dispose() {} };
            const identifiers = collectIdentifiers(model, 8);
            const results = await Promise.all(
              identifiers.map(async (identifier) => ({
                identifier,
                symbol: await lookupFirstSymbol(workspaceId, identifier.word),
              })),
            );
            return {
              hints: results
                .filter((item): item is { identifier: { word: string; line: number; column: number }; symbol: CodeSymbolNavProjection } =>
                  item.symbol !== null,
                )
                .map(({ identifier, symbol }) => ({
                  position: {
                    lineNumber: identifier.line,
                    column: identifier.column + identifier.word.length,
                  },
                  label: `: ${symbol.symbol_kind}`,
                  kind: monacoApi.languages.InlayHintKind.Type,
                  tooltip: `Staleness: ${codeSymbolStalenessLabel(symbol.staleness)}`,
                })),
              dispose() {},
            };
          },
        }),
      );
    }
  }
}

export function installHandshakeCodeIntelligenceEditorActions(editor: StandaloneEditor): IDisposable | null {
  if (!editor.addAction) return null;
  return editor.addAction({
    id: HANDSHAKE_OPEN_CODE_SYMBOL_COMMAND,
    label: "Open Handshake code symbol",
    run: (_editor, symbolEntityId?: unknown) => {
      if (typeof symbolEntityId === "string") {
        config.openCodeSymbol?.(symbolEntityId);
      }
    },
  });
}

export async function refreshHandshakeCodeIntelligenceMarkers(
  editor: StandaloneEditor,
  monacoApi: MonacoApi,
): Promise<void> {
  const model = editor.getModel();
  if (!model) return;
  const setModelMarkers = (monacoApi.editor as {
    setModelMarkers?: (model: TextModel, owner: string, markers: Monaco.editor.IMarkerData[]) => void;
  }).setModelMarkers;
  if (!setModelMarkers) return;
  const workspaceId = activeWorkspaceId();
  if (!workspaceId) {
    setModelMarkers(model, MARKER_OWNER, []);
    return;
  }
  const identifiers = collectIdentifiers(model, LENS_IDENTIFIER_LIMIT);
  const results = await Promise.all(
    identifiers.map(async (identifier) => ({
      identifier,
      symbol: await lookupFirstSymbol(workspaceId, identifier.word),
    })),
  );
  const markers: Monaco.editor.IMarkerData[] = [];
  for (const { identifier, symbol } of results) {
    if (!symbol || !symbol.staleness || typeof symbol.staleness !== "object") continue;
    const fresh = (symbol.staleness as Record<string, unknown>).fresh === true;
    if (fresh) continue;
    markers.push({
      severity: monacoApi.MarkerSeverity.Warning,
      message: `Code index is ${codeSymbolStalenessLabel(symbol.staleness)} for ${symbol.display_name}.`,
      startLineNumber: identifier.line,
      startColumn: identifier.column,
      endLineNumber: identifier.line,
      endColumn: identifier.column + identifier.word.length,
    });
    emitStaleCodeIntelligenceDiagnostic(workspaceId, model, identifier, symbol);
  }
  setModelMarkers(model, MARKER_OWNER, markers);
}
