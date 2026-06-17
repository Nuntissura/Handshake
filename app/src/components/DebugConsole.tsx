// WP-KERNEL-009 MT-254 DebugAdapterCore — debug console.
//
// The operator's debug REPL: type an expression, evaluate it in the currently
// paused frame via the live DebugSession (REAL Debugger.evaluateOnCallFrame),
// and see the result. Also renders the streamed dap:// output lines (stdout /
// stderr / console.*). No mock: every result comes from the running debuggee.

import { useState } from "react";

export type DebugConsoleEntry =
  | { kind: "input"; text: string }
  | { kind: "result"; text: string }
  | { kind: "error"; text: string }
  | { kind: "output"; category: string; text: string };

export type DebugConsoleProps = {
  /** Output + eval log, newest last. */
  entries: DebugConsoleEntry[];
  /** True when the session is paused on a frame (eval requires a paused frame). */
  canEvaluate: boolean;
  /** Evaluate an expression in the current paused frame. */
  onEvaluate: (expression: string) => void | Promise<void>;
};

export function DebugConsole({ entries, canEvaluate, onEvaluate }: DebugConsoleProps) {
  const [expression, setExpression] = useState("");

  const submit = async () => {
    const trimmed = expression.trim();
    if (!trimmed || !canEvaluate) return;
    setExpression("");
    await onEvaluate(trimmed);
  };

  return (
    <section className="debug-console" data-testid="debug-console" aria-label="Debug console">
      <header className="debug-console__header">Debug Console</header>
      <ol className="debug-console__log" data-testid="debug-console.log">
        {entries.map((entry, index) => (
          <li
            key={index}
            className={`debug-console__entry debug-console__entry--${entry.kind}`}
            data-testid={`debug-console.entry.${entry.kind}`}
            data-entry-kind={entry.kind}
          >
            {entry.kind === "input" ? `> ${entry.text}` : entry.text}
          </li>
        ))}
      </ol>
      <div className="debug-console__prompt">
        <input
          type="text"
          className="debug-console__input"
          data-testid="debug-console.input"
          placeholder={canEvaluate ? "Evaluate in paused frame…" : "Pause to evaluate"}
          value={expression}
          disabled={!canEvaluate}
          onChange={(event) => setExpression(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void submit();
            }
          }}
        />
        <button
          type="button"
          className="debug-console__eval"
          data-testid="debug-console.eval"
          disabled={!canEvaluate || expression.trim().length === 0}
          onClick={() => void submit()}
        >
          Evaluate
        </button>
      </div>
    </section>
  );
}
