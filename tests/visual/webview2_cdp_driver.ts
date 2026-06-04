import { chromium } from "playwright";
import type {
  Browser,
  Page,
  PageScreenshotOptions,
} from "playwright";
import type { ChildProcess } from "node:child_process";

const DEFAULT_CONNECT_TIMEOUT_MS = 30_000;
const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

type ForbiddenReadOnlyPattern = {
  pattern: RegExp;
  reason: string;
};

const FORBIDDEN_READ_ONLY_PATTERNS: ForbiddenReadOnlyPattern[] = [
  { pattern: /\b(?:document|window)\.location\s*=/, reason: "location mutation" },
  { pattern: /\bdocument\.write\s*\(/, reason: "document.write mutation" },
  { pattern: /\b(?:localStorage|sessionStorage)\.setItem\s*\(/, reason: "browser storage write" },
  { pattern: /\b(?:appendChild|removeChild|replaceChild|insertBefore)\s*\(/, reason: "DOM tree mutation" },
  { pattern: /\b(?:innerHTML|outerHTML|textContent|value)\s*=/, reason: "DOM property assignment" },
  { pattern: /\b(?:click|focus|blur|dispatchEvent)\s*\(/, reason: "synthetic user interaction" },
  { pattern: /\b(?:fetch|XMLHttpRequest)\b/, reason: "network side effect" },
];

export type WebView2CdpDriverOptions = {
  endpointUrl?: string;
  remoteDebuggingPort?: number;
  connectTimeoutMs?: number;
  verifyTauriIpcPort?: boolean;
};

export type ReadOnlyPage = {
  screenshot(options?: PageScreenshotOptions): Promise<Buffer>;
  evaluate<T>(expression: string): Promise<T>;
  locatorCount(selector: string): Promise<number>;
  textContent(selector: string): Promise<string | null>;
  visualDebugPortViaTauriIpc(): Promise<number | null>;
};

export class WebView2CdpDriver {
  private browser: Browser | null = null;
  private page: Page | null = null;

  constructor(private readonly options: WebView2CdpDriverOptions = {}) {}

  static endpointFromEnv(env: NodeJS.ProcessEnv = process.env): string | null {
    const endpoint = env.HANDSHAKE_WEBVIEW2_CDP_ENDPOINT?.trim();
    if (endpoint) return normalizeEndpoint(endpoint);

    const rawPort = env.HANDSHAKE_WEBVIEW2_CDP_PORT?.trim();
    if (!rawPort) return null;
    const port = parsePort(rawPort);
    return endpointForPort(port);
  }

  static readOnly(page: Page): ReadOnlyPage {
    return buildReadOnlyPage(page);
  }

  async connect(handshakeProcess?: ChildProcess): Promise<Page> {
    const endpointUrl = this.resolveEndpoint(handshakeProcess);
    const expectedPort = portFromEndpoint(endpointUrl);
    this.browser = await chromium.connectOverCDP(endpointUrl, {
      timeout: this.options.connectTimeoutMs ?? DEFAULT_CONNECT_TIMEOUT_MS,
      isLocal: true,
      noDefaults: true,
    });

    const context = this.browser.contexts()[0];
    if (!context) {
      throw new Error("WebView2 CDP connection did not expose a default browser context");
    }
    const page = context.pages()[0];
    if (!page) {
      throw new Error("WebView2 CDP connection did not expose the main WebView2 page");
    }

    this.page = page;
    if (this.options.verifyTauriIpcPort) {
      const tauriPort = await this.visualDebugPortViaTauriIpc();
      if (tauriPort !== expectedPort) {
        throw new Error(
          `Tauri IPC kernel_visual_debug_port returned ${tauriPort ?? "null"}, expected ${expectedPort}`,
        );
      }
    }
    return page;
  }

  readOnly(page: Page | null = this.page): ReadOnlyPage {
    if (!page) {
      throw new Error("WebView2CdpDriver.readOnly() requires a connected page");
    }
    return buildReadOnlyPage(page);
  }

  async visualDebugPortViaTauriIpc(page: Page | null = this.page): Promise<number | null> {
    if (!page) {
      throw new Error("visualDebugPortViaTauriIpc() requires a connected page");
    }
    return readVisualDebugPortViaTauriIpc(page);
  }

  async disconnect(): Promise<void> {
    const browser = this.browser;
    this.browser = null;
    this.page = null;
    if (browser) {
      await browser.close();
    }
  }

  private resolveEndpoint(handshakeProcess?: ChildProcess): string {
    if (this.options.endpointUrl) {
      return normalizeEndpoint(this.options.endpointUrl);
    }
    if (this.options.remoteDebuggingPort !== undefined) {
      return endpointForPort(this.options.remoteDebuggingPort);
    }
    const envEndpoint = WebView2CdpDriver.endpointFromEnv();
    if (envEndpoint) return envEndpoint;

    const processPort = parsePortFromProcessArgs(handshakeProcess);
    if (processPort !== null) return endpointForPort(processPort);

    throw new Error(
      "WebView2 CDP endpoint is unavailable. Start Handshake with MT-018 WebView2 CDP enabled "
      + "and pass HANDSHAKE_WEBVIEW2_CDP_PORT, HANDSHAKE_WEBVIEW2_CDP_ENDPOINT, "
      + "or a ChildProcess whose spawn arguments include --remote-debugging-port=<port>.",
    );
  }
}

export function isPngBuffer(bytes: Buffer | Uint8Array): boolean {
  if (bytes.byteLength < PNG_SIGNATURE.byteLength) return false;
  const buffer = Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes);
  return PNG_SIGNATURE.every((value, index) => buffer[index] === value);
}

export function assertReadOnlyExpression(expression: string): void {
  const source = expression.trim();
  if (!source) {
    throw new Error("readOnly.evaluate() requires a non-empty expression");
  }
  const violation = FORBIDDEN_READ_ONLY_PATTERNS.find((entry) => entry.pattern.test(source));
  if (violation) {
    throw new Error(`readOnly.evaluate() rejected ${violation.reason}: ${source}`);
  }
}

async function readVisualDebugPortViaTauriIpc(page: Page): Promise<number | null> {
  return page.evaluate(async () => {
    const win = window as unknown as {
      __TAURI__?: { core?: { invoke?: (command: string) => Promise<unknown> } };
      __TAURI_INTERNALS__?: { invoke?: (command: string) => Promise<unknown> };
    };
    const invoke = win.__TAURI__?.core?.invoke ?? win.__TAURI_INTERNALS__?.invoke;
    if (!invoke) return null;
    for (const command of ["kernel_visual_debug_port", "kernel.visual_debug.port"]) {
      try {
        const result = await invoke(command);
        return typeof result === "number" ? result : Number(result);
      } catch {
        // Try the legacy/dotted command alias before reporting unavailable.
      }
    }
    return null;
  });
}

function buildReadOnlyPage(page: Page): ReadOnlyPage {
  return {
    async screenshot(options = {}) {
      const bytes = await page.screenshot({ type: "png", ...options });
      if (!isPngBuffer(bytes)) {
        throw new Error("Playwright screenshot did not return PNG bytes");
      }
      return bytes;
    },
    async evaluate<T>(expression: string) {
      assertReadOnlyExpression(expression);
      return page.evaluate<T>(expression);
    },
    locatorCount(selector: string) {
      return page.locator(selector).count();
    },
    textContent(selector: string) {
      return page.locator(selector).first().textContent();
    },
    visualDebugPortViaTauriIpc() {
      return readVisualDebugPortViaTauriIpc(page);
    },
  };
}

function normalizeEndpoint(endpoint: string): string {
  const trimmed = endpoint.trim();
  if (!trimmed) throw new Error("CDP endpoint must not be empty");
  if (/^https?:\/\//i.test(trimmed) || /^wss?:\/\//i.test(trimmed)) return trimmed;
  return endpointForPort(parsePort(trimmed));
}

function endpointForPort(port: number): string {
  return `http://127.0.0.1:${port}`;
}

function parsePort(value: string | number): number {
  const port = Number(value);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error(`Invalid WebView2 CDP port: ${value}`);
  }
  return port;
}

function parsePortFromProcessArgs(handshakeProcess?: ChildProcess): number | null {
  const args = handshakeProcess?.spawnargs ?? [];
  const text = args.join(" ");
  const match = text.match(/--remote-debugging-port[= ](\d{1,5})/);
  return match ? parsePort(match[1]) : null;
}

function portFromEndpoint(endpointUrl: string): number {
  try {
    return parsePort(new URL(endpointUrl).port);
  } catch (error) {
    throw new Error(`Unable to parse CDP endpoint port from ${endpointUrl}: ${error}`);
  }
}
