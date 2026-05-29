---
schema: handshake.indexed_spec.module@1
spec_version: "v02.187"
bundle_id: "master-spec-v02.187"
module_id: "09"
section_id: "9"
title: "9. Continuous Local Skill Distillation (Skill Bank & Pipeline)"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "89fdf3859d94ecb75e1f1d677dabce95d476aae744cb89198ea7b68f18cf0a4d"
body_sha256: "89fdf3859d94ecb75e1f1d677dabce95d476aae744cb89198ea7b68f18cf0a4d"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 9. Continuous Local Skill Distillation (Skill Bank & Pipeline)

**Why**
- Capture the complete Skill Bank and distillation pipeline (teacher/student) inside the Master Spec without losing any technical detail.
- Ensure alignment with AI Job Model, Workflow Engine, Flight Recorder, and capability/privacy controls.

**How it integrates**
- Data model fields (messages, snapshots, engines, context refs, telemetry, quality, trust, checkpoints, examples) map to Section 3 storage/indexing and provenance rules; no token logs are stored, tokenization is per-engine at train time.
- Distillation jobs (sample/select -> teacher -> student -> score -> checkpoint -> eval/promotion) must run through the Workflow Engine with capability gates; Flight Recorder logs models, tokenizers, params, files, tools, metrics, reward features, lineage, and data_signature/job_ids_json.
- Optional (traceability only): Skill Bank artifacts (examples/checkpoints/eval runs) MAY be linked into the workspace Knowledge Graph as typed edges to EntityRefs and/or MemoryItems. This does not change training inputs (datasets remain artifact-backed and capability-governed).
- Safety/consent: PII/secret redaction at log time + pre-training scrubbing; export controls via capability; rollback via checkpoint lineage.
- Risks/mitigations and gap analysis remain normative and are included verbatim below.

**AI-Ready Data Architecture Integration [ADD v02.115]**

The Skill Bank receives training data from the AI-Ready Data Architecture (Â§2.3.14):

| Data Architecture Component | Skill Bank Usage | Training Impact |
|-----------------------------|------------------|-----------------|
| Bronze content | Original input preservation | Reproducible training data |
| Silver chunks | Processed training examples | Clean, chunked inputs |
| Retrieval quality (MRR, recall) | Training data selection | Weight good retrievals higher |
| User edits | Preference signal | Style vs reasoning distinction |
| Contextual prefixes (Â§2.3.14.2) | System prompt examples | RAG distillation |
| Embedding model version | Reproducibility | Re-training triggers |

**Quality-Weighted Training Data Selection:**

Training data selection MUST weight samples by signals from Â§2.3.14:
1. **User signal:** thumbs up/down, edit ratio (from `QualityMeta`)
2. **Auto-eval:** tests passed, compile success, reasoning score
3. **Retrieval signal:** Was retrieved content used? Did it help?
4. **Data trust score:** Combined 0-1 weight for training (`data_trust_score` field)

**LoRA Training Data Format Requirements:**
1. All training examples MUST include Bronze provenance (`bronze_ref`)
2. All examples MUST have quality scores from `QualityMeta`
3. Style vs reasoning edits MUST be distinguished (`style_only_edit`)
4. Retrieval context MUST be included for RAG distillation (`context_refs`)
5. Embedding model version MUST be tracked for re-training triggers

**Retrieval Quality â†’ Training Selection Flow:**

```
FR-EVT-DATA-009 (retrieval_executed)
         â”‚
         â–¼
   Quality Signals:
   - MRR score
   - Recall@k
   - User feedback
         â”‚
         â–¼
   Training Data Selector
         â”‚
         â–¼
   SkillBankLogEntry.quality.data_trust_score
         â”‚
         â–¼
   LoRA Training Dataset
```

### 9.1 Canonical Specification (verbatim import)

### Handshake Continuous Local Skill Distillation â€“ Technical Specification

This document defines the canonical specification for the Skill Bank, Distillation Pipeline, and associated security/reliability controls used in Handshakeâ€™s continuous local Teacherâ†’Student skill distillation.

---

#### 1. Implementation Guide

##### 1.1 Core data model (Python)

The following Python dataclasses define the in-orchestrator representations that map directly to the Skill Bank storage and distillation pipeline.

###### 1.1.1 Chat and content structures

```python
from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Literal, Optional, Union, Dict
from uuid import UUID
from datetime import datetime


Role = Literal["system", "user", "assistant", "tool", "router"]
ContentSegmentType = Literal["text", "code", "diff", "markdown"]
SnapshotFormat = Literal["chatml", "openai_chat", "raw_text"]

QualityTag = Literal["good", "bad", "needs_edit", "unrated"]
ThumbValue = Literal["up", "down", "neutral", "none"]
ActorRole = Literal["student", "teacher", "tool", "router"]


@dataclass
class ContentSegment:
    type: ContentSegmentType
    text: str
    language: Optional[str] = None
    file_path: Optional[str] = None


Content = Union[str, List[ContentSegment]]


@dataclass
class ChatMessage:
    id: UUID
    parent_id: Optional[UUID]
    role: Role
    content: Content
    metadata: Dict[str, object] = field(default_factory=dict)
    # metadata keys MAY include (non-exhaustive):
    #   "turn_index": int
    #   "tags": List[str]
    #   "code_lang": str
    #   "style_features": Dict[str, float]  # indentation ratio, markdown density, etc.


@dataclass
class ChatSnapshot:
    format: SnapshotFormat
    messages: List[ChatMessage]
    # For training, this is the assistant message treated as the target.
    focus_message_id: Optional[UUID] = None
    # NOTE:
    # - We only store TEXT, never teacher or student token IDs.
    # - Tokenization is always recomputed per engine to avoid cross-tokenizer errors.
```

###### 1.1.2 Skill Bank entry and metadata

```python
@dataclass
class SessionMeta:
    session_id: UUID
    turn_index: int
    task_id: Optional[str] = None
    user_id_hash: Optional[str] = None
    workspace_id: Optional[str] = None


@dataclass
class TaskMeta:
    type: str                        # e.g. "code_generate", "refactor"
    subtype: Optional[str] = None
    language: Optional[str] = None   # "python", "rust", ...
    tags: List[str] = field(default_factory=list)
    request_summary: Optional[str] = None
```

```python
@dataclass
class EngineMeta:
    actor_role: ActorRole            # "student" or "teacher"
    model_name: str                  # exact model id
    model_family: Optional[str] = None
    model_revision: Optional[str] = None
    provider: Optional[str] = None   # "local", "openai", "anthropic", ...

    tokenizer_id: Optional[str] = None          # exact tokenizer / vocab id
    tokenizer_family: Optional[str] = None      # e.g. "llama3", "qwen2"
    context_window_tokens: Optional[int] = None
    precision: Optional[str] = None             # "fp16", "q4_k_m", ...

    inference_params: Dict[str, object] = field(default_factory=dict)
    # Keys: temperature, top_p, top_k, max_tokens,
    #       presence_penalty, frequency_penalty,
    #       stop_sequences, seed
```

```python
@dataclass
class FileSelectionRange:
    start_line: int
    end_line: int


@dataclass
class FileContextRef:
    path: str                        # repo-relative path
    hash: Optional[str] = None       # content hash at time of call
    selection_ranges: List[FileSelectionRange] = field(default_factory=list)


@dataclass
class ToolInvocationRef:
    invocation_id: UUID
    name: str
    type: Optional[str] = None
    status: Literal["success", "error", "timeout", "skipped"]
    latency_ms: Optional[int] = None
    error_type: Optional[str] = None
    truncated_output: Optional[bool] = None


@dataclass
class ContextRefs:
    files: List[FileContextRef] = field(default_factory=list)
    spec_sections: List[str] = field(default_factory=list)     # e.g. "SEC:7.6.2"
    requirements: List[str] = field(default_factory=list)      # e.g. "RID:0041"
    tools_invoked: List[ToolInvocationRef] = field(default_factory=list)
```

```python
@dataclass
class AutoEvalMeta:
    tests_passed: int = 0
    tests_failed: int = 0
    compile_success: Optional[bool] = None
    security_flags: List[str] = field(default_factory=list)    # "sql_injection", ...
    toxicity_scores: Dict[str, float] = field(default_factory=dict)

    # Explicit decoupling of style vs reasoning/factuality.
    style_score: Optional[float] = None       # 0â€“1, higher = better formatting/style
    reasoning_score: Optional[float] = None   # 0â€“1, higher = better correctness/logic
    factuality_score: Optional[float] = None  # 0â€“1 for non-code factual tasks (if used)
```

```python
@dataclass
class UserEditStats:
    output_was_edited: bool = False
    edit_char_fraction: Optional[float] = None   # 0.0â€“1.0
    edit_summary: Optional[str] = None

    # Optional: what changed?
    style_only_edit: Optional[bool] = None       # True if changes are formatting-only


@dataclass
class QualityMeta:
    quality_tag: QualityTag
    thumb: ThumbValue = "none"
    score: Optional[float] = None               # overall score (0â€“1 or -1â€“1)
    source: Optional[str] = None                # "user", "auto_eval", "curator"
    labels: List[str] = field(default_factory=list)
    auto_eval: AutoEvalMeta = field(default_factory=AutoEvalMeta)
    user_edit_stats: UserEditStats = field(default_factory=UserEditStats)

    # Data trust after all filters applied (0â€“1), used as sample weight.
    data_trust_score: Optional[float] = None

    # For echo-chamber / reward-hacking diagnostics.
    reward_features: Dict[str, float] = field(default_factory=dict)
    # Example keys: "output_char_len", "markdown_header_count", ...
```

```python
@dataclass
class TelemetryMeta:
    latency_ms: Optional[int] = None
    prompt_tokens: Optional[int] = None      # per-engine tokenizer_id
    completion_tokens: Optional[int] = None
    total_tokens: Optional[int] = None
    truncation_occurred: Optional[bool] = None
    cache_hit: Optional[bool] = None

    # For length-based reward-hacking detection.
    output_char_len: Optional[int] = None
    output_line_count: Optional[int] = None


@dataclass
class EnvironmentMeta:
    handshake_version: Optional[str] = None
    orchestrator_build: Optional[str] = None
    git_commit: Optional[str] = None
    os: Optional[str] = None
    hardware_profile: Optional[str] = None
    config_profile: Optional[str] = None


@dataclass
class PrivacyMeta:
    contains_secrets: bool = False
    pii_present: bool = False
    can_export_off_device: bool = False
    redaction_applied: bool = False
```

```python
@dataclass
class SkillBankLogEntry:
    version: str
    log_id: UUID
    timestamp: datetime

    session: SessionMeta
    task: TaskMeta
    engine: EngineMeta
    context_refs: ContextRefs

    snapshots_input: ChatSnapshot
    snapshots_output_raw: ChatSnapshot
    snapshots_output_final: Optional[ChatSnapshot]

    quality: QualityMeta
    telemetry: TelemetryMeta
    environment: EnvironmentMeta
    privacy: PrivacyMeta
```

###### 1.1.3 PII/secret redaction API

Redaction must be applied before a `SkillBankLogEntry` is persisted or used for training.

```python
@dataclass
class RedactionResult:
    redacted_entry: SkillBankLogEntry
    secrets_found: bool
    pii_found: bool


def redact_entry(raw_entry: SkillBankLogEntry) -> RedactionResult:
    # Scan all text in snapshots_input / snapshots_output_* for:
    #   - high-entropy tokens
    #   - API key / secret regexes
    #   - .env-style key=value patterns
    #   - PII (email, phone, IBAN, etc.)
    # Replace detected spans with typed placeholders and set privacy flags.
    ...
```

Implementation detail: use a chain of cheap regexes + entropy heuristics, with optional heavier PII detectors gated by configuration.

---

##### 1.2 SQL schema

SQLite is the primary target; Postgres can map types 1:1. JSON-1 is assumed for nested fields.

###### 1.2.1 Core Skill Bank tables

```sql
CREATE TABLE skill_log_entry (
    id                      TEXT PRIMARY KEY,   -- UUID
    version                 TEXT NOT NULL,
    created_at              TEXT NOT NULL,      -- ISO-8601 UTC

    -- Session meta
    session_id              TEXT NOT NULL,
    session_turn_index      INTEGER NOT NULL,
    task_id                 TEXT,
    user_id_hash            TEXT,
    workspace_id            TEXT,

    -- Task meta
    task_type               TEXT NOT NULL,
    task_subtype            TEXT,
    task_language           TEXT,
    task_tags               TEXT,               -- JSON array of strings
    task_request_summary    TEXT,

    -- Engine meta
    actor_role              TEXT NOT NULL,      -- "student" | "teacher" | ...
    model_name              TEXT NOT NULL,
    model_family            TEXT,
    model_revision          TEXT,
    provider                TEXT,
    tokenizer_id            TEXT,
    tokenizer_family        TEXT,
    context_window_tokens   INTEGER,
    precision               TEXT,
    inference_params        TEXT,               -- JSON object

    -- Context refs and snapshots
    context_refs_json           TEXT NOT NULL,  -- JSON object (files, tools, spec refs)
    snapshots_input_json        TEXT NOT NULL,  -- ChatSnapshot JSON
    snapshots_output_raw_json   TEXT NOT NULL,
    snapshots_output_final_json TEXT,           -- nullable

    -- Quality meta
    quality_tag             TEXT NOT NULL,      -- "good" | "bad" | ...
    thumb                   TEXT NOT NULL,      -- "up" | "down" | ...
    quality_score           REAL,
    quality_source          TEXT,
    quality_labels          TEXT,               -- JSON array
    auto_eval_json          TEXT NOT NULL,      -- tests, compile, security flags, scores
    user_edit_stats_json    TEXT NOT NULL,
    data_trust_score        REAL,               -- 0â€“1, nullable

    -- Auto-eval detail (optionally duplicated for indexing)
    auto_style_score        REAL,
    auto_reasoning_score    REAL,
    auto_factuality_score   REAL,

    -- Telemetry
    latency_ms              INTEGER,
    prompt_tokens           INTEGER,
    completion_tokens       INTEGER,
    total_tokens            INTEGER,
    truncation_occurred     INTEGER,            -- 0/1
    cache_hit               INTEGER,            -- 0/1
    output_char_len         INTEGER,
    output_line_count       INTEGER,

    -- Environment
    handshake_version       TEXT,
    orchestrator_build      TEXT,
    git_commit              TEXT,
    os                      TEXT,
    hardware_profile        TEXT,
    config_profile          TEXT,

    -- Privacy
    contains_secrets        INTEGER NOT NULL DEFAULT 0,
    pii_present             INTEGER NOT NULL DEFAULT 0,
    can_export_off_device   INTEGER NOT NULL DEFAULT 0,
    redaction_applied       INTEGER NOT NULL DEFAULT 0,

    -- Reward / diagnostic features
    reward_features_json    TEXT                -- JSON object
);

CREATE INDEX idx_skill_log_entry_session
    ON skill_log_entry (session_id, session_turn_index);

CREATE INDEX idx_skill_log_entry_quality
    ON skill_log_entry (quality_tag, thumb);

CREATE INDEX idx_skill_log_entry_privacy
    ON skill_log_entry (contains_secrets, pii_present, can_export_off_device);

CREATE INDEX idx_skill_log_entry_task_type
    ON skill_log_entry (task_type, task_language);
```

Optional normalized file and tool reference tables:

```sql
CREATE TABLE skill_log_file_ref (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    log_entry_id    TEXT NOT NULL REFERENCES skill_log_entry(id) ON DELETE CASCADE,
    path            TEXT NOT NULL,
    hash            TEXT
);

CREATE INDEX idx_skill_log_file_ref_log
    ON skill_log_file_ref (log_entry_id);
```

###### 1.2.2 Distillation jobs, examples, checkpoints, eval

```sql
CREATE TABLE distill_job (
    id              TEXT PRIMARY KEY,      -- UUID
    created_at      TEXT NOT NULL,
    status          TEXT NOT NULL,        -- "pending" | "running" | "completed" | "failed"
    description     TEXT,
    config_json     TEXT NOT NULL         -- adapter hyperparams, data filters, etc.
);

CREATE TABLE distill_example (
    job_id              TEXT NOT NULL REFERENCES distill_job(id) ON DELETE CASCADE,
    log_entry_id        TEXT NOT NULL REFERENCES skill_log_entry(id) ON DELETE CASCADE,
    role                TEXT NOT NULL,       -- "teacher" | "student"
    is_replay           INTEGER NOT NULL,    -- 0/1
    sample_weight       REAL NOT NULL,       -- typically = data_trust_score
    PRIMARY KEY (job_id, log_entry_id, role)
);

CREATE INDEX idx_distill_example_job
    ON distill_example (job_id, is_replay);
```

Adapter checkpoints and lineage:

```sql
CREATE TABLE adapter_checkpoint (
    id                      TEXT PRIMARY KEY,      -- UUID
    created_at              TEXT NOT NULL,
    parent_checkpoint_id    TEXT REFERENCES adapter_checkpoint(id),

    base_model_name         TEXT NOT NULL,
    adapter_type            TEXT NOT NULL,        -- "lora" | "dora"
    rank_r                  INTEGER NOT NULL,
    alpha                   INTEGER NOT NULL,
    learning_rate           REAL NOT NULL,
    precision               TEXT NOT NULL,        -- e.g. "4bit-nf4"

    path                    TEXT NOT NULL,        -- filesystem path to adapter weights
    ewc_state_json          TEXT,                 -- Fisher diag, lambda, etc.
    eval_summary_json       TEXT,                 -- metrics on fixed eval suite

    -- Lineage / provenance
    data_signature          TEXT,                 -- hash of training data spec
    job_ids_json            TEXT,                 -- JSON array of distill_job ids

    is_approved             INTEGER NOT NULL DEFAULT 0,  -- passed gates
    is_current              INTEGER NOT NULL DEFAULT 0   -- currently served student
);

CREATE INDEX idx_adapter_checkpoint_current
    ON adapter_checkpoint (is_current);
```

Evaluation runs:

```sql
CREATE TABLE eval_run (
    id                  TEXT PRIMARY KEY,
    job_id              TEXT REFERENCES distill_job(id),
    checkpoint_id       TEXT REFERENCES adapter_checkpoint(id),
    created_at          TEXT NOT NULL,
    suite_name          TEXT NOT NULL,          -- e.g. "core_code_eval_v1"
    metrics_json        TEXT NOT NULL           -- pass@k, compile rate, collapse indicators
);
```

Replay candidates as a view:

```sql
CREATE VIEW replay_candidates AS
SELECT *
FROM skill_log_entry
WHERE quality_tag = 'good'
  AND contains_secrets = 0
  AND pii_present = 0;
```

---

##### 1.3 Distillation control loop

Each distillation cycle consists of:

1. Candidate selection.
2. Filtering and trust scoring.
3. Dataset assembly (new vs replay).
4. Adapter training (QLoRA/DoRA).
5. Evaluation.
6. Promotion or rollback.

###### 1.3.1 Data Trust Score

```python
def compute_data_trust_score(entry: SkillBankLogEntry) -> float:
    # Returns a score in [0, 1] indicating suitability for training.
    # Hard excludes (score = 0.0) if:
    #   - contains_secrets or pii_present
    #   - quality_tag == "bad"
    #   - auto_eval.compile_success is False
    #   - tests_failed > 0 where tests exist
    # Soft scoring:
    #   - +0.4 if quality_tag == "good"
    #   - +0.2 if thumb == "up"
    #   - +0.2 * test_pass_ratio
    #   - +0.1 if no security_flags
    #   - +0.2 * (reasoning_score - 0.5)  if present
    #   - +0.2 * (factuality_score - 0.5) if present
    #   - +0.05 * (style_score - 0.5)     if present
    #   - -0.1 if output is short (<128 chars) but thumb == "up"
    # The final value is clamped to [0.0, 1.0].
    ae = entry.quality.auto_eval
    q = entry.quality

    if entry.privacy.contains_secrets or entry.privacy.pii_present:
        return 0.0
    if q.quality_tag == "bad":
        return 0.0
    if ae.compile_success is False:
        return 0.0
    if ae.tests_failed > 0 and (ae.tests_passed + ae.tests_failed) > 0:
        return 0.0

    score = 0.0

    if q.quality_tag == "good":
        score += 0.4
    if q.thumb == "up":
        score += 0.2

    if ae.tests_passed + ae.tests_failed > 0:
        test_ratio = ae.tests_passed / (ae.tests_passed + ae.tests_failed)
        score += 0.2 * test_ratio

    if not ae.security_flags:
        score += 0.1

    if ae.reasoning_score is not None:
        score += 0.2 * (ae.reasoning_score - 0.5)

    if ae.factuality_score is not None:
        score += 0.2 * (ae.factuality_score - 0.5)

    if ae.style_score is not None:
        score += 0.05 * (ae.style_score - 0.5)

    length = entry.telemetry.output_char_len or 0
    if length < 128 and q.thumb == "up":
        score -= 0.1

    return max(0.0, min(1.0, score))
```

Key properties:

- Correctness (tests, compile, reasoning/factuality) dominates.
- Style has bounded influence.
- Very short but highly upvoted outputs are penalized to counter echo-chamber length bias.

###### 1.3.2 Dataset assembly (new vs replay)

```python
def build_distill_dataset(job_id: UUID, target_role: ActorRole) -> None:
    # 1) SELECT candidate entries from skill_log_entry:
    #       - quality_tag IN ('good', 'needs_edit')
    #       - contains_secrets = 0 AND pii_present = 0
    #       - task_type in code-related categories
    # 2) COMPUTE data_trust_score if NULL and update DB.
    # 3) SPLIT into:
    #       - new_batch: entries from a recent time window (e.g. last N days).
    #       - replay_batch: random sample from replay_candidates view.
    # 4) WRITE rows into distill_example with is_replay and sample_weight.
    ...
```

The new vs replay mixture ratio (e.g. 70/30) is encoded in `distill_job.config_json`.

###### 1.3.3 Adapter training configuration

```python
ADAPTER_CONFIG_CONTINUOUS = {
    "adapter_type": "dora",      # or "lora" where DoRA is unavailable
    "rank_r": 32,
    "alpha": 64,
    "modules": ["q_proj", "k_proj", "v_proj", "o_proj", "up_proj", "down_proj"],
    "dropout": 0.05,
    "learning_rate": 5e-5,
    "weight_decay": 0.01,
    "scheduler": "cosine",
    "warmup_ratio": 0.1,
    "precision": "4bit-nf4"
}
```

```python
def run_distill_job(job_id: UUID, base_checkpoint_id: UUID) -> str:
    # - Load base_checkpoint (adapter or base model).
    # - Materialize dataset from distill_example for this job.
    # - Extract (prompt, completion) pairs from snapshots_input and snapshots_output_final.
    # - For each engine, tokenize text with its own tokenizer_id.
    # - Train adapters using ADAPTER_CONFIG_CONTINUOUS, optionally with EWC.
    # - Save new adapter_checkpoint with eval_summary_json.
    # - Return new checkpoint id.
    ...
```

Quantization and adapter assumptions:

- Base Student is a 7B code-tuned model in 4-bit (QLoRA-style NF4/FP4) with frozen backbone.
- DoRA adapters are preferred where supported; LoRA is the fallback.
- Only adapter parameters are updated during distillation.
- [ADD v02.157] Phase 1 keeps adapter training late-stage and stub-backed. When later phases activate local adapter training, rank/alpha/repeats/epochs MUST be explicit tracked hyperparameters, adapter merges MUST be benchmark-gated, and promotion MUST remain rollback-safe.
- [ADD v02.157] Current research posture from vendor docs, papers, GitHub/Hugging Face training stacks, and community LoRA guidance is that data quality, caption/annotation fidelity, and benchmark-gated adapter promotion matter more than brute dataset volume; Handshake MUST preserve those signals as first-class lineage/eval fields instead of hidden trainer defaults.

###### 1.3.4 Cross-tokenizer-safe distillation

Constraints:

- Skill Bank stores only text snapshots plus tokenizer metadata; no shared token ID space.
- For log-prob-based distillation:
  - Teacher and Student tokenize the same text independently with their own tokenizers.
  - Distillation loss uses methods compatible with differing tokenizers or avoids logprob terms entirely when this is not implemented.
- For pure SFT:
  - Teacher text is used as target; tokenization remains per-engine with no cross-tokenizer coupling.

The implementation must never pass Student-tokenized sequences into the Teacher or assume shared vocabularies between Teacher and Student.

###### 1.3.5 Style vs reasoning aware loss

For Student updates, the overall loss is conceptually:

- `L_total = L_token + Î»_style * L_style + Î»_reg * L_reg`

Where:

- `L_token`: standard cross-entropy on assistant target tokens.
- `L_style`: optional loss term enforcing minimal formatting invariants (e.g. fenced code blocks) but never dominating correctness.
- `L_reg`: regularization comprising Elastic Weight Consolidation and/or KL constraints to previous Student/Teacher checkpoints.

Guidelines:

- For code tasks:
  - `L_token` is measured primarily on segments known to pass tests in Teacher output.
  - `Î»_style` is small (e.g. 0.05) so code correctness dominates.
- For explanation tasks:
  - Where factuality models are available, they inform which samples are eligible and their trust scores; style loss is still strictly auxiliary.

---

##### 1.4 Evaluation and promotion

```python
def evaluate_and_maybe_promote(
    candidate_ckpt: AdapterCheckpoint,
    previous_ckpt: AdapterCheckpoint,
    teacher_ckpt: AdapterCheckpoint,
    eval_suite: EvalSuiteConfig
) -> bool:
    # 1) Run eval_run for candidate_ckpt on eval_suite (fixed set of code tasks).
    # 2) Compute metrics:
    #      - pass@1 / pass@k
    #      - compile_success_rate
    #      - test_pass_rate
    #      - collapse indicators (repetition, entropy, syntax errors)
    # 3) Apply thresholds:
    #      - candidate >= previous_ckpt - epsilon on core metrics
    #      - candidate >= teacher_ckpt - delta where teacher is reference
    #      - no unacceptable increase in security flags or collapse indicators
    # 4) If thresholds satisfied, approve and promote candidate_ckpt.
    ...
```

Checkpoints form a lineage via `parent_checkpoint_id`. Manual rollback is always possible by promoting an older approved checkpoint back to `is_current = 1`.

---

#### 2. Risk Mitigation Strategy

This section covers model collapse, PII leakage, data poisoning, drift, and the specific â€œsilent killersâ€ observed in enterprise distillation pipelines.

##### 2.1 Model collapse and synthetic data

Risk:

- Repeatedly training on model-generated data (especially self-generated Student outputs) leads to model collapse:
  - Loss of tail behaviors.
  - Overly bland, repetitive outputs.
  - Forgetting of the original data distribution.

Mitigations:

1. Data source mixture constraints.  
2. Eligibility filters for Student outputs (quality, tests, compilation, security flags).  
3. Periodic Teacher re-anchoring after a bounded number of generations.  
4. Collapse detection via eval metrics and degeneracy indicators.  
5. Data lineage for root-cause analysis.
6. Bounded self-distillation ratios and replay-buffer diversity so Student-generated traces never dominate trusted human/teacher/eval-backed data.

##### 2.2 PII leakage and membership inference

Risk:

- Secrets or PII logged into Skill Bank, if used for training, may be memorized and later extracted.

Mitigations:

1. Log-time redaction before persistence and training.  
2. Pre-training scrubbing with stronger detection.  
3. Structural exclusions for secret-bearing paths.  
4. Export controls via `can_export_off_device`.  
5. Regularized training without secrets/PII in the training set.

Residual non-secret privacy risk is tracked separately and may require DP training in future versions.

##### 2.3 Data poisoning and drift

Risk:

- Malicious or mistaken thumbs; small datasets; high sensitivity to poisoned examples.

Mitigations:

1. Multi-signal labels: thumbs + objective metrics + trust score.  
2. Example-level outlier detection, marking conflicting signals as suspected poison.  
3. User-level correlation checks to down-weight misaligned raters.  
4. Robust optimization via per-example loss and gradient clipping.  
5. Always treating local training data as high-sensitivity.

##### 2.4 Drift and regression

Risk:

- Repeated updates cause regressions or silent capability drift.

Mitigations:

1. Fixed, versioned eval suite.  
2. Promotion gates against Teacher and previous Student checkpoints.  
3. Full checkpoint lineage and rollback.  
4. Environment metadata to distinguish model vs runtime regressions.

##### 2.5 Silent Killers

###### 2.5.1 Format vs Fact drift

Risk: Student imitates Teacherâ€™s formatting while degrading in reasoning, factuality, or code correctness.

Mitigations:

- Separate style vs reasoning/factuality in metadata.  
- Make correctness dominate trust score and gating.  
- Treat style loss as auxiliary only.  
- Use compiler/tests as primary ground truth for code.  
- Integrate factuality/QA models where applicable for non-code.

###### 2.5.2 Tokenizer mismatches

Risk: Teacher and Student tokenizers differ; naÃ¯ve distillation corrupts training signals.

Mitigations:

- Text-only logging with explicit tokenizer metadata.  
- Per-engine tokenization at training time.  
- Cross-tokenizer-safe methods for logprob distillation, or SFT-only when needed.  
- Strict prohibition on passing Student-tokenized sequences to the Teacher.

###### 2.5.3 Echo chamber / reward hacking

Risk: Model optimizes a proxy (e.g. short answers) instead of correctness.

Mitigations:

- Log reward features (length, style, density).  
- Analyze correlations between thumbs and proxies.  
- Down-weight thumbs when they conflict with objective metrics.  
- Use regularization/constraints if RLHF phases are added.  
- Cap the influence of thumbs and style-derived signals on training.

###### 2.5.4 Versioning and lineage loss

Risk: Adapter checkpoints without clear provenance (â€œlora_v15â€ with no dataset history).

Mitigations:

- `data_signature` and `job_ids_json` on adapter checkpoints.  
- Reconstructable datasets from Skill Bank and distill_example rows.  
- Semantic versioning only after eval and lineage recording.  
- Optional mirroring into external lineage tools.

---

#### 3. Industry Reference

Key research and practice areas informing this spec:

1. QLoRA for efficient finetuning of quantized LLMs.  
2. DoRA for improved low-rank adaptation stability.  
3. Model collapse on synthetic data (curse of recursion).  
4. Training data extraction and membership inference attacks.  
5. RLHF and preference-data poisoning.  
6. Low-volume poisoning sensitivity.  
7. Cross-tokenizer distillation methods.  
8. Reward hacking and over-optimization.  
9. Factuality vs style tuning.  
10. Data and model lineage tooling (MLflow, DVC, etc.).

The normative requirements are encoded in the schema and pipeline definitions; these references justify the design choices but are not required at runtime.

---

#### 4. Gap Analysis

Known gaps and planned work:

1. **PII/secret detection quality**  
   - Need benchmarks, metrics, and detector versioning to support re-scrubbing.

2. **Formal privacy guarantees**  
   - No DP training yet; future work to evaluate DP-compatible adapter training.

3. **Data Trust Score calibration**  
   - Requires empirical calibration against real distillation runs and eval outcomes.

4. **Replay buffer and EWC configuration**  
   - Requires ablation studies to tune replay ratios and EWC Î», plus diversity monitoring.

5. **Evaluation suite completeness**  
   - Needs concrete task set, language coverage, and versioning process.

6. **DoRA tooling maturity**  
   - Requires benchmarking against LoRA on target hardware and models; define fallback behavior.

7. **Format vs Fact residual risk**  
   - Integrate factuality models and quantify safe ranges for style loss weighting.

8. **Cross-tokenizer edge cases**  
   - Standardize cross-tokenizer distillation methods and mandate SFT-only when necessary.

9. **Reward hacking diagnostics**  
   - Add richer reward-feature logging and more robust detection of proxy exploitation.

10. **External versioning integration**  
    - Provide reference integrations and schema migration strategy for the Skill Bank.

---

This document is canonical for the Skill Bank and Distillation Pipeline design and is ready to be merged into the Handshake Master Spec and roadmap.

---

<a id="10-product-surfaces"></a>
