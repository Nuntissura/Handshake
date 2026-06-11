// WP-KERNEL-009 / MT-031 — DependencyFailureMessaging proof.
//
// Mocks are used ONLY to simulate FAILURE conditions (operator rule): the
// banner, the registry, the Monaco setup guard, the WP-009 Tiptap extension
// guard, and the TiptapEditor document guard are all the REAL product
// modules. The tests prove:
//   1. a simulated Monaco worker-construction failure flows through the real
//      setup guard into a visible, actionable banner (no blank screen);
//   2. a failing optional Tiptap extension flows through the real
//      buildWp009ExtensionSet guard into the banner while the set still
//      builds (degraded mode, core editor keeps booting);
//   3. a failing document-editor extension set makes TiptapEditor render
//      nothing while the banner at the mount site explains the failure;
//   4. the banner stays silent with no failures, dismisses on demand, and
//      re-surfaces on new failures.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DependencyFailureBanner } from "./DependencyFailureBanner";
import {
  TiptapEditor,
  setTiptapDocumentExtensionFactoryForTests,
} from "./TiptapEditor";
import { dependencyFailures } from "../lib/dependency_policy/dependency_failure";
import { buildWp009ExtensionSet } from "../lib/tiptap/extension_set";
import type { Wp009ExtensionSetOptions } from "../lib/tiptap/extension_set";

beforeEach(() => {
  dependencyFailures.clear();
});

afterEach(() => {
  setTiptapDocumentExtensionFactoryForTests(null);
  dependencyFailures.clear();
  cleanup();
});

describe("MT-031 dependency failure banner", () => {
  it("renders nothing while no bundled dependency has failed", () => {
    render(<DependencyFailureBanner />);
    expect(screen.queryByTestId("dependency-failure-banner")).toBeNull();
  });

  it("surfaces a simulated Monaco worker-construction failure with actionable text", { timeout: 120_000 }, async () => {
    // jsdom gaps monaco-editor touches at MODULE INIT (environment shims, not
    // behavior mocks): clipboard capability probe + media-query probe.
    const doc = document as Document & { queryCommandSupported?: (cmd: string) => boolean };
    doc.queryCommandSupported ??= () => false;
    (window as Window & { matchMedia?: (q: string) => Partial<MediaQueryList> }).matchMedia ??= (
      query: string,
    ) =>
      ({
        matches: false,
        media: query,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        addListener: () => undefined,
        removeListener: () => undefined,
        onchange: null,
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList;

    // Real product guard: setMonacoWorkerFactoryForTests is the seam inside
    // src/lib/monaco/setup.ts; the thrown error takes the exact production
    // reporting path (constructWorker -> dependencyFailures.report). The
    // monaco-editor module graph is large; the first vitest transform of it
    // dominates this test's runtime — hence the generous timeout.
    const { ensureMonacoEnvironment, setMonacoWorkerFactoryForTests } = await import(
      "../lib/monaco/setup"
    );
    render(<DependencyFailureBanner />);
    setMonacoWorkerFactoryForTests(() => {
      throw new Error("simulated worker bootstrap failure");
    });
    try {
      ensureMonacoEnvironment();
      const environment = (
        globalThis as {
          MonacoEnvironment?: { getWorker(id: string, label: string): Worker };
        }
      ).MonacoEnvironment;
      expect(environment).toBeDefined();
      act(() => {
        expect(() => environment!.getWorker("workerMain", "typescript")).toThrow(
          "simulated worker bootstrap failure",
        );
      });
    } finally {
      setMonacoWorkerFactoryForTests(null);
    }

    const banner = await screen.findByTestId("dependency-failure-banner");
    // Actionable: names the dependency + component, states degraded mode and
    // the bundled/no-download recovery path. Never a blank screen.
    expect(banner.textContent).toContain("monaco-editor");
    expect(banner.textContent).toContain("worker:typescript");
    expect(banner.textContent).toContain("degraded mode");
    expect(banner.textContent).toContain("Restart Handshake");
    expect(banner.textContent).toContain("cause: simulated worker bootstrap failure");
  });

  it("surfaces a failing optional Tiptap extension while the set still builds (degraded mode)", async () => {
    render(<DependencyFailureBanner />);
    // Failure-only simulation: an options object whose mention source blows
    // up INSIDE the real guarded factory of buildWp009ExtensionSet.
    const faultyOptions = {} as Wp009ExtensionSetOptions;
    Object.defineProperty(faultyOptions, "mentionItems", {
      get() {
        throw new Error("simulated mention extension failure");
      },
    });
    let extensions: ReturnType<typeof buildWp009ExtensionSet> = [];
    act(() => {
      extensions = buildWp009ExtensionSet(faultyOptions);
    });
    // Degraded, not dead: the other extensions still constructed.
    expect(extensions.length).toBeGreaterThan(3);

    const banner = await screen.findByTestId("dependency-failure-banner");
    expect(banner.textContent).toContain("@tiptap/extension-mention");
    expect(banner.textContent).toContain("extension_init");
    expect(banner.textContent).toContain("cause: simulated mention extension failure");
  });

  it("renders the banner instead of a blank editor when the document extension set fails", async () => {
    setTiptapDocumentExtensionFactoryForTests(() => {
      throw new Error("simulated starter-kit failure");
    });
    const { container } = render(
      <div>
        {/* Same composition as the DocumentView mount site. */}
        <DependencyFailureBanner />
        <TiptapEditor initialContent={null} onChange={() => undefined} />
      </div>,
    );

    const banner = await screen.findByTestId("dependency-failure-banner");
    expect(banner.textContent).toContain("@tiptap/starter-kit");
    expect(banner.textContent).toContain("cause: simulated starter-kit failure");
    // The editor surface did not boot a broken schema (no .tiptap-editor),
    // and the mount site is NOT blank: the banner occupies it.
    expect(container.querySelector(".tiptap-editor")).toBeNull();
    expect(container.textContent).toContain("What you can do");
  });

  it("dismisses on demand and re-surfaces on the next failure", async () => {
    render(<DependencyFailureBanner />);
    act(() => {
      dependencyFailures.report({
        dependency: "monaco-editor",
        component: "worker:json",
        phase: "worker_construction",
        message: "Bundled dependency failed to load: monaco-editor (worker:json, worker_construction).",
      });
    });
    const banner = await screen.findByTestId("dependency-failure-banner");
    expect(banner).toBeVisible();

    fireEvent.click(screen.getByTestId("dependency-failure-banner-dismiss"));
    expect(screen.queryByTestId("dependency-failure-banner")).toBeNull();

    act(() => {
      dependencyFailures.report({
        dependency: "monaco-editor",
        component: "worker:css",
        phase: "worker_construction",
        message: "Bundled dependency failed to load: monaco-editor (worker:css, worker_construction).",
      });
    });
    const resurfaced = await screen.findByTestId("dependency-failure-banner");
    expect(resurfaced.textContent).toContain("worker:css");
  });
});
