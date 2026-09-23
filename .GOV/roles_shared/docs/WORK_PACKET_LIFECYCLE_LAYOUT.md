# WORK_PACKET_LIFECYCLE_LAYOUT

**Status:** ACTIVE  
**Policy version:** `2026-04-05`

## Purpose

Define a governed lifecycle layout for Work Packets without forcing risky filesystem churn.

## Physical layout today

- active packets: current physical storage root under `.GOV/task_packets/`
- stubs: `.GOV/task_packets/stubs/`
- reserved archive root: `.GOV/task_packets/_archive/`
  - superseded packets: `.GOV/task_packets/_archive/superseded/`
  - validated closed packets: `.GOV/task_packets/_archive/validated_closed/`

## Resolver rule

The logical resolver name `work_packets` is authoritative (resolver script deleted 2026-09-23).

Resolver order:

1. active Work Packet root
2. archive roots
3. legacy flat compatibility inside those roots

## Migration rule

- Do not move existing packets by hand.
- Create or update packets only in the active storage root.
- Archive roots are reserved so future governed migrations can relocate superseded or validated-closed packets without breaking path resolution.
