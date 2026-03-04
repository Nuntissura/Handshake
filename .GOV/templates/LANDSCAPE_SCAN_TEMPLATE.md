## LANDSCAPE_SCAN (prior art / better approaches)

Use this snippet inside the Technical Refinement Block.

Requirements:
- Timebox the scan.
- Include references (vendor docs, papers, OSS repos, adjacent products).
- Extract patterns/constraints you want to steal (not code).
- Record ADOPT/ADAPT/REJECT decisions with rationale.
- Include a LICENSE/IP note for any code-level reuse.
- If this changes primitives/techniques/UI surface: mark SPEC_IMPACT=YES and BLOCK delegation until the Master Spec is enriched.

Template:
```md
### LANDSCAPE_SCAN
- TIMEBOX: <30m|2h|4h>
- SEARCH_SCOPE: <what you searched for; key terms>
- REFERENCES:
  - <ref 1>
  - <ref 2>
  - <ref 3>
- PATTERNS_EXTRACTED:
  - <pattern/invariant/interface worth stealing>
- DECISIONS (ADOPT/ADAPT/REJECT):
  - <decision + rationale>
- LICENSE/IP_NOTES: <constraints for reuse; or NONE>
- SPEC_IMPACT: <YES|NO>
- SPEC_IMPACT_REASON: <if YES, what must change in Main Body and/or EOF Appendix blocks>
```

