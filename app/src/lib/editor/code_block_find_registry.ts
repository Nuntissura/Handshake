// WP-KERNEL-009 / MT-244 — code-block find registry (Monaco reveal bridge).
//
// A ProseMirror inline decoration cannot reach INSIDE an atom node's Monaco
// model, so in-code-block match reveal/selection is owned by the NodeView:
// every mounted MonacoCodeBlockView registers a handle here (keyed by its
// live getPos thunk), and the FindReplacePanel asks the registry to reveal a
// {start,end} character range when the current match lives in a code block.
// The handle reveals through the real Monaco instance when mounted, or the
// degraded <textarea> fallback otherwise — the same fail-soft ladder the view
// itself uses (MT-165).
//
// Module-level registry (not React context) because ProseMirror NodeViews and
// the panel live in different React trees; entries unregister on unmount so
// the set never leaks across documents.

export interface CodeBlockFindHandle {
  /** Live position thunk of the owning monacoCodeBlock node. */
  getPos: () => number | undefined;
  /** Reveals + selects [start,end) (character offsets into the code text). */
  reveal: (start: number, end: number) => void;
}

const handles = new Set<CodeBlockFindHandle>();

/** Registers a NodeView handle; returns the unregister disposer. */
export function registerCodeBlockFindHandle(handle: CodeBlockFindHandle): () => void {
  handles.add(handle);
  return () => handles.delete(handle);
}

/**
 * Reveals a character range inside the code block at `nodePos`. Returns true
 * when a mounted NodeView owned the position and performed the reveal.
 */
export function revealCodeBlockRange(nodePos: number, start: number, end: number): boolean {
  for (const handle of handles) {
    let pos: number | undefined;
    try {
      pos = handle.getPos();
    } catch {
      continue;
    }
    if (pos === nodePos) {
      handle.reveal(start, end);
      return true;
    }
  }
  return false;
}

/** Test/debug introspection: number of live registered code-block handles. */
export function codeBlockFindHandleCount(): number {
  return handles.size;
}
