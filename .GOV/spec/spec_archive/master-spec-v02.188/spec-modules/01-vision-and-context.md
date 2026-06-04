---
schema: handshake.indexed_spec.module@1
spec_version: "v02.188"
bundle_id: "master-spec-v02.188"
module_id: "01"
section_id: "1"
title: "1. Vision & Context"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "38bba865c30bd356be48dee97b199a31aeabd631700ac5ea0c3b2c9665a62878"
body_sha256: "38bba865c30bd356be48dee97b199a31aeabd631700ac5ea0c3b2c9665a62878"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 1. Vision & Context

## 1.1 Executive Summary

**Why**  
Provides high-level orientation for readers new to the specification. Establishes context before technical details.

**What**  
Quick-start overview of Project Handshake: what it is, who it's for, and how this document evolved from both infrastructure research AND three months of AI governance R&D (the Prompt Diaries project).

**Jargon**  
- **Local-first**: Data lives on your device; cloud is optional backup/sync.
- **AI-native**: AI integrated from inception, not bolted on.
- **Workspace**: Unified environment combining docs, canvases, tables.
- **Diary**: The Prompt Diaries project â€” 3 months of governance R&D that Handshake now implements.
- **RID**: Rule Identifier â€” a numbered, machine-checkable governance clause.

---

#### 1.1.1 TL;DR Box

> **Project Handshake** is a desktop application combining:
> - **Notion-like** document editing with databases
> - **Milanote-like** visual canvas/moodboards  
> - **Excel-like** spreadsheets with formulas
> - **Local AI models** for writing, coding, and image generation
> - **Descriptor extraction** for tracking taste and building searchable creative references
> 
> **Tech Stack Decision:** Tauri + React + TypeScript (frontend) + Python (AI backend)
> 
> **Key Insight:** Run AI models locally for privacy, speed, and cost savingsâ€”with cloud fallback when needed.
>
> **Governance Origin:** The Prompt Diaries project spent 3 months building ~1,232 governance clauses to make LLMs reliable. Handshake implements this governance in code.

---

#### 1.1.2 What We're Building

**Project Handshake is a "local AI cloud" on your desktop.** Instead of sending your documents, ideas, and data to cloud services like Notion or Google Docs, everything stays on your computer. AI assistants run locally too, meaning your sensitive information never leaves your machine.

The application combines three types of tools that creative professionals typically use separately:

| Tool Type | Inspiration | Use Case |
|-----------|-------------|----------|
| **Rich Documents** | Notion | Writing, planning, structured databases |
| **Visual Canvas** | Milanote | Mood boards, brainstorming, spatial organization |
| **Spreadsheets** | Excel | Data manipulation, calculations, analysis |

**What makes this different:** Local AI models collaborate to help you. One AI might plan your project, another writes the code, and a third generates imagesâ€”all coordinated automatically.

**The hidden layer:** A comprehensive governance system (ported from the Diary) ensures AI behavior is reliable, deterministic, and auditable.

---

#### 1.1.3 Key Architecture Decisions (From Research)

Based on extensive research across multiple documents, the following decisions have been validated:

| Decision | Choice | Why |
|----------|--------|-----|
| Desktop Shell | **Tauri** (not Electron) | 90% less memory usage; critical when running AI models |
| Frontend | **React + TypeScript** | Rich ecosystem, same code works in both shells |
| Backend | **Python** | Best AI/ML library support, orchestration frameworks |
| AI Orchestration | **AutoGen or LangGraph** | Mature multi-agent coordination |
| Data Sync | **CRDTs (Yjs)** | Offline-first, conflict-free collaboration |
| Storage | **File-tree based** | Human-readable, portable, git-friendly |
| Governance | **Code-enforced (from Diary)** | LLMs can't violate what code prevents |

---

#### 1.1.4 Why Local-First Matters

ðŸ“Œ **Key Point:** The entire architecture is designed around "local-first" principles:

1. **Privacy:** Your documents and AI conversations never leave your computer
2. **Speed:** No network latency for AI responses
3. **Cost:** After initial model download, AI usage is essentially free
4. **Reliability:** Works without internet, on airplanes, in poor connectivity
5. **Control:** You own your data in standard file formats

---

#### 1.1.5 Hardware Context

The target hardware for development and initial deployment:

| Component | Specification | Why It Matters |
|-----------|--------------|----------------|
| CPU | Ryzen 9 5950X (16 cores) | Handles multiple processes, CPU inference fallback |
| RAM | 128 GB | Multiple AI models can stay loaded in memory |
| GPU | RTX 3090 (24GB VRAM) | Runs large AI models, image generation |
| Storage | NVMe SSD | Fast model loading, responsive file operations |

âš ï¸ **Warning:** This hardware is above average. The app design must handle graceful degradation for users with less powerful systems, including cloud fallback options.

---

**Key Takeaways**
- Handshake is a local-first, AI-native desktop workspace
- Combines Notion-style docs, Miro-style canvases, and Excel-style tables
- Designed for power users with high-end hardware (RTX 3090, 128GB RAM)
- **Includes governance layer from Diary** â€” 3 months of R&D on making AI reliable
- This specification covers product vision, governance implementation, and mechanical integrations

---

## 1.2 The Diary Origin Story

**Why**  
Understanding where Handshake's governance comes from explains why it's built the way it is. The Diary was 3 months of R&D that discovered what it actually takes to make AI reliable.

**What**  
This section explains the creative goal that started everything, the problems LLMs caused, the governance solution that emerged, and how Handshake transforms that into code.

**Jargon**  
- **Diary / Prompt Diaries**: The R&D project that preceded Handshake. A plain-text governance system.
- **RID**: Rule Identifier â€” a numbered governance rule (e.g., DES-001, COR-701).
- **Clause**: A single, machine-checkable requirement within a RID.
- **Descriptor**: A structured record describing an image or creative reference.
- **CORPUS**: The accumulated collection of descriptors.
- **CONFIG**: Vocabulary and profile definitions that govern extraction.

---

### 1.2.1 The Goal

The Prompt Diaries project started with a creative goal:
- **Track taste** â€” build a personal aesthetic vocabulary
- **Describe images** â€” extract structured descriptors from visual content  
- **Build a corpus** â€” accumulate tagged, searchable creative references

This is what DES-001 (Descriptor Extraction), IMG-001 (Image Analysis), and SYM-001 (Symbolic Layers) are for. **These three RIDs are the actual product.**

---

### 1.2.2 The Problem

LLMs couldn't reliably do this work because:
- They **drift** â€” forget rules mid-conversation
- They **can't edit reliably** â€” surgical changes corrupt surrounding content
- They **don't know where they are** â€” lose track of document position
- They **guess** â€” fabricate content when uncertain instead of stopping

Every attempt to extract descriptors resulted in:
- Schema violations
- Content in wrong locations
- Silent modifications to existing data
- Inconsistent output formats

---

### 1.2.3 The Solution (That Became Its Own Project)

To make LLMs reliable, the project built a comprehensive governance system:
- **RIDs** â€” Rules with machine-checkable clauses
- **Layers** â€” L1 (immutable), L2 (promotion-only), L3 (writable)
- **Gates** â€” Validation checkpoints before any operation
- **Modes** â€” Explicit work contexts with different permissions
- **Lint rules** â€” Automated compliance checking
- **Answer governance** â€” Structured output formats

This governance layer grew to **~1,232 clauses across 14 RIDs** plus Bootloader and Execution Charter.

The governance infrastructure became so comprehensive that it overshadowed the creative extraction core it was built to enable. The Diary became known for its rules, not its purpose.

---

### 1.2.4 What Handshake Changes

Handshake moves enforcement from **rules in context** (unreliable) to **code enforcement** (reliable):

```
DIARY (Before):
  Rules live in text â†’ LLM reads them â†’ LLM may or may not follow them
  
HANDSHAKE (After):
  Rules become code â†’ Code enforces them â†’ LLM literally cannot violate them
```

**The Diary was R&D. Handshake is the product.**

---

**Key Takeaways**
- The real product is **descriptor extraction** (DES-001, IMG-001, SYM-001)
- Governance exists because LLMs couldn't do extraction reliably
- ~1,232 clauses were needed to make LLMs behave
- Handshake implements these clauses in code, not text
- Code enforcement is at the top of the reliability hierarchy; rules-in-context is near the bottom

---

## 1.3 The Four-Layer Architecture

**Why**  
Understanding the layers helps you know where each piece of functionality lives. When something goes wrong, you know which layer to debug.

**What**  
Handshake has four layers: LLM (decides what), Orchestrator (enforces rules), Mechanical (executes deterministically), and Validation (confirms correctness).

**Jargon**  
- **LLM Layer**: The AI model that reasons about what to do.
- **Orchestrator Layer**: The code that translates AI intent into safe operations.
- **Mechanical Layer**: Deterministic engines (Word, Excel, Docling) that execute operations.
- **Validation Layer**: Checks that confirm output matches expectations.

---

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  LLM LAYER                                                      â”‚
â”‚  Decides WHAT to change                                         â”‚
â”‚  Outputs: structured instruction (not raw text)                 â”‚
â”‚                         â”‚                                       â”‚
â”‚                         â–¼                                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  ORCHESTRATOR LAYER                                             â”‚
â”‚  Translates instruction â†’ API calls                             â”‚
â”‚  Enforces capability constraints                                â”‚
â”‚  Loads relevant rules (not all 1,232)                           â”‚
â”‚                         â”‚                                       â”‚
â”‚                         â–¼                                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  MECHANICAL LAYER                                               â”‚
â”‚  Descriptor extraction / Document editing engine                â”‚
â”‚  Executes deterministically                                     â”‚
â”‚  LLM never touches data directly                                â”‚
â”‚                         â”‚                                       â”‚
â”‚                         â–¼                                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  VALIDATION LAYER                                               â”‚
â”‚  SHA: did input become expected output?                         â”‚
â”‚  Lint: did instruction make sense?                              â”‚
â”‚  Diff: is change within allowed scope?                          â”‚
â”‚  Failure is visible and recoverable                             â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**Key principle:** LLM steers, software executes, code validates.

---

### 1.3.1 How the Layers Work Together

1. **User request** arrives (e.g., "extract descriptors from this image")
2. **LLM Layer** reasons about the task, outputs structured instruction
3. **Orchestrator Layer** checks: Is this operation permitted? Which RIDs govern it?
4. **Mechanical Layer** executes deterministically (IMG-001 pipeline extracts descriptors)
5. **Validation Layer** confirms: Schema valid? Gates passed? SHA matches?
6. **Result** returns to user (or error with recovery path)

The LLM **never directly touches** document content or descriptor data. It only emits instructions that the mechanical layer executes.

---

**Key Takeaways**
- Four layers: LLM â†’ Orchestrator â†’ Mechanical â†’ Validation
- LLM decides WHAT, never HOW
- Mechanical layer is deterministic (same input â†’ same output)
- Validation catches failures before they reach the user
- This architecture makes AI behavior auditable and recoverable

---

## 1.4 LLM Reliability Hierarchy

**Why**  
This hierarchy explains why some AI behaviors are trustworthy and others aren't. It guides every design decision: push enforcement UP the hierarchy.

**What**  
A ranking from most reliable (code enforcement) to least reliable (hoping the model remembers). Handshake operates at the top; the Diary operated near the bottom.

**Jargon**  
- **Code enforcement**: Rules the LLM literally cannot violate (compile-time, type system).
- **Structured output**: JSON schema, grammar constraints that force valid format.
- **Verbatim markers**: Explicit tags that mechanical code processes (not the LLM).
- **Rules in context**: Instructions the LLM can read but may ignore.

---

```
MOST RELIABLE
     â”‚
     â”‚  1. Code enforcement (literally cannot violate)
     â”‚        â†’ Rust type system, compile-time checks
     â”‚        â†’ HANDSHAKE OPERATES HERE
     â”‚
     â”‚  2. Verbatim markers + mechanical execution
     â”‚        â†’ "Write descriptor to block X" (code does the writing)
     â”‚
     â”‚  3. Structured output + validation
     â”‚        â†’ JSON schema, grammar constraints, SHA verification
     â”‚
     â”‚  4. Explicit state passed every prompt
     â”‚        â†’ State machine, not memory
     â”‚
     â”‚  5. Rules in context
     â”‚        â†’ May be read, may not be applied
     â”‚        â†’ DIARY OPERATED HERE
     â”‚
     â”‚  6. Rules the model "should remember"
     â”‚        â†’ Will drift, will guess, will fail
     â”‚
LEAST RELIABLE
```

---

### 1.4.1 Why This Matters

The Diary spent 3 months writing ~1,232 clauses. These were **rules in context** (level 5). The LLM could read them but might not apply them. Every drift required more rules, which increased context load, which caused more drift.

Handshake breaks this cycle by implementing rules as **code** (level 1). The LLM can't violate a constraint that's enforced by the type system.

| Level | Diary Approach | Handshake Approach |
|-------|----------------|-------------------|
| 1 | â€” | Rust types, `LayerGuard`, immutable references |
| 2 | â€” | Mechanical pipelines (IMG-001, Docling) |
| 3 | â€” | JSON schema validation, Gate trait |
| 4 | Partial (state in prompts) | `StateSnapshot` in every request |
| 5 | **Primary** (RIDs in context) | Fallback only |
| 6 | Sometimes | Never |

---

**Key Takeaways**
- Design for the top of the hierarchy, never the bottom
- Code enforcement > structured output > rules in context
- The Diary's rules were level 5; Handshake implements them at level 1-3
- This is why Handshake will work where the Diary struggled

---

## 1.5 What Gets Ported from the Diary

**Why**  
Not everything from the Diary becomes Handshake code. Understanding the categories helps you know what to implement, what to configure, and what to skip.

**What**  
The ~1,232 Diary clauses fall into four categories: PORTED (becomes Rust types), TRANSFORMED (rules become code), PRESERVED (extraction core), and DEPRECATED (text-format specifics not needed).

**Jargon**  
- **PORTED**: Diary concepts that become Rust structs/enums directly.
- **TRANSFORMED**: Rules that were text now become code enforcement.
- **PRESERVED**: The extraction pipeline (the actual product).
- **DEPRECATED**: Text-format rules that Rust types make unnecessary.

---

### 1.5.1 PORTED: Concepts Become Rust Types

| Diary Concept | Handshake Implementation |
|---------------|--------------------------|
| Layers (L1/L2/L3) | `Layer` enum + `LayerGuard` |
| Work Modes | `WorkMode` enum + mode state machine |
| Gates (COR-701) | `Gate` trait + 11 implementations |
| PlannedOperation | `PlannedOperation` struct |
| DescriptorRow | `DescriptorRow` struct |
| SHOT_DNA | `ShotDna` struct with field enums |
| Flight Recorder | `FlightRecorder` append-only log |

---

### 1.5.2 TRANSFORMED: Rules Become Code Enforcement

| Diary Enforcement | Handshake Enforcement |
|-------------------|----------------------|
| "L1 is immutable" (text rule) | `&L1Content` (no `&mut`, compile-time) |
| "Must pass 11 gates" (text rule) | `GatePipeline::validate()` (runtime check) |
| "Use CONFIG vocab only" (text rule) | `Vocab::validate(value)` (type-checked) |
| RID lint rules | `ValidatorConfig` patterns |

---

### 1.5.3 PRESERVED: The Extraction Core (The Product)

| Extraction RID | What It Does                              | Status |
|----------------|-------------------------------------------|--------|
| DES-001        | Descriptor schema + extraction rules      | **Core product** |
| IMG-001        | Image â†’ Descriptor pipeline               | **Core product** |
| SYM-001        | SHOT_DNA â†’ Layer scores                   | **Core product** |
| TXT-001        | Text descriptor schema + extraction rules | **Core product** |

These are not governance overhead. **These are the point.**
---

### 1.5.4 DEPRECATED: Text-Format Specifics

| Diary Feature | Why Not Needed in Handshake |
|---------------|----------------------------|
| Rail patterns (`====`) | Rust structs replace text delimiters |
| Topic markers (`[[SUB:X]]`) | Struct fields replace markers |
| File naming conventions | Handshake manages its own storage |
| Text lint patterns | Type system prevents invalid states |

---

**Key Takeaways**
- ~400 clauses become Rust code (PORTED + TRANSFORMED)
- ~200 clauses become validator configs
- ~300 clauses are reference documentation
- ~180 clauses are deferred for post-MVP
- The extraction core (DES-001, IMG-001, SYM-001) is the actual product

---

## 1.6 Design Philosophy: Self-Enforcing Governance

**Why**  
Understanding why the Diary embeds its own enforcement explains a key principle Handshake must preserve: rules and their validators must live together.

**What**  
Traditional document governance fails because rules and enforcement are separate. The Diary embeds lint rules and machine code alongside the RIDs they enforce. Handshake must preserve this pattern.

**Jargon**  
- **Self-governance loop**: Rules, validators, and helpers all version together.
- **Embedded enforcement**: Lint rules live in the same document as the rules they check.
- **Provenance**: The trail from clause ID to code to test.

**Subsystem Laws (LAW blocks)**  
Some subsystems include an internal **LAW** section that is normative (example: **Calendar Law** in Â§10.4.1). For every LAW block, Handshake MUST ship:

1. A validator binding point (Gate / Orchestrator / Engine) that rejects bypass paths.
2. A governance compliance test suite (Â§5.4.6) proving the LAW holds.
3. A CI gate that runs those tests on every merge.

A LAW without (1)-(3) is incomplete scope, not â€œdocumentationâ€.

---

### 1.6.1 The Problem: Governance Drift

Traditional document governance fails because rules and enforcement are separate:

```
Human writes rule â†’ Human remembers to follow it â†’ Human checks own work â†’ Drift happens
```

Over time:
- Rules get forgotten or misremembered
- External linters drift from rule intent
- Scripts produce non-compliant output
- Nobody notices until it's too late

---

### 1.6.2 The Solution: Embedded Enforcement

The Diary embeds its own immune system. Rules, validators, and automation live together:

```
Human writes RID â†’ Lint checks compliance â†’ Machine code automates â†’ Consistency enforced
```

This creates a **self-governance loop**:

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                      DIARY                          â”‚
â”‚                                                     â”‚
â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     governs      â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚   â”‚   RIDs   â”‚â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¶â”‚   Helpers    â”‚   â”‚
â”‚   â”‚  (LAW)   â”‚                  â”‚(MACHINE_CODE)â”‚   â”‚
â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚        â”‚                               â”‚           â”‚
â”‚        â”‚ defines                       â”‚ checked   â”‚
â”‚        â–¼                               â–¼           â”‚
â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     enforces     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚   â”‚   Lint   â”‚â—€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”‚   Output     â”‚   â”‚
â”‚   â”‚  Rules   â”‚                  â”‚  (CORPUS)    â”‚   â”‚
â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                                     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 1.6.3 What This Means for Handshake

Handshake must preserve the self-governance property:

| Diary Component | Handshake Equivalent | Preservation Requirement |
|-----------------|---------------------|--------------------------|
| RIDs (LAW) | Clause-attributed code | `#[clause("ID", "desc")]` links code to law |
| Lint rules | `ValidatorConfig` | Patterns loaded from same source as rules |
| Helpers | Service implementations | Services carry clause provenance |
| CORPUS | Output artifacts | All output validated by same validators |

**Key principle:** In Handshake, a clause should never exist without its validator, and a validator should never exist without its clause. They are born together, live together, and die together.

---

### 1.6.4 Clause Provenance Pattern

Every implemented clause uses the provenance attribute:

```rust
#[clause("BL-270", "Treat change as PATCH not rewrite")]
pub struct PlannedOperation { ... }
```

For validator configs, use comments:

```rust
// FMT-140-F140-23: Rail pattern must match exactly
pub const RAIL_PATTERN: &str = r"^={4,}.*={4,}$";
```

For deferred clauses, use TODO:

```rust
// TODO(IMG001-180): Implement InputCollection stage (tracked; stub only in v02.32)
// Deferred: Image analysis is complex; using stub until implementation lands
pub fn input_collection_stub() -> Vec<ImageSource> { vec![] }
```

This enables:
- Grep for clause ID finds all related code
- Clause changes â†’ easy to find what needs updating
- Audit trail from law to implementation

---

**Key Takeaways**
- Rules and enforcement must live together
- The Diary's self-governance loop must be preserved in Handshake
- Every clause gets a `#[clause("ID", "desc")]` attribute
- Grep for clause ID finds all related code, tests, validators
- This is how you maintain 1,232 clauses without drift

---

## 1.7 Success Criteria

**Why**  
Clear success criteria tell you when the implementation is working. Without these, you can't know if you're done.

**What**  
Six checkpoints that define a working Handshake implementation.

---

The implementation works when:

1. **LLM outputs structured instruction** â€” not raw text edits
2. **Orchestrator validates against capability profile** â€” derived from RIDs
3. **Mechanical layer executes deterministically** â€” DES-001/IMG-001/SYM-001
4. **Validator confirms output** â€” gates pass, schema valid
5. **On failure: visible error, recoverable state** â€” Flight Recorder tracks everything
6. **On success: provenance tracked** â€” clause IDs in code, audit trail complete

**The LLM never directly touches document/descriptor content.**

---

**Key Takeaways**
- Success = all six criteria pass
- The LLM steering / software executing pattern is non-negotiable
- If you can't recover from a failure, the implementation is incomplete

---

<a id="18-introduction"></a>
## 1.8 Introduction

**Why**  
This section establishes the foundational identity, target users, and design philosophy of Handshake. Without this grounding, subsequent technical decisions lack context and rationale.

**What**  
Defines Handshake as a local-first, AI-native desktop workspace that unifies document editing, visual canvases, and spreadsheets. Documents the specification's evolution and clarifies its relationship to the underlying infrastructure research.

**Jargon**  
- **Local-first**: Data lives primarily on the user's device; cloud sync is optional and never required.
- **AI-native**: AI models and agents are integrated into the core data model and workflows from inception, not bolted on later.
- **Raw/Derived/Display**: Three-layer content separation where user content (Raw) is never silently modified, AI output (Derived) is regenerable, and UI rendering (Display) applies policy/formatting.
- **Desktop-first**: Initial target is a powerful workstation, with laptop and mobile coming later.

---

### 1.8.1 Product Vision & Guiding Principles

#### 1.8.1.1 What Handshake Is

Handshake is a **local-first, AI-enhanced desktop workspace** that unifies three major modes of work:

- **Notion-style docs and databases**
- **Milanote / Miro / tldraw-style visual canvases and moodboards**
- **Excel-style tables, formulas, and data manipulation**

All of these views sit on top of a **single local workspace data graph** backed by a robust data layer. The app is:

- **Desktop-first**, initially targeting a powerful workstation (Ryzen + 128GB RAM + RTX-class GPU), with later paths to laptop and eventually mobile.
- **Local-first**, offline-capable by default, with optional sync and small-team collaboration later.
- **AI-native**, not AI-bolted-on â€“ models and agents are integrated into the data model, workflows, and UX from day one.

#### 1.8.1.2 Target Users

**Primary:**
- A single power user (you) running heavy local models, building workflows, and using the app as a personal production studio, research hub, and coding assistant.

**Longer-term:**
- Small creative / technical teams who need a private, sovereign workspace with powerful AI but without SaaS lock-in or cloud dependence.

#### 1.8.1.3 Guiding Principles

1. **Local-first, truly sovereign**
   - Data lives on the user's machine first. Sync is optional, encrypted, and never assumed.
   - Cloud models are **optional helpers**, not hard dependencies.

2. **Raw / Derived / Display separation**
   - **RawContent** is user-authored content and canonical external inputs. It is never silently changed by AI or filters.
   - **DerivedContent** is AI-generated metadata, summaries, plans, embeddings, layouts, taste descriptors, etc.
   - **DisplayContent** is what the UI shows and what gets exported, including safety filtering and formatting.
   - Censorship and policy enforcement apply **only at Display/Export**, never to Raw/Derived.

3. **AI as collaborator, not overlord**
   - AI is treated as a **co-editor/agent** with its own identity in the data/sync layer, not as magical hidden automation.
   - Every AI action is inspectable, revertible, and attributed.

4. **Composable, inspectable workflows**
   - Automations and agents operate through explicit, typed workflows, not opaque monolithic "magic" buttons.
   - Users can see, edit, disable, or delete anything the system automates for them.

5. **Safety through architecture, not just prompts**
   - Capability-limited tools, sandboxing, durable logs, and typed operations are the main safety tools.
   - Prompts and policy text are layered on top of a secure foundation, not a replacement for it.

6. **Progressive complexity**
   - MVP focuses on single-user workflows and a small set of high-value AI capabilities.
   - More complex multi-agent orchestration, collaboration, and marketplaces come later, on top of a stable core.

### 1.8.1.4 Worksurfaces and Modules (product framing; normative)

Handshake is a single product umbrella. It is composed of **modules**, each of which may expose one or more **worksurfaces**.

- **Worksurface:** a user-facing editor or operator surface bound to the Workspace Graph and AI Job Model (Docs, Canvas, Tables, Media, Stage, Dev Command Center, Operator Consoles).
- **Module:** a cohesive package of contracts + implementations that may include:
  - worksurface UI(s)
  - AI job profiles and workflows
  - validators / gates / policies
  - Mechanical Tool Bus engines (MEX)
  - canonical tool definitions (Unified Tool Surface Contract, Â§6.0.2)
  - capability bindings + consent UX

**Naming guardrail (MUST):**
- The product is **Handshake**.
- â€œDesign Studioâ€ is an additive module inside Handshake (canvas-first design workflows). It does **not** rename Handshake and does **not** replace other modules.
- â€œStageâ€ is an additive module inside Handshake (spatial/3D workflows). It does **not** replace Docs/Canvas/Tables/Media.
- UI labels MAY use a prefix like `Studio:` for worksurfaces, but this is naming/IA only and MUST NOT change capability boundaries or storage layout.

This framing exists to prevent â€œsurface sprawlâ€: new capabilities should be introduced as modules that reuse shared primitives (Workspace Graph, AI Job Model, Tool Surface, Flight Recorder, Capability System) rather than as one-off pipelines.

---

### 1.8.2 Specification Evolution

This document integrates multiple research sources:

- **GPT-4o Handshake research paper (v1.0)** â€” Original architecture and vision
- **Gemini COMBO synthesis** â€” Shadow Workspace, graph/relational data stack, taste engine implementation, capability tokens, Flight Recorder patterns
- **Claude Opus 4.5 research** â€” AI interaction patterns, workflow safety model, RAG/indexing patterns, doc/canvas behaviors, dev-tools and terminal/agent safety
- **Docs & Sheets AI Integration Protocol** â€” AI jobs over documents and tables with stable IDs and provenance
- **Prompt Diaries governance (v02.00)** â€” ~1,232 clauses of governance R&D, extraction pipeline, validation gates

**Major additions in this version:**

| Area | What Was Added |
|------|----------------|
| **Data & Indexing** | Shadow Workspace with incremental parsing, graph-relational knowledge graph, hybrid retrieval |
| **Collaboration** | CRDTs (Yjs) as core Humanâ€“AI concurrency fabric, AI as first-class CRDT site ID |
| **Implementation** | Rust coordinator + Tauri + React desktop shell, Model Runtime Layer, embedded local stores |
| **Security** | WASI-style capability model, capability contracts, scoped tokens |
| **Observability** | Flight Recorder for full trace logging and replay |
| **AI UX** | Command palette, structural editor, background agent patterns tied to Raw/Derived/Display |
| **Taste Engine** | CLIP embeddings, authorial LoRA adapters, JSON taste descriptors, DPO-style learning |
| **Workflows** | Typed node set, strong validation pipeline, durable local execution |
| **Governance** | Diary RIDs ported to code â€” layers, gates, modes, extraction pipeline |

---

### 1.8.3 Relationship to Base Research

This document defines **behaviours, UX patterns, and architectural constraints** for Handshake.
All underlying infrastructure choices for:

- Storage and sync (file-tree, CRDTs, databases)
- Inference runtimes and model hosting
- Plugin / extension patterns and sandboxing
- Observability and benchmarking

...are inherited from the broader research document `Project_Handshake_Research_merged_v2`.

Where this spec talks about graphs, runtimes, logging, or workflows, it is describing **how to use those base mechanisms** rather than introducing parallel infrastructure. If there is any ambiguity, the base research document is the reference for concrete tool/runtime/database selection; this spec is the reference for how those pieces should behave together.

---

**Key Takeaways**
- Handshake is a sovereign, offline-capable desktop workspace combining docs, canvases, and tables with deep AI integration
- The specification evolved through multi-model research synthesis (GPT-4o, Gemini, Claude), now including Diary governance
- Behavioral spec and infrastructure research are complementary views of one architecture
- Core principles: local-first sovereignty, Raw/Derived/Display separation, AI as attributed collaborator, safety through architecture

---


<a id="2-system-architecture"></a>
