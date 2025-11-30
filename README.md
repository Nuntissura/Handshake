# PROJECT HANDSHAKE - Research Document

**Version:** 1.0 | **Date:** November 29, 2025 | **Status:** Foundation Research  
**Purpose:** Complete technical research compilation for building a local-first AI-powered desktop application  
**Target:** Non-technical founder building an intelligent productivity platform

---

## Project Context

- **What we're building:** A desktop application that runs multiple AI models locally on your computer, with collaborative document editing, plugin extensibility, and privacy-first design. Think Notion meets local ChatGPT with the extensibility of VS Code.
- **Current stage:** Pre-development research phase
- **Key constraints:** Single RTX 3090 GPU (24GB VRAM), Windows-first, solo/small team development, local-first (works offline)
- **Success criteria:** A working desktop app where users can edit documents/boards/spreadsheets, use AI assistants for various tasks, install third-party plugins safely, and sync across devices—all while keeping data primarily on their own machine.

## Document History

- v1.0 (November 29, 2025): Initial research compilation from 8 source documents covering LLM infrastructure, data sync, plugins, security, and observability

---

## How to Use This Document

📌 **For Learning:** Read sections in order—each builds on previous concepts  
📌 **For Reference:** Use the Table of Contents to jump to specific topics  
📌 **For Implementation:** Look for `✓ Action Items` and `Decision Points` boxes  
📌 **For LLM Context:** Include relevant section anchors when asking for help

**Reading Time Estimates:**
- Quick skim (headers + key takeaways): ~30 minutes
- Core concepts only (`[CORE]` sections): ~2 hours
- Complete read-through: ~4-5 hours

---

## Table of Contents

### Part I: Foundations
- [1.0 Understanding the Big Picture](#1-understanding-the-big-picture)
  - [1.1 What is a Local-First Application?](#11-what-is-a-local-first-application)
  - [1.2 Project Architecture Overview](#12-project-architecture-overview)
  - [1.3 Hardware Context: The RTX 3090 Setup](#13-hardware-context-the-rtx-3090-setup)

### Part II: LLM Infrastructure
- [2.0 LLM Fundamentals](#2-llm-fundamentals)
  - [2.1 How LLMs Work (Simplified)](#21-how-llms-work-simplified)
  - [2.2 Key Concepts: Tokens, VRAM, Quantization](#22-key-concepts-tokens-vram-quantization)
  - [2.3 Model Sizes and What Fits](#23-model-sizes-and-what-fits)
- [3.0 LLM Inference Runtimes](#3-llm-inference-runtimes)
  - [3.1 What is an Inference Runtime?](#31-what-is-an-inference-runtime)
  - [3.2 Runtime Comparison: Ollama vs vLLM vs TGI vs Others](#32-runtime-comparison)
  - [3.3 Recommended Runtime Strategy](#33-recommended-runtime-strategy)
- [4.0 Model Selection & Roles](#4-model-selection-and-roles)
  - [4.1 Specialized Models for Different Tasks](#41-specialized-models-for-different-tasks)
  - [4.2 Model Recommendations by Role](#42-model-recommendations-by-role)
  - [4.3 GPU Memory Management](#43-gpu-memory-management)
  - [4.4 Scheduling & Contention](#44-scheduling-and-contention)
- [5.0 Image Generation (Stable Diffusion)](#5-image-generation)
  - [5.1 SD vs SDXL Overview](#51-sd-vs-sdxl-overview)
  - [5.2 VRAM Requirements & Performance](#52-vram-requirements-and-performance)
  - [5.3 Integrating with LLM Workloads](#53-integrating-with-llm-workloads)

### Part III: Data Architecture
- [6.0 Local-First Data Fundamentals](#6-local-first-data-fundamentals)
  - [6.1 What "Local-First" Really Means](#61-what-local-first-really-means)
  - [6.2 The Problem: Concurrent Editing](#62-the-problem-concurrent-editing)
  - [6.3 Solution: CRDTs Explained](#63-solution-crdts-explained)
- [7.0 CRDT Libraries Comparison](#7-crdt-libraries-comparison)
  - [7.1 Yjs Deep Dive](#71-yjs-deep-dive)
  - [7.2 Automerge Deep Dive](#72-automerge-deep-dive)
  - [7.3 Loro and Emerging Options](#73-loro-and-emerging-options)
  - [7.4 Recommendation: Which CRDT Library?](#74-recommendation-which-crdt-library)
- [8.0 Database & Sync Patterns](#8-database-and-sync-patterns)
  - [8.1 Local Database Options (SQLite)](#81-local-database-options)
  - [8.2 Combining CRDT + Database](#82-combining-crdt-and-database)
  - [8.3 Sync Topologies](#83-sync-topologies)
- [9.0 Conflict Resolution UX](#9-conflict-resolution-ux)
  - [9.1 User-Facing Conflict Patterns](#91-user-facing-conflict-patterns)
  - [9.2 Version History UI](#92-version-history-ui)

### Part IV: Plugin & Extension System
- [10.0 Plugin Architecture Fundamentals](#10-plugin-architecture-fundamentals)
  - [10.1 Why Plugins Matter](#101-why-plugins-matter)
  - [10.2 Learning from Existing Systems](#102-learning-from-existing-systems)
- [11.0 Plugin System Design](#11-plugin-system-design)
  - [11.1 Manifest & Registration](#111-manifest-and-registration)
  - [11.2 Plugin Types & Categories](#112-plugin-types-and-categories)
  - [11.3 API Design Patterns](#113-api-design-patterns)
- [12.0 Sandboxing & Security](#12-sandboxing-and-security)
  - [12.1 Why Sandbox Untrusted Code](#121-why-sandbox-untrusted-code)
  - [12.2 Sandboxing Technologies Compared](#122-sandboxing-technologies-compared)
  - [12.3 Permission Models](#123-permission-models)
  - [12.4 Recommended Security Architecture](#124-recommended-security-architecture)

### Part V: Observability & Testing
- [13.0 AI Observability](#13-ai-observability)
  - [13.1 What to Monitor in AI Apps](#131-what-to-monitor-in-ai-apps)
  - [13.2 Tools Comparison](#132-tools-comparison)
  - [13.3 Privacy-Sensitive Logging](#133-privacy-sensitive-logging)
  - [13.4 Metrics & Dashboards](#134-metrics-and-dashboards)
- [14.0 Evaluation & Quality](#14-evaluation-and-quality)
  - [14.1 Testing LLM Outputs](#141-testing-llm-outputs)
  - [14.2 Multi-Agent Tracing](#142-multi-agent-tracing)
- [15.0 Benchmark Harness](#15-benchmark-harness)
  - [15.1 Benchmark Architecture](#151-benchmark-architecture)
  - [15.2 Scenarios & Adapters](#152-scenarios-and-adapters)
  - [15.3 Reporting & Analysis](#153-reporting-and-analysis)

### Part VI: Implementation
- [16.0 Technology Stack Summary](#16-technology-stack-summary)
- [17.0 Implementation Roadmap](#17-implementation-roadmap)
- [18.0 Gap Analysis & Open Questions](#18-gap-analysis)

### End Matter
- [Consolidated Glossary](#consolidated-glossary)
- [Sources Referenced](#sources-referenced)

---
---

### Part VII: Consolidated Architecture & Roadmap
- [19. Executive Summary ](#19-executive-summary)
- [20. Foundation Concepts ](#20-foundation-concepts)
  - [20.1 What is a Desktop Application Shell? ](#201-what-is-a-desktop-application-shell)
  - [20.2 Understanding Local-First Software ](#202-understanding-local-first-software)
  - [20.3 What are AI Models and How Do They Run Locally? ](#203-what-are-ai-models-and-how-do-they-run-locally)
  - [20.4 Multi-Model Orchestration Explained ](#204-multi-model-orchestration-explained)
- [21. Architecture Decisions ](#21-architecture-decisions)
  - [21.1 Desktop Shell: Tauri vs Electron ](#211-desktop-shell-tauri-vs-electron)
  - [21.2 Overall System Architecture ](#212-overall-system-architecture)
  - [21.3 Data Architecture: File-Tree Model ](#213-data-architecture-file-tree-model)
- [22. User Interface Components ](#22-user-interface-components)
  - [22.1 Rich Text Editor (Notion-like) ](#221-rich-text-editor-notion-like)
  - [22.2 Freeform Canvas (Milanote-like) ](#222-freeform-canvas-milanote-like)
  - [22.3 Spreadsheet Engine (Excel-like) ](#223-spreadsheet-engine-excel-like)
  - [22.4 Additional Views: Kanban, Calendar, Timeline ](#224-additional-views-kanban-calendar-timeline)
- [23. AI Model Strategy ](#23-ai-model-strategy)
  - [23.1 Model Categories and Recommendations ](#231-model-categories-and-recommendations)
  - [23.2 Local Model Runtimes ](#232-local-model-runtimes)
  - [23.3 Cloud Fallback Strategy ](#233-cloud-fallback-strategy)
  - [23.4 Image Generation with ComfyUI ](#234-image-generation-with-comfyui)
- [24. Multi-Agent Orchestration ](#24-multi-agent-orchestration)
  - [24.1 Framework Comparison: AutoGen vs LangGraph vs CrewAI ](#241-framework-comparison-autogen-vs-langgraph-vs-crewai)
  - [24.2 The Lead/Worker Pattern ](#242-the-leadworker-pattern)
  - [24.3 Shared Context and Memory ](#243-shared-context-and-memory)
  - [24.4 Task Routing and Fallback Logic ](#244-task-routing-and-fallback-logic)
- [25. Collaboration and Sync ](#25-collaboration-and-sync)
  - [25.1 Understanding CRDTs ](#251-understanding-crdts)
  - [25.2 Offline-First Architecture ](#252-offline-first-architecture)
  - [25.3 Google Workspace Integration ](#253-google-workspace-integration)
- [26. Plugin and Extension System ](#26-plugin-and-extension-system)
  - [26.1 Plugin Architecture Patterns ](#261-plugin-architecture-patterns)
  - [26.2 Security and Sandboxing ](#262-security-and-sandboxing)
- [27. Reference Application Analysis ](#27-reference-application-analysis)
  - [27.1 AppFlowy ](#271-appflowy)
  - [27.2 AFFiNE ](#272-affine)
  - [27.3 Obsidian ](#273-obsidian)
  - [27.4 Logseq ](#274-logseq)
  - [27.5 Lessons Learned ](#275-lessons-learned)
- [28. Development Workflow ](#28-development-workflow)
  - [28.1 Using AI Coding Assistants Effectively ](#281-using-ai-coding-assistants-effectively)
  - [28.2 Project Health and Hygiene ](#282-project-health-and-hygiene)
  - [28.3 CI/CD and Testing Strategy ](#283-cicd-and-testing-strategy)
- [29. Development Roadmap ](#29-development-roadmap)
  - [29.1 Phase Overview ](#291-phase-overview)
  - [29.2 MVP Definition ](#292-mvp-definition)
  - [29.3 Build Order and Dependencies ](#293-build-order-and-dependencies)
- [30. Risk Assessment ](#30-risk-assessment)
- [31. Technology Stack Summary ](#31-technology-stack-summary)
- [32. Consolidated Glossary ](#32-consolidated-glossary)
- [33. Open Questions and Next Steps ](#33-open-questions-and-next-steps)
- [34. Sources Referenced ](#34-sources-referenced)

# PART I: FOUNDATIONS

---

## 1.0 Understanding the Big Picture {#1-understanding-the-big-picture}

**Prerequisites:** None - foundational  
**Related to:** All subsequent sections  
**Implements:** Core project understanding  
**Read time:** ~15 minutes

**This section explains what we're building and why, establishing the mental model you'll use throughout this document.**

---

### 1.1 What is a Local-First Application? {#11-what-is-a-local-first-application}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters for This Project |
|------|---------------|--------------------------------|
| **Local-First** | Your data lives primarily on your computer, not in "the cloud" (someone else's computer). The app works fully offline. | This is our core philosophy—users own their data, AI runs locally, and the app works without internet |
| **Cloud-First** | The opposite—your data lives on company servers, and you need internet to use the app (like Google Docs) | What we're NOT building. Understand this to understand our tradeoffs |
| **Offline-Capable** | Can work without internet temporarily, but really needs the cloud | Weaker than local-first. We want TRUE local-first |
| **Sync** | Keeping data consistent across multiple devices (your laptop and phone showing the same notes) | We want this eventually, but local-first makes it harder |

#### The Core Idea

**Local-first means your computer is the primary home for your data, and the cloud is just a backup or sync helper.** Traditional apps like Google Docs work the opposite way: Google's servers hold the "real" copy, and your browser just shows you a window into it.

```
CLOUD-FIRST (Google Docs):                LOCAL-FIRST (What we're building):
                                          
┌─────────────────────┐                   ┌─────────────────────┐
│   Google's Servers  │ ← "Real" data     │   YOUR Computer     │ ← "Real" data
│   (the cloud)       │                   │                     │
└─────────┬───────────┘                   └─────────┬───────────┘
          │                                         │
          ▼                                         ▼
┌─────────────────────┐                   ┌─────────────────────┐
│   Your Browser      │ ← Just a window   │   Cloud (optional)  │ ← Backup/sync
└─────────────────────┘                   └─────────────────────┘
```

#### Why Local-First for This Project?

📌 **Privacy:** AI processes your documents locally. Your private notes never leave your machine.

📌 **Speed:** No waiting for server round-trips. The AI model is right there on your GPU.

📌 **Ownership:** Your data is literally files on your computer. No company can lock you out.

📌 **Offline:** Works on airplanes, in basements, anywhere. No "you're offline" errors.

⚠️ **The Tradeoff:** Syncing between devices becomes much harder. When two devices edit the same document offline, we need special technology (CRDTs) to merge the changes. This is covered in [Part III](#6-local-first-data-fundamentals).

#### Key Takeaways

- Local-first = your computer holds the authoritative data, cloud is secondary
- This gives us privacy, speed, and offline capability
- The main challenge is syncing between devices (covered later)
- We're building a desktop app, not a web app, which makes local-first natural

---

### 1.2 Project Architecture Overview {#12-project-architecture-overview}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **Desktop App** | An application you install on your computer (like Word or Photoshop), not one you use in a browser | We're building this, not a website |
| **Tauri** | A framework for building desktop apps using web technologies (HTML, CSS, JavaScript) with a Rust backend | Our likely app framework—lighter than Electron |
| **Electron** | Another desktop app framework (used by VS Code, Slack, Discord) | Alternative to Tauri, heavier but more mature |
| **Frontend** | The part users see and interact with (buttons, text fields, etc.) | Our user interface |
| **Backend/Orchestrator** | The "brain" that handles logic, talks to AI models, manages data | Where the complex stuff happens |
| **GPU** | Graphics Processing Unit—originally for games, now also runs AI models very fast | Our RTX 3090 runs the AI |

#### The Big Picture

Our app has four major layers that work together:

```
┌────────────────────────────────────────────────────────────────┐
│                    USER INTERFACE (Frontend)                    │
│         Documents | Boards | Spreadsheets | Chat | Settings     │
│                        [Tauri + React/Vue]                      │
└────────────────────────────────┬───────────────────────────────┘
                                 │ Commands & Events
                                 ▼
┌────────────────────────────────────────────────────────────────┐
│                   ORCHESTRATOR (Python Backend)                 │
│  • Routes requests to appropriate AI models                     │
│  • Manages which models are loaded                              │
│  • Handles plugin execution                                     │
│  • Coordinates data sync                                        │
└───────────┬──────────────────┬─────────────────┬───────────────┘
            │                  │                 │
            ▼                  ▼                 ▼
┌───────────────────┐ ┌────────────────┐ ┌──────────────────────┐
│   LLM RUNTIMES    │ │  LOCAL DATA    │ │    PLUGIN SYSTEM     │
│ (Ollama, vLLM)    │ │ (SQLite+CRDT)  │ │  (Sandboxed code)    │
│                   │ │                │ │                      │
│ • Mistral-7B      │ │ • Documents    │ │ • User automations   │
│ • CodeLlama       │ │ • Boards       │ │ • AI tools           │
│ • Creative LLM    │ │ • Spreadsheets │ │ • Integrations       │
│ • SDXL (images)   │ │ • Sync state   │ │                      │
└─────────┬─────────┘ └───────┬────────┘ └──────────────────────┘
          │                   │
          ▼                   ▼
┌───────────────────┐ ┌────────────────┐
│   RTX 3090 GPU    │ │   Hard Drive   │
│   (24GB VRAM)     │ │   (Files)      │
└───────────────────┘ └────────────────┘
```

#### Component Breakdown

**1. User Interface (Frontend)**
- What users see: text editor, kanban boards, spreadsheets, chat interface
- Built with web technologies (HTML/CSS/JavaScript) inside a desktop wrapper
- Communicates with the backend through local messages (IPC)

**2. Python Orchestrator (Backend)**
- The "brain" that coordinates everything
- Decides which AI model to use for each task
- Manages GPU memory (can't run everything at once)
- Handles plugin permissions and execution

**3. LLM Runtimes**
- Software that actually runs AI models
- We'll likely use Ollama (easy) and/or vLLM (fast)
- Exposes models through a standardized API

**4. Local Data Layer**
- SQLite database for structured data
- CRDT library (Yjs) for collaborative editing
- Files stored on local disk

**5. Plugin System**
- Lets users/developers extend the app
- Runs in a sandbox for security
- Can add new AI tools, automations, integrations

#### Key Takeaways

- Four main layers: UI → Orchestrator → Services → Hardware
- Python orchestrator is the central coordinator
- AI models run on the GPU via runtime software
- Data lives locally in SQLite + files
- Plugins extend functionality in a sandboxed environment

---

### 1.3 Hardware Context: The RTX 3090 Setup {#13-hardware-context-the-rtx-3090-setup}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **VRAM** | Video RAM—memory on your graphics card. AI models must fit here to run fast | Our 24GB limit determines which/how many models we can run |
| **System RAM** | Regular computer memory (your 128GB) | Backup when VRAM is full, but much slower |
| **GPU** | The graphics card processor itself | Does the actual AI computation |
| **CUDA** | NVIDIA's technology for running non-graphics computations on GPUs | Required for our AI workloads |
| **Bandwidth** | How fast data can move (like a pipe's width) | GPU memory is ~6x faster than system RAM |

#### Your Hardware Profile

```
┌─────────────────────────────────────────────────────────┐
│                   YOUR SETUP                            │
├─────────────────────────────────────────────────────────┤
│  CPU:  AMD Ryzen 5950X (16 cores, 32 threads)          │
│  RAM:  128 GB DDR4                                      │
│  GPU:  NVIDIA RTX 3090 (24 GB VRAM)                    │
│  OS:   Windows                                          │
└─────────────────────────────────────────────────────────┘
```

#### What 24GB VRAM Means for Us

**VRAM is the critical constraint.** AI models must be loaded into VRAM to run at full speed. Think of VRAM like a desk—you can only have so many documents open at once.

```
═══════════════════════════════════════════════════════════════
                    CORE CONCEPT: VRAM BUDGET
═══════════════════════════════════════════════════════════════
  
  24 GB Total VRAM
  ├── ~1-2 GB: System/driver overhead (always used)
  ├── Remaining: ~22 GB for models
  │
  │   Example allocations:
  │   ┌─────────────────────────────────────────────────┐
  │   │ Option A: Two medium models + headroom         │
  │   │   Mistral-7B (4GB) + CodeLlama-7B (4GB)       │
  │   │   = 8GB used, 14GB free for context/images    │
  │   ├─────────────────────────────────────────────────┤
  │   │ Option B: One large model                      │
  │   │   Llama2-70B-4bit (17GB)                       │
  │   │   = 17GB used, 5GB free (tight!)              │
  │   ├─────────────────────────────────────────────────┤
  │   │ Option C: Medium model + image generation      │
  │   │   Mistral-7B (4GB) + SDXL (7-10GB)            │
  │   │   = 11-14GB used                               │
  │   └─────────────────────────────────────────────────┘

═══════════════════════════════════════════════════════════════
```

#### The Speed Difference: GPU vs CPU

⚡ **Critical:** Running models from GPU VRAM is approximately 6x faster than running them from system RAM.

| Where Model Lives | Speed | When to Use |
|-------------------|-------|-------------|
| GPU VRAM | ~50-130 tokens/sec | Always prefer this |
| System RAM (CPU) | ~8-20 tokens/sec | Last resort / fallback |

This is why we obsess over VRAM management—moving models to CPU makes the app feel sluggish.

#### Practical Rules of Thumb

📌 **Model Size Formula:** A 7B parameter model at 4-bit quantization ≈ 4GB VRAM

📌 **Safe Concurrent Limit:** 2-3 small models (7B) OR 1-2 medium models (13B) at once

📌 **Don't Mix Heavy Workloads:** Running SDXL image generation while querying a large LLM will likely exceed VRAM

📌 **Buffer for Context:** Long conversations use extra VRAM for "context" (what the model remembers). Budget 2-4GB headroom.

#### Key Takeaways

- 24GB VRAM is generous but not unlimited
- GPU memory is ~6x faster than system RAM—avoid CPU fallback
- Plan to run 2-3 small models OR 1-2 medium models concurrently
- Heavy image generation (SDXL) competes with LLMs for VRAM
- Always leave headroom for context and system overhead

---
---

# PART II: LLM INFRASTRUCTURE

---

## 2.0 LLM Fundamentals {#2-llm-fundamentals}

**Prerequisites:** Section 1.3 (Hardware Context)  
**Related to:** Sections 3, 4, 5  
**Implements:** Understanding needed to choose models and runtimes  
**Read time:** ~20 minutes

**This section explains how Large Language Models work at the level needed to make good decisions about which models to use and how to run them.**

---

### 2.1 How LLMs Work (Simplified) {#21-how-llms-work-simplified}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **LLM** | Large Language Model—AI that generates human-like text by predicting the next word | The core AI technology we're using |
| **Parameters** | The "knowledge" of a model, stored as numbers. More parameters = more knowledge but more memory | Determines model capability and size |
| **Inference** | Using a trained model to generate outputs (vs. "training" which creates the model) | We do inference, not training |
| **Prompt** | The text you give the model as input | What users type |
| **Completion** | The text the model generates in response | What the AI responds with |

#### The Basic Idea

**An LLM is a very sophisticated autocomplete.** Given some text, it predicts what text should come next—but it's so good at this that it can write essays, code, answer questions, and more.

```
You type:       "Write a haiku about programming"
                           │
                           ▼
                  ┌─────────────────┐
                  │   LLM Model     │
                  │  (Billions of   │
                  │   parameters)   │
                  └────────┬────────┘
                           │
                           ▼
Model outputs:  "Code flows like water
                 Bugs emerge from the depths below
                 Debug, rinse, repeat"
```

#### What "Parameters" Mean

Think of parameters as the model's "brain cells"—connections that store patterns learned from training data.

```
Model Size Guide:
─────────────────────────────────────────────────────────
  3B-4B   │  Small  │  Fast, limited capability     
  7B-8B   │  Medium │  Good balance, our sweet spot 
  13B     │  Large  │  Better quality, slower       
  27B-30B │  XL     │  Near-GPT-3.5 quality         
  70B+    │  XXL    │  Best quality, very demanding 
─────────────────────────────────────────────────────────
```

💡 **For our project:** 7B-13B models hit the sweet spot of quality vs. resource usage on a 3090.

#### Key Takeaways

- LLMs predict "what text comes next" so well they seem intelligent
- More parameters = smarter but hungrier for resources
- We'll use 7B-13B models as our primary workhorses
- We do "inference" (using models), not "training" (creating models)

---

### 2.2 Key Concepts: Tokens, VRAM, Quantization {#22-key-concepts-tokens-vram-quantization}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **Token** | A chunk of text (roughly ¾ of a word). Models think in tokens, not letters or words | How we measure input/output size and cost |
| **Context Window** | How many tokens a model can "see" at once (its working memory) | Limits how much conversation history or document text we can include |
| **Quantization** | Compressing a model to use less memory by reducing number precision | How we fit big models into limited VRAM |
| **Q4/Q5/Q8** | Quantization levels: Q4 = 4-bit (smallest), Q8 = 8-bit (highest quality) | Trade-off between size and quality |
| **GGUF** | A file format for quantized models, works with llama.cpp | The format most local models use |

#### Understanding Tokens

**Tokens are how models measure text.** One token ≈ 4 characters ≈ 0.75 words.

```
Example tokenization:
"Hello, how are you today?" 
→ ["Hello", ",", " how", " are", " you", " today", "?"]
→ 7 tokens

Rough conversion:
  100 tokens  ≈ 75 words   ≈ 1 short paragraph
  1000 tokens ≈ 750 words  ≈ 1.5 pages
  4000 tokens ≈ 3000 words ≈ 6 pages
```

📌 **Why tokens matter:** 
- Models have a maximum context window (e.g., 4096 or 8192 tokens)
- Cloud APIs charge per token
- More tokens = slower responses and more memory

#### Understanding Context Windows

**The context window is the model's "working memory."** It includes BOTH your prompt AND the model's response.

```
┌─────────────────────────────────────────────────────────┐
│              4096 TOKEN CONTEXT WINDOW                  │
├─────────────────────────────────────────────────────────┤
│  System prompt (instructions)     │  ~200 tokens       │
│  Conversation history             │  ~2000 tokens      │
│  Current user message             │  ~300 tokens       │
│  ─────────────────────────────────┼────────────────────│
│  Space for model's response       │  ~1596 tokens      │
└─────────────────────────────────────────────────────────┘
```

⚠️ **Warning:** Long conversations eventually "forget" earlier messages when context fills up.

#### Understanding Quantization

**Quantization shrinks models by reducing number precision.** Like saving a photo as JPEG instead of RAW—smaller file, slight quality loss.

```
═══════════════════════════════════════════════════════════════
                    CORE CONCEPT: QUANTIZATION
═══════════════════════════════════════════════════════════════
  
  Original model: 7B parameters at 16-bit = ~14 GB
  
  Quantized versions:
  ┌──────────┬──────────┬─────────────┬────────────────────┐
  │ Format   │ Bits     │ Size        │ Quality Loss       │
  ├──────────┼──────────┼─────────────┼────────────────────┤
  │ Q8_0     │ 8-bit    │ ~7 GB       │ Minimal (<1%)      │
  │ Q5_K_M   │ 5-bit    │ ~5 GB       │ Very small (~1-2%) │
  │ Q4_K_M   │ 4-bit    │ ~4 GB       │ Small (~2-3%)      │ ← Sweet spot
  │ Q3_K_M   │ 3-bit    │ ~3 GB       │ Noticeable (~5%)   │
  └──────────┴──────────┴─────────────┴────────────────────┘
  
  📌 Q4_K_M is the most common choice: good quality, big savings

═══════════════════════════════════════════════════════════════
```

💡 **For our project:** We'll primarily use Q4_K_M quantized models in GGUF format.

#### VRAM Usage: Putting It Together

```
Formula for VRAM estimate:
  VRAM ≈ (Parameters in billions) × (Bits ÷ 2) GB
  
  Examples with Q4 (4-bit):
  • 7B model:  7 × (4÷2) = 7 × 2 = ~3.5-4 GB
  • 13B model: 13 × (4÷2) = 13 × 2 = ~6.5-8 GB  
  • 70B model: 70 × (4÷2) = 70 × 2 = ~35 GB... but actually fits in ~17-18GB 
                          (due to efficient formats)
```

#### Key Takeaways

- Tokens ≈ 0.75 words; context window limits total conversation length
- Quantization (Q4/Q5) shrinks models 3-4x with minimal quality loss
- GGUF is the standard format for local quantized models
- 7B Q4 model ≈ 4GB VRAM; this is our planning baseline

---

### 2.3 Model Sizes and What Fits {#23-model-sizes-and-what-fits}

`[CORE]`

#### Quick Reference Table

| Model Size | Quantization | VRAM Needed | Speed (tokens/sec) | Quality Level |
|------------|--------------|-------------|-------------------|---------------|
| 3-4B | Q4 | ~2-3 GB | 60-200 | Basic tasks |
| 7-8B | Q4 | ~4-5 GB | 50-130 | Good general use |
| 13B | Q4 | ~7-9 GB | 30-70 | Very good |
| 27B | Q4 | ~14 GB | 20-30 | Excellent |
| 70B | Q4 | ~17-18 GB | 10-15 | Near GPT-3.5 |

#### What Fits on Our 24GB RTX 3090?

```
Scenario Planning for 24 GB VRAM:
──────────────────────────────────────────────────────────────

✓ COMFORTABLE (with headroom):
  • 3× 7B models (12 GB) + context buffer
  • 2× 13B models (16 GB) + some headroom  
  • 1× 7B + 1× 13B + 1× 4B (15 GB)

⚡ TIGHT (works but careful):
  • 1× 70B model (17-18 GB) alone
  • 1× 27B + 1× 7B (18 GB)
  • 2× 7B + SDXL image generation (8 + 10 = 18 GB)

✗ WON'T FIT:
  • 2× 70B models (34+ GB)
  • 70B + any substantial other model
  • Multiple 27B+ models
```

#### Key Takeaways

- Our sweet spot: 2-3 models in the 7B-13B range loaded simultaneously
- One 70B model is possible but leaves little room for anything else
- Always budget 2-4GB headroom for context and system overhead

---

## 3.0 LLM Inference Runtimes {#3-llm-inference-runtimes}

**Prerequisites:** Section 2.0 (LLM Fundamentals)  
**Related to:** Sections 4.0, 5.0  
**Implements:** Runtime infrastructure decisions  
**Read time:** ~25 minutes

**This section compares the software that actually runs LLM models, helping you choose the right tool for different scenarios.**

---

### 3.1 What is an Inference Runtime? {#31-what-is-an-inference-runtime}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **Runtime** | Software that loads and runs AI models | We need this to use any LLM |
| **API** | Application Programming Interface—a way for programs to talk to each other | How our app communicates with the runtime |
| **OpenAI-compatible API** | An API that works like OpenAI's, so code written for ChatGPT works locally | Makes integration easy |
| **Streaming** | Sending response tokens one at a time as they're generated (vs. waiting for the full response) | Better user experience—text appears progressively |
| **Batching** | Processing multiple requests together for efficiency | Important for handling many users/requests |

#### The Role of an Inference Runtime

**A runtime is the software layer between your application and the AI model.** It handles:

```
Your App                    Runtime                     GPU
┌─────────┐    HTTP API    ┌──────────┐   CUDA/GPU    ┌─────────┐
│ "Write  │ ──────────────>│ • Load   │ ───────────> │ Matrix  │
│  me a   │                │   model  │              │ math on │
│  poem"  │ <──────────────│ • Run    │ <─────────── │ tensors │
└─────────┘   Streaming    │   infer  │              └─────────┘
              Response     │ • Manage │
                          │   memory │
                          └──────────┘
```

#### Why Runtime Choice Matters

Different runtimes optimize for different things:

| Priority | Best Runtime | Trade-off |
|----------|-------------|-----------|
| Ease of use | Ollama | Lower max throughput |
| Maximum speed | vLLM | More complex setup |
| Enterprise features | TGI | Heavier infrastructure |
| Simplicity (single model) | llamafile | Very limited features |

#### Key Takeaways

- Runtime = software that loads and runs your AI models
- All major runtimes now support OpenAI-compatible APIs
- Choice depends on: ease of use vs. performance vs. features

---

### 3.2 Runtime Comparison {#32-runtime-comparison}

`[CORE]`

#### Overview Table

```
┌─────────────┬─────────────┬──────────────┬───────────────┬───────────────┐
│ Runtime     │ Multi-Model │ Performance  │ Ease of Use   │ Best For      │
├─────────────┼─────────────┼──────────────┼───────────────┼───────────────┤
│ Ollama      │ Yes (swap)  │ Moderate     │ ⭐⭐⭐⭐⭐ Easy   │ Development   │
│ vLLM        │ No (1 each) │ ⭐⭐⭐⭐⭐ Best  │ ⭐⭐ Complex   │ Production    │
│ TGI         │ No (1 each) │ Very Good    │ ⭐⭐⭐ Medium   │ Enterprise    │
│ LM Studio   │ Yes (GUI)   │ Moderate     │ ⭐⭐⭐⭐⭐ Easy   │ Exploration   │
│ llamafile   │ No          │ Low          │ ⭐⭐⭐⭐⭐ Easy   │ Distribution  │
│ llama.cpp   │ No          │ Good         │ ⭐⭐⭐ Medium   │ Embedding     │
└─────────────┴─────────────┴──────────────┴───────────────┴───────────────┘
```

---

#### Ollama — The Easy Choice

**What it is:** A user-friendly CLI tool and server for running local LLMs. Think "Docker for AI models."

**How it works:**
```bash
# Install and run a model in one command
ollama run mistral

# Or start as a server
ollama serve
# Then call via API at localhost:11434
```

**Pros:**
- ⭐ Incredibly easy to set up (one-line install)
- ⭐ Built-in model management (download, update, delete)
- ⭐ OpenAI-compatible API out of the box
- ⭐ Automatic GPU/CPU fallback
- ⭐ Supports multiple models (swaps them in/out of VRAM)

**Cons:**
- ⚠️ Not optimized for high concurrency (~41 tokens/sec under load vs vLLM's ~793)
- ⚠️ No advanced batching (processes one request fully before next)
- ⚠️ Model switching has latency (unload/load takes seconds)

**Performance Numbers:**
```
Ollama on RTX 3090 (single user):
  • Mistral-7B Q4:  ~100-130 tokens/sec
  • Llama2-13B Q4:  ~40-50 tokens/sec
  • Under heavy load: drops to ~41 tokens/sec (no batching)
```

**Best for:** Development, personal use, low-concurrency production

---

#### vLLM — The Performance Champion

**What it is:** A high-performance inference engine from UC Berkeley, optimized for throughput.

**How it works:**
```bash
# Start vLLM server
python -m vllm.entrypoints.openai.api_server \
  --model mistralai/Mistral-7B-v0.1 \
  --port 8000
```

**Pros:**
- ⭐ Extremely fast: ~793 tokens/sec under load (vs Ollama's ~41)
- ⭐ Continuous batching: efficiently handles many concurrent requests
- ⭐ PagedAttention: optimizes memory usage
- ⭐ Scales almost linearly with more requests
- ⭐ OpenAI-compatible API

**Cons:**
- ⚠️ One model per process (need multiple processes for multiple models)
- ⚠️ More complex setup than Ollama
- ⚠️ GPU-only (no CPU fallback)
- ⚠️ Python-based (adds some overhead to embed)

**Performance Numbers:**
```
vLLM on RTX 3090:
  • Single request:   Similar to Ollama
  • 10 concurrent:    ~793 tokens/sec total (vs Ollama's ~41)
  • Scales to 100s of concurrent requests efficiently
```

**Best for:** High-concurrency production, batch processing, when speed matters most

---

#### HuggingFace TGI (Text Generation Inference)

**What it is:** HuggingFace's production-grade inference server, used in their cloud offerings.

**How it works:**
```bash
# Run via Docker
docker run --gpus all -p 8080:80 \
  ghcr.io/huggingface/text-generation-inference \
  --model-id mistralai/Mistral-7B-v0.1
```

**Pros:**
- ⭐ Production-tested at scale (powers HuggingFace Inference Endpoints)
- ⭐ Continuous batching like vLLM
- ⭐ Built-in metrics (Prometheus) and tracing
- ⭐ Supports many quantization formats (GPTQ, AWQ, bitsandbytes)
- ⭐ OpenAI-compatible API

**Cons:**
- ⚠️ One model per container
- ⚠️ Requires Docker (adds complexity on Windows)
- ⚠️ Heavier setup than Ollama

**Best for:** Enterprise production, when you need built-in observability

---

#### Other Options (Brief)

**LM Studio:**
- GUI application for exploring models
- Has a server mode with OpenAI API
- Great for testing, not ideal for production automation
- Closed-source

**llamafile:**
- Single executable per model (bundles model + runtime)
- Just download and run—no installation
- Limited features, single-threaded
- Best for distributing a pre-packaged model to end users

**llama.cpp (via Python bindings):**
- The engine under Ollama and many others
- Can embed directly in your code
- More control, more complexity
- Good for custom integrations

---

### 3.3 Recommended Runtime Strategy {#33-recommended-runtime-strategy}

`[CORE]`

```
═══════════════════════════════════════════════════════════════
                    DECISION POINT: Runtime Strategy
═══════════════════════════════════════════════════════════════

RECOMMENDED APPROACH: Ollama Primary + vLLM for Heavy Loads

Phase 1 (Development & MVP):
  └── Use Ollama exclusively
      • Fastest to set up
      • Easy model management
      • Good enough for single-user
      
Phase 2 (Multi-user or batch processing):
  └── Add vLLM for specific high-throughput needs
      • Route "fast lane" traffic to Ollama
      • Route "batch" or "heavy" jobs to vLLM

═══════════════════════════════════════════════════════════════
```

#### Integration Pattern

```python
# Conceptual routing logic
def route_request(request):
    if request.type == "interactive_chat":
        # Quick responses, single user
        return call_ollama(request)
    elif request.type == "batch_process":
        # Processing many documents
        return call_vllm(request)
    elif request.type == "code_generation":
        # Code needs fast iteration
        return call_ollama(request, model="codellama")
    else:
        # Fallback
        return call_ollama(request)
```

#### Key Takeaways

- **Start with Ollama:** Easy setup, good enough for development and single-user
- **Add vLLM later if needed:** When you need high concurrency or batch processing
- **Both expose OpenAI-compatible APIs:** Your code works with either
- **Model management:** Ollama handles it; vLLM requires manual setup per model

---

## 4.0 Model Selection & Roles {#4-model-selection-and-roles}

**Prerequisites:** Sections 2.0, 3.0  
**Related to:** Section 5.0 (Image Generation)  
**Implements:** Which specific models to use  
**Read time:** ~20 minutes

**This section recommends specific models for different tasks and explains how to manage multiple models on limited VRAM.**

---

### 4.1 Specialized Models for Different Tasks {#41-specialized-models-for-different-tasks}

`[CORE]`

#### Why Not One Model for Everything?

**Specialized models outperform generalists at specific tasks while using less resources.**

```
Analogy: Hiring Staff

Option A: One expensive expert who does everything "pretty well"
  └── 70B generalist model (17GB VRAM, slow)

Option B: Team of specialists, each excellent at their job
  └── 7B code model (4GB) + 7B chat model (4GB) + 7B creative (4GB)
  └── Total: 12GB, all running simultaneously, each faster at their specialty

For our project: Option B is better
```

#### Role Categories

| Role | What It Does | Characteristics Needed |
|------|--------------|------------------------|
| **Orchestrator** | General reasoning, routing decisions, conversation | Fast, good instruction-following |
| **Code Assistant** | Writing and explaining code | Trained on code, good at syntax |
| **Creative Writer** | Long-form content, stories, marketing | Larger context, creative outputs |
| **Utility/Fast** | Simple tasks: classification, extraction, yes/no | Tiny, extremely fast |

---

### 4.2 Model Recommendations by Role {#42-model-recommendations-by-role}

`[CORE]`

#### Orchestrator / General Purpose

**Primary Pick: Mistral-7B**

```
┌────────────────────────────────────────────────────────────┐
│  MISTRAL-7B (Q4_K_M)                                       │
├────────────────────────────────────────────────────────────┤
│  Parameters:  7.3B                                         │
│  VRAM:        ~4.1 GB                                      │
│  Speed:       ~130 tokens/sec on 3090                      │
│  Context:     4K tokens (limited) or 8K with some variants│
├────────────────────────────────────────────────────────────┤
│  Strengths:                                                │
│    • Outperforms Llama2-13B despite being smaller         │
│    • Excellent instruction following                       │
│    • Very fast inference                                   │
│  Weaknesses:                                               │
│    • 4K context can be limiting for long conversations    │
└────────────────────────────────────────────────────────────┘
```

**Alternative: Llama2-13B** (when you need more capability or longer context)
- ~9 GB VRAM, ~40-50 tokens/sec
- 8K context window
- Better for complex reasoning

---

#### Code Generation

**Primary Pick: CodeLlama-7B**

```
┌────────────────────────────────────────────────────────────┐
│  CODELLAMA-7B (Q4_K_M)                                     │
├────────────────────────────────────────────────────────────┤
│  Parameters:  7B                                           │
│  VRAM:        ~3.8 GB                                      │
│  Speed:       ~100 tokens/sec on 3090                      │
│  Context:     16K tokens                                   │
├────────────────────────────────────────────────────────────┤
│  Strengths:                                                │
│    • Fine-tuned specifically for code                      │
│    • Supports Python, JS, C++, and more                   │
│    • Large context for reading whole files                │
│  Weaknesses:                                               │
│    • Less capable at general conversation                  │
└────────────────────────────────────────────────────────────┘
```

**Alternatives:**
- **StarCoder-7B:** Open-source, 16K context
- **WizardCoder-15B:** Higher quality (~8-9GB), better for complex tasks

---

#### Creative / Long-Form Writing

**Primary Pick: Llama2-13B or Mistral-7B**

For most creative tasks, the orchestrator model works fine. For serious long-form writing:

**Consider: Llama2-70B (4-bit)** — Best quality, but uses ~17-18GB
- Only load when specifically needed for creative work
- Unload other models first
- ~15 tokens/sec (slower but higher quality)

---

#### Utility / Fast Tasks

**Primary Pick: Phi-4 Mini (3.8B) or Gemma-3-4B**

```
┌────────────────────────────────────────────────────────────┐
│  SMALL UTILITY MODELS                                      │
├────────────────────────────────────────────────────────────┤
│  Phi-4 Mini (3.8B Q4):    ~2.5 GB, ~60 tokens/sec         │
│  Gemma-3 4B (4-bit):      ~2.6 GB, ~200+ tokens/sec       │
│  Qwen2.5-3B (Q4):         ~2-3 GB, ~40 tokens/sec         │
├────────────────────────────────────────────────────────────┤
│  Use for:                                                  │
│    • Classification ("is this spam?")                      │
│    • Extraction ("find the date in this text")            │
│    • Simple Q&A                                            │
│    • Routing decisions                                     │
└────────────────────────────────────────────────────────────┘
```

---

#### Recommended Starting Configuration

```
═══════════════════════════════════════════════════════════════
                    DECISION POINT: Initial Model Setup
═══════════════════════════════════════════════════════════════

RECOMMENDED: "Always Hot" + "On-Demand" Strategy

Always Loaded ("Hot"):
  ├── Mistral-7B (4GB)      → General orchestrator, fast chat
  └── CodeLlama-7B (4GB)    → Code assistance
  Total: ~8 GB (leaves 14GB free)

Load On-Demand:
  ├── Llama2-13B (9GB)      → Complex reasoning when needed
  ├── Llama2-70B (17GB)     → Best quality (swap out others first)
  └── SDXL (7-10GB)         → Image generation

Rationale:
  • Two 7B models handle 90% of tasks
  • Fast switching between chat and code
  • Load larger models only for complex work
  • Preserves VRAM for image generation

═══════════════════════════════════════════════════════════════
```

---

### 4.3 GPU Memory Management {#43-gpu-memory-management}

`[CORE]`

#### The Loading Problem

**Models must be in VRAM to run fast.** Loading a model takes time:
- 7B model: ~3-5 seconds
- 13B model: ~5-10 seconds  
- 70B model: ~15-30 seconds

This creates a user experience challenge: if users request a model that isn't loaded, they wait.

#### Strategies

**1. Keep "Hot" Models Resident**
```
Always keep your most-used models in VRAM:
  • Set Ollama: OLLAMA_MAX_LOADED_MODELS=2
  • These stay loaded even when idle
  • Instant response for common tasks
```

**2. On-Demand Loading with Feedback**
```
When user needs a different model:
  • Show loading indicator: "Loading creative writing model..."
  • Expected wait: 5-15 seconds
  • Consider preloading if you can predict need
```

**3. Never Use CPU Fallback for Primary Tasks**
```
CPU inference is ~6x slower:
  • GPU: 100 tokens/sec
  • CPU: ~15 tokens/sec
  
Only use CPU for:
  • Truly background tasks
  • When GPU is fully occupied with priority work
  • Emergency fallback (better slow than nothing)
```

#### KV Cache: The Hidden Memory User

**Context uses extra VRAM beyond model weights.**

```
VRAM breakdown for a 7B model with long conversation:

  Model weights:        ~4 GB
  KV cache (context):   +2-4 GB for 4K tokens
  ─────────────────────────────
  Total:                ~6-8 GB actual usage

⚠️ Long conversations can DOUBLE your VRAM usage!
```

💡 **Tip:** For multi-model setups, keep conversations shorter or implement context summarization.

---

### 4.4 Scheduling & Contention {#44-scheduling-and-contention}

`[CORE]`

#### The Core Problem

**Only one heavy task can use the GPU efficiently at a time.** Running two things simultaneously doesn't make each run at half speed—it makes both run poorly or crash.

#### Priority Rules

```
Priority Queue (highest to lowest):

  1. Interactive Chat    → User is waiting, <100ms latency matters
  2. Code Generation     → User is waiting, but can tolerate 1-2sec
  3. Image Generation    → User expects to wait (5-30 seconds)
  4. Background Tasks    → Batch processing, can run overnight
```

#### Practical Scheduling Pattern

```python
# Pseudocode for GPU scheduling
class GPUScheduler:
    def handle_request(self, request):
        if request.priority == "interactive":
            # Pause any batch jobs
            self.pause_background_tasks()
            # Run immediately
            return self.run_now(request)
            
        elif request.priority == "image":
            if self.vram_available() < 10_GB:
                # Not enough VRAM, queue it
                return self.queue(request, 
                    message="Waiting for VRAM...")
            else:
                return self.run_now(request)
                
        else:  # background
            # Only run if GPU is idle
            if self.gpu_is_idle():
                return self.run_now(request)
            else:
                return self.queue(request)
```

#### Key Takeaways

- Keep 2 small "hot" models loaded for instant response
- Load larger models on-demand with user feedback
- Never rely on CPU fallback for user-facing tasks
- Context (KV cache) can double VRAM usage
- Implement priority queuing: interactive > image > background

---

## 5.0 Image Generation {#5-image-generation}

**Prerequisites:** Section 4.0 (Model Selection)  
**Related to:** Section 4.4 (Scheduling)  
**Implements:** Image generation capability  
**Read time:** ~10 minutes

**This section covers Stable Diffusion integration for generating images from text prompts.**

---

### 5.1 SD vs SDXL Overview {#51-sd-vs-sdxl-overview}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **Stable Diffusion (SD)** | Open-source AI that generates images from text descriptions | Our image generation capability |
| **SD 1.5/2.1** | Older versions, smaller, faster | Good for quick generations |
| **SDXL** | Newest version, higher quality, larger | Best quality but heavier |
| **U-Net** | The neural network architecture SD uses | Understanding helps with VRAM planning |
| **ComfyUI** | A visual workflow tool for Stable Diffusion | Efficient way to run SD |
| **Automatic1111** | Popular SD web interface | Alternative to ComfyUI, less efficient |

#### Quick Comparison

```
┌────────────────┬──────────────────┬──────────────────────────┐
│                │    SD 1.5/2.1    │         SDXL             │
├────────────────┼──────────────────┼──────────────────────────┤
│ Output Size    │ 512×512          │ 1024×1024                │
│ VRAM Needed    │ 6-8 GB           │ 7-16 GB (varies)         │
│ Speed (3090)   │ ~0.2-0.3s/image  │ ~4-10s/image             │
│ Quality        │ Good             │ Excellent                │
│ Best For       │ Quick previews   │ Final outputs            │
└────────────────┴──────────────────┴──────────────────────────┘
```

---

### 5.2 VRAM Requirements & Performance {#52-vram-requirements-and-performance}

`[CORE]`

#### Detailed VRAM Breakdown

```
SD 1.5 (512×512, 25 steps):
  • VRAM:    ~6-8 GB
  • Speed:   ~0.2-0.3 seconds per image
  • Rate:    ~4-5 images/second possible on 3090

SDXL Base (1024×1024, 30 steps):
  • VRAM:    ~6-14 GB (depends on optimizations)
  • Speed:   ~4-10 seconds per image
  • With optimizations (OneDiff + Tiny VAE): 
    - VRAM drops to ~6.9 GB
    - Speed improves to ~4 seconds

SDXL with Refiner:
  • VRAM:    ~7-16 GB
  • Speed:   ~6-12 seconds per image
  • Higher quality details
```

⚡ **Key Finding:** With optimizations, SDXL can run alongside a 7B LLM (4GB + 7GB = 11GB total).

---

### 5.3 Integrating with LLM Workloads {#53-integrating-with-llm-workloads}

`[CORE]`

#### The Contention Problem

**Image generation and LLM inference compete for the same GPU.**

```
Scenario: User chatting while generating an image

WRONG approach (simultaneous):
  ┌─────────────────────────────────────────┐
  │  Mistral-7B (4GB) + SDXL (10GB) = 14GB │
  │  Both running = GPU thrashing          │
  │  Result: Both slow, possible crash     │
  └─────────────────────────────────────────┘

RIGHT approach (serialized + priority):
  ┌─────────────────────────────────────────┐
  │  1. Chat request arrives               │
  │  2. Pause/queue image generation       │
  │  3. Process chat (fast, <1 sec)        │
  │  4. Resume image generation            │
  └─────────────────────────────────────────┘
```

#### Recommended Strategy

```
═══════════════════════════════════════════════════════════════
                    DECISION POINT: Image Generation
═══════════════════════════════════════════════════════════════

RECOMMENDED: ComfyUI + Sequential Processing

Setup:
  • Run ComfyUI as a separate process
  • Call it via HTTP API when images needed
  • Keep LLM models hot; unload for big image jobs

Priority:
  • Chat/code requests ALWAYS preempt image generation
  • Queue images, show progress to user
  • Run image generation when GPU is otherwise idle

VRAM Management:
  • For quick SD 1.5: Can run alongside 7B model
  • For quality SDXL: Unload secondary LLM, keep orchestrator

═══════════════════════════════════════════════════════════════
```

#### Key Takeaways

- SD 1.5: Fast, lower VRAM, good for previews
- SDXL: Higher quality, needs more VRAM and time
- Never run heavy image generation and LLM simultaneously
- Use ComfyUI for efficiency (better than Automatic1111)
- Implement job queuing with priority for interactive requests

---
---

# PART III: DATA ARCHITECTURE

---

## 6.0 Local-First Data Fundamentals {#6-local-first-data-fundamentals}

**Prerequisites:** Section 1.1 (Local-First Concept)  
**Related to:** Sections 7, 8, 9  
**Implements:** Data storage and sync strategy  
**Read time:** ~20 minutes

**This section explains the core challenge of local-first apps—keeping data consistent across devices—and introduces the technology that solves it.**

---

### 6.1 What "Local-First" Really Means {#61-what-local-first-really-means}

`[CORE]`

#### The Promise

**Local-first software keeps your data on your devices, with optional cloud sync.** This gives you:

- **Ownership:** Your files are literally on your computer
- **Speed:** No network round-trip for every action
- **Offline:** Works without internet
- **Privacy:** Data doesn't have to touch company servers

#### The Challenge

**What happens when you edit the same document on two devices while offline?**

```
Timeline of a Conflict:

Monday 9am:  Both laptop and tablet sync → same document state
Monday 10am: You go offline on both devices
Monday 11am: On laptop, you add paragraph A
Monday 11am: On tablet, you add paragraph B
Monday 2pm:  Both come online again

QUESTION: What should the document look like now?

  Option 1: Last-write-wins → One person's work is LOST ❌
  Option 2: Keep both versions, ask user to choose → Annoying ❌  
  Option 3: Automatically merge both changes → ✓ This is what CRDTs do
```

---

### 6.2 The Problem: Concurrent Editing {#62-the-problem-concurrent-editing}

`[CORE]`

#### Why This is Hard

**Traditional databases assume one "source of truth."** When you save a document, you overwrite what was there. If two people edit simultaneously, one overwrites the other.

```
Traditional Approach (Google Docs style):
  
  [Device A]                    [Server]                    [Device B]
      │                            │                            │
      │──── Edit: "Hello" ────────>│                            │
      │                            │<──── Edit: "World" ────────│
      │                            │                            │
      │                   Server decides order                  │
      │                   "Hello" then "World"                  │
      │                   OR "World" then "Hello"               │
      │                            │                            │
      
  Problem: Server is required. No offline support.
```

```
Local-First Challenge:
  
  [Device A - Offline]                              [Device B - Offline]
      │                                                  │
      │ Edit: "Hello"                                    │ Edit: "World"
      │     (no server to ask!)                          │     (no server!)
      │                                                  │
      ▼                                                  ▼
  Local state: "Hello"                          Local state: "World"
  
  Later, when both reconnect... now what?
```

---

### 6.3 Solution: CRDTs Explained {#63-solution-crdts-explained}

`[CORE]`

#### Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **CRDT** | Conflict-free Replicated Data Type—a special data structure that can merge automatically | The technology that makes local-first sync possible |
| **Merge** | Combining two versions into one | CRDTs guarantee merges always produce the same result |
| **Eventual Consistency** | All devices eventually have the same data, even if they're temporarily different | What CRDTs guarantee |
| **Operation-based (Op-based)** | A CRDT style that syncs by sharing operations ("insert 'A' at position 3") | One approach |
| **State-based** | A CRDT style that syncs by sharing entire state snapshots | Another approach |

#### The Magic of CRDTs

**CRDTs are data structures designed so that merging always works and always produces the same result.**

```
═══════════════════════════════════════════════════════════════
                    CORE CONCEPT: How CRDTs Work
═══════════════════════════════════════════════════════════════

Key insight: Instead of storing "the text is Hello", store 
"character H was inserted by device A at time T1, character e 
was inserted by device A at time T2..."

This extra information lets us ALWAYS merge correctly:

Device A's operations:              Device B's operations:
  1. Insert "H" at start             1. Insert "W" at start
  2. Insert "e" after "H"            2. Insert "o" after "W"
  3. Insert "l" after "e"            3. Insert "r" after "o"
  ...                                ...
  
When merging:
  • Each operation has a unique ID
  • We can replay ALL operations in a deterministic order
  • Both devices end up with: "HelloWorld" (or "WorldHello")
  • The SAME result regardless of which device syncs first!

═══════════════════════════════════════════════════════════════
```

#### Types of CRDT Data Structures

```
For Text Documents:
  • Tracks each character insertion/deletion
  • Handles concurrent typing in different places
  • Libraries: Yjs (Y.Text), Automerge (Text type)

For JSON-like Objects:
  • Tracks changes to keys and values
  • Handles concurrent edits to different fields
  • Libraries: Yjs (Y.Map), Automerge (objects)

For Lists/Arrays:
  • Tracks insertions, deletions, moves
  • Handles concurrent list modifications
  • Libraries: Yjs (Y.Array), Loro (MovableList)

For Rich Text:
  • Tracks formatting (bold, italic, etc.)
  • Handles concurrent formatting changes
  • Libraries: Yjs + editor bindings
```

#### What CRDTs DON'T Solve

⚠️ **CRDTs merge automatically, but "automatic" doesn't mean "smart."**

```
Example: Two users both edit the SAME sentence:

Original:        "The quick brown fox"
User A changes:  "The fast brown fox"      (quick → fast)
User B changes:  "The quick red fox"       (brown → red)

CRDT merge:      "The fast red fox"        (both changes applied)

Is this right? Maybe! But maybe User A wanted to keep "brown" and 
User B wanted to keep "quick". The CRDT doesn't understand INTENT,
it just merges the characters.
```

💡 **Key insight:** CRDTs prevent data loss and conflicts, but users may still need to review merged results for semantic correctness.

#### Key Takeaways

- CRDTs are special data structures that merge automatically
- They track operations (not just state) to enable consistent merging
- Different CRDT types for different data: text, objects, lists
- CRDTs merge mechanically—they don't understand meaning
- This is the foundation for local-first sync

---

## 7.0 CRDT Libraries Comparison {#7-crdt-libraries-comparison}

**Prerequisites:** Section 6.3 (CRDTs Explained)  
**Related to:** Section 8 (Database Integration)  
**Implements:** Choosing a CRDT library  
**Read time:** ~20 minutes

**This section compares the main CRDT libraries and recommends which to use.**

---

### 7.1 Yjs Deep Dive {#71-yjs-deep-dive}

`[CORE]`

#### What is Yjs?

**Yjs is the most popular CRDT library for JavaScript/TypeScript applications.** It's battle-tested, fast, and has excellent editor integrations.

#### Key Features

```
┌─────────────────────────────────────────────────────────────┐
│                          Yjs                                 │
├─────────────────────────────────────────────────────────────┤
│ Language:     JavaScript/TypeScript                         │
│ Also:         Rust port (Yrs), Python, Swift, Ruby          │
│ Data Types:   Y.Text, Y.Map, Y.Array, Y.XmlFragment         │
│ Performance:  Excellent (~260K inserts: 1s, 10MB memory)    │
│ History:      No full history (snapshots optional)          │
│ Sync:         WebSocket, WebRTC, custom providers           │
│ Editors:      ProseMirror, TipTap, Monaco, Quill, more      │
└─────────────────────────────────────────────────────────────┘
```

#### How Yjs Works

```javascript
// Basic Yjs usage
import * as Y from 'yjs'

// Create a document
const doc = new Y.Doc()

// Get a shared text type
const text = doc.getText('content')

// Make changes
text.insert(0, 'Hello World')

// Observe changes (for updating UI)
text.observe(event => {
  console.log('Text changed:', text.toString())
})

// Export for sync/storage
const update = Y.encodeStateAsUpdate(doc)  // Binary format
```

#### Pros and Cons

**Pros:**
- ⭐ Best performance and memory efficiency
- ⭐ Rich editor integrations (drop-in for popular editors)
- ⭐ Large community, many examples
- ⭐ Multiple sync options (WebSocket, WebRTC, file-based)
- ⭐ Cross-platform via ports (Yrs for Rust/Tauri)

**Cons:**
- ⚠️ No built-in full history (only current state)
- ⚠️ Learning curve for understanding shared types
- ⚠️ Need to manually handle persistence

---

### 7.2 Automerge Deep Dive {#72-automerge-deep-dive}

#### What is Automerge?

**Automerge is an academically rigorous CRDT library with full history tracking.** Version 2 is written in Rust with JavaScript bindings.

#### Key Features

```
┌─────────────────────────────────────────────────────────────┐
│                       Automerge                              │
├─────────────────────────────────────────────────────────────┤
│ Language:     Rust core, JS/WASM bindings                   │
│ Data Types:   JSON-like objects, lists, text, counters      │
│ Performance:  Slower (~260K inserts: 1.8s, 44MB memory)     │
│ History:      Full operation history (like Git)             │
│ Sync:         Custom sync protocol                          │
│ Best For:     When you need complete version history        │
└─────────────────────────────────────────────────────────────┘
```

#### Pros and Cons

**Pros:**
- ⭐ Full version history—can reconstruct any past state
- ⭐ Cleaner API—works like normal JS objects
- ⭐ Academic backing—provably correct
- ⭐ Good for debugging (can replay history)

**Cons:**
- ⚠️ Higher memory usage (~4x more than Yjs)
- ⚠️ Slower for large documents
- ⚠️ Larger storage requirements (keeps all operations)
- ⚠️ Fewer editor integrations

---

### 7.3 Loro and Emerging Options {#73-loro-and-emerging-options}

#### What is Loro?

**Loro is a new CRDT library aiming to combine the best of Yjs and Automerge.** It offers high performance AND full history.

#### Key Features

```
┌─────────────────────────────────────────────────────────────┐
│                          Loro                                │
├─────────────────────────────────────────────────────────────┤
│ Language:     Rust core, JS/WASM bindings                   │
│ Data Types:   MovableList, Map, Tree, Text, Counter         │
│ Performance:  Very high (designed to beat both Yjs/Automerge)│
│ History:      Full version DAG (like Git)                   │
│ Unique:       Movable trees (great for outlines/kanban)     │
│ Maturity:     Newer, less battle-tested                     │
└─────────────────────────────────────────────────────────────┘
```

#### Pros and Cons

**Pros:**
- ⭐ Rust-native (great for Tauri)
- ⭐ Full history like Automerge, speed like Yjs (claimed)
- ⭐ Movable trees perfect for hierarchical data (outlines, kanban)
- ⭐ Time-travel debugging possible

**Cons:**
- ⚠️ Newer, less proven in production
- ⚠️ Smaller community and fewer integrations
- ⚠️ API may still change

---

### 7.4 Recommendation: Which CRDT Library? {#74-recommendation-which-crdt-library}

`[CORE]`

```
═══════════════════════════════════════════════════════════════
                    DECISION POINT: CRDT Library Choice
═══════════════════════════════════════════════════════════════

FOR ELECTRON + TypeScript:
  └── Use Yjs
      • Best performance and editor integrations
      • Largest community, most resources
      • Add snapshots for version history if needed

FOR TAURI + Rust:
  └── Consider Loro or Yrs (Yjs Rust port)
      • Loro: If you need movable trees and version history
      • Yrs: If you want Yjs compatibility across platforms

RECOMMENDATION FOR THIS PROJECT (starting):
  └── Start with Yjs
      • Proven, fast, well-documented
      • Works in both Electron and Tauri (via Yrs)
      • Easiest path to editor integration
      • Migrate to Loro later if needed for hierarchical data

═══════════════════════════════════════════════════════════════
```

#### Comparison Summary Table

| Aspect | Yjs | Automerge | Loro |
|--------|-----|-----------|------|
| Performance | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ (claimed) |
| Memory | ⭐⭐⭐⭐⭐ (10MB) | ⭐⭐⭐ (44MB) | ⭐⭐⭐⭐ |
| Full History | ❌ (snapshots only) | ✅ | ✅ |
| Editor Integration | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐ |
| Rust Native | Via Yrs | Via WASM | ✅ Native |
| Maturity | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ |
| Movable Trees | ❌ | ❌ | ✅ |

---

## 8.0 Database & Sync Patterns {#8-database-and-sync-patterns}

**Prerequisites:** Sections 6, 7  
**Related to:** Section 9 (Conflict Resolution UX)  
**Implements:** Data storage architecture  
**Read time:** ~15 minutes

**This section explains how to combine CRDT with a local database for querying and persistence.**

---

### 8.1 Local Database Options {#81-local-database-options}

`[CORE]`

#### Why Use a Database with CRDT?

**CRDTs handle sync, but databases handle queries.** You often need both:

```
CRDT alone:
  ✓ Sync across devices
  ✓ Merge concurrent edits
  ✗ "Find all documents containing 'budget'" → Slow (must scan all)
  ✗ "Sort documents by date" → Not built-in
  ✗ Complex queries → Difficult

Database alone:
  ✓ Fast queries with indexes
  ✓ Sort, filter, aggregate
  ✗ Sync across devices → Conflicts!
  ✗ Offline merge → Data loss

CRDT + Database:
  ✓ Sync via CRDT
  ✓ Query via database
  ✓ Best of both worlds
```

#### SQLite: The Recommended Choice

```
┌─────────────────────────────────────────────────────────────┐
│                        SQLite                                │
├─────────────────────────────────────────────────────────────┤
│ Type:         Embedded SQL database                         │
│ Storage:      Single file on disk                           │
│ Performance:  Very fast for local operations                │
│ Features:     Full SQL, indexes, full-text search (FTS)     │
│ Size:         Tiny (library is ~1MB)                        │
│ Reliability:  Extremely battle-tested                       │
└─────────────────────────────────────────────────────────────┘
```

**Why SQLite for this project:**
- ⭐ Standard across all platforms (Windows, macOS, Linux)
- ⭐ Works great with Electron AND Tauri
- ⭐ Full-text search for finding documents
- ⭐ ACID guarantees (data integrity)
- ⭐ Single file = easy backup

---

### 8.2 Combining CRDT and Database {#82-combining-crdt-and-database}

`[CORE]`

#### Architecture Pattern

```
┌────────────────────────────────────────────────────────────┐
│                    HYBRID ARCHITECTURE                      │
├────────────────────────────────────────────────────────────┤
│                                                             │
│    User Edit                                                │
│        │                                                    │
│        ▼                                                    │
│  ┌───────────┐                                              │
│  │   CRDT    │  ◄─── Handles: Sync, Merge, Collaboration   │
│  │  (Yjs)    │                                              │
│  └─────┬─────┘                                              │
│        │                                                    │
│        │ On every CRDT change:                              │
│        │ • Update SQLite                                    │
│        │ • Update indexes                                   │
│        ▼                                                    │
│  ┌───────────┐                                              │
│  │  SQLite   │  ◄─── Handles: Queries, Search, Indexes     │
│  │           │                                              │
│  └─────┬─────┘                                              │
│        │                                                    │
│        │ Query results                                      │
│        ▼                                                    │
│    UI Display                                               │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

#### What Goes Where?

```
In CRDT (Yjs):
  • Document content (text, rich text)
  • Board/canvas positions
  • List ordering
  • Everything that needs to sync and merge

In SQLite:
  • Document metadata (title, dates, tags)
  • Search indexes
  • User preferences
  • Derived/computed data
  • Anything that needs fast querying

Example Schema:
  documents:
    - id (primary key)
    - title (indexed)
    - created_at
    - updated_at
    - tags (indexed)
    - crdt_id (reference to CRDT document)
    - content_preview (first 200 chars for search)
```

#### Sync Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    COMPLETE SYNC FLOW                        │
└─────────────────────────────────────────────────────────────┘

1. USER MAKES EDIT (Device A)
   │
   ├──► CRDT update applied locally
   │
   ├──► SQLite updated with new content/metadata
   │
   └──► CRDT update sent to sync server (or peer)

2. SYNC UPDATE RECEIVED (Device B)
   │
   ├──► CRDT merges incoming update
   │
   ├──► SQLite updated to reflect merged state
   │
   └──► UI refreshes to show changes

3. CONFLICT HANDLED AUTOMATICALLY
   │
   └──► CRDT merge is deterministic
       │
       └──► Same SQLite state on all devices (eventually)
```

---

### 8.3 Sync Topologies {#83-sync-topologies}

#### Options for Syncing Data

```
┌─────────────────────────────────────────────────────────────┐
│                    SYNC TOPOLOGY OPTIONS                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  OPTION A: Peer-to-Peer                                      │
│  ┌─────┐         ┌─────┐                                     │
│  │Dev A│◄───────►│Dev B│   Direct device-to-device          │
│  └─────┘         └─────┘                                     │
│     │               │                                        │
│     └───────────────┘                                        │
│  Pros: No server needed, private                             │
│  Cons: Both devices must be online simultaneously            │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  OPTION B: Central Server                                    │
│  ┌─────┐       ┌──────┐       ┌─────┐                       │
│  │Dev A│──────►│Server│◄──────│Dev B│                       │
│  └─────┘       └──────┘       └─────┘                       │
│  Pros: Works when only one device online                    │
│  Cons: Requires running/paying for server                   │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  OPTION C: File Sync (OneDrive/Dropbox)                     │
│  ┌─────┐       ┌────────┐       ┌─────┐                     │
│  │Dev A│──────►│OneDrive│◄──────│Dev B│                     │
│  └─────┘       └────────┘       └─────┘                     │
│  Pros: No custom server, leverages existing sync             │
│  Cons: File-level conflicts, coarse merging                  │
│        (need CRDT on top to handle conflicts)               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Recommendation

```
═══════════════════════════════════════════════════════════════
                    DECISION POINT: Sync Topology
═══════════════════════════════════════════════════════════════

PHASE 1 (MVP): File Sync + CRDT
  • Store CRDT updates as files in a synced folder
  • Let OneDrive/Dropbox/iCloud handle file sync
  • CRDT handles merge when files conflict
  • Zero server infrastructure needed

PHASE 2 (Multi-user): Central Sync Server
  • Build or use WebSocket sync server
  • Real-time collaboration possible
  • More complex but better UX

Libraries that help:
  • y-indexeddb: Local persistence for Yjs
  • y-websocket: WebSocket sync for Yjs
  • ElectricSQL: Postgres ↔ SQLite sync
  • Replicache: Client-server sync framework

═══════════════════════════════════════════════════════════════
```

---

## 9.0 Conflict Resolution UX {#9-conflict-resolution-ux}

**Prerequisites:** Sections 6, 7, 8  
**Related to:** User interface design  
**Implements:** How users experience data sync  
**Read time:** ~10 minutes

**This section covers how to show sync status and handle conflicts in the UI.**

---

### 9.1 User-Facing Conflict Patterns {#91-user-facing-conflict-patterns}

`[CORE]`

#### The Good News

**Most of the time, users shouldn't see conflicts at all.** CRDTs merge automatically, and if users edit different parts of a document, everything "just works."

#### When to Show Something

```
┌─────────────────────────────────────────────────────────────┐
│              WHEN TO SHOW SYNC FEEDBACK                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ALWAYS SHOW:                                                │
│    • Sync status indicator (synced ✓, syncing ↻, offline ⚡)│
│    • "X minutes ago" last sync time                         │
│                                                              │
│  SHOW ON EVENT:                                              │
│    • "Document updated by another device" notification      │
│    • Highlight recently changed sections (briefly)          │
│                                                              │
│  SHOW ON POTENTIAL ISSUE:                                    │
│    • "This section was edited while you were offline.       │
│       Review the changes?" (when same paragraph edited)     │
│                                                              │
│  DON'T BOTHER USER WITH:                                     │
│    • Every automatic merge (too noisy)                      │
│    • Technical details ("CRDT vector clock updated")        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Simple Sync Status UI

```
┌──────────────────────────────────────────────────────────┐
│ My Document.md                    ✓ Synced 2 min ago    │
├──────────────────────────────────────────────────────────┤

OR when syncing:

┌──────────────────────────────────────────────────────────┐
│ My Document.md                    ↻ Syncing...          │
├──────────────────────────────────────────────────────────┤

OR when offline:

┌──────────────────────────────────────────────────────────┐
│ My Document.md                    ⚡ Working offline     │
├──────────────────────────────────────────────────────────┤
```

---

### 9.2 Version History UI {#92-version-history-ui}

`[CORE]`

#### Why Provide Version History

Even with automatic merging, users want:
- **Safety net:** "I accidentally deleted something, can I get it back?"
- **Audit trail:** "What changed since yesterday?"
- **Comparison:** "What's different from the old version?"

#### Implementation Approach

```
┌─────────────────────────────────────────────────────────────┐
│                 VERSION HISTORY PANEL                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ◄ Document History                                          │
│                                                              │
│  TODAY                                                       │
│  ├── 3:45 PM - You edited (current)                        │
│  ├── 2:30 PM - Synced from MacBook                         │
│  └── 10:15 AM - You edited                                  │
│                                                              │
│  YESTERDAY                                                   │
│  ├── 8:00 PM - You edited                                   │
│  └── 2:00 PM - Created                                      │
│                                                              │
│  ─────────────────────────────────────────                  │
│  [Preview Selected] [Restore to This Version]               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Technical Implementation

```
For Yjs (no built-in history):
  • Take periodic snapshots (every N minutes or on significant changes)
  • Store snapshots in SQLite with timestamps
  • To restore: load old snapshot, create new CRDT state
  
For Automerge/Loro (built-in history):
  • History is automatically tracked
  • Can "time travel" to any past state
  • Trade-off: larger storage requirements
```

#### Key Takeaways

- CRDTs handle most conflicts invisibly
- Show sync status but don't over-communicate
- Highlight concurrent edits from other devices
- Provide version history as a safety net
- For Yjs: implement snapshots manually; for Automerge/Loro: built-in

---
---

# PART IV: PLUGIN & EXTENSION SYSTEM

---

## 10.0 Plugin Architecture Fundamentals {#10-plugin-architecture-fundamentals}

**Prerequisites:** Section 1.2 (Project Overview)  
**Related to:** Sections 11, 12  
**Implements:** Extensibility strategy  
**Read time:** ~15 minutes

**This section explains why plugins matter and what we can learn from existing plugin systems.**

---

### 10.1 Why Plugins Matter {#101-why-plugins-matter}

`[CORE]`

#### The Power of Extensibility

**Plugins let your users (and you) add features without changing the core application.**

```
Without Plugins:
  • Every feature request requires core development
  • One-size-fits-all: everyone gets everything or nothing
  • Slow iteration: changes go through your release cycle
  • Limited: can only do what YOU thought of

With Plugins:
  • Users can add their own integrations
  • Personalization: each user's setup is unique
  • Community innovation: features you never imagined
  • Faster: plugins ship independently of core app
```

#### Examples of Plugin Value

```
Your app with no plugins:
  └── Basic AI chat + documents
  
Your app with plugins:
  ├── Todoist integration (someone's plugin)
  ├── Custom AI model loader (power user)
  ├── Citation manager (academic user)
  ├── Code formatter (developer)
  ├── Voice commands (accessibility)
  └── [Hundreds more possibilities]
```

---

### 10.2 Learning from Existing Systems {#102-learning-from-existing-systems}

`[CORE]`

#### VS Code: The Gold Standard

```
┌─────────────────────────────────────────────────────────────┐
│                VS CODE EXTENSION MODEL                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Runtime:     Separate "Extension Host" process              │
│  Language:    JavaScript/TypeScript                          │
│  Manifest:    package.json with "contributes" section        │
│  Security:    No sandbox—full Node.js access                │
│  Trust:       "Trust this publisher?" prompt                 │
│                                                              │
│  What they got right:                                        │
│    ✓ Rich API for extending UI                              │
│    ✓ Lazy loading (activation events)                       │
│    ✓ Declarative contributions (commands, menus)            │
│    ✓ Huge ecosystem (50,000+ extensions)                    │
│                                                              │
│  What we'd do differently:                                   │
│    • Add sandboxing (they have none)                        │
│    • Require permission declarations                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Figma: Security-First

```
┌─────────────────────────────────────────────────────────────┐
│                   FIGMA PLUGIN MODEL                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Runtime:     Sandboxed JavaScript (no DOM, no XHR)          │
│  UI:          Separate iframe for plugin UI                  │
│  API:         Only Figma document access via figma.*         │
│  Network:     Must whitelist domains in manifest             │
│                                                              │
│  What they got right:                                        │
│    ✓ True sandbox—plugins can't escape                      │
│    ✓ UI separated from logic                                │
│    ✓ Explicit network permissions                           │
│    ✓ User can cancel runaway plugins                        │
│                                                              │
│  What we'd adapt:                                            │
│    • Similar sandbox model                                   │
│    • Manifest-declared network permissions                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Browser Extensions: Permission Model

```
┌─────────────────────────────────────────────────────────────┐
│             BROWSER EXTENSION MODEL (Manifest V3)            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Key Innovation: Explicit permissions                        │
│                                                              │
│  manifest.json:                                              │
│  {                                                           │
│    "permissions": ["storage", "tabs"],                      │
│    "host_permissions": ["https://api.example.com/*"]        │
│  }                                                           │
│                                                              │
│  User sees at install:                                       │
│  ┌──────────────────────────────────────┐                   │
│  │ "MyExtension" wants to:              │                   │
│  │ • Read and change your browsing data │                   │
│  │   on api.example.com                 │                   │
│  │ • Store data locally                 │                   │
│  │                                      │                   │
│  │  [Add Extension]  [Cancel]           │                   │
│  └──────────────────────────────────────┘                   │
│                                                              │
│  What we'd copy:                                             │
│    ✓ Manifest-declared permissions                          │
│    ✓ User consent at install                                │
│    ✓ Clear permission descriptions                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Obsidian: Cautionary Tale

```
┌─────────────────────────────────────────────────────────────┐
│                   OBSIDIAN PLUGIN MODEL                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Runtime:     Main Electron process (no isolation!)          │
│  Access:      Full Node.js—plugins can do ANYTHING          │
│  Trust:       Community ratings + open source review         │
│                                                              │
│  ⚠️ Security Issue:                                         │
│  "Obsidian plugins have all the same permissions you do     │
│  to read/write all the files in your vault"                 │
│                                                              │
│  A malicious plugin could:                                   │
│    • Read any file on your computer                         │
│    • Send data to external servers                          │
│    • Install malware                                         │
│    • Encrypt your files (ransomware)                        │
│                                                              │
│  What NOT to copy:                                           │
│    ✗ No sandboxing                                          │
│    ✗ Full system access                                     │
│    ✗ Trust based only on community review                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Key Takeaways

- VS Code: Rich API, lazy loading, declarative contributions (good); no sandbox (bad)
- Figma: True sandbox, UI separation, explicit permissions (all good)
- Browser: Permission model with user consent (excellent)
- Obsidian: No security (avoid this pattern)

---

## 11.0 Plugin System Design {#11-plugin-system-design}

**Prerequisites:** Section 10  
**Related to:** Section 12 (Security)  
**Implements:** Plugin API and structure  
**Read time:** ~15 minutes

**This section designs our plugin manifest, registration, and API patterns.**

---

### 11.1 Manifest & Registration {#111-manifest-and-registration}

`[CORE]`

#### Plugin Manifest Format

```json
{
  "manifestVersion": 1,
  "id": "com.example.my-plugin",
  "name": "My Awesome Plugin",
  "version": "1.2.3",
  "description": "Does something useful",
  "author": "Your Name",
  "homepage": "https://github.com/you/plugin",
  
  "minAppVersion": "2.0.0",
  "main": "dist/index.js",
  "ui": "dist/ui.html",
  
  "type": ["automation", "ui"],
  
  "permissions": {
    "readData": ["documents", "boards"],
    "writeData": ["documents"],
    "filesystem": false,
    "network": ["https://api.myservice.com"],
    "ai": {
      "models": ["local"],
      "maxTokensPerDay": 10000
    }
  },
  
  "contributes": {
    "commands": [
      {
        "id": "myplugin.doThing",
        "title": "Do the Thing",
        "shortcut": "Ctrl+Shift+T"
      }
    ],
    "menus": [
      {
        "location": "tools",
        "items": [{ "command": "myplugin.doThing" }]
      }
    ]
  }
}
```

#### Key Manifest Sections Explained

| Section | Purpose |
|---------|---------|
| `id` | Unique identifier (reverse domain style) |
| `main` | Entry point JavaScript file |
| `ui` | Optional HTML file for plugin UI panel |
| `permissions` | What the plugin is allowed to access |
| `contributes` | What UI elements the plugin adds |

---

### 11.2 Plugin Types & Categories {#112-plugin-types-and-categories}

#### Three Main Categories

```
┌─────────────────────────────────────────────────────────────┐
│                      PLUGIN TYPES                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. AUTOMATION PLUGINS                                       │
│     • Background tasks and macros                           │
│     • Triggered by events or commands                       │
│     • May not have UI                                        │
│     Example: "Auto-backup to Dropbox"                       │
│                                                              │
│  2. UI PLUGINS                                               │
│     • Add panels, views, or widgets                         │
│     • Render custom interfaces                               │
│     • Interact with user directly                           │
│     Example: "Kanban board view"                            │
│                                                              │
│  3. AI TOOL PLUGINS                                          │
│     • Add new AI capabilities                               │
│     • May integrate external models or APIs                 │
│     • Often combine UI + automation                         │
│     Example: "AI image generator", "Translation tool"       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### 11.3 API Design Patterns {#113-api-design-patterns}

`[CORE]`

#### Registration API Example

```javascript
// Plugin code (index.js)
export function activate(api) {
  // Register a command
  api.registerCommand("myplugin.sayHello", {
    title: "Say Hello",
    handler: async () => {
      api.showNotification("Hello from my plugin!");
    }
  });
  
  // Register a view
  api.registerView("myplugin.dashboard", {
    title: "My Dashboard",
    location: "sidebar",
    render: (container) => {
      container.innerHTML = "<h1>Dashboard</h1>";
    }
  });
  
  // Subscribe to events
  api.onDocumentSaved((doc) => {
    console.log("Document saved:", doc.id);
  });
}

export function deactivate() {
  // Cleanup when plugin is disabled
}
```

#### Workspace Data API

```javascript
// Reading data
const docs = await api.workspace.query({
  type: "document",
  where: { tags: { contains: "important" } },
  limit: 10
});

// Writing data
await api.workspace.update("document", docId, {
  title: "New Title"
});

// Subscribing to changes
api.workspace.onDidChange((change) => {
  if (change.type === "document") {
    // Handle document change
  }
});
```

#### Key Design Principles

📌 **Explicit Registration:** Plugins declare what they contribute via manifest AND register at runtime

📌 **Namespaced:** All plugin commands/views prefixed with plugin ID (`myplugin.command`)

📌 **Promise-based:** All async operations return Promises

📌 **Observable:** Plugins can subscribe to app events

📌 **Permission-gated:** API calls check permissions before executing

---

## 12.0 Sandboxing & Security {#12-sandboxing-and-security}

**Prerequisites:** Sections 10, 11  
**Related to:** Plugin implementation  
**Implements:** Plugin security architecture  
**Read time:** ~25 minutes

**This section covers how to run untrusted plugin code safely.**

---

### 12.1 Why Sandbox Untrusted Code {#121-why-sandbox-untrusted-code}

`[CORE]`

#### The Risk

**Any code you run can do anything your user can do** (unless sandboxed).

```
┌─────────────────────────────────────────────────────────────┐
│              WHAT UNSANDBOXED CODE CAN DO                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ⚠️ A malicious plugin WITHOUT sandboxing could:            │
│                                                              │
│  • Read ANY file on the computer                            │
│    - Browser passwords                                       │
│    - SSH keys                                                │
│    - Financial documents                                     │
│                                                              │
│  • Send data to external servers                            │
│    - Steal personal information                             │
│    - Exfiltrate business documents                          │
│                                                              │
│  • Modify or delete files                                   │
│    - Ransomware (encrypt and demand payment)                │
│    - Destroy data                                            │
│                                                              │
│  • Install malware                                          │
│    - Keyloggers                                              │
│    - Cryptocurrency miners                                   │
│                                                              │
│  This is NOT hypothetical—it happens regularly              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Defense Layers

```
Security = Multiple Layers

Layer 1: PERMISSION MODEL
  • Plugin declares what it needs
  • User consents at install
  • App only grants what was approved

Layer 2: SANDBOX
  • Plugin code runs in isolation
  • Cannot access system outside sandbox
  • Even if code is malicious, damage is limited

Layer 3: REVIEW PROCESS
  • Marketplace review before listing
  • Automated security scanning
  • Community reporting

Layer 4: MONITORING
  • Track plugin behavior
  • Alert on suspicious activity
  • Ability to remotely disable malicious plugins
```

---

### 12.2 Sandboxing Technologies Compared {#122-sandboxing-technologies-compared}

`[CORE]`

#### Overview Table

```
┌──────────────┬────────────┬─────────────┬──────────────┬──────────────┐
│ Technology   │ Security   │ Performance │ Complexity   │ Best For     │
├──────────────┼────────────┼─────────────┼──────────────┼──────────────┤
│ WASM         │ ⭐⭐⭐⭐⭐    │ ⭐⭐⭐⭐      │ ⭐⭐⭐ Medium  │ Most plugins │
│ Pyodide      │ ⭐⭐⭐⭐⭐    │ ⭐⭐⭐        │ ⭐⭐⭐ Medium  │ Python AI    │
│ OS Subprocess│ ⭐⭐⭐⭐      │ ⭐⭐⭐⭐⭐     │ ⭐⭐ Complex  │ Legacy code  │
│ Containers   │ ⭐⭐⭐⭐⭐    │ ⭐⭐          │ ⭐ Very High  │ Heavy/risky  │
└──────────────┴────────────┴─────────────┴──────────────┴──────────────┘
```

---

#### WebAssembly (WASM) — Recommended

**What it is:** A binary instruction format that runs in a secure sandbox.

```
═══════════════════════════════════════════════════════════════
                    CORE CONCEPT: WASM Sandbox
═══════════════════════════════════════════════════════════════

  Plugin code compiles to WASM (from Rust, C++, AssemblyScript)
  
  ┌─────────────────────────────────────────────────────────┐
  │                    YOUR APPLICATION                      │
  │                                                          │
  │  ┌─────────────────────────────────────────────────┐    │
  │  │              WASM SANDBOX                        │    │
  │  │  ┌───────────────────────────────────────────┐  │    │
  │  │  │         PLUGIN CODE                       │  │    │
  │  │  │  • Cannot access filesystem               │  │    │
  │  │  │  • Cannot make network requests           │  │    │
  │  │  │  • Cannot read memory outside sandbox     │  │    │
  │  │  │  • Can ONLY call functions YOU expose     │  │    │
  │  │  └───────────────────────────────────────────┘  │    │
  │  │                                                  │    │
  │  │  Exposed Functions (your API):                  │    │
  │  │  • readDocument(id) → document                  │    │
  │  │  • saveDocument(id, content)                    │    │
  │  │  • showUI(html)                                 │    │
  │  │  • [nothing else—no system access]             │    │
  │  └─────────────────────────────────────────────────┘    │
  │                                                          │
  └─────────────────────────────────────────────────────────┘

═══════════════════════════════════════════════════════════════
```

**Why WASM is secure:**
- Memory is completely isolated (can't read/write outside sandbox)
- No system calls unless explicitly provided
- Even buggy code can't escape
- Industry-proven (used by Figma, Cloudflare, etc.)

**Performance:**
- Near-native speed (JIT compiled)
- Fast startup (milliseconds)
- Small overhead

**Complexity:**
- Plugins must be compiled to WASM
- Need to design the host API carefully
- Debugging is harder than native code

---

#### Pyodide (Python in WASM)

**What it is:** Full Python interpreter compiled to WASM.

```
Pyodide gives you Python plugins with WASM security.

Pros:
  ✓ Full Python ecosystem (numpy, pandas, etc.)
  ✓ Inherits WASM sandbox properties
  ✓ Plugin authors write normal Python

Cons:
  ✗ Slower than native Python
  ✗ Large initial download (~10MB+)
  ✗ Startup time can be significant
```

**Best for:** AI/data plugins that need Python libraries.

---

#### OS Subprocess Sandboxing

**What it is:** Running plugins as separate OS processes with restricted permissions.

```
┌─────────────────────────────────────────────────────────────┐
│                 OS-LEVEL SANDBOXING                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Main App                    Plugin Process                  │
│  ┌───────────┐              ┌───────────────────────────┐   │
│  │           │  IPC/Pipes   │ Restricted by:            │   │
│  │   Your    │◄────────────►│ • seccomp (Linux)         │   │
│  │   App     │              │ • AppArmor (Linux)        │   │
│  │           │              │ • sandbox-exec (macOS)    │   │
│  └───────────┘              │ • AppContainer (Windows)  │   │
│                             └───────────────────────────┘   │
│                                                              │
│  Can block:                                                  │
│    • File access outside allowed paths                      │
│    • Network access                                          │
│    • Process spawning                                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Pros:**
- Plugins written in normal languages (Python, Node)
- Native performance
- Familiar debugging

**Cons:**
- Different implementation per OS
- Easier to misconfigure (weaker guarantee)
- Heavier than WASM (process overhead)

---

### 12.3 Permission Models {#123-permission-models}

`[CORE]`

#### Capability Categories

```
┌─────────────────────────────────────────────────────────────┐
│                  PERMISSION CATEGORIES                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  FILESYSTEM SCOPES                                           │
│  ├── fs.read[/workspace/*]     Read specific paths          │
│  ├── fs.write[/workspace/out]  Write to specific paths      │
│  └── fs.none                   No filesystem access         │
│                                                              │
│  NETWORK SCOPES                                              │
│  ├── net.none                  No network (default)         │
│  ├── net.host[api.example.com] Specific domains only        │
│  └── net.any                   Unrestricted (dangerous)     │
│                                                              │
│  AI/MODEL SCOPES                                             │
│  ├── ai.none                   Cannot use AI                │
│  ├── ai.local                  Local models only            │
│  ├── ai.cloud                  Can call cloud APIs          │
│  └── ai.budget[10000]          Token limit per day          │
│                                                              │
│  WORKSPACE DATA SCOPES                                       │
│  ├── workspace.read            Read documents/boards        │
│  ├── workspace.write           Modify data                  │
│  └── workspace.none            No access to user data       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Install-Time Permission Dialog

```
┌─────────────────────────────────────────────────────────────┐
│                                                              │
│  Install "AI Writing Assistant"?                            │
│                                                              │
│  This plugin requests:                                       │
│                                                              │
│  📁 Read your documents                                      │
│     To analyze and improve your writing                     │
│                                                              │
│  🌐 Network access to api.grammarly.com                     │
│     To check grammar and spelling                           │
│                                                              │
│  🤖 Use local AI models                                      │
│     To generate writing suggestions                         │
│                                                              │
│  ─────────────────────────────────────────────              │
│                                                              │
│  ⚠️ This plugin cannot:                                     │
│     • Access files outside your workspace                   │
│     • Access other websites                                 │
│     • Modify system settings                                │
│                                                              │
│         [Cancel]                [Install]                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### 12.4 Recommended Security Architecture {#124-recommended-security-architecture}

`[CORE]`

```
═══════════════════════════════════════════════════════════════
                    DECISION POINT: Security Architecture
═══════════════════════════════════════════════════════════════

RECOMMENDED: WASM-First with Permission Model

Phase 1 (Internal Plugins):
  └── Simple process isolation
      • Run plugins as subprocesses
      • Limit via OS mechanisms where easy
      • Internal plugins are trusted (from your team)

Phase 2 (Community Plugins):
  └── WASM sandbox for all third-party code
      • Compile plugins to WASM
      • Expose only necessary APIs
      • Manifest-declared permissions
      • User consent dialog at install

Phase 3 (Marketplace):
  └── Full security pipeline
      • Automated security scanning
      • Manual review for sensitive permissions
      • Code signing
      • Remote disable capability

DEFAULT STANCE: Deny Everything
  • No filesystem access by default
  • No network by default
  • No AI access by default
  • Plugin must request; user must grant

═══════════════════════════════════════════════════════════════
```

#### Key Takeaways

- Plugins are a major security risk if not sandboxed
- WASM provides strong, proven isolation
- Implement permission model like browser extensions
- Default deny: plugins only get what they explicitly request and user approves
- Phase in security: start simple, add WASM sandbox for community plugins

---
---

# PART V: OBSERVABILITY & TESTING

---

## 13.0 AI Observability {#13-ai-observability}

**Prerequisites:** Sections 2-5 (LLM Infrastructure)  
**Related to:** Section 14 (Evaluation), Section 15 (Benchmarking)  
**Implements:** Monitoring and debugging AI behavior  
**Read time:** ~20 minutes

**This section covers how to monitor, debug, and understand what your AI systems are doing.**

---

### 13.1 What to Monitor in AI Apps {#131-what-to-monitor-in-ai-apps}

`[CORE]`

#### Why AI Needs Different Observability

**Traditional apps are deterministic; AI apps are probabilistic.** The same input might produce different outputs. This makes debugging harder.

```
Traditional App:
  Input: login("user", "pass")
  Output: Always same result (success or specific error)
  
AI App:
  Input: "Write me a poem about cats"
  Output: Different poem every time
  Problem: How do you know if it's working "correctly"?
```

#### Key Metrics to Track

```
┌─────────────────────────────────────────────────────────────┐
│                    AI OBSERVABILITY METRICS                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  PERFORMANCE METRICS                                         │
│  ├── Latency (p50, p95, p99)   How long requests take       │
│  ├── Tokens per second         Throughput measure           │
│  ├── Time to first token       Perceived responsiveness     │
│  └── Queue depth               Backlog of requests          │
│                                                              │
│  RESOURCE METRICS                                            │
│  ├── GPU memory usage          Are we close to OOM?         │
│  ├── GPU utilization %         Is GPU being used?           │
│  ├── CPU/RAM usage             System health                │
│  └── Model load/unload events  Memory management working?   │
│                                                              │
│  QUALITY SIGNALS                                             │
│  ├── Error rate                Model failures               │
│  ├── Retry rate                Had to try again             │
│  ├── Fallback rate             Local→cloud switches         │
│  ├── User feedback             Thumbs up/down               │
│  └── Task completion           Did user accomplish goal?    │
│                                                              │
│  COST METRICS (if using cloud APIs)                         │
│  ├── Tokens consumed           Input + output               │
│  ├── API spend                 Actual money                 │
│  └── Local vs cloud ratio      How much offloaded?         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### 13.2 Tools Comparison {#132-tools-comparison}

`[CORE]`

#### Overview

```
┌─────────────┬────────────────┬──────────────┬───────────────┐
│ Tool        │ Type           │ Local-First? │ Best For      │
├─────────────┼────────────────┼──────────────┼───────────────┤
│ OTel+Prom   │ General obs.   │ ✓ Yes        │ Core metrics  │
│ Langfuse    │ LLM-specific   │ Self-hosted  │ Full tracing  │
│ LangSmith   │ LLM-specific   │ Cloud only   │ LangChain     │
│ Helicone    │ LLM proxy      │ Self-hosted  │ Caching       │
└─────────────┴────────────────┴──────────────┴───────────────┘
```

---

#### OpenTelemetry + Prometheus + Grafana — Recommended Core

**What it is:** Industry-standard observability stack.

```
The "boring but reliable" choice:

┌─────────────────────────────────────────────────────────────┐
│                                                              │
│  Your App ──► OpenTelemetry ──► Prometheus ──► Grafana     │
│  (metrics)    (collection)      (storage)     (dashboards) │
│                                                              │
│  Also:                                                       │
│  Your App ──► OTel ──► Jaeger/Tempo ──► Grafana            │
│  (traces)                (storage)      (visualization)     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Pros:**
- ⭐ Fully local—no data leaves your machine
- ⭐ Vendor-neutral standard
- ⭐ Works with any backend (vLLM, TGI expose Prometheus metrics)
- ⭐ Flexible—you define what to track

**Cons:**
- ⚠️ No LLM-specific features out of box
- ⚠️ Must design your own metrics/spans
- ⚠️ Setup requires several components

---

#### Langfuse — Best LLM-Specific (Self-Hosted)

**What it is:** Open-source LLM observability platform.

```
Langfuse tracks:
  • Every prompt and response
  • Token counts and costs
  • Latency breakdowns
  • Tool calls within agents
  • User feedback
```

**Pros:**
- ⭐ Open-source, self-hostable
- ⭐ Purpose-built for LLM debugging
- ⭐ Tracks costs and tokens automatically
- ⭐ Integrates via OpenTelemetry

**Cons:**
- ⚠️ Requires running Postgres + Langfuse server
- ⚠️ Heavier setup than plain OTel

---

### 13.3 Privacy-Sensitive Logging {#133-privacy-sensitive-logging}

`[CORE]`

#### The Problem

**LLM logs contain user prompts, which may contain sensitive information.**

```
Example dangerous log:

{
  "timestamp": "2024-01-15T10:30:00Z",
  "prompt": "Write an email to john.doe@company.com about 
             my salary negotiation. My current salary is 
             $85,000 and I want to ask for $100,000",
  "response": "..."
}

This log contains:
  • Email address (PII)
  • Salary information (sensitive)
  • Professional context (private)
```

#### Best Practices

```
┌─────────────────────────────────────────────────────────────┐
│                PRIVACY-SAFE LOGGING                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. REDACT BEFORE LOGGING                                    │
│     • Use regex/libraries to detect PII                     │
│     • Replace: "john.doe@company.com" → "[EMAIL]"           │
│     • Tools: llm-guard Anonymize scanner                    │
│                                                              │
│  2. LOG METADATA, NOT CONTENT                                │
│     Good: { task: "email_draft", tokens_in: 50, success: T }│
│     Bad:  { prompt: "Write email to john...", ... }         │
│                                                              │
│  3. SAMPLE, DON'T LOG EVERYTHING                             │
│     • Log 10% of interactions for debugging                 │
│     • Full logs only with explicit user consent             │
│                                                              │
│  4. SHORT RETENTION                                          │
│     • Delete detailed logs after 7-30 days                  │
│     • Keep aggregated metrics longer                        │
│                                                              │
│  5. LOCAL ONLY                                               │
│     • Never send raw prompts to cloud services              │
│     • If cloud needed, anonymize first                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Safe Logging Schema

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "task_type": "email_draft",
  "agent": "writing_assistant",
  "model": "mistral-7b",
  "tokens_in": 50,
  "tokens_out": 120,
  "latency_ms": 850,
  "success": true,
  "error": null,
  "pii_detected": false,
  "user_feedback": null
}
```

Note: No actual prompt or response content logged.

---

### 13.4 Metrics & Dashboards {#134-metrics-and-dashboards}

`[CORE]`

#### Essential Dashboard Panels

```
┌─────────────────────────────────────────────────────────────┐
│                    GRAFANA DASHBOARD                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ROW 1: HEALTH AT A GLANCE                                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │ Requests/min│ │ Error Rate  │ │ p95 Latency │            │
│  │    42       │ │   0.5%      │ │   850ms     │            │
│  └─────────────┘ └─────────────┘ └─────────────┘            │
│                                                              │
│  ROW 2: LATENCY OVER TIME                                    │
│  ┌──────────────────────────────────────────────┐           │
│  │  ────p50   ────p95   ────p99                 │           │
│  │     ╭──────╮      ╭─────────╮                │           │
│  │  ───╯      ╰──────╯         ╰────────────    │           │
│  └──────────────────────────────────────────────┘           │
│                                                              │
│  ROW 3: RESOURCES                                            │
│  ┌──────────────────────┐ ┌──────────────────────┐          │
│  │ GPU Memory           │ │ GPU Utilization      │          │
│  │ █████████░░░ 75%    │ │ ████████░░░░ 67%     │          │
│  └──────────────────────┘ └──────────────────────┘          │
│                                                              │
│  ROW 4: BY MODEL                                             │
│  ┌──────────────────────────────────────────────┐           │
│  │ Model      │ Requests │ Avg Latency │ Errors │           │
│  │ mistral-7b │ 1,234    │ 340ms       │ 0.2%   │           │
│  │ codellama  │ 567      │ 520ms       │ 0.8%   │           │
│  └──────────────────────────────────────────────┘           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Instrumentation Example

```python
from opentelemetry import trace, metrics

tracer = trace.get_tracer(__name__)
meter = metrics.get_meter(__name__)

# Define metrics
request_counter = meter.create_counter(
    "llm_requests_total",
    description="Total LLM requests"
)
latency_histogram = meter.create_histogram(
    "llm_latency_seconds",
    description="LLM request latency"
)

# Instrument a function
async def call_llm(prompt, model):
    with tracer.start_as_current_span("llm_call") as span:
        span.set_attribute("model", model)
        
        start = time.time()
        try:
            response = await model.generate(prompt)
            
            request_counter.add(1, {"model": model, "status": "success"})
            latency_histogram.record(time.time() - start, {"model": model})
            
            return response
        except Exception as e:
            request_counter.add(1, {"model": model, "status": "error"})
            span.record_exception(e)
            raise
```

---

## 14.0 Evaluation & Quality {#14-evaluation-and-quality}

**Prerequisites:** Section 13  
**Related to:** Section 15 (Benchmarking)  
**Implements:** Quality assurance for AI outputs  
**Read time:** ~15 minutes

**This section covers how to test and evaluate LLM output quality.**

---

### 14.1 Testing LLM Outputs {#141-testing-llm-outputs}

`[CORE]`

#### The Challenge

**LLM outputs are non-deterministic.** Traditional unit tests expect exact outputs:

```python
# Traditional test (deterministic)
def test_add():
    assert add(2, 3) == 5  # Always passes or fails consistently

# LLM test (non-deterministic)
def test_poem():
    poem = llm("Write a haiku about code")
    assert poem == "???"  # What do we check?
```

#### Testing Strategies

```
┌─────────────────────────────────────────────────────────────┐
│                  LLM TESTING STRATEGIES                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. GOLDEN TEST SUITES                                       │
│     • Define representative test prompts                    │
│     • For deterministic tasks: check exact output           │
│     • For generative tasks: check key properties            │
│                                                              │
│  Example:                                                    │
│    Prompt: "What is 2+2?"                                   │
│    Assert: "4" in response.lower()                          │
│                                                              │
│  2. PROPERTY-BASED TESTS                                     │
│     • Check structural properties, not exact content        │
│     • Response length in expected range                     │
│     • Contains required keywords                            │
│     • Valid JSON/format                                     │
│                                                              │
│  Example:                                                    │
│    Prompt: "Write JSON with name and age"                   │
│    Assert: valid JSON, has "name" key, has "age" key        │
│                                                              │
│  3. LLM-AS-JUDGE                                             │
│     • Use another LLM to evaluate output quality            │
│     • Rate on criteria: correctness, coherence, helpfulness │
│     • Scalable but adds latency/cost                        │
│                                                              │
│  Example:                                                    │
│    Ask GPT-4: "Rate this response 1-5 for helpfulness: ..." │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Golden Test Example

```python
# tests/test_llm_golden.py

GOLDEN_TESTS = [
    {
        "name": "math_simple",
        "prompt": "What is 15 + 27?",
        "expected_contains": ["42"],
    },
    {
        "name": "code_function",
        "prompt": "Write a Python function that adds two numbers",
        "expected_contains": ["def ", "return"],
    },
    {
        "name": "json_extraction",
        "prompt": "Extract the name and date from: 'Meeting with Alice on Jan 5th'",
        "validate": lambda r: "alice" in r.lower() and "jan" in r.lower(),
    },
]

def test_golden_suite():
    for test in GOLDEN_TESTS:
        response = call_llm(test["prompt"])
        
        if "expected_contains" in test:
            for expected in test["expected_contains"]:
                assert expected in response, f"Failed {test['name']}"
        
        if "validate" in test:
            assert test["validate"](response), f"Failed {test['name']}"
```

---

### 14.2 Multi-Agent Tracing {#142-multi-agent-tracing}

#### The Complexity

**Multi-agent systems have many components talking to each other.** Debugging requires seeing the full flow.

```
User Request: "Summarize this document and create action items"

Agent Flow:
  ┌─────────────┐
  │ Orchestrator│──► "This needs summarization + extraction"
  └──────┬──────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌───────┐ ┌─────────┐
│Summary│ │Extractor│
│ Agent │ │  Agent  │
└───┬───┘ └────┬────┘
    │          │
    ▼          ▼
┌───────┐ ┌─────────┐
│Mistral│ │CodeLlama│
│  LLM  │ │   LLM   │
└───┬───┘ └────┬────┘
    │          │
    └────┬─────┘
         │
         ▼
    ┌─────────┐
    │ Combine │
    │ Results │
    └─────────┘
```

#### Tracing with OpenTelemetry

```python
# Each agent action becomes a span
with tracer.start_as_current_span("user_request") as root:
    root.set_attribute("request_type", "summarize_and_extract")
    
    with tracer.start_as_current_span("orchestrator_decision") as span:
        span.set_attribute("decision", "parallel_agents")
    
    # These run in parallel but are child spans
    with tracer.start_as_current_span("summary_agent") as span:
        with tracer.start_as_current_span("llm_call_mistral") as llm:
            summary = await call_mistral(document)
            
    with tracer.start_as_current_span("extractor_agent") as span:
        with tracer.start_as_current_span("llm_call_codellama") as llm:
            actions = await call_codellama(document)
    
    with tracer.start_as_current_span("combine_results") as span:
        result = combine(summary, actions)
```

#### Trace Visualization

```
In Jaeger/Tempo, you'd see:

user_request                     [═══════════════════════════════] 2.5s
  └─ orchestrator_decision       [══]                               0.1s
  └─ summary_agent               [══════════════]                   1.2s
       └─ llm_call_mistral       [════════════]                     1.0s
  └─ extractor_agent             [════════════════]                 1.5s
       └─ llm_call_codellama     [══════════════]                   1.3s
  └─ combine_results             [══]                               0.1s
```

---

## 15.0 Benchmark Harness {#15-benchmark-harness}

**Prerequisites:** Sections 13, 14  
**Related to:** Performance optimization  
**Implements:** Systematic performance testing  
**Read time:** ~15 minutes

**This section describes a benchmark system for measuring and comparing model/runtime performance.**

---

### 15.1 Benchmark Architecture {#151-benchmark-architecture}

`[CORE]`

#### Why Build a Benchmark Harness?

**Reproducible performance testing** lets you:
- Compare runtimes (Ollama vs vLLM)
- Compare models (Mistral-7B vs Llama2-7B)
- Measure impact of configuration changes
- Track performance over time

#### System Design

```
┌─────────────────────────────────────────────────────────────┐
│                  BENCHMARK HARNESS                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  CONFIG FILES (YAML)                                         │
│  ├── models.yml      Model endpoints and settings           │
│  ├── scenarios.yml   Test scenarios to run                  │
│  └── prompts.yml     Standard prompts for testing           │
│                                                              │
│  ADAPTERS                                                    │
│  ├── OllamaAdapter   Talks to Ollama                        │
│  ├── VLLMAdapter     Talks to vLLM                          │
│  ├── TGIAdapter      Talks to TGI                           │
│  └── ImageAdapter    Talks to ComfyUI                       │
│                                                              │
│  RUNNERS                                                     │
│  ├── SingleLLMRunner      One model, one prompt             │
│  ├── ConcurrentRunner     Multiple parallel requests        │
│  └── MixedWorkloadRunner  LLM + Image together              │
│                                                              │
│  OUTPUT                                                      │
│  ├── results.jsonl   Raw timing data                        │
│  └── report.md       Summary statistics                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### 15.2 Scenarios & Adapters {#152-scenarios-and-adapters}

#### Example Configuration

```yaml
# models.yml
models:
  - id: mistral-7b-ollama
    type: ollama
    endpoint: http://localhost:11434
    model_name: mistral
    
  - id: mistral-7b-vllm
    type: vllm
    endpoint: http://localhost:8000
    model_name: mistralai/Mistral-7B-v0.1

# scenarios.yml
scenarios:
  - id: single_chat
    type: single_llm
    models: [mistral-7b-ollama, mistral-7b-vllm]
    prompts: [short_qa, medium_qa, long_generation]
    iterations: 10
    
  - id: concurrent_load
    type: load_sweep
    models: [mistral-7b-vllm]
    prompts: [medium_qa]
    concurrency_levels: [1, 2, 4, 8, 16]
    iterations: 5

# prompts.yml
prompts:
  - id: short_qa
    text: "What is the capital of France?"
    max_tokens: 50
    
  - id: medium_qa
    text: "Explain how photosynthesis works in 3 paragraphs."
    max_tokens: 300
```

#### Adapter Interface

```python
# adapters.py
class LLMAdapter:
    """Base class for model adapters"""
    
    async def generate(self, prompt: str, params: dict) -> Result:
        raise NotImplementedError

class OllamaAdapter(LLMAdapter):
    async def generate(self, prompt: str, params: dict) -> Result:
        start = time.time()
        response = await httpx.post(
            f"{self.endpoint}/api/generate",
            json={"model": self.model, "prompt": prompt, **params}
        )
        elapsed = time.time() - start
        
        data = response.json()
        return Result(
            text=data["response"],
            tokens_in=data["prompt_eval_count"],
            tokens_out=data["eval_count"],
            latency=elapsed
        )
```

---

### 15.3 Reporting & Analysis {#153-reporting-and-analysis}

#### Output Format

```
# Benchmark Report - 2024-01-15

## Summary

| Scenario       | Model             | Avg Latency | p50    | p95    | Tokens/sec |
|----------------|-------------------|-------------|--------|--------|------------|
| single_chat    | mistral-7b-ollama | 340ms       | 320ms  | 450ms  | 88         |
| single_chat    | mistral-7b-vllm   | 310ms       | 300ms  | 420ms  | 97         |
| concurrent_8   | mistral-7b-vllm   | 180ms       | 170ms  | 250ms  | 620        |

## Findings

- vLLM is ~10% faster for single requests
- vLLM scales much better under load (620 vs ~100 tokens/sec at 8 concurrent)
- Ollama shows consistent latency regardless of load (no batching)

## Recommendations

- Use Ollama for development/single-user
- Use vLLM for production/multi-user scenarios
```

---
---

# PART VI: IMPLEMENTATION

---

## 16.0 Technology Stack Summary {#16-technology-stack-summary}

**Read time:** ~5 minutes

```
┌─────────────────────────────────────────────────────────────┐
│              COMPLETE TECHNOLOGY STACK                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  DESKTOP FRAMEWORK                                           │
│  ├── Primary: Tauri (Rust) + React/Vue                      │
│  └── Alternative: Electron (if more JS ecosystem needed)    │
│                                                              │
│  LLM INFRASTRUCTURE                                          │
│  ├── Runtime: Ollama (dev) + vLLM (production)              │
│  ├── Models: Mistral-7B, CodeLlama-7B, Llama2-13B           │
│  └── Images: ComfyUI + SDXL                                 │
│                                                              │
│  DATA LAYER                                                  │
│  ├── CRDT: Yjs (or Loro for Rust)                           │
│  ├── Database: SQLite                                        │
│  └── Sync: Yjs WebSocket provider (later)                   │
│                                                              │
│  PLUGIN SYSTEM                                               │
│  ├── Sandbox: WASM (Wasmtime)                               │
│  ├── Language: AssemblyScript/Rust → WASM                   │
│  └── Permissions: Manifest-based capability model           │
│                                                              │
│  OBSERVABILITY                                               │
│  ├── Telemetry: OpenTelemetry                               │
│  ├── Metrics: Prometheus                                    │
│  ├── Visualization: Grafana                                 │
│  └── Traces: Jaeger or Grafana Tempo                        │
│                                                              │
│  LANGUAGES                                                   │
│  ├── Backend: Python (orchestrator) + Rust (Tauri)          │
│  ├── Frontend: TypeScript + React/Vue                       │
│  └── Plugins: AssemblyScript → WASM                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 17.0 Implementation Roadmap {#17-implementation-roadmap}

`[CORE]`

```
═══════════════════════════════════════════════════════════════
                    IMPLEMENTATION PHASES
═══════════════════════════════════════════════════════════════

PHASE 0: Foundation (Weeks 1-4)
──────────────────────────────
  ✓ Set up Tauri project with basic UI shell
  ✓ Install Ollama, download Mistral-7B
  ✓ Basic chat interface calling local model
  ✓ SQLite setup for storing chat history
  
  Deliverable: "Hello world" AI chat app

PHASE 1: Core Editor (Weeks 5-8)
────────────────────────────────
  ✓ Integrate Yjs for collaborative editing
  ✓ Build rich text editor (TipTap + Yjs)
  ✓ Document storage in SQLite
  ✓ Basic AI commands in editor (summarize, rewrite)
  
  Deliverable: Local-first document editor with AI

PHASE 2: Multi-Model (Weeks 9-12)
─────────────────────────────────
  ✓ Add CodeLlama for code tasks
  ✓ Build model routing logic in orchestrator
  ✓ GPU memory management (load/unload)
  ✓ Basic observability (Prometheus + Grafana)
  
  Deliverable: Specialized AI for different tasks

PHASE 3: Plugin System MVP (Weeks 13-16)
────────────────────────────────────────
  ✓ Design plugin manifest format
  ✓ Build simple subprocess sandbox
  ✓ Create plugin API (register commands, access docs)
  ✓ 1-2 sample plugins (internal)
  
  Deliverable: Working internal plugin system

PHASE 4: Polish & Security (Weeks 17-20)
────────────────────────────────────────
  ✓ WASM sandbox for third-party plugins
  ✓ Permission model and install dialogs
  ✓ Sync between devices (file-based or server)
  ✓ Performance optimization
  
  Deliverable: Beta-ready application

═══════════════════════════════════════════════════════════════
```

---

## 18.0 Gap Analysis & Open Questions {#18-gap-analysis}

`[CORE]`

### What the Research DOESN'T Cover

```
┌─────────────────────────────────────────────────────────────┐
│                      RESEARCH GAPS                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  USER INTERFACE                                              │
│  • No detailed UI/UX designs                                │
│  • No accessibility considerations                          │
│  • No mobile/responsive strategy                            │
│  Action: Need separate UI design research                   │
│                                                              │
│  AUTHENTICATION & MULTI-USER                                 │
│  • No user account system design                            │
│  • No team/sharing model                                    │
│  • No encryption for sensitive data                         │
│  Action: Research if/when adding cloud sync                 │
│                                                              │
│  BUSINESS MODEL                                              │
│  • No pricing strategy                                      │
│  • No marketplace economics for plugins                     │
│  Action: Business planning separate from technical          │
│                                                              │
│  SPECIFIC MODEL FINE-TUNING                                  │
│  • Research covers pre-trained models only                  │
│  • No guidance on fine-tuning for specific use cases        │
│  Action: May need if default models insufficient            │
│                                                              │
│  WINDOWS-SPECIFIC ISSUES                                     │
│  • Limited coverage of Windows sandboxing options           │
│  • No Windows installer/distribution guidance               │
│  Action: Platform-specific research needed                  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Open Technical Questions

```
┌─────────────────────────────────────────────────────────────┐
│                    OPEN QUESTIONS                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Tauri vs Electron final decision?                       │
│     • Tauri: Smaller, faster, Rust backend                  │
│     • Electron: More mature, larger ecosystem               │
│     → Recommendation: Start with Tauri, reconsider if       │
│       ecosystem limitations become blocking                 │
│                                                              │
│  2. How to handle very long documents?                      │
│     • Context windows are limited (4K-8K tokens)            │
│     • Options: Chunking, summarization, RAG                 │
│     → Need: RAG (Retrieval Augmented Generation) research   │
│                                                              │
│  3. Offline-first sync strategy?                            │
│     • File sync (OneDrive/Dropbox) simple but limited       │
│     • Custom sync server more powerful but complex          │
│     → Recommendation: Start with file sync, add server      │
│       when multi-user collaboration is priority             │
│                                                              │
│  4. Plugin language choice?                                 │
│     • WASM requires compilation (barrier to entry)          │
│     • JavaScript simpler but harder to sandbox              │
│     → Recommendation: Support both—sandboxed JS for         │
│       simple plugins, WASM for advanced/untrusted           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---
---

# END MATTER

---

## Consolidated Glossary {#consolidated-glossary}

| Term | Definition |
|------|------------|
| **API** | Application Programming Interface—a defined way for programs to communicate with each other |
| **Automerge** | A CRDT library that stores full history, good for version tracking but higher memory usage |
| **Batching** | Processing multiple requests together for efficiency |
| **Context Window** | How many tokens an LLM can "see" at once—its working memory |
| **CRDT** | Conflict-free Replicated Data Type—data structures that can automatically merge without conflicts |
| **CUDA** | NVIDIA's technology for running computations on GPUs |
| **Electron** | A framework for building desktop apps with web technologies (HTML/CSS/JS) |
| **GGUF** | A file format for quantized LLM models, used by llama.cpp and Ollama |
| **GPU** | Graphics Processing Unit—hardware that runs AI models very fast |
| **Inference** | Using a trained AI model to generate outputs (vs training which creates the model) |
| **KV Cache** | Key-Value cache—memory used to store conversation context during inference |
| **Langfuse** | Open-source LLM observability platform |
| **LLM** | Large Language Model—AI that generates text by predicting the next word |
| **Local-First** | Software design where data lives primarily on user's device, not in the cloud |
| **Loro** | A new CRDT library with full history and movable trees, written in Rust |
| **Manifest** | A configuration file declaring a plugin's metadata, permissions, and capabilities |
| **Ollama** | A user-friendly tool for running LLMs locally |
| **OpenTelemetry (OTel)** | Industry standard for collecting metrics, traces, and logs |
| **Parameters** | The "knowledge" of an AI model, stored as numbers |
| **Prometheus** | Time-series database commonly used for metrics |
| **Quantization** | Compressing a model to use less memory by reducing number precision |
| **Q4/Q5/Q8** | Quantization levels—lower numbers mean smaller size but slightly lower quality |
| **Runtime** | Software that loads and executes AI models |
| **Sandbox** | An isolated environment where untrusted code can run without affecting the rest of the system |
| **SDXL** | Stable Diffusion XL—the latest version of the Stable Diffusion image generation model |
| **SQLite** | A lightweight embedded database stored as a single file |
| **Streaming** | Sending response tokens one at a time as they're generated |
| **Tauri** | A framework for building desktop apps with web frontend and Rust backend |
| **TGI** | Text Generation Inference—HuggingFace's production LLM server |
| **Token** | A chunk of text (roughly ¾ of a word) that LLMs process |
| **vLLM** | A high-performance LLM inference engine optimized for throughput |
| **VRAM** | Video RAM—memory on the GPU where models must be loaded to run fast |
| **WASM** | WebAssembly—a binary format that runs in a secure sandbox |
| **Yjs** | A popular CRDT library for JavaScript, known for performance and editor integrations |

---

## Sources Referenced {#sources-referenced}

This document synthesizes research from the following source documents:

1. **LLM Inference Runtimes** (8 pages) — Runtime comparison, model candidates, image generation, scheduling patterns

2. **Inference Runtimes** (7 pages) — Runtime comparison, model selection by role, GPU bottlenecks, recommendations

3. **Benchmark Harness Design** (5 pages) — Modular Python benchmark architecture, adapters, scenarios, reporting

4. **Local-First Data and Sync Architecture** (9 pages) — CRDT libraries, database patterns, sync topologies, conflict resolution UX

5. **Extension Platforms: Architectural Overview** (10 pages) — Plugin system analysis (VS Code, Obsidian, Figma, browsers), proposed architecture

6. **Local-First Multi-Model LLM Hosting** (8 pages) — Runtime survey, integration strategies, memory management, recommendations

7. **Sandboxing Options for Untrusted Code** (12 pages) — WASM, Pyodide, OS sandboxing, permission models, security architecture

8. **AI Observability and Evaluation** (10 pages) — Logging, metrics, privacy, evaluation methods, multi-agent tracing, phased rollout

---

## Document Navigation Tips

**For Claude/LLM Context:**
When referencing this document in future conversations, you can use section anchors:
- "See #3-llm-inference-runtimes for runtime details"
- "Reference #12-sandboxing-and-security for plugin security"

**For Quick Decisions:**
Search for "DECISION POINT" to find all major technical choices with recommendations.

**For Implementation:**
Search for "✓ Action Items" or follow the roadmap in Section 17.

---



---

# PART VII: CONSOLIDATED ARCHITECTURE & ROADMAP

---

# 19. Executive Summary {#19-executive-summary}

**Prerequisites:** None - start here  
**Related to:** All sections  
**Implements:** Project overview and orientation  
**Read time:** ~5 minutes

**This section provides a bird's-eye view of Project Handshake: what it is, why it matters, and the key decisions that have been made based on research.**

---

### TL;DR Box

> **Project Handshake** is a desktop application combining:
> - **Notion-like** document editing with databases
> - **Milanote-like** visual canvas/moodboards  
> - **Excel-like** spreadsheets with formulas
> - **Local AI models** for writing, coding, and image generation
> 
> **Tech Stack Decision:** Tauri + React + TypeScript (frontend) + Python (AI backend)
> 
> **Key Insight:** Run AI models locally for privacy, speed, and cost savings—with cloud fallback when needed.

---

### What We're Building

**Project Handshake is a "local AI cloud" on your desktop.** Instead of sending your documents, ideas, and data to cloud services like Notion or Google Docs, everything stays on your computer. AI assistants run locally too, meaning your sensitive information never leaves your machine.

The application combines three types of tools that creative professionals typically use separately:

| Tool Type | Inspiration | Use Case |
|-----------|-------------|----------|
| **Rich Documents** | Notion | Writing, planning, structured databases |
| **Visual Canvas** | Milanote | Mood boards, brainstorming, spatial organization |
| **Spreadsheets** | Excel | Data manipulation, calculations, analysis |

**What makes this different:** Local AI models collaborate to help you. One AI might plan your project, another writes the code, and a third generates images—all coordinated automatically.

---

### Key Architecture Decisions (From Research)

Based on extensive research across multiple documents, the following decisions have been validated:

| Decision | Choice | Why |
|----------|--------|-----|
| Desktop Shell | **Tauri** (not Electron) | 90% less memory usage; critical when running AI models |
| Frontend | **React + TypeScript** | Rich ecosystem, same code works in both shells |
| Backend | **Python** | Best AI/ML library support, orchestration frameworks |
| AI Orchestration | **AutoGen or LangGraph** | Mature multi-agent coordination |
| Data Sync | **CRDTs (Yjs)** | Offline-first, conflict-free collaboration |
| Storage | **File-tree based** | Human-readable, portable, git-friendly |

---

### Why Local-First Matters

📌 **Key Point:** The entire architecture is designed around "local-first" principles:

1. **Privacy:** Your documents and AI conversations never leave your computer
2. **Speed:** No network latency for AI responses
3. **Cost:** After initial model download, AI usage is essentially free
4. **Reliability:** Works without internet, on airplanes, in poor connectivity
5. **Control:** You own your data in standard file formats

---

### Hardware Context

The target hardware for development and initial deployment:

| Component | Specification | Why It Matters |
|-----------|--------------|----------------|
| CPU | Ryzen 9 5950X (16 cores) | Handles multiple processes, CPU inference fallback |
| RAM | 128 GB | Multiple AI models can stay loaded in memory |
| GPU | RTX 3090 (24GB VRAM) | Runs large AI models, image generation |
| Storage | NVMe SSD | Fast model loading, responsive file operations |

⚠️ **Warning:** This hardware is above average. The app design must handle graceful degradation for users with less powerful systems, including cloud fallback options.

---

# 20. Foundation Concepts {#20-foundation-concepts}

Before diving into specific technical decisions, let's establish foundational understanding of the core concepts that appear throughout this document.

---

## 20.1 What is a Desktop Application Shell? {#201-what-is-a-desktop-application-shell}

**Prerequisites:** None - foundational  
**Related to:** Section 3.1 (Tauri vs Electron)  
**Implements:** Understanding architecture choices  
**Read time:** ~4 minutes

**A "shell" is the container that turns web code into a desktop application. It's the bridge between your web-based user interface and the operating system.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Desktop Shell** | A program that wraps website-style code so it runs as a regular desktop app (with window controls, file access, etc.) | We need to choose between Tauri and Electron as our shell |
| **Electron** | The most popular shell; used by VS Code, Slack, Discord. Bundles a complete Chrome browser inside your app | Higher memory usage but battle-tested and familiar |
| **Tauri** | A newer, lighter shell using Rust. Uses the operating system's built-in browser instead of bundling one | Much lower memory usage—critical when AI models need that RAM |
| **WebView** | A "browser window without the browser"—just the part that displays web pages | Tauri uses the system's webview; Electron bundles its own |
| **IPC (Inter-Process Communication)** | How different parts of a program talk to each other | How the UI will communicate with the Python AI backend |

---

### The Mental Model

Think of building a desktop app like building a food truck:

```
┌─────────────────────────────────────────────┐
│              DESKTOP SHELL                   │
│         (The food truck itself)              │
│  ┌─────────────────────────────────────┐    │
│  │           YOUR WEB APP               │    │
│  │      (The kitchen equipment)         │    │
│  │  ┌─────────────────────────────┐    │    │
│  │  │    React + TypeScript       │    │    │
│  │  │   (The menu & recipes)      │    │    │
│  │  └─────────────────────────────┘    │    │
│  └─────────────────────────────────────┘    │
│                    │                         │
│                    ▼                         │
│    ┌─────────────────────────────┐          │
│    │     Operating System        │          │
│    │   (Where the truck parks)   │          │
│    └─────────────────────────────┘          │
└─────────────────────────────────────────────┘
```

**Electron** = A food truck that brings its own generator, water supply, and waste system—self-contained but heavy.

**Tauri** = A food truck that plugs into the venue's electricity and plumbing—lighter but depends on what's available.

---

### Why This Matters for Handshake

═══ CORE CONCEPT ═══

> Every megabyte of RAM the shell uses is a megabyte NOT available for AI models.
> 
> - Electron idle: ~150-300 MB RAM
> - Tauri idle: ~10-50 MB RAM
> 
> That 200+ MB difference could mean running a larger AI model or faster response times.

---

### Key Takeaways

- ✓ A desktop shell turns web code into a native application
- ✓ Electron is mature but memory-hungry; Tauri is lean but newer
- ✓ For AI-heavy apps, memory efficiency becomes critical
- ✓ Both shells run the same React/TypeScript frontend code

**See Also:** [Section 3.1 - Tauri vs Electron Decision](#211-desktop-shell-tauri-vs-electron)

---

## 20.2 Understanding Local-First Software {#202-understanding-local-first-software}

**Prerequisites:** None - foundational  
**Related to:** Section 7 (Collaboration and Sync)  
**Implements:** Core design philosophy  
**Read time:** ~5 minutes

**"Local-first" means your data lives on YOUR computer first, and optionally syncs to the cloud—the opposite of how most modern apps work.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Local-First** | Data stored on your device as the "source of truth," not on company servers | Core philosophy—you own your data |
| **Cloud-First** | Data lives on servers; your device just displays it (Google Docs, Notion) | What we're avoiding |
| **Offline-First** | App works without internet; syncs when connection returns | Handshake must work on airplanes |
| **Sync** | Keeping multiple copies of data up-to-date with each other | Needed for multi-device and collaboration |
| **Conflict Resolution** | Deciding what happens when two people edit the same thing | CRDTs handle this automatically |

---

### The Spectrum of Data Ownership

```
CLOUD-FIRST                                    LOCAL-FIRST
     │                                              │
     ▼                                              ▼
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│ Google  │    │ Notion  │    │Obsidian │    │Handshake│
│  Docs   │    │         │    │         │    │(our app)│
└─────────┘    └─────────┘    └─────────┘    └─────────┘
     │              │              │              │
  Server       Server+Cache    Files+Sync    Files+AI+
  Required      Preferred       Local         Optional
                               Primary         Cloud
```

---

### Why Local-First for an AI Productivity App?

═══ CORE CONCEPT ═══

> **Privacy + Performance + Cost + Control**
> 
> 1. **Privacy:** AI sees your documents. Do you want that on someone else's servers?
> 2. **Performance:** No network round-trip for AI responses
> 3. **Cost:** Cloud AI APIs charge per request; local models are "free" after download
> 4. **Control:** Export everything to standard formats anytime

---

### Real-World Analogy

**Cloud-First (like Notion):**
- Your files are stored in a bank vault
- You need to visit the bank (internet) to see them
- The bank could close, change terms, or read your files
- Very secure from local theft, but you depend on the bank

**Local-First (like Handshake):**
- Your files are in a safe in your home
- You can access them anytime, even with no internet
- You can make copies anywhere you want
- You're responsible for backups

---

### The Challenge: Collaboration

The main trade-off: **If data is on your computer, how do multiple people edit together?**

Solution: **CRDTs** (Conflict-free Replicated Data Types)—special data structures that can merge edits from multiple sources without conflicts.

💡 **Tip:** Think of CRDTs like a Google Doc that works offline. Everyone types on their own copy, and when they reconnect, the document intelligently merges all changes.

**See Also:** [Section 7.1 - Understanding CRDTs](#251-understanding-crdts)

---

### Key Takeaways

- ✓ Local-first = your data lives on your device primarily
- ✓ Critical for privacy when AI models access your documents
- ✓ Enables offline work and eliminates API costs
- ✓ CRDTs enable collaboration without central servers
- ✓ You can still sync to cloud—it's just optional

---

## 20.3 What are AI Models and How Do They Run Locally? {#203-what-are-ai-models-and-how-do-they-run-locally}

**Prerequisites:** None - foundational  
**Related to:** Section 5 (AI Model Strategy)  
**Implements:** Understanding AI integration approach  
**Read time:** ~6 minutes

**An AI model is a very large mathematical formula that takes in text (or images) and produces intelligent-seeming responses. "Running locally" means this formula executes on YOUR computer, not a company's servers.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **LLM (Large Language Model)** | An AI trained on massive text to understand and generate language. ChatGPT is an LLM. | The "brain" that will write, summarize, and reason |
| **Parameters** | The "knobs" inside the AI model. More parameters = smarter but heavier. "7B" = 7 billion parameters | Determines which models fit on your hardware |
| **VRAM** | Video RAM—memory on your graphics card | Where AI models live during use; RTX 3090 has 24GB |
| **Inference** | The AI actually doing its job (generating a response) | What happens when you ask the AI something |
| **Quantization** | Shrinking a model to fit in less memory (with some quality loss) | How we fit big models on consumer hardware |
| **GGUF** | A file format for quantized models | The format we'll download models in |

---

### How Big Are These Models?

```
Model Size vs. Quality vs. Hardware Requirements

┌────────────────────────────────────────────────────────┐
│ 70B (GPT-4 class)  ████████████████████████  │ 140GB+ │
│  - Smartest, needs multiple GPUs or cloud    │        │
├────────────────────────────────────────────────────────┤
│ 34B (Very Good)    ████████████████         │ ~70GB  │
│  - Excellent quality, pushes 3090 limits     │        │
├────────────────────────────────────────────────────────┤
│ 13B (Good)         ████████                 │ ~26GB  │
│  - Great balance, fits 3090 with room       │ ← Sweet│
├────────────────────────────────────────────────────────┤
│ 7B (Decent)        ████                     │ ~14GB  │
│  - Fast, leaves room for other models        │  Spot  │
├────────────────────────────────────────────────────────┤
│ 3B (Basic)         ██                       │ ~6GB   │
│  - Quick tasks, limited capability           │        │
└────────────────────────────────────────────────────────┘
```

---

### The Model Zoo

Handshake needs DIFFERENT models for DIFFERENT tasks:

| Task | Model Type | Example | Size |
|------|-----------|---------|------|
| Writing & Reasoning | General LLM | Llama 3, Mistral | 7-13B |
| Code Generation | Code-specialized | Code Llama, StarCoder | 7-15B |
| Image Generation | Diffusion Model | SDXL | ~3B |
| Task Planning | Reasoning LLM | GPT-OSS-20B | 20B |

═══ CORE CONCEPT ═══

> **You won't run all models simultaneously.** The orchestrator loads/unloads models based on what's needed. The 3090 has 24GB; a 13B model uses ~14GB quantized, leaving 10GB for SDXL image generation.

---

### Local vs. Cloud AI

```
                    LOCAL                     CLOUD (API)
                      │                           │
    ┌─────────────────┴──────────────┐  ┌────────┴────────┐
    │ ✓ Private - data stays home   │  │ ✗ Data sent to  │
    │ ✓ Free after download         │  │   company       │
    │ ✓ Works offline               │  │ ✗ Per-request   │
    │ ✗ Limited by your hardware    │  │   cost          │
    │ ✗ Slower than cloud GPUs      │  │ ✗ Needs internet│
    │                               │  │ ✓ Latest models │
    │ GOOD FOR: Frequent,           │  │ ✓ Most powerful │
    │ routine tasks                 │  │                 │
    └───────────────────────────────┘  │ GOOD FOR: Hard  │
                                       │ tasks, fallback │
                                       └─────────────────┘
```

---

### How It Actually Works

```
┌─────────────────────────────────────────────────────────┐
│                    YOUR COMPUTER                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │              PYTHON BACKEND                      │    │
│  │  ┌─────────────────────────────────────────┐    │    │
│  │  │         MODEL RUNTIME                    │    │    │
│  │  │  (vLLM, Ollama, or llama.cpp)           │    │    │
│  │  │                                          │    │    │
│  │  │   ┌───────┐  ┌───────┐  ┌───────┐       │    │    │
│  │  │   │Llama 3│  │ Code  │  │ SDXL  │       │    │    │
│  │  │   │ 13B   │  │ Llama │  │       │       │    │    │
│  │  │   └───────┘  └───────┘  └───────┘       │    │    │
│  │  │        │          │          │          │    │    │
│  │  │        └──────────┴──────────┘          │    │    │
│  │  │                   │                     │    │    │
│  │  │           GPU (RTX 3090)                │    │    │
│  │  └──────────────────┬──────────────────────┘    │    │
│  │                     │                           │    │
│  │           Orchestrator (AutoGen/LangGraph)      │    │
│  └─────────────────────┬───────────────────────────┘    │
│                        │                                │
│              ┌─────────┴─────────┐                      │
│              │  HTTP/WebSocket   │                      │
│              └─────────┬─────────┘                      │
│                        │                                │
│  ┌─────────────────────┴───────────────────────────┐    │
│  │              TAURI SHELL + REACT UI              │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ AI models are mathematical formulas with billions of "knobs"
- ✓ Larger models = smarter but need more GPU memory (VRAM)
- ✓ The RTX 3090's 24GB can run 7-13B models comfortably
- ✓ Quantization shrinks models to fit, with some quality loss
- ✓ Different tasks need different specialized models
- ✓ Models swap in/out of GPU memory as needed

**See Also:** [Section 5 - AI Model Strategy](#23-ai-model-strategy)

---

## 20.4 Multi-Model Orchestration Explained {#204-multi-model-orchestration-explained}

**Prerequisites:** Section 2.3 (AI Models)  
**Related to:** Section 6 (Multi-Agent Orchestration)  
**Implements:** Core AI collaboration approach  
**Read time:** ~5 minutes

**"Orchestration" means coordinating multiple AI models to work together on complex tasks—like a conductor directing an orchestra where each instrument (model) plays its part.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Agent** | An AI model with a specific job and the ability to take actions | Our AI assistants for different tasks |
| **Multi-Agent System** | Multiple AI "agents" working together | How we'll coordinate writing, coding, and image AI |
| **Orchestrator** | The "boss" code that decides which agent handles what | The Python backend that manages everything |
| **Task Routing** | Sending a request to the right AI model | "Summarize this" → text model; "Create diagram" → image model |
| **Lead/Worker Pattern** | A smart model makes the plan; simpler models execute it | GPT-4 plans, local model implements |

---

### Why Multiple Models?

═══ CORE CONCEPT ═══

> **No single AI model is best at everything.** Just like you wouldn't ask a novelist to debug your code, you shouldn't ask a writing model to generate images.
>
> - **Writing AI:** Excellent at prose, summaries, creative content
> - **Code AI:** Trained specifically on programming languages
> - **Image AI:** Completely different architecture, generates pixels not text
> - **Reasoning AI:** Better at logic, planning, breaking down complex tasks

---

### The Orchestra Analogy

```
┌─────────────────────────────────────────────────────────────┐
│                    THE ORCHESTRATOR                          │
│                    (The Conductor)                           │
│                          │                                   │
│    "Build me a project   │                                   │
│     management page"     │                                   │
│                          ▼                                   │
│         ┌────────────────────────────────┐                  │
│         │        TASK BREAKDOWN          │                  │
│         │ 1. Plan the page structure     │                  │
│         │ 2. Write the content           │                  │
│         │ 3. Generate header image       │                  │
│         │ 4. Create data schema          │                  │
│         └────────────────────────────────┘                  │
│                          │                                   │
│     ┌────────────────────┼────────────────────┐             │
│     ▼                    ▼                    ▼             │
│ ┌────────┐          ┌────────┐          ┌────────┐         │
│ │Reasoning│          │Writing │          │ Image  │         │
│ │  Model  │          │ Model  │          │ Model  │         │
│ │ (Plan)  │          │(Content)│         │(SDXL)  │         │
│ └────────┘          └────────┘          └────────┘         │
│     │                    │                    │             │
│     └────────────────────┴────────────────────┘             │
│                          │                                   │
│                          ▼                                   │
│              ┌─────────────────────┐                        │
│              │   COMBINED RESULT   │                        │
│              │ (Page with content, │                        │
│              │  schema, and image) │                        │
│              └─────────────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

---

### The Lead/Worker Pattern

This is the key pattern for making local AI practical:

```
┌─────────────────────────────────────────────────────────┐
│                    COMPLEX REQUEST                       │
│           "Write a blog post series on AI"              │
│                         │                                │
│                         ▼                                │
│  ┌─────────────────────────────────────────────────┐    │
│  │              LEAD MODEL (GPT-4 Cloud)            │    │
│  │                                                  │    │
│  │  "Here's the plan:                              │    │
│  │   Post 1: Introduction - 500 words              │    │
│  │   Post 2: History - 700 words                   │    │
│  │   Post 3: Future - 600 words                    │    │
│  │   Each should have..."                          │    │
│  │                                                  │    │
│  │  [Complex reasoning, one-time API cost]         │    │
│  └─────────────────────┬───────────────────────────┘    │
│                        │                                 │
│           ┌────────────┼────────────┐                   │
│           ▼            ▼            ▼                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐        │
│  │   WORKER    │ │   WORKER    │ │   WORKER    │        │
│  │  (Local 7B) │ │  (Local 7B) │ │  (Local 7B) │        │
│  │             │ │             │ │             │        │
│  │ Write Post 1│ │ Write Post 2│ │ Write Post 3│        │
│  │             │ │             │ │             │        │
│  │ [Free, fast,│ │ [Free, fast,│ │ [Free, fast,│        │
│  │  local]     │ │  local]     │ │  local]     │        │
│  └─────────────┘ └─────────────┘ └─────────────┘        │
└─────────────────────────────────────────────────────────┘
```

💡 **Tip:** The lead/worker pattern balances cost and quality. Use expensive cloud AI for the hard thinking (once), then cheap local AI for the bulk work.

---

### Key Takeaways

- ✓ Different AI models excel at different tasks
- ✓ An "orchestrator" coordinates which model handles what
- ✓ The lead/worker pattern: smart model plans, simple models execute
- ✓ This approach balances quality, cost, and speed
- ✓ All coordination happens in the Python backend

**See Also:** [Section 6 - Multi-Agent Orchestration](#24-multi-agent-orchestration)

---

# 21. Architecture Decisions {#21-architecture-decisions}

This section covers the major architectural choices for Project Handshake, based on research and multi-source analysis.

---

## 21.1 Desktop Shell: Tauri vs Electron {#211-desktop-shell-tauri-vs-electron}

**Prerequisites:** Section 2.1 (Desktop Shell concepts)  
**Related to:** Section 3.2 (Overall Architecture)  
**Implements:** Core technology choice  
**Read time:** ~7 minutes

**This section explains why Tauri was chosen over Electron as the desktop shell, based on consensus from multiple AI advisors and research documents.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Chromium** | The open-source browser that Chrome is built on | Electron bundles this; it's why Electron apps are large |
| **Rust** | A programming language focused on speed and safety | Tauri's backend is written in Rust |
| **System WebView** | The browser component already on your computer | Tauri uses this instead of bundling Chromium |
| **Binary Size** | How big the app installer is | Tauri: ~10-30MB; Electron: ~100-200MB |
| **Memory Footprint** | RAM used when app is running | Critical when AI models need that RAM |

---

### The Decision

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION POINT                            │
├─────────────────────────────────────────────────────────────┤
│ What needs to be decided: Desktop application shell          │
│                                                              │
│ Options researched:                                          │
│   • Electron (used by VS Code, Slack, Discord)              │
│   • Tauri (newer, used by some AI apps)                     │
│   • Flutter (AppFlowy uses this - different paradigm)       │
│                                                              │
│ Recommendation: TAURI                                        │
│                                                              │
│ Rationale:                                                   │
│   • 90% less memory usage (crucial for AI models)           │
│   • Smaller install size                                     │
│   • Better security model for plugins                        │
│   • Python backend means shell is "just a wrapper"          │
│                                                              │
│ Tradeoffs:                                                   │
│   • Smaller ecosystem than Electron                          │
│   • Rust knowledge needed for advanced shell features       │
│   • Some webview quirks across operating systems            │
│   • AFFiNE actually switched FROM Tauri TO Electron         │
└─────────────────────────────────────────────────────────────┘
```

---

### Head-to-Head Comparison

| Factor | Electron | Tauri | Winner for Handshake |
|--------|----------|-------|---------------------|
| **Memory at idle** | 150-300 MB | 10-50 MB | ⚡ **Tauri** |
| **Install size** | 100-200 MB | 10-30 MB | **Tauri** |
| **Startup time** | 1-2 seconds | Sub-second | **Tauri** |
| **Ecosystem maturity** | Excellent | Growing | Electron |
| **Documentation** | Extensive | Good | Electron |
| **Security model** | Permissive | Deny-by-default | ⚡ **Tauri** |
| **Cross-platform consistency** | Very consistent | Some quirks | Electron |
| **Node.js integration** | Built-in | Not applicable | Electron |
| **Rust backend** | Not applicable | Built-in | Context-dependent |

---

### Why Memory Matters So Much

═══ CORE CONCEPT ═══

```
Available GPU Memory (RTX 3090): 24 GB
───────────────────────────────────────────

WITH ELECTRON (300MB shell overhead):
┌────────────────────────────────────────────┐
│████████████████████████████░░░░░░░░░░░░░░░│
│  LLM Model (14GB)          │  SDXL(~8GB)  │
│                            │  Cramped!    │
└────────────────────────────────────────────┘
System RAM also constrained for model loading

WITH TAURI (30MB shell overhead):
┌────────────────────────────────────────────┐
│████████████████████████████████░░░░░░░░░░░│
│  LLM Model (14GB)          │  SDXL (10GB) │
│                            │  Comfortable │
└────────────────────────────────────────────┘
270MB more RAM available for models/context
```

---

### The Research Consensus

Three independent analyses (GPT-4, Claude, and Gemini) were asked to evaluate this decision. **All three recommended Tauri** for the following reasons:

📌 **Key Points from Multi-AI Analysis:**

1. **Resource Efficiency Under AI Load**
   > "Every megabyte of RAM you save in the shell is headroom for bigger models, more context windows, and smoother SDXL runs."

2. **Architecture Alignment**
   > "Your backend is Python, not Node. The hard logic is not written in Rust; it is in Python and TypeScript."

3. **Long-Term Product Vision**
   > "This is not a tiny helper tool; it is your primary local-first, multi-model AI workspace."

4. **Security for Plugins**
   > "Tauri has a stricter, deny-by-default permission model, which makes it safer to load third-party code."

---

### ⚠️ Risk: AFFiNE's Tauri-to-Electron Switch

One research document notes that AFFiNE, a similar local-first workspace app, **switched FROM Tauri BACK to Electron** due to webview limitations on macOS.

**Mitigation strategies:**
- Test extensively on all target platforms early
- Keep Tauri shell responsibilities minimal (just window management and IPC)
- Design the architecture so a shell swap is possible if absolutely necessary
- Monitor Tauri's development and webview improvements

---

### What Tauri Actually Does in This Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    TAURI'S RESPONSIBILITIES                  │
│                    (Keep this list SHORT)                    │
├─────────────────────────────────────────────────────────────┤
│  ✓ Create application window                                │
│  ✓ Load the React UI                                        │
│  ✓ Spawn Python backend process                             │
│  ✓ Handle file system access (with permissions)             │
│  ✓ Manage window state (minimize, maximize, etc.)           │
│  ✓ Surface system metrics (GPU usage, memory)               │
├─────────────────────────────────────────────────────────────┤
│                    NOT TAURI'S JOB                          │
├─────────────────────────────────────────────────────────────┤
│  ✗ AI orchestration (Python does this)                      │
│  ✗ Data processing (Python/TypeScript)                      │
│  ✗ Business logic (React/Python)                            │
│  ✗ Model management (Python backend)                        │
└─────────────────────────────────────────────────────────────┘
```

💡 **Tip:** Think of Tauri as a "thin wrapper"—it should do as little as possible. Complex logic stays in Python and TypeScript where iteration is easier.

---

### Key Takeaways

- ✓ **Decision: Use Tauri** as the desktop shell
- ✓ Primary reason: Memory efficiency for AI models
- ✓ Secondary reasons: Security model, smaller installs, faster startup
- ✓ Risk acknowledged: AFFiNE switched away; we mitigate by keeping Tauri's role minimal
- ✓ Frontend code (React/TypeScript) works identically in both shells
- ✓ If issues arise, shell swap is possible without rewriting business logic

**See Also:** [Section 3.2 - Overall System Architecture](#212-overall-system-architecture)

---

## 21.2 Overall System Architecture {#212-overall-system-architecture}

**Prerequisites:** Section 2.1-2.4 (Foundation Concepts), Section 3.1 (Tauri Decision)  
**Related to:** All implementation sections  
**Implements:** System blueprint  
**Read time:** ~8 minutes

**This section presents the complete system architecture: how all the pieces connect and communicate.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Frontend** | The part users see and interact with (buttons, text, etc.) | React/TypeScript in the Tauri window |
| **Backend** | The "behind the scenes" code that does heavy lifting | Python: AI, file processing, orchestration |
| **API** | A set of "commands" one program can send to another | How frontend talks to backend |
| **REST API** | A common style for APIs using web requests (GET, POST, etc.) | Simple, well-understood pattern |
| **WebSocket** | A persistent connection for real-time, two-way communication | For streaming AI responses |
| **Monorepo** | One repository containing multiple related projects | Frontend and backend code together |
| **Microservices** | Breaking an app into separate, independent services | Each AI model could be its own service |

---

### The Big Picture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           USER'S COMPUTER                                │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                        TAURI SHELL                               │    │
│  │  ┌───────────────────────────────────────────────────────────┐  │    │
│  │  │                    REACT FRONTEND                          │  │    │
│  │  │                                                            │  │    │
│  │  │   ┌──────────┐   ┌──────────┐   ┌──────────┐             │  │    │
│  │  │   │ Document │   │  Canvas  │   │  Sheets  │   ···       │  │    │
│  │  │   │  Editor  │   │  Board   │   │  Grid    │             │  │    │
│  │  │   │(Tiptap)  │   │(Excali)  │   │(Hyper)   │             │  │    │
│  │  │   └──────────┘   └──────────┘   └──────────┘             │  │    │
│  │  │                                                            │  │    │
│  │  │   ┌────────────────────────────────────────────────────┐  │  │    │
│  │  │   │              FILE TREE SIDEBAR                      │  │  │    │
│  │  │   │     (Workspace Navigator)                           │  │  │    │
│  │  │   └────────────────────────────────────────────────────┘  │  │    │
│  │  └────────────────────────────┬──────────────────────────────┘  │    │
│  └───────────────────────────────┼─────────────────────────────────┘    │
│                                  │                                       │
│                    HTTP/WebSocket (localhost)                           │
│                                  │                                       │
│  ┌───────────────────────────────┴─────────────────────────────────┐    │
│  │                      PYTHON BACKEND                              │    │
│  │                                                                  │    │
│  │   ┌────────────────────────────────────────────────────────┐    │    │
│  │   │                   ORCHESTRATOR                          │    │    │
│  │   │              (AutoGen or LangGraph)                     │    │    │
│  │   │                                                         │    │    │
│  │   │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐  │    │    │
│  │   │   │   Planner   │   │   Writer    │   │   Coder     │  │    │    │
│  │   │   │   Agent     │   │   Agent     │   │   Agent     │  │    │    │
│  │   │   └─────────────┘   └─────────────┘   └─────────────┘  │    │    │
│  │   └────────────────────────────┬───────────────────────────┘    │    │
│  │                                │                                 │    │
│  │   ┌────────────────────────────┴───────────────────────────┐    │    │
│  │   │                  MODEL RUNTIMES                         │    │    │
│  │   │                                                         │    │    │
│  │   │   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌───────┐  │    │    │
│  │   │   │  Ollama │   │  vLLM   │   │ComfyUI  │   │Cloud  │  │    │    │
│  │   │   │  (LLMs) │   │  (LLMs) │   │ (SDXL)  │   │Fallbck│  │    │    │
│  │   │   └─────────┘   └─────────┘   └─────────┘   └───────┘  │    │    │
│  │   └─────────────────────────────────────────────────────────┘    │    │
│  │                                │                                 │    │
│  └────────────────────────────────┼─────────────────────────────────┘    │
│                                   │                                      │
│  ┌────────────────────────────────┴─────────────────────────────────┐    │
│  │                     LOCAL FILE SYSTEM                             │    │
│  │                                                                   │    │
│  │   /Handshake/                                                    │    │
│  │   ├── workspaces/                                                │    │
│  │   │   └── my-project/                                           │    │
│  │   │       ├── notes/           (Markdown files)                 │    │
│  │   │       ├── canvas/          (JSON board data)                │    │
│  │   │       ├── sheets/          (CSV/JSON data)                  │    │
│  │   │       ├── images/          (Generated + uploaded)           │    │
│  │   │       └── .handshake/      (Metadata, CRDT state)          │    │
│  │   ├── models/                  (Downloaded AI models)           │    │
│  │   └── config/                  (User settings)                  │    │
│  └───────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│                         ┌───────────────────┐                           │
│                         │   OPTIONAL CLOUD   │                           │
│                         │  (Google Drive,    │                           │
│                         │   GPT-4 API, etc.) │                           │
│                         └───────────────────┘                           │
└─────────────────────────────────────────────────────────────────────────┘
```

---

### Architecture Pattern: Monorepo with Hybrid Processes

═══ CORE CONCEPT ═══

> **One codebase, multiple processes.** Everything lives in one Git repository, but runs as separate programs that communicate over the network.
>
> ```
> /handshake-repo/
> ├── ui/              # React/TypeScript frontend
> ├── backend/         # Python orchestrator + APIs  
> ├── shared/          # Type definitions, schemas
> └── docs/            # Documentation
> ```
>
> This gives us:
> - ✓ Unified versioning (frontend and backend always match)
> - ✓ Isolation (Python crash doesn't kill UI)
> - ✓ Flexibility (can restart backend without UI reload)

---

### Communication Flow

```
┌────────────────────────────────────────────────────────────────┐
│                    USER INTERACTION FLOW                        │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. User clicks "Summarize this document"                      │
│                        │                                        │
│                        ▼                                        │
│  2. React sends HTTP POST to localhost:8000/api/summarize      │
│     {                                                          │
│       "document_id": "abc123",                                 │
│       "style": "brief"                                         │
│     }                                                          │
│                        │                                        │
│                        ▼                                        │
│  3. Python backend receives, routes to orchestrator            │
│                        │                                        │
│                        ▼                                        │
│  4. Orchestrator picks model: local Llama 3 (13B)             │
│                        │                                        │
│                        ▼                                        │
│  5. Model generates summary, streaming via WebSocket           │
│                        │                                        │
│                        ▼                                        │
│  6. React displays streaming text to user                      │
│                        │                                        │
│                        ▼                                        │
│  7. Final result saved to document file                        │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

### Why Not Full Microservices?

```
┌─────────────────────────────────────────────────────────────────────┐
│                    DECISION POINT                                    │
├─────────────────────────────────────────────────────────────────────┤
│ What needs to be decided: How to structure backend services          │
│                                                                      │
│ Options researched:                                                  │
│   • Full microservices (each model in its own Docker container)     │
│   • Monolith (everything in one Python process)                     │
│   • Hybrid (multiple processes, no containers)                      │
│                                                                      │
│ Recommendation: HYBRID APPROACH                                      │
│                                                                      │
│ Rationale:                                                           │
│   • Full microservices adds Docker complexity                       │
│   • Monolith risks one crash killing everything                     │
│   • Hybrid: spawn Python processes for each service                 │
│                                                                      │
│ Implementation:                                                      │
│   • Main orchestrator process                                        │
│   • Model runtimes as separate processes (can restart independently)│
│   • Communication via localhost HTTP (simple, debuggable)           │
│                                                                      │
│ Tradeoffs:                                                           │
│   • Slightly more complex than monolith                             │
│   • Less isolated than Docker (shared filesystem)                   │
│   • Good balance for desktop app context                            │
└─────────────────────────────────────────────────────────────────────┘
```

---

### Startup Sequence

```
┌──────────────────────────────────────────────────────────────┐
│                    APP STARTUP SEQUENCE                       │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  1. User double-clicks Handshake.app                         │
│                        │                                      │
│                        ▼                                      │
│  2. Tauri shell starts                                       │
│     • Creates application window                             │
│     • Loads React frontend                                   │
│                        │                                      │
│                        ▼                                      │
│  3. Tauri spawns Python backend                              │
│     • python -m handshake.server                            │
│     • Backend starts on localhost:8000                       │
│                        │                                      │
│                        ▼                                      │
│  4. Backend initializes orchestrator                         │
│     • Loads model registry (what models are available)       │
│     • Does NOT load models yet (wait for demand)             │
│                        │                                      │
│                        ▼                                      │
│  5. Frontend polls /health endpoint                          │
│     • Shows "Loading..." until backend ready                 │
│     • Then displays workspace                                │
│                        │                                      │
│                        ▼                                      │
│  6. First AI request triggers model loading                  │
│     • Model loaded to GPU on first use                       │
│     • Subsequent requests are fast                           │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Three-layer architecture:** Tauri shell → React frontend → Python backend
- ✓ **Monorepo structure:** All code in one repository, easier to manage
- ✓ **Hybrid process model:** Multiple processes, no Docker complexity
- ✓ **File-tree based storage:** Human-readable, portable data
- ✓ **Lazy model loading:** Models load on first use, not at startup
- ✓ **Local-first with cloud options:** Works offline, syncs when available

**See Also:** [Section 3.3 - Data Architecture](#213-data-architecture-file-tree-model)

---

## 21.3 Data Architecture: File-Tree Model {#213-data-architecture-file-tree-model}

**Prerequisites:** Section 2.2 (Local-First), Section 3.2 (Overall Architecture)  
**Related to:** Section 7 (Collaboration and Sync)  
**Implements:** Data storage approach  
**Read time:** ~6 minutes

**Instead of a traditional database, Handshake stores data as files in folders—like how you organize documents on your computer, but structured for the application.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **File-Tree Architecture** | Using folders and files instead of a database | Data is human-readable, portable, git-friendly |
| **Workspace** | A project or collection of related documents | Top-level folder for a user's project |
| **Sidecar File** | A small file that travels with another file (like subtitles with a video) | Stores metadata without modifying original files |
| **SQLite** | A lightweight database in a single file | Used for indexing/search, not primary storage |
| **CRDT State** | The sync information stored alongside content | Enables conflict-free collaboration |

---

### Why Files Instead of a Database?

═══ CORE CONCEPT ═══

> **Your data should be yours, in formats you can read.**
>
> | Database Approach | File-Tree Approach |
> |-------------------|-------------------|
> | Data locked in app-specific format | Data in Markdown, JSON, CSV |
> | Need special tools to read | Open in any text editor |
> | Backup requires export | Copy folder = backup |
> | Hard to version control | Git works perfectly |
> | App dies = data access complex | App dies = files remain |

---

### The Folder Structure

```
/Handshake/
│
├── workspaces/                          # All user projects
│   │
│   ├── my-startup-project/              # One workspace
│   │   │
│   │   ├── notes/                       # Document editor content
│   │   │   ├── meeting-notes.md         # Markdown files
│   │   │   ├── product-spec.md
│   │   │   └── .meta/                   # Metadata sidecar
│   │   │       ├── meeting-notes.json   # Block IDs, timestamps
│   │   │       └── product-spec.json
│   │   │
│   │   ├── canvas/                      # Moodboard/canvas content
│   │   │   ├── brainstorm.json          # Board data
│   │   │   └── wireframes.json
│   │   │
│   │   ├── sheets/                      # Spreadsheet data
│   │   │   ├── budget.csv               # Actual data (portable!)
│   │   │   └── .meta/
│   │   │       └── budget.json          # Formulas, formatting
│   │   │
│   │   ├── databases/                   # Notion-style databases
│   │   │   ├── tasks.json               # Structured data
│   │   │   └── contacts.json
│   │   │
│   │   ├── images/                      # All images
│   │   │   ├── generated/               # AI-created
│   │   │   │   └── logo-v1.png
│   │   │   └── uploaded/                # User-added
│   │   │       └── reference.jpg
│   │   │
│   │   └── .handshake/                  # App-specific data
│   │       ├── workspace.json           # Settings, preferences
│   │       ├── crdt/                    # Sync state (if enabled)
│   │       │   └── sync-state.bin
│   │       └── index.db                 # SQLite search index
│   │
│   └── personal-notes/                  # Another workspace
│       └── ...
│
├── models/                              # Downloaded AI models
│   ├── llama-3-13b.gguf
│   ├── codellama-7b.gguf
│   └── sdxl-base.safetensors
│
└── config/                              # Global settings
    ├── settings.json
    ├── api-keys.encrypted               # Google OAuth, etc.
    └── model-registry.json              # What models are available
```

---

### File Formats by Content Type

| Content Type | Primary Format | Why This Format |
|-------------|----------------|-----------------|
| **Documents** | Markdown (.md) | Universal, readable, version-control friendly |
| **Canvas Boards** | JSON | Structured data, easy to parse |
| **Spreadsheets** | CSV + JSON sidecar | CSV = data (portable), JSON = formulas/formatting |
| **Databases** | JSON | Flexible schema, human-readable |
| **Images** | PNG/JPG + JSON sidecar | Standard formats, sidecar stores AI prompts |
| **Sync State** | Binary CRDT | Compact, efficient for sync algorithms |
| **Search Index** | SQLite | Fast full-text search |

---

### How AI-Generated Images Are Stored

```
/images/generated/
│
├── logo-v1.png                          # The actual image
│
└── logo-v1.json                         # Sidecar metadata
    {
      "generated_at": "2025-11-29T10:30:00Z",
      "model": "sdxl-1.0",
      "prompt": "minimalist tech startup logo, blue gradient",
      "negative_prompt": "text, watermark",
      "seed": 42,
      "steps": 30,
      "cfg_scale": 7.5,
      "workflow": "comfyui/basic-txt2img.json"
    }
```

💡 **Tip:** Storing generation parameters means you can recreate or tweak images later. The sidecar JSON acts like a "recipe" for the image.

---

### The Role of SQLite

⚠️ **Important:** SQLite is used for **indexing**, not as the primary data store.

```
┌─────────────────────────────────────────────────────────────┐
│                    DATA vs. INDEX                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  FILES (Source of Truth)           SQLite (Index/Cache)     │
│  ─────────────────────            ─────────────────────     │
│  • Markdown documents      ───►   • Full-text search        │
│  • JSON databases          ───►   • Tag lookups             │
│  • Canvas boards           ───►   • Quick queries           │
│  • Spreadsheets            ───►   • Recent files list       │
│                                                              │
│  If SQLite corrupts, rebuild from files.                    │
│  Files are authoritative; SQLite is derived.                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Files are the source of truth**, not a database
- ✓ Standard formats (Markdown, CSV, JSON) = portable, readable data
- ✓ Sidecar files store metadata without modifying originals
- ✓ SQLite used only for fast search/indexing
- ✓ Folder structure mirrors logical organization
- ✓ AI generation parameters stored for reproducibility

**See Also:** [Section 7 - Collaboration and Sync](#25-collaboration-and-sync)

---

# 22. User Interface Components {#22-user-interface-components}

This section covers the frontend UI components that make up the Handshake user experience, combining the best features of Notion, Milanote, and Excel.

---

## 22.1 Rich Text Editor (Notion-like) {#221-rich-text-editor-notion-like}

**Prerequisites:** Section 3.2 (Overall Architecture)  
**Related to:** Section 4.4 (Additional Views)  
**Implements:** Core document editing  
**Read time:** ~6 minutes

**The document editor is the heart of Handshake—a "block-based" editor where every paragraph, image, and element is a separate, movable piece.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Block-Based Editor** | Instead of one continuous document, content is made of stackable "blocks" (paragraphs, images, lists, etc.) | Enables drag/drop, AI operations on specific sections |
| **Tiptap** | A popular open-source editor framework built on ProseMirror | Leading candidate for our editor |
| **BlockNote** | A Notion-style block editor built on Tiptap | Pre-built Notion-like components |
| **Slash Commands** | Type "/" to get a menu of things to insert (like /heading, /image) | Familiar UX from Notion |
| **Real-Time Collaboration** | Multiple people editing the same document simultaneously | Requires CRDT integration |

---

### The Block Mental Model

```
┌─────────────────────────────────────────────────────────────┐
│              TRADITIONAL DOCUMENT                            │
│  ─────────────────────────────────────                      │
│  One continuous blob of formatted text                      │
│  that flows from top to bottom. Hard to                     │
│  rearrange, hard for AI to understand                       │
│  structure.                                                 │
└─────────────────────────────────────────────────────────────┘

                         vs.

┌─────────────────────────────────────────────────────────────┐
│              BLOCK-BASED DOCUMENT                            │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ BLOCK: Heading                                       │ ☰  │
│  │ "Project Overview"                                   │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ BLOCK: Paragraph                                     │ ☰  │
│  │ "This project aims to..."                           │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ BLOCK: AI-Generated Summary                         │ ☰  │
│  │ "Key points: 1) ... 2) ... 3) ..."                 │ 🤖 │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ BLOCK: Image                                        │ ☰  │
│  │ [diagram.png]                                       │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ☰ = Drag handle (reorder blocks)                          │
│  🤖 = AI-generated content indicator                        │
└─────────────────────────────────────────────────────────────┘
```

---

### Technology Choice

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION POINT                            │
├─────────────────────────────────────────────────────────────┤
│ What needs to be decided: Rich text editor framework         │
│                                                              │
│ Options researched:                                          │
│   • Tiptap/ProseMirror - Most extensible, proven            │
│   • BlockNote - Notion-style, built on Tiptap               │
│   • Lexical (Meta) - Newer, less collaboration support      │
│   • Slate.js - Flexible but complex                         │
│                                                              │
│ Recommendation: TIPTAP with BLOCKNOTE components             │
│                                                              │
│ Rationale:                                                   │
│   • BlockNote provides Notion-style blocks out of the box   │
│   • Tiptap is highly extensible for custom AI blocks        │
│   • Yjs integration available for real-time collaboration   │
│   • Large community and good documentation                   │
│                                                              │
│ Tradeoffs:                                                   │
│   • Some learning curve                                      │
│   • May need custom extensions for AI features              │
└─────────────────────────────────────────────────────────────┘
```

---

### Block Types to Implement

| Block Type | Priority | Description |
|------------|----------|-------------|
| **Paragraph** | [CORE] | Basic text |
| **Heading** | [CORE] | H1, H2, H3 |
| **List** | [CORE] | Bullet, numbered, checklist |
| **Image** | [CORE] | With AI generation capability |
| **Code** | [CORE] | Syntax highlighting |
| **Quote** | [CORE] | Blockquotes |
| **Divider** | [CORE] | Horizontal rule |
| **Table** | [OPTIONAL] | Basic tables |
| **Callout** | [OPTIONAL] | Colored highlight boxes |
| **Toggle** | [OPTIONAL] | Collapsible sections |
| **Embed** | [ADVANCED] | YouTube, tweets, etc. |
| **Database View** | [ADVANCED] | Inline Notion-style databases |
| **AI Block** | [CORE] | AI-generated content with indicators |

---

### AI Integration Points

```
┌─────────────────────────────────────────────────────────────┐
│              AI-ENHANCED EDITING                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  SLASH COMMAND MENU (type "/")                              │
│  ┌─────────────────────────────────┐                        │
│  │ / Basic                         │                        │
│  │   Paragraph, Heading, List...   │                        │
│  │                                 │                        │
│  │ / AI Actions ✨                 │                        │
│  │   📝 Generate text              │                        │
│  │   📋 Summarize above            │                        │
│  │   🔄 Rewrite selection          │                        │
│  │   🌐 Translate                  │                        │
│  │   🎨 Generate image             │                        │
│  │   💻 Generate code              │                        │
│  │   📊 Create table from text     │                        │
│  └─────────────────────────────────┘                        │
│                                                              │
│  CONTEXT MENU (select text, right-click)                    │
│  ┌─────────────────────────────────┐                        │
│  │ Improve writing                 │                        │
│  │ Make shorter                    │                        │
│  │ Make longer                     │                        │
│  │ Fix grammar                     │                        │
│  │ Explain this                    │                        │
│  │ Ask AI...                       │                        │
│  └─────────────────────────────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Block-based editing** enables flexible layouts and AI operations
- ✓ **Tiptap + BlockNote** is the recommended stack
- ✓ **Slash commands** provide quick access to AI features
- ✓ Blocks can be drag-and-dropped, nested, and reordered
- ✓ Real-time collaboration via Yjs integration

**See Also:** [Section 7.1 - Understanding CRDTs](#251-understanding-crdts)

---

## 22.2 Freeform Canvas (Milanote-like) {#222-freeform-canvas-milanote-like}

**Prerequisites:** Section 3.2 (Overall Architecture)  
**Related to:** Section 4.1 (Rich Text Editor)  
**Implements:** Visual brainstorming space  
**Read time:** ~5 minutes

**The canvas is an infinite whiteboard where you can drag notes, images, and shapes anywhere—like a digital corkboard for visual thinkers.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Infinite Canvas** | A workspace that extends forever in all directions | No page boundaries, unlimited space |
| **Excalidraw** | Popular open-source whiteboard with hand-drawn look | Leading candidate for our canvas |
| **React-Konva** | Library for drawing graphics in React | Alternative for custom canvas needs |
| **Pan & Zoom** | Moving around and magnifying the canvas | Essential for large boards |

---

### The Canvas vs. Document Distinction

```
┌─────────────────────────────────────────────────────────────┐
│                    DOCUMENT EDITOR                           │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                                                     │    │
│  │  Text flows top-to-bottom                          │    │
│  │                                                     │    │
│  │  Linear structure                                  │    │
│  │                                                     │    │
│  │  Like a Word document or web page                  │    │
│  │                                                     │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  BEST FOR: Writing, documentation, structured content       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    CANVAS BOARD                              │
│                                                              │
│      ┌───────┐                      ┌───────┐               │
│      │ Note  │──────────────────────│ Image │               │
│      └───────┘                      └───────┘               │
│            \                                                │
│             \     ┌───────────┐                             │
│              ─────│ Idea Box  │                             │
│                   └───────────┘                             │
│                         │                                    │
│    ┌───────┐           │                                    │
│    │Sketch │───────────┘          ┌───────────┐            │
│    └───────┘                      │ Reference │            │
│                                    └───────────┘            │
│                                                              │
│  BEST FOR: Brainstorming, mood boards, spatial thinking     │
└─────────────────────────────────────────────────────────────┘
```

---

### Technology Choice

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION POINT                            │
├─────────────────────────────────────────────────────────────┤
│ What needs to be decided: Canvas/whiteboard library          │
│                                                              │
│ Options researched:                                          │
│   • Excalidraw - Mature, MIT-licensed, hand-drawn feel      │
│   • tldraw - Modern, React-focused, good collaboration      │
│   • React-Konva - Low-level, full control                   │
│   • Fabric.js - Canvas library, more work to integrate      │
│                                                              │
│ Recommendation: EXCALIDRAW                                   │
│                                                              │
│ Rationale:                                                   │
│   • Production-proven (used by many products)               │
│   • Built-in collaboration support                          │
│   • Familiar "whiteboard" UX                                │
│   • Can embed in React easily                               │
│                                                              │
│ Tradeoffs:                                                   │
│   • "Hand-drawn" aesthetic may not fit all use cases        │
│   • May need customization for Milanote-style features      │
└─────────────────────────────────────────────────────────────┘
```

---

### Canvas Element Types

| Element | Priority | Description |
|---------|----------|-------------|
| **Sticky Note** | [CORE] | Text cards that can be moved |
| **Image** | [CORE] | Photos, generated images |
| **Shape** | [CORE] | Rectangles, circles, arrows |
| **Line/Arrow** | [CORE] | Connect elements |
| **Text** | [CORE] | Freestanding labels |
| **Drawing** | [OPTIONAL] | Freehand sketching |
| **Frame/Group** | [OPTIONAL] | Organize related items |
| **Embedded Note** | [ADVANCED] | Link to full document |
| **AI Image Generation** | [CORE] | Generate images directly on canvas |

---

### AI Integration for Canvas

```
┌─────────────────────────────────────────────────────────────┐
│              AI-ENHANCED CANVAS                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  RIGHT-CLICK ON CANVAS:                                     │
│  ┌─────────────────────────────────┐                        │
│  │ 🎨 Generate image here...       │                        │
│  │ 📝 Add AI note about...         │                        │
│  │ 💡 Brainstorm ideas about...    │                        │
│  └─────────────────────────────────┘                        │
│                                                              │
│  SELECT MULTIPLE ITEMS:                                     │
│  ┌─────────────────────────────────┐                        │
│  │ 📋 Summarize selected items     │                        │
│  │ 🔗 Find connections             │                        │
│  │ 📊 Organize into categories     │                        │
│  └─────────────────────────────────┘                        │
│                                                              │
│  DRAG IMAGE ONTO CANVAS:                                    │
│  ┌─────────────────────────────────┐                        │
│  │ 🔍 Describe this image          │                        │
│  │ 🎨 Generate variations          │                        │
│  │ ✂️ Remove background             │                        │
│  └─────────────────────────────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Canvas = spatial thinking**, Documents = linear thinking
- ✓ **Excalidraw** is the recommended library
- ✓ Supports infinite pan/zoom, drag-and-drop
- ✓ AI can generate images directly onto canvas
- ✓ Works alongside (not replacing) the document editor

---

## 22.3 Spreadsheet Engine (Excel-like) {#223-spreadsheet-engine-excel-like}

**Prerequisites:** Section 3.2 (Overall Architecture)  
**Related to:** Section 4.1 (Rich Text Editor)  
**Implements:** Data manipulation capabilities  
**Read time:** ~5 minutes

**Spreadsheets let you organize data in rows and columns with formulas—essential for budgets, project tracking, and any structured data work.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Cell** | One box in the grid (like A1, B2) | The basic unit of spreadsheets |
| **Formula** | An equation that calculates a value (=SUM(A1:A10)) | What makes spreadsheets powerful |
| **HyperFormula** | Open-source formula engine with 400+ functions | The "brain" that calculates formulas |
| **Data Grid** | UI component for displaying/editing cell tables | What the user sees and interacts with |
| **Handsontable** | Popular JavaScript spreadsheet grid | One option for the UI layer |

---

### The Separation: UI vs. Engine

═══ CORE CONCEPT ═══

> **Two separate pieces work together:**
>
> 1. **Data Grid (UI)** - What you see: cells, scrolling, selection, editing
> 2. **Formula Engine** - The math: calculating formulas, dependencies
>
> ```
> ┌─────────────────────────────────────────────────────────┐
> │                    USER TYPES: =SUM(A1:A3)              │
> │                           │                              │
> │                           ▼                              │
> │  ┌─────────────────────────────────────────────────┐    │
> │  │              DATA GRID (Handsontable)           │    │
> │  │  "User typed something in B1"                   │    │
> │  └─────────────────────────────────────────────────┘    │
> │                           │                              │
> │                           ▼                              │
> │  ┌─────────────────────────────────────────────────┐    │
> │  │            FORMULA ENGINE (HyperFormula)        │    │
> │  │  "=SUM(A1:A3) equals 150"                       │    │
> │  └─────────────────────────────────────────────────┘    │
> │                           │                              │
> │                           ▼                              │
> │  ┌─────────────────────────────────────────────────┐    │
> │  │              DATA GRID (Handsontable)           │    │
> │  │  "Display 150 in cell B1"                       │    │
> │  └─────────────────────────────────────────────────┘    │
> └─────────────────────────────────────────────────────────┘
> ```

---

### Technology Choice

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION POINT                            │
├─────────────────────────────────────────────────────────────┤
│ What needs to be decided: Spreadsheet implementation         │
│                                                              │
│ Options researched:                                          │
│   Grid UI:                                                   │
│   • Handsontable - Feature-rich, some license concerns      │
│   • AG Grid - Professional, complex                         │
│   • Wolf-Table (x-spreadsheet) - Lightweight                │
│                                                              │
│   Formula Engine:                                            │
│   • HyperFormula - 400+ functions, open source              │
│                                                              │
│ Recommendation: WOLF-TABLE + HYPERFORMULA                    │
│                                                              │
│ Rationale:                                                   │
│   • HyperFormula is clearly the best formula engine         │
│   • Wolf-Table is lightweight and MIT-licensed              │
│   • Combination gives Excel-like functionality              │
│                                                              │
│ Tradeoffs:                                                   │
│   • Less polished than Handsontable out-of-box             │
│   • May need more custom work for advanced features         │
└─────────────────────────────────────────────────────────────┘
```

---

### Feature Scope

| Feature | Priority | Notes |
|---------|----------|-------|
| **Basic cells** | [CORE] | Text, numbers, dates |
| **Formulas** | [CORE] | HyperFormula's 400+ functions |
| **Cell formatting** | [CORE] | Bold, colors, alignment |
| **Copy/paste** | [CORE] | Including from Excel |
| **Sorting/filtering** | [CORE] | Column operations |
| **Multiple sheets** | [OPTIONAL] | Tabs within workbook |
| **Charts** | [OPTIONAL] | Basic visualizations |
| **Pivot tables** | [ADVANCED] | Data summarization |
| **Scripts/macros** | [ADVANCED] | Automation |

---

### AI Integration for Spreadsheets

```
┌─────────────────────────────────────────────────────────────┐
│              AI-ENHANCED SPREADSHEETS                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  SELECT CELLS, RIGHT-CLICK:                                 │
│  ┌─────────────────────────────────┐                        │
│  │ 📊 Analyze this data            │                        │
│  │ 📝 Explain this formula         │                        │
│  │ 🔧 Fix this formula             │                        │
│  │ 📈 Suggest visualizations       │                        │
│  └─────────────────────────────────┘                        │
│                                                              │
│  NATURAL LANGUAGE FORMULAS:                                 │
│  ┌─────────────────────────────────┐                        │
│  │ User types: "total of column A" │                        │
│  │ AI suggests: =SUM(A:A)          │                        │
│  └─────────────────────────────────┘                        │
│                                                              │
│  DATA GENERATION:                                           │
│  ┌─────────────────────────────────┐                        │
│  │ "Fill with sample customer data"│                        │
│  │ AI generates realistic test data│                        │
│  └─────────────────────────────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Two components:** Data Grid (UI) + Formula Engine (HyperFormula)
- ✓ **HyperFormula** provides Excel-compatible formulas
- ✓ Data stored as CSV (portable) with JSON sidecar for formatting
- ✓ AI can help write formulas and analyze data
- ✓ Start simple, add advanced features later

---

## 22.4 Additional Views: Kanban, Calendar, Timeline {#224-additional-views-kanban-calendar-timeline}

**Prerequisites:** Section 4.1 (Rich Text Editor), Section 4.3 (Spreadsheets)  
**Related to:** Section 3.3 (Data Architecture)  
**Implements:** Notion-style database views  
**Read time:** ~4 minutes

**The same data can be viewed different ways: as a table, as Kanban cards, as calendar events, or as a timeline.**

---

### The "Views" Concept

═══ CORE CONCEPT ═══

> **One dataset, many presentations.** A list of tasks can be:
> - A **table** (spreadsheet-style rows)
> - A **Kanban board** (cards in columns like "To Do", "In Progress", "Done")
> - A **calendar** (if tasks have dates)
> - A **timeline/Gantt** (showing duration and dependencies)
>
> The underlying data is identical; only the visualization changes.

```
┌─────────────────────────────────────────────────────────────┐
│                    SAME DATA, DIFFERENT VIEWS                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  DATABASE: Tasks                                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ID │ Title        │ Status      │ Due Date │ Owner   │   │
│  │ 1  │ Design logo  │ In Progress │ Dec 1    │ Alice   │   │
│  │ 2  │ Write copy   │ To Do       │ Dec 3    │ Bob     │   │
│  │ 3  │ Launch site  │ To Do       │ Dec 10   │ Alice   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│           │                    │                    │        │
│           ▼                    ▼                    ▼        │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   TABLE      │  │   KANBAN     │  │   CALENDAR   │       │
│  │   VIEW       │  │   VIEW       │  │   VIEW       │       │
│  │              │  │              │  │              │       │
│  │ Spreadsheet  │  │ To Do │ In   │  │    Dec       │       │
│  │ style rows   │  │       │Progr │  │ 1 [Design]   │       │
│  │              │  │ [Copy]│[Logo]│  │ 3 [Copy]     │       │
│  │              │  │ [Site]│      │  │ 10 [Launch]  │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Implementation Priority

| View Type | Priority | Library Options |
|-----------|----------|-----------------|
| **Table** | [CORE] | AG Grid, React Table |
| **Kanban** | [CORE] | react-beautiful-dnd, dnd-kit |
| **Calendar** | [OPTIONAL] | FullCalendar, react-big-calendar |
| **Timeline/Gantt** | [ADVANCED] | frappe-gantt, custom |
| **Gallery** | [OPTIONAL] | Custom grid layout |

---

### Key Takeaways

- ✓ Views are different visualizations of the same data
- ✓ Kanban is high priority (project management is a key use case)
- ✓ Start with Table and Kanban, add Calendar later
- ✓ Database structure stored in JSON files

---

# 23. AI Model Strategy {#23-ai-model-strategy}

This section details which AI models to use, how to run them locally, and when to fall back to cloud services.

---

## 23.1 Model Categories and Recommendations {#231-model-categories-and-recommendations}

**Prerequisites:** Section 2.3 (AI Models Basics)  
**Related to:** Section 5.2 (Local Model Runtimes)  
**Implements:** AI model selection  
**Read time:** ~7 minutes

**Different tasks need different AI models. This section recommends specific models for each task type based on the research.**

---

### Model Categories Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    AI MODELS BY TASK TYPE                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐  │
│  │ GENERAL LLM     │    │ CODE MODEL      │    │ IMAGE MODEL     │  │
│  │                 │    │                 │    │                 │  │
│  │ Writing         │    │ Code generation │    │ Image generation│  │
│  │ Summarizing     │    │ Code completion │    │ Image editing   │  │
│  │ Q&A             │    │ Bug fixing      │    │ Style transfer  │  │
│  │ Translation     │    │ Code review     │    │                 │  │
│  │                 │    │                 │    │                 │  │
│  │ Llama 3         │    │ Code Llama      │    │ SDXL            │  │
│  │ Mistral         │    │ StarCoder       │    │ Stable Diffusion│  │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘  │
│                                                                      │
│  ┌─────────────────┐    ┌─────────────────┐                         │
│  │ REASONING/      │    │ CREATIVE        │                         │
│  │ PLANNING        │    │ WRITING         │                         │
│  │                 │    │                 │                         │
│  │ Task breakdown  │    │ Fiction         │                         │
│  │ Decision making │    │ Storytelling    │                         │
│  │ Multi-step      │    │ Brainstorming   │                         │
│  │ planning        │    │                 │                         │
│  │                 │    │                 │                         │
│  │ GPT-OSS-20B     │    │ NeuralStar      │                         │
│  │ DeepSeek        │    │ 4x7B MoE        │                         │
│  └─────────────────┘    └─────────────────┘                         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

### Specific Model Recommendations

Based on the research, here are recommended models for each category:

#### General Writing & Reasoning

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **Llama 3 13B** | 13B | ~14GB (Q4) | Balanced quality/speed | Default text tasks |
| **Mistral 7B** | 7B | ~8GB (Q4) | Fast, efficient | Quick responses |
| **GPT-OSS-20B** | 20B | ~16GB | Strong reasoning | Complex planning |

📌 **Recommendation:** Start with **Llama 3 13B** as the default general model. Use Mistral 7B for fast, simple tasks.

---

#### Code Generation

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **Code Llama 13B** | 13B | ~14GB (Q4) | Multi-language | Primary code model |
| **Code Llama 7B** | 7B | ~7GB (Q4) | Fast completion | Autocomplete |
| **StarCoder 15B** | 15B | ~15GB | Broad language support | Alternative |

📌 **Recommendation:** **Code Llama 13B** for code generation, 7B variant for real-time autocomplete.

---

#### Image Generation

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **SDXL 1.0** | ~3B | ~10GB | Best quality | Primary image gen |
| **SD 1.5** | ~1B | ~4GB | Faster, lighter | Quick drafts |

📌 **Recommendation:** **SDXL 1.0** via ComfyUI for quality image generation.

---

#### Creative Writing (Specialized)

| Model | Size | VRAM Needed | Strengths | Use For |
|-------|------|-------------|-----------|---------|
| **NeuralStar AlphaWriter 4x7B** | 24B MoE | ~20GB (Q4) | Fiction-tuned | Stories, creative |

─── Nice to Know ───

> **MoE (Mixture of Experts)** means the model has multiple "expert" sub-models inside. Only some experts activate for each request, making it more efficient than a dense 24B model.

---

### Memory Budget Planning

═══ CORE CONCEPT ═══

> **You can't run all models at once.** With 24GB VRAM on an RTX 3090, plan which models are loaded when:

```
┌─────────────────────────────────────────────────────────────┐
│                    VRAM BUDGET (24GB RTX 3090)               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  SCENARIO A: Text-focused work                              │
│  ┌────────────────────────────────────────────────────┐     │
│  │████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│     │
│  │   Llama 3 13B (14GB)        │     Free (10GB)    │     │
│  └────────────────────────────────────────────────────┘     │
│                                                              │
│  SCENARIO B: Image generation                               │
│  ┌────────────────────────────────────────────────────┐     │
│  │██████████████████████████████░░░░░░░░░░░░░░░░░░░░│     │
│  │         SDXL (10GB)         │ Mistral 7B │ Free  │     │
│  └────────────────────────────────────────────────────┘     │
│                                                              │
│  SCENARIO C: Code + Chat                                    │
│  ┌────────────────────────────────────────────────────┐     │
│  │██████████████████████████████████░░░░░░░░░░░░░░░░│     │
│  │   Code Llama 13B    │   Mistral 7B   │   Free    │     │
│  └────────────────────────────────────────────────────┘     │
│                                                              │
│  ⚡ Models swap in/out based on task                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Llama 3 13B** is the recommended default general model
- ✓ **Code Llama 13B** for code tasks, 7B for autocomplete
- ✓ **SDXL 1.0** via ComfyUI for image generation
- ✓ Models swap in/out of VRAM based on current task
- ✓ The 24GB RTX 3090 can handle most scenarios with smart scheduling

---

## 23.2 Local Model Runtimes {#232-local-model-runtimes}

**Prerequisites:** Section 5.1 (Model Categories)  
**Related to:** Section 3.2 (Overall Architecture)  
**Implements:** How models actually run  
**Read time:** ~5 minutes

**A "runtime" is the software that loads AI models and runs them. Different runtimes have different strengths.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Ollama** | Easy-to-use model runner, like "Docker for AI models" | Simplest way to run local LLMs |
| **vLLM** | High-performance model server from Berkeley | Best for production, supports batching |
| **llama.cpp** | Efficient CPU/GPU inference, uses GGUF format | Most flexible for quantized models |
| **ComfyUI** | Node-based UI for Stable Diffusion | Best for image generation workflows |
| **TGI** | HuggingFace's text generation server | Alternative to vLLM |

---

### Runtime Comparison

| Runtime | Ease of Use | Performance | Flexibility | Best For |
|---------|-------------|-------------|-------------|----------|
| **Ollama** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | Quick start, development |
| **vLLM** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Production, high throughput |
| **llama.cpp** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Custom setups, edge cases |
| **ComfyUI** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Image generation (required) |

---

### Recommended Setup

```
┌─────────────────────────────────────────────────────────────┐
│                    RUNTIME ARCHITECTURE                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              PYTHON ORCHESTRATOR                     │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│          ┌────────────────┼────────────────┐                │
│          ▼                ▼                ▼                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   OLLAMA    │  │   COMFYUI   │  │   CLOUD     │         │
│  │             │  │             │  │   APIs      │         │
│  │ • Llama 3   │  │ • SDXL      │  │ • GPT-4     │         │
│  │ • Mistral   │  │ • SD 1.5    │  │ • Claude    │         │
│  │ • CodeLlama │  │ • Workflows │  │ (fallback)  │         │
│  │             │  │             │  │             │         │
│  │ Port: 11434 │  │ Port: 8188  │  │ HTTPS       │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│                                                              │
│  Development: Start with Ollama (easiest)                   │
│  Production: Consider vLLM for better performance           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Ollama** for LLMs (easiest to set up and manage)
- ✓ **ComfyUI** for image generation (required for SDXL workflows)
- ✓ Cloud APIs as fallback for complex tasks
- ✓ All runtimes expose HTTP APIs that the Python orchestrator calls

---

## 23.3 Cloud Fallback Strategy {#233-cloud-fallback-strategy}

**Prerequisites:** Section 5.1-5.2 (Models and Runtimes)  
**Related to:** Section 6.2 (Lead/Worker Pattern)  
**Implements:** Handling tasks too hard for local models  
**Read time:** ~4 minutes

**When local models aren't enough, fall back to powerful cloud APIs—but do it strategically to minimize cost.**

---

### When to Use Cloud

| Use Cloud When | Why |
|---------------|-----|
| Local model fails/low confidence | Quality matters |
| Task requires 100K+ context | Local models limited to 4-32K |
| Complex multi-step reasoning | Cloud models more capable |
| User explicitly requests "best" | Preference for quality over speed |

| Stay Local When | Why |
|----------------|-----|
| Simple summarization | Local handles fine |
| Basic Q&A about document | Fast and free |
| Code completion | Real-time speed needed |
| Privacy-sensitive content | Data stays local |

---

### Cost-Aware Routing

```
┌─────────────────────────────────────────────────────────────┐
│                    INTELLIGENT ROUTING                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  USER REQUEST: "Write a marketing strategy"                 │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              COMPLEXITY ANALYSIS                     │    │
│  │                                                      │    │
│  │  Length estimate: ~2000 words                       │    │
│  │  Reasoning required: High                           │    │
│  │  Domain knowledge: Marketing (general)              │    │
│  │                                                      │    │
│  │  ⚡ DECISION: Use Lead/Worker pattern               │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│           ┌───────────────┴───────────────┐                 │
│           ▼                               ▼                 │
│  ┌─────────────────┐            ┌─────────────────┐        │
│  │ CLOUD (GPT-4)   │            │ LOCAL (Llama)   │        │
│  │                 │            │                 │        │
│  │ Create outline  │───────────▶│ Write sections  │        │
│  │ and strategy    │            │ based on        │        │
│  │ framework       │            │ outline         │        │
│  │                 │            │                 │        │
│  │ Cost: ~$0.10    │            │ Cost: $0.00     │        │
│  │ (one-time)      │            │ (unlimited)     │        │
│  └─────────────────┘            └─────────────────┘        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ Cloud APIs for planning and complex reasoning (paid but smart)
- ✓ Local models for execution and bulk work (free)
- ✓ Automatic fallback when local quality is insufficient
- ✓ User can override to force local or cloud

---

## 23.4 Image Generation with ComfyUI {#234-image-generation-with-comfyui}

**Prerequisites:** Section 5.1 (Model Categories)  
**Related to:** Section 4.2 (Canvas)  
**Implements:** AI image generation  
**Read time:** ~5 minutes

**ComfyUI is a node-based tool for creating images with AI. Instead of just typing a prompt, you can build complex image processing pipelines.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **ComfyUI** | Visual tool for building AI image generation workflows | Our image generation backend |
| **Workflow** | A saved pipeline of image processing steps | Can be triggered programmatically |
| **Node** | One step in the pipeline (like "load model" or "apply style") | Building blocks of workflows |
| **Checkpoint** | A saved AI model file | SDXL base, custom fine-tunes |
| **ControlNet** | Guides image generation with poses, edges, etc. | Advanced control over output |

---

### Why ComfyUI?

═══ CORE CONCEPT ═══

> ComfyUI workflows are **saved as JSON** and can be **triggered via API**. This means:
> 1. Design complex pipelines visually
> 2. Save them as templates
> 3. Trigger from Handshake with different prompts
> 4. Receive generated images back

```
┌─────────────────────────────────────────────────────────────┐
│                    COMFYUI INTEGRATION                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  USER IN HANDSHAKE                                          │
│  "Generate a logo for my startup"                           │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              PYTHON ORCHESTRATOR                     │    │
│  │                                                      │    │
│  │  1. Pick workflow: "logo_generation.json"           │    │
│  │  2. Insert prompt into workflow                     │    │
│  │  3. POST to ComfyUI API                             │    │
│  │  4. Poll for completion                             │    │
│  │  5. Retrieve generated image                        │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              COMFYUI (localhost:8188)                │    │
│  │                                                      │    │
│  │  [Load SDXL]──▶[CLIP Encode]──▶[KSampler]──▶[Save] │    │
│  │                                                      │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  IMAGE RETURNED + SAVED WITH METADATA                       │
│  (prompt, seed, settings stored in sidecar JSON)           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Pre-Built Workflows to Create

| Workflow | Description | Use Case |
|----------|-------------|----------|
| **txt2img_basic** | Simple text to image | Quick generations |
| **txt2img_quality** | High quality with refiner | Final outputs |
| **img2img** | Modify existing image | Variations |
| **inpaint** | Edit parts of image | Touch-ups |
| **upscale** | Increase resolution | Print-ready |

---

### Key Takeaways

- ✓ **ComfyUI** runs as a separate service, controlled via API
- ✓ Workflows are JSON files that can be version controlled
- ✓ Generated images stored with full metadata for reproducibility
- ✓ Can build progressively complex workflows over time

---

# 24. Multi-Agent Orchestration {#24-multi-agent-orchestration}

This section covers how multiple AI models coordinate to accomplish complex tasks.

---

## 24.1 Framework Comparison: AutoGen vs LangGraph vs CrewAI {#241-framework-comparison-autogen-vs-langgraph-vs-crewai}

**Prerequisites:** Section 2.4 (Multi-Model Orchestration)  
**Related to:** Section 5 (AI Model Strategy)  
**Implements:** Orchestration framework choice  
**Read time:** ~6 minutes

**Orchestration frameworks help coordinate multiple AI agents. Each framework has a different approach and strengths.**

---

### Framework Philosophies

```
┌─────────────────────────────────────────────────────────────┐
│              THREE APPROACHES TO ORCHESTRATION               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  AUTOGEN (Microsoft)                                        │
│  Philosophy: Agents CONVERSE with each other                │
│                                                              │
│       Agent A ◄────────────────────► Agent B                │
│          │                              │                    │
│          └──────────► Agent C ◄─────────┘                   │
│                                                              │
│  Like: A meeting where experts discuss until done           │
│  Best for: Complex reasoning, human-in-loop                 │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  LANGGRAPH (LangChain)                                      │
│  Philosophy: Tasks flow through a GRAPH of steps            │
│                                                              │
│       [Start]──▶[Plan]──▶[Execute]──▶[Review]──▶[End]      │
│                    │                    │                    │
│                    └──────◄─────────────┘                   │
│                         (if review fails)                   │
│                                                              │
│  Like: A flowchart where you define exactly what happens    │
│  Best for: Predictable workflows, complex conditionals      │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  CREWAI                                                     │
│  Philosophy: Agents have ROLES and work in SEQUENCE         │
│                                                              │
│       [Researcher]──▶[Writer]──▶[Editor]──▶[Publisher]     │
│                                                              │
│  Like: An assembly line with specialists                    │
│  Best for: Simple, linear workflows                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Detailed Comparison

| Aspect | AutoGen | LangGraph | CrewAI |
|--------|---------|-----------|--------|
| **Learning Curve** | Medium | High | Low |
| **Flexibility** | High | Very High | Medium |
| **Debugging** | Conversation logs | Visual graph | Role inspection |
| **Human-in-Loop** | Excellent | Good | Limited |
| **Complex Branching** | Good | Excellent | Limited |
| **Setup Effort** | Medium | Higher | Low |
| **Documentation** | Good | Good | Growing |
| **Local-First** | Yes | Yes | Yes |

---

### Decision Point

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION POINT                            │
├─────────────────────────────────────────────────────────────┤
│ What needs to be decided: Multi-agent orchestration framework│
│                                                              │
│ Options researched:                                          │
│   • AutoGen - Conversational agents, Microsoft-backed        │
│   • LangGraph - Graph-based workflows, very flexible         │
│   • CrewAI - Simple role-based pipelines                    │
│                                                              │
│ Recommendation: START WITH AUTOGEN, consider LangGraph      │
│                                                              │
│ Rationale:                                                   │
│   • AutoGen balances power and approachability              │
│   • Good human-in-loop support (important for AI trust)     │
│   • Microsoft backing suggests long-term maintenance        │
│   • Can migrate to LangGraph if more control needed         │
│                                                              │
│ Tradeoffs:                                                   │
│   • Less explicit flow control than LangGraph               │
│   • Conversation logging can be verbose                     │
│   • May need custom work for complex branching              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **AutoGen** recommended for initial development
- ✓ **LangGraph** as alternative if explicit flow control needed
- ✓ **CrewAI** too limited for complex Handshake workflows
- ✓ All frameworks run locally with any LLM

---

## 24.2 The Lead/Worker Pattern {#242-the-leadworker-pattern}

**Prerequisites:** Section 6.1 (Framework Comparison)  
**Related to:** Section 5.3 (Cloud Fallback)  
**Implements:** Cost-effective multi-model approach  
**Read time:** ~4 minutes

**Use a powerful model to PLAN, then cheaper models to EXECUTE. This balances quality and cost.**

---

### The Pattern Explained

═══ CORE CONCEPT ═══

```
┌─────────────────────────────────────────────────────────────┐
│                    LEAD/WORKER PATTERN                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  COMPLEX TASK: "Create a product launch plan with           │
│                 marketing copy and social media posts"      │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              LEAD (GPT-4 Cloud)                      │    │
│  │                                                      │    │
│  │  "Here's the plan:                                  │    │
│  │   1. Executive summary (100 words)                  │    │
│  │   2. Target audience analysis                       │    │
│  │   3. Key messaging (3 bullet points)                │    │
│  │   4. Timeline with milestones                       │    │
│  │   5. Social posts: Twitter (3), LinkedIn (2)        │    │
│  │                                                      │    │
│  │   Each section should follow format X..."           │    │
│  │                                                      │    │
│  │  Cost: $0.15 (one complex reasoning call)           │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │           WORKERS (Local Llama 3 13B)               │    │
│  │                                                      │    │
│  │  Task 1: Write executive summary ─────▶ Done        │    │
│  │  Task 2: Write audience analysis ─────▶ Done        │    │
│  │  Task 3: Write key messaging ─────────▶ Done        │    │
│  │  Task 4: Create timeline ─────────────▶ Done        │    │
│  │  Task 5: Write social posts ──────────▶ Done        │    │
│  │                                                      │    │
│  │  Cost: $0.00 (local, unlimited)                     │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  TOTAL COST: ~$0.15 instead of ~$1.50+ if all cloud        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ Smart model plans, simple model executes
- ✓ Reduces cloud API costs by 90%+
- ✓ Local execution is fast and private
- ✓ Fall back to cloud lead if local worker fails

---

## 24.3 Shared Context and Memory {#243-shared-context-and-memory}

**Prerequisites:** Section 6.1-6.2 (Orchestration basics)  
**Related to:** Section 3.3 (Data Architecture)  
**Implements:** How agents share information  
**Read time:** ~4 minutes

**Agents need to share information. A "shared memory" system ensures one agent's output is available to others.**

---

### Memory Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    SHARED MEMORY SYSTEM                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              SHARED CONTEXT STORE                    │    │
│  │                                                      │    │
│  │  • Conversation history (all agents)                │    │
│  │  • Working documents (current task files)           │    │
│  │  • User preferences and context                     │    │
│  │  • Retrieved knowledge (RAG results)                │    │
│  │                                                      │    │
│  └─────────────────────────────────────────────────────┘    │
│         │              │              │                      │
│         ▼              ▼              ▼                      │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐                │
│  │ Planner  │   │ Writer   │   │ Reviewer │                │
│  │  Agent   │   │  Agent   │   │  Agent   │                │
│  └──────────┘   └──────────┘   └──────────┘                │
│                                                              │
│  STORAGE OPTIONS:                                           │
│  • File system (matches our data architecture)             │
│  • SQLite for structured queries                           │
│  • Vector store for semantic search (ChromaDB/FAISS)       │
│  • Redis/ZeroMQ for real-time passing                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ Agents share context through a central store
- ✓ File-based storage aligns with overall architecture
- ✓ Vector store enables semantic search over past interactions
- ✓ Essential for coherent multi-step tasks

---

## 24.4 Task Routing and Fallback Logic {#244-task-routing-and-fallback-logic}

**Prerequisites:** Section 6.1-6.3  
**Related to:** Section 5 (AI Model Strategy)  
**Implements:** Intelligent model selection  
**Read time:** ~4 minutes

**The orchestrator must decide which model handles each task, and what to do if it fails.**

---

### Routing Decision Tree

```
┌─────────────────────────────────────────────────────────────┐
│                    TASK ROUTING LOGIC                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  NEW TASK ARRIVES                                           │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────────┐                                    │
│  │ Is it code-related? │──── Yes ──▶ Code Llama            │
│  └─────────────────────┘                                    │
│         │ No                                                 │
│         ▼                                                    │
│  ┌─────────────────────┐                                    │
│  │ Is it image gen?    │──── Yes ──▶ SDXL/ComfyUI          │
│  └─────────────────────┘                                    │
│         │ No                                                 │
│         ▼                                                    │
│  ┌─────────────────────┐                                    │
│  │ Is it complex       │──── Yes ──▶ Lead/Worker           │
│  │ multi-step?         │            (GPT-4 → Local)        │
│  └─────────────────────┘                                    │
│         │ No                                                 │
│         ▼                                                    │
│  ┌─────────────────────┐                                    │
│  │ Default             │──────────▶ Local LLM (Llama 3)    │
│  └─────────────────────┘                                    │
│                                                              │
│                                                              │
│  IF ANY MODEL FAILS:                                        │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 1. Check error type                                 │    │
│  │ 2. If quality issue → retry with larger model      │    │
│  │ 3. If timeout → retry with smaller model           │    │
│  │ 4. If persistent failure → escalate to cloud       │    │
│  │ 5. Log everything for debugging                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ Route tasks based on type and complexity
- ✓ Automatic fallback on failures
- ✓ Comprehensive logging for debugging
- ✓ User can override routing preferences

---

# 25. Collaboration and Sync {#25-collaboration-and-sync}

This section covers how Handshake enables multiple users and devices to work together.

---

## 25.1 Understanding CRDTs {#251-understanding-crdts}

**Prerequisites:** Section 2.2 (Local-First)  
**Related to:** Section 7.2 (Offline-First Architecture)  
**Implements:** Conflict-free collaboration  
**Read time:** ~5 minutes

**CRDTs are special data structures that allow multiple people to edit simultaneously without conflicts—even while offline.**

---

### Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **CRDT** | Conflict-free Replicated Data Type - data that merges automatically | Enables real-time collaboration |
| **Yjs** | Most popular JavaScript CRDT library | Our likely choice for sync |
| **Automerge** | Alternative CRDT library | Fallback option |
| **Merge** | Combining two versions of a document | Happens automatically with CRDTs |
| **Operational Transform (OT)** | Older technique (Google Docs uses this) | CRDTs are newer and better for offline |

---

### How CRDTs Work (Simplified)

═══ CORE CONCEPT ═══

> Traditional documents: "Last write wins" (someone's work gets lost)
> 
> CRDT documents: "All writes merge" (everyone's work is preserved)

```
┌─────────────────────────────────────────────────────────────┐
│           TRADITIONAL SYNC (CONFLICTS!)                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Original: "Hello World"                                    │
│                                                              │
│  Alice (offline):  "Hello World!" (added !)                 │
│  Bob (offline):    "Hello Earth" (changed World)            │
│                                                              │
│  When both sync:                                            │
│  ❌ CONFLICT! Which version wins?                           │
│  • Keep Alice's? Bob loses his change.                      │
│  • Keep Bob's? Alice loses her change.                      │
│  • Show conflict dialog? Annoying.                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│           CRDT SYNC (NO CONFLICTS!)                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Original: "Hello World"                                    │
│                                                              │
│  Alice (offline): Insert "!" at position 11                 │
│  Bob (offline):   Replace "World" with "Earth"              │
│                                                              │
│  When both sync:                                            │
│  ✅ CRDT merges both operations:                            │
│  Result: "Hello Earth!"                                     │
│                                                              │
│  Both changes preserved! No conflict dialog!                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Yjs: Our CRDT Choice

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION POINT                            │
├─────────────────────────────────────────────────────────────┤
│ What needs to be decided: CRDT implementation                │
│                                                              │
│ Options researched:                                          │
│   • Yjs - Most popular, used by many editors                │
│   • Automerge - Good, Rust implementation available         │
│   • Custom - Too much work                                   │
│                                                              │
│ Recommendation: YJS                                          │
│                                                              │
│ Rationale:                                                   │
│   • Tiptap (our editor) has Yjs integration built-in       │
│   • Large ecosystem and community                           │
│   • Works offline natively                                   │
│   • Can sync via any transport (WebSocket, WebRTC, file)   │
│                                                              │
│ Tradeoffs:                                                   │
│   • JavaScript-focused (need yrs for Rust interop)         │
│   • Learning curve for CRDT concepts                        │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **CRDTs automatically merge edits** without conflicts
- ✓ **Yjs** is the recommended library
- ✓ Works perfectly with offline-first architecture
- ✓ Tiptap editor has built-in Yjs support

---

## 25.2 Offline-First Architecture {#252-offline-first-architecture}

**Prerequisites:** Section 7.1 (CRDTs)  
**Related to:** Section 3.3 (Data Architecture)  
**Implements:** Working without internet  
**Read time:** ~3 minutes

**Handshake works completely offline. Sync happens when you're online, but it's never required.**

---

### How Offline-First Works

```
┌─────────────────────────────────────────────────────────────┐
│                    OFFLINE-FIRST FLOW                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  WORKING OFFLINE:                                           │
│  1. All data stored locally in files                       │
│  2. AI models run locally                                   │
│  3. Changes saved with CRDT state                          │
│  4. Everything works normally                               │
│                                                              │
│  WHEN ONLINE:                                               │
│  1. Check for remote changes                               │
│  2. CRDT automatically merges                              │
│  3. Push local changes to cloud (optional)                 │
│  4. Continue working                                        │
│                                                              │
│  SYNC IS OPTIONAL:                                          │
│  • Works forever with no account                           │
│  • Add sync when you want multi-device                     │
│  • Choose sync provider (Google Drive, custom server)      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ App is fully functional offline
- ✓ Sync is optional, not required
- ✓ CRDTs handle conflict-free merging
- ✓ User chooses if/where to sync

---

## 25.3 Google Workspace Integration {#253-google-workspace-integration}

**Prerequisites:** Section 7.2 (Offline-First)  
**Related to:** Section 3.2 (Overall Architecture)  
**Implements:** Gmail, Drive, Calendar sync  
**Read time:** ~4 minutes

**Optionally sync with Google services: backup to Drive, import emails, show calendar events.**

---

### Integration Points

| Service | Integration | Priority |
|---------|-------------|----------|
| **Google Drive** | Backup workspace, sync files | [OPTIONAL] |
| **Gmail** | Import emails as documents | [OPTIONAL] |
| **Calendar** | Show events in calendar view | [OPTIONAL] |
| **Google Docs** | Export/import documents | [ADVANCED] |

---

### OAuth2 Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    GOOGLE AUTH FLOW                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. User clicks "Connect Google Account"                    │
│                     │                                        │
│                     ▼                                        │
│  2. Opens system browser to Google login                    │
│                     │                                        │
│                     ▼                                        │
│  3. User grants permissions (minimal scopes)                │
│                     │                                        │
│                     ▼                                        │
│  4. Google redirects back to app with auth code             │
│                     │                                        │
│                     ▼                                        │
│  5. App exchanges code for tokens                           │
│                     │                                        │
│                     ▼                                        │
│  6. Tokens stored encrypted locally                         │
│                     │                                        │
│                     ▼                                        │
│  7. App can now call Google APIs                            │
│                                                              │
│  SECURITY: Tokens never leave user's machine                │
│  PRIVACY: Minimal scopes requested                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ Google integration is **optional**
- ✓ OAuth2 for secure authentication
- ✓ Tokens stored locally and encrypted
- ✓ Minimal permission scopes requested

---

# 26. Plugin and Extension System {#26-plugin-and-extension-system}

This section covers how to design Handshake as an extensible platform.

---

## 26.1 Plugin Architecture Patterns {#261-plugin-architecture-patterns}

**Prerequisites:** Section 3.2 (Overall Architecture)  
**Related to:** Section 8.2 (Security)  
**Implements:** Extensibility foundation  
**Read time:** ~5 minutes

**A good plugin system lets third parties (and you) extend the app without modifying core code.**

---

### Lessons from Reference Apps

Based on research of existing apps:

| App | Plugin Approach | Lesson for Handshake |
|-----|-----------------|---------------------|
| **Obsidian** | JS plugins in main process | Large ecosystem, some stability risks |
| **Joplin** | Sandboxed, separate process | Safer but more complex |
| **Logseq** | JS API, ClojureScript | Good API, some breaking changes |
| **VS Code** | Extension host process | Gold standard, but complex |

---

### Recommended Approach

```
┌─────────────────────────────────────────────────────────────┐
│                    PLUGIN ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  PHASE 1 (MVP): Internal Extension Points                   │
│  • Define stable internal APIs                              │
│  • Build core features as "internal plugins"                │
│  • Establishes patterns for later                           │
│                                                              │
│  PHASE 2: User Scripts                                      │
│  • Allow simple automation scripts                          │
│  • Sandboxed JavaScript/Python execution                    │
│  • Limited API surface                                      │
│                                                              │
│  PHASE 3: Full Plugin System                                │
│  • Public plugin API                                        │
│  • Plugin marketplace                                        │
│  • Sandboxed execution (like Joplin)                        │
│  • Permission model                                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Extension Categories to Plan For

| Category | Examples | API Needed |
|----------|----------|-----------|
| **Custom Blocks** | New editor block types | Block registration, rendering |
| **AI Agents** | Specialized AI workflows | Agent API, model access |
| **Integrations** | Third-party services | HTTP, auth storage |
| **Views** | New database views | View registration, data access |
| **Themes** | Visual customization | CSS variables, style hooks |

---

### Key Takeaways

- ✓ Design with extensibility in mind from day one
- ✓ Build core features as internal "plugins" first
- ✓ Full plugin system is Phase 3, not MVP
- ✓ Learn from Obsidian's success and Joplin's security

---

## 26.2 Security and Sandboxing {#262-security-and-sandboxing}

**Prerequisites:** Section 8.1 (Plugin Architecture)  
**Related to:** Section 3.1 (Tauri Decision)  
**Implements:** Safe plugin execution  
**Read time:** ~4 minutes

**Plugins can be dangerous. Sandboxing restricts what they can do to prevent damage.**

---

### Security Principles

═══ CORE CONCEPT ═══

> **Principle of Least Privilege:** Plugins get only the permissions they need, nothing more.
>
> | Permission Level | Can Access |
> |-----------------|------------|
> | **Level 0** | Nothing (pure computation) |
> | **Level 1** | Read workspace data |
> | **Level 2** | Write workspace data |
> | **Level 3** | Network access |
> | **Level 4** | Full filesystem access |
> | **Level 5** | Execute system commands |
>
> Most plugins should only need Levels 1-2.

---

### Tauri's Security Advantage

Tauri provides "deny-by-default" security:
- Plugins must explicitly request each capability
- User approves permissions on install
- Cleaner than Electron's more permissive model

---

### Key Takeaways

- ✓ Sandbox plugin execution
- ✓ Explicit permission requests
- ✓ User approval for sensitive permissions
- ✓ Tauri's security model helps here

---

# 27. Reference Application Analysis {#27-reference-application-analysis}

This section summarizes lessons from analyzing similar open-source applications.

---

## 27.1 AppFlowy {#271-appflowy}

**Stack:** Flutter (Dart) + Rust backend  
**Data:** CRDT-based (yrs), RocksDB storage  
**Sync:** Offline-first CRDT via Supabase

**Key Insights:**
- ✓ Flutter provides native performance and feel
- ✓ Rust CRDT implementation is solid
- ⚠️ Flutter limits JavaScript plugin ecosystem
- ⚠️ Minimal plugin API currently

---

## 27.2 AFFiNE {#272-affine}

**Stack:** Electron + React/TypeScript  
**Data:** OctoBase (custom Rust CRDT)  
**Sync:** P2P CRDT, local-first

**Key Insights:**
- ✓ "Everything is a block" model works well
- ✓ Blocksuite component library is promising
- ⚠️ Switched from Tauri to Electron (webview issues)
- ⚠️ Performance issues with large documents
- ⚠️ No mature plugin API yet

---

## 27.3 Obsidian {#273-obsidian}

**Stack:** Electron + TypeScript  
**Data:** Plain Markdown files  
**Sync:** Local vault with optional Obsidian Sync

**Key Insights:**
- ✓ Thriving plugin ecosystem (hundreds of plugins)
- ✓ Markdown files = portable, future-proof
- ✓ Excellent community engagement
- ✓ Proprietary but well-regarded
- ⚠️ Some performance issues with huge vaults

---

## 27.4 Logseq {#274-logseq}

**Stack:** Electron + ClojureScript  
**Data:** Markdown/EDN files, SQLite  
**Sync:** Git/WebDAV/LiveSync options

**Key Insights:**
- ✓ Mature JS plugin API
- ✓ Bidirectional linking works well
- ⚠️ Performance issues with large graphs/pages
- ⚠️ Team added pagination to mitigate

---

## 27.5 Lessons Learned {#275-lessons-learned}

**Prerequisites:** Sections 9.1-9.4  
**Implements:** Design guidance from research  
**Read time:** ~4 minutes

---

### Patterns to Follow

| Pattern | Why It Works | Handshake Application |
|---------|--------------|----------------------|
| **File-based storage** | Portable, user-owned data | ✓ Already planned |
| **Block-based editing** | Flexible, AI-friendly | ✓ Using Tiptap/BlockNote |
| **CRDT sync** | Offline-first, conflict-free | ✓ Using Yjs |
| **Plugin API early** | Builds ecosystem | Plan internal APIs from start |

---

### Patterns to Avoid

| Anti-Pattern | What Went Wrong | Handshake Mitigation |
|--------------|-----------------|---------------------|
| **Full doc re-render** | AFFiNE lag on keystroke | Virtualization, incremental updates |
| **Monolithic DB** | Joplin RAM bloat | File-based with SQLite index only |
| **No export path** | Athens shutdown orphaned users | Standard formats, export from day 1 |
| **Tauri webview issues** | AFFiNE switched to Electron | Minimal Tauri responsibilities, test early |

---

### Key Takeaways

- ✓ Learn from others' mistakes before building
- ✓ Performance at scale is a real concern
- ✓ Export/migration paths are essential
- ✓ Plugin ecosystems take years to build

---

# 28. Development Workflow {#28-development-workflow}

This section covers how to actually build Handshake efficiently.

---

## 28.1 Using AI Coding Assistants Effectively {#281-using-ai-coding-assistants-effectively}

**Prerequisites:** Section 2.3 (AI Models)  
**Related to:** Section 10.2 (Project Health)  
**Implements:** Development efficiency  
**Read time:** ~5 minutes

**The research documents provide a clear model for using AI assistants during development.**

---

### The Three-Layer Model

═══ CORE CONCEPT ═══

```
┌─────────────────────────────────────────────────────────────┐
│           AI ASSISTANTS IN DEVELOPMENT                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │         GPT-4 / CLAUDE (Architects)                  │    │
│  │                                                      │    │
│  │  USE FOR:                                           │    │
│  │  • Feature specs and requirements                   │    │
│  │  • Architecture decisions                           │    │
│  │  • Trade-off analysis                               │    │
│  │  • Code review                                      │    │
│  │  • Debugging complex issues                         │    │
│  │  • Test strategy                                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │         CODEX / CODE MODELS (Implementers)          │    │
│  │                                                      │    │
│  │  USE FOR:                                           │    │
│  │  • Writing code from specs                          │    │
│  │  • Mechanical refactoring                           │    │
│  │  • Generating tests                                 │    │
│  │  • Writing boilerplate                              │    │
│  │  • Documentation comments                           │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │         N8N / AUTOMATION (Operations)               │    │
│  │                                                      │    │
│  │  USE FOR:                                           │    │
│  │  • CI/CD workflows                                  │    │
│  │  • Health monitoring                                │    │
│  │  • Notifications                                    │    │
│  │  • External integrations                            │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### AI Development Workflow

| Phase | Use Generalist (GPT-4/Claude) | Use Code Model (Codex) |
|-------|------------------------------|------------------------|
| **Planning** | ✓ Define specs, goals, non-goals | |
| **Architecture** | ✓ Design systems, APIs | Scaffold structure |
| **Implementation** | Review PRs | ✓ Write code from specs |
| **Testing** | Design test strategy | ✓ Write test code |
| **Debugging** | ✓ Analyze logs, hypothesize | Apply fixes |
| **Documentation** | ✓ Write overviews | Docstrings, comments |

---

### Key Takeaways

- ✓ **Generalists (GPT-4/Claude)** for thinking, **Code models** for doing
- ✓ Always write specs before code
- ✓ AI reviews AI-generated code (human oversight too)
- ✓ n8n for DevOps automation

---

## 28.2 Project Health and Hygiene {#282-project-health-and-hygiene}

**Prerequisites:** Section 10.1 (AI Assistants)  
**Related to:** Section 10.3 (CI/CD)  
**Implements:** Maintainable codebase  
**Read time:** ~5 minutes

**A clean, consistent codebase is essential—especially when AI assistants help write code.**

---

### The Single Health Command

═══ CORE CONCEPT ═══

> **One command to rule them all:** A single command that validates the entire codebase.
>
> ```bash
> make check   # or: npm run check / python -m tools.health_check
> ```
>
> This command runs:
> 1. Linters (code style)
> 2. Type checking
> 3. Tests (fast subset)
> 4. Build verification
>
> **Why it matters:** Humans and AI both have ONE clear way to know if code is "good."

---

### Tool Stack

| Layer | Python (Backend) | TypeScript (Frontend) |
|-------|-----------------|----------------------|
| **Linting** | Ruff or flake8 | ESLint |
| **Formatting** | Black | Prettier |
| **Type Checking** | Pydantic, mypy | TypeScript strict |
| **Testing** | pytest | vitest or jest |
| **Import Sorting** | isort or Ruff | ESLint rules |

---

### Pre-Commit Hooks

```yaml
# .pre-commit-config.yaml (example)
repos:
  - repo: local
    hooks:
      - id: format-python
        name: Format Python (Black)
        entry: black
        language: system
        files: \.py$
      - id: lint-python
        name: Lint Python (Ruff)
        entry: ruff check
        language: system
        files: \.py$
```

💡 **Tip:** Pre-commit hooks catch issues before they reach CI, saving time and keeping history clean.

---

### Key Takeaways

- ✓ **One health command** for all checks
- ✓ Linters and formatters for consistency
- ✓ Pre-commit hooks to catch issues early
- ✓ Type annotations for AI and human safety

---

## 28.3 CI/CD and Testing Strategy {#283-cicd-and-testing-strategy}

**Prerequisites:** Section 10.2 (Project Health)  
**Related to:** Section 11 (Development Roadmap)  
**Implements:** Automated quality assurance  
**Read time:** ~4 minutes

**Continuous Integration ensures every code change is tested automatically.**

---

### CI Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                    CI PIPELINE (on every push)               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. LINT                                                    │
│     └─ Ruff, ESLint                                        │
│                                                              │
│  2. TYPE CHECK                                              │
│     └─ mypy, TypeScript                                    │
│                                                              │
│  3. UNIT TESTS                                              │
│     └─ pytest, vitest (fast tests only)                    │
│                                                              │
│  4. BUILD                                                   │
│     └─ Frontend bundle, backend validation                 │
│                                                              │
│  IF ALL PASS → ✅ Ready to merge                            │
│  IF ANY FAIL → ❌ Block merge, fix issues                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Testing Pyramid

```
            ┌───────────┐
            │   E2E     │  Few, slow, high confidence
            │   Tests   │
            └─────┬─────┘
                  │
         ┌───────┴───────┐
         │ Integration   │  Some, medium speed
         │    Tests      │
         └───────┬───────┘
                 │
        ┌────────┴────────┐
        │    Unit Tests    │  Many, fast, low coupling
        └──────────────────┘
```

---

### Key Takeaways

- ✓ CI runs on every push/PR
- ✓ Failures block merges
- ✓ Fast tests in CI, slow tests on schedule
- ✓ Testing pyramid: many unit, some integration, few E2E

---

# 29. Development Roadmap {#29-development-roadmap}

This section provides a practical build order for Project Handshake.

---

## 29.1 Phase Overview {#291-phase-overview}

**Prerequisites:** All previous sections  
**Implements:** Project execution plan  
**Read time:** ~5 minutes

---

### The Phases

```
┌─────────────────────────────────────────────────────────────┐
│                    DEVELOPMENT PHASES                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  PHASE 0: FOUNDATION (2-4 weeks)                            │
│  ├─ Set up monorepo structure                               │
│  ├─ Tauri + React scaffolding                               │
│  ├─ Python backend skeleton                                 │
│  ├─ Health check / CI pipeline                              │
│  └─ Basic IPC between frontend and backend                  │
│                                                              │
│  PHASE 1: CORE EDITING (4-6 weeks)                          │
│  ├─ Block editor with Tiptap/BlockNote                      │
│  ├─ File-tree storage system                                │
│  ├─ Workspace navigator sidebar                             │
│  └─ Basic CRUD operations                                   │
│                                                              │
│  PHASE 2: AI INTEGRATION (4-6 weeks)                        │
│  ├─ Ollama integration for local LLM                        │
│  ├─ Basic AI actions (summarize, write, translate)          │
│  ├─ Orchestrator setup (AutoGen or LangGraph)               │
│  └─ Streaming responses to UI                               │
│                                                              │
│  PHASE 3: VISUAL TOOLS (3-4 weeks)                          │
│  ├─ Excalidraw canvas integration                           │
│  ├─ Basic spreadsheet with HyperFormula                     │
│  └─ ComfyUI integration for images                          │
│                                                              │
│  PHASE 4: POLISH & SYNC (4+ weeks)                          │
│  ├─ Yjs CRDT integration                                    │
│  ├─ Optional Google Drive sync                              │
│  ├─ UI polish and performance                               │
│  └─ Packaging and distribution                              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 29.2 MVP Definition {#292-mvp-definition}

**Minimum Viable Product** = Phases 0-2 completed

### MVP Features

| Feature | Included in MVP |
|---------|-----------------|
| Document editor | ✅ |
| File-tree storage | ✅ |
| Basic AI (summarize, write) | ✅ |
| Local LLM via Ollama | ✅ |
| Canvas/whiteboard | ❌ (Phase 3) |
| Spreadsheets | ❌ (Phase 3) |
| Image generation | ❌ (Phase 3) |
| Multi-device sync | ❌ (Phase 4) |

---

## 29.3 Build Order and Dependencies {#293-build-order-and-dependencies}

```
┌─────────────────────────────────────────────────────────────┐
│                    DEPENDENCY GRAPH                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   [Monorepo Setup]                                          │
│         │                                                    │
│         ├──────────────┬──────────────┐                     │
│         ▼              ▼              ▼                     │
│   [Tauri Shell]  [Python Backend]  [CI Pipeline]           │
│         │              │              │                     │
│         └──────────────┼──────────────┘                     │
│                        ▼                                     │
│              [Frontend-Backend IPC]                         │
│                        │                                     │
│         ┌──────────────┼──────────────┐                     │
│         ▼              ▼              ▼                     │
│   [Block Editor]  [File Storage]  [Ollama Integration]      │
│         │              │              │                     │
│         └──────────────┼──────────────┘                     │
│                        ▼                                     │
│              [AI Actions in Editor]                         │
│                        │                                     │
│         ┌──────────────┼──────────────┐                     │
│         ▼              ▼              ▼                     │
│     [Canvas]    [Spreadsheet]    [ComfyUI]                  │
│         │              │              │                     │
│         └──────────────┼──────────────┘                     │
│                        ▼                                     │
│              [Yjs Sync / Polish]                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### Key Takeaways

- ✓ **Phase 0** must be solid before adding features
- ✓ **MVP is achievable in ~12 weeks** with focused effort
- ✓ Build foundational pieces first (IPC, storage, CI)
- ✓ AI integration comes after basic editing works

---

# 30. Risk Assessment {#30-risk-assessment}

**Prerequisites:** All previous sections  
**Implements:** Risk awareness and mitigation  
**Read time:** ~4 minutes

---

### Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Tauri webview issues** | Medium | High | Minimal Tauri role; test early on all platforms |
| **Local model performance** | Medium | Medium | Cloud fallback; smaller model options |
| **Complexity overwhelm** | High | High | Strict MVP scope; phases; hire help |
| **CRDT learning curve** | Medium | Medium | Use Yjs (proven); start with single-user |
| **Plugin security** | Low | High | Delay plugins; learn from Joplin model |
| **Scope creep** | High | High | Written MVP definition; say no to extras |

---

### Complexity Ratings

| Component | Complexity | Notes |
|-----------|------------|-------|
| Tauri setup | ⚠️ Medium | Some Rust knowledge needed |
| Block editor | ⚠️ Medium | Tiptap helps a lot |
| AI orchestration | ⚠️⚠️ High | Multi-model coordination is complex |
| Canvas | ⚠️ Medium | Excalidraw does heavy lifting |
| Spreadsheets | ⚠️ Medium | HyperFormula helps |
| CRDT sync | ⚠️⚠️ High | Conceptually challenging |
| ComfyUI integration | ⚠️ Medium | API-based, manageable |
| Plugin system | ⚠️⚠️ High | Defer to post-MVP |

---

# 31. Technology Stack Summary {#31-technology-stack-summary}

**Complete list of technologies mentioned across all research documents.**

---

### Core Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Desktop Shell** | Tauri | Cross-platform wrapper |
| **Frontend** | React + TypeScript | User interface |
| **Backend** | Python (FastAPI) | API server, orchestration |
| **AI Runtime** | Ollama, ComfyUI | Model execution |
| **Storage** | File system + SQLite | Data persistence |
| **Sync** | Yjs (CRDT) | Collaboration |

---

### Frontend Libraries

| Library | Purpose |
|---------|---------|
| Tiptap / BlockNote | Block-based editor |
| Excalidraw | Canvas/whiteboard |
| HyperFormula | Spreadsheet formulas |
| Wolf-Table | Spreadsheet UI |
| React Table / AG Grid | Data grid views |
| React Beautiful DnD | Drag and drop |

---

### Backend Libraries

| Library | Purpose |
|---------|---------|
| FastAPI | HTTP API server |
| AutoGen or LangGraph | Agent orchestration |
| Ollama API | Local LLM access |
| ComfyUI API | Image generation |
| Pydantic | Data validation |
| SQLAlchemy | SQLite access |

---

### AI Models

| Model | Purpose | Size |
|-------|---------|------|
| Llama 3 13B | General text | ~14GB |
| Code Llama 13B | Code generation | ~14GB |
| Mistral 7B | Fast responses | ~8GB |
| SDXL 1.0 | Image generation | ~10GB |

---

### DevOps Tools

| Tool | Purpose |
|------|---------|
| GitHub Actions | CI/CD |
| Ruff, Black, isort | Python linting/formatting |
| ESLint, Prettier | TypeScript linting/formatting |
| pytest | Python testing |
| vitest | TypeScript testing |
| n8n (optional) | Workflow automation |

---

# 32. Consolidated Glossary {#32-consolidated-glossary}

**Alphabetical list of all technical terms defined in this document.**

| Term | Definition |
|------|------------|
| **Agent** | An AI model configured for a specific role with the ability to take actions |
| **API** | Application Programming Interface - how programs communicate with each other |
| **AutoGen** | Microsoft's multi-agent conversation framework |
| **Block-Based Editor** | Editor where content is made of stackable blocks instead of continuous text |
| **Chromium** | Open-source browser engine that Chrome is built on |
| **ComfyUI** | Node-based visual tool for Stable Diffusion image generation |
| **CRDT** | Conflict-free Replicated Data Type - enables automatic merge of concurrent edits |
| **Desktop Shell** | Program that wraps web code to run as a native desktop application |
| **Electron** | Popular desktop shell that bundles Chromium and Node.js |
| **GGUF** | File format for quantized AI models |
| **HyperFormula** | Open-source spreadsheet formula engine |
| **IPC** | Inter-Process Communication - how different parts of an app talk to each other |
| **LangGraph** | LangChain's graph-based agent orchestration framework |
| **Lead/Worker Pattern** | Smart model plans, simpler models execute |
| **LLM** | Large Language Model - AI trained on text to understand and generate language |
| **Local-First** | Architecture where data lives primarily on user's device |
| **Monorepo** | Single repository containing multiple related projects |
| **OAuth2** | Standard protocol for secure third-party authorization |
| **Ollama** | Easy-to-use local LLM runner |
| **Orchestrator** | Code that coordinates multiple AI models to work together |
| **Parameters** | The "knobs" inside an AI model (more = smarter but heavier) |
| **Quantization** | Shrinking AI models to use less memory |
| **REST API** | Common style for web APIs using HTTP methods |
| **SDXL** | Stable Diffusion XL - high-quality image generation model |
| **Sidecar File** | Small metadata file that accompanies a main file |
| **SQLite** | Lightweight database contained in a single file |
| **Tauri** | Lightweight desktop shell using Rust and system webview |
| **Tiptap** | Extensible rich text editor framework |
| **VRAM** | Video RAM - memory on graphics card where AI models run |
| **WebSocket** | Protocol for real-time, two-way communication |
| **Yjs** | Popular JavaScript CRDT library |

---

# 33. Open Questions and Next Steps {#33-open-questions-and-next-steps}

**Things the research doesn't fully answer that need further investigation.**

---

### Unresolved Questions

| Question | Why It Matters | Suggested Action |
|----------|---------------|------------------|
| **Exact Tauri version?** | v1 vs v2 have API differences | Check latest stable, test early |
| **Python bundling strategy?** | How to package Python with Tauri | Research PyInstaller + Tauri sidecar |
| **Model download UX?** | How do users get 10GB+ models? | Design in-app download + progress UI |
| **License audit** | Some libraries have complex licenses | Full audit before production |
| **Performance benchmarks** | Real numbers on target hardware | Build prototype, measure |

---

### Research Gaps

The documents **don't cover**:
- Mobile versions (iOS/Android)
- Web version (browser-only)
- Enterprise features (SSO, audit logs)
- Monetization strategy
- Analytics/telemetry approach
- Accessibility (a11y) requirements

---

### Immediate Next Steps

1. **Set up monorepo** with Tauri + React + Python structure
2. **Validate Tauri** on Windows, Mac, Linux
3. **Prototype IPC** between React and Python
4. **Test Ollama** integration
5. **Build health check** command

---

# 34. Sources Referenced {#34-sources-referenced}

This document consolidates research from the following source documents:

1. **Handshake_Project.pdf** (9 pages)
   - Core specification: multi-model orchestration, UI frameworks, Google API integration, ComfyUI, architecture overview

2. **Model_Strategy_and_Tooling_Guide.pdf** (4 pages)
   - AI assistant usage strategy, Codex vs GPT-4/Claude roles, n8n evaluation

3. **Reference_App_Deep_Dive_Local-First_Open_Workspace_Tools.pdf** (7 pages)
   - Technical analysis of AppFlowy, AFFiNE, Anytype, Logseq, Obsidian, Joplin

4. **Tauri_Electron_Decision.pdf** (4 pages)
   - Framework comparison, consensus from multiple AI advisors recommending Tauri

5. **Project_Health_Hygiene_Guide.pdf** (7 pages)
   - Codebase standards, testing, CI/CD, logging, AI-friendly practices

6. **Development_Roadmap_Draft.pdf** (7 pages)
   - Phase planning, implementation order, testing strategy, deployment

7. **Notion_vs_Milanote_vs_Excel_Feature_Comparison.pdf** (4 pages)
   - Target app analysis, orchestration framework comparison, local model recommendations

---

## Document End

**Total estimated read time:** ~90 minutes for complete document

**For quick reference:**
- [Executive Summary](#19-executive-summary) - 5 min overview
- [Technology Stack Summary](#31-technology-stack-summary) - Quick reference
- [Development Roadmap](#29-development-roadmap) - What to build when

---

*This document was compiled on November 29, 2025 from 7 research documents totaling ~42 pages.*
