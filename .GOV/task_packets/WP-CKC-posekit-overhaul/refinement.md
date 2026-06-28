# WP-CKC-posekit-overhaul Refinement

> Projection only. Authority lives in `.GOV/task_packets/WP-CKC-posekit-overhaul/refinement.json`.

## Operator Request

Create a hotfix copy of `wtc-native-editors-v1` for CKC/PoseKit overhaul under Atelier, create `WP-CKC-posekit-overhaul`, update taskboard/build-order/traceability, and use one microtask per future requested change before implementation.

## Intent

CKC and PoseKit should be native Handshake features, written and wired in the Rust/native architecture where possible, with outputs available to the rest of Handshake as force multipliers.

## Initial Order

1. Inventory and address non-Rust CKC/PoseKit/Atelier surfaces.
2. Compare against the original proven test app.
3. Perform visual and behavioral overhaul inside Handshake.

## Controls

- Work only in `../wtc-ckc-posekit-overhaul` for product edits.
- Do not disturb `../wtc-native-editors-v1`.
- Create one `MT-###.json` before each product-code edit.
- Keep `mt_index.json` and WP communications current enough for state recovery.
