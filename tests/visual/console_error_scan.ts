const { expect, test: base } = require("@playwright/test");
const fs = require("node:fs/promises");
const path = require("node:path");

type CDPSession = import("playwright").CDPSession;
type Page = import("playwright").Page;
type TestInfo = import("playwright").TestInfo;

type ConsoleScanSeverity = "warning" | "error";
type ConsoleScanSource =
  | "cdp_runtime_console"
  | "cdp_runtime_exception"
  | "playwright_pageerror"
  | "scan_summary";

type ConsoleScanEvent = {
  schema_id: "hsk.visual.console_error_scan_event@1";
  event_type: "CONSOLE_SCAN_EVENT";
  timestamp_utc: string;
  source: ConsoleScanSource;
  severity: ConsoleScanSeverity;
  message: string;
  url?: string;
  line_number?: number;
  column_number?: number;
  stack?: string;
};

type ConsoleScanSummary = {
  schema_id: "hsk.visual.console_error_scan_summary@1";
  event_type: "CONSOLE_SCAN_SUMMARY";
  timestamp_utc: string;
  test_title: string;
  total_events: number;
  warning_events: number;
  error_events: number;
};

type ConsoleErrorScanOptions = {
  failOnWarnings?: boolean;
};

const ERROR_CONSOLE_TYPES = new Set(["error", "warning"]);

class ConsoleErrorScan {
  private readonly eventsBuffer: ConsoleScanEvent[] = [];
  private readonly failOnWarnings: boolean;
  private cdpSession: CDPSession | null = null;
  private subscribed = false;

  private readonly pageErrorListener = (error: Error) => {
    this.record({
      source: "playwright_pageerror",
      severity: "error",
      message: error.message || String(error),
      stack: error.stack,
    });
  };

  private readonly cdpConsoleListener = (payload: RuntimeConsoleApiCalledPayload) => {
    if (!ERROR_CONSOLE_TYPES.has(payload.type)) return;
    this.record({
      source: "cdp_runtime_console",
      severity: payload.type === "warning" ? "warning" : "error",
      message: consolePayloadMessage(payload),
      stack: payload.stackTrace ? JSON.stringify(payload.stackTrace) : undefined,
    });
  };

  private readonly cdpExceptionListener = (payload: RuntimeExceptionThrownPayload) => {
    this.record({
      source: "cdp_runtime_exception",
      severity: "error",
      message: payload.exceptionDetails?.text
        ?? payload.exceptionDetails?.exception?.description
        ?? payload.exceptionDetails?.exception?.value
        ?? "Runtime.exceptionThrown",
      url: payload.exceptionDetails?.url,
      line_number: payload.exceptionDetails?.lineNumber,
      column_number: payload.exceptionDetails?.columnNumber,
      stack: payload.exceptionDetails?.stackTrace
        ? JSON.stringify(payload.exceptionDetails.stackTrace)
        : payload.exceptionDetails?.exception?.description,
    });
  };

  constructor(
    private readonly page: Page,
    private readonly options: ConsoleErrorScanOptions = {},
  ) {
    this.failOnWarnings = options.failOnWarnings ?? true;
  }

  async subscribe(): Promise<void> {
    if (this.subscribed) return;
    this.subscribed = true;
    this.page.on("pageerror", this.pageErrorListener);
    this.cdpSession = await this.page.context().newCDPSession(this.page);
    this.cdpSession.on("Runtime.consoleAPICalled", this.cdpConsoleListener);
    this.cdpSession.on("Runtime.exceptionThrown", this.cdpExceptionListener);
    await this.cdpSession.send("Runtime.enable");
  }

  async dispose(): Promise<void> {
    this.page.removeListener("pageerror", this.pageErrorListener);
    const session = this.cdpSession;
    this.cdpSession = null;
    this.subscribed = false;
    if (!session) return;
    session.off("Runtime.consoleAPICalled", this.cdpConsoleListener);
    session.off("Runtime.exceptionThrown", this.cdpExceptionListener);
    await session.detach().catch(() => undefined);
  }

  events(): readonly ConsoleScanEvent[] {
    return this.eventsBuffer;
  }

  assertNoErrors(): void {
    const blockingEvents = this.eventsBuffer.filter((event) => (
      event.severity === "error" || (this.failOnWarnings && event.severity === "warning")
    ));
    if (blockingEvents.length === 0) return;
    const detail = blockingEvents
      .map((event) => `${event.severity.toUpperCase()} ${event.source}: ${event.message}`)
      .join("\n");
    throw new Error(`Renderer console/runtime scan captured ${blockingEvents.length} blocking event(s):\n${detail}`);
  }

  async writeJsonl(testInfo: Pick<TestInfo, "title" | "outputPath">): Promise<string> {
    const logPath = testInfo.outputPath("console-error-scan.jsonl");
    await fs.mkdir(path.dirname(logPath), { recursive: true });
    const summary: ConsoleScanSummary = {
      schema_id: "hsk.visual.console_error_scan_summary@1",
      event_type: "CONSOLE_SCAN_SUMMARY",
      timestamp_utc: new Date().toISOString(),
      test_title: testInfo.title,
      total_events: this.eventsBuffer.length,
      warning_events: this.eventsBuffer.filter((event) => event.severity === "warning").length,
      error_events: this.eventsBuffer.filter((event) => event.severity === "error").length,
    };
    const rows = [summary, ...this.eventsBuffer]
      .map((entry) => JSON.stringify(entry))
      .join("\n");
    await fs.writeFile(logPath, `${rows}\n`, "utf8");
    return logPath;
  }

  private record(event: Omit<ConsoleScanEvent, "schema_id" | "event_type" | "timestamp_utc">): void {
    this.eventsBuffer.push({
      schema_id: "hsk.visual.console_error_scan_event@1",
      event_type: "CONSOLE_SCAN_EVENT",
      timestamp_utc: new Date().toISOString(),
      ...event,
    });
  }
}

const test = base.extend({
  consoleScan: [async ({ page }, use, testInfo) => {
    const scan = new ConsoleErrorScan(page);
    await scan.subscribe();
    let bodyError: unknown = null;
    try {
      await use(scan);
    } catch (error) {
      bodyError = error;
    } finally {
      await scan.writeJsonl(testInfo);
      await scan.dispose();
    }
    if (bodyError) throw bodyError;
    scan.assertNoErrors();
  }, { auto: true }],
});

function defaultConsoleScanArtifactRoot(): string {
  const repoRoot = path.resolve(__dirname, "..", "..");
  return process.env.HANDSHAKE_ARTIFACT_ROOT
    ?? path.resolve(repoRoot, "..", "Handshake_Artifacts");
}

type RuntimeConsoleApiCalledPayload = {
  type: string;
  args?: Array<{ value?: unknown; description?: string; unserializableValue?: string }>;
  stackTrace?: unknown;
};

type RuntimeExceptionThrownPayload = {
  exceptionDetails?: {
    text?: string;
    url?: string;
    lineNumber?: number;
    columnNumber?: number;
    stackTrace?: unknown;
    exception?: {
      description?: string;
      value?: string;
    };
  };
};

function consolePayloadMessage(payload: RuntimeConsoleApiCalledPayload): string {
  const values = (payload.args ?? [])
    .map((arg) => {
      if (arg.value !== undefined) return String(arg.value);
      if (arg.description) return arg.description;
      if (arg.unserializableValue) return arg.unserializableValue;
      return "";
    })
    .filter(Boolean);
  return values.join(" ") || `console.${payload.type}`;
}

module.exports = {
  ConsoleErrorScan,
  defaultConsoleScanArtifactRoot,
  expect,
  test,
};
