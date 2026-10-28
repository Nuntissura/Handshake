---
file_id: cabinet-host-readme
file_kind: product-module-readme
updated_at: 2026-10-28
---

<topic id="cabinet-host">

# Cabinet host adapter

`cabinet_host` owns Handshake's tree interaction projection and asynchronous request custody over the pinned Cabinet core. Native Project tree and Loom folder tree consume it directly. Hierarchy, selection and core request validation remain in Cabinet.

Call `sync` with canonical entries and their scope, generation and source revision. `request`, `prepare_rename` and `load_children_and_expand` create intentions; Native retains authentication, domain clients, dispatch and canonical data. Bind a dispatched request to its actual epoch and sequence. `terminal` rejects a mismatched binding.

A failed `Open`, `LoadChildren` or `LoadPage` terminal locally retires its matching descriptor without a canonical reread. It completes any matching local Cabinet request as an error; it does not change canonical entries or cancel a backend job. Successful read terminals and both success and failure write terminals retain custody until the subsequent authoritative read: call `canonical_refetch` after that read arrives, then `sync` its data and source revision. Read-only `Open` can also be locally retired through `acknowledge_read`. A same-context projection refresh or failed replacement request does not by itself retire a dispatched descriptor. Use `fail_unstarted` or its bound counterpart only for an intention known not to have started.

`expand` preserves Cabinet's lazy `LoadPage` intention for keyboard/disclosure navigation. `projection_expand` restores local disclosure without requesting I/O. `load_children_and_expand` preserves the separate label-click `LoadChildren` operation. Immutable `model()` reads expose presentation; controlled selection, draft and error methods handle mutation. Cloning resets local state and transfers no request ownership. A scope or generation change resets the local projection; backend work remains separately owned by its dispatcher.

The standalone sibling manifest owns this crate's dependency graph. It depends only on pinned Cabinet, egui with AccessKit support, and the standard library. Native consumes the relative path dependency. Canonical authentication, storage, backend dispatch and rendering remain in their existing owners.

</topic>
