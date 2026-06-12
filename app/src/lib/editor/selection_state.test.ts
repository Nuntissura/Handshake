// WP-KERNEL-009 / MT-171 — EditorSelectionAndCursorState tests.
//
// Proves actor-attributed selection snapshots carry the range + ownership, apply
// the privacy policy (non-operator text redacted by default; operator text
// shared), drop presence entirely in quiet mode, and merge into a stable
// operator-first attributable list.

import { describe, it, expect } from "vitest";
import {
  buildSelectionSnapshot,
  snapshotFromOffsets,
  mergePresence,
  DEFAULT_PRESENCE_POLICY,
  type SelectionActor,
} from "./selection_state";

const OPERATOR: SelectionActor = { kind: "operator", id: "op-1", label: "Operator" };
const MODEL: SelectionActor = { kind: "cloud_model", id: "m-9", label: "Cloud model" };

const SEL = { text: "secret text", startUtf8: 4, endUtf8: 15, collapsed: false };

describe("selection/cursor state (MT-171)", () => {
  it("shares the operator's own selection text and range", () => {
    const snap = buildSelectionSnapshot(OPERATOR, SEL);
    expect(snap).not.toBeNull();
    expect(snap?.text).toBe("secret text");
    expect(snap?.startUtf8).toBe(4);
    expect(snap?.endUtf8).toBe(15);
    expect(snap?.redactedReason).toBe("none");
  });

  it("redacts a non-operator actor's text by default but keeps the range", () => {
    const snap = buildSelectionSnapshot(MODEL, SEL);
    expect(snap?.text).toBeNull();
    expect(snap?.redactedReason).toBe("privacy_non_operator");
    // Range is still attributable for cursor rendering.
    expect(snap?.startUtf8).toBe(4);
    expect(snap?.actor.id).toBe("m-9");
  });

  it("shares non-operator text only when the policy permits it", () => {
    const snap = buildSelectionSnapshot(MODEL, SEL, {
      ...DEFAULT_PRESENCE_POLICY,
      shareNonOperatorText: true,
    });
    expect(snap?.text).toBe("secret text");
    expect(snap?.redactedReason).toBe("none");
  });

  it("drops all presence in quiet mode", () => {
    expect(
      buildSelectionSnapshot(OPERATOR, SEL, { ...DEFAULT_PRESENCE_POLICY, quietMode: true }),
    ).toBeNull();
    expect(
      buildSelectionSnapshot(MODEL, SEL, { ...DEFAULT_PRESENCE_POLICY, quietMode: true }),
    ).toBeNull();
  });

  it("snapshotFromOffsets marks a collapsed caret", () => {
    const caret = snapshotFromOffsets(OPERATOR, "", 7, 7);
    expect(caret?.collapsed).toBe(true);
  });

  it("merges presence operator-first and drops nulls", () => {
    const merged = mergePresence([
      buildSelectionSnapshot(MODEL, SEL),
      null,
      buildSelectionSnapshot(OPERATOR, SEL),
    ]);
    expect(merged).toHaveLength(2);
    expect(merged[0].actor.kind).toBe("operator");
    expect(merged[1].actor.kind).toBe("cloud_model");
  });
});
