---
file_id: diagnostics-motion-js-classify
file_kind: diagnostics
lane: SMB-MOTION-JS
crate: hsk-studio-motion-js
input: validation/R5-review-b.json (SMB-MOTION-JS CONFIRMED r3_remediation_incomplete); R5 verdict FAIL_V2 + NEEDS_NEW_APPROACH
engine: rquickjs 0.14.0 / rquickjs-core 0.14.0 / rquickjs-sys 0.14.0 (bundled QuickJS-NG 0.16.2), local cargo registry source
updated_at: 2026-10-11T05:44:34Z
---

<topic id="root-cause" status="current" summary="cause, evidence">

## Root cause (<=120 chars)

Budget(Stack|Memory) came from message text a script can write; QuickJS limit errors are ordinary catchable objects.

## Evidence

- Classification at candidate 1d404f8: `quickjs.rs:167-171` (BOOTSTRAP `describe` sets flags from `indexOf(message, "Maximum call stack")` / `"out of memory"`), `quickjs.rs:680-682` (`classify`: RangeError + flag -> `Budget(Stack)`, InternalError + flag -> `Budget(Memory)`); `provider.rs:184-186` (`deny` counts the denial), `provider.rs:231` (`Budget(kind) => self.deny(kind)`).
- Engine side, QuickJS-NG 0.16.2 (`rquickjs-sys-0.14.0/quickjs/quickjs.c`):
  - stack overflow = `JS_ThrowStackOverflow` -> `JS_ThrowRangeError(ctx, "Maximum call stack size exceeded")` (:8565-8568), triggered by `js_check_stack_overflow` (:2307-2312, C stack pointer vs `rt->stack_limit`);
  - heap limit = `JS_ThrowOutOfMemory` -> `JS_ThrowInternalError(ctx, "out of memory")` (:8554-8562);
  - both build the object with `JS_MakeError2` (:8402-8415: `native_error_proto[n]`, class `JS_CLASS_ERROR`, own `message`), the same shape `js_error_constructor` (:42721-42806) gives `new RangeError(m)`; both get a backtrace (:20895-20899) and both are catchable;
  - the ONLY uncatchable engine error is the interrupt: `JS_ThrowInterrupted` (:8642-8646) marks it with `JS_SetUncatchableError`.
- Signals rquickjs 0.14.0 exposes through its safe API (`rquickjs-core-0.14.0/src`):
  - `Runtime::set_interrupt_handler` (`runtime/base.rs:120-128`): our closure; the engine polls it every `JS_INTERRUPT_COUNTER_INIT` = 10000 steps (`quickjs.c:556`, `:8648-8659`), never at a stack or heap failure;
  - `Error` kinds (`result.rs:60-66`): `Allocation` is raised only by wrapper-side allocation failures (`runtime/raw.rs:186,220`, `context/base.rs:74,90`); a script-level OOM arrives as `Error::Exception` with the thrown value (`context/ctx.rs:245-271`);
  - `Runtime::memory_usage` (`runtime/base.rs:199-202`) = `JSMemoryUsage` (`quickjs.h:590-604`): current sizes and counts only, no failure or peak counter;
  - `Value::is_uncatchable_error` (`value.rs:392-396`): true only for the interrupt error above;
  - `quickjs.h` has no stack-overflow or memory-limit callback (only `JS_SetMemoryLimit` :512, `JS_SetMaxStackSize` :518, `JS_SetInterruptHandler` :1180);
  - a custom allocator (`Runtime::new_with_alloc`, `runtime/base.rs:71`, cargo feature `allocator`) would give a host-side OOM flag, but EXPR-PROVIDER-03 keeps `allocator`/`rust-alloc` absent pending a separate accounting audit (`backend-redteam/expression-provider-adoption.json` required_controls[2], build_feature_contract).
- So no engine-side state distinguishes a genuine stack or heap limit hit from `throw new RangeError("Maximum call stack size exceeded")` or `throw new InternalError("out of memory")`.

</topic>

<topic id="why-r3-fix-missed" status="current" summary="why a53c52d kept the class">

## Why the R3 fix (a53c52d) did not remove the class

- R3 code (404ff3a `quickjs.rs:410-416`) matched `.name` + message; a53c52d replaced the user-writable `.name` with captured intrinsic prototype identity (correct for the kind) and moved the poison decision to the rquickjs error type, but kept the message match for the two budget classes.
- a53c52d's lane notes record this as "RESIDUAL BY DESIGN: message-derived classes stay forgeable" (`builders/lanes/hsk-studio-motion-js.json` finding `r3-internalerror-poison-fixed`): the forgery was judged harmless (a typed denial for that script only). R3's remediation listed "forged RangeError -> Budget" explicitly, so the residual left the R3 class open.
- Its regression tests (`tests/quickjs_contract.rs:338-357`) forged names and the InternalError poison only; no test threw a budget-shaped message.

</topic>

<topic id="new-approach" status="current" summary="classification by construction">

## New approach (forgery impossible by construction)

- Nothing read from the realm decides a budget class. `describe` returns only `[kind, stack]` (kind = captured prototype identity, used for Parse / ReferenceMissing as before); the message flags are deleted, including the `motionjs-unavailable` marker flag (same class: message text deciding a class; provider-visible result unchanged, Unsupported with span).
- Budget classes come only from host-side state user code cannot set:
  - `Fuel`, `Deadline`, `Canceled`: our interrupt-handler state (`Tripped`), unchanged;
  - `Memory`: `rquickjs::Error::Allocation` (wrapper-side allocation failure, not a thrown value), unchanged;
  - `Stack`: no engine signal exists (see root-cause), so the QuickJS backend never reports `Budget(Stack)`.
- Consequence (recorded, not hidden): a genuine catchable stack overflow or heap-limit error is now `Thrown { Other }` -> `ExprErrorCode::Unsupported` with span, like any thrown error; the limit still stops the script (no crash, no poison). DESCRIPTOR lists "typed Budget(Stack/Memory) for catchable QuickJS limit errors" as NOT_PROVEN, with the two routes that could provide a signal later: the allocator feature after the EXPR-PROVIDER-03 audit (host-side OOM flag), or an engine patch that makes limit errors uncatchable like the interrupt.
- Rejected: branding user-constructed errors in the bootstrap (a caught engine RangeError with another message can be re-messaged; `Object.setPrototypeOf` re-kinds any engine error); prepareStackTrace failure at depth (forgeable by recursing to the limit and constructing there); memory_usage heuristics (no peak counter; a failed huge request leaves usage low); re-running with other limits (forged and genuine infinite cases behave the same).

</topic>

<topic id="proof" status="current" summary="cheapest proof">

## Cheapest proof

- `tests/quickjs_contract.rs` forged-message tests, stack and memory: `throw new RangeError('Maximum call stack size exceeded')`, `throw new InternalError('out of memory')`, re-messaged caught errors and `Object.create` variants -> `Unsupported`, `denied(Stack) == denied(Memory) == 0`, provider not poisoned. RED on the unfixed code (Budget), GREEN after.
- Existing genuine-limit cases (`'x'.repeat(100000000)` at 4 MiB, runaway recursion on a 128 KiB caller thread) stay typed rejections without crash or poison; their expected code changes from Budget to Unsupported (documented consequence).
- Same assertions run in-process (`test-faults`) and against the worker (`quickjs` only), so both backends classify identically.

</topic>
