# 7. User Experience & Development

## 7.1 User Interface Components

**Why**  
The UI components define how users interact with Handshake. Choosing the right libraries and patterns ensures a familiar yet powerful experience combining the best of Notion, Milanote, and Excel.

**What**  
Covers the three main UI components: Rich Text Editor (Notion-like block-based editing with Tiptap/BlockNote), Freeform Canvas (Milanote-like infinite whiteboard with Excalidraw), Spreadsheet Engine (Excel-like data manipulation with Wolf-Table + HyperFormula), and Additional Views (Kanban, Calendar, Timeline).

**Jargon**  
- **Block-Based Editor**: Content made of stackable, movable blocks (paragraphs, images, lists) rather than continuous text.
- **Tiptap**: Popular open-source editor framework built on ProseMirror.
- **BlockNote**: Notion-style block editor built on Tiptap with pre-built components.
- **Slash Commands**: Type "/" to access insertion menu (like /heading, /image).
- **Excalidraw**: Popular open-source whiteboard with hand-drawn aesthetic.
- **Infinite Canvas**: Workspace extending forever in all directions with pan/zoom.
- **HyperFormula**: Open-source formula engine with 400+ Excel-compatible functions.
- **Wolf-Table (x-spreadsheet)**: Lightweight JavaScript spreadsheet grid.

---
This section covers the frontend UI components that make up the Handshake user experience, combining the best features of Notion, Milanote, and Excel.

---

### 7.1.0 Cross-View Tool Integration Overview

All UI components sit on top of the same workspace and job model. There is no separate "doc app", "canvas app", or "spreadsheet app" internally.

**Shared foundation**

- **Workspace entities:** Documents, Blocks, Canvas Nodes, Tables, Tasks/Events, Assets (Section 2.2.1).
- **Layers:** RawContent / DerivedContent / DisplayContent with strict rules (Section 2.2.2).
- **Jobs:** Docs & Sheets Profile and other AI Job Profiles (Sections 2.5.10 and 2.6.6).
- **Sync:** CRDT-based collaboration via Yjs (Section 7.3).

**UI tool families**

| View / Area      | Primary OSS Libraries                            | Primary Entities           | Notes |
|------------------|--------------------------------------------------|----------------------------|-------|
| Rich documents   | Tiptap + BlockNote                               | Document, Block            | Block-based editor; blocks map directly to workspace nodes. |
| Freeform canvas  | Excalidraw (+ custom canvas node rendering)      | Canvas, Canvas Node, Block | Cards reference underlying blocks; frames and groups map to graph clusters. |
| Tables / sheets  | Wolf-Table (grid) + HyperFormula (formulas)      | Table, Row, Cell           | Sheet operations executed via Docs & Sheets AI Job Profile. |
| Charts / dashboards | Apache ECharts (charts)                          | Chart (refs Table)         | Charts reference table IDs/ranges; dashboards are layouts over existing entities. |
| Decks / slideshows  | Reveal.js (in-app present) + PptxGenJS (export)  | Deck, Slide                | Deck composes references to blocks/canvas frames/charts; export produces artifacts with provenance. |
| Mechanical ingest| Docling, Unstructured, Tika, converters, ASR     | Document, Asset, Table     | Produce RawContent/DerivedContent that the views consume. |

**Integration rules**

1. UI components **MUST NOT** introduce their own persistent storage or IDs for core entities; they use workspace IDs and schemas.
2. Any operation that crosses views (e.g. "send selection to canvas", "turn table into doc section") **MUST** preserve entity IDs instead of duplicating content.
3. Mechanical tools (Section 6) integrate via the Model Runtime Layer and AI Jobs; from the UI's point of view, they are invoked like any other background operation and return changes to workspace entities.

---

### 7.1.1 Rich Text Editor (Notion-like)

**Prerequisites:** Section 2.1 (High-Level Architecture)  
**Related to:** Section 7.1 (User Interface Components)  
**Implements:** Core document editing  
**Read time:** ~6 minutes

**The document editor is the heart of Handshakeâ€”a "block-based" editor where every paragraph, image, and element is a separate, movable piece.**

---

#### 7.1.1.1 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Block-Based Editor** | Instead of one continuous document, content is made of stackable "blocks" (paragraphs, images, lists, etc.) | Enables drag/drop, AI operations on specific sections |
| **Tiptap** | A popular open-source editor framework built on ProseMirror | Leading candidate for our editor |
| **BlockNote** | A Notion-style block editor built on Tiptap | Pre-built Notion-like components |
| **Slash Commands** | Type "/" to get a menu of things to insert (like /heading, /image) | Familiar UX from Notion |
| **Real-Time Collaboration** | Multiple people editing the same document simultaneously | Requires CRDT integration |

---

#### 7.1.1.2 The Block Mental Model

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              TRADITIONAL DOCUMENT                            â”‚
â”‚  â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€                      â”‚
â”‚  One continuous blob of formatted text                      â”‚
â”‚  that flows from top to bottom. Hard to                     â”‚
â”‚  rearrange, hard for AI to understand                       â”‚
â”‚  structure.                                                 â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜

                         vs.

â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              BLOCK-BASED DOCUMENT                            â”‚
â”‚                                                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚ BLOCK: Heading                                       â”‚ â˜°  â”‚
â”‚  â”‚ "Project Overview"                                   â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚ BLOCK: Paragraph                                     â”‚ â˜°  â”‚
â”‚  â”‚ "This project aims to..."                           â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚ BLOCK: AI-Generated Summary                         â”‚ â˜°  â”‚
â”‚  â”‚ "Key points: 1) ... 2) ... 3) ..."                 â”‚ ðŸ¤– â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚ BLOCK: Image                                        â”‚ â˜°  â”‚
â”‚  â”‚ [diagram.png]                                       â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                                              â”‚
â”‚  â˜° = Drag handle (reorder blocks)                          â”‚
â”‚  ðŸ¤– = AI-generated content indicator                        â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.1.1.3 Technology Choice

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    DECISION POINT                            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ What needs to be decided: Rich text editor framework         â”‚
â”‚                                                              â”‚
â”‚ Options researched:                                          â”‚
â”‚   â€¢ Tiptap/ProseMirror - Most extensible, proven            â”‚
â”‚   â€¢ BlockNote - Notion-style, built on Tiptap               â”‚
â”‚   â€¢ Lexical (Meta) - Newer, less collaboration support      â”‚
â”‚   â€¢ Slate.js - Flexible but complex                         â”‚
â”‚                                                              â”‚
â”‚ Recommendation: TIPTAP with BLOCKNOTE components             â”‚
â”‚                                                              â”‚
â”‚ Rationale:                                                   â”‚
â”‚   â€¢ BlockNote provides Notion-style blocks out of the box   â”‚
â”‚   â€¢ Tiptap is highly extensible for custom AI blocks        â”‚
â”‚   â€¢ Yjs integration available for real-time collaboration   â”‚
â”‚   â€¢ Large community and good documentation                   â”‚
â”‚                                                              â”‚
â”‚ Tradeoffs:                                                   â”‚
â”‚   â€¢ Some learning curve                                      â”‚
â”‚   â€¢ May need custom extensions for AI features              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.1.1.4 Block Types to Implement

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

#### 7.1.1.5 AI Integration Points

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              AI-ENHANCED EDITING                             â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  SLASH COMMAND MENU (type "/")                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                        â”‚
â”‚  â”‚ / Basic                         â”‚                        â”‚
â”‚  â”‚   Paragraph, Heading, List...   â”‚                        â”‚
â”‚  â”‚                                 â”‚                        â”‚
â”‚  â”‚ / AI Actions âœ¨                 â”‚                        â”‚
â”‚  â”‚   ðŸ“ Generate text              â”‚                        â”‚
â”‚  â”‚   ðŸ“‹ Summarize above            â”‚                        â”‚
â”‚  â”‚   ðŸ”„ Rewrite selection          â”‚                        â”‚
â”‚  â”‚   ðŸŒ Translate                  â”‚                        â”‚
â”‚  â”‚   ðŸŽ¨ Generate image             â”‚                        â”‚
â”‚  â”‚   ðŸ’» Generate code              â”‚                        â”‚
â”‚  â”‚   ðŸ“Š Create table from text     â”‚                        â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                        â”‚
â”‚                                                              â”‚
â”‚  CONTEXT MENU (select text, right-click)                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                        â”‚
â”‚  â”‚ Improve writing                 â”‚                        â”‚
â”‚  â”‚ Make shorter                    â”‚                        â”‚
â”‚  â”‚ Make longer                     â”‚                        â”‚
â”‚  â”‚ Fix grammar                     â”‚                        â”‚
â”‚  â”‚ Explain this                    â”‚                        â”‚
â”‚  â”‚ Ask AI...                       â”‚                        â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.1.1.6 Key Takeaways

- âœ“ **Block-based editing** enables flexible layouts and AI operations
- âœ“ **Tiptap + BlockNote** is the recommended stack
- âœ“ **Slash commands** provide quick access to AI features
- âœ“ Blocks can be drag-and-dropped, nested, and reordered
- âœ“ Real-time collaboration via Yjs integration

**See Also:** [Section 3.2 - CRDT Libraries Comparison](#32-crdt-libraries-comparison)

---


#### 7.1.1.7 Inline Mentions & Tags (Loom) [ADD v02.130]

The Rich Text Editor MUST support **inline relational tokens** for Loom (Â§10.12):

- **@mentions** â†’ create `LoomEdgeType.MENTION` edges (Â§2.3.7.1)
- **#tags** â†’ create `LoomEdgeType.TAG` edges (Â§2.3.7.1) where the tag target is a `LoomBlock(content_type=TAG_HUB)`

**Rendering + interaction (normative)**  
- Tokens MUST render as interactive UI chips/links (not plain text).
- Tokens MUST resolve targets by UUID (rename-safe).
- Creating `@mention` to a non-existent target MUST auto-create a `LoomBlock(content_type=NOTE)` with that title (**[LM-LINK-004]**).
- Creating `#tag` for a non-existent tag MUST auto-create a `LoomBlock(content_type=TAG_HUB)` (**[LM-TAG-002]**).

**Anchoring (normative)**  
When an inline token is created, the editor MUST persist a `source_anchor` on the corresponding LoomEdge (document_id, block_id, offset_start, offset_end) so backlinks can show context snippets (**[LM-BACK-003]**).

---
### 7.1.2 Freeform Canvas (Milanote-like)

**Prerequisites:** Section 2.1 (High-Level Architecture)  
**Related to:** Section 7.1 (User Interface Components)  
**Implements:** Visual brainstorming space  
**Read time:** ~5 minutes

**The canvas is an infinite whiteboard where you can drag notes, images, and shapes anywhereâ€”like a digital corkboard for visual thinkers.**

---

#### 7.1.2.1 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Infinite Canvas** | A workspace that extends forever in all directions | No page boundaries, unlimited space |
| **Excalidraw** | Popular open-source whiteboard with hand-drawn look | Leading candidate for our canvas |
| **React-Konva** | Library for drawing graphics in React | Alternative for custom canvas needs |
| **Pan & Zoom** | Moving around and magnifying the canvas | Essential for large boards |

---

#### 7.1.2.2 The Canvas vs. Document Distinction

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    DOCUMENT EDITOR                           â”‚
â”‚                                                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚                                                     â”‚    â”‚
â”‚  â”‚  Text flows top-to-bottom                          â”‚    â”‚
â”‚  â”‚                                                     â”‚    â”‚
â”‚  â”‚  Linear structure                                  â”‚    â”‚
â”‚  â”‚                                                     â”‚    â”‚
â”‚  â”‚  Like a Word document or web page                  â”‚    â”‚
â”‚  â”‚                                                     â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                                              â”‚
â”‚  BEST FOR: Writing, documentation, structured content       â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜

â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    CANVAS BOARD                              â”‚
â”‚                                                              â”‚
â”‚      â”Œâ”€â”€â”€â”€â”€â”€â”€â”                      â”Œâ”€â”€â”€â”€â”€â”€â”€â”               â”‚
â”‚      â”‚ Note  â”‚â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”‚ Image â”‚               â”‚
â”‚      â””â”€â”€â”€â”€â”€â”€â”€â”˜                      â””â”€â”€â”€â”€â”€â”€â”€â”˜               â”‚
â”‚            \                                                â”‚
â”‚             \     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                             â”‚
â”‚              â”€â”€â”€â”€â”€â”‚ Idea Box  â”‚                             â”‚
â”‚                   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                             â”‚
â”‚                         â”‚                                    â”‚
â”‚    â”Œâ”€â”€â”€â”€â”€â”€â”€â”           â”‚                                    â”‚
â”‚    â”‚Sketch â”‚â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜          â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”            â”‚
â”‚    â””â”€â”€â”€â”€â”€â”€â”€â”˜                      â”‚ Reference â”‚            â”‚
â”‚                                    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜            â”‚
â”‚                                                              â”‚
â”‚  BEST FOR: Brainstorming, mood boards, spatial thinking     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.1.2.3 Technology Choice

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    DECISION POINT                            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ What needs to be decided: Canvas/whiteboard library          â”‚
â”‚                                                              â”‚
â”‚ Options researched:                                          â”‚
â”‚   â€¢ Excalidraw - Mature, MIT-licensed, hand-drawn feel      â”‚
â”‚   â€¢ tldraw - Modern, React-focused, good collaboration      â”‚
â”‚   â€¢ React-Konva - Low-level, full control                   â”‚
â”‚   â€¢ Fabric.js - Canvas library, more work to integrate      â”‚
â”‚                                                              â”‚
â”‚ Recommendation: EXCALIDRAW                                   â”‚
â”‚                                                              â”‚
â”‚ Rationale:                                                   â”‚
â”‚   â€¢ Production-proven (used by many products)               â”‚
â”‚   â€¢ Built-in collaboration support                          â”‚
â”‚   â€¢ Familiar "whiteboard" UX                                â”‚
â”‚   â€¢ Can embed in React easily                               â”‚
â”‚                                                              â”‚
â”‚ Tradeoffs:                                                   â”‚
â”‚   â€¢ "Hand-drawn" aesthetic may not fit all use cases        â”‚
â”‚   â€¢ May need customization for Milanote-style features      â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.1.2.4 Canvas Element Types

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

#### 7.1.2.5 AI Integration for Canvas

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              AI-ENHANCED CANVAS                              â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  RIGHT-CLICK ON CANVAS:                                     â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                        â”‚
â”‚  â”‚ ðŸŽ¨ Generate image here...       â”‚                        â”‚
â”‚  â”‚ ðŸ“ Add AI note about...         â”‚                        â”‚
â”‚  â”‚ ðŸ’¡ Brainstorm ideas about...    â”‚                        â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                        â”‚
â”‚                                                              â”‚
â”‚  SELECT MULTIPLE ITEMS:                                     â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                        â”‚
â”‚  â”‚ ðŸ“‹ Summarize selected items     â”‚                        â”‚
â”‚  â”‚ ðŸ”— Find connections             â”‚                        â”‚
â”‚  â”‚ ðŸ“Š Organize into categories     â”‚                        â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                        â”‚
â”‚                                                              â”‚
â”‚  DRAG IMAGE ONTO CANVAS:                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                        â”‚
â”‚  â”‚ ðŸ” Describe this image          â”‚                        â”‚
â”‚  â”‚ ðŸŽ¨ Generate variations          â”‚                        â”‚
â”‚  â”‚ âœ‚ï¸ Remove background             â”‚                        â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.1.3.6 Key Takeaways

- âœ“ **Two components:** Data Grid (UI) + Formula Engine (HyperFormula)
- âœ“ **HyperFormula** provides Excel-compatible formulas
- âœ“ Data stored as CSV (portable) with JSON sidecar for formatting
- âœ“ AI can help write formulas and analyze data
- âœ“ Start simple, add advanced features later

---

### 7.1.4 Additional Views: Kanban, Calendar, Timeline

**Prerequisites:** Section 7.1 (User Interface Components), Section 7.1 (User Interface Components)  
**Related to:** Section 2.2 (Data & Content Model)  
**Implements:** Notion-style database views  
**Read time:** ~4 minutes

**The same data can be viewed different ways: as a table, as Kanban cards, as calendar events, or as a timeline.**

---

#### 7.1.4.1 The "Views" Concept

â•â•â• CORE CONCEPT â•â•â•

> **One dataset, many presentations.** A list of tasks can be:
> - A **table** (spreadsheet-style rows)
> - A **Kanban board** (cards in columns like "To Do", "In Progress", "Done")
> - A **calendar** (if tasks have dates)
> - A **timeline/Gantt** (showing duration and dependencies)
>
> The underlying data is identical; only the visualization changes.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    SAME DATA, DIFFERENT VIEWS                â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  DATABASE: Tasks                                            â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚ ID â”‚ Title        â”‚ Status      â”‚ Due Date â”‚ Owner   â”‚   â”‚
â”‚  â”‚ 1  â”‚ Design logo  â”‚ In Progress â”‚ Dec 1    â”‚ Alice   â”‚   â”‚
â”‚  â”‚ 2  â”‚ Write copy   â”‚ To Do       â”‚ Dec 3    â”‚ Bob     â”‚   â”‚
â”‚  â”‚ 3  â”‚ Launch site  â”‚ To Do       â”‚ Dec 10   â”‚ Alice   â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                                              â”‚
â”‚           â”‚                    â”‚                    â”‚        â”‚
â”‚           â–¼                    â–¼                    â–¼        â”‚
â”‚                                                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”       â”‚
â”‚  â”‚   TABLE      â”‚  â”‚   KANBAN     â”‚  â”‚   CALENDAR   â”‚       â”‚
â”‚  â”‚   VIEW       â”‚  â”‚   VIEW       â”‚  â”‚   VIEW       â”‚       â”‚
â”‚  â”‚              â”‚  â”‚              â”‚  â”‚              â”‚       â”‚
â”‚  â”‚ Spreadsheet  â”‚  â”‚ To Do â”‚ In   â”‚  â”‚    Dec       â”‚       â”‚
â”‚  â”‚ style rows   â”‚  â”‚       â”‚Progr â”‚  â”‚ 1 [Design]   â”‚       â”‚
â”‚  â”‚              â”‚  â”‚ [Copy]â”‚[Logo]â”‚  â”‚ 3 [Copy]     â”‚       â”‚
â”‚  â”‚              â”‚  â”‚ [Site]â”‚      â”‚  â”‚ 10 [Launch]  â”‚       â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜       â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.1.4.2 Implementation Priority

| View Type | Priority | Library Options |
|-----------|----------|-----------------|
| **Table** | [CORE] | AG Grid, React Table |
| **Kanban** | [CORE] | react-beautiful-dnd, dnd-kit |
| **Calendar** | [OPTIONAL] | FullCalendar, react-big-calendar |
| **Timeline/Gantt** | [ADVANCED] | frappe-gantt, custom |
| **Gallery** | [OPTIONAL] | Custom grid layout |

---


#### 7.1.4.3 Loom Views (All / Unlinked / Sorted / Pins) [ADD v02.130]

Loom introduces a **library-style surface family** composed of four browsing projections over the same underlying LoomBlock dataset (Â§2.2.1.14, Â§10.12). These are **DisplayContent** projections (Â§2.2.2.3), not separate stores.

| View | Query | Purpose | UX |
|------|-------|---------|----|
| **All** | All LoomBlocks, `updated_at DESC` | Chronological feed (like a photo stream) | Infinite scroll; grid or list |
| **Unlinked** | LoomBlocks with zero MENTION + TAG edges (`backlink_count + mention_count + tag_count == 0`) | Triage queue for unorganized items | Items disappear immediately when linked |
| **Sorted** | LoomBlocks grouped by their TAG and MENTION targets | Browse by structure | Expandable groups; each group is a mini-feed |
| **Pins** | LoomBlocks where `pinned == true` | Quick access | Grid; user reorder |

**Normative requirements**

- **[LM-VIEW-001]** All four views MUST be available as workspace-level surfaces. They are NOT separate stores â€” they query the same LoomBlock table with different filters/groupings.
- **[LM-VIEW-002]** The Unlinked view MUST update in real-time as the user creates links. A block that gains its first MENTION or TAG edge MUST disappear from the Unlinked view immediately.
- **[LM-VIEW-003]** Views MUST support filtering by: content_type, file MIME type, date range, specific tags, specific mentions.
- **[LM-VIEW-004]** Views MUST support switching between grid layout (media) and list layout (notes/documents).

**Cross-view integration**  
Loom views follow Â§7.1.0 Cross-View Tool Integration rules:
- Loom views MUST NOT introduce their own persistent storage or IDs.
- LoomBlocks that are also Documents participate in the document editor.
- LoomBlocks that contain Assets participate in the photo/media pipeline (Â§10.10).
- Dragging a LoomBlock onto a Canvas creates a Canvas Node referencing the same entity (no copy).

---
## 7.2 Multi-Agent Orchestration

**Why**  
Complex tasks require coordinating multiple specialized AI models. This section explains how to orchestrate agents effectively using the lead/worker pattern for cost-effective, high-quality results.

**What**  
Compares orchestration frameworks (AutoGen, LangGraph, CrewAI), explains the lead/worker pattern for cost optimization, covers shared context/memory between agents, and defines task routing and fallback logic.

**Jargon**  
- **Agent**: An AI model with a specific job and ability to take actions.
- **Orchestrator**: The "boss" code that decides which agent handles what.
- **AutoGen**: Microsoft's conversational multi-agent framework.
- **LangGraph**: LangChain's graph-based workflow framework.
- **CrewAI**: Simple role-based sequential pipeline framework.
- **Lead/Worker Pattern**: Smart model plans (once), simpler models execute (many times).
- **Shared Context Store**: Central memory where agents share information.

---
This section covers how multiple AI models coordinate to accomplish complex tasks.

---



### 7.2.0 Handshake Dual-Channel Inter-Model Communication (Normative) [ADD v02.120]

Handshake uses **two distinct channels** for inter-model coordination. This separation prevents â€œchat-as-stateâ€ failures and keeps contracts enforceable.

#### 7.2.0.1 Channel 1: Contract Channel (Task Board + Task Packets + Locus) (Normative)

**Purpose:** durable, authoritative work contracts.

- Task Board / Task Packets define scope, DONE_MEANS, acceptance criteria, and state transitions.
- Locus Work Packets track macro-task lifecycle, checklists, and artifacts.
- Micro-Task Executor reads MT definitions and produces outputs with validation.

**Hard rule:** authoritative task state MUST live in Contract Channel artifacts, never in mailbox chat.

#### 7.2.0.2 Channel 2: Role Mailbox (MailboxKind=COLLAB) (Normative)

**Purpose:** coordination messages between roles/models (handoffs, clarification, blockers).

- Role Mailbox is used to request clarifications, notify blockers, and hand off intermediate artifacts.
- Role Mailbox MUST NOT be treated as authoritative state.
- Any governance-relevant change MUST be transcribed into canonical artifacts (SpecIntent refinements, waivers, scope changes).
- [ADD v02.173] Role Mailbox threads MUST carry explicit lifecycle state, allowed-response posture, and linked stable identifiers so asynchronous collaboration does not depend on transcript order or unread badges alone.
- [ADD v02.173] A mailbox reply, acknowledgement, snooze, or escalation request MAY change mailbox triage posture, but it MUST NOT mutate authoritative work state unless a governed action or explicit transcription updates the linked record.

See the Role Mailbox schema and invariants in Â§2.6.8.10.

#### 7.2.0.3 Why Two Channels (Informative)

- Contract Channel provides enforceable structure and deterministic state.
- Mailbox provides flexible coordination without risking contract corruption.
- This mirrors the Lead/Worker pattern (Â§7.2.2) and the MT execution loop (Â§2.6.6.8).

#### 7.2.0.4 Message Flow Patterns (Normative) [ADD v02.120]

1. **Handoff**
   - Worker completes subtask â†’ sends `Handoff` message with artifact refs â†’ Orchestrator updates WP contract.
2. **Clarification loop**
   - Worker asks `ClarificationRequest` â†’ Orchestrator replies `ClarificationResponse` â†’ updated requirements are transcribed.
3. **Blocker**
   - Worker sends `Blocker` â†’ Orchestrator either unblocks (new artifact) or defers via governance decision.

#### 7.2.0.5 Multi-Model Infrastructure (Normative) [UPDATED v02.137]

The `MultiModelSession` is a governed runtime primitive (not a future extension). It represents the orchestrator's view of all active model sessions and their routing.

```typescript
// UPDATED v02.137
export interface MultiModelSession {
  session_id: string;                          // registry-level session group ID
  active_sessions: Record<string, ModelSession>; // session_id -> ModelSession (Â§4.3.9.12)
  routing_policy: RoutingPolicy;
  spawn_limits: SpawnLimits;                   // Â§4.3.9.15
  scheduler_config: SessionSchedulerConfig;    // Â§4.3.9.13
  last_swap_event?: string;
}

export type RoutingPolicy = {
  strategy: "round_robin" | "least_busy" | "affinity" | "broadcast" | "work_profile_driven";
  affinity_key?: string;                       // e.g., "wp_id" for WP-affinity routing
  broadcast_max_targets?: number;              // cap for broadcast strategy
};
```

**Session Registry (normative):** The system MUST maintain a `SessionRegistry` that tracks all active `ModelSession` instances, their states, and their parent-child relationships. The registry is the authority for session lifecycle; UI and scheduler query it.

[ADD v02.162] The Session Registry MUST also preserve enough information to correlate parallel model sessions to tracked Work Packets, Micro-Task occupancy, workflow runs, and Dev Command Center orchestration state without requiring ad hoc session scraping or console-local heuristics.

**Routing (normative):** The `work_profile_driven` strategy (default) delegates model selection to Work Profiles (Â§4.3.9.4). Other strategies are available for operator override or specialized patterns (e.g., broadcast for consensus validation).

**Cross-reference:** Session data model (Â§4.3.9.12), scheduler (Â§4.3.9.13), spawn (Â§4.3.9.15), observability (Â§4.3.9.18).


#### 7.2.0.6 Locus Integration (Informative)

Role Mailbox threads SHOULD reference `wp_id` and `mt_id` so that coordination is traceable. Implementations may also create â€œMailbox Thread Summariesâ€ as artifacts attached to the WP for audit/debug.

#### 7.2.0.7 Operator Console Integration (Informative)

Operator consoles SHOULD provide a Role Mailbox inspector surfaced with a qualified, configurable label (e.g., â€œRole Mailboxâ€ / â€œCollab Inboxâ€), not bare â€œInboxâ€:
- Thread list by role / WP / MT
- Message previews via `RoleMailboxBodyV0_5.summary`
- Correlation to Flight Recorder events and Locus artifacts


### 7.2.1 Framework Comparison: AutoGen vs LangGraph vs CrewAI

**Prerequisites:** Section 7.2 (Multi-Agent Orchestration)  
**Related to:** Section 4 (LLM Infrastructure)  
**Implements:** Orchestration framework choice  
**Read time:** ~6 minutes

**Orchestration frameworks help coordinate multiple AI agents. Each framework has a different approach and strengths.**

---

#### 7.2.1.1 Framework Philosophies

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              THREE APPROACHES TO ORCHESTRATION               â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  AUTOGEN (Microsoft)                                        â”‚
â”‚  Philosophy: Agents CONVERSE with each other                â”‚
â”‚                                                              â”‚
â”‚       Agent A â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–º Agent B                â”‚
â”‚          â”‚                              â”‚                    â”‚
â”‚          â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–º Agent C â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                   â”‚
â”‚                                                              â”‚
â”‚  Like: A meeting where experts discuss until done           â”‚
â”‚  Best for: Complex reasoning, human-in-loop                 â”‚
â”‚                                                              â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  LANGGRAPH (LangChain)                                      â”‚
â”‚  Philosophy: Tasks flow through a GRAPH of steps            â”‚
â”‚                                                              â”‚
â”‚       [Start]â”€â”€â–¶[Plan]â”€â”€â–¶[Execute]â”€â”€â–¶[Review]â”€â”€â–¶[End]      â”‚
â”‚                    â”‚                    â”‚                    â”‚
â”‚                    â””â”€â”€â”€â”€â”€â”€â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                   â”‚
â”‚                         (if review fails)                   â”‚
â”‚                                                              â”‚
â”‚  Like: A flowchart where you define exactly what happens    â”‚
â”‚  Best for: Predictable workflows, complex conditionals      â”‚
â”‚                                                              â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  CREWAI                                                     â”‚
â”‚  Philosophy: Agents have ROLES and work in SEQUENCE         â”‚
â”‚                                                              â”‚
â”‚       [Researcher]â”€â”€â–¶[Writer]â”€â”€â–¶[Editor]â”€â”€â–¶[Publisher]     â”‚
â”‚                                                              â”‚
â”‚  Like: An assembly line with specialists                    â”‚
â”‚  Best for: Simple, linear workflows                         â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.2.1.2 Detailed Comparison

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

#### 7.2.1.3 Decision Point

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    DECISION POINT                            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ What needs to be decided: Multi-agent orchestration frameworkâ”‚
â”‚                                                              â”‚
â”‚ Options researched:                                          â”‚
â”‚   â€¢ AutoGen - Conversational agents, Microsoft-backed        â”‚
â”‚   â€¢ LangGraph - Graph-based workflows, very flexible         â”‚
â”‚   â€¢ CrewAI - Simple role-based pipelines                    â”‚
â”‚                                                              â”‚
â”‚ Recommendation: START WITH AUTOGEN, consider LangGraph      â”‚
â”‚                                                              â”‚
â”‚ Rationale:                                                   â”‚
â”‚   â€¢ AutoGen balances power and approachability              â”‚
â”‚   â€¢ Good human-in-loop support (important for AI trust)     â”‚
â”‚   â€¢ Microsoft backing suggests long-term maintenance        â”‚
â”‚   â€¢ Can migrate to LangGraph if more control needed         â”‚
â”‚                                                              â”‚
â”‚ Tradeoffs:                                                   â”‚
â”‚   â€¢ Less explicit flow control than LangGraph               â”‚
â”‚   â€¢ Conversation logging can be verbose                     â”‚
â”‚   â€¢ May need custom work for complex branching              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.2.1.4 Key Takeaways

- âœ“ **AutoGen** recommended for initial development
- âœ“ **LangGraph** as alternative if explicit flow control needed
- âœ“ **CrewAI** too limited for complex Handshake workflows
- âœ“ All frameworks run locally with any LLM

---

### 7.2.2 The Lead/Worker Pattern

**Prerequisites:** Section 7.2 (Multi-Agent Orchestration)  
**Related to:** Section 4.2 (LLM Inference Runtimes)  
**Implements:** Cost-effective multi-model approach  
**Read time:** ~4 minutes

**Use a powerful model to PLAN, then cheaper models to EXECUTE. This balances quality and cost.**

---

#### 7.2.2.1 The Pattern Explained

â•â•â• CORE CONCEPT â•â•â•

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    LEAD/WORKER PATTERN                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  COMPLEX TASK: "Create a product launch plan with           â”‚
â”‚                 marketing copy and social media posts"      â”‚
â”‚                                                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚              LEAD (GPT-4 Cloud)                      â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚  "Here's the plan:                                  â”‚    â”‚
â”‚  â”‚   1. Executive summary (100 words)                  â”‚    â”‚
â”‚  â”‚   2. Target audience analysis                       â”‚    â”‚
â”‚  â”‚   3. Key messaging (3 bullet points)                â”‚    â”‚
â”‚  â”‚   4. Timeline with milestones                       â”‚    â”‚
â”‚  â”‚   5. Social posts: Twitter (3), LinkedIn (2)        â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚   Each section should follow format X..."           â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚  Cost: $0.15 (one complex reasoning call)           â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                           â”‚                                  â”‚
â”‚                           â–¼                                  â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚           WORKERS (Local Llama 3 13B)               â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚  Task 1: Write executive summary â”€â”€â”€â”€â”€â–¶ Done        â”‚    â”‚
â”‚  â”‚  Task 2: Write audience analysis â”€â”€â”€â”€â”€â–¶ Done        â”‚    â”‚
â”‚  â”‚  Task 3: Write key messaging â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¶ Done        â”‚    â”‚
â”‚  â”‚  Task 4: Create timeline â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¶ Done        â”‚    â”‚
â”‚  â”‚  Task 5: Write social posts â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¶ Done        â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚  Cost: $0.00 (local, unlimited)                     â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                                              â”‚
â”‚  TOTAL COST: ~$0.15 instead of ~$1.50+ if all cloud        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.2.3.2 Key Takeaways

- âœ“ Agents share context through a central store
- âœ“ File-based storage aligns with overall architecture
- âœ“ Vector store enables semantic search over past interactions
- âœ“ Essential for coherent multi-step tasks

---

### 7.2.4 Task Routing and Fallback Logic

**Prerequisites:** Section 3.1  
**Related to:** Section 4 (LLM Infrastructure)  
**Implements:** Intelligent model selection  
**Read time:** ~4 minutes

**The orchestrator must decide which model handles each task, and what to do if it fails.**

---

#### 7.2.4.1 Routing Decision Tree

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    TASK ROUTING LOGIC                        â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  NEW TASK ARRIVES                                           â”‚
â”‚         â”‚                                                    â”‚
â”‚         â–¼                                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                    â”‚
â”‚  â”‚ Is it code-related? â”‚â”€â”€â”€â”€ Yes â”€â”€â–¶ Code Llama            â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                                    â”‚
â”‚         â”‚ No                                                 â”‚
â”‚         â–¼                                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                    â”‚
â”‚  â”‚ Is it image gen?    â”‚â”€â”€â”€â”€ Yes â”€â”€â–¶ SDXL/ComfyUI          â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                                    â”‚
â”‚         â”‚ No                                                 â”‚
â”‚         â–¼                                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                    â”‚
â”‚  â”‚ Is it complex       â”‚â”€â”€â”€â”€ Yes â”€â”€â–¶ Lead/Worker           â”‚
â”‚  â”‚ multi-step?         â”‚            (GPT-4 â†’ Local)        â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                                    â”‚
â”‚         â”‚ No                                                 â”‚
â”‚         â–¼                                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                    â”‚
â”‚  â”‚ Default             â”‚â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¶ Local LLM (Llama 3)    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                                    â”‚
â”‚                                                              â”‚
â”‚                                                              â”‚
â”‚  IF ANY MODEL FAILS:                                        â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚ 1. Check error type                                 â”‚    â”‚
â”‚  â”‚ 2. If quality issue â†’ retry with larger model      â”‚    â”‚
â”‚  â”‚ 3. If timeout â†’ retry with smaller model           â”‚    â”‚
â”‚  â”‚ 4. If persistent failure â†’ escalate to cloud       â”‚    â”‚
â”‚  â”‚ 5. Log everything for debugging                    â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

**Key Takeaways**  
- AutoGen recommended for conversational multi-agent orchestration with good human-in-loop support.
- Lead/Worker pattern optimizes costs: cloud models plan, local models execute.
- Shared context store enables agents to collaborate without redundant processing.
- Task routing uses complexity analysis and confidence thresholds for intelligent fallback.

---

### 7.2.5 Local-First Agentic Work and MCP (Handshake Positioning)

Handshake is **local-first**. â€œAgentic workâ€ (multi-step tool use, planning/execution loops, background agents) MUST be designed so that a fully local/offline configuration is the default and remains first-class.

**Normative stance**
- Default: local model runtime + local tool execution; no network required for core workflows.
- Cloud models MAY be used, but only as an opt-in escalation path with explicit consent/capability gating (AÂ§11.1, AÂ§5.2).
- MCP MAY be used as an adapter/transport layer, but it MUST NOT be a required dependency for the core local product.

**Contract (local and remote behave the same at the artifact level)**
- Every agentic step MUST produce artifact-first outputs (workspace entity refs + hashes) and MUST emit Flight Recorder events with trace linkage (AÂ§2.1.5, AÂ§11.5).
- Remote results MUST be cacheable as artifacts so repeated runs can be offline/reproducible where policy allows (AÂ§2.3.8, AÂ§11.4).
- When a remote service is unavailable, the system MUST degrade deterministically (fallback to local, or fail with a structured Problem + evidence).

---

## 7.3 Collaboration and Sync

**Why**  
Multi-device and multi-user collaboration requires robust synchronization. This section covers how CRDT-based sync enables real-time collaboration while maintaining offline-first functionality.

**What**  
Explains sync architecture using Yjs, covers server infrastructure options, handles conflict resolution, and defines sharing/permissions model.

**Jargon**  
- **CRDT**: Conflict-free Replicated Data Typeâ€”data structures that automatically merge without conflicts.
- **Yjs**: JavaScript CRDT library chosen for real-time collaboration.
- **y-websocket**: Yjs sync provider using WebSocket connections.
- **y-indexeddb**: Yjs persistence provider using browser IndexedDB.
- **Awareness**: Yjs feature showing who's online and cursor positions.

---
This section covers how Handshake enables multiple users and devices to work together.

---

### 7.3.1 Understanding CRDTs

**Prerequisites:** Section 3.1 (Local-First Data Fundamentals)  
**Related to:** Section 3.1 (Offline-First Architecture)  
**Implements:** Conflict-free collaboration  
**Read time:** ~5 minutes

**CRDTs are special data structures that allow multiple people to edit simultaneously without conflictsâ€”even while offline.**

---

#### 7.3.1.1 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **CRDT** | Conflict-free Replicated Data Type - data that merges automatically | Enables real-time collaboration |
| **Yjs** | Most popular JavaScript CRDT library | Our likely choice for sync |
| **Automerge** | Alternative CRDT library | Fallback option |
| **Merge** | Combining two versions of a document | Happens automatically with CRDTs |
| **Operational Transform (OT)** | Older technique (Google Docs uses this) | CRDTs are newer and better for offline |

---

#### 7.3.1.2 How CRDTs Work (Simplified)

â•â•â• CORE CONCEPT â•â•â•

> Traditional documents: "Last write wins" (someone's work gets lost)
> 
> CRDT documents: "All writes merge" (everyone's work is preserved)

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚           TRADITIONAL SYNC (CONFLICTS!)                      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  Original: "Hello World"                                    â”‚
â”‚                                                              â”‚
â”‚  Alice (offline):  "Hello World!" (added !)                 â”‚
â”‚  Bob (offline):    "Hello Earth" (changed World)            â”‚
â”‚                                                              â”‚
â”‚  When both sync:                                            â”‚
â”‚  âŒ CONFLICT! Which version wins?                           â”‚
â”‚  â€¢ Keep Alice's? Bob loses his change.                      â”‚
â”‚  â€¢ Keep Bob's? Alice loses her change.                      â”‚
â”‚  â€¢ Show conflict dialog? Annoying.                          â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜

â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚           CRDT SYNC (NO CONFLICTS!)                         â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  Original: "Hello World"                                    â”‚
â”‚                                                              â”‚
â”‚  Alice (offline): Insert "!" at position 11                 â”‚
â”‚  Bob (offline):   Replace "World" with "Earth"              â”‚
â”‚                                                              â”‚
â”‚  When both sync:                                            â”‚
â”‚  âœ… CRDT merges both operations:                            â”‚
â”‚  Result: "Hello Earth!"                                     â”‚
â”‚                                                              â”‚
â”‚  Both changes preserved! No conflict dialog!                â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.3.1.3 Yjs: Our CRDT Choice

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    DECISION POINT                            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ What needs to be decided: CRDT implementation                â”‚
â”‚                                                              â”‚
â”‚ Options researched:                                          â”‚
â”‚   â€¢ Yjs - Most popular, used by many editors                â”‚
â”‚   â€¢ Automerge - Good, Rust implementation available         â”‚
â”‚   â€¢ Custom - Too much work                                   â”‚
â”‚                                                              â”‚
â”‚ Recommendation: YJS                                          â”‚
â”‚                                                              â”‚
â”‚ Rationale:                                                   â”‚
â”‚   â€¢ Tiptap (our editor) has Yjs integration built-in       â”‚
â”‚   â€¢ Large ecosystem and community                           â”‚
â”‚   â€¢ Works offline natively                                   â”‚
â”‚   â€¢ Can sync via any transport (WebSocket, WebRTC, file)   â”‚
â”‚                                                              â”‚
â”‚ Tradeoffs:                                                   â”‚
â”‚   â€¢ JavaScript-focused (need yrs for Rust interop)         â”‚
â”‚   â€¢ Learning curve for CRDT concepts                        â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.3.2.2 Key Takeaways

- âœ“ App is fully functional offline
- âœ“ Sync is optional, not required
- âœ“ CRDTs handle conflict-free merging
- âœ“ User chooses if/where to sync

---

### 7.3.3 Google Workspace Integration

**Prerequisites:** Section 3.1 (Offline-First)  
**Related to:** Section 2.1 (High-Level Architecture)  
**Implements:** Gmail, Drive, Calendar sync  
**Read time:** ~4 minutes

**Optionally sync with Google services: backup to Drive, import emails, show calendar events.**

---

#### 7.3.3.1 Integration Points

| Service | Integration | Priority |
|---------|-------------|----------|
| **Google Drive** | Backup workspace, sync files | [OPTIONAL] |
| **Gmail** | Import emails as documents | [OPTIONAL] |
| **Calendar** | Show events in calendar view | [OPTIONAL] |
| **Google Docs** | Export/import documents | [ADVANCED] |

---

#### 7.3.3.2 OAuth2 Flow

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    GOOGLE AUTH FLOW                          â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  1. User clicks "Connect Google Account"                    â”‚
â”‚                     â”‚                                        â”‚
â”‚                     â–¼                                        â”‚
â”‚  2. Opens system browser to Google login                    â”‚
â”‚                     â”‚                                        â”‚
â”‚                     â–¼                                        â”‚
â”‚  3. User grants permissions (minimal scopes)                â”‚
â”‚                     â”‚                                        â”‚
â”‚                     â–¼                                        â”‚
â”‚  4. Google redirects back to app with auth code             â”‚
â”‚                     â”‚                                        â”‚
â”‚                     â–¼                                        â”‚
â”‚  5. App exchanges code for tokens                           â”‚
â”‚                     â”‚                                        â”‚
â”‚                     â–¼                                        â”‚
â”‚  6. Tokens stored encrypted locally                         â”‚
â”‚                     â”‚                                        â”‚
â”‚                     â–¼                                        â”‚
â”‚  7. App can now call Google APIs                            â”‚
â”‚                                                              â”‚
â”‚  SECURITY: Tokens never leave user's machine                â”‚
â”‚  PRIVACY: Minimal scopes requested                          â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

**Key Takeaways**  
- Yjs provides the CRDT foundation for real-time collaboration.
- Sync server can be self-hosted or use managed services.
- Offline-first means changes always save locally first.
- Permissions model uses simple owner/editor/viewer roles.

---

## 7.4 Reference Application Analysis

**Why**  
Learning from similar applications avoids repeating their mistakes. This section summarizes insights from analyzing AppFlowy, AFFiNE, Obsidian, and Logseq.

**What**  
Analyzes four reference applications (their stacks, data models, sync approaches), identifies patterns to follow and patterns to avoid.

**Jargon**  
- **AppFlowy**: Flutter + Rust open-source Notion alternative.
- **AFFiNE**: Electron + React workspace with custom Rust CRDT (OctoBase).
- **Obsidian**: Electron + TypeScript note-taking app with thriving plugin ecosystem.
- **Logseq**: Electron + ClojureScript outliner with bidirectional linking.

---

### 7.4.1 Reference Applications

#### 7.4.1.1 AppFlowy
**Stack:** Flutter (Dart) + Rust backend  
**Data:** CRDT-based (yrs), RocksDB storage  
**Sync:** Offline-first CRDT via Supabase

**Key Insights:**
- âœ“ Flutter provides native performance and feel
- âœ“ Rust CRDT implementation is solid
- âš ï¸ Flutter limits JavaScript plugin ecosystem
- âš ï¸ Minimal plugin API currently

#### 7.4.1.2 AFFiNE
**Stack:** Electron + React/TypeScript  
**Data:** OctoBase (custom Rust CRDT)  
**Sync:** P2P CRDT, local-first

**Key Insights:**
- âœ“ "Everything is a block" model works well
- âœ“ Blocksuite component library is promising
- âš ï¸ Switched from Tauri to Electron (webview issues)
- âš ï¸ Performance issues with large documents
- âš ï¸ No mature plugin API yet

#### 7.4.1.3 Obsidian
**Stack:** Electron + TypeScript  
**Data:** Plain Markdown files  
**Sync:** Local vault with optional Obsidian Sync

**Key Insights:**
- âœ“ Thriving plugin ecosystem (hundreds of plugins)
- âœ“ Markdown files = portable, future-proof
- âœ“ Excellent community engagement
- âš ï¸ Some performance issues with huge vaults

#### 7.4.1.4 Logseq
**Stack:** Electron + ClojureScript  
**Data:** Markdown/EDN files, SQLite  
**Sync:** Git/WebDAV/LiveSync options

**Key Insights:**
- âœ“ Mature JS plugin API
- âœ“ Bidirectional linking works well
- âš ï¸ Performance issues with large graphs/pages
- âš ï¸ Team added pagination to mitigate

---

### 7.4.2 Lessons Learned

#### 7.4.2.1 Patterns to Follow

| Pattern | Why It Works | Handshake Application |
|---------|--------------|----------------------|
| **File-based storage** | Portable, user-owned data | âœ“ Already planned |
| **Block-based editing** | Flexible, AI-friendly | âœ“ Using Tiptap/BlockNote |
| **CRDT sync** | Offline-first, conflict-free | âœ“ Using Yjs |
| **Plugin API early** | Builds ecosystem | Plan internal APIs from start |

#### 7.4.2.2 Patterns to Avoid

| Anti-Pattern | What Went Wrong | Handshake Mitigation |
|--------------|-----------------|---------------------|
| **Full doc re-render** | AFFiNE lag on keystroke | Virtualization, incremental updates |
| **Monolithic DB** | Joplin RAM bloat | File-based with SQLite index only |
| **No export path** | Athens shutdown orphaned users | Standard formats, export from day 1 |
| **Tauri webview issues** | AFFiNE switched to Electron | Minimal Tauri responsibilities, test early |

---

**Key Takeaways**  
- Learn from others' mistakes before building.
- Performance at scale is a real concernâ€”virtualize and paginate.
- Export/migration paths are essential for user trust.
- Plugin ecosystems take years to buildâ€”start API design early.
- Test Tauri thoroughly early; keep its responsibilities minimal.

---

## 7.5 Development Workflow

**Why**  
Consistent development practices ensure code quality and team productivity. This section defines the tooling, processes, and standards for the project.

**What**  
Covers repository structure (monorepo with Turborepo), code quality tools (ESLint, Prettier, Ruff), CI/CD pipeline (GitHub Actions), testing strategy, and project health practices.
Cross-ref: sections 10/11 for Terminal/Monaco dev-surface requirements and shared capability/observability contracts to be exercised in workflows and CI.

**Jargon**  
- **Monorepo**: Single repository containing multiple packages/projects.
- **Turborepo**: Monorepo build tool with intelligent caching.
- **ESLint**: JavaScript/TypeScript linting tool.
- **Prettier**: Code formatter for consistent style.
- **Ruff**: Fast Python linter and formatter.
- **CI/CD**: Continuous Integration/Continuous Deployment pipeline.
- **Pre-commit Hooks**: Scripts that run before commits to catch issues early.

---
This section covers how to actually build Handshake efficiently.

---

### 7.5.1 Using AI Coding Assistants Effectively

**Prerequisites:** Section 4 (AI Models)  
**Related to:** Section 7.5 (Development Workflow)  
**Implements:** Development efficiency  
**Read time:** ~5 minutes

**The research documents provide a clear model for using AI assistants during development.**

---

#### 7.5.1.1 The Three-Layer Model

â•â•â• CORE CONCEPT â•â•â•

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚           AI ASSISTANTS IN DEVELOPMENT                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚         GPT-4 / CLAUDE (Architects)                  â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚  USE FOR:                                           â”‚    â”‚
â”‚  â”‚  â€¢ Feature specs and requirements                   â”‚    â”‚
â”‚  â”‚  â€¢ Architecture decisions                           â”‚    â”‚
â”‚  â”‚  â€¢ Trade-off analysis                               â”‚    â”‚
â”‚  â”‚  â€¢ Code review                                      â”‚    â”‚
â”‚  â”‚  â€¢ Debugging complex issues                         â”‚    â”‚
â”‚  â”‚  â€¢ Test strategy                                    â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                           â”‚                                  â”‚
â”‚                           â–¼                                  â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚         CODEX / CODE MODELS (Implementers)          â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚  USE FOR:                                           â”‚    â”‚
â”‚  â”‚  â€¢ Writing code from specs                          â”‚    â”‚
â”‚  â”‚  â€¢ Mechanical refactoring                           â”‚    â”‚
â”‚  â”‚  â€¢ Generating tests                                 â”‚    â”‚
â”‚  â”‚  â€¢ Writing boilerplate                              â”‚    â”‚
â”‚  â”‚  â€¢ Documentation comments                           â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                           â”‚                                  â”‚
â”‚                           â–¼                                  â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚         N8N / AUTOMATION (Operations)               â”‚    â”‚
â”‚  â”‚                                                      â”‚    â”‚
â”‚  â”‚  USE FOR:                                           â”‚    â”‚
â”‚  â”‚  â€¢ CI/CD workflows                                  â”‚    â”‚
â”‚  â”‚  â€¢ Health monitoring                                â”‚    â”‚
â”‚  â”‚  â€¢ Notifications                                    â”‚    â”‚
â”‚  â”‚  â€¢ External integrations                            â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.5.1.2 AI Development Workflow

| Phase | Use Generalist (GPT-4/Claude) | Use Code Model (Codex) |
|-------|------------------------------|------------------------|
| **Planning** | âœ“ Define specs, goals, non-goals | |
| **Architecture** | âœ“ Design systems, APIs | Scaffold structure |
| **Implementation** | Review PRs | âœ“ Write code from specs |
| **Testing** | Design test strategy | âœ“ Write test code |
| **Debugging** | âœ“ Analyze logs, hypothesize | Apply fixes |
| **Documentation** | âœ“ Write overviews | Docstrings, comments |

---

#### 7.5.2.4 Key Takeaways

- âœ“ **One health command** for all checks
- âœ“ Linters and formatters for consistency
- âœ“ Pre-commit hooks to catch issues early
- âœ“ Type annotations for AI and human safety

---

### 7.5.3 CI/CD and Testing Strategy

**Prerequisites:** Section 7.5 (Development Workflow)  
**Related to:** Section 7.6 (Development Roadmap)  
**Implements:** Automated quality assurance  
**Read time:** ~4 minutes

**Continuous Integration ensures every code change is tested automatically.**

---

#### 7.5.3.1 CI Pipeline

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    CI PIPELINE (on every push)               â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  1. LINT                                                    â”‚
â”‚     â””â”€ Ruff, ESLint                                        â”‚
â”‚                                                              â”‚
â”‚  2. TYPE CHECK                                              â”‚
â”‚     â””â”€ mypy, TypeScript                                    â”‚
â”‚                                                              â”‚
â”‚  3. UNIT TESTS                                              â”‚
â”‚     â””â”€ pytest, vitest (fast tests only)                    â”‚
â”‚                                                              â”‚
â”‚  4. BUILD                                                   â”‚
â”‚     â””â”€ Frontend bundle, backend validation                 â”‚
â”‚                                                              â”‚
â”‚  IF ALL PASS â†’ âœ… Ready to merge                            â”‚
â”‚  IF ANY FAIL â†’ âŒ Block merge, fix issues                   â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 7.5.3.2 Testing Pyramid

```
            â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
            â”‚   E2E     â”‚  Few, slow, high confidence
            â”‚   Tests   â”‚
            â””â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”˜
                  â”‚
         â”Œâ”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”
         â”‚ Integration   â”‚  Some, medium speed
         â”‚    Tests      â”‚
         â””â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜
                 â”‚
        â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”
        â”‚    Unit Tests    â”‚  Many, fast, low coupling
        â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

**Key Takeaways**  
- Monorepo structure with Turborepo enables efficient builds and caching.
- Consistent linting and formatting enforced through pre-commit hooks.
- CI/CD pipeline automates testing and deployment.
- Project health practices prevent technical debt accumulation.
- Calendar is a â€œLAW-backedâ€ subsystem: ship golden ICS fixtures, recurrence property tests, and provider sync simulations; mandatory in CI (Â§10.4.1, Â§5.4.6.4).
---

### 7.5.4 Governance Kernel: Mechanical Gated Workflow (Project-Agnostic) (HARD)

**Purpose**  
Define a reusable, project-agnostic governance kernel that enables:
- deterministic multi-role collaboration (Operator / Orchestrator / Coder / Validator)
- rigorous auditability (evidence-first; append-only logs)
- reliable handoff between small-context local models and large-context cloud models

This section is project-agnostic by design: it defines the workflow and artifacts that make work portable across projects. Handshake implements this kernel concretely via repo files and enforcement scripts (see `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`).

#### 7.5.4.1 Authority stack and precedence (kernel)

Kernel rule: precedence MUST be explicit, stable, and enforceable.

Recommended precedence order (highest â†’ lowest):
1. Platform/system constraints (sandbox, secrets, tool limits).
2. Project Codex (`<PROJECT> Codex vX.Y.md`).
3. Master Spec (`<PROJECT>_Master_Spec_vNN.NNN.md`) + `.GOV/spec/SPEC_CURRENT.md`.
4. Role protocols (`.GOV/roles/*/*_PROTOCOL.md`).
5. Repo-local guardrails (`AGENTS.md`).
6. Work authorization artifacts (task packets / refinements / board).
7. Mechanical enforcers (scripts/hooks/CI; command surface).

Conflict rule: higher source wins; overrides MUST be explicit and logged.

#### 7.5.4.2 Roles and separation of duties (kernel)

Roles are capability envelopes, not personas:
- **OPERATOR**: sets priorities; grants approvals for destructive/sync ops; issues one-time signatures.
- **ORCHESTRATOR**: translates spec â†’ executable work contracts; maintains board/traceability/audits; does not implement product code.
- **CODER**: implements only what the active task packet authorizes, within IN_SCOPE_PATHS; produces evidence/manifests.
- **VALIDATOR**: independent audit; verifies evidence against requirements; controls PASSâ†’commit gate sequence.

#### 7.5.4.3 Canonical governance artifacts (kernel)

Kernel objective: a fresh agent can reconstruct â€œwhat is trueâ€ by opening a small stable set of files.

Required artifacts (canonical locations):
- `.GOV/spec/SPEC_CURRENT.md`: single pointer to the current authoritative Master Spec.
- `.GOV/roles_shared/records/TASK_BOARD.md`: global execution state SSoT (minimal entries; details live in packets).
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`: Base WP â†’ Active Packet mapping (prevents revision ambiguity).
- `.GOV/refinements/<WP_ID>.md`: Technical Refinement Block artifact (ASCII-only; spec anchors; enrichment decision; approval evidence).
- `.GOV/task_packets/stubs/<WP_ID>.md`: non-executable backlog stub (no signature).
- `.GOV/task_packets/<WP_ID>.md`: executable task contract (ASCII-only; required headings; validation manifests).
- `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`: append-only signature log.
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, `.GOV/validator_gates/{WP_ID}.json`: gate state (deterministic JSON).
- `.GOV/templates/`: canonical templates for stubs/refinements/packets (prevents drift).

Failure mode if missing: small-context handoff breaks (agents must â€œremember chatâ€), and validation becomes social rather than mechanical.

#### 7.5.4.4 Task packet contract (kernel minimum)

An activated task packet MUST be the single executable authority for a coder and validator.

Minimum required sections (names are normative; minor formatting variations allowed if gate tooling matches deterministically):
- `## METADATA` (includes `WP_ID`, `BASE_WP_ID`, `Status`, `USER_SIGNATURE`, authoring role)
- `## TECHNICAL_REFINEMENT` (links refinement file)
- `## SCOPE` (explicit `IN_SCOPE_PATHS` and `OUT_OF_SCOPE`)
- `## QUALITY_GATE` (`TEST_PLAN`, `DONE_MEANS`, rollback hint)
- `## AUTHORITY` (spec baseline + spec target pointer + anchors + codex/board/registry pointers)
- `## BOOTSTRAP` (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)
- `## SKELETON` (interface-first design)
- `## IMPLEMENTATION` (only after approval gate)
- `## HYGIENE`
- `## VALIDATION` (per-file deterministic manifest blocks; COR-701-style)
- `## STATUS_HANDOFF` (handoff summary without claiming validation)
- `## EVIDENCE` (logs/outputs; append-only)
- `## VALIDATION_REPORTS` (validator append-only)

Binary phase gate requirement:
- A literal line containing exactly `SKELETON APPROVED` MUST exist outside code fences before implementation evidence is accepted.

#### 7.5.4.5 Gate semantics (kernel)

Kernel assumes mechanical gates enforce state transitions.

Orchestrator gates (state machine):
1. **REFINEMENT recorded** (refinement file is structurally complete; signature not allowed in same turn).
2. **SIGNATURE recorded** (one-time; requires deterministic USER_APPROVAL_EVIDENCE; forbidden if ENRICHMENT_NEEDED=YES).
3. **PREPARE recorded** (branch/worktree exists; coder assignment recorded).
4. **PACKET creation** (blocked unless refinement + gates + audit log all agree).

Coder gates:
- `gate-check`: enforces BOOTSTRAP â†’ SKELETON â†’ `SKELETON APPROVED` before implementation evidence.
- `pre-work`: blocks work unless packet+refinement are signed and checkpoint-committed (prevents artifact loss).
- `post-work`: enforces per-file manifest vs git diff (window + line-delta + pre/post hashes).

Validator gates (state machine):
- REPORT_PRESENTED â†’ USER_ACKNOWLEDGED â†’ WP_APPENDED â†’ COMMITTED (PASS-only).

#### 7.5.4.6 Small-context handoff (kernel)

Kernel rule: chat is not state. Any scope/requirement/approval/evidence MUST live in artifacts.

Minimum continuity requirements on handoff:
- Task Board status is current.
- Packet `## STATUS_HANDOFF` is current (what changed, what remains, next command).
- `## VALIDATION` manifests cover all changed non-doc files.
- Logs are appended in `## EVIDENCE` (no â€œtrust meâ€).

#### 7.5.4.7 CI/hook parity and drift control (kernel)

Kernel requirements:
- CI runs the same governance gates as local (or stricter).
- Determinism config is explicit (EOL policy, toolchain pinning).
- Drift is treated as a first-class failure mode (old codex/spec references in CI/hooks/docs are detected and remediated explicitly).

Kernel reference docs:
- Project-agnostic kernel: `.GOV/GOV_KERNEL/README.md` and `.GOV/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md` .. `.GOV/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md`.
- Handshake mapping (non-normative): `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`.

#### 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)

**Purpose**  
Handshake MUST implement the project-agnostic Governance Kernel (7.5.4; `.GOV/GOV_KERNEL/*`) as a project-parameterized **Governance Pack** so the same strict workflow can be generated and enforced for arbitrary projects (not Handshake-specific).

**Definitions**
- **Governance Pack**: a versioned bundle of templates + gate semantics that instantiate:
  - project codex,
  - role protocols,
  - canonical governance artifacts and templates,
  - mechanical gate tooling (scripts/hooks/CI) and a single command surface (e.g., `just`),
  - deterministic exports (including `.GOV/ROLE_MAILBOX/` when enabled by governance mode).

**Project identity (normative)**

```rust
pub struct ExternalToolPaths {
    pub cargo_target_dir: Option<String>, // project-specific; may be external
    pub node_package_manager: Option<String>,
    pub additional_paths: std::collections::HashMap<String, String>,
}

pub struct NamingPolicy {
    // Recommended defaults: underscore-separated, no spaces (shell/OS safe; deterministic parsing).
    pub master_spec_pattern: String, // e.g. "<PROJECT>_Master_Spec_vNN.NNN.md"
    pub codex_pattern: String,       // e.g. "<PROJECT>_Codex_vX.Y.md"
}

pub struct ProjectIdentity {
    pub project_code: String,            // short stable prefix, e.g. "COOK"
    pub project_display_name: String,    // human name
    pub naming_policy: NamingPolicy,
    pub language_layout_profile_id: String,   // always present; project-specific (no Handshake-hardcoded paths)
    pub role_mailbox_export_dir: String,      // MUST default to ".GOV/ROLE_MAILBOX/"
    pub external_tool_paths: ExternalToolPaths,
}
```

**Invariants (HARD)**
- Language/layout guardrails MUST always exist and MUST be project-specific (no Handshake-hardcoded paths).
- External tool paths MUST be explicit, prompted/configured per project, and persisted (workspace settings and repo-exported identity).
- The Governance Pack MUST NOT hardcode `Handshake_*` filenames when instantiating non-Handshake projects.
- For GOV_STANDARD and GOV_STRICT, the Trinity roles MUST be enforced (11.1.5.1).

**Conformance and alternate implementations (HARD)**
- Node/just/bash reference implementations are allowed and preferred for strict determinism.
- Alternate implementations (different language/tooling) are allowed ONLY if:
  - they enforce the same semantics,
  - they are deterministic,
  - and they ship a conformance proof (tests/harness) plus an explicit "intent" note describing equivalence and any deviations.

**Kernel parity rule (HARD)**
Any project claiming Governance Kernel conformance MUST be able to reconstruct, from canonical artifacts alone:
- current authoritative spec,
- authorized work and scope,
- evidence and remaining gates,
- active/in-progress/done/stub state,
- role mailbox transcripts (if used) via `.GOV/ROLE_MAILBOX/`.

**Repo/runtime boundary (HARD)**  
- `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
- `docs/` MAY exist as a temporary compatibility bundle only (non-authoritative governance state).
- Handshake product runtime MUST NOT read from or write to `/.GOV/` (hard boundary; enforce via CI/gates).
- Runtime governance state MUST live in product-owned storage. Handshake default: `.handshake/gov/` (configurable). This directory contains runtime governance state only.
- [ADD v02.181] Software-delivery governance SHALL be treated as one additive Governance Pack overlay profile inside Handshake rather than as a replacement for the general governance kernel.
- [ADD v02.181] Imported repo-governance artifacts MAY define overlay meaning, templates, checks, rubrics, and export snapshots, but once ingested they MUST be treated as definition/import sources instead of live product runtime authority.
- [ADD v02.181] When software-delivery governance is active, authoritative runtime truth for active work, approvals, validator posture, and closeout MUST resolve through product-owned runtime records under `.handshake/gov/` (or an equivalent configured runtime store). Repo exports and readable mirrors MAY remain canonical snapshots where allowed by the Governance Pack, but they MUST NOT become the live execution authority.
- [ADD v02.181] Imported overlay definitions MAY describe claim/lease, queued-instruction, retry-budget, or control-plane policy, but no imported repo artifact MAY directly grant live ownership, inject steer-next intent, or trigger start/cancel/close/recover semantics until those facts are materialized as product-owned runtime records.

---

#### 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) [ADD v02.180]

**Purpose**
Define a bounded, observable execution layer for imported governance checks so that Handshake validates software-delivery workflows through capability-gated, recorder-visible, product-owned execution instead of raw shell bypass.

**Definitions**
- **CheckDescriptor**: a typed execution record derived from a `GovernanceArtifactRegistryEntry` with `kind=Checks` or `kind=Rubrics`. It carries the check identifier, required capabilities, timeout_ms, input schema, and version provenance from the registry.
- **CheckResult**: a typed result contract with exactly five variants:
  - `PASS` -- check completed and all assertions satisfied
  - `FAIL` -- check completed and one or more assertions failed
  - `BLOCKED` -- check could not execute due to capability denial or precondition failure
  - `ADVISORY_ONLY` -- check completed but findings are informational and do not gate progress
  - `UNSUPPORTED` -- check kind or descriptor version is not executable in the current runtime
- **CheckRunner**: the product service that executes a `CheckDescriptor` through the Unified Tool Surface Contract and produces a `CheckResult` with evidence.

**Execution Lifecycle**
The CheckRunner MUST implement a three-phase bounded lifecycle:
1. **PreCheck**: validate `CheckDescriptor` schema, verify required capabilities through `CapabilityGate`, and confirm `timeout_ms` is within runtime bounds. Failure here MUST produce `CheckResult::Blocked` immediately without proceeding to execution.
2. **Check**: invoke the check body through the `governance.check.run` tool surface. Execution is bounded by `timeout_ms`. A timeout or execution error MUST produce `CheckResult::Blocked`.
3. **PostCheck**: capture the raw result, map it to the `CheckResult` enum, store evidence artifacts with content hash, and emit Flight Recorder events.

**Tool Surface**
The `governance.check.run` tool_id MUST be registered under the Unified Tool Surface Contract (6.0.2) with:
- `side_effect: GOVERNED_WRITE`
- Required capabilities declared in the `CheckDescriptor`
- Input schema: `{ check_id: string, descriptor_ref: string, input_args: object }`
- Output schema: `CheckResult` JSON

**Flight Recorder Events**
Every check execution MUST emit the following FR events:
- `FR-EVT-GOV-CHECK-001` (`governance.check.started`): payload includes `check_id`, `session_id`, `check_descriptor_hash`
- `FR-EVT-GOV-CHECK-002` (`governance.check.completed`): payload includes `check_id`, `session_id`, `result_status`, `duration_ms`, `evidence_artifact_id`
- `FR-EVT-GOV-CHECK-003` (`governance.check.blocked`): payload includes `check_id`, `session_id`, `blocked_reason`

FR events MUST be emitted for all result variants including `BLOCKED` and `UNSUPPORTED`. Silent skip is prohibited.

**Additive Overlay Rule**
Imported governance checks MUST extend the product governance surface additively. No imported check MAY:
- overwrite or disable native Handshake governance rules
- modify base-envelope structured collaboration records
- acquire capabilities beyond those declared in its `CheckDescriptor`

**Validator-gate convergence (HARD)** [ADD v02.181]
- Software-delivery validation posture MUST resolve through a dedicated product-owned validator-gate runtime record family or an equivalent canonical runtime record keyed by stable work and gate identifiers.
- `CheckResult` executions MAY contribute evidence and status updates to that canonical gate state, but a raw check result MUST NOT become workflow truth or closeout truth by itself.
- `PASS`, `FAIL`, `BLOCKED`, `ADVISORY_ONLY`, and `UNSUPPORTED` outcomes MUST remain queryable through canonical gate state together with evidence references and the originating descriptor provenance.
- When validator posture participates in workflow progression, closeout, cancellation, or recovery, the canonical gate view MUST also preserve any required authority proof, claim/lease posture, checkpoint lineage, and queued follow-up state that explains why work may or may not advance.

**Unsupported Checks**
A check descriptor with an unrecognized `kind`, unsupported schema version, or missing required execution surface MUST return `CheckResult::Unsupported` with an explicit reason string. Silent skip is prohibited.

**Storage**
All `CheckDescriptor` and `CheckResult` persistence MUST go through the `Database` trait boundary. No direct SQLite calls outside the storage module are permitted.

---

#### 7.5.4.10 Governance Pack: Template Volume (HARD)

**Purpose**  
This section inlines the canonical Governance Pack templates used to instantiate strict multi-role governance in arbitrary projects.
These templates MUST be rendered with project-specific values (no Handshake-hardcoding) before use.

**Hard rule (Instantiation)**
- The Governance Pack export MUST include these templates with ALL placeholders resolved.
- The exported repo MUST provide a single deterministic command surface (e.g., `just pre-work`, `just post-work`, `just validate-workflow`).
- Project-specific naming/layout/tool paths MUST live in `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md` (do not hardcode in templates).
- Any alternate implementation MUST preserve semantics and determinism (7.5.4.8).
- [ADD v02.154] Governance Pack export is also a product runtime surface: it MUST run through the Workflow Engine, enforce the server-side `export.governance_pack` capability profile, emit a `governance_pack_export` Flight Recorder event, and persist export artifacts/records with portable manifest semantics.

##### 7.5.4.10.1 Placeholder Glossary (HARD)
- `{{PROJECT_CODE}}`: short stable code, e.g., `COOK`.
- `{{PROJECT_DISPLAY_NAME}}`: human name, e.g., `Cooking App`.
- `{{PROJECT_PREFIX}}`: filesystem-safe prefix for filenames; recommended: `{{PROJECT_CODE}}`.
- `{{CODEX_VERSION}}`: codex version string (e.g., `1.4`).
- `{{CODEX_FILENAME}}`: `{{PROJECT_PREFIX}}_Codex_v{{CODEX_VERSION}}.md`.
- `{{ISSUE_PREFIX}}`: issue prefix for TODO tagging, e.g., `COOK` (used as `TODO({{ISSUE_PREFIX}}-1234)` / error codes).
- `{{MASTER_SPEC_FILENAME}}`: current master spec filename (repo root).
- `{{FRONTEND_ROOT_DIR}}`, `{{FRONTEND_SRC_DIR}}`: frontend layout roots (project-specific).
- `{{BACKEND_ROOT_DIR}}`, `{{BACKEND_CRATE_DIR}}`, `{{BACKEND_SRC_DIR}}`, `{{BACKEND_MIGRATIONS_DIR}}`: backend layout roots (project-specific).
- `{{BACKEND_JOBS_DIR}}`, `{{BACKEND_LLM_DIR}}`, `{{BACKEND_STORAGE_DIR}}`, `{{BACKEND_OBSERVABILITY_DIR}}`, `{{BACKEND_API_DIR}}`, `{{BACKEND_LOCAL_MODELS_DIR}}`, `{{BACKEND_PIPELINE_DIR}}`, `{{BACKEND_UTIL_DIR}}`: backend subdirectories used by enforcement scripts and protocols (project-specific).
- `{{CARGO_TARGET_DIR}}`: external Cargo target dir (project-specific; may be outside repo).
- `{{POSTGRES_TEST_DB}}`: CI/test Postgres database name (project-specific).
- `{{DEFAULT_BASE_BRANCH}}`: default base branch name for role worktrees (operator machine; e.g., `main`).
- `{{OPERATOR_WORKTREE_DIR}}`, `{{OPERATOR_BRANCH}}`, `{{ORCHESTRATOR_WORKTREE_DIR}}`, `{{ORCHESTRATOR_BRANCH}}`, `{{VALIDATOR_WORKTREE_DIR}}`, `{{VALIDATOR_BRANCH}}`: local role worktree mapping (operator machine; see `.GOV/roles_shared/docs/ROLE_WORKTREES.md`).

##### 7.5.4.10.2 Template Index (HARD)
| Path | Intent |
|------|--------|
| `AGENTS.md` | Repo-level guardrails that constrain agent behavior (hard rules). |
| `{{CODEX_FILENAME}}` | Project Codex: deterministic enforcement laws and invariants (role-agnostic). |
| `justfile` | Single command surface for governance gates and hygiene (mechanical enforcement). |
| `.gitattributes` | Deterministic line-ending policy for governance artifacts (drift control). |
| `.cargo/config.toml` | Deterministic build artifact location (keeps repo clean; avoids CI/worktree drift). |
| `deny.toml` | Supply-chain policy config for cargo-deny (license/advisory/bans/sources). |
| `.github/workflows/ci.yml` | CI parity: runs the same (or stricter) mechanical gates as local. |
| `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md` | Project-specific invariants (identity, naming, layout profile, tool paths). REQUIRED for Governance Pack instantiation. |
| `.GOV/spec/SPEC_CURRENT.md` | Authoritative pointer to the current Master Spec and Governance Reference (drift guard target). |
| `.GOV/roles_shared/docs/START_HERE.md` | Navigation pack for humans and agents (canonical entry point + workflow commands). |
| `.GOV/roles_shared/docs/ARCHITECTURE.md` | Module/area map and allowed dependency boundaries (architecture drift control). |
| `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` | Debug runbook (first-5-minutes flow + CI triage map). |
| `.GOV/roles_shared/PAST_WORK_INDEX.md` | Archaeology pointers (prevents guesswork when context is missing). |
| `.GOV/roles_shared/docs/OWNERSHIP.md` | Path ownership routing map for review/triage. |
| `.GOV/roles_shared/records/TASK_BOARD.md` | Machine-checkable shared work memory (WP lifecycle; STUB/IN_PROGRESS/DONE). |
| `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` | Single source of truth mapping Base WP -> Active Packet (keeps Master Spec WP-free). |
| `.GOV/roles_shared/records/SIGNATURE_AUDIT.md` | Central registry of consumed USER_SIGNATURE tokens (anti-replay / audit trail). |
| `.GOV/roles_shared/docs/QUALITY_GATE.md` | Risk-tiered quality gate contract (pre/post-work expectations; validator posture). |
| `.GOV/roles_shared/records/OSS_REGISTER.md` | Supply-chain manifest: dependency licenses, integration modes, and approval notes. |
| `.GOV/roles_shared/docs/ROLE_WORKTREES.md` | Local mapping of role -> (worktree directory, branch) for the operator machine. |
| `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` | Mechanical Orchestrator gate state model (initial empty state). |
| `.GOV/validator_gates/{WP_ID}.json` | Mechanical Validator gate state model (per-WP; merge-safe). |
| `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` | Orchestrator role protocol (refinement loop, signature gate, delegation contract). |
| `.GOV/roles/coder/CODER_PROTOCOL.md` | Coder role protocol (implementation rules, self-checks, mechanical gate compliance). |
| `.GOV/roles/validator/VALIDATOR_PROTOCOL.md` | Validator role protocol (independent audit, evidence requirements, verdict semantics). |
| `.GOV/agents/AGENT_REGISTRY.md` | Registry of known agents/roles/models and their intended use (routing aid). |
| `.GOV/GOV_KERNEL/README.md` | Governance Kernel overview (project-agnostic). |
| `.GOV/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md` | Kernel: authority stack and roles. |
| `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md` | Kernel: canonical artifacts and contracts. |
| `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md` | Kernel: gate semantics and enforcers. |
| `.GOV/GOV_KERNEL/04_SMALL_CONTEXT_HANDOFF.md` | Kernel: small-context handoff rules. |
| `.GOV/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md` | Kernel: CI/hook parity and config determinism. |
| `.GOV/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md` | Kernel: versioning and drift control. |
| `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md` | Reference mapping to a concrete repo implementation (non-normative example; optional export). |
| `.GOV/templates/TASK_PACKET_TEMPLATE.md` | Canonical task packet template (Gate 0 input). |
| `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md` | Canonical stub task packet template (pre-activation; no signature). |
| `.GOV/templates/REFINEMENT_TEMPLATE.md` | Canonical refinement template (required before signature). |
| `.GOV/templates/AI_WORKFLOW_TEMPLATE.md` | Reusable AI workflow template for other repos/projects. |
| `.GOV/scripts/hooks/pre-commit` | Local git hook enforcing codex checks and quick hygiene at commit time. |
| `.GOV/scripts/close-wp-branch.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/codex-check-test.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/create-task-packet.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/new-api-endpoint.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/new-react-component.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/scaffold-check.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/spec-current-check.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/worktree-add.mjs` | Repo script (governance support or scaffolding helper). |
| `.GOV/scripts/README.md` | Script directory documentation (how to use gates/tools). |
| `.GOV/scripts/fixtures/forbidden_fetch.ts` | Fixture for codex-check-test (ensures gates fail when they should). |
| `.GOV/scripts/fixtures/forbidden_todo.txt` | Fixture for codex-check-test (ensures gates fail when they should). |
| `.GOV/scripts/validation/ci-traceability-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/codex-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/cor701-sha.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/cor701-spec.json` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/gate-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/orchestrator_gates.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/post-work-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/pre-work-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/refinement-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/task-board-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/task-packet-claim-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-coverage-gaps.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-dal-audit.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-error-codes.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-git-hygiene.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-hygiene-full.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-packet-complete.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-phase-gate.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-scan.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-spec-regression.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator-traceability.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/validator_gates.mjs` | Mechanical governance gate (see filename + internal docstrings). |
| `.GOV/scripts/validation/worktree-concurrency-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |

##### 7.5.4.10.3 Template Bodies (HARD)
<!-- GOV_PACK_TEMPLATE_VOLUME_BEGIN -->

###### Template File: `AGENTS.md`
Intent: Repo-level guardrails that constrain agent behavior (hard rules).
````md
<INSTRUCTIONS>
## {{PROJECT_DISPLAY_NAME}} Repo Guardrails (HARD RULES)

### No destructive cleanup
- Do NOT run destructive commands that can delete/overwrite work (especially untracked files) unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If any cleanup/reset is requested, make it reversible first: `git stash push -u -m "SAFETY: before <operation>"`, then show what would be deleted (`git clean -nd`) and wait for explicit approval.

### Branching & concurrency
- Default: one WP = one feature branch (e.g., `feat/WP-{ID}`).
- When more than one coder/WP is active concurrently, use `git worktree` per active WP (separate working directories). Do NOT share a single working tree across concurrent WPs.

### Safety commit gate (prevents packet/refinement loss)
- After creating a WP task packet + refinement and obtaining `USER_SIGNATURE`, create a checkpoint commit on the WP branch that includes:
  - `.GOV/task_packets/WP-{ID}.md`
  - `.GOV/refinements/WP-{ID}.md`
</INSTRUCTIONS>

````

###### Template File: `{{CODEX_FILENAME}}`
Intent: Project Codex: deterministic enforcement laws and invariants (role-agnostic).
````md
# {{PROJECT_DISPLAY_NAME}} Codex v{{CODEX_VERSION}} (AI Autonomy with Deterministic Enforcement)

## 0. Meta

[CX-000] NAME: {{PROJECT_DISPLAY_NAME}} Codex
[CX-001] VERSION: v{{CODEX_VERSION}}
[CX-002] PURPOSE: Define repo layout, key invariants, and AI assistant behaviour for the {{PROJECT_DISPLAY_NAME}} project. Optimized for AI-autonomous software engineering with deterministic workflow enforcement and "Main-Body First" specification discipline.

---

## 1. LAW Stack and Precedence

[CX-010] LAW_1: This codex (`{{PROJECT_DISPLAY_NAME}} Codex v{{CODEX_VERSION}}`) is the primary implementation + behaviour reference.
[CX-011] LAW_2: The {{PROJECT_DISPLAY_NAME}} Master Spec (`{{PROJECT_PREFIX}}_Master_Spec_*.md`) defines product intent and architecture; only provided slices are binding in a given session.
[CX-012] LAW_3: Subsystem specs (L1 docs) in `/.GOV/operator/docs_local/` are binding when explicitly designated for a task.
[CX-013] LAW_4: Bootloaders (Micro-Logger, Diary, etc.) are additional behavioural LAW when either (a) the user declares the session bootloader-governed, or (b) a bootloader artefact is present in-session and not explicitly disabled.

[CX-020] PRECEDENCE_PRODUCT: For product behaviour and high-level architecture, LAW_2 and relevant LAW_3 override this codex when conflict exists.
[CX-021] PRECEDENCE_IMPL: For repo layout and assistant behaviour, this codex (LAW_1) applies unless the user explicitly overrides it.
[CX-022] PRECEDENCE_BEHAVIOUR: When a bootloader is active, its behavioural rules stack with this codex; bootloader governs *how* to act, specs + codex govern *what* may change.

[CX-030] UNKNOWN_SPEC: The assistant MUST treat any non-provided parts of LAW_2 / LAW_3 as unknown and MUST NOT assume, invent, or rely on specific content from them.
[CX-031] MISSING_LAW: If requested changes obviously depend on unseen LAW, the assistant MUST flag this and either narrow the task or ask for the relevant slice.

---

## 2. Hard Invariants (Core Rules)

[CX-100] HARD_RDD: The Raw / Derived / Display separation is a hard architectural invariant for document-like content.
[CX-101] HARD_LLM_CLIENT: All LLM / external AI calls MUST go through a shared client abstraction in `{{BACKEND_LLM_DIR}}` (e.g. `LLMClient`).
[CX-102] HARD_NO_DIRECT_HTTP: Jobs and feature modules MUST NOT bake provider-specific HTTP calls or SDK logic directly; they MUST use the shared client or adapters.
[CX-103] HARD_STORAGE_LAYER: Only storage modules under `{{BACKEND_STORAGE_DIR}}` (or clearly marked equivalents) MAY talk directly to DB/filesystem.
[CX-104] HARD_LOGGING: Production code MUST use shared logging utilities under `{{BACKEND_OBSERVABILITY_DIR}}` and SHOULD avoid `print()` outside tests and `/archive/`.
[CX-105] HARD_NO_LAW_EDIT: The assistant MUST NOT edit the Master Spec or this codex unless the user explicitly requests spec / LAW changes.
[CX-106] HARD_NO_TOPDIR: The assistant MUST NOT introduce new top-level directories without explicit user confirmation.

[CX-107] HARD_NO_DESTRUCTIVE_OPS: The assistant MUST NOT run destructive commands that can delete/overwrite work (especially untracked files) unless the user explicitly authorizes it in the same turn; show what would change and wait for approval before proceeding.

[CX-108] HARD_GIT_WORKTREE_REWRITE_CONSENT (HARD): The assistant MUST NOT run git commands that rewrite/hide the on-disk working tree unless the user explicitly authorizes it in the same turn. This includes: `git stash`, `git checkout`, `git switch`, `git merge`, `git rebase`, `git reset`, and `git clean`.

[CX-598] MAIN-BODY ALIGNMENT INVARIANT (HARD): A Phase or Work Packet is NOT DONE simply by checking off a Roadmap bullet. "Done" is defined as the 100% implementation of every technical rule, schema, and "LAW" block found in the Main Body (Â§1-6 or Â§9-11) that governs that roadmap item. This includes every line of text, idea, or constraint in the corresponding Main Body section. If a roadmap item is "checked" but the corresponding Main Body logic is missing, the task is BLOCKED. i as user do not declare a phase finished as everything in the roadmap is done, this means must deliverables as also every other line of text in that phase and the coresponding text, ideas or other in the master spec main body.

[CX-598A] ROADMAP_COVERAGE_MATRIX (HARD): The Master Spec Roadmap (7.6.1) MUST maintain a section-level Coverage Matrix listing every non-Roadmap section number (all `## X.Y` headings outside 7.6 plus the top-level `# 9.` section), including whether it is Main Body authority (CX-598) and which phase(s) cover it. If the matrix is missing/incomplete/duplicated/out-of-date, planning and phase-closure claims are BLOCKED until the matrix is corrected via Spec Enrichment.

[CX-599] CROSS-PHASE GOVERNANCE CONTINUITY: All requirements for Spec Alignment, Quality Gates, and Evidence-Based Reporting are cumulative. These requirements carry over automatically to Phase 2, 3, and all future work. Starting a new Phase never relaxes the rules of the previous ones.

---

## 3. Repository Layout (Guiding Structure)

[CX-200] ROOT_BACKEND: `{{BACKEND_ROOT_DIR}}/` SHOULD host the backend (language-agnostic: Rust/Python/etc.): orchestrator, job engine, services.
[CX-201] ROOT_FRONTEND: `{{FRONTEND_ROOT_DIR}}/` SHOULD host the desktop UI (e.g. Tauri + React).
[CX-202] ROOT_SHARED: `/src/shared/` SHOULD host shared types, DTOs, and protocol definitions.
[CX-203] ROOT_DOCS_LOCAL: `/.GOV/operator/docs_local/` SHOULD host local design docs and subsystem (L1) specs.
[CX-204] ROOT_ARCHIVE: `/archive/` SHOULD host experiments, throwaways, and dead ends only.
[CX-205] ROOT_SCRIPTS: `/.GOV/scripts/` SHOULD host dev/ops scripts (setup, run, tests, maintenance).
[CX-206] ROOT_TESTS: `/tests/` SHOULD host automated tests (unit, integration, end-to-end).
[CX-207] ROOT_DOCS: Root `*.md` files SHOULD hold Master Spec, Codex, roadmap, and other high-level docs.

[CX-208] ROOT_DOCS_CANONICAL: `/.GOV/` MUST contain canonical operational docs used for onboarding, navigation, and debugging.
[CX-209] NAVIGATION_PACK_FILES: `/.GOV/roles_shared/` MUST include (at minimum): `START_HERE.md`, `SPEC_CURRENT.md`, `ARCHITECTURE.md`, `RUNBOOK_DEBUG.md`.
[CX-213] TASK_PACKETS_DIR: `/.GOV/task_packets/` MUST exist and MUST contain task packet files for all active and recent work.
[CX-214] ROOT_APP_CURRENT: If `{{FRONTEND_ROOT_DIR}}/` exists, it SHOULD be treated as the primary application root (frontend in `{{FRONTEND_SRC_DIR}}/`, backend in `{{FRONTEND_SRC_DIR}}-tauri/`) unless `.GOV/roles_shared/docs/ARCHITECTURE.md` explicitly states otherwise.
[CX-215] DOCS_LOCAL_STAGING: `/.GOV/operator/docs_local/` SHOULD be treated as staging/drafts. Assistants MUST NOT treat `/.GOV/operator/docs_local/` as canonical onboarding/debugging guidance unless a document is explicitly promoted into `/.GOV/`.
[CX-216] PAST_WORK_INDEX: `/.GOV/roles_shared/` SHOULD include a `PAST_WORK_INDEX.md` (or equivalent) that links to older root-level specs/logs and `/.GOV/operator/docs_local/` drafts, so future maintainers can find prior work quickly without guesswork.

[CX-217] TASK_BOARD: `/.GOV/roles_shared/records/TASK_BOARD.md` MUST exist and serve as the high-level, at-a-glance status tracker.
- Orchestrator manages planning states (Ready for Dev/Blocked; Stub Backlog).
- Coders manage execution state in the **task packet** (set `**Status:** In Progress` + claim fields) and produce a docs-only bootstrap commit early.
- Validator maintains the Operator-visible `main` Task Board via docs-only "status sync" commits (update `## In Progress`; optionally also update `## Active (Cross-Branch Status)` for branch/coder visibility).

[CX-210] NEW_TOP_DIR_DOC: When new top-level directories are added with user approval, they SHOULD be documented in a future codex version.

[CX-220] BACKEND_JOBS: `{{BACKEND_JOBS_DIR}}/` SHOULD contain job engine and concrete job implementations.
[CX-221] BACKEND_LLM: `{{BACKEND_LLM_DIR}}/` SHOULD contain LLM client abstractions and provider adapters.
[CX-222] BACKEND_LOCAL_MODELS: `{{BACKEND_LOCAL_MODELS_DIR}}/` SHOULD contain local model runners (Ollama/vLLM, ASR, vision, etc.).
[CX-223] BACKEND_PIPELINE: `{{BACKEND_PIPELINE_DIR}}/` SHOULD contain Raw/Derived/Display pipeline logic, parsing, indexing, and sync.
[CX-224] BACKEND_STORAGE: `{{BACKEND_STORAGE_DIR}}/` SHOULD contain persistence logic (DB, filesystem, blobs) and migrations.
[CX-225] BACKEND_OBSERVABILITY: `{{BACKEND_OBSERVABILITY_DIR}}/` SHOULD contain logging, metrics, tracing, and debug utilities.
[CX-226] BACKEND_API: `{{BACKEND_API_DIR}}/` SHOULD contain API surface exposed to the frontend (HTTP, IPC, etc.).
[CX-227] BACKEND_UTIL: `{{BACKEND_UTIL_DIR}}/` SHOULD contain generic utilities that avoid app-specific dependencies.

[CX-230] FRONTEND_APP: `{{FRONTEND_SRC_DIR}}/` SHOULD hold shell, routing, and layout.
[CX-231] FRONTEND_FEATURES: `{{FRONTEND_SRC_DIR}}/features/` SHOULD hold feature modules (editor, file browser, jobs view, logs view, etc.).
[CX-232] FRONTEND_COMPONENTS: `{{FRONTEND_SRC_DIR}}/components/` SHOULD hold reusable UI components.
[CX-233] FRONTEND_STATE: `{{FRONTEND_SRC_DIR}}/state/` SHOULD hold client-side state/store logic.
[CX-234] FRONTEND_API: `{{FRONTEND_SRC_DIR}}/api/` SHOULD hold the client API layer talking to the backend.
[CX-235] FRONTEND_STYLES: `{{FRONTEND_SRC_DIR}}/styles/` SHOULD hold global styles and theme.

[CX-240] ARCHIVE_NON_PROD: Code in `/archive/` SHOULD NOT be treated as production and SHOULD NOT be wired in as a core dependency without explicit refactor.

---

## 4. Architectural Invariants (Detailed)

### 4.1 Raw / Derived / Display

[CX-300] RDD_DEF_RAW: RAW is canonical stored content (closest to DB/disk).
[CX-301] RDD_DEF_DERIVED: DERIVED is computed artefacts (indexes, embeddings, summaries, ASTs, etc.).
[CX-302] RDD_DEF_DISPLAY: DISPLAY is UI-oriented views (annotated text, layout, markers).

[CX-310] RDD_MUTATE_RAW: Persistent content changes SHOULD be expressed at the RAW layer.
[CX-311] RDD_RECOMPUTE: DERIVED and DISPLAY SHOULD be recomputed or refreshed from RAW rather than used as write-back sources.
[CX-312] RDD_SHORTCUTS: Shortcuts that temporarily bypass this pipeline MAY be used for experiments but SHOULD be clearly marked as technical debt with rationale.

### 4.2 LLM Client and External Tools

[CX-320] LLM_SINGLE_CLIENT: All LLM calls MUST flow through the shared client / adapter layer in `{{BACKEND_LLM_DIR}}/`.
[CX-321] LLM_PROVIDER_WRAP: Provider-specific logic SHOULD live in dedicated adapters, not scattered across jobs.
[CX-322] LLM_CLIENT_DUTIES: The shared client SHOULD handle routing, provider selection, token budgeting, retries, and logging.

### 4.3 Logging and Observability

[CX-330] LOGGING_SHARED_UTIL: Production code SHOULD use shared logging utilities in `{{BACKEND_OBSERVABILITY_DIR}}/`.
[CX-331] LOGGING_PRINT_LIMIT: `print()` SHOULD be limited to tests and `/archive/` experiments.
[CX-332] LOGGING_CONTEXT: Logs SHOULD include enough context (job IDs, doc IDs, user/session IDs where helpful) to debug issues.

[CX-333] LOG_ATTRIBUTION: Work artefacts (task packets, task board entries, milestone logs, review notes, commit messages) SHOULD include a stable `AGENT_ID` and `ROLE` so "who did what" remains searchable months later.
[CX-334] AGENT_REGISTRY: The repo SHOULD keep an `AGENT_REGISTRY` (`/.GOV/agents/AGENT_REGISTRY.md`) mapping `AGENT_ID` -> current model/tooling + responsibility; changes to mappings SHOULD be logged.
[CX-335] LOG_MODEL_LABELS_OPTIONAL: If model/vendor names are captured for convenience, they SHOULD be treated as secondary labels (not primary identifiers) and SHOULD live in structured metadata fields (not scattered through free text), subject to any active bootloader constraints.



#### 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]

**Addresses:** GAP A (no first-class session/thread data model for LLM conversations).  
**Informed by:** OpenClaw session persistence (`acquireSessionWriteLock`, session file model), TinyClaw durable SQLite queue, nullclaw encrypted session state.

##### 4.3.9.12.1 Purpose

A `ModelSession` is the persistent, addressable unit of a model conversation in Handshake. Without it, parallel runs devolve into independent completion calls with no coherent steering, audit, or governance.

##### 4.3.9.12.2 Schema (Normative)

```yaml
# ADD v02.137
ModelSession:
  session_id: string                     # stable UUID; referenced across UI, logs, artifacts, governance receipts
  parent_session_id: string | null       # for spawned child sessions (see Â§4.3.9.15)
  spawn_depth: int                       # 0 = root; incremented on spawn
  state: enum [CREATED, ACTIVE, PAUSED, BLOCKED, COMPLETED, FAILED, CANCELLED]
  model_id: ModelId
  backend: ModelBackend
  parameter_class: ParameterClass
  role: string                           # Orchestrator / Coder / Validator / custom
  wp_id: string | null                   # bound Work Packet
  mt_id: string | null                   # bound Micro-Task
  target_entity_ids: string[] | null     # optional worksurface/entity binding (Design Studio Phase 2+)
  work_profile_id: string | null
  execution_mode: ExecutionMode
  created_at: string                     # ISO 8601
  updated_at: string
  # Memory / context policy
  memory_policy: enum [EPHEMERAL, SESSION_SCOPED, WORKSPACE_SCOPED]
  context_window_used: int | null        # tokens currently consumed
  context_window_max: int | null         # model limit
  # Governance bindings
  consent_receipt_id: string | null      # ConsentReceipt for cloud sessions (Â§4.3.9.14)
  projection_plan_id: string | null
  capability_grants: string[]            # effective capabilities for this session (resolved via CapabilityRegistry Â§11.1)
  capability_token_ids: string[] | null  # references approval tokens/receipts that grant the above (deny-by-default; Â§11.1)
  # Crash recovery
  checkpoint_artifact_id: string | null  # last checkpoint (Â§4.3.9.19)
  last_checkpoint_at: string | null
```

##### 4.3.9.12.3 Message Thread Schema (Normative)

Each session owns a durable message thread:

```yaml
# ADD v02.137
SessionMessage:
  message_id: string                     # stable UUID
  session_id: string                     # FK â†’ ModelSession
  role: enum [SYSTEM, USER, ASSISTANT, TOOL_CALL, TOOL_RESULT]
  content_hash: string                   # SHA-256 of content (content stored as artifact, not inline in event log)
  content_artifact_id: string            # reference to stored content
  timestamp: string
  token_count: int | null
  redacted: bool                         # true if content was redacted for export/logging
  tool_call_id: string | null            # for TOOL_CALL / TOOL_RESULT correlation
  attachments: string[]                  # artifact references
```

##### 4.3.9.12.4 Invariants

- **INV-SESS-001:** Every `ModelSession` MUST have a stable `session_id` that is referenced in UI, Flight Recorder events, governance receipts, and artifact provenance.
- **INV-SESS-002:** Session message content MUST be stored as artifacts (with SHA-256 hashes), never inline in Flight Recorder events or governance artifacts. Flight Recorder events reference `content_hash` / `content_artifact_id` only (precedent: FR-EVT-006 `prompt_hash`/`response_hash`).
- **INV-SESS-003:** A session in state `ACTIVE` MUST have exactly one executing model call at a time (sequential within a session; parallelism is across sessions, not within one).
- **INV-SESS-004:** Session `memory_policy` MUST be declared at creation and MUST NOT change mid-session (create a new session for different policy).
- **INV-SESS-005 (cross-session isolation):** One session MUST NOT read another session's message thread without explicit operator approval and a Flight Recorder event recording the cross-session access. This prevents the "implicit trust boundary" anti-pattern identified in nanobot's bus-injected messages.

##### 4.3.9.12.5 Storage

- Phase 1: SQLite table `model_sessions` + `session_messages` in workspace database.
- Message content artifacts stored in `.handshake/artifacts/` per existing artifact discipline.
- Session state changes emit Flight Recorder events (Â§4.3.9.18).

##### 4.3.9.12.6 ModelSession.state â†” JobState Mapping (Normative) [ADD v02.137]

`ModelSession.state` is a **session-level aggregate** that summarizes the lifecycle of the session across one or more `model_run` jobs. `JobState` (Â§2.6.6.3) remains the authoritative state machine for the underlying jobs; the Session Scheduler derives `ModelSession.state` deterministically.

Recommended mapping:

| ModelSession.state | Typical scheduler condition | Active `model_run` JobState (if any) |
|---|---|---|
| CREATED | Session record exists; no dispatch yet | *(none)* or `Queued` if a first job is already enqueued |
| ACTIVE | A `model_run` is executing (incl. streaming) | `Running` |
| PAUSED | Operator or policy paused the session; no executing job | *(none)* |
| BLOCKED | Waiting on approval / consent / external dependency | `AwaitingUser` or `Stalled` |
| COMPLETED | Terminal success; no more work scheduled | `Completed` or `CompletedWithIssues` |
| FAILED | Terminal failure (non-retryable) | `Failed` or `Poisoned` |
| CANCELLED | Terminal cancellation (operator/policy) | `Cancelled` |

**Derivation rules (MUST)**
- A session MUST be `ACTIVE` iff it has exactly one executing `model_run` (INV-SESS-003).
- If the current job enters `AwaitingUser` (e.g., Tool Gate approval required), the session MUST enter `BLOCKED` until the gate resolves.
- If the session is paused, pending queued jobs MUST remain enqueued but MUST NOT dispatch until the session resumes.
- Terminal session states (`COMPLETED`, `FAILED`, `CANCELLED`) MUST be sticky.

---


##### 4.3.9.12.7 Front End Memory System integration (FEMS) (Normative) [ADD v02.138]

`ModelSession` integrates with the Front End Memory System (FEMS) to provide **bounded continuity** across turns and across sessions without introducing prompt drift.

**Read semantics (per-call)**
- `memory_policy = EPHEMERAL`
  - No `MemoryPack` is injected.
  - No memory write proposals may be generated.
- `memory_policy = SESSION_SCOPED`
  - Inject only **session-scoped working memory** (e.g., open loops + current assumptions) derived from this sessionâ€™s `SessionLog`.
  - Do not retrieve or inject workspace/project memory.
- `memory_policy = WORKSPACE_SCOPED`
  - Inject a bounded `MemoryPack` compiled from `LongTermMemory` + project/WP scope (see Â§2.6.6.7.6.2).
  - Workspace-scoped injection MUST still respect sensitivity classification and cloud consent gates.

**Placement (anti-poisoning)**
- `MemoryPack` SHOULD appear in the `PromptEnvelope.variable_suffix` (never in stable prefix by default).
- `MemoryPack` MAY appear in the stable prefix only if every included item is:
  - `memory_class = procedural`, AND
  - `trust_level = local_authoritative`, AND
  - review-approved (DCC / Approval Inbox).

**Write semantics (governed)**
- Sessions MAY produce `MemoryWriteProposal` artifacts via `memory_extract_v0.1` (see Â§2.6.6.6.6).  
- Commits to `LongTermMemory` MUST be performed by an explicit commit job (validator + policy gated) and are never implicit.

**Cloud boundary**
- When `provider = cloud`, FEMS MUST build a **cloud-safe** `MemoryPack` variant:
  - exclude `classification=high` unless an explicit consent receipt allows it,
  - redact or omit contact/CRM items by default,
  - record the decision in the `ContextSnapshot` / `CloudConsentGate` receipt.

**State pointer**
- `memory_state_ref` SHOULD reference the most recent committed `MemoryPack` artifact id/hash for this session, enabling:
  - deterministic replay (same pack hash under `replay`),
  - UI preview (DCC â€œWhat did the model see?â€),
  - drift debugging (pack invalidation when memory changes).

#### 4.3.9.13 Session Scheduler: Model Calls as Queued Work (Normative) [ADD v02.137]

**Addresses:** GAP B (no orchestrator-level scheduler that treats model calls as queued work).  
**Informed by:** OpenClaw per-session lane queues + global lane, TinyClaw SQLite-backed queue serialization, nullclaw autonomy levels.

##### 4.3.9.13.1 Purpose

Parallelism requires controlled concurrency, not ad-hoc spawning. The Session Scheduler is the subsystem that treats model calls as jobs with explicit concurrency limits, rate limiting, and priority.

##### 4.3.9.13.2 Job Kind: `model_run` (Normative)

The Session Scheduler introduces `job_kind = "model_run"` into the AI Job Model (Â§2.6.6):

```yaml
# ADD v02.137
ModelRunJob:
  job_kind: "model_run"
  session_id: string                     # FK â†’ ModelSession
  model_id: ModelId
  backend: ModelBackend
  lane: enum [PRIMARY, SUBAGENT, BACKGROUND, VALIDATION]
  priority: int                          # 0 = highest; default 50
  # Concurrency and rate controls
  concurrency_group: string | null       # shared limit key (e.g., provider name)
  max_retries: int                       # default 3
  retry_backoff: enum [FIXED, EXPONENTIAL]  # default EXPONENTIAL
  timeout_ms: int                        # per-call timeout; default 120000
  cancellation_token: string | null      # correlates to operator cancel action
  # Budget
  max_tokens_budget: int | null          # hard cap; job fails if exceeded
  estimated_cost_usd: number | null      # pre-flight estimate for operator approval
```

##### 4.3.9.13.3 Concurrency Limits (Normative)

```yaml
# ADD v02.137
SessionSchedulerConfig:
  max_concurrent_sessions_global: int         # default 8
  max_concurrent_sessions_per_provider: int   # default 4
  max_concurrent_sessions_per_model: int      # default 2
  rate_limit_requests_per_minute: int | null  # per-provider; null = unlimited
  rate_limit_tokens_per_minute: int | null    # per-provider; null = unlimited
```

##### 4.3.9.13.4 Scheduling Invariants

- **INV-SCHED-001:** All model invocations in `RuntimeMode=AI_ENABLED` MUST be routed through the Session Scheduler. No direct `LlmClient.complete()` calls outside the scheduler in production paths.
- **INV-SCHED-002:** When a concurrency limit is reached, new `model_run` jobs MUST be enqueued (not dropped) and MUST show `QUEUED` state in UI and Flight Recorder.
- **INV-SCHED-003:** Rate limiting MUST use token-bucket or sliding-window with deterministic backoff. Backoff parameters MUST be logged.
- **INV-SCHED-004:** Cancellation MUST be cooperative: the scheduler sets a cancellation flag; the active model call MUST check the flag at tool-call boundaries and between streaming chunks. Cancelled jobs transition to `CANCELLED` (not `FAILED`).
- **INV-SCHED-005 (lane isolation â€” from OpenClaw):** Subagent and background lanes MUST NOT starve the primary lane. Implementation SHOULD use weighted fair queuing or dedicated lane quotas.

##### 4.3.9.13.5 Flight Recorder Events

```yaml
# ADD v02.137
FR-EVT-SESS-SCHED-001:
  event_type: "session_scheduler.enqueue"
  payload: { session_id, job_id, lane, priority, concurrency_group }

FR-EVT-SESS-SCHED-002:
  event_type: "session_scheduler.dispatch"
  payload: { session_id, job_id, lane, queue_wait_ms }

FR-EVT-SESS-SCHED-003:
  event_type: "session_scheduler.rate_limited"
  payload: { session_id, job_id, provider, backoff_ms, attempt }

FR-EVT-SESS-SCHED-004:
  event_type: "session_scheduler.cancelled"
  payload: { session_id, job_id, reason, cancelled_by }
```

---

#### 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions (Normative) [ADD v02.137]

**Addresses:** GAP C (governance UX for cloud calls is not yet a "session gate" system).  
**Informed by:** Handshake existing `guard.rs` (ProjectionPlan + ConsentReceipt), OpenClaw safe-bin policies + operator gating.

##### 4.3.9.14.1 Purpose

An orchestrator that spins up multiple cloud runs without durable consent + audit is unsafe. This section defines the consent lifecycle for parallel cloud sessions.

##### 4.3.9.14.2 Consent Flow (Normative)

1. **Pre-flight projection:** Before any cloud model call, the system MUST generate a `ProjectionPlan` that discloses:
   - exact prompt content (or content hash + size for large payloads),
   - model provider + endpoint,
   - attachments and their types/sizes,
   - estimated token cost.

2. **Fan-out disclosure:** If the orchestrator will send the same (or derived) task to multiple cloud sessions (broadcast/fan-out), the ProjectionPlan MUST explicitly state:
   - number of target sessions,
   - provider(s) and model(s) per session,
   - total estimated cost (sum across sessions).

3. **Consent receipt issuance:** The operator (or an automation policy with pre-approved scope) MUST issue a `ConsentReceipt` bound to:
   - the ProjectionPlan hash,
   - the session_id(s) it covers,
   - a validity window (default: single-use; policy may allow session-scoped or WP-scoped receipts).

4. **Receipt binding:** The `ModelSession` MUST store `consent_receipt_id` and the Session Scheduler MUST verify receipt validity before dispatching a cloud `model_run` job.

5. **Receipt audit:** All consent receipts MUST be persisted as governance artifacts and linked in Flight Recorder.

##### 4.3.9.14.3 Policy Scopes (Normative)

```yaml
# ADD v02.137
ConsentScope: enum
  - SINGLE_CALL        # one receipt per model call (strictest)
  - SESSION_SCOPED     # one receipt covers all calls in one session
  - WP_SCOPED          # one receipt covers all calls in one Work Packet
  - BROADCAST_SCOPED   # one receipt covers a fan-out to N sessions (must disclose N)
```

Default: `SINGLE_CALL`. Operator MAY widen scope via Work Profile or project policy; the widened scope MUST be logged.

##### 4.3.9.14.4 Invariants

- **INV-CONSENT-001:** No cloud model call MAY execute without a valid, non-expired `ConsentReceipt` bound to the target session. Violation = hard block with `CX-MM-CONSENT-MISSING`.
- **INV-CONSENT-002:** A `BROADCAST_SCOPED` receipt MUST enumerate target session_ids at issuance. Adding sessions after issuance requires a new receipt.
- **INV-CONSENT-003:** Revoking a receipt MUST cancel all pending `model_run` jobs covered by it and transition affected sessions to `BLOCKED`.

---

#### 4.3.9.15 Session Spawn Contract and Lifecycle (Normative) [ADD v02.137]

**Addresses:** GAP B (controlled spawning), anti-pattern guards.  
**Informed by:** OpenClaw `sessions_spawn` â†’ `spawnSubagentDirect` (non-blocking, announce-back), depth limits + active-children caps, TinyClaw named-agent queue delegation.

##### 4.3.9.15.1 Purpose

An orchestrator role (or an executing session) may need to spawn child sessions for delegation. This section defines the spawn contract to prevent runaway delegation storms and ensure auditability.

##### 4.3.9.15.2 Spawn Request Schema (Normative)

```yaml
# ADD v02.137
SessionSpawnRequest:
  requester_session_id: string
  requester_role: string
  child_role: string                     # role for spawned session
  child_model_preference: ModelId | null # null = use Work Profile default
  child_backend_preference: ModelBackend | null
  task_payload_artifact_id: string       # work packet / micro-task / instruction artifact
  spawn_mode: enum [ONE_SHOT, SESSION_PERSISTENT]
  announce_back: bool                    # default true; child announces completion back to requester
  trigger_source:
    stage_session_id: string | null       # provenance link to Stage Session (Â§10.13)
    stage_tab_id: string | null           # provenance link to Stage Tab (Â§10.13)
    external_ref: string | null           # optional external correlation key
  # Limits (inherited or overridden)
  max_child_depth: int | null            # null = inherit from parent or global default
  max_active_children: int | null        # null = inherit from parent or global default
```

##### 4.3.9.15.3 Spawn Response (Normative)

Spawn MUST return immediately (non-blocking):

```yaml
# ADD v02.137
SessionSpawnResponse:
  accepted: bool
  child_session_id: string | null        # null if rejected
  rejection_reason: string | null        # e.g., "depth_limit_exceeded", "children_cap_reached"
```

The child session runs asynchronously. When complete, it announces back to the requester via Role Mailbox (Â§2.6.8.10) using `message_type=SessionAnnounceBack` (MailboxKind=`COLLAB`) with a normalized summary artifact.

##### 4.3.9.15.4 Depth and Children Caps (HARD)

```yaml
# ADD v02.137
SpawnLimits:
  max_spawn_depth: int                   # default 3; HARD ceiling configurable per workspace
  max_active_children_per_session: int   # default 4
  max_total_active_sessions: int         # from SessionSchedulerConfig.max_concurrent_sessions_global
```

- **INV-SPAWN-001:** Spawn depth MUST be enforced before creating the child session. Exceeding `max_spawn_depth` = immediate rejection with reason `"depth_limit_exceeded"`.
- **INV-SPAWN-002:** Active children count MUST be checked against `max_active_children_per_session`. Exceeding = immediate rejection with reason `"children_cap_reached"`.
- **INV-SPAWN-003:** Spawned sessions MUST inherit (or further restrict) the parent's session-scoped capability tokens/grants (Â§11.1). A child session MUST NOT have capabilities wider than its parent.
- **INV-SPAWN-004 (cascade kill):** Killing/cancelling a parent session MUST recursively cancel all descendant sessions. This prevents orphaned sessions consuming resources.

##### 4.3.9.15.5 Announce-Back Contract (Normative)

When a child session completes (`COMPLETED` or `FAILED`):

1. It MUST produce a summary artifact (structured: status, key outputs, error if any).
2. It MUST send an announce-back message to the requester's Role Mailbox thread (MailboxKind=`COLLAB`) as `message_type=SessionAnnounceBack`, referencing the summary artifact (no inline content; artifact refs + hashes only).
3. If `spawn_mode = ONE_SHOT`, the child session's message thread MAY be archived/cleaned up per retention policy after announce-back.
4. If `spawn_mode = SESSION_PERSISTENT`, the child session remains addressable for follow-up work.

##### 4.3.9.15.6 Flight Recorder Events

```yaml
# ADD v02.137
FR-EVT-SESS-SPAWN-001:
  event_type: "session.spawn_requested"
  payload: { requester_session_id, child_role, spawn_depth, spawn_mode }

FR-EVT-SESS-SPAWN-002:
  event_type: "session.spawn_accepted"
  payload: { requester_session_id, child_session_id, child_role, spawn_depth }

FR-EVT-SESS-SPAWN-003:
  event_type: "session.spawn_rejected"
  payload: { requester_session_id, rejection_reason, spawn_depth, active_children_count }

FR-EVT-SESS-SPAWN-004:
  event_type: "session.announce_back"
  payload: { child_session_id, requester_session_id, status, summary_artifact_id, mailbox_message_id }

FR-EVT-SESS-SPAWN-005:
  event_type: "session.cascade_cancel"
  payload: { root_session_id, cancelled_session_ids[], reason }
```

---

#### 4.3.9.16 Provider Feature Coverage: Agentic Orchestration Ready (Normative) [ADD v02.137]

**Addresses:** GAP F (provider feature coverage is not yet agentic-ready).  
**Informed by:** OpenClaw Pi embedded runner (streaming, tool loop, multi-turn), provider quirks hardening (Anthropic refusal scrubbing, Ollama native stream).

##### 4.3.9.16.1 Purpose

Many parallel work patterns rely on long-running streams, tool calls, and structured plans. The existing `LlmClient` trait (Â§4.2.3) is completion-based. This section extends the provider contract for orchestration readiness.

##### 4.3.9.16.2 Extended Capability Flags (Normative)

The `LlmClient` capability detection (Â§4.2.3.2) MUST be extended with:

```yaml
# ADD v02.137
ProviderCapabilities:
  supports_streaming: bool               # incremental token delivery
  supports_tool_calling: bool            # function/tool calling schema
  supports_structured_output: bool       # JSON schema constraint on output
  supports_multi_turn: bool              # multi-message chat (not just single completion)
  supports_vision: bool                  # image input
  supports_thinking: bool                # extended thinking / chain-of-thought blocks
  max_context_window: int                # tokens
  max_output_tokens: int                 # tokens
  # Provider quirks (informative; used for runtime hardening)
  requires_tool_id_sanitization: bool    # e.g., strict providers reject certain ID formats
  requires_thinking_block_scrubbing: bool # e.g., providers that reject persisted thinking
  native_stream_endpoint: string | null  # e.g., Ollama "/api/chat" for reliability
```

##### 4.3.9.16.3 Multi-Turn Chat Adapter (Normative)

The system MUST support a multi-turn chat interface in addition to single-prompt completion:

```yaml
# ADD v02.137
ChatRequest:
  session_id: string
  messages: SessionMessage[]             # full or windowed history
  tools: ToolDefinition[] | null         # Tool Registry entries (Unified Tool Surface Contract Â§6.0.2)
  structured_output_schema: JsonSchema | null
  stream: bool                           # default true
  trace_id: string

ChatResponse:
  message: SessionMessage                # assistant response
  tool_calls: ToolCall[] | null          # if the model requests tool use
  usage: TokenUsage
  finish_reason: enum [STOP, TOOL_USE, LENGTH, CONTENT_FILTER, ERROR]
```

##### 4.3.9.16.4 Tool Calling Schema (Normative)

Tool definitions MUST be provider-agnostic and translated to provider-specific schemas at the adapter layer:

**Unification rule (HARD):** `ToolDefinition` here is a projection of the Unified Tool Surface Contract Tool Registry entry (Â§6.0.2). Implementations MUST NOT maintain a parallel tool schema; the Tool Registry is the SSoT. Provider adapters MAY serialize a provider-specific subset, but it MUST be generated from the Tool Registry.

**Capability rule (HARD):** Tool Gate MUST evaluate `required_capabilities` against the session-scoped effective capabilities for the originating `ModelSession` (intersection), not just global capability state.

```yaml
# ADD v02.137
ToolDefinition:
  tool_id: string                        # stable identifier
  name: string                           # display name
  description: string
  parameters: JsonSchema                 # input schema
  # Governance integration
  required_capabilities: string[]        # capability grants needed to invoke
  risk_level: enum [LOW, MEDIUM, HIGH]   # informs approval flow
```

- **INV-PROV-001:** Tool call results MUST be routed back into the session's message thread as `TOOL_RESULT` messages with correlation to the originating `TOOL_CALL`.
- **INV-PROV-002:** If a provider does not support tool calling, the system MUST either (a) emulate via prompt-based extraction with deterministic parsing, or (b) fail the tool call with an explicit error. Silent degradation is not allowed.
- **INV-PROV-003 (from OpenClaw quirks hardening):** Provider-specific sanitization (tool ID formats, thinking block scrubbing, refusal string detection) MUST be implemented in the adapter layer, not in session/orchestration code.

---

#### 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions (Normative) [ADD v02.137]

**Addresses:** GAP G (workspace/repo safety for parallel workers).  
**Informed by:** PicoClaw `restrict_to_workspace` + command denylist, OpenClaw workspace-root guards on FS tools, INV-MM-003 (existing non-overlap rule).

##### 4.3.9.17.1 Purpose

Parallel sessions touching the same repo need guardrails to prevent self-conflicts and accidental overwrites. This section extends INV-MM-003 with concrete isolation strategies.

##### 4.3.9.17.2 Session Isolation Strategies (Normative)

The system MUST support at least one of:

1. **Worktree isolation (preferred):** Each parallel session that writes to the repo operates in a dedicated git worktree or branch. Merge-back is an explicit, operator-approved step.
2. **File-scope lock isolation (fallback):** When worktree isolation is impractical, strict file-scope locks (Â§4.3.9.2.4) MUST prevent overlapping writes. This is the existing INV-MM-003 contract.

**Non-code worksurfaces (Design Studio, Phase 2+):** For CRDT-backed entities (canvas/docs/tables), git worktrees are irrelevant. The equivalent isolation strategy is **entity-level locking** (node/section locks) and/or CRDT branch/merge discipline. This complement is deferred to Design Studio Phase 2; the same deny-by-default cross-session write isolation invariant applies.

##### 4.3.9.17.3 Command Denylist (Normative)

Spawned sessions (child sessions via Â§4.3.9.15) and background sessions MUST NOT be permitted to execute destructive workspace commands unless explicitly approved per-invocation by the operator. Denied commands include (at minimum):

- `git reset --hard`, `git clean -fd`, `git rebase` (interactive)
- `rm -rf` on paths outside the session's scoped workspace
- any command that modifies `.handshake/gov/` artifacts without going through governance gates

This mirrors PicoClaw's `restrict_to_workspace` + command denylist pattern and is enforced by the Tool Gate (Â§6.0.2).

##### 4.3.9.17.4 Merge-Back Discipline (Normative)

When worktree isolation is used:

1. Session completes work in its worktree.
2. Session produces a merge-ready artifact (diff/patch) with provenance.
3. Orchestrator or operator reviews the diff (via DCC Â§10.11).
4. Merge-back is executed as an explicit governance action with Flight Recorder logging.
5. Merge conflicts MUST surface as `BLOCKED` state with explicit conflict report, not silent resolution.

##### 4.3.9.17.5 Invariants

- **INV-WS-001:** A session MUST NOT write to files outside its declared `IN_SCOPE_PATHS` (from Work Unit lock contract Â§4.3.9.2.4).
- **INV-WS-002 (fail-closed exec â€” from OpenClaw):** If a session requests execution (shell command, script) and no sandbox or workspace restriction is in place, the system MUST deny the execution rather than silently running on the host. This is the "fail-closed default" pattern from OpenClaw's tool construction.
- **INV-WS-003:** Cross-session file access (one session reading another session's uncommitted worktree changes) MUST be denied by default. Explicit operator approval + FR event required.

---

#### 4.3.9.18 Session Observability: ActivitySpan and ModelSessionSpan Binding (Normative) [ADD v02.137]

**Addresses:** GAP E (incomplete observability for multi-session execution).  
**Cross-references:** Â§11.9.1 (ActivitySpan/SessionSpan), Â§11.5 (Flight Recorder).

##### 4.3.9.18.1 Purpose

Parallelism without strong observability becomes un-debuggable and unsafe. This section binds ModelSessions to ActivitySpan primitives and introduces a distinct `ModelSessionSpan` (to avoid collision with the operator `SessionSpan`).

##### 4.3.9.18.2 Span Binding (Normative)

- Every `ModelSession` MUST create a `ModelSessionSpan` at session creation and close it at session completion/cancellation.
- Every `model_run` job within a session MUST create an `ActivitySpan` nested under the session's `ModelSessionSpan`.
- Tool calls within a model run MUST create child `ActivitySpan`s under the model run span.

```yaml
# ADD v02.137
ModelSessionSpanBinding:
  session_id: string
  model_session_span_id: string
  parent_model_session_span_id: string | null  # null for root sessions; parent session's span for children

ActivitySpanBinding:
  activity_span_id: string
  model_session_span_id: string                # FK â†’ ModelSessionSpanBinding
  job_id: string                         # FK â†’ model_run job
  model_id: ModelId
  start_time: string
  end_time: string | null
  token_count: int | null
  cost_usd: number | null
```

**Correlation rule (HARD):** Every Flight Recorder event emitted in the context of a `ModelSession` MUST set `FlightRecorderEventBase.model_session_id = ModelSession.session_id` (see Â§11.5.1). This is the primary correlation key for session-wide queries; span IDs are supplemental.

##### 4.3.9.18.3 Aggregated Cost/Budget Views (Normative)

The system MUST provide (in UI and API):

- **Per-session cost:** sum of token costs across all model_run jobs in a session.
- **Per-WP cost:** sum across all sessions bound to a Work Packet.
- **Global concurrent cost rate:** tokens/minute and estimated $/minute across all active sessions.
- **Budget alerts:** when a session or WP approaches a configured budget threshold, emit a warning event and optionally pause.

##### 4.3.9.18.4 Flight Recorder Events (Session Lifecycle)

```yaml
# ADD v02.137
FR-EVT-SESS-001:
  event_type: "session.created"
  payload: { session_id, model_id, backend, role, wp_id, mt_id, memory_policy, spawn_depth }

FR-EVT-SESS-002:
  event_type: "session.state_change"
  payload: { session_id, from_state, to_state, reason }

FR-EVT-SESS-003:
  event_type: "session.completed"
  payload: { session_id, total_tokens, total_cost_usd, duration_ms, messages_count }

FR-EVT-SESS-004:
  event_type: "session.message"
  payload: { session_id, message_id, role, content_hash, token_count }
  # NOTE: content is stored as artifact; event carries hash only (INV-SESS-002)

FR-EVT-SESS-005:
  event_type: "session.budget_warning"
  payload: { session_id, budget_type, current_value, threshold_value }
```

##### 4.3.9.18.5 Artifact References for Prompts/Responses (Normative)

- Prompt and response content MUST be stored as artifacts (with SHA-256 hashes) so operators can audit/debug without content leaking into Flight Recorder events.
- Flight Recorder events reference `content_artifact_id` and `content_hash` only.
- Export/debug bundles MAY include content if the operator's redaction policy allows it.

---

#### 4.3.9.19 Session Crash Recovery and Checkpointing (Normative) [ADD v02.137]

**Addresses:** GAP H (failure handling + crash recovery for long-lived sessions).  
**Informed by:** OpenClaw session file locking + lane queues, Handshake existing RunLedger (Â§2.6.6.8).

##### 4.3.9.19.1 Purpose

Parallel orchestration increases failure modes. Without resume, it becomes fragile.

##### 4.3.9.19.2 Checkpoint Contract (Normative)

```yaml
# ADD v02.137
SessionCheckpoint:
  checkpoint_id: string
  session_id: string
  timestamp: string
  # State snapshot
  session_state: ModelSession             # full session state at checkpoint
  message_thread_tail_id: string          # last message_id at checkpoint
  pending_tool_calls: ToolCall[]          # in-flight tool calls
  # Artifact
  checkpoint_artifact_id: string          # stored as a governance artifact
```

##### 4.3.9.19.3 Checkpoint Policy (Normative)

- Sessions MUST be checkpointed:
  - after every successful tool call completion,
  - after every operator approval/steering action,
  - before any state transition to `PAUSED` or `BLOCKED`,
  - on graceful shutdown.
- Checkpoint frequency for streaming completion: at minimum every N messages (configurable; default 5) or every M tokens (configurable; default 4096).

##### 4.3.9.19.4 Resume Semantics (Normative)

After app crash/restart:

1. Session Scheduler scans for sessions in `ACTIVE` state with no live process.
2. For each orphaned session:
   - Load last checkpoint.
   - Transition to `PAUSED` with reason `"crash_recovery"`.
   - Notify operator via DCC (Â§10.11) with session details and last-known state.
   - Operator MAY resume (from checkpoint) or cancel.
3. Resume replays pending tool calls (idempotent where possible) and continues from the last message.

##### 4.3.9.19.5 Invariants

- **INV-RECOVER-001:** Session state MUST be recoverable from checkpoints alone, without relying on in-memory state.
- **INV-RECOVER-002:** Pending tool calls at crash time MUST be re-evaluated (not blindly re-executed) since side effects may have partially completed.
- **INV-RECOVER-003:** All recovery actions MUST be logged in Flight Recorder with `FR-EVT-WF-RECOVERY` correlation.

[ADD v02.165] Recovery-safe run history MUST preserve queue-state transitions, workflow-node execution lineage, tool-call lineage, checkpoint chronology, and operator replay decisions by stable `workflow_run_id`, `workflow_node_execution_id`, `session_id`, `tool_call_id`, and `checkpoint_id` values. Dev Command Center replay or history views MUST NOT reconstruct chronology only from message order or the latest visible transcript.

---

#### 4.3.9.20 Inbound Trust Boundary Rules (Normative) [ADD v02.137]

**Informed by:** OpenClaw security hotspots (dispatcher trust boundary), nanobot bus-injected system messages, anti-pattern registry from all forks.

##### 4.3.9.20.1 Purpose

When sessions can receive messages from external sources (MCP, plugins, other sessions), the inbound trust boundary becomes critical. This section defines rules to prevent prompt injection and privilege escalation across sessions.

##### 4.3.9.20.2 Rules (HARD)

- **TRUST-001 (system message provenance):** Only the Handshake runtime (Workflow Engine, Session Scheduler, governance gates) MAY inject `SYSTEM` role messages into a session's thread. External sources (MCP, plugins, other sessions) MUST be injected as `USER` role with explicit source attribution.
- **TRUST-002 (cross-session message routing):** If a session receives a message routed from another session (via Role Mailbox or announce-back), the message MUST carry:
  - source session_id,
  - source role,
  - content_hash,
  - a flag indicating whether the source is trusted (internal) or untrusted (external/plugin).
- **TRUST-003 (tool surface narrowing for children):** Spawned child sessions MUST have equal or narrower tool permissions than their parent. A child MUST NOT be able to escalate its own tool access. This mirrors OpenClaw's subagent policy narrowing.
- **TRUST-004 (no dangerous-bypass flags):** Handshake MUST NOT provide CLI flags, environment variables, or configuration switches that disable sandbox, approval gates, or capability checks globally. Any test/debug bypass MUST be scoped to a single session and logged. This directly addresses the TinyClaw `--dangerously-skip-permissions` anti-pattern.

---

#### 4.3.9.21 Anti-Pattern Registry (Informative) [ADD v02.137]

**Informed by:** OpenClaw forks research paper â€” "Patterns that predict incidents" and threat-model checklist.

The following patterns are explicitly documented as anti-patterns that Handshake's design MUST prevent:

| ID | Anti-Pattern | Source | Handshake Prevention |
|---|---|---|---|
| AP-001 | "Dangerously bypass approvals/sandbox" flags | TinyClaw | TRUST-004: no global bypass flags; session-scoped debug only |
| AP-002 | Untrusted inbox + wide tool surface = remote action pipeline | OpenClaw hotspot analysis | TRUST-001/002: provenance-tagged inbound; tool surface narrowing per session |
| AP-003 | Implicit system message privilege without provenance | nanobot bus injection | TRUST-001: only runtime injects SYSTEM; external = USER with attribution |
| AP-004 | No audit trail / no action trace | Multiple forks | INV-SESS-001/002 + FR events: every session action has Flight Recorder evidence |
| AP-005 | Auto-installing plugins/skills from registries without pinning | OpenClaw supply chain risk | Handshake requires pinned versions + hashes; capability-gated installation (Â§5.1) |
| AP-006 | Runaway delegation storms (recursive spawn) | OpenClaw subagent amplification | INV-SPAWN-001/002: hard depth + children caps; cascade kill |
| AP-007 | Silent fallback from sandbox to host execution | OpenClaw exec host selection | INV-WS-002: fail-closed; deny if no sandbox rather than run on host |
| AP-008 | Session state loss on crash without recovery path | General | Â§4.3.9.19: mandatory checkpointing + structured resume |

---

### 4.4 Storage and Persistence

[CX-340] STORAGE_LAYERED: DB/filesystem access SHOULD be centralised in storage modules under `{{BACKEND_STORAGE_DIR}}/`.
[CX-341] STORAGE_INDIRECT: Other modules SHOULD go through storage interfaces/services instead of raw DB drivers.
[CX-342] STORAGE_DOCS: New core tables/collections SHOULD get a short note in `/.GOV/operator/docs_local/` when they affect core concepts.

[CX-343] DEBUG_ANCHORS: New errors SHOULD emit stable, searchable anchors (e.g., error codes like `{{ISSUE_PREFIX}}-####` or consistent log tags). `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` SHOULD reference those anchors and the primary entrypoints for triage.

---

## 5. Spec Usage Protocol

[CX-400] SPEC_PRIMARY: When Master Spec or subsystem specs are provided, they are the primary reference for product and architecture.
[CX-401] SPEC_OVERRULE_PRIORS: Provided specs SHOULD override model priors and generic "best practices" if they conflict.

[CX-402] SPEC_CURRENT_POINTER: If multiple versions of the Master Spec exist in the repo, assistants MUST treat `.GOV/spec/SPEC_CURRENT.md` as the canonical pointer to the current Master Spec for the active workline/session.

[CX-405] SPEC_PROPOSAL_GATE: Before applying any changes to the Master Spec (LAW_2) or Codex (LAW_1), the assistant MUST present a "Spec Proposal" summary to the user.

[CX-406] SPEC_CO_AUTHOR_REVIEW: The Spec Proposal must summarize *what* is changing, *why*, and explicit *architectural impacts*. The assistant MUST pause and await user confirmation or tweaks before committing the change to the file.

[CX-407] SPEC_VERSIONING: Any modification to the Master Spec (LAW_2) MUST trigger a version increment (e.g., v02.xx -> v02.xy). The assistant MUST rename the file to reflect the new version and update the version metadata in the file header.

[CX-410] SPEC_FIND: For non-trivial tasks, the assistant SHOULD identify which provided sections govern the feature/subsystem.
[CX-411] SPEC_SOURCE_BLOCK: The assistant SHOULD quote or summarise relevant spec fragments in a small SOURCE block in its answer.
[CX-412] SPEC_ALIGN: The assistant SHOULD explain how its proposal aligns with those fragments.
[CX-413] SPEC_SILENCE: When specs are clearly silent or incomplete, the assistant SHOULD say so directly.

[CX-420] SPEC_ASSUMPTIONS: When specs are silent, the assistant MAY introduce minimal assumptions.
[CX-421] SPEC_ASSUMPTIONS_TAG: Such assumptions SHOULD be tagged as ASSUMPTION / PROVISIONAL DECISION.
[CX-422] SPEC_ASSUMPTIONS_LOCAL: Assumptions SHOULD be kept local to the current change and not treated as spec updates.

[CX-430] NO_REDEFINE_ARCH: If no spec slice is provided for a domain, the assistant MUST NOT redefine global architecture and MUST prefer local, easily reversible decisions.

---

## 6. Assistant Behaviour (General)

### 6.1 Role and Scope

[CX-500] ROLE_PAIR_DEV: The assistant acts as a pair developer and spec enforcer for this repo.
[CX-501] ROLE_OBEY_HARD: The assistant MUST obey the hard invariants in Â§2 unless the user explicitly suspends them for exploration.
[CX-502] ROLE_OBEY_GUIDE: The assistant SHOULD follow the layout and behavioural guidance in this codex when reasonable.
[CX-503] ROLE_AI_AUTONOMY: AI assistants are expected to operate autonomously within codex constraints. The human user may not have coding expertise and relies on deterministic workflow enforcement to ensure correctness.

[CX-504] USER_EXPERTISE: The human user of this session is NOT a coder or software engineer. All communication from AI agents (Orchestrator, Coder, etc.) MUST be presented in clear, non-technical language, explaining every step and providing analogies suitable for a non-expert audience, unless explicitly instructed otherwise by the user. Every Task Packet MUST include a "User Context" non-technical explainer.

[CX-505] WORKFLOW_BRANCHING: The STANDARD workflow is Feature Branching.
- Agents SHOULD create and work in `feat/WP-{ID}`.
- Direct editing of `main` is discouraged for non-trivial work (requires Waiver).
- **Validator Authority:** Upon issuing a PASS verdict, the Validator Agent is responsible for performing the final git commit or merge to `main`. Coders MUST NOT merge their own work.

[CX-654] USER_CONTEXT_INVARIANT (HARD): In any Work Packet (Task Packet), the "User Context" or "Non-Technical Explainer" section MUST NEVER be rewritten or deleted. It can only be APPENDED to. This ensures the user's original intent and oversight are preserved for the duration of the task.

### 6.2 Task Intake and Clarification

[CX-510] TASK_RESTATE: For non-trivial tasks, the assistant SHOULD restate the task in its own words.
[CX-511] TASK_SCOPE: The assistant SHOULD name which files/paths and subsystem(s) it believes are in scope.
[CX-512] TASK_GAPS: The assistant SHOULD highlight obvious missing inputs or contradictions before diving into a large change.
[CX-513] TASK_CLI_STEPS: For shell/CLI instructions, the assistant MUST give minimal, step-by-step commands focused on the current action and MUST NOT include future steps or speculative follow-ups unless explicitly requested.

### 6.3 Artefacts and Patch Semantics

[CX-520] ARTEFACT_PRIMARY: When concrete artefacts (files, folders, spec slices) are provided, they SHOULD be treated as primary ground truth.
[CX-521] ARTEFACT_NO_GUESS: The assistant SHOULD avoid assuming structure or content for artefacts it has not seen.

[CX-530] PATCH_PREF: The assistant SHOULD express changes as PATCHES (path + BEFORE/AFTER for changed regions) for any non-trivial modification.
[CX-531] PATCH_SINGLE_PURPOSE: Each PATCH SHOULD have a clear purpose and avoid mixing unrelated clean-ups with main changes.
[CX-532] PATCH_FULL_FILE_ALLOWED: When the user explicitly asks to "rewrite this file" or provides whole-file context, the assistant MAY return a full-file rewrite instead of fine-grained patches, but SHOULD still avoid unrelated changes.
[CX-533] PATCH_UNCERTAIN: If file state is clearly partial or uncertain, the assistant SHOULD either request more context or narrow the change, rather than hallucinate content.

### 6.4 Assumptions, Risks, and Alternatives

[CX-540] ASSUME_MINIMAL: The assistant SHOULD minimise assumptions and base decisions on artefacts/specs first.
[CX-541] RISK_NOTE: For non-trivial changes, the assistant SHOULD mention at least one plausible risk or failure mode when it seems useful to the user.
[CX-542] OPTIONS_RECOMMENDED: For bigger design choices, the assistant SHOULD prefer giving one recommended path plus at least one credible alternative.
[CX-543] OPTIONS_FIXED: If the user has already made the choice, the assistant MAY skip alternatives and SHOULD acknowledge that the choice is fixed.

### 6.5 Answer Structure and Self-Check (Lenient)

[CX-550] ANSWER_SHAPE: For substantial answers, the assistant SHOULD structure output into:
- ANSWER: direct response or proposed design.
- RATIONALE: short explanation or trade-offs.
- PATCHES / CHANGES: concrete changes if relevant.
- NEXT_STEPS: optional follow-up actions.

[CX-551] DCR_OPTIONAL: The assistant SHOULD internally run a simple Draft â†’ Critique â†’ Refine loop for substantial or risky tasks; this MAY be skipped for small, mechanical edits.
[CX-552] SELF_CHECK_SOFT: Before finalising substantial answers, the assistant SHOULD briefly self-check for correctness vs artefacts/specs and for obvious gaps; explicit self-check commentary in the answer is OPTIONAL unless requested.
[CX-553] RUBRIC_RESPECT: If the user provides a quality rubric/checklist, the assistant MUST respect it and SHOULD say that it followed it.
[CX-554] NO_SCOPE_SWAP: The assistant MUST NOT silently change, narrow, or expand the user's requested task scope; if it proposes a different or smaller scope, it MUST state this explicitly.

### 6.6 Consistency with Prior Work

[CX-560] CONSISTENCY_PRIOR: The assistant SHOULD aim to keep new answers consistent with prior decisions and cited specs in the conversation.
[CX-561] CONSISTENCY_CONFLICT: On spotting a conflict, the assistant SHOULD flag it and propose either adjusting the new answer or revisiting the earlier decision with user confirmation.

---

### 6.7 Review and Validation Gate

[CX-570] REVIEW_GATE: Any repo-changing patch MUST be reviewed (by a distinct Reviewer role/agent or an explicit review pass) before merge or before being treated as "done".
[CX-571] REVIEW_MIN_OUTPUT: A review MUST record: intent summary, key risks, required fixes, and exact validation commands run (or explicitly not run) with outcomes.
[CX-572] OK_REQUIRES_VALIDATION: The assistant MUST NOT claim a change is "OK", "verified", or "working" unless either (a) tests/checks ran and passed, or (b) the user explicitly validated the behaviour.
[CX-573] TRACEABILITY_MIN: Repo-changing work MUST be traceable to a work item (task packet / log entry / issue ID) referenced in the review note and ideally in the commit message.
[CX-573A] AI_VALIDATOR_GATE: Repo-changing work MUST be validated by the designated AI Validator agent (Red Hat Auditor) against the Quality Rubric and the Master Spec Main Body. The Validator's report is the primary evidence for closure.

### 6.7A The Quality Rubric Gate

[CX-573B] RUBRIC_DRIVEN_VALIDATION: All non-trivial work packets delivered by a Coder role MUST be evaluated by the Orchestrator/Validator role against the official Quality Rubric. The Coder MUST use the rubric for self-assessment before submitting work, and the Validator MUST use it for the final review.

| Category | Needs Improvement (1) | Meets Expectations (2) | Exceeds Expectations (3) |
| :--- | :--- | :--- | :--- |
| **Correctness & Functionality** | Feature is incomplete, buggy, or does not meet the core requirements of the task packet. | Feature is implemented correctly as per the spec. All validation commands pass. | Functionality is robust, handles edge cases not explicitly mentioned, and is highly polished. |
| **Code Quality & Readability** | Code is difficult to understand, violates project conventions, or is poorly structured. | Code is clear, follows existing project conventions and style, and is reasonably easy to follow. | Code is exceptionally clear, idiomatic, and improves the structure of the surrounding code. |
| **Testing & Verification** | No tests are added for new functionality, or existing tests are broken. | New functionality is covered by adequate tests (unit or integration). All tests pass. | Tests are comprehensive, covering important edge cases, and significantly improve confidence in the code's reliability. |
| **Hygiene & Best Practices** | Linter fails. Obvious "code smells" (e.g., very large functions, commented-out code, magic numbers) are introduced. | Code passes all linter checks. Follows general best practices for the language and framework. | Code not only passes checks but actively reduces technical debt (e.g., refactors a messy section, improves typing). |
| **Reporting & Communication**| Report is missing, inaccurate, or does not provide the requested information for validation. | Report is accurate, complete, and provides all information requested in the task packet's `REPORTING` section. | Report provides extra insights, clearly explains complex trade-offs, and proactively identifies future risks or opportunities. |

[CX-573C] VALIDATOR_PROTOCOL: The Validator role MUST follow `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`. This requires evidence-based inspection (Spec-to-Code mapping, Hygiene Audit, Test Verification) and the production of a structured Validation Report. "Rubber-stamping" (approving without evidence) is strictly prohibited.

[CX-573D] ZERO_PLACEHOLDER_POLICY (HARD): Production code under `/src/` MUST NOT contain "placeholder" logic, "hollow" structs, or "mock" implementations for core architectural invariants (Tokenization, Security Gates, Storage Guards). If an external dependency is missing, the task is BLOCKED, not "Baseline."

[CX-573E] FORBIDDEN_PATTERN_AUDIT (HARD): Before issuing a PASS verdict, the Validator MUST execute a `search_file_content` for "Forbidden Patterns" defined in the Spec (e.g., `split_whitespace`, `unwrap`, `Value`). If a forbidden pattern is found in a production path, the verdict is AUTO-FAIL.

---

### 6.8 Bootstrap Navigation Protocol (Non-Negotiable)

[CX-574] BOOTSTRAP_READ_SET: Before proposing changes, debugging, or reviewing, the assistant MUST read: `.GOV/roles_shared/docs/START_HERE.md` and `.GOV/spec/SPEC_CURRENT.md` (and the current logger if bootloader is active).
[CX-575] BOOTSTRAP_TASK_TYPE: The assistant MUST classify the task as one of: `DEBUG | FEATURE | REVIEW | REFACTOR | HYGIENE`.
[CX-576] BOOTSTRAP_FOLLOWUP_READ: After classification, the assistant MUST read the matching guide:
- DEBUG -> `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
- FEATURE/REFACTOR -> `.GOV/roles_shared/docs/ARCHITECTURE.md`
- REVIEW -> `.GOV/roles_shared/docs/ARCHITECTURE.md` + the diff/patch + validation instructions
[CX-577] BOOTSTRAP_OUTPUT_BLOCK: The assistant's first response in the session MUST include a short BOOTSTRAP block with:
- FILES_TO_OPEN: 5â€“15 concrete repo paths it will inspect first.
- SEARCH_TERMS: 5â€“20 exact strings/symbols/error codes it will grep.
- RUN_COMMANDS: the exact commands it will run (or `UNKNOWN` with explicit TODO placeholders).
- RISK_MAP: 3â€“8 likely failure modes and which subsystem they map to.
[CX-577A] BOOTSTRAP_TEMPLATE: The BOOTSTRAP block SHOULD follow this shape:
```
BOOTSTRAP
- FILES_TO_OPEN: .GOV/roles_shared/docs/START_HERE.md; .GOV/spec/SPEC_CURRENT.md; .GOV/roles_shared/docs/ARCHITECTURE.md; .GOV/roles_shared/docs/RUNBOOK_DEBUG.md; <feature/debug-specific paths>
- SEARCH_TERMS: "<key symbol>"; "<error>"; "<command>"; "<feature name>"
- RUN_COMMANDS: pnpm -C {{FRONTEND_ROOT_DIR}} tauri dev; pnpm -C {{FRONTEND_ROOT_DIR}} test; cargo test --manifest-path {{BACKEND_CARGO_TOML}}; (add task-specific)
- RISK_MAP: "<risk> -> <subsystem>"; "<risk> -> <subsystem>"
```
[CX-578] NAVIGATION_UPDATE_TRIGGER: When work uncovers new entrypoints, invariants, or a repeatable failure mode, the assistant MUST update the relevant doc in `/.GOV/roles_shared/` (START_HERE/ARCHITECTURE/RUNBOOK_DEBUG) as part of the same work packet/commit unless the user explicitly defers.
[CX-579] NAVIGATION_GATE: For non-trivial repo-changing work, the reviewer MUST block completion if no `/.GOV/roles_shared/` navigation pointer was added/updated (or a clear justification is recorded).

### 6.9 Orchestrator Task Packet Protocol (AI Autonomy - Mandatory)

[CX-580] ORCH_PACKET_REQUIRED: Orchestrators MUST create a task packet before delegating work to coder/debugger agents. The packet MUST be written to `.GOV/task_packets/{WP_ID}.md` OR embedded in the handoff message with full structure.

[CX-580C] ORCH_WP_ID_NAMING (HARD): Work Packet IDs and filenames MUST NOT include date/time stamps. Use `WP-{phase}-{name}` and, if a revision is required, `WP-{phase}-{name}-v{N}` (e.g., `WP-1-Tokenization-Service-v3`).
Legacy note: historical packets may contain date-coded IDs created before this invariant; do not create new date-stamped packet IDs. All new revisions MUST use `-v{N}`.

[CX-580D] WP_TRACEABILITY_REGISTRY (HARD): Base WP IDs are stable planning identifiers; when multiple packet revisions exist for the same Base WP, the Orchestrator MUST record the mapping (Base WP â†’ Active Packet) in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`. Coders and Validators MUST consult the registry; if the mapping is missing or ambiguous, work is BLOCKED until resolved.

[CX-580E] WP_LINEAGE_AUDIT_VARIANTS (HARD): When creating a revision packet (`-v{N}`) for a Base WP, the Orchestrator MUST perform and record a **Lineage Audit** that proves the Base WP (and ALL its prior packet versions) are a correct translation of: Roadmap pointer â†’ Master Spec Main Body â†’ repo code. The audit MUST validate that no requirements were lost/forgotten across versions and that the current repo state satisfies every governing Main Body MUST/SHOULD for that Base WP. If the audit is missing or incomplete, delegation is BLOCKED.

[CX-580A] ORCH_NO_CODING_BLOCK (HARD): The Orchestrator role is **STRICTLY FORBIDDEN** from modifying `{{BACKEND_ROOT_DIR}}/`, `{{FRONTEND_ROOT_DIR}}/`, `tests/`, or `.GOV/scripts/`. This is an absolute constraint; no automated response or work can override this.

[CX-580B] ORCH_NO_ROLE_SWITCH (HARD): The Orchestrator role is **STRICTLY FORBIDDEN** from switching to the Coder role. The Orchestrator's turn ends immediately upon task delegation. No automated response or work can override this constraint.

[CX-581] ORCH_PACKET_STRUCTURE: Every packet MUST include:
- TASK_ID: WP-{phase}-{short-name}
- RISK_TIER: LOW | MEDIUM | HIGH
- USER_CONTEXT: Non-technical explainer (APPEND-ONLY [CX-654])
- SCOPE: Clear description of what's in/out of scope
- IN_SCOPE_PATHS: Specific files/directories
- OUT_OF_SCOPE: What NOT to change
- TEST_PLAN: Exact validation commands
- DONE_MEANS: Specific success criteria
- ROLLBACK_HINT: How to undo changes
- BOOTSTRAP: FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP

[CX-582] ORCH_PACKET_VERIFICATION: The orchestrator MUST verify the packet file exists (if file-based) OR that the embedded packet is complete before delegating work.

[CX-583] ORCH_HANDOFF_PROTOCOL: When delegating to a coder agent, the orchestrator MUST include:
- Path to task packet file (if file-based) OR full packet content (if embedded)
- WP_ID for traceability
- RISK_TIER from packet
- Explicit confirmation: "âœ… Task packet {WP_ID} created and verified"

[CX-584] ORCH_BLOCKING_RULE: If the orchestrator cannot create a complete packet (unclear requirements, missing context, ambiguous scope), it MUST STOP and request clarification from the user. The orchestrator MUST NOT delegate incomplete or ambiguous work.

[CX-585] ORCH_TASK_BOARD_UPDATE: The orchestrator SHOULD update `.GOV/roles_shared/records/TASK_BOARD.md` upon creating a task packet. Logger entries for task creation are OPTIONAL and generally discouraged to avoid noise.

[CX-585F] TASK_BOARD_ENTRY_FORMAT (HARD): `.GOV/roles_shared/records/TASK_BOARD.md` entries MUST be minimal in all non-planning states. Specifically: entries in `## In Progress`, `## Done`, and `## Superseded (Archive)` MUST include only the WP identifier and the current status token (e.g., `[IN_PROGRESS]`, `[VALIDATED]`, `[FAIL]`, `[OUTDATED_ONLY]`, `[SUPERSEDED]`). Planning/backlog lists (e.g., `## Ready for Dev`) MAY contain additional notes temporarily, but final verdict reasoning MUST live in the task packet / validator report (not the Task Board).

[CX-585A] MANDATORY_SPEC_REFINEMENT (THE STRATEGIC PAUSE): The Orchestrator MUST use the "Refinement Loop" to ensure the Master Spec reflects the detailed design/requirements of the task BEFORE delegation.
- **Spec-Version Lock:** The Orchestrator is **FORBIDDEN** from outputting a final Task Packet for delegation unless it has **first** created a new version of the Master Spec (`v02.xx+1`) that explicitly defines the technical approach (env vars, signatures, constraints).
- **The Strategic Pause:** This pause exists to allow the user (non-coder) to enrich the Main Body, especially if methods or software choices deviate from the original plan. Document these shifts in the Main Body for hygiene and provenance.
- **Pointer Update:** `.GOV/spec/SPEC_CURRENT.md` MUST point to this new version.
- **Delegation Block:** If the Spec does not contain the exact requirements, delegation is BLOCKED. We do not "implement then specify"; we "specify then implement".

[CX-585B] RED_HAT_REVIEW: During the "Proposed" phase, the Orchestrator MUST perform a "Red Hat" review (looking for risks, security flaws, architectural debt) and refine the task packet to address them.

[CX-585C] UNIQUE_USER_SIGNATURE: Every `USER_SIGNATURE` provided by the human user MUST be globally unique within the repository. AI agents are **STRICTLY FORBIDDEN** from fabricating, guessing, or reusing a signature string. If a signature is missing or identical to a previous one, the Refinement Loop is **BLOCKED**.

[CX-585D] THE_STRATEGIC_PAUSE: The mandatory pause during the Refinement Loop exists to prevent "automation momentum". It allows the human co-author to enrich topics, change direction, and validate the technical approach before code is written.

[CX-585E] MAIN_BODY_ENRICHMENT_MANDATORY: Technical details (schemas, API signatures, error codes, logic invariants) MUST be documented in the **Main Body** of the Master Spec (Sections 1-6 or 9-11). The **Roadmap** (Section 7.6) is reserved for high-level scheduling and MUST point to the relevant Main Body section for implementation details. Task Packets MUST reference the Main Body sections as their primary authority.

[CX-585G] REFINEMENT_BLOCK_IN_CHAT (HARD): Before requesting any USER_SIGNATURE or delegating work, the Orchestrator MUST paste the full Technical Refinement Block into the chat for explicit user review/approval. Writing it only to disk (e.g., `.GOV/refinements/*.md`) is insufficient.

[CX-586] ORCH_AUTHORITY_DOCS: Packets MUST include pointers to: `.GOV/roles_shared/docs/START_HERE.md`, `.GOV/spec/SPEC_CURRENT.md`, `.GOV/roles_shared/docs/ARCHITECTURE.md`, `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`, `.GOV/roles_shared/docs/QUALITY_GATE.md` (logger pointer OPTIONAL, only if logger will be used for this WP).

[CX-587] ORCH_PRE_WORK_CHECK: Before delegating, the orchestrator SHOULD run (or instruct the coder to run): `just pre-work {WP_ID}` to verify the packet is complete and system is ready for work.

### 6.10 Coder Pre-Work Verification (AI Autonomy - Mandatory)

[CX-620] CODER_PACKET_CHECK: Before writing any code, the coder agent MUST verify a task packet exists by checking:
1. File exists at `.GOV/task_packets/WP-*.md` (created recently), OR
2. Orchestrator message includes complete TASK_PACKET block

[CX-621] CODER_BLOCKING_RULE: If no task packet is found, the coder MUST:
1. Output: "âŒ BLOCKED: No task packet found [CX-620]"
2. STOP all work immediately
3. Request task packet from orchestrator or user
4. DO NOT write any code until packet is verified

[CX-622] CODER_BOOTSTRAP_MANDATORY: The coder MUST output a BOOTSTRAP block per [CX-577] BEFORE the first file modification. This confirms the coder has read the task packet and understands scope.

[CX-625] INTERFACE-FIRST INVARIANT: For non-trivial tasks, the coder MUST output the proposed **Traits, Structs, or Interfaces** (The Skeleton) and receive Validator approval before implementing any logic.

[CX-623] CODER_VALIDATION_LOG: Before claiming work is complete, the coder MUST:
1. Run all commands from TEST_PLAN
2. Document results in a VALIDATION block
3. Include command + outcome for each check
4. Run `just post-work {WP_ID}` to verify completeness

[CX-627] EVIDENCE_MAPPING_REQUIREMENT: The coder's final report MUST include an `EVIDENCE_MAPPING` block mapping every "MUST" requirement from the Spec to specific lines of code.

### 6.11 Hygiene Gate (commands + scope)

[CX-630] HYGIENE_SCOPE: Changes SHOULD stay scoped to the task; avoid drive-by refactors or unrelated cleanups.
[CX-631] HYGIENE_COMMANDS: For repo-changing work, assistants SHOULD run (or explicitly note not run): `just docs-check`; `just codex-check`; `pnpm -C {{FRONTEND_ROOT_DIR}} run lint`; `pnpm -C {{FRONTEND_ROOT_DIR}} test`; `pnpm -C {{FRONTEND_ROOT_DIR}} run depcruise`; `cargo fmt`; `cargo clippy --all-targets --all-features`; `cargo test --manifest-path {{BACKEND_CARGO_TOML}}`; `cargo deny check advisories licenses bans sources`.
[CX-632] HYGIENE_TODOS: When touching code near TODOs, assistants SHOULD either resolve them or leave a dated note explaining why they remain.
[CX-633] HYGIENE_DOC_UPDATE: If new entrypoints, commands, or repeatable failures are introduced or discovered, assistants SHOULD update the relevant doc (START_HERE/ARCHITECTURE/RUNBOOK_DEBUG) in the same packet unless the user defers.

### 6.12 Determinism Anchors (large-system hygiene)

[CX-640] ANCHOR_ERRORS: New errors SHOULD include stable error codes (`{{ISSUE_PREFIX}}-####`) and/or log tags; these anchors SHOULD be referenced in `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` when adding repeatable failures.
[CX-641] OWNERSHIP_MAP: Area/module ownership SHOULD be captured in `/.GOV/roles_shared/docs/OWNERSHIP.md` with paths, reviewers, and notes; packets SHOULD consult/update it when adding new surface area.
[CX-642] PRIMITIVE_TESTS: New primitives/features SHOULD ship with at least one targeted test and a short invariant note (place in `.GOV/roles_shared/docs/ARCHITECTURE.md` or inline doc comment); silence requires an explicit reason.
[CX-643] CI_GATE: Continuous integration SHOULD run `just validate` (or an equivalent subset) and block merge on failures.
[CX-644] FLAGS: New interwoven features SHOULD use a feature flag or clearly documented toggle; note the flag/toggle location in `.GOV/roles_shared/docs/ARCHITECTURE.md` or the relevant module doc.
[CX-645] ERROR_CODES_REQUIRED: New errors SHOULD introduce stable error codes/log tags (e.g., `{{ISSUE_PREFIX}}-####`) and record them in `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` when they become repeatable.
[CX-646] TEST_EXPECTATION: Logic changes SHOULD add or update at least one targeted test; if omitted, a written reason MUST be recorded in the review/task packet.
[CX-647] REVIEW_REQUIRED: Repo-changing work SHOULD have a distinct reviewer role sign off, recording commands run and outcomes.
[CX-648] SECRETS_AND_SUPPLY_CHAIN: CI SHOULD include secret scanning and dependency audit steps; assistants MUST avoid committing secrets and SHOULD pin critical dependencies/lockfiles.
[CX-649] ROLLBACK_HINTS: Reviews/commits SHOULD include a brief rollback hint (e.g., git hash or steps) for traceability.
[CX-649A] TODO_POLICY: New TODOs in source code and scripts MUST include a tracking tag in the form `TODO({{ISSUE_PREFIX}}-####): ...` and be searchable by ID. Docs SHOULD use `TBD ({{ISSUE_PREFIX}}-####)` or explicit prose instead of TODO.

### 6.13 Task Packets as Primary Log; Logger Milestone-Only

[CX-650] TASK_LOG_PRIMARY: `.GOV/roles_shared/records/TASK_BOARD.md` + the task packet are the primary, mandatory micro-log for day-to-day work. Validation commands/outcomes and status updates MUST be recorded in the task packet. The {{PROJECT_DISPLAY_NAME}} logger is optional and reserved for milestones or hard bugs when explicitly requested.

[CX-651] LOGGER_USE_CASES: The {{PROJECT_DISPLAY_NAME}} logger SHOULD be used only when the user requests it or when recording major milestones/critical incidents. Routine Work Packet completion MUST NOT be blocked on a logger entry.

[CX-652] TASK_PACKET_VALIDATION: Before requesting commit, the coder MUST verify the task packet contains a VALIDATION block with commands run and outcomes, and that `.GOV/roles_shared/records/TASK_BOARD.md` reflects the current status.

[CX-653] TASK_PACKET_UNIQUENESS: Each Work Packet MUST have its own task packet file (do not reuse an old file for a new WP). Status/notes/validation may be updated within that WP's file as the work progresses.

---

## 7. Bootloader Integration (Optional)

[CX-700] BOOTLOADER_OPTIONAL: Micro-Logger, Diary, or other bootloaders are optional; this codex MUST remain usable without them.
[CX-701] BOOTLOADER_ACTIVE: When either (a) the user declares bootloader mode, or (b) a bootloader artefact is present in-session, bootloader schemas and rules become additional behavioural LAW unless explicitly disabled.

[CX-702] BOOTLOADER_DISABLE: If the user explicitly disables bootloader mode for a session, the assistant MUST treat bootloader rules as inactive for that session.

[CX-710] BOOTLOADER_STACK: Under a bootloader, the assistant MUST obey:
- Bootloader rules for logging, timestamps, and schemas.
- Hard invariants in Â§2.
- Spec usage rules in Â§5.

[CX-720] BOOTLOADER_SCHEMA_NO_TOUCH: The assistant MUST NOT change bootloader schemas unless explicitly asked to edit the bootloader itself.
[CX-721] BOOTLOADER_NO_FAKE: The assistant MUST NOT fabricate past log entries or fake history.

[CX-730] BOOTLOADER_HANDOVER: At natural boundaries in bootloader mode, the assistant SHOULD provide a short handover summary (what changed, main risks, where to continue).

---

## 8. Drift and Known Deviations

[CX-800] DRIFT_AWARENESS: The assistant SHOULD assume the codex may occasionally lag behind the actual repo; when mismatch is detected, it SHOULD call it out instead of forcing the repo to match a clearly stale rule.
[CX-801] KNOWN_DEVIATIONS_SECTION: A `KNOWN_DEVIATIONS` section MAY be added by the user to document intentional gaps between codex and reality; assistants SHOULD treat that section as overriding older conflicting rules.

[CX-810] KNOWN_DEVIATION_APP_LAYOUT: Repos may deviate from codex layout guidance. If codex guidance conflicts with the repo's observed layout (e.g., frontend under `{{FRONTEND_ROOT_DIR}}/` and shell under `{{FRONTEND_SRC_DIR}}-tauri/`), assistants MUST follow the observed layout and document the deviation in `.GOV/roles_shared/docs/ARCHITECTURE.md`.
[CX-811] KNOWN_DEVIATION_MULTI_SPECS: The repo may contain multiple `{{PROJECT_PREFIX}}_Master_Spec_v*.md` versions at root. `.GOV/spec/SPEC_CURRENT.md` is the authoritative pointer for current work.
[CX-812] KNOWN_DEVIATION_DOC_SPLIT: `/.GOV/` is canonical operational guidance; `/.GOV/operator/docs_local/` is staging/drafts; root-level `*.md` may contain governance/history.

---

## 9. Automated Enforcement (AI Autonomy Requirements)

[CX-900] ENFORCEMENT_PURPOSE: For AI-autonomous operation, the workflow MUST be enforced by automated scripts and checks. Manual enforcement is insufficient when the human user lacks coding expertise.

[CX-901] ENFORCEMENT_SCRIPTS: The repo MUST include enforcement scripts in `/.GOV/scripts/validation/`:
- `pre-work-check.mjs` - Verifies task packet exists before work starts
- `post-work-check.mjs` - Verifies task packet validation/status (logger only if requested)
- `task-packet-check.mjs` - Validates packet structure
- `ci-traceability-check.mjs` - CI verification of workflow compliance

[CX-902] ENFORCEMENT_HOOKS: Git hooks SHOULD enforce:
- pre-commit: Blocks commits without WP-ID traceability
- pre-push: Verifies all commits reference valid task packets

[CX-903] ENFORCEMENT_JUST: The `justfile` MUST include:
- `just create-task-packet {wp-id}` - Creates task packet from template
- `just pre-work {wp-id}` - Validates readiness before implementation
- `just post-work {wp-id}` - Validates completeness before commit
- `just validate-workflow {wp-id}` - Full workflow compliance check

[CX-904] ENFORCEMENT_CI: GitHub Actions SHOULD verify:
- All commits reference task packets via WP-ID
- Validation commands are documented in task packets/commits/reviews
- Logger entries are only required when explicitly requested (milestones/hard bugs)
- No commits bypass workflow requirements

[CX-905] ENFORCEMENT_FAILURE: If automated checks fail, work MUST be rejected with:
1. Clear error message indicating which rule was violated
2. Reference to codex rule number (e.g., "[CX-620]")
3. Remediation steps to fix the issue
4. AI agents MUST NOT override enforcement without explicit user permission

[CX-906] ENFORCEMENT_PROTOCOLS: The repo MUST include protocol files in `.GOV/roles/`:
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` - Mandatory checklist for orchestrators
- `.GOV/roles/coder/CODER_PROTOCOL.md` - Mandatory checklist for coders
- These protocols MUST be read by AI agents before performing their respective roles

---

## 10. Versioning

[CX-950] VERSION_ID: This codex is `{{PROJECT_DISPLAY_NAME}} Codex v{{CODEX_VERSION}} (AI Autonomy with Deterministic Enforcement)`.
[CX-951] VERSION_FROM: v1.4 supersedes v1.3 for all use. v1.3 MAY still be referenced for comparison but v1.4 is authoritative.

[CX-960] CHANGE_SUMMARY_V08_1: v0.8 strengthens orchestrator and coder requirements from SHOULD to MUST for AI autonomy. Task packet creation [CX-580] and coder pre-work verification [CX-620] are now mandatory and blocking.

[CX-961] CHANGE_SUMMARY_V08_2: v0.8 adds Â§9 "Automated Enforcement" defining required scripts, hooks, and CI checks to enforce workflow deterministically without relying on AI agent compliance alone.

[CX-962] CHANGE_SUMMARY_V08_3: v0.8 clarifies workflow traceability: `.GOV/roles_shared/records/TASK_BOARD.md` + task packets are the primary micro-log; the {{PROJECT_DISPLAY_NAME}} logger is optional for milestones/hard bugs when explicitly requested.

[CX-963] CHANGE_SUMMARY_V08_4: v0.8 adds [CX-503] explicitly stating this codex is optimized for AI-autonomous operation where the human user may not have coding expertise.

[CX-964] CHANGE_SUMMARY_V08_5: v0.8 adds [CX-213] requiring `.GOV/task_packets/` directory and [CX-906] requiring `.GOV/roles/` protocol files for orchestrator/coder agents.

[CX-965] CHANGE_SUMMARY_V11: v1.1 adds [CX-598] and [CX-599] Hard Invariants regarding Main-Body alignment and cross-phase governance continuity. Standardizes versioning metadata across document.

[CX-966] CHANGE_SUMMARY_V12: v1.2 adds Lead Architect constraints for Orchestrators ([CX-585A-E]) and Senior Engineer constraints for Coders ([CX-625, CX-627]). Mandates Spec-Locking, Unique User Signatures, and Evidence Mapping to eliminate vibe-coding.

[CX-967] CHANGE_SUMMARY_V14: v1.4 adds Hard Invariants for Validators [CX-573D] (Zero Placeholder Policy) and [CX-573E] (Forbidden Pattern Audit) to prevent leniency. 

[CX-968] CHANGE_SUMMARY_V14_CODER: v1.4 adds Hard Invariants for Coders [CX-628] (Anti-Vibe Verification) and [CX-629] (Block-Over-Placeholder) to force adversarial self-scrutiny before submission.

---

## SUMMARY FOR AI AGENTS

**If you are an Orchestrator:**
1. Read `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` FIRST
2. **Refine the Spec FIRST** [CX-585A]
3. Create task packet (`just create-task-packet WP-{ID}`) â€” new file per WP
4. Update `.GOV/roles_shared/records/TASK_BOARD.md` to "Ready for Dev"
5. Verify (`just pre-work WP-{ID}`)
6. Only then delegate to coder

**If you are a Coder/Debugger:**
1. Read `.GOV/roles/coder/CODER_PROTOCOL.md` FIRST
2. Verify task packet exists [CX-620]
3. **Extract Verbatim Spec** [CX-624]
4. **Propose Skeleton/Interface** [CX-625]
5. Set task packet `**Status:** In Progress` + claim fields and create a docs-only bootstrap claim commit (Validator status-syncs `main`)
6. Output BOOTSTRAP block [CX-622]
7. Implement within scope
8. **Run Anti-Vibe Verification [CX-628]** (Search for `split_whitespace`, `unwrap`, etc.)
9. **Enforce Block-Over-Placeholder [CX-629]**
10. Run validation (`just post-work {WP_ID}`)
11. **Map Evidence to Spec** [CX-627]
12. Request Validator validation/merge (Validator updates `main` Task Board to Done on PASS/FAIL)

**If you are a Reviewer/Validator:**
1. Verify task packet exists for the work
2. Verify evidence mapping exists and is accurate [CX-627]
3. **Execute Forbidden Pattern Audit [CX-573E]** (Search for `split_whitespace`, `unwrap`, etc.)
4. **Enforce Zero Placeholder Policy [CX-573D]**
5. Produce a structured Validation Report per VALIDATOR_PROTOCOL.md
6. Block merge if workflow was bypassed or spec alignment is incomplete

**Blocking rules apply.** If any MUST requirement is violated, work stops until fixed.

````

###### Template File: `justfile`
Intent: Single command surface for governance gates and hygiene (mechanical enforcement).
````make
set dotenv-load := false
# Use a Windows-friendly shell if available; defaults remain for *nix.
# Powershell is present on Windows by default.
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

dev:
	cd {{FRONTEND_ROOT_DIR}}; pnpm run tauri dev

lint:
	cd {{FRONTEND_ROOT_DIR}}; pnpm run lint
	cd {{BACKEND_CRATE_DIR}}; cargo clippy --all-targets --all-features

test:
	cd {{BACKEND_CRATE_DIR}}; cargo test

# Fail if any required docs are missing (navigation pack + past work index)
docs-check:
	node -e "['.GOV/roles_shared/docs/START_HERE.md', '.GOV/spec/SPEC_CURRENT.md', '.GOV/roles_shared/docs/ARCHITECTURE.md', '.GOV/roles_shared/docs/RUNBOOK_DEBUG.md', '.GOV/roles_shared/PAST_WORK_INDEX.md'].forEach(f => { if (!require('fs').existsSync(f)) { console.error('Missing: ' + f); process.exit(1); } })"

# Format backend Rust
fmt:
	cd {{BACKEND_CRATE_DIR}}; cargo fmt

# Clean Cargo artifacts in the external target dir ({{CARGO_TARGET_DIR}})
cargo-clean:
	cargo clean -p {{BACKEND_CRATE_NAME}} --manifest-path {{BACKEND_CARGO_TOML}} --target-dir "{{CARGO_TARGET_DIR}}"

# Full hygiene pass: docs, lint, tests, fmt, clippy
validate:
	just docs-check
	just codex-check
	just scaffold-check
	just codex-check-test
	cd {{FRONTEND_ROOT_DIR}}; pnpm run lint
	cd {{FRONTEND_ROOT_DIR}}; pnpm test
	cd {{FRONTEND_ROOT_DIR}}; pnpm run depcruise
	cd {{BACKEND_CRATE_DIR}}; cargo fmt
	cd {{BACKEND_CRATE_DIR}}; cargo clippy --all-targets --all-features
	cd {{BACKEND_CRATE_DIR}}; cargo test
	cargo deny check advisories licenses bans sources

# Codex guardrails: prevent direct fetch in components, println/eprintln in backend, and doc drift.
codex-check:
	node .GOV/scripts/validation/codex-check.mjs

# Worktrees (recommended when >1 WP active)
# Creates a dedicated working directory for the WP branch.
worktree-add wp-id base="main" branch="" dir="":
	node .GOV/scripts/worktree-add.mjs {{wp-id}} {{base}} {{branch}} {{dir}}

task-board-check:
	node .GOV/scripts/validation/task-board-check.mjs

task-packet-claim-check:
	node .GOV/scripts/validation/task-packet-claim-check.mjs

# Dependency cruise (frontend architecture)
depcruise:
	cd {{FRONTEND_ROOT_DIR}}; pnpm run depcruise

# Dependency & license checks (Rust)
deny:
	cargo deny check advisories licenses bans sources

# Scaffolding
new-react-component name:
	node .GOV/scripts/new-react-component.mjs {{name}}

new-api-endpoint name:
	node .GOV/scripts/new-api-endpoint.mjs {{name}}

scaffold-check:
	node .GOV/scripts/scaffold-check.mjs

codex-check-test:
	node .GOV/scripts/codex-check-test.mjs

# Close a WP branch after it has been merged into main.
close-wp-branch wp-id remote="":
	node .GOV/scripts/close-wp-branch.mjs {{wp-id}} {{remote}}

# === Workflow Enforcement Commands (Codex v{{CODEX_VERSION}}) ===

# Record a technical refinement for a work packet [CX-585A]
record-refinement wp-id detail="":
	@node .GOV/scripts/validation/orchestrator_gates.mjs refine {{wp-id}} "{{detail}}"

# Record a user signature for a work packet [CX-585C]
record-signature wp-id signature:
	@node .GOV/scripts/validation/orchestrator_gates.mjs sign {{wp-id}} {{signature}}

# Record WP preparation (branch/worktree + coder assignment) after signature and before packet creation.
record-prepare wp-id coder_id branch="" worktree_dir="":
	@node .GOV/scripts/validation/orchestrator_gates.mjs prepare {{wp-id}} {{coder_id}} {{branch}} {{worktree_dir}}

# Create new task packet from template [CX-580]
create-task-packet wp-id:
	@echo "Creating task packet: {{wp-id}}..."
	@node .GOV/scripts/create-task-packet.mjs {{wp-id}}

# Pre-work validation - run before starting implementation [CX-587, CX-620]
pre-work wp-id:
	@just gate-check {{wp-id}}
	@node .GOV/scripts/validation/pre-work-check.mjs {{wp-id}}

# Post-work validation - run before commit [CX-623, CX-651]
post-work wp-id:
	@just gate-check {{wp-id}}
	@node .GOV/scripts/validation/post-work-check.mjs {{wp-id}}

# Helper: compute deterministic COR-701 Pre/Post SHA1 for a file.
cor701-sha file:
	@node .GOV/scripts/validation/cor701-sha.mjs {{file}}

# Automated workflow validation for a work packet
validate-workflow wp-id:
	@echo "Running automated workflow validation for {{wp-id}}..."
	@echo ""
	@echo "Step 0: Gate Check"
	@just gate-check {{wp-id}}
	@echo ""
	@echo "Step 1: Pre-work check"
	@just pre-work {{wp-id}}
	@echo ""
	@echo "Step 2: Code quality validation"
	@just validate
	@echo ""
	@echo "Step 3: Post-work check"
	@just post-work {{wp-id}}
	@echo ""
	@echo "âœ… Automated workflow validation passed for {{wp-id}} (manual review required)"

# Gate check (protocol-aligned)
gate-check wp-id:
	@node .GOV/scripts/validation/gate-check.mjs {{wp-id}}

# Validator helpers (protocol-aligned)
validator-scan:
	@node .GOV/scripts/validation/validator-scan.mjs

validator-dal-audit:
	@node .GOV/scripts/validation/validator-dal-audit.mjs

validator-spec-regression:
	@node .GOV/scripts/validation/validator-spec-regression.mjs

validator-phase-gate phase="Phase-1":
	@node .GOV/scripts/validation/validator-phase-gate.mjs {{phase}}

validator-packet-complete wp-id:
	@node .GOV/scripts/validation/validator-packet-complete.mjs {{wp-id}}

validator-error-codes:
	@node .GOV/scripts/validation/validator-error-codes.mjs

validator-coverage-gaps *targets:
	@node .GOV/scripts/validation/validator-coverage-gaps.mjs {{targets}}

validator-traceability *targets:
	@node .GOV/scripts/validation/validator-traceability.mjs {{targets}}

validator-git-hygiene:
	@node .GOV/scripts/validation/validator-git-hygiene.mjs

validator-hygiene-full:
	@node .GOV/scripts/validation/validator-hygiene-full.mjs

# Validator Gate Commands [CX-VAL-GATE] - Mechanical enforcement of validation sequence
validator-gate-present wp-id verdict:
	@node .GOV/scripts/validation/validator_gates.mjs present-report {{wp-id}} {{verdict}}

validator-gate-acknowledge wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs acknowledge {{wp-id}}

validator-gate-append wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs append {{wp-id}}

validator-gate-commit wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs commit {{wp-id}}

validator-gate-status wp-id:
	@node .GOV/scripts/validation/validator_gates.mjs status {{wp-id}}

validator-gate-reset wp-id *confirm:
	@node .GOV/scripts/validation/validator_gates.mjs reset {{wp-id}} {{confirm}}

````

###### Template File: `.gitattributes`
Intent: Deterministic line-ending policy for governance artifacts (drift control).
````text
.gitattributes text eol=lf
.GOV/spec/SPEC_CURRENT.md text eol=lf
.GOV/roles_shared/docs/START_HERE.md text eol=lf
.GOV/roles_shared/records/TASK_BOARD.md text eol=lf
.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md text eol=lf
.GOV/roles_shared/records/SIGNATURE_AUDIT.md text eol=lf
.GOV/roles_shared/docs/QUALITY_GATE.md text eol=lf
.GOV/roles_shared/docs/ARCHITECTURE.md text eol=lf
.GOV/roles_shared/docs/RUNBOOK_DEBUG.md text eol=lf
.GOV/roles_shared/PAST_WORK_INDEX.md text eol=lf
.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json text eol=lf
.GOV/validator_gates/*.json text eol=lf
.GOV/roles/validator/VALIDATOR_GATES.json text eol=lf
.GOV/task_packets/*.md text eol=lf
.GOV/refinements/*.md text eol=lf
````

###### Template File: `.cargo/config.toml`
Intent: Deterministic build artifact location (keeps repo clean; avoids CI/worktree drift).
````toml
[build]
# Keep Cargo artifacts outside the repo to avoid bloating the workspace/Git mirror.
# NOTE: This points to a sibling directory dedicated to this repo (no other files).
target-dir = "{{CARGO_TARGET_DIR}}"

````

###### Template File: `deny.toml`
Intent: Supply-chain policy config for cargo-deny (license/advisory/bans/sources).
````toml
[advisories]
db-urls = ["https://github.com/RustSec/advisory-db"]
ignore = [
    "RUSTSEC-2025-0119", # number_prefix
    "RUSTSEC-2024-0436", # paste
]
yanked = "deny"

[licenses]
allow = [
  "Apache-2.0",
  "MIT",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "CC0-1.0",
  "Unlicense",
  "MPL-2.0",
  "Unicode-DFS-2016",
  "Unicode-3.0",
  "CDLA-Permissive-2.0",
]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]

````

###### Template File: `.github/workflows/ci.yml`
Intent: CI parity: runs the same (or stricter) mechanical gates as local.
````yaml
name: CI

on:
  push:
    branches: ["**"]
  pull_request:

permissions:
  contents: read

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"
          cache: "pnpm"
          cache-dependency-path: {{FRONTEND_ROOT_DIR}}/pnpm-lock.yaml

      - name: Enable corepack
        run: corepack enable

      - name: Install frontend deps
        run: pnpm -C {{FRONTEND_ROOT_DIR}} install --frozen-lockfile

      - name: Install ripgrep
        run: sudo apt-get update && sudo apt-get install -y ripgrep

      - name: Docs check
        run: |
          test -s .GOV/roles_shared/docs/START_HERE.md
          test -s .GOV/spec/SPEC_CURRENT.md
          test -s .GOV/roles_shared/docs/ARCHITECTURE.md
          test -s .GOV/roles_shared/docs/RUNBOOK_DEBUG.md
          test -s .GOV/roles_shared/PAST_WORK_INDEX.md

      - name: Codex checks
        run: |
          echo "Disallow direct fetch in {{FRONTEND_SRC_DIR}} (outside lib/api.ts)..."
          rg -n "\\bfetch\\s*\\(" {{FRONTEND_SRC_DIR}} | rg -v "{{FRONTEND_SRC_DIR}}/lib/api.ts" && exit 1 || exit 0
          echo "Disallow println!/eprintln! in backend..."
          rg -n "eprintln!|println!" {{BACKEND_SRC_DIR}} && exit 1 || exit 0
          echo "Docs must reference Codex v{{CODEX_VERSION}}..."
          rg -n "Codex v0\\.5|Codex v0\\.6|Codex v0\\.7|{{PROJECT_DISPLAY_NAME}} Codex v0\\.5|{{PROJECT_DISPLAY_NAME}} Codex v0\\.6|{{PROJECT_DISPLAY_NAME}} Codex v0\\.7" docs && exit 1 || exit 0
          echo "SPEC_CURRENT must reference latest master spec..."
          node .GOV/scripts/spec-current-check.mjs
          echo "Task Board entries must be minimal..."
          node .GOV/scripts/validation/task-board-check.mjs
          echo "In Progress task packets must include coder claim fields..."
          node .GOV/scripts/validation/task-packet-claim-check.mjs
          echo "TODOs must include {{ISSUE_PREFIX}} issue tags..."
          rg -n --pcre2 "TODO(?!\\({{ISSUE_PREFIX}}-\\d+\\))" {{FRONTEND_SRC_DIR}} src/backend scripts --glob "!.GOV/scripts/fixtures/**" --glob "!.GOV/scripts/codex-check-test.mjs" && exit 1 || exit 0

      - name: Git hygiene (build/cache artifacts)
        run: node .GOV/scripts/validation/validator-git-hygiene.mjs

      - name: Codex check tests
        run: node .GOV/scripts/codex-check-test.mjs

      - name: Frontend lint
        run: pnpm -C {{FRONTEND_ROOT_DIR}} run lint

      - name: Frontend architecture (dependency-cruiser)
        run: pnpm -C {{FRONTEND_ROOT_DIR}} run depcruise

      - name: Frontend tests
        run: pnpm -C {{FRONTEND_ROOT_DIR}} test

      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Rust fmt check
        working-directory: {{BACKEND_CRATE_DIR}}
        run: cargo fmt -- --check

      - name: Rust clippy
        run: cargo clippy --manifest-path {{BACKEND_CARGO_TOML}} --all-targets --all-features

      - name: Rust tests
        run: cargo test --manifest-path {{BACKEND_CARGO_TOML}}

  backend-storage:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        backend: [sqlite, postgres]
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: {{POSTGRES_TEST_DB}}
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U postgres -d {{POSTGRES_TEST_DB}}"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 20
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Configure SQLite env
        if: matrix.backend == 'sqlite'
        run: echo "DATABASE_URL=sqlite::memory:" >> $GITHUB_ENV

      - name: Configure Postgres env
        if: matrix.backend == 'postgres'
        run: |
          echo "POSTGRES_TEST_URL=postgres://postgres:postgres@localhost:5432/{{POSTGRES_TEST_DB}}" >> $GITHUB_ENV
          echo "DATABASE_URL=postgres://postgres:postgres@localhost:5432/{{POSTGRES_TEST_DB}}" >> $GITHUB_ENV

      - name: Storage conformance tests
        run: cargo test --manifest-path {{BACKEND_CARGO_TOML}} --tests storage_conformance

  secret_scan:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Gitleaks scan
        uses: gitleaks/gitleaks-action@v2

  dependency_audit:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"
          cache: "pnpm"
          cache-dependency-path: {{FRONTEND_ROOT_DIR}}/pnpm-lock.yaml

      - name: Enable corepack
        run: corepack enable

      - name: Install frontend deps
        run: pnpm -C {{FRONTEND_ROOT_DIR}} install --frozen-lockfile

      - name: Frontend audit
        run: pnpm -C {{FRONTEND_ROOT_DIR}} audit --audit-level high

      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-deny
        run: cargo install cargo-deny

      - name: Rust dependency policy
        run: cargo deny check advisories licenses bans sources

  traceability:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 50  # Need commit history for traceability

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"

      - name: Traceability check
        run: node .GOV/scripts/validation/ci-traceability-check.mjs

````

###### Template File: `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
Intent: Project-specific invariants (identity, naming, layout profile, tool paths). REQUIRED for Governance Pack instantiation.
````md
# PROJECT_INVARIANTS (HARD)

This file is REQUIRED in every repo that claims Governance Kernel conformance.
It is the single place where project-specific identity, naming, layout, and tool paths are declared.

## Identity
- PROJECT_CODE: {{PROJECT_CODE}}
- PROJECT_DISPLAY_NAME: {{PROJECT_DISPLAY_NAME}}
- PROJECT_PREFIX: {{PROJECT_PREFIX}}  # recommended: PROJECT_CODE (filesystem-safe; deterministic)

## Naming Policy (Repo Root)
- MASTER_SPEC_FILENAME_PREFIX: {{PROJECT_PREFIX}}_Master_Spec
- MASTER_SPEC_FILENAME_GLOB: {{PROJECT_PREFIX}}_Master_Spec_v*.md
- CODEX_VERSION: {{CODEX_VERSION}}
- CODEX_FILENAME: {{PROJECT_PREFIX}}_Codex_v{{CODEX_VERSION}}.md
- LOGGER_FILENAME_GLOB: {{PROJECT_PREFIX}}_logger_*.md

## Issue / Ticketing
- ISSUE_PREFIX: {{ISSUE_PREFIX}}  # used for TODO tagging, e.g., TODO({{ISSUE_PREFIX}}-1234)

## Language/Layout Guardrails (ALWAYS PRESENT)
- LANGUAGE_LAYOUT_PROFILE_ID: {{LANGUAGE_LAYOUT_PROFILE_ID}}
- FRONTEND_ROOT_DIR: {{FRONTEND_ROOT_DIR}}  # e.g., app
- FRONTEND_SRC_DIR: {{FRONTEND_SRC_DIR}}  # e.g., app/src
- BACKEND_ROOT_DIR: {{BACKEND_ROOT_DIR}}  # e.g., src/backend
- BACKEND_CRATE_NAME: {{BACKEND_CRATE_NAME}}  # e.g., handshake_core
- BACKEND_CRATE_DIR: {{BACKEND_CRATE_DIR}}  # e.g., src/backend/handshake_core
- BACKEND_SRC_DIR: {{BACKEND_SRC_DIR}}  # e.g., src/backend/handshake_core/src
- BACKEND_TESTS_DIR: {{BACKEND_TESTS_DIR}}  # e.g., src/backend/handshake_core/tests
- BACKEND_MIGRATIONS_DIR: {{BACKEND_MIGRATIONS_DIR}}  # e.g., src/backend/handshake_core/migrations
- BACKEND_CARGO_TOML: {{BACKEND_CARGO_TOML}}  # e.g., src/backend/handshake_core/Cargo.toml
- BACKEND_JOBS_DIR: {{BACKEND_JOBS_DIR}}  # e.g., src/backend/handshake_core/src/jobs
- BACKEND_LLM_DIR: {{BACKEND_LLM_DIR}}  # e.g., src/backend/handshake_core/src/llm
- BACKEND_STORAGE_DIR: {{BACKEND_STORAGE_DIR}}  # e.g., src/backend/handshake_core/src/storage
- BACKEND_OBSERVABILITY_DIR: {{BACKEND_OBSERVABILITY_DIR}}  # e.g., src/backend/handshake_core/src/observability
- BACKEND_API_DIR: {{BACKEND_API_DIR}}  # e.g., src/backend/handshake_core/src/api
- BACKEND_LOCAL_MODELS_DIR: {{BACKEND_LOCAL_MODELS_DIR}}  # e.g., src/backend/handshake_core/src/local_models
- BACKEND_PIPELINE_DIR: {{BACKEND_PIPELINE_DIR}}  # e.g., src/backend/handshake_core/src/content_pipeline
- BACKEND_UTIL_DIR: {{BACKEND_UTIL_DIR}}  # e.g., src/backend/handshake_core/src/util

## CI/Test Defaults (Optional)
- POSTGRES_TEST_DB: {{POSTGRES_TEST_DB}}  # e.g., {{PROJECT_PREFIX}}_test

## External Tool Paths (Optional but Explicit)
- CARGO_TARGET_DIR: {{CARGO_TARGET_DIR}}  # may be external/sibling dir; required if Rust is present
- CARGO_TARGET_DIR_NAME: {{CARGO_TARGET_DIR_NAME}}  # e.g., project-cargo-target
- NODE_PACKAGE_MANAGER: {{NODE_PACKAGE_MANAGER}}  # e.g., pnpm|npm|yarn
- ADDITIONAL_PATHS:
  - KEY: VALUE

````

###### Template File: `.GOV/spec/SPEC_CURRENT.md`
Intent: Authoritative pointer to the current Master Spec and Governance Reference (drift guard target).
````md
# SPEC_CURRENT

The current authoritative Master Specification is:

**{{MASTER_SPEC_FILENAME}}**

(Updated: 2026-01-13 - Inlined the full Governance Pack Template Volume (codex + role protocols + governance artifacts + mechanical hard-gate tooling) as project-agnostic templates and required PROJECT_INVARIANTS for project-specific naming/layout/tool paths [ilja130120260124])

---

The current authoritative Governance Reference is:

**{{CODEX_FILENAME}}**

````

###### Template File: `.GOV/roles_shared/docs/START_HERE.md`
Intent: Navigation pack for humans and agents (canonical entry point + workflow commands).
````md
# {{PROJECT_DISPLAY_NAME}} Project: Start Here

Authority: Master Spec (see `.GOV/spec/SPEC_CURRENT.md`).
---
## Canonical sources
- **Spec:** `.GOV/spec/SPEC_CURRENT.md` (points to the current master spec; see `.GOV/spec/SPEC_CURRENT.md`)..
- **WP Traceability:** `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP â†’ Active Packet mapping; resolves `-vN` revisions without putting WP IDs into the Master Spec).
- **Governance guardrails:** `{{PROJECT_DISPLAY_NAME}} Codex v{{CODEX_VERSION}}` (repo root) + `.GOV/roles_shared/records/TASK_BOARD.md` + task packets. {{PROJECT_DISPLAY_NAME}} logger is for milestones/hard bugs when requested.
- **Architecture & debug:** `.GOV/roles_shared/docs/ARCHITECTURE.md` and `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`.

## AI Agent Workflow (Mandatory for AI-Autonomous Operation)

**[CX-503, CX-580-623]** This repository is designed for AI-autonomous software engineering. Human users may not have coding expertise and rely on deterministic workflow enforcement.

**Two agent roles:**
1. **Orchestrator** â€” Creates task packets, delegates work, manages workflow
2. **Coder/Debugger** â€” Implements work per task packet scope

**Mandatory protocols:**
- **Orchestrators:** Read `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` before delegating
- **Coders:** Read `.GOV/roles/coder/CODER_PROTOCOL.md` before writing any code

**Workflow enforcement commands:**
```bash
# Orchestrator: Create task packet from template
just create-task-packet WP-{phase}-{name}

# Orchestrator: Verify packet complete before delegation
just pre-work WP-{ID}

# Coder: Verify packet exists before coding
just pre-work WP-{ID}

# Coder: Verify work complete before commit
just post-work WP-{ID}

# Full workflow validation (pre-work + validate + post-work)
just validate-workflow WP-{ID}
```

**Gate 0 (Pre-Work):** Task packet MUST exist and pass `just pre-work WP-{ID}` before implementation starts. If blocked, STOP and request help.

**Gate 1 (Post-Work):** All validation MUST pass `just post-work WP-{ID}` before commit. If blocked, fix issues and re-run.

**See:** `.GOV/roles_shared/docs/QUALITY_GATE.md` for Gate 0 and Gate 1 requirements.

## Repo map (open in an editor and `rg`)
- `{{FRONTEND_ROOT_DIR}}/` â€” frontend root (UI); source lives under `{{FRONTEND_SRC_DIR}}/`.
- `{{FRONTEND_SRC_DIR}}-tauri/` â€” Tauri shell; spawns `{{BACKEND_CRATE_NAME}}` from `{{BACKEND_CRATE_DIR}}`.
- `{{BACKEND_CRATE_DIR}}/` â€” Rust backend crate (API, data, logging).
- `src/shared/` â€” placeholder for cross-stack types/contracts (none defined yet).
- `tests/` â€” top-level test harness placeholder.
- `.GOV/scripts/` â€” ops/dev scripts (currently empty scaffold).
- `data/` â€” runtime artifacts; backend logs are written to `data/logs/{{BACKEND_CRATE_NAME}}.log`.
- `.GOV/` â€” governance workspace (canonical) + `.GOV/adr/` (accepted ADRs).
- `.GOV/operator/docs_local/` â€” staging/non-canonical notes and diaries.
- `log_archive/` â€” historical logger drops.
- `.GOV/roles_shared/docs/OWNERSHIP.md` â€” path/area owners for routing reviews.
- Root files: `{{PROJECT_PREFIX}}_Master_Spec_v*.md`, `{{CODEX_FILENAME}}`, `{{PROJECT_PREFIX}}_logger_*`, phase/plan docs.
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` and `.GOV/roles/coder/CODER_PROTOCOL.md` â€” AI agent workflow protocols.

## How to run
> **WARNING for AI Agents:** Commands like `pnpm -C {{FRONTEND_ROOT_DIR}} tauri dev` or `just dev` start a long-running development server. They MUST NOT be executed with a blocking tool (like `run_shell_command`). These commands should be run in a separate, dedicated terminal by the user or as a true background process.
```bash
# Frontend dev shell (Tauri + React)
pnpm -C {{FRONTEND_ROOT_DIR}} tauri dev

# With just (if installed)
just dev

# Backend tests
cargo test --manifest-path {{BACKEND_CARGO_TOML}}

# Frontend tests
pnpm -C {{FRONTEND_ROOT_DIR}} test

# Lint
pnpm -C {{FRONTEND_ROOT_DIR}} run lint
# or
just lint

# Full hygiene (lint/tests/depcruise/clippy/deny)
just validate

# Scaffolding
just new-react-component <ComponentName>
just new-api-endpoint <endpoint_name>

# Git hook (pre-commit checks)
git config core.hooksPath .GOV/scripts/hooks
```
If additional setup (DB seed, env) is required: TBD ({{ISSUE_PREFIX}}-1001) â€” document once known.

For task packets: include scope, expected behavior, in-scope paths, DONE_MEANS, BOOTSTRAP block (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP), and these commands.

CI expectation: run `just validate`; manual validator review is required for MEDIUM/HIGH risk work.

## Bug triage map (jump into RUNBOOK_DEBUG)
- UI/frontend: see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#ui-and-shell` (app React + Tauri window lifecycle).
- Backend/API/logic: see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#backend-api-and-logic` (Rust `api/*.rs`, models, logging).
- IPC / orchestrator (Tauri â†” Rust core): see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#ipc-tauri-bridge` (`{{FRONTEND_SRC_DIR}}-tauri/src/lib.rs` spawn + commands).
- Data/migrations/storage: see `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md#data-storage-and-migrations` (`migrations/`, SQLite, RDD model).

## More context
- Architecture table: `.GOV/roles_shared/docs/ARCHITECTURE.md`
- Debug runbook: `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
- Current spec + governance: `.GOV/spec/SPEC_CURRENT.md`
- Quality gate (risk tiers + required checks): `.GOV/roles_shared/docs/QUALITY_GATE.md`
- Task packet template: `.GOV/templates/TASK_PACKET_TEMPLATE.md`
- Workflow template for reuse: `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`

## Past work
Pointer to prior specs/logs/notes: `.GOV/roles_shared/PAST_WORK_INDEX.md`

````

###### Template File: `.GOV/roles_shared/docs/ARCHITECTURE.md`
Intent: Module/area map and allowed dependency boundaries (architecture drift control).
````md
# ARCHITECTURE

| Module/Area | Responsibility | Entry files/dirs | Allowed dependencies | Where to add features |
| --- | --- | --- | --- | --- |
| .claude/ (Claude Code instructions) | Local AI prompt/instruction storage for Claude Code | `.claude/` | None | Do not add features; instructions only |
| Frontend shell (Tauri + React) | Desktop window, UI components, invokes backend | `{{FRONTEND_SRC_DIR}}/main.tsx`, `{{FRONTEND_SRC_DIR}}/`, `{{FRONTEND_SRC_DIR}}-tauri/src/lib.rs` | Uses Tauri APIs, frontend packages, shared TS types when they land; may call backend via IPC/HTTP; avoid direct DB/filesystem writes except via Tauri | New UI flows/components in `{{FRONTEND_SRC_DIR}}`; new Tauri commands/wiring in `{{FRONTEND_SRC_DIR}}-tauri/src/lib.rs` |
| Backend core (Rust) | API + orchestration, data access, logging | `{{BACKEND_SRC_DIR}}/main.rs`, `{{BACKEND_SRC_DIR}}/api/*.rs`, `models.rs`, `logging.rs` | Rust crates, SQLite via migrations; expose commands/endpoints for frontend; do not depend on frontend code | Add endpoints in `{{BACKEND_SRC_DIR}}/api/`; data models in `models.rs`; logging via `logging.rs` |
| Data + migrations | Schema, seeds, storage layout | `{{BACKEND_MIGRATIONS_DIR}}/`, `data/` runtime artifacts | Touched by backend only; migrations structured for SQLite; no ad-hoc schema drift | Add/modify migrations under `migrations/`; runtime logs land in `data/logs/` |
| Shared contracts | Cross-stack types and schemas | `src/shared/` | Intended for dual Rust/TS types; TBD ({{ISSUE_PREFIX}}-1002): define actual shared types | Place shared DTOs/schemas here when ready; update both stacks to consume them |
| Tooling / scripts | Developer ergonomics, automation | `justfile`, `.GOV/scripts/` | Shell/CLI dependencies only; do not bake business logic here | Add repeatable tasks to `justfile`; helper scripts under `.GOV/scripts/` |

Note: Frontend and shell live under `{{FRONTEND_ROOT_DIR}}/` and `{{FRONTEND_SRC_DIR}}-tauri/` (per `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`). Backend crate lives under `{{BACKEND_CRATE_DIR}}/`.

Feature flags/toggles: If introducing flags, document the flag name and location here and in relevant modules.

## Raw / Derived / Display (RDD)
- Raw: SQLite-backed content is persisted by the backend (`{{BACKEND_MIGRATIONS_DIR}}/` and API handlers in `{{BACKEND_SRC_DIR}}/api/`).
- Derived: TBD ({{ISSUE_PREFIX}}-1003) - no concrete derived pipeline is implemented yet; track when indexing/embeddings land.
- Display: UI rendering in `{{FRONTEND_SRC_DIR}}/` (DocumentView/CanvasView) builds display state from backend responses; no persisted display layer yet.

````

###### Template File: `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
Intent: Debug runbook (first-5-minutes flow + CI triage map).
````md
# RUNBOOK_DEBUG

## First 5 minutes
> **WARNING for AI Agents:** Commands like `pnpm -C {{FRONTEND_ROOT_DIR}} tauri dev` or `just dev` start a long-running development server. They MUST NOT be executed with a blocking tool (like `run_shell_command`). These commands should be run in a separate, dedicated terminal by the user or as a true background process.
- Repro fast: `pnpm -C {{FRONTEND_ROOT_DIR}} tauri dev` (frontend + Tauri) and keep terminal output visible; note console errors.
- Check backend health while reproing: `cargo run --bin {{BACKEND_CRATE_NAME}}` (or rely on the Tauri spawn) and watch `data/logs/{{BACKEND_CRATE_NAME}}.log`.
- Confirm branch/spec alignment: skim `.GOV/spec/SPEC_CURRENT.md` for the exact feature expectation before changing code.
- Isolate layer: decide if the failure is UI, IPC, backend, or data; jump to the matching section below.
- Run the smallest relevant test: `pnpm -C {{FRONTEND_ROOT_DIR}} test <pattern>` for UI, `cargo test --manifest-path {{BACKEND_CARGO_TOML}}` for backend.

## Logs and verbosity
- Backend logs: `data/logs/{{BACKEND_CRATE_NAME}}.log` (JSON via `tracing_subscriber`). Set `HS_LOG_LEVEL=debug` to increase verbosity; default is `info`.
- Frontend/Tauri: stdout from `pnpm -C {{FRONTEND_ROOT_DIR}} tauri dev`; use browser devtools console for React logs.
- Historical investigation: `{{PROJECT_PREFIX}}_logger_*` in repo root and `log_archive/` capture prior runs/decisions.

## Common symptom -> where to look
| Symptom | Where to look | Search terms / commands |
| --- | --- | --- |
| UI not rendering / blank window | `{{FRONTEND_SRC_DIR}}/` components & routing | `rg "App" {{FRONTEND_SRC_DIR}}`, `pnpm -C {{FRONTEND_ROOT_DIR}} test` |
| Button/interaction does nothing | `{{FRONTEND_SRC_DIR}}/` handler, Tauri invoke wiring in `{{FRONTEND_SRC_DIR}}-tauri/src/lib.rs` | `rg "invoke" {{FRONTEND_SRC_DIR}} {{FRONTEND_SRC_DIR}}-tauri/src/lib.rs` |
| Backend API error / panic | `{{BACKEND_SRC_DIR}}/api/*.rs`, `models.rs`, `logging.rs` | `rg "Result<" {{BACKEND_SRC_DIR}}/api`, check `data/logs/{{BACKEND_CRATE_NAME}}.log` |
| IPC/bridge issues (frontend <-> backend) | Tauri orchestrator spawn in `{{FRONTEND_SRC_DIR}}-tauri/src/lib.rs`, backend entry `{{BACKEND_SRC_DIR}}/main.rs` | `rg "Command::new(\"cargo\")" {{FRONTEND_SRC_DIR}}-tauri/src/lib.rs`, `rg "@tauri" {{FRONTEND_SRC_DIR}}` |
| Data/migration problems | `{{BACKEND_MIGRATIONS_DIR}}/`, database path under `data/` | `rg "migration" {{BACKEND_CRATE_DIR}}`, inspect schema diffs |
| Build/test fails | `justfile`, package configs (`{{FRONTEND_ROOT_DIR}}/package.json`, `{{BACKEND_CARGO_TOML}}`) | Re-run `pnpm -C {{FRONTEND_ROOT_DIR}} test`, `cargo test --manifest-path {{BACKEND_CARGO_TOML}}` |

## If you only remember one thing
- Use `rg "<feature or error string>" {{FRONTEND_SRC_DIR}} {{BACKEND_CRATE_DIR}}` to jump to the owning layer, then open the matching file and cross-check the expected behavior in `.GOV/spec/SPEC_CURRENT.md`.
- When adding new repeatable errors, assign a code/tag like `{{ISSUE_PREFIX}}-####` and note it here with the primary entrypoint to triage.

## Debugging a failed CI check
- codex-check: run `just codex-check` and inspect outputs for forbidden `fetch(`, `println!/eprintln!`, or doc drift.
- depcruise: run `pnpm -C {{FRONTEND_ROOT_DIR}} run depcruise` to see layer violations.
- cargo-deny: run `cargo deny check advisories licenses bans sources` (install via `cargo install cargo-deny` if needed).
- gitleaks: rerun in CI or locally with `gitleaks detect --source .` if installed.
- todo-policy: `rg -n --pcre2 "TODO(?!\\({{ISSUE_PREFIX}}-\\d+\\))" {{FRONTEND_SRC_DIR}} {{BACKEND_ROOT_DIR}} scripts` to find non-tagged TODOs.

````

###### Template File: `.GOV/roles_shared/PAST_WORK_INDEX.md`
Intent: Archaeology pointers (prevents guesswork when context is missing).
````md
# PAST_WORK_INDEX

## Root-level specs and logs (canonical history)
- [{{MASTER_SPEC_FILENAME}}](../{{MASTER_SPEC_FILENAME}}) - current master spec (current).
- [{{CODEX_FILENAME}}](../{{CODEX_FILENAME}}) - current governance and operating rules.
- [{{PROJECT_PREFIX}}_logger_<date>.md](../{{PROJECT_PREFIX}}_logger_<date>.md) - latest logger; older loggers remain in root and `log_archive/`.

## .GOV/operator/docs_local/ (staging, non-canonical)
- (Populate as needed)

## log_archive/
- Stored historical loggers for archaeology and regressions.

````

###### Template File: `.GOV/roles_shared/docs/OWNERSHIP.md`
Intent: Path ownership routing map for review/triage.
````md
# OWNERSHIP (fill as features land)

Path ownership map for review and routing. Update when new areas appear.

| Path/Area | Owner(s) / Reviewers | Notes |
| --- | --- | --- |
| {{FRONTEND_ROOT_DIR}}/ (frontend) | Frontend Coder (AGENT_FRONTEND) | UI reviewer (layout per PROJECT_INVARIANTS) |
| {{FRONTEND_SRC_DIR}}-tauri/ (shell + orchestrator) | Tauri/IPC Coder (AGENT_SHELL) | Tauri bridge / orchestrator reviewer |
| {{BACKEND_CRATE_DIR}}/ (Rust backend) | Backend Coder (AGENT_BACKEND) | API/data/logging reviewer |
| src/shared/ (cross-stack types) | Shared Contracts (AGENT_SHARED) | Add reviewers when shared types exist |
| .GOV/ (governance workspace) | Docs Reviewer (AGENT_DOCS) | Navigation pack updates |
| CI / hygiene workflows | CI/Hygiene (AGENT_CI) | `just validate`/CI changes |

````

###### Template File: `.GOV/roles_shared/records/TASK_BOARD.md`
Intent: Machine-checkable shared work memory (WP lifecycle; STUB/IN_PROGRESS/DONE).
````md
# {{PROJECT_DISPLAY_NAME}} Project Task Board (Phase 1: EXHAUSTIVE STRATEGIC AUDIT)

## Spec Authority Rule [CX-598] (HARD INVARIANT)

**The Roadmap (Section 7.6) is ONLY a pointer. The Master Spec Main Body (Sections 1-6, 9-11) is the SOLE definition of "Done."**

| Principle | Enforcement |
|-----------|-------------|
| **Roadmap = Pointer** | Section 7.6.x items point to Main Body sections where requirements are defined |
| **Main Body = Truth** | Every MUST/SHOULD in Sections 1-6, 9-11 must be implemented - no exceptions |
| **No Debt** | Skipping requirements poisons the project; later phases inherit rotten foundations |
| **No Phase Closes** | Until ALL referenced Main Body requirements are VALIDATED |

**Why:** {{PROJECT_DISPLAY_NAME}} is complex software. Treating roadmap bullets as requirements (instead of pointers) leads to surface-level compliance, technical debt, and project failure.

This board provides an exhaustive tracking of all Roadmap items from A7.6.3. Phase 1 cannot close until every item below is validated against the current Master Spec (see `.GOV/spec/SPEC_CURRENT.md`).

**Task Board entry format (enforced for In Progress/Done/Superseded via `just task-board-check`):**
- In Progress: `- **[WP_ID]** - [IN_PROGRESS]`
- Done: `- **[WP_ID]** - [VALIDATED|FAIL|OUTDATED_ONLY]`
- Superseded: `- **[WP_ID]** - [SUPERSEDED]`
Keep details (failure reasons, commands, evidence, \"SUPERSEDED by ...\") in the task packet to avoid drift/noise.

**Backlog stubs (pre-activation):**
- Track not-yet-activated work as STUB items (no USER_SIGNATURE yet). Details live in `.GOV/task_packets/stubs/`.
- Stubs MUST be activated into official task packets before any coding starts (see `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- Base WP â†” packet revision mapping (v2/v3/v4) is tracked in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

## Active (Cross-Branch Status)

This section exists to keep the Operator's **main-branch** Task Board up to date when multiple Coders are working in separate WP branches/worktrees.

Rules:
- This section is informational for visibility across branches (who is working on what).
- Do NOT use `[IN_PROGRESS]` here (that token is reserved for the script-checked `## In Progress` list).
- Validator maintains this section on `main` via small docs-only "status sync" commits.

Entry format (recommended):
- `- **[WP_ID]** - [ACTIVE] - branch: feat/WP-{ID} - coder: <name/model> - last_sync: YYYY-MM-DD`

---


## Ready for Dev

A WP is only Ready for Dev if its Active Packet (per `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`) is an official packet under `.GOV/task_packets/` (not a stub).


## Stub Backlog (Not Activated)
- **[WP-1-Example-v1]** - [STUB]
- **[WP-1-Another-v1]** - [STUB]


## In Progress

Assignee/model is recorded in the task packet (CODER_MODEL, CODER_REASONING_STRENGTH). Task Board stays minimal.


## Done
- **[WP-1-Example-v1]** - [VALIDATED]
- **[WP-1-Another-v1]** - [VALIDATED]



## Blocked

---

## Superseded (Archive)
- **[WP-1-AppState-Refactoring]** - [SUPERSEDED]
- **[WP-1-AppState-Refactoring-v2]** - [SUPERSEDED]
- **[WP-1-Tokenization-Service-20251228]** - [SUPERSEDED]
- **[WP-1-Storage-Foundation-20251228]** - [SUPERSEDED]
- **[WP-1-Gate-Check-Tool]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles-v2]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles-v1]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles]** - [SUPERSEDED]
- **[WP-1-Diagnostic-Pipe]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-v2]** - [SUPERSEDED]
- **[WP-1-Workflow-Engine-v3]** - [SUPERSEDED]
- **[WP-1-Workflow-Engine-v2]** - [SUPERSEDED]
- **[WP-1-AI-Job-Model-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Validators]** - [SUPERSEDED]
- **[WP-1-ACE-Validators-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Validators-v3]** - [SUPERSEDED]
- **[WP-1-Dual-Backend-Tests]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-UI]** - [SUPERSEDED]
- **[WP-1-LLM-Core]** - [SUPERSEDED]
- **[WP-1-Security-Gates]** - [SUPERSEDED]
- **[WP-1-Security-Gates-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW-v2]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle-v2]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle]** - [SUPERSEDED]
- **[WP-1-Storage-Abstraction-Layer]** - [SUPERSEDED]
- **[WP-1-Storage-Abstraction-Layer-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Auditability]** - [SUPERSEDED]
- **[WP-1-ACE-Runtime]** - [SUPERSEDED]
- **[WP-1-AI-Job-Model-v3]** - [SUPERSEDED]
- **[WP-1-AI-UX-Actions]** - [SUPERSEDED]
- **[WP-1-AI-UX-Rewrite]** - [SUPERSEDED]
- **[WP-1-AI-UX-Summarize-Display]** - [SUPERSEDED]
- **[WP-1-Atelier-Lens]** - [SUPERSEDED]
- **[WP-1-Calendar-Lens]** - [SUPERSEDED]
- **[WP-1-Canvas-Typography]** - [SUPERSEDED]
- **[WP-1-Capability-SSoT]** - [SUPERSEDED]
- **[WP-1-Distillation]** - [SUPERSEDED]
- **[WP-1-Editor-Hardening]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-UI-v2]** - [SUPERSEDED]
- **[WP-1-Governance-Hooks]** - [SUPERSEDED]
- **[WP-1-MCP-End-to-End]** - [SUPERSEDED]
- **[WP-1-MCP-Skeleton-Gate]** - [SUPERSEDED]
- **[WP-1-Metrics-OTel]** - [SUPERSEDED]
- **[WP-1-Metrics-Traces]** - [SUPERSEDED]
- **[WP-1-MEX-Observability]** - [SUPERSEDED]
- **[WP-1-MEX-Safety-Gates]** - [SUPERSEDED]
- **[WP-1-MEX-UX-Bridges]** - [SUPERSEDED]
- **[WP-1-Migration-Framework]** - [SUPERSEDED]
- **[WP-1-Model-Profiles]** - [SUPERSEDED]
- **[WP-1-Mutation-Traceability]** - [SUPERSEDED]
- **[WP-1-OSS-Governance]** - [SUPERSEDED]
- **[WP-1-PDF-Pipeline]** - [SUPERSEDED]
- **[WP-1-Photo-Studio]** - [SUPERSEDED]
- **[WP-1-RAG-Iterative]** - [SUPERSEDED]
- **[WP-1-Semantic-Catalog]** - [SUPERSEDED]
- **[WP-1-Supply-Chain-MEX]** - [SUPERSEDED]
- **[WP-1-Workspace-Bundle]** - [SUPERSEDED]

````

###### Template File: `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
Intent: Single source of truth mapping Base WP -> Active Packet (keeps Master Spec WP-free).
````md
# Work Packet Traceability Registry (SSoT)

**Purpose**  
{{PROJECT_DISPLAY_NAME}} uses Work Packets (WPs) as execution units, but the Master Spec Main Body must remain stable and WP-free. This registry is the **single source of truth** for mapping:

- **Base WP IDs** (stable planning identifiers used by Roadmap/Task Board), to
- **Active Task Packet files** (the concrete packet to implement/validate), including any `-vN` revisions.

This avoids retroactively embedding WP IDs into the Master Spec and prevents drift when packets are revised (v2/v3/v4) after audits.

---

## Definitions

- **Base WP ID**: The stable identifier for a scope of work, formatted `WP-{phase}-{name}` (e.g., `WP-1-Workflow-Engine`).  
  - Base IDs **do not** include packet revision suffixes.
  - Base IDs are the preferred identifiers for Roadmap pointers and Task Board tracking.

- **Packet Revision**: A revised task packet for the same Base WP, named `WP-{phase}-{name}-v{N}` (e.g., `WP-1-Workflow-Engine-v4`).  
  - Naming rule is governed by {{PROJECT_DISPLAY_NAME}} Codex v{{CODEX_VERSION}} `[CX-580C]` (no date/time stamps; use `-vN`).
  - **Legacy exception:** historical packets may contain date stamps (e.g., `-20251228`). Do not create new date-stamped packet IDs; convert future revisions to `-vN`.

- **Active Packet**: The single packet file that is currently authoritative for implementation/validation of a Base WP.

- **Superseded Packet**: A prior packet revision that is no longer authoritative. Superseded packets are immutable history; do not â€œcatch them upâ€.

---

## Workflow (Deterministic)

1. **Roadmap points to Base WP IDs** (not packet revisions).  
2. **Task Board tracks WPs** (Base IDs and/or packet revisions). This registry resolves the Base WP â†’ Active Packet mapping when `-vN` revisions exist.
3. **Task packets live in** `.GOV/task_packets/`. **Stubs live in** `.GOV/task_packets/stubs/`.
4. If a packet must change due to audit/spec drift:
   - Create a **new packet revision** `...-v{N}` (do not edit locked history).
   - Mark the older packet as **Superseded** on `.GOV/roles_shared/records/TASK_BOARD.md`.
   - Update this registry to point the Base WP to the new Active Packet.

**Registry update is mandatory whenever more than one packet exists for the same Base WP.** If mapping is missing or ambiguous, the WP is governance-blocked until resolved.

### How to use with `just` / validation scripts (frictionless rule)

- When running `just pre-work`, `just post-work`, `just gate-check`, validator scripts, etc., use the **Active Packet WP_ID** (the filename stem), not the Base WP ID.
  - Example: if Active Packet is `.GOV/task_packets/WP-1-Workflow-Engine-v4.md`, run `just pre-work WP-1-Workflow-Engine-v4`.
- If the Active Packet is a stub under `.GOV/task_packets/stubs/`, it is **not executable**: activate it first (Technical Refinement Block â†’ USER_SIGNATURE â†’ create official task packet).

---

## Registry (Phase 1)

Format:
- **Base WP ID**: stable
- **Active Packet**: authoritative file path
- **Task Board**: where to find the status entry
- **Notes**: supersedes history / special cases

| Base WP ID | Active Packet | Task Board | Notes |
|-----------:|---------------|------------|-------|
| WP-1-Example | .GOV/task_packets/stubs/WP-1-Example-v1.md | Stub Backlog (Not Activated): WP-1-Example-v1 | stub |

````

###### Template File: `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
Intent: Central registry of consumed USER_SIGNATURE tokens (anti-replay / audit trail).
````md
# SIGNATURE_AUDIT

**Authoritative registry of all user signatures consumed for spec enrichment and work packet creation**

**Status:** ACTIVE
**Updated:** 2026-01-13
**Authority:** ORCHESTRATOR_PROTOCOL Part 2.5 [CX-585A/B/C]

---

## Signature Rules (MANDATORY)

- **Format:** `{username}{DDMMYYYYHHMM}` (e.g., `ilja251225032800`)
- **One-time use only:** Each signature consumed exactly ONCE in entire repo
- **External clock:** Timestamp from user-verified external source
- **Verification:** `grep -r "{signature}" .` must return only audit log entry
- **Blocks work:** Cannot create work packets without valid, unused signature
- **Purpose:** Prevents autonomous spec drift; ensures user intentionality

---

## Consumed Signatures

| Signature | Used By | Date/Time | Purpose | Master Spec Version | Notes |
|-----------|---------|-----------|---------|-------------------|-------|
| <signature> | Orchestrator | <YYYY-MM-DD HH:MM> | Spec update: vXX.XXX <short> | vXX.XXX | <notes> |

---

## How to Use This Log

### When Orchestrator Receives New User Signature:

1. **Verify format:** `{username}{DDMMYYYYHHMM}`
   - Example: `ilja251225032800` = username "ilja" + 25/12/2025 03:28:00

2. **Search repo for reuse:**
   ```bash
   grep -r "ilja251225032800" .
   ```
   - Should return ONLY lines you're about to add
   - If found elsewhere: REJECT, request new signature

3. **Record in this table:**
   - Add new row with signature, date/time, purpose, spec version, notes

4. **Reference in task packets:**
   ```markdown
   **Authority:** Master Spec v02.85, Strategic Pause approval [ilja251225032800]
   ```

5. **Update .GOV/spec/SPEC_CURRENT.md** to new version if enrichment occurred

---

## Signature History (For Reference)

### v02.50 â†’ v02.81
- Rogue assistant enriched spec (multiple iterations)
- No signatures recorded in this audit log (governance gap from early design)
- v02.81 represents first major enrichment cycle

### v02.81 â†’ v02.82 â†’ v02.83 â†’ v02.84
- Continued enrichment iterations
- Signatures likely used but not recorded here (audit log was created later)
- v02.84 is current baseline

### v02.84 â†’ v02.85+ (Forward)
- All future enrichments will be recorded in Consumed Signatures table above
- Each signature tracked, one-time use enforced
- Full provenance audit trail maintained

---

## Verification Commands

```bash
# Check if specific signature has been used
grep -r "ilja251225032800" .

# List all signatures in audit log
grep "^| " .GOV/roles_shared/records/SIGNATURE_AUDIT.md | grep -v "^| Signature"

# Verify no orphaned signatures in code/docs
grep -r "DDMMYYYYHHMM\|[a-z]*[0-9]\{12\}" . --include="*.md" | grep -v "SIGNATURE_AUDIT"

# Ensure all task packets reference a signature in SIGNATURE_AUDIT
grep -r "Strategic Pause approval \[" .GOV/task_packets/ | awk -F'[' '{print $NF}' | tr -d ']' | sort -u
```

---

**Last Updated:** 2025-12-25
**Version:** 1.0
**Maintained By:** Orchestrator Agent

````

###### Template File: `.GOV/roles_shared/docs/QUALITY_GATE.md`
Intent: Risk-tiered quality gate contract (pre/post-work expectations; validator posture).
````md
# QUALITY_GATE

Purpose: reduce coding errors by standard checks and clear risk tiers.

## Gate 0: Pre-Work Validation (AI Autonomy - Mandatory)

**[CX-620, CX-587]** Before any implementation work starts, Gate 0 MUST pass.

**For Orchestrator Agents:**
- Task packet MUST exist in `.GOV/task_packets/WP-{ID}.md`
- All task packet fields MUST be filled (no `{placeholders}`)
- Verification: `just pre-work WP-{ID}` MUST pass

**For Coder Agents:**
- Task packet MUST be verified before writing any code
- If no packet found, work MUST be BLOCKED immediately
- Bootstrap protocol MUST be followed (read START_HERE, SPEC_CURRENT, packet)
- BOOTSTRAP block MUST be output before first code change

**Enforcement:** Gate 0 is automated via validation scripts. Failure exits 1 and blocks work.

**Why:** For AI-autonomous operation, the workflow requires deterministic enforcement. Human users may not have coding expertise and rely on these gates to ensure correctness.

## Risk tiers
| Tier | Use when | Required checks | Review |
| --- | --- | --- | --- |
| LOW | Docs-only or comments; no behavior change | `just docs-check` (if docs touched) | Optional owner review |
| MEDIUM | Code change within one module; no schema/IPC changes | `just validate` (or record why not) | Owner review required |
| HIGH | Cross-module, IPC, migrations, auth/security, dependency updates, perf-critical | `just validate` + manual test steps | Two reviewers (owner + secondary) |

If uncertain, choose the higher tier.

## Task packet fields (required)
- RISK_TIER (LOW/MEDIUM/HIGH)
- TEST_PLAN (commands + manual steps, or "None" with reason)
- ROLLBACK_HINT (how to revert if needed)
- DONE_MEANS (what must be true to accept)

## Definition of done (minimum)
- Required commands run (or recorded why not).
- Any new error codes/tags documented in `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`.
- New flags/toggles documented in `.GOV/roles_shared/docs/ARCHITECTURE.md`.
- Targeted test added for logic changes, or explicit reason recorded.
- Manual validator review completed and recorded (status + evidence mapping); no automated review required.

`just validate` runs: `just docs-check`, `just codex-check`, `pnpm -C {{FRONTEND_ROOT_DIR}} run lint`, `pnpm -C {{FRONTEND_ROOT_DIR}} test`, `pnpm -C {{FRONTEND_ROOT_DIR}} run depcruise`, `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --manifest-path {{BACKEND_CARGO_TOML}}`, `cargo deny check advisories licenses bans sources`.

## Gate 1: Post-Work Validation (AI Autonomy - Mandatory)

**[CX-623, CX-651]** Before requesting commit, Gate 1 MUST pass.

**Required:**
- All TEST_PLAN commands MUST have been run
- Validation results MUST be documented in the task packet (logger only if explicitly requested)
- Git status MUST show changes (work actually done)
- For MEDIUM/HIGH: Manual validator review must be complete before marking Done
- Task packet MUST capture current status/result
- Verification: `just post-work WP-{ID}` MUST pass

**Enforcement:** Gate 1 is automated via validation scripts. Failure exits 1 and blocks commit.

**Full workflow validation:**
```bash
just validate-workflow WP-{ID}  # Runs pre-work, validate, post-work
```

## Self-review checklist (required)
1) Diff scan: every line is necessary for the task; no drive-by changes.
2) Placement: files and functions live in the correct module (see `.GOV/roles_shared/docs/ARCHITECTURE.md`).
3) Errors/observability: new repeatable errors have `{{ISSUE_PREFIX}}-####` tags and `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` updates.
4) Tests: at least one targeted test for logic changes (or a written reason).
5) Docs drift: update `.GOV/roles_shared/docs/START_HERE.md`, `.GOV/roles_shared/docs/ARCHITECTURE.md`, `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` if behavior/entrypoints changed.

Scaffolding: for new components or API endpoints, prefer `just new-react-component <Name>` or `just new-api-endpoint <name>` to keep structure consistent.
For MEDIUM/HIGH tasks adding new components or endpoints, scaffolds are required unless explicitly waived; record the reason in the task packet and run `just scaffold-check` before merge.

````

###### Template File: `.GOV/roles_shared/records/OSS_REGISTER.md`
Intent: Authoritative Open Source Software register (supply-chain manifest + license/integration posture).
````md
# OSS REGISTER

**Authoritative Open Source Software Manifest**
**Status:** ACTIVE
**Updated:** {{YYYY-MM-DD}} (initial template; update whenever dependencies change)

> Scope: Captures all dependencies and dev/build tools declared in backend manifests (e.g., `Cargo.toml` + lockfile) and frontend manifests (e.g., `package.json` + lockfile). Copyleft guard remains default-deny (GPL/AGPL only via `external_process`) unless explicitly approved and recorded.

## Backend Direct â€” `{{BACKEND_CARGO_TOML}}`

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| {{BACKEND_DEP_NAME}} | {{LICENSE}} | embedded_lib | Runtime | {{PURPOSE}} |

## Backend Transitive â€” (lockfile)

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| {{BACKEND_TRANSITIVE_DEP_NAME}} | {{LICENSE}} | embedded_lib | Runtime | {{PURPOSE}} |

## Frontend Direct â€” (manifest)

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| {{FRONTEND_DEP_NAME}} | {{LICENSE}} | embedded_lib | Runtime | {{PURPOSE}} |

## Frontend Transitive â€” (lockfile)

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| {{FRONTEND_TRANSITIVE_DEP_NAME}} | {{LICENSE}} | embedded_lib | Runtime | {{PURPOSE}} |

## Policy Notes (HARD)
- Every dependency MUST have: license, integration mode, scope, and a one-line purpose.
- Unknown/ambiguous licenses MUST be treated as BLOCKING until clarified.
- Copyleft (GPL/AGPL) MUST NOT be embedded; only permitted via explicit `external_process` boundaries with user approval and documented reasoning.
- Any dependency updates for MEDIUM/HIGH work MUST be called out in the task packet and validated by the Validator.

````

###### Template File: `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
Intent: Local worktree/branch policy for role-governed sessions (operator-machine specific).
````md
# ROLE_WORKTREES (Local Worktree Policy)

This document defines the local worktree/branch policy used on the Operator machine for role-governed sessions.

If you are an AI assistant operating in this repo:
- You MUST read this file during session start (Pre-Flight) for your assigned role.
- You MUST verify you are operating from the correct worktree directory and branch for your role before any repo changes.
- If the required worktree/branch does not exist, you MUST STOP and request the Orchestrator/Operator to create it (see "Creation commands").
- IMPORTANT: Creating worktrees/branches uses `git` operations that are blocked unless the user explicitly authorizes them in the same turn (Codex [CX-108]). If not authorized, STOP and request authorization.

## Role Worktrees (Operator machine)

| Role | Worktree directory | Branch |
| --- | --- | --- |
| OPERATOR (human) | `{{OPERATOR_WORKTREE_DIR}}` | `{{OPERATOR_BRANCH}}` |
| ORCHESTRATOR | `{{ORCHESTRATOR_WORKTREE_DIR}}` | `{{ORCHESTRATOR_BRANCH}}` |
| VALIDATOR | `{{VALIDATOR_WORKTREE_DIR}}` | `{{VALIDATOR_BRANCH}}` |
| CODER (agent) | WP-assigned worktree only (no default) | WP branch only (no default) |

Notes:
- CODER agents MUST work only in the WP-assigned worktree/branch created and recorded by the Orchestrator. They must not "pick" a worktree.
- WP assignment is recorded in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` as a `PREPARE` entry (via `just record-prepare ...`) with `branch` and `worktree_dir`.
- ORCHESTRATOR/VALIDATOR role work (governance/validation work outside a specific WP worktree) uses the dedicated role worktrees above.

## Verification Commands (run at session start)

- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git worktree list`

## Creation Commands (only if explicitly authorized in the same turn)

From the main repo working tree:

- Create ORCHESTRATOR worktree:
  - `git worktree add -b {{ORCHESTRATOR_BRANCH}} \"{{ORCHESTRATOR_WORKTREE_DIR}}\" {{DEFAULT_BASE_BRANCH}}`
- Create VALIDATOR worktree:
  - `git worktree add -b {{VALIDATOR_BRANCH}} \"{{VALIDATOR_WORKTREE_DIR}}\" {{DEFAULT_BASE_BRANCH}}`

WP worktrees (Orchestrator action, not Coder):
- Create a WP worktree/branch:
  - `just worktree-add WP-{ID}`
- Record the coder assignment (writes `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`):
  - `just record-prepare WP-{ID} {Coder-A|Coder-B} [branch] [worktree_dir]`

````

###### Template File: `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
Intent: Mechanical Orchestrator gate state model (initial empty state).
````json
{
  "gate_logs": []
}
````

###### Template File: `.GOV/validator_gates/{WP_ID}.json`
Intent: Mechanical Validator gate state model (per-WP; merge-safe).
````json
{
  "validation_sessions": {},
  "archived_sessions": []
}
````

###### Template File: `.GOV/roles/validator/VALIDATOR_GATES.json` (LEGACY ARCHIVE)
Intent: Legacy Mechanical Validator gate state archive (read-only; MUST NOT receive new sessions).
````json
{
  "validation_sessions": {},
  "archived_sessions": []
}
````

###### Template File: `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
Intent: Orchestrator role protocol (refinement loop, signature gate, delegation contract).
````md
# ORCHESTRATOR_PROTOCOL [CX-600-616]

**MANDATORY** - Lead Architect must read this to manage Phase progression and maintain governance invariants

## Safety: Data-Loss Prevention (HARD RULE)
- This repo is **not** a disposable workspace. Untracked files may be critical work (e.g., WPs/refinements).
- **Do not** run destructive commands that can delete/overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If a cleanup/reset is ever requested, first make it reversible: `git stash push -u -m "SAFETY: before <operation>"`, then show the user exactly what would be deleted (`git clean -nd`) and get explicit approval.

---

## Part 1: Strategic Priorities (Phase 1 Focus) [CX-600A]

### [PRIORITY_1] Storage Backend Portability [CX-DBP-001]
- Enforce the four pillars defined in Master Spec Â§2.3.13 and Trait Purity [CX-DBP-040]
- Block all database-touching work that bypasses the `Database` trait
- Goal: Make PostgreSQL migration a 1-week task (not 4-6 weeks)

### [PRIORITY_2] Spec-to-Code Alignment [CX-598]
- "Done" = 100% implementation of Main Body text, NOT just roadmap bullets
- Reject any Work Packet that treats the Main Body as optional
- Extract ALL MUST/SHOULD from spec section; map each to evidence (file:line)
- Enforce Roadmap Coverage Matrix completeness (Spec Â§7.6.1; Codex [CX-598A]) so Main Body sections cannot be silently omitted from planning

### [PRIORITY_3] Deterministic Enforcement [CX-585A/C]
- Spec-Version Lock: Master Spec immutable during phase execution
- Signature Gate: Zero implementation without technical refinement pause
- If spec change needed: run the Spec Enrichment workflow (new spec version file + update `.GOV/spec/SPEC_CURRENT.md`) under a one-time user signature and record it in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`. Do NOT edit locked task packets to "catch up" to the new spec; keep history immutable and create a NEW remediation WP only if new-spec deltas require new code changes.
- Historical completion policy: if Validator returns **OUTDATED_ONLY** (baseline-correct but spec evolved), keep the WP archived as Done/Validated history and create a NEW remediation WP only if current-spec deltas are actually needed. Do not churn the original WP back into Ready for Dev for drift-only.

### [PRIORITY_4] Phase 1 Closure Gate [CX-585D]
- Phase 1 only closes when ALL WPs in phase are VALIDATED (not just "done")
- All phase-blocking dependencies resolved
- Spec integrity check passed (run `just validator-spec-regression`)

### [PRIORITY_5] Task Packet as Single Source of Truth [CX-573B]
- Task packets contain SPEC_ANCHOR references (not orchestrator interpretation)
- Coder receives ONLY the task packet (no ad-hoc requests)
- Validator uses task packet for scope definition
- Lock packets with USER_SIGNATURE after creation; prevent edits

### [PRIORITY_6] Work Dependency Mapping [CX-573E]
- Identify blocking dependencies BEFORE work starts
- Block upstream WP work until blocker is VALIDATED
- Document dependency chain in TASK_BOARD

### [PRIORITY_7] Hardened Security Enforcement [CX-VAL-HARD]
- **Zero-Hollow implementation:** Reject any validator that only checks metadata; content-awareness is MANDATORY.
- **Strict Evidence Mapping:** Every security guard must cite the specific substring/offset that triggered the violation.
- **Deterministic Normalization:** All security scanning must occur on NFC-normalized, case-folded text to prevent bypasses.

### Risk Management Focus [CX-600B]
- **Anti-Vibe Guard:** Audit every Coder submission for placeholders, unwrap(), generic JSON blobs
- **Security Gates:** Prioritize WP-1-Security-Gates (MEX runtime integrity)
- **Supply Chain Safety:** Maintain OSS_REGISTER.md; block un-vetted dependencies
- **Instruction Creep Prevention:** Lock packets with USER_SIGNATURE; create NEW packets for changes
- **Spec Regression Guard:** Before phase closure run `just validator-spec-regression`
- **Waiver Audit Trail:** All waivers logged with approval date; expire at phase boundary

---

## Deterministic Manifest & Gate (current workflow, COR-701 discipline)
- Every task packet MUST keep the deterministic manifest template in `## Validation` (target_file, start/end, line_delta, pre/post SHA1, gates checklist). Packets must stay ASCII-only.
- Orchestrator ensures new packets are created from `.GOV/templates/TASK_PACKET_TEMPLATE.md` without stripping the manifest; reject packet creation/revision that removes it.
- `just pre-work WP-{ID}` must pass before handoff (template present), and `just post-work WP-{ID}` is the mandatory deterministic gate before Done/commit (enforces manifest completeness, SHA1s, window bounds, gates).

## Branching & Concurrency (preferred; low-friction)
- Default: one WP = one feature branch (e.g., `feat/WP-{ID}`).
- **Concurrency rule (MANDATORY when >1 Coder is active):** use `git worktree` per active WP (separate working directories) to prevent collisions and accidental loss of uncommitted work.
  - Orchestrator sets up worktrees and assigns each Coder a dedicated working directory.
  - Coders MUST NOT share a single working tree when working concurrently.
- Coders may commit freely on their WP branch. The Validator performs the final merge/commit to `main` after PASS (per Codex [CX-505]).

## Worktree + Branch Gate [CX-WT-001] (BLOCKING)

Orchestrator work MUST be performed from the correct worktree directory and branch.

Source of truth:
- `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (default role worktrees/branches)
- The assigned WP worktree/branch for the WP being orchestrated

Required verification (run at session start and whenever context is unclear):
- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git worktree list`

If the required worktree/branch does not exist:
- STOP and request explicit user authorization to create it (Codex [CX-108]).
- Only after authorization, create it using the commands in `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (role worktrees) or the repo's WP worktree helpers (WP worktrees).

Coder worktree rule:
- CODER agents must work only in WP-assigned worktrees/branches recorded via `just record-prepare` (writes `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`).

## Stop-Work Gate: Worktree + Assignment Before Packet Creation (HARD RULE)
- After a refinement is signed (`just record-signature WP-{ID} ...`), the Orchestrator MUST:
  1) Create the WP branch/worktree (`just worktree-add WP-{ID}`), and
  2) Record coder assignment (`just record-prepare WP-{ID} {Coder-A|Coder-B}`),
  before creating the task packet (`just create-task-packet WP-{ID}`).
- Rationale: prevents packet creation in an unassigned/shared working tree and forces a clean handoff to the correct work directory.

## Safety Commit Gate (HARD RULE; prevents untracked WP loss)
- Immediately after creating a WP task packet + refinement and obtaining `USER_SIGNATURE`, create a **checkpoint commit on the WP branch** that includes:
  - `.GOV/task_packets/WP-{ID}.md`
  - `.GOV/refinements/WP-{ID}.md`
- Rationale: untracked/uncommitted packets/refinements are vulnerable to accidental deletion (e.g., a mistaken cleanup). A checkpoint commit makes the WP recoverable deterministically.

## Part 2: Pre-Orchestration Checklist [CX-600]

**Complete ALL steps before creating task packets.**

### Step 1: Spec Currency Verification âœ‹ STOP
```bash
cat .GOV/spec/SPEC_CURRENT.md
just validator-spec-regression
```
- [ ] SPEC_CURRENT.md is current
- [ ] Points to latest Master Spec version
- [ ] Regression check returns PASS

### Step 2: Task Board Review âœ‹ STOP
- [ ] TASK_BOARD.md is current
- [ ] No stalled WPs (>2 weeks idle)
- [ ] All "Done" WPs show VALIDATED status (Validator approved them)
- [ ] Blocked WPs have documented reason + ETA for unblocking

**CLARIFICATION:** Orchestrator's role is to:
1. **CHECK** that the Operator-visible TASK_BOARD on `main` correctly reflects packet status (is it in sync?)
2. **UPDATE** TASK_BOARD planning states (Ready for Dev/Blocked/Stub Backlog) and supersedence; Validator status-syncs `main` for In Progress/Done
3. **RECORD** governance actions (signature usage, spec pointer updates, mapping decisions) â€” Orchestrator does NOT issue validation verdicts

Orchestrator does NOT do validation (Validator does). Orchestrator just tracks status.

### Step 3: Supply Chain Audit âœ‹ STOP
```bash
cargo deny check && npm audit
```
- [ ] OSS_REGISTER.md exists and is complete
- [ ] `cargo deny check` returns 0 violations
- [ ] `npm audit` returns 0 critical/high vulnerabilities

### Step 4: Phase Status âœ‹ STOP
- [ ] Current phase identified
- [ ] Phase-critical WPs identified
- [ ] Dependencies documented in TASK_BOARD

### Step 5: Governance Files Current âœ‹ STOP
- [ ] ORCHESTRATOR_PROTOCOL.md is current
- [ ] CODER_PROTOCOL.md is current
- [ ] VALIDATOR_PROTOCOL.md is current
- [ ] Master Spec is current

---

## Part 2.5: Strategic Pause & Signature Gate [CX-585A/B/C]

**BLOCKING GATE: Every task packet creation requires spec enrichment approval**

This gate prevents autonomous spec drift and ensures user intentionality at each work cycle.

### Part 2.5.1 Trigger: When to Pause (Decision Tree)

**CLARIFICATION: Enrichment vs. Transcription**

Orchestrator MUST NOT enrich speculatively. Instead, use this decision tree:

#### Definition: "Clearly Covers" (Objective 5-Point Checklist)

A requirement "clearly covers" (passes Main Body criteria) when it satisfies ALL 5 points:

1. âœ… **Appears in Main Body** â€” Not in Roadmap, not aspirational, not "Phase 2+"
2. âœ… **Explicitly Named** â€” Reader immediately finds it without inference (section number, title, explicit text)
3. âœ… **Specific** â€” Not "storage SHOULD be portable" but "storage API MUST implement X trait with Y methods"
4. âœ… **Measurable Acceptance Criteria** â€” Clear yes/no test (e.g., "trait has 6 required async methods")
5. âœ… **No Ambiguity** â€” Single valid interpretation; no multiple ways to read it

**Result:**
- **PASS (all 5 âœ…)** â†’ Requirement clearly covered. Proceed to task packet creation (no enrichment needed).
- **FAIL (any âŒ)** â†’ Requirement NOT clearly covered. Ask user for clarification OR enrich spec (with user signature).

**Examples:**

CLEARLY COVERS âœ…:
```
Â§2.3.13.1: Database trait MUST have these 6 async methods:
- async fn get_blocks(&self, id: &str) -> Result<Vec<Block>>
- async fn save_blocks(&self, blocks: Vec<Block>) -> Result<()>
- ...etc (all 5 criteria met; unambiguous)
```
â†’ Proceed without enrichment

DOES NOT CLEARLY COVER âŒ:
```
Â§2.3.13: Storage abstraction SHOULD be portable
```
â†’ Criteria 3 fails (not specific); criteria 4 fails (no acceptance criteria)
â†’ Requires user clarification OR enrichment (with signature)

---

**Decision Tree:**

```
Does Master Spec Main Body clearly cover this requirement?
â”œâ”€ YES (all 5 criteria met)
â”‚  â””â”€ Proceed to task packet creation (no enrichment needed)
â”‚
â”œâ”€ NO, but it's in Roadmap
â”‚  â””â”€ Promote roadmap item to Main Body + enrich spec
â”‚     (This is NECESSARY enrichment, user-intended)
â”‚
â”œâ”€ NO, and it's NEW or UNCLEAR
â”‚  â””â”€ ASK USER for clarification BEFORE enriching
â”‚     (Enrichment requires user signature; don't guess)
â”‚
â””â”€ CONFLICTING signals (spec says one thing, user implies another)
   â””â”€ ESCALATE to user; get explicit decision before proceeding
      (Don't interpret; let user clarify intent)
```

**When Enrichment is REQUIRED (after user clarification):**
1. User request clearly implies requirement not yet in Main Body
2. Roadmap item needs promotion to Main Body for clarity
3. Phase gate reveals missing acceptance criteria
4. User explicitly requests spec clarification (with signature)

**When Enrichment is FORBIDDEN (DO NOT enrich speculatively):**
- Spec seems incomplete but user hasn't asked for enrichment
- You're guessing what the requirement "should be"
- Timeline pressure (don't enrich to save schedule)
- Enrichment would require major spec redesign (escalate instead)

**Rule: Zero speculative enrichment. Enrichment requires user signature (approval).**

### Part 2.5.2 Enrichment Workflow âœ‹ BLOCKING

**Step 1: Identify gaps in Master Spec Main Body**
Orchestrator MUST perform a "Technical Refinement Audit" and present the results to the user.

**Step 1.1: The Technical Refinement Block (MANDATORY)**
Before requesting a USER_SIGNATURE, the Orchestrator MUST output a block containing:
- **Gaps Identified:** Specific sections/logic missing in the current Master Spec.
- **Interaction with flight recorder: Specific event IDs and telemetry triggers:** Specific event IDs, telemetry triggers, and log data structures.
- **red team advisory: Architectural risks and security failure modes:** Specific architectural risks and security failure modes.
- **proposed Spec Enrichment: The FULL, VERBATIM normative text to be added to the Master Spec:**
    - **CRITICAL:** Summaries are FORBIDDEN.
    - **CRITICAL:** You MUST output the exact Markdown text (headings, rules, code blocks) that will be inserted.
    - **CRITICAL:** The user must be able to copy-paste this text directly into the Master Spec if they chose to do so manually.
- **primitives:** Specific Traits, Structs, or Enums that must be implemented.

**Non-negotiable presentation rule:** The Technical Refinement Block MUST be pasted into the Orchestrator's chat message for user review (not only written to a file). The Orchestrator MUST NOT proceed to signature or packet creation until the user explicitly approves the refinement in-chat (e.g., `APPROVE REFINEMENT {WP_ID}`) or requests edits.

**Deterministic approval evidence (repo-enforced):**
- Before consuming a one-time signature, the refinement file MUST contain: - USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT {WP_ID} (exact match). This prevents signature-by-momentum and makes the approval step mechanically checkable.


**Hard enforcement rule (procedure; repo-enforced):**
- If the refinement concludes **ENRICHMENT_NEEDED=YES** (or otherwise identifies unresolved ambiguity requiring new normative text), the Orchestrator MUST STOP. Do NOT record a WP packet signature and do NOT create/lock a task packet. Complete Spec Enrichment first (new spec version + update `.GOV/spec/SPEC_CURRENT.md`), then create a NEW WP variant anchored to the updated spec with a fresh one-time signature.

**Step 2: Enrich Master Spec (after user approval)**
If gaps found:
1. Locate: Current Master Spec version (e.g., v02.91)
2. Create: NEW version file (e.g., v02.92.md)
3. Copy: Entire current spec
4. Add: Required sections/clarifications (using the Proposed Spec Enrichment text)
5. Add: CHANGELOG entry with reason for update
6. Update: .GOV/spec/SPEC_CURRENT.md to point to new version

**Step 3: Update all workflow files to reference new spec**

```
Orchestrator MUST update these files to point to new spec version:
- .GOV/roles/coder/CODER_PROTOCOL.md: Update spec version references
- .GOV/roles/validator/VALIDATOR_PROTOCOL.md: Update spec version references
- .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md: Update spec version references
- .GOV/roles_shared/docs/START_HERE.md: Update spec version references
- .GOV/roles_shared/docs/ARCHITECTURE.md: Update spec anchors if changed
- .GOV/spec/SPEC_CURRENT.md: Point to the new spec (authoritative)

Do NOT mass-edit historical/signed task packets to "catch up" to new governance/spec. Signed packets are immutable; create new variants/remediation WPs instead.
```

**Verification:**
```bash
# Check all protocol files reference latest spec version
grep -r "Master Spec v02" .GOV/roles_shared/*.md .GOV/roles/*/*.md .GOV/task_packets/*.md
# Should all show v02.85 (or latest), no orphaned older versions in active files
```

**Rule:** Requesting a USER_SIGNATURE without first presenting the Technical Refinement Block is a **CRITICAL PROTOCOL VIOLATION**.

### Part 2.5.3 Signature Gate (One-Time Use) âœ‹ BLOCKING

**Orchestrator MUST request USER_SIGNATURE before creating work packets.**

#### Work Packet Stubs (Backlog) [CX-585C]

A **Work Packet Stub** is an optional planning artifact used to track Roadmap/Main Body work before activation.

- Stubs are legitimate backlog items, but they are NOT executable task packets/work packets.
- Stubs MUST live in `.GOV/task_packets/stubs/` and should be listed on `.GOV/roles_shared/records/TASK_BOARD.md` under a STUB section.
- If a Base WP has multiple packets (or a stub + official packet), the Base WP â†’ Active Packet mapping MUST be recorded in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.
- Stubs MUST NOT be handed off to Coder/Validator and MUST NOT be used to start implementation.
- Stubs do not require USER_SIGNATURE, a refinement file, or deterministic gates.
- Stub template: `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`

Activation rule (mandatory): Before any coding starts, activate the stub by following the normal workflow (in-chat Technical Refinement Block -> USER_SIGNATURE -> `.GOV/refinements/WP-*.md` -> `just create-task-packet WP-*` -> move TASK_BOARD entry out of STUB).

**Signature format:** `{username}{DDMMYYYYHHMM}`

Example: `ilja251225032800` (ilja + 25/12/2025 03:28:00)

**Signature rules (MANDATORY):**

1. **One-time use only** â€” Each signature can be used exactly ONCE in entire repo
2. **External clock source** â€” User must provide timestamp from external/verified source
3. **Prevents reuse** â€” Grep repo to verify signature never appears before
4. **Audit trail** â€” Record in SIGNATURE_AUDIT.md when signature is consumed
5. **Blocks work** â€” Cannot create work packets without valid, unused signature

**Orchestrator verification (BEFORE creating work packets):**

```bash
# Check if signature has been used before
grep -r "ilja251225032800" .

# Should return ONLY the lines you're about to add (audit log + work packet reference)
# If it appears elsewhere, REJECT and request NEW signature
```

**If signature found elsewhere:**
```
âŒ BLOCKED: Signature already used [CX-585B]

Signature: ilja251225032800
First use: {file and date when first used}
Current request: New task packet creation

Each signature can only be used once. Request new signature from user.
```

### Part 2.5.4 Signature Audit Log [CX-585B]

**Orchestrator MUST maintain `.GOV/roles_shared/records/SIGNATURE_AUDIT.md` as central registry.**

```markdown
# SIGNATURE_AUDIT.md

Record of all user signatures consumed for spec enrichment and work packet creation.

---

## Consumed Signatures

| Signature | Used By | Date | Purpose | Master Spec Version | Notes |
|-----------|---------|------|---------|-------------------|-------|
| ilja251225032800 | Orchestrator | 2025-12-25 03:28 | Strategic Pause: Spec enrichment for Phase 1 storage foundation | v02.85 | Enriched spec with Storage Backend Portability requirements |
| ilja251225041500 | Orchestrator | 2025-12-25 04:15 | Task packet creation: WP-1-Storage-Abstraction-Layer | v02.85 | Spec already enriched by ilja251225032800 |

---

## How to Use This Log

1. When Orchestrator receives new user signature
2. Verify signature format: `{username}{DDMMYYYYHHMM}`
3. Search repo: `grep -r "{signature}" .`
4. If found anywhere except this file: REJECT (already used)
5. If not found: Proceed with enrichment/work packet creation
6. Add row to Consumed Signatures table
7. Include signature in relevant docs as reference: `[Approved: ilja251225032800]`
```

### Part 2.5.5 Workflow Integration

**Complete flow before task packet creation:**

```
Pre-Orchestration Checklist (Part 2, Steps 1-5) âœ… PASS
    â†“
ðŸš§ STRATEGIC PAUSE & SIGNATURE GATE (Part 2.5)
    â†“
1. Identify spec gaps (Master Spec Main Body coverage)
    â†“
2. Enrich spec if needed (version bump, update all protocol files)
    â†“
3. Request USER_SIGNATURE from user
    â†“
User provides: ilja251225032800 (name + DDMMYYYYHHMM)
    â†“
4. Verify signature is unused (grep repo)
    â†“
5. Record signature in SIGNATURE_AUDIT.md
    â†“
6. Reference signature in work packet metadata
    â†“
âœ… GATE UNLOCKED: Proceed to Task Packet Creation (Part 4)
    â†“
Create work packets aligned with enriched, user-approved spec
```

**Example in task packet metadata:**
```markdown
# Task Packet: WP-1-Storage-Abstraction-Layer

**Authority:** Master Spec v02.85, Strategic Pause approval [ilja251225032800]
```

### Part 2.5.6 Non-Negotiables for Signature Gate [CX-585C]

**âŒ DO NOT:**
1. Create work packets without spec enrichment
2. Use signature twice
3. Skip signature verification (grep check)
4. Proceed without user signature
5. Forge signature from internal clock
6. Update spec without bumping version
7. Forget to update protocol files when spec changes
8. Leave signature audit log blank

**âœ… DO:**
1. Always enrich Master Spec before task packets
2. Verify each signature is one-time use only
3. Run grep check to confirm signature is unused
4. Update ALL protocol files (CODER, VALIDATOR, ORCHESTRATOR)
5. Record signature in SIGNATURE_AUDIT.md
6. Document Master Spec version in task packets
7. Include signature reference in work packet authority
8. Keep audit trail complete for all enrichments

### Part 2.5.7 Automated Gate Enforcement (Orchestrator Gates)

To physically prevent the merging of Refinement, Signature, and Creation phases, the Orchestrator MUST use the code-enforced turn lock:

1. **Record Refinement:** Immediately after presenting a Technical Refinement Block, the Orchestrator MUST run `just record-refinement {wp-id}`.
2. **Mandatory Turn Boundary:** The Orchestrator MUST STOP and wait for a NEW turn.
3. **Record Signature:** Only in a new turn can the Orchestrator run `just record-signature {wp-id} {signature}`.
4. **Hard Block:** The `.GOV/scripts/validation/orchestrator_gates.mjs` script will return an error if Step 1 and Step 3 occur in the same turn. This error is a **Hard Stop**; the Orchestrator must not attempt to bypass it via manual file writes.

### 2.6 Work Packet Lifecycle

---

## Part 3: Role & Critical Rules

You are an **Orchestrator** (Lead Architect / Engineering Manager). Your job is to:
1. Translate Master Spec requirements into concrete task packets
2. Manage phase progression (gate closure on VALIDATED work, not estimates)
3. Prevent instruction creep and maintain spec integrity
4. Coordinate between Coder and Validator
5. Escalate blockers and manage risk

**CRITICAL RULES:**
1. **NO CODING:** You MUST NOT write code in `{{BACKEND_ROOT_DIR}}/`, `{{FRONTEND_ROOT_DIR}}/`, `tests/`, or `.GOV/scripts/` (except task packets in `.GOV/task_packets/`).
2. **TRANSCRIPTION NOT INVENTION:** Task packets point to SPEC_ANCHOR; they do not interpret or invent requirements.
3. **SPEC_ANCHOR REQUIRED:** Every WP MUST reference a requirement in Master Spec Main Body (not Roadmap).
4. **LOCK PACKETS:** Use USER_SIGNATURE to prevent post-creation edits; create NEW packets for changes (WP-{ID}-variant).
5. **PHASE GATES MANDATORY:** Phase only closes if ALL WPs are VALIDATED (not just "done").
6. **DEPENDENCY ENFORCEMENT:** Block upstream work until blockers are VALIDATED.

---

## Part 3-Error-Recovery: How to Recover from Orchestrator Mistakes [CX-611]

**Governance violations happen. This section shows how to recover.**

### Error 1: Signature Used Twice (Typo/Mistake)

**Problem:** You used the same signature twice in the repo.

**Prevention:** Always grep before using:
```bash
grep -r "{signature}" .
# Should return ZERO results (except audit log entry you're about to add)
```

**Recovery if error occurs:**
1. Mark signature INVALID in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
   ```markdown
   | ilja251225032800 | Orchestrator | 2025-12-25 03:28 | (INVALID - used twice by mistake) | v02.85 | Signature rejected; same timestamp used multiple times |
   ```

2. Request NEW signature from user (different timestamp)
   ```
   âŒ Signature already consumed [CX-611-A]

   Signature: ilja251225032800
   First use: {file and line when first used}

   Please provide a NEW signature with a different timestamp.
   Format: {username}{DDMMYYYYHHMM}
   ```

3. Update task packets to reference new signature
4. Document in WP NOTES: "Original signature invalid (used twice); replaced with ilja251225032801"

---

### Error 2: Wrong SPEC_ANCHOR in Locked Packet

**Problem:** Packet is locked but SPEC_ANCHOR points to wrong requirement.

**Prevention:** Verify SPEC_ANCHOR exists in Master Spec BEFORE locking:
```bash
grep -n "Â§X\.X\.X" .GOV/spec/SPEC_CURRENT.md
# Should return non-zero (section exists)
```

**Recovery if error occurs:**

**Step 1: Check severity**
- **CRITICAL (wrong scope):** SPEC_ANCHOR refers to totally different requirement
  â†’ Create variant packet (WP-{ID}-v2)

- **MINOR (wrong section, same scope):** SPEC_ANCHOR points to same requirement in wrong subsection
  â†’ Add ERRATA section (read-only)

**Step 2: If CRITICAL â€” Create variant:**
```markdown
# Task Packet: WP-1-Storage-Abstraction-Layer-v2

## Authority
- **SPEC_ANCHOR**: Â§2.3.13.3 (CORRECTED)
- **Note**: Original WP-1-Storage-Abstraction-Layer used wrong SPEC_ANCHOR (Â§2.3.10); superseded by this version

(Copy rest of original packet, update SPEC_ANCHOR only)

---

**User Signature Locked:** ilja251225041502 (new signature for corrected packet)
```

Update TASK_BOARD to reference v2 (remove original from active list, mark superseded).

**Step 3: If MINOR â€” Add ERRATA:**
```markdown
## ERRATA

- **Original SPEC_ANCHOR:** Â§2.3.13 (too broad)
- **Correct SPEC_ANCHOR:** Â§2.3.13.3 (specific subsection)
- **Reason:** Typo in section reference; scope unchanged
- **Date corrected:** 2025-12-25
- **Action:** No variant needed; correct the section reference mentally
```

Mark packet with ERRATA note but keep it active (no v2 needed).

---

### Error 3: TASK_BOARD Out of Sync with Packets

**Problem:** Operator-visible TASK_BOARD (on `main`) shows an incorrect state vs. the task packet `**Status:**` field (common in multi-branch worktrees).

**Prevention:** Use docs-only status-sync commits:
- Coder produces a docs-only bootstrap claim commit when starting (task packet set to `In Progress` with claim fields).
- Validator mirrors that to `main` by updating `.GOV/roles_shared/records/TASK_BOARD.md` -> `## Active (Cross-Branch Status)` (and later moves items on PASS/FAIL).

**Recovery if error occurs:**
1. Compare TASK_BOARD status vs. each WP's STATUS field
   ```bash
   grep "^- STATUS:" .GOV/task_packets/WP-*.md | sort
   # Compare with .GOV/roles_shared/records/TASK_BOARD.md sections
   ```

2. Identify discrepancies
3. Update `main` TASK_BOARD to match packet reality (task packets are source of truth)
4. Log in decision log (optional): "Status-sync: TASK_BOARD was {X days} out of sync"
5. Review: Why did sync break? What to do differently?

---

### Error 4: Blocker Status Missed in Step 1

**Problem:** You created WP without checking if its blocker was VALIDATED.

**Prevention:** In Part 4 Step 1, always check blocker status:
```bash
grep -A3 "BLOCKER" .GOV/task_packets/WP-{upstream-id}.md
# Should show: STATUS: Done, verdict: VALIDATED
```

**Recovery if error occurs:**
1. Immediately mark new WP as BLOCKED in TASK_BOARD
2. Document: "Discovered blocker after creation; should have been caught in Step 1"
3. Add to WP NOTES: "Blocker: WP-X (Status: {current status})"
4. Review: Why was blocker missed? Improve your Step 1 checklist.

---

### Error 5: Enrichment Without User Signature

**Problem:** You enriched spec but didn't get user signature beforehand.

**Prevention:** Request signature BEFORE enriching spec (Part 2.5.3).

**Recovery if error occurs:**
1. Retroactively request user signature for enrichment
   ```
   âš ï¸ Signature required (retroactive) [CX-611-B]

   I enriched Master Spec v02.84 â†’ v02.85 with Storage Backend Portability requirements.

   To complete governance, please provide user signature:
   Format: {username}{DDMMYYYYHHMM}
   ```

2. Add to SIGNATURE_AUDIT.md once user provides signature:
   ```markdown
   | ilja251225050000 | Orchestrator | 2025-12-25 05:00 | (RETROACTIVE) Strategic Pause: Spec enrichment for Phase 1 storage | v02.85 | Retroactive approval for enrichment done at 2025-12-25 03:28 |
   ```

3. Update task packets to reference signature
4. Note: "This is debt. Avoid in future by requesting signature BEFORE enriching spec."

---

### Error 6: Missing Signature in SIGNATURE_AUDIT.md

**Problem:** You recorded a signature somewhere (WP, protocol, etc.) but forgot to add it to SIGNATURE_AUDIT.md.

**Prevention:** Record EVERY signature immediately upon use in SIGNATURE_AUDIT.md.

**Recovery if error occurs:**
1. Find the orphaned signature in codebase:
   ```bash
   grep -r "ilja251225041500" .GOV/ *_Master_Spec_v*.md
   # Shows where it was used
   ```

2. Add missing entry to SIGNATURE_AUDIT.md with metadata from actual usage
3. Verify signature format is correct: `{username}{DDMMYYYYHHMM}`
4. Note: "Added retroactively; ensure all future signatures recorded immediately"

---

---

## Part 3.5: What Orchestrator MUST Provide to Coder [CX-608]

**BLOCKING REQUIREMENT: Task packets are contracts between Orchestrator and Coder. Every field is mandatory.**

The CODER_PROTOCOL [CX-620-623] defines 11 steps that Coder MUST follow. This section specifies what **Orchestrator MUST provide** to enable Coder's execution. If any field is incomplete, Coder will BLOCK at Step 2 and return the packet for completion.

### Overview: 10 Required Task Packet Fields

Every task packet MUST include all 10 fields in this exact structure:

| Field | Purpose | Completeness Criteria |
|-------|---------|----------------------|
| **TASK_ID + WP_ID** | Unique identifier for tracking | Format: `WP-{phase}-{short-name}` (e.g., `WP-1-Storage-DAL`) |
| **STATUS** | Coder knows when to start | MUST be `Ready-for-Dev` or `In-Progress` (not TBD/Draft) |
| **RISK_TIER** | Determines validation rigor | MUST be `LOW`, `MEDIUM`, or `HIGH` (with clear justification) |
| **SCOPE** | Coder knows what to change | 1-2 sentence description + rationale (Business/technical WHY) |
| **IN_SCOPE_PATHS** | Coder knows which files to modify | EXACT file paths or directories (5-20 entries); no vague patterns like "backend" |
| **OUT_OF_SCOPE** | Coder knows what NOT to change | Explicit list of deferred work, related tasks, refactoring NOT included |
| **TEST_PLAN** | Coder knows how to validate | EXACT bash commands (cargo test, pnpm test, etc.); no placeholders |
| **DONE_MEANS** | Coder knows success criteria | Concrete checklist (3-8 items); 1:1 mapped to SPEC_ANCHOR; no "works well" vagueness |
| **HARDENED_INVARIANTS** | Security-critical requirements | Mandatory for RISK_TIER: HIGH. Includes: Content-Awareness, NFC Normalization, Atomic Poisoning. |
| **ROLLBACK_HINT** | Coder knows how to undo | `git revert {commit}` OR explicit undo steps (if multi-step changes) |
| **BOOTSTRAP** | Coder knows where to start | 4 sub-fields (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP) |

**Coder will verify all 10 fields exist in Step 2 of CODER_PROTOCOL. Missing = BLOCK.**

---

### Field 1: TASK_ID & WP_ID (Unique Identifier) [CX-600]

**What Coder expects:**
- Unique identifier format: `WP-{phase}-{name}`
- Example: `WP-1-Storage-Abstraction-Layer`
- Used for: Task board tracking, commit messages, validation logs

**What "complete" means:**
- âœ… ID is unique (no duplicates in .GOV/task_packets/)
- âœ… Format matches pattern `WP-{1-9}-{descriptive-name}`
- âœ… Name reflects actual work (not generic like "Feature-A")

**Example:**
```markdown
## Metadata
- TASK_ID: WP-1-Storage-Abstraction-Layer
- WP_ID: WP-1-Storage-Abstraction-Layer
```

---

### Field 2: STATUS (Work State) [CX-601]

**What Coder expects:**
- Coder will BLOCK if status is not clearly "Ready-for-Dev" or "In-Progress"
- If status is TBD/Draft/Pending, Coder cannot start

**What "complete" means:**
- âœ… STATUS is `Ready-for-Dev` (packet complete, awaiting assignment)
- âœ… OR STATUS is `In-Progress` (actively assigned)
- âœ… NOT: Draft, TBD, Pending, Waiting, Proposed

**Example:**
```markdown
## Metadata
- STATUS: Ready-for-Dev
```

**Why it matters:**
- Coder uses this as the GO/NO-GO signal
- If status is Draft, Coder interprets as incomplete packet

---

### Field 3: RISK_TIER (Validation Rigor) [CX-602]

**What Coder expects:**
- Clear tier that determines validation scope
- LOW = Docs-only, no behavior change
- MEDIUM = Code change, one module, no migrations
- HIGH = Cross-module, migrations, IPC, security

**What "complete" means:**
- âœ… RISK_TIER is LOW, MEDIUM, or HIGH
- âœ… Justification provided (why this tier, not lower)
- ? Matches TEST_PLAN complexity; note manual review requirement for MEDIUM/HIGH in DONE_MEANS or NOTES

**Example:**
```markdown
## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Cross-module refactor (AppState, jobs, workflows); includes migration
  - Requires: cargo test + pnpm test; manual review required
```

**Why it matters:**
- LOW tier: Manual review optional
- MEDIUM tier: Manual review required
- HIGH tier: Manual review required (blocker if issues remain)

---

### Field 4: SCOPE (What to Change) [CX-603]

**What Coder expects:**
- Clear, unambiguous description of the work
- Business rationale (WHY this change?)
- No ambiguity about boundaries

**What "complete" means:**
- âœ… One-sentence summary: "Add {feature/fix/refactor}"
- âœ… Business/technical rationale: "Because {reason}"
- âœ… Boundary clarity: "This does NOT include {related work}"

**Examples:**

âŒ **Incomplete SCOPE:**
```markdown
SCOPE: Improve job handling
```

âœ… **Complete SCOPE:**
```markdown
## Scope
- **What**: Add `/jobs/:id/cancel` endpoint to allow users to stop running jobs
- **Why**: Users currently have no way to cancel jobs; reduces support load for stuck workflows
- **Boundary**: This does NOT include retry logic (separate task), UI changes (separate task), or job timeout enforcement (Phase 2)
```

**Why it matters:**
- Coder uses SCOPE to decide what's "done"
- Ambiguous scope = scope creep (Coder implements too much or too little)

---

### Field 5: IN_SCOPE_PATHS (Exact File Boundaries) [CX-604]

**What Coder expects:**
- EXACT file paths Coder is allowed to modify
- No vague patterns ("backend", "api", "feature-X")
- 5-20 entries (not 100+)

**What "complete" means:**
- âœ… Specific file paths (not directories alone): `/{{BACKEND_SRC_DIR}}/api/jobs.rs`
- âœ… OR specific directory paths (if entire directory): `/{{BACKEND_MIGRATIONS_DIR}}/`
- âœ… 5-20 entries (if >20, likely scope creep; split into multiple WPs)
- âœ… Paths relative to repo root
- âœ… Every path in this list is justified by SCOPE

âŒ **Incomplete IN_SCOPE_PATHS:**
```markdown
IN_SCOPE_PATHS:
- {{BACKEND_ROOT_DIR}}/
- {{FRONTEND_ROOT_DIR}}/
```

âœ… **Complete IN_SCOPE_PATHS:**
```markdown
## Scope
- **IN_SCOPE_PATHS**:
  * {{BACKEND_SRC_DIR}}/api/jobs.rs (add cancel endpoint)
  * {{BACKEND_SRC_DIR}}/jobs.rs (update status enum)
  * {{BACKEND_SRC_DIR}}/workflows.rs (stop workflow on cancel)
  * {{BACKEND_MIGRATIONS_DIR}}/0003_job_status.sql (new status value)
  * {{BACKEND_TESTS_DIR}}/job_cancel_tests.rs (new tests)
```

**Why it matters:**
- Coder will ONLY modify these files
- Validator will flag changes outside IN_SCOPE_PATHS as scope creep
- Prevents "drive-by" refactoring of unrelated code

---

### Field 6: OUT_OF_SCOPE (What NOT to Change) [CX-604B]

**What Coder expects:**
- Explicit list of what Coder should NOT touch
- Deferred work, related tasks, refactoring NOT included

**What "complete" means:**
- âœ… List 3-8 items that sound related but are OUT_OF_SCOPE
- âœ… Each item has brief reason ("separate task", "Phase 2", "high risk")
- âœ… Protects against scope creep

âŒ **Incomplete OUT_OF_SCOPE:**
```markdown
OUT_OF_SCOPE:
- Unrelated work
```

âœ… **Complete OUT_OF_SCOPE:**
```markdown
## Scope
- **OUT_OF_SCOPE**:
  * UI changes (cancel button in Jobs view) â†’ separate WP
  * Retry logic (failed job retry) â†’ Phase 2 task
  * Timeout enforcement (cancel if >N seconds) â†’ Phase 2 task
  * Job history/audit trail â†’ separate task
  * Workspace-level job management â†’ separate WP
```

**Why it matters:**
- Coder sees these and avoids temptation to "fix it while we're here"
- Validator can check for scope creep against this list
- Prevents incomplete features (UI missing when backend is done)

---

### Field 7: TEST_PLAN (Exact Validation Commands) [CX-605]

**What Coder expects:**
- EXACT bash commands to run
- Not "test the feature"; exact `cargo test`, `pnpm test` commands
- Coder will copy-paste these commands

**What "complete" means:**
- âœ… For LOW tier: At least 2-3 commands (cargo test, lint)
- âœ… For MEDIUM tier: 4-5 commands (manual review noted separately)
- âœ… For HIGH tier: 5-6 commands (manual review noted separately, stricter checks)
- âœ… Each command is literal (can be copy-pasted)
- âœ… Commands are in logical order (build â†’ test â†’ review)
- âœ… `just post-work WP-{ID}` is ALWAYS included (Step 10 of CODER_PROTOCOL)
- âœ… `just cargo-clean` (uses {{CARGO_TARGET_DIR}}) is listed before post-work/self-eval to flush Cargo artifacts outside the repo

âŒ **Incomplete TEST_PLAN:**
```markdown
TEST_PLAN:
- Run tests
- Check quality
```

âœ… **Complete TEST_PLAN:**
```markdown
## Quality Gate
- **TEST_PLAN**:
  ```bash
  # Compile and unit test
  cargo test --manifest-path {{BACKEND_CARGO_TOML}}

  # React component tests
  pnpm -C {{FRONTEND_ROOT_DIR}} test

  # Linting
  pnpm -C {{FRONTEND_ROOT_DIR}} run lint
  cargo clippy --all-targets --all-features


  # External Cargo target hygiene (keeps repo/mirror slim)
  just cargo-clean

  # Post-work validation
  just post-work WP-1-Storage-Abstraction-Layer
  ```
```

**Why it matters:**
- Coder runs EVERY command in TEST_PLAN before claiming done (Step 7 of CODER_PROTOCOL)
- Exact commands prevent misinterpretation
- Order matters: compile first, then test, then post-work
- `just post-work` is the final gate before commit

---

### Field 8: DONE_MEANS (Success Criteria) [CX-606]

**What Coder expects:**
- Concrete, measurable checklist of "done"
- 1:1 mapped to SPEC_ANCHOR requirements
- Not vague ("works", "passes tests")

**What "complete" means:**
- âœ… 3-8 items, each testable
- âœ… Each item maps to SPEC_ANCHOR: "per Â§2.3.13.1 storage API requirement"
- âœ… Uses MUST/SHOULD language from spec
- âœ… Includes validation success: "All tests pass", "manual review complete"
- âœ… Each item has YES/NO answer (not subjective)

âŒ **Incomplete DONE_MEANS:**
```markdown
DONE_MEANS:
- Feature works
- Tests pass
```

âœ… **Complete DONE_MEANS:**
```markdown
## Quality Gate
- **DONE_MEANS**:
  * âœ… Storage trait defined per Â§2.3.13.1 with 6 required methods (get_blocks, save_blocks, etc.)
  * âœ… AppState refactored to use `Arc<dyn Database>` (not concrete SqlitePool)
  * âœ… SqliteDatabase implements trait with all 6 methods (Â§2.3.13.2)
  * âœ… PostgresDatabase stub created with method signatures (Â§2.3.13.3)
  * âœ… All existing tests pass (5 units + 3 integration tests)
  * âœ… All NEW tests pass (2 trait tests + 2 sqlite impl tests)
  * âœ… manual review complete (PASS/FAIL); unresolved blockers must be fixed
  * âœ… `just post-work WP-1-Storage-Abstraction-Layer` returns PASS
```

**Why it matters:**
- Validator will check each item against code (file:line mapping)
- Spec anchor references prove this WP is NOT inventing requirements
- Clear success criteria prevent "done" wars

---

### Field 9: ROLLBACK_HINT (How to Undo) [CX-607]

**What Coder expects:**
- Clear way to revert the work if something goes wrong
- Simple: `git revert {commit}`
- Complex: Step-by-step undo instructions

**What "complete" means:**
- âœ… Simple case: `git revert {commit-hash}` (once Coder provides commit)
- âœ… Complex case: Multi-step undo guide:
  ```bash
  # Step 1: Revert migration
  # Step 2: Revert trait definition
  # Step 3: Restore AppState
  ```
- âœ… If data migration: Include restore procedure

âŒ **Incomplete ROLLBACK_HINT:**
```markdown
ROLLBACK_HINT: Undo changes if needed
```

âœ… **Complete ROLLBACK_HINT:**
```markdown
## Authority
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  # Single commit reverts:
  # 1. Trait definition (storage.rs new file)
  # 2. AppState refactor (app_state.rs)
  # 3. Migration (0003_storage_api.sql)
  # 4. Tests (new test file)

  # If already deployed, manual steps:
  # - Restore previous AppState code
  # - Run: cargo build
  # - Restart service
  ```
```

**Why it matters:**
- Validator wants to know rollback cost before approving
- Guides incident response if WP causes regression

---

### Field 10: BOOTSTRAP (Coder's Work Plan) [CX-608]

**What Coder expects:**
- Clear map of what to read before coding
- List of files to open, search patterns, commands to run
- So Coder can validate understanding (Step 5 of CODER_PROTOCOL)

**What "complete" means:**

**Sub-field 10A: FILES_TO_OPEN (5-15 files)**
- âœ… Always include: `.GOV/roles_shared/docs/START_HERE.md`, `.GOV/spec/SPEC_CURRENT.md`, `.GOV/roles_shared/docs/ARCHITECTURE.md`
- âœ… Then: 5-15 implementation files (exact paths)
- âœ… Order matters: context first, implementation last

**Sub-field 10B: SEARCH_TERMS (10-20 grep patterns)**
- âœ… Key symbols: "Database", "AppState", "trait"
- âœ… Error messages: "connection failed", "pool exhausted"
- âœ… Feature names: "storage", "migration", "backend"
- âœ… Total: 10-20 patterns for grep -r searches

**Sub-field 10C: RUN_COMMANDS (3-6 startup commands)**
- âœ… `just dev` (start dev environment)
- âœ… `cargo test --manifest-path ...` (verify setup)
- âœ… `pnpm -C {{FRONTEND_ROOT_DIR}} test` (verify frontend setup)
- âœ… Commands Coder can run to validate dev environment

**Sub-field 10D: RISK_MAP (3-8 failure modes)**
- âœ… "{Failure mode}" -> "{Affected subsystem}"
- âœ… Examples:
  - "Trait method missing" -> "Storage layer"
  - "IPC contract breaks" -> "Tauri bridge"
  - "Migration fails" -> "Database layer"

âŒ **Incomplete BOOTSTRAP:**
```markdown
## Bootstrap
- FILES_TO_OPEN: Some files
- SEARCH_TERMS: storage, database
- RUN_COMMANDS: cargo test
- RISK_MAP: TBD
```

âœ… **Complete BOOTSTRAP:**
```markdown
## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/docs/START_HERE.md (repository overview)
  * .GOV/spec/SPEC_CURRENT.md (current spec version)
  * .GOV/roles_shared/docs/ARCHITECTURE.md (storage architecture)
  * {{BACKEND_SRC_DIR}}/lib.rs (module structure)
  * {{BACKEND_SRC_DIR}}/api/mod.rs (API layer)
  * {{BACKEND_SRC_DIR}}/api/jobs.rs (job endpoints - MODIFY)
  * {{BACKEND_SRC_DIR}}/jobs.rs (job logic - MODIFY)
  * {{BACKEND_SRC_DIR}}/workflows.rs (workflow logic - MODIFY)
  * {{BACKEND_SRC_DIR}}/storage/ (new module - CREATE)
  * {{BACKEND_MIGRATIONS_DIR}}/ (schema changes)
  * {{FRONTEND_SRC_DIR}}/components/JobsView.tsx (frontend display)

- **SEARCH_TERMS**:
  * "pub struct AppState" (current app state)
  * "pub struct SqlitePool" (direct DB access - refactor away)
  * "pub trait Database" (new trait we're defining)
  * "impl Database for SqliteDatabase" (implementation)
  * "fn get_blocks", "fn save_blocks" (trait methods)
  * "migration", "CREATE TABLE" (schema changes)
  * "#[tokio::test]" (test patterns)
  * "dyn Database" (trait object usage)
  * "Arc<dyn Database>" (correct dependency injection)
  * "PostgreSQL", "sqlite3" (backend references)

- **RUN_COMMANDS**:
  ```bash
  just dev          # Start dev environment (backend + frontend)
  cargo test --manifest-path {{BACKEND_CARGO_TOML}}  # Unit/integration tests
  pnpm -C {{FRONTEND_ROOT_DIR}} test  # React component tests
  just validate     # Full hygiene check
  ```

- **RISK_MAP**:
  * "Trait method signature mismatch" -> "Storage layer" (causes compilation failure)
  * "AppState refactor incomplete" -> "All job/workflow endpoints" (runtime panics)
  * "Migration doesn't match new schema" -> "Database layer" (corrupt schema)
  * "Impl for SqliteDatabase incomplete" -> "Local storage" (missing functionality)
  * "PostgreSQL stub not compilable" -> "Build pipeline" (compilation blocker)
  * "Test coverage gap" -> "Validator blocks merge" (validation failure)
```

**Why it matters:**
- Coder uses this to output BOOTSTRAP block before implementing (Step 5 of CODER_PROTOCOL)
- Validator checks: "Did Coder read these files?" via BOOTSTRAP output
- Risk map helps Coder understand impact of mistakes

---

### Summary: How Orchestrator Uses This Section

**Before creating task packet:**
1. âœ… Fill all 10 fields with the completeness criteria above
2. âœ… Validate: Every field has no TBDs, placeholders, or vagueness
3. âœ… Run `just pre-work WP-{ID}` to verify file structure
4. âœ… Pass to Validator if they exist, or proceed to delegation

**When delegating to Coder:**
- Coder will verify all 10 fields in Step 2 of CODER_PROTOCOL
- If ANY field is incomplete, Coder will BLOCK and return for fixes
- Once all 10 fields are complete, Coder can proceed confidently

**When Validator reviews:**
- Validator will check: Does task packet enable Coder's work?
- Validator will also check: Are DONE_MEANS 1:1 with SPEC_ANCHOR?
- Validator will verify: Is IN_SCOPE_PATHS necessary and sufficient?

---

## Part 4: Task Packet Creation Workflow [CX-601-607]

---

## Pre-Delegation Checklist (BLOCKING âœ‹)

Complete ALL steps before delegating. If any step fails, STOP and fix it.

### Step 1: Verify Understanding & Blockers âœ‹ STOP

**Before creating task packet, ensure:**
- [ ] User request is clear and unambiguous
- [ ] Scope is well-defined (what's in/out)
- [ ] Success criteria are measurable
- [ ] You understand acceptance criteria

**NEW: Check for blocking dependencies:**
```bash
# Verify blocker status in TASK_BOARD
grep -A5 "## Blocked" .GOV/roles_shared/records/TASK_BOARD.md
```

**NEW: Concurrency / File-Lock Conflict Check (multi-coder sessions) [CX-CONC-001]**

When multiple Coders work in the repo concurrently, treat `IN_SCOPE_PATHS` as the exclusive file lock set for that WP.

- Lock source of truth: Operator-visible Task Board on `main` (recommended: `git show main:.GOV/roles_shared/records/TASK_BOARD.md`) -> `## In Progress` (and `## Active (Cross-Branch Status)` if present).
- Lock set definition: for each in-progress WP, its lock set is the exact file paths listed under its task packet's `IN_SCOPE_PATHS`.
- Hard rule: do NOT delegate/start a new WP if ANY `IN_SCOPE_PATHS` entry overlaps with ANY in-progress WP's `IN_SCOPE_PATHS`.
  - If overlap is required, this is a blocker: re-scope to avoid overlap OR sequence the work (mark WP BLOCKED: "File lock conflict").
- Task Board stays minimal: `## In Progress` uses script-checked lines only. Claim details live in the task packet metadata (CODER_MODEL, CODER_REASONING_STRENGTH); optional branch/coder metadata may be tracked under `## Active (Cross-Branch Status)` on `main`.

Blocking template (use when overlap is detected):
```
Æ’?O BLOCKED: File lock conflict [CX-CONC-001]

Candidate WP: {WP_ID}
Conflicts with in-progress WP: {OTHER_WP_ID} (see task packet CODER_MODEL / CODER_REASONING_STRENGTH)

Overlapping paths:
- {path1}
- {path2}

Action required:
1) Re-scope candidate WP to avoid overlap, OR
2) Sequence work: wait until {OTHER_WP_ID} is VALIDATED and leaves In Progress.
```
- [ ] If this WP has a blocker: Is blocker VALIDATED? âœ…
- [ ] If blocker is not VALIDATED: Mark new WP as BLOCKED (don't proceed yet)
- [ ] If blocker failed validation (FAIL): Escalate; don't create this WP until blocker fixed

**BLOCKING RULE:** Never create downstream WP if blocker is not VALIDATED.
If blocker is READY/IN-PROGRESS/BLOCKED â†’ Mark new WP as BLOCKED in TASK_BOARD.

**IF UNCLEAR (Requirements ambiguous):**
```
âŒ BLOCKED: Requirements unclear [CX-584]

I need clarification on:
1. [Specific ambiguity]
2. [Missing information]
3. [Conflicting requirements]

Please provide clarification before I can create a task packet.
```

**IF BLOCKER NOT READY (Dependency not VALIDATED):**
```
âš ï¸ BLOCKED: Depends on unresolved blocker [CX-635]

This WP depends on:
- WP-1-Storage-Abstraction-Layer (Status: In Progress, not VALIDATED)

Blocker ETA: [when do you expect this to VALIDATE?]

Action: I'm marking this WP as BLOCKED in TASK_BOARD.
When blocker VALIDATEs, I'll move this to READY FOR DEV.
```

**STOP** - Do not proceed with assumptions or unresolved blockers.

---

### Step 2: Create Task Packet âœ‹ STOP

**1. Check for ID collision:**
```bash
ls .GOV/task_packets/WP-{phase}-{name}*.md
```
*Do NOT use date/time stamps in WP IDs. If the base WP ID already exists, create a revision packet using `-v{N}`.*
*Example: `WP-1-Tokenization-Service-v3`*

**2. Use template generator:**
```bash
just create-task-packet "WP-{phase}-{name}-v{N}"
```
*If script fails -> STOP. Resolve collision.*

**3. Fill details (Update only):**
Edit `.GOV/task_packets/WP-{ID}.md` to fill placeholders.

Use this template:
```markdown
# Task Packet: WP-{phase}-{short-name}

## Metadata
- TASK_ID: WP-{phase}-{short-name}
- DATE: {ISO 8601 timestamp}
- REQUESTOR: {user or source}
- AGENT_ID: {your agent ID}
- ROLE: Orchestrator

## Scope
- **What**: {1-2 sentence description}
- **Why**: {Business/technical rationale}
- **IN_SCOPE_PATHS**:
  * {specific file or directory}
  * {another specific path}
- **OUT_OF_SCOPE**:
  * {what NOT to change}
  * {deferred work}

## Quality Gate
- **RISK_TIER**: LOW | MEDIUM | HIGH
  - LOW: Docs-only, no behavior change
  - MEDIUM: Code change, one module, no migrations
  - HIGH: Cross-module, migrations, IPC, security
- **TEST_PLAN**:
  ```bash
  # Commands coder MUST run:
  cargo test --manifest-path {{BACKEND_CARGO_TOML}}
  pnpm -C {{FRONTEND_ROOT_DIR}} test
  pnpm -C {{FRONTEND_ROOT_DIR}} run lint
  ```
- **DONE_MEANS**:
  * {Specific criterion 1}
  * {Specific criterion 2}
  * All tests pass
  * Validation clean
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha>
  # OR: Specific undo steps
  ```

## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/docs/START_HERE.md
  * .GOV/spec/SPEC_CURRENT.md
  * .GOV/roles_shared/docs/ARCHITECTURE.md
  * {5-10 implementation-specific files}
- **SEARCH_TERMS**:
  * "{key symbol/function}"
  * "{error message}"
  * "{feature name}"
  * "{5-20 grep targets}"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path {{BACKEND_CARGO_TOML}}
  pnpm -C {{FRONTEND_ROOT_DIR}} test
  ```
- **RISK_MAP**:
  * "Database migration fails" -> Storage layer
  * "IPC contract breaks" -> Tauri bridge
  * "{3-8 failure modes}" -> "{affected subsystem}"

## Authority
- **SPEC_BASELINE**: {{PROJECT_PREFIX}}_Master_Spec_vXX.XX.md (spec at packet creation time; provenance)
- **SPEC_TARGET**: .GOV/spec/SPEC_CURRENT.md (binding spec for closure/revalidation; resolved at validation time)
- **SPEC_ANCHOR**: {master spec section(s) / anchors}
- **Codex**: {{CODEX_FILENAME}}
- **Task Board**: .GOV/roles_shared/records/TASK_BOARD.md
- **Logger**: (optional) latest {{PROJECT_PREFIX}}_logger_* if requested for milestone/hard bug
- **ADRs**: {if relevant}

## Notes
- **Assumptions**: {If any - mark as ASSUMPTION}
- **Open Questions**: {If any - must resolve before coding}
- **Dependencies**: {Other work this depends on}
```

**Verify file created:**
```bash
ls -la .GOV/task_packets/WP-*.md
```

---

### Step 3: Update Task Board âœ‹ STOP

**Update `.GOV/roles_shared/records/TASK_BOARD.md`:**
- Move WP-{ID} to "Ready for Dev"
- Or "In Progress" if assigning immediately

**Verify file updated:**
```bash
grep "WP-{ID}" .GOV/roles_shared/records/TASK_BOARD.md
```

**Note:** You DO NOT need to create a logger entry at this stage. Logger entries are reserved for work completion, milestones, or critical blockers.

---

### Step 4: Verification âœ‹ STOP

**Run automated check:**
```bash
just pre-work WP-{ID}
```

**MUST see:**
```
âœ… Pre-work validation PASSED

You may proceed with delegation.
```

**If FAIL:**
```
âŒ Pre-work validation FAILED

Errors:
  1. [Error description]

Fix these issues before delegating.
```

Fix errors, then re-run `just pre-work`.

---

### Step 5: Delegate to Coder

**Hand

off message format:**
```
Task Packet: .GOV/task_packets/WP-{ID}.md
WP_ID: WP-{ID}
RISK_TIER: {LOW|MEDIUM|HIGH}

ðŸ“‹ Task: {One line summary}

You are a Coder agent. Before writing code:
1. Read .claude/CODER_PROTOCOL.md
2. Read the task packet above
3. Run: just pre-work WP-{ID}
4. Output BOOTSTRAP block per [CX-622]
5. Verify packet scope matches user request

Authority docs:
- .GOV/roles_shared/docs/START_HERE.md
- .GOV/spec/SPEC_CURRENT.md
- .GOV/roles_shared/docs/ARCHITECTURE.md
- {{CODEX_FILENAME}}

âœ… Orchestrator checklist complete. Task packet WP-{ID} created and verified.

Begin implementation when ready.
```

---

## Task State Management (Shared Responsibility)

Task state is managed by the agent currently holding the "ball":
1. **Orchestrator**: Creates WP -> Adds to `Ready for Dev`.
2. **Coder**: Starts work -> Moves to `In Progress` (during BOOTSTRAP).
3. **Validator**: Approves work -> Moves to `Done` (during VALIDATION).
4. **Orchestrator**: Escalation/Blocker -> Moves to `Blocked`.

### Orchestrator Board Integrity Check âœ‹
When updating the board, the Orchestrator MUST ensure these 5 fixed sections exist (DO NOT delete them even if empty):
- `## Ready for Dev`
- `## In Progress`
- `## Done`
- `## Blocked`
- `## Superseded (Archive)`

### Step 1: Update Task Packet STATUS

When a task's state changes (e.g., from `Ready-for-Dev` to `In-Progress`, or to `Done`), the active agent MUST edit the corresponding task packet markdown file to update the `STATUS` field in the metadata.

### Step 2: Update the Task Board

Immediately after updating the packet's status, the active agent MUST also edit `.GOV/roles_shared/records/TASK_BOARD.md` to move the `WP-ID` to the correct column.

**This two-step process ensures both the detailed ticket and the high-level board are always in sync.**

---

## BLOCKING RULES (Non-Negotiable)

### âŒ DO NOT delegate if:
1. Requirements are unclear or ambiguous [CX-584]
2. Task packet file does not exist [CX-580]
3. `just pre-work` validation fails [CX-587]
4. You haven't confirmed packet completeness [CX-582]

### âœ… DO delegate when:
1. All steps complete
2. `just pre-work WP-{ID}` returns PASS
3. Handoff message includes all required info
4. You've confirmed coder understands the task

---

## If Blocked

**Scenario**: User request is too vague

**Response**:
```
âŒ BLOCKED: Cannot create task packet [CX-584]

The request is ambiguous on:
- {Specific ambiguity 1}
- {Specific ambiguity 2}

Please clarify:
1. {Question 1}
2. {Question 2}

Once clarified, I can create a complete task packet.
```

**Scenario**: Missing context (no spec slice provided)

**Response**:
```
âŒ BLOCKED: Missing LAW context [CX-031]

This task requires information from:
- {Spec section or context needed}

Please provide this context OR narrow the task to what's feasible without it.
```

**Scenario**: Too large/complex for single packet

**Response**:
```
âš ï¸ WARNING: Task is large [CX-584]

This task touches:
- {Multiple subsystems}
- {High complexity areas}

Recommendation: Break into smaller work packets:
1. WP-{phase}-{part-A}: {Scope A}
2. WP-{phase}-{part-B}: {Scope B}

Proceed with breakdown? Or continue with full scope?
```

---

## Common Mistakes (Avoid These)

### âŒ Mistake 1: Vague scope
**Wrong:**
```
SCOPE: Improve the job system
```
**Right:**
```
SCOPE: Add `/jobs/:id/cancel` endpoint to allow users to cancel running jobs
IN_SCOPE_PATHS:
- {{BACKEND_SRC_DIR}}/api/jobs.rs
- {{BACKEND_SRC_DIR}}/jobs.rs
OUT_OF_SCOPE:
- Job retry logic (separate task)
- UI changes (separate task)
```

### âŒ Mistake 2: Missing DONE_MEANS
**Wrong:**
```
DONE_MEANS: Feature works
```
**Right:**
```
DONE_MEANS:
- POST /jobs/:id/cancel returns 200 for running jobs
- Job status updates to "cancelled" in database
- Workflow execution stops within 5 seconds
- cargo test passes (2 new tests added)
- pnpm test passes
```

### âŒ Mistake 3: Incomplete BOOTSTRAP
**Wrong:**
```
FILES_TO_OPEN: Some files
```
**Right:**
```
FILES_TO_OPEN:
- .GOV/roles_shared/docs/START_HERE.md
- .GOV/roles_shared/docs/ARCHITECTURE.md
- {{BACKEND_SRC_DIR}}/api/jobs.rs
- {{BACKEND_SRC_DIR}}/jobs.rs
- {{BACKEND_SRC_DIR}}/workflows.rs
- {{BACKEND_SRC_DIR}}/models.rs
- {{BACKEND_MIGRATIONS_DIR}}/0002_create_ai_core_tables.sql
```

### âŒ Mistake 4: Delegating without verification
**Wrong:**
```
I created the packet. Coder, start coding.
```
**Right:**
```
Running verification:
$ just pre-work WP-1-Job-Cancel

âœ… Pre-work validation PASSED

Task Packet: .GOV/task_packets/WP-1-Job-Cancel.md
[Full handoff message...]
```

---

## Success Criteria

**You succeeded if:**
- âœ… Task packet file exists and is complete
- âœ… `just pre-work WP-{ID}` passes
- âœ… Coder receives clear handoff message
- âœ… **YOU STOPPED TALKING** after the handoff message

**You failed if:**
- âŒ You wrote code in `{{BACKEND_ROOT_DIR}}/` or `{{FRONTEND_ROOT_DIR}}/`
- âŒ Coder asks "what should I do?"
- âŒ Coder starts coding without packet
- âŒ Work gets rejected at review for missing packet
- âŒ Scope confusion leads to wrong implementation

---

## Quick Reference

**Commands:**
```bash
# Create packet
just create-task-packet WP-{ID}

# Verify readiness
just pre-work WP-{ID}

# Check packet exists
ls .GOV/task_packets/WP-*.md
```

**Codex rules enforced:**
- [CX-580]: Packet MUST be created before delegation
- [CX-581]: Packet MUST have required structure
- [CX-582]: Packet MUST be verified before delegation
- [CX-584]: MUST NOT delegate ambiguous work
- [CX-585]: Update task board; logger only if explicitly requested for milestone/hard bug
- [CX-587]: SHOULD run pre-work check

**Remember**: Better to spend 10 minutes on a good task packet than 2 hours fixing misunderstood work.

---

## Part 5: Work Packet Lifecycle in Detail [CX-620-625]

### 5.1 Required Fields in Every Work Packet

Every work packet MUST include these sections (in order):

```markdown
# Task Packet: WP-{phase}-{name}

## Metadata
- TASK_ID: WP-{phase}-{name}
- DATE: {ISO 8601 timestamp}
- REQUESTOR: {user or source}
- AGENT_ID: {your agent ID}
- ROLE: Orchestrator
- STATUS: {Ready-for-Dev|In-Progress|Done|Blocked}

## Scope
- **What**: {1-2 sentence description}
- **Why**: {Business/technical rationale}
- **IN_SCOPE_PATHS**: {Exact file paths - NOT vague directories}
  * {{BACKEND_SRC_DIR}}/storage/mod.rs
  * {{BACKEND_SRC_DIR}}/storage/sqlite.rs
- **OUT_OF_SCOPE**: {What Coder CANNOT touch}
  * Migrations rewrite (â†’ WP-1-Migration-Framework)

## Quality Gate
- **RISK_TIER**: LOW | MEDIUM | HIGH
- **TEST_PLAN**: {Exact bash commands}
- **DONE_MEANS**: {Measurable criteria - 1:1 mapped to SPEC_ANCHOR}
- **ROLLBACK_HINT**: {How to undo}

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**: {5-15 key files}
- **SEARCH_TERMS**: {10-20 grep targets}
- **RUN_COMMANDS**: {Startup + validation commands}
- **RISK_MAP**: {Failure modes â†’ subsystems (3-8 items)}

## Authority
- **SPEC_ANCHOR**: Â§{section} ({requirement})
- **Codex**: {version}
- **Task Board**: .GOV/roles_shared/records/TASK_BOARD.md
- **Logger**: {if applicable}

## Notes
- **Assumptions**: {Any assumptions}
- **Open Questions**: {Questions to resolve}
- **Dependencies**: {Other WPs this depends on}

---

**Last Updated:** {date}
**User Signature Locked:** {signature}
```

[ADD v02.178] Work Packets are authoritative execution contracts, not retrieval summaries. When a Work Packet id, active packet path, or linked binding is known, consuming systems MUST load the canonical packet record or mirror directly; hybrid RAG MAY supply adjacent precedent or related evidence but MUST NOT override active status, blockers, dependencies, or gate state.

### 5.2 SPEC_ANCHOR Requirement (CRITICAL) [CX-601]

**EVERY WP MUST reference Master Spec Main Body (NOT Roadmap).**

**CLARIFICATION: Orchestrator's Role in SPEC_ANCHOR Verification**

Orchestrator DOES verify (checklist below):
- âœ… SPEC_ANCHOR cites a Main Body section (not Roadmap)
- âœ… Cited section exists in SPEC_CURRENT.md
- âœ… Section number is specific (Â§2.3.13.1, not Â§2.3.13 alone)

Orchestrator DOES NOT verify (Validator verifies this):
- âŒ Whether the cited requirement is the RIGHT interpretation
- âŒ Whether this requirement is complete/correct
- âŒ Whether all MUST/SHOULD from that section are covered

**If SPEC_ANCHOR is ambiguous** (could map to multiple sections):
â†’ ESCALATE to user; get explicit decision before proceeding.
Do not guess which section is correct.

**Valid SPEC_ANCHOR examples:**
- `Â§2.3.13.1 (Four Portability Pillars)`
- `Â§2.3.13.3 (Storage API Abstraction Pattern)`
- `Â§A9.2.1 (Error Code Registry)`

**Invalid (REJECT these):**
- `Â§Future Work (Phase 2+)` â€” Not Main Body
- `Â§Roadmap` â€” Not specific enough
- No SPEC_ANCHOR at all â€” Every WP requires one
- `Â§2.3.13` alone â€” Too broad; need specific subsection

**Orchestrator verification checklist:**
- [ ] SPEC_ANCHOR references MAIN BODY section (before Roadmap)
- [ ] SPEC_ANCHOR exists in latest Master Spec version
- [ ] Section number is specific (Â§X.X.X format)
- [ ] If multiple valid sections exist â†’ ESCALATE to user for clarification

**If FAIL:** Reject WP; request Orchestrator cite spec requirement explicitly or escalate.

### 5.3 IN_SCOPE_PATHS Precision [CX-603]

**Orchestrator MUST be specific (NOT vague).**

```
âŒ WRONG: IN_SCOPE_PATHS: src/backend
âŒ WRONG: IN_SCOPE_PATHS: src/
âŒ WRONG: IN_SCOPE_PATHS: Everything related to storage

âœ… RIGHT: IN_SCOPE_PATHS:
  - {{BACKEND_SRC_DIR}}/storage/mod.rs
  - {{BACKEND_SRC_DIR}}/storage/sqlite.rs
  - {{BACKEND_SRC_DIR}}/api/jobs.rs
```

**Why:** Coder needs to know EXACTLY which files they can modify. Vague scope = scope creep.

### 5.4 DONE_MEANS Mapping [CX-602]

**Every DONE_MEANS MUST map 1:1 to SPEC_ANCHOR requirement.**

Example:
```markdown
SPEC_ANCHOR: Â§2.3.13.3 (Storage API Abstraction Pattern)

Spec says:
- "MUST: Define Database trait with async methods"
- "MUST: Implement SqliteDatabase wrapper"
- "MUST: Create PostgresDatabase stub"

DONE_MEANS (mapped):
- [ ] MUST: Database trait defined (Â§2.3.13.3, requirement 1)
- [ ] MUST: SqliteDatabase implemented (Â§2.3.13.3, requirement 2)
- [ ] MUST: PostgresDatabase stub created (Â§2.3.13.3, requirement 3)
- [ ] All tests pass
- [ ] Validator sign-off (PASS verdict)
```

**Rule:** If DONE_MEANS doesn't map to spec, Validator rejects it.

### 5.5 BOOTSTRAP Completeness [CX-606]

**Orchestrator MUST provide:**

1. **FILES_TO_OPEN (5-15 files minimum)**
   - Spec docs (SPEC_CURRENT.md, Master Spec section)
   - Architecture docs (ARCHITECTURE.md, relevant design docs)
   - Implementation files (files Coder will modify)
   - Related modules (dependencies, imports)

2. **SEARCH_TERMS (10-20 grep targets minimum)**
   - Key symbols to find (`SqlitePool`, `state.pool`)
   - Error messages to look for
   - Feature names to search
   - Pattern names (`DefaultStorageGuard`)

3. **RUN_COMMANDS (startup + validation)**
   - Dev environment startup (`just dev`)
   - Test commands (`cargo test`, `pnpm test`)
   - Validation commands (`just validate`) + manual review requirement

4. **RISK_MAP (3-8 failure modes)**
   - Specific failure mode
   - Which subsystem breaks
   - Example: `"Hollow trait implementation" â†’ Portability Failure (Phase 1 blocker)`

### 5.6 Work Packet Locking [CX-607]

**Orchestrator MUST lock packet after creation:**

```markdown
---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220250328

**IMPORTANT: This packet is locked. No edits allowed.**
**If changes needed: Create NEW packet (WP-{ID}-variant), do NOT edit this one.**
```

**Rule of Locking:**
- âœ… Once locked, packet is immutable
- âœ… Prevents instruction creep mid-work
- âœ… Creates audit trail (version history)
- âŒ Cannot edit locked packet (violates governance)
- âŒ If changes needed, must create new packet

**When to create variant packets:**
- WP-1-Storage-Abstraction-Layer (original, locked)
- WP-1-Storage-Abstraction-Layer-v2 (changes needed, new packet)
- OR: WP-1-Storage-Abstraction-Layer-v3 (next revision; no date/time stamps)

**Traceability rule (mandatory when variants exist):**
- Treat `WP-1-Storage-Abstraction-Layer` as the **Base WP ID**.
- If you create `...-v{N}`, update `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` so the Base WP maps to the single Active Packet, and mark the older packet(s) as Superseded on `.GOV/roles_shared/records/TASK_BOARD.md`.
- When instructing Coders/Validators to run `just pre-work` / `just post-work`, always provide the **Active Packet WP_ID** (often includes `-vN`) to avoid ambiguous matches.


#### 5.6.1 File-scope locks for concurrent Work Units (WP/MT) (Normative) [ADD v02.122]

Work Packet **immutability** (this section) is a governance rule. Separately, **file-scope locks** prevent concurrent Work Units from mutating overlapping files.

This contract formalizes INV-MM-003 (Â§4.3.9.3): **strict non-overlap of file scopes under concurrency.**

##### 5.6.1.1 Definitions (normative)

- **Work Unit**: a WP or an MT executing under Locus governance.
- **Lock set**: the canonical set of file paths the Work Unit may modify.
  - For WPs, this is `IN_SCOPE_PATHS` in the Task Packet.
  - For MTs, this is either:
    - an explicit subset declared in the MT definition, or
    - inherited from the parent WP if not specified.

##### 5.6.1.2 Rules (HARD)

- If more than one Work Unit is active/executing, each Work Unit MUST hold a lock set.
- Two concurrently executing Work Units MUST NOT hold overlapping lock sets.
- On overlap detection, the system MUST deterministically block one Work Unit and surface:

  - `code = CX-MM-002` (File-scope lock conflict),
  - explicit conflicting paths,
  - required operator action (choose priority, split scope, or queue).

##### 5.6.1.3 Required surfaces (normative)

When `CX-MM-002` occurs, it MUST appear in:

- Task Board status for the blocked Work Unit,
- the Work Unitâ€™s canonical status artifact (WP/MT status),
- the single-line HSK_STATUS marker (`lock=CONFLICT code=CX-MM-002`) (see Â§4.3.9.8.3),
- Flight Recorder events correlated to the WP/MT and the conflicting Work Unit(s).

##### 5.6.1.4 Relation to existing multi-coder lock check

The existing operator check **Concurrency / File-Lock Conflict Check (multi-coder sessions) [CX-CONC-001]** is compatible with this contract. In v02.122, `CX-MM-002` is the canonical machine-visible code for lock conflicts; `CX-CONC-001` remains a human process/checklist identifier.


### 5.7 Variant Lineage Audit (ALL versions) [CX-580E] (BLOCKING)

When you create a revision packet (`-v{N}`) for a Base WP, you MUST include a **Lineage Audit** inside the new packet before delegation.

**Goal:** Prevent â€œspecâ†’packetâ†’codeâ€ gaps caused by version churn. A `-v{N}` packet is NOT allowed to validate only â€œwhat changed in v{N}â€; it must prove the **entire Base WP requirement** is satisfied in the repo as of SPEC_TARGET.

**MANDATORY:** Add `## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]` to the new packet and include, at minimum:
- `BASE_WP_ID` and the new `WP_ID` being created.
- Roadmap pointer(s) (if applicable) AND the governing Master Spec Main Body anchors for â€œDoneâ€.
- `SPEC_TARGET` resolved at creation time (from `.GOV/spec/SPEC_CURRENT.md`).
- A list of ALL known prior packet files for the Base WP (v1/v2/...) and their statuses (Superseded/FAIL/Historical/etc.).
- A requirement map showing every governing Main Body MUST/SHOULD translated to current repo evidence:
  - `SPEC_ANCHOR` (exact clause ID)
  - Code evidence (`path:line` in the repo)
  - Provenance (introducing commit via `git blame`, or explicit â€œpresent before v{N}â€)
  - If anything is missing: declare GAP and STOP (create a remediation WP or initiate spec enrichment).

**Suggested commands (examples):**
- `cat .GOV/spec/SPEC_CURRENT.md`
- `rg -n "<forbidden symbols>" src/`
- `git blame -n -L <line>,<line> <path>`
- `git log --oneline --decorate -- <path>`

**Template (copy into the packet):**
```markdown
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-...
- WP_ID: WP-1-...-vN
- SPEC_TARGET: {{PROJECT_PREFIX}}_Master_Spec_vXX.XXX.md (from .GOV/spec/SPEC_CURRENT.md)
- Roadmap pointer: Â§7.6.x (pointer only; Main Body is authority)
- Prior packets:
  - .GOV/task_packets/WP-1-....md (status: ...)
  - .GOV/task_packets/WP-1-....-v2.md (status: ...)

| SPEC_ANCHOR | Main Body requirement (MUST/SHOULD) | Repo evidence (path:line) | Introduced (commit) | Notes |
|---|---|---|---|---|
| A?.?.? | ... | ... | ... | ... |
```

---

## Part 6: Task Board Maintenance [CX-625-630]

### 6.1 Task Board Structure (Single Source of Truth)

**Orchestrator maintains `.GOV/roles_shared/records/TASK_BOARD.md` as the authoritative status tracker.**

**Version-tag review rule (normative):** If Task Board entries include a spec-version tag (e.g., `v02.116`), then whenever that spec version's scope is amended, Orchestrator MUST run a revision pass over those entries (status, scope, links) and revise/update them to match current spec + roadmap semantics.

```markdown
# {{PROJECT_DISPLAY_NAME}} Project Task Board

This board is a shared state file updated by the active agent (Orchestrator, Coder, Validator).
Updated whenever WP status changes.

---

## ðŸš¨ PHASE 1 CLOSURE GATES (BLOCKING)

**Authority:** Master Spec Â§2.3.13, Architecture Decision {date}

Storage Backend Portability Foundation (Sequential):

1. **[WP-1-Storage-Abstraction-Layer]** - Define trait-based storage API
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 15-20 hours
   - Status: [READY FOR DEV ðŸ”´]
   - Blocker: None (foundational)

2. **[WP-1-AppState-Refactoring]** - Remove SqlitePool from AppState
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 8-10 hours
   - Status: [GAP ðŸŸ¡]
   - Blocker: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)

---

## In Progress

- **[WP_ID]** - {VALIDATION_STATUS}

## Ready for Dev

- **[WP_ID]** - {VALIDATION_STATUS}

## Done

- **[WP_ID]** - {VALIDATION_STATUS}

## Blocked

- **[WP_ID]** - BLOCKED: {Reason for block}

## Superseded (Archive)

- **[WP_ID]** - SUPERSEDED: {Reason for archival}
```

### 6.2 Status Values (CX-625)

| Status | Symbol | Meaning | When to Use |
|--------|--------|---------|------------|
| **READY FOR DEV** | ðŸ”´ | Verified, waiting for Coder | After pre-work checklist PASS |
| **IN PROGRESS** | ðŸŸ  | Coder is working | After Coder outputs BOOTSTRAP |
| **BLOCKED** | ðŸŸ¡ | Waiting for dependency/clarification | Document specific reason |
| **DONE** | âœ… | Merged to main | After Validator approves |
| **GAP** | ðŸŸ¡ | Not yet created as packet | Before Orchestrator creates |

### 6.3 Orchestrator Responsibilities for TASK_BOARD

**Ensure TASK_BOARD is updated IMMEDIATELY when:**
1. New WP created â†’ Move to "Ready for Dev"
2. Coder starts work â†’ Ensure the Coder has produced a docs-only bootstrap claim commit; Validator status-syncs `main` (updates `## In Progress`; optionally also `## Active (Cross-Branch Status)`).
3. Blocker discovered â†’ Move to "Blocked" + document reason
4. Validator approves â†’ Validator moves to "Done" (Orchestrator verifies TASK_BOARD reflects reality)
5. Dependency unblocked â†’ Move blocked WP to "Ready for Dev"

**Keep TASK_BOARD in sync with reality:**
```
Never let TASK_BOARD drift from actual WP status.
If the Operator-visible Task Board on `main` does not reflect packet reality, the Validator must run a docs-only status-sync commit to correct it.
```

### 6.4 Phase Gate Status Tracking [CX-609]

**Orchestrator must maintain Phase Gate section:**

```markdown
## ðŸš¨ PHASE 1 CLOSURE GATES (BLOCKING - MUST COMPLETE)

**Status:** HOLDING - 3 of 4 gate-critical WPs not yet created

Gate-critical WPs:
1. âœ… WP-1-Storage-Abstraction-Layer [READY FOR DEV]
2. âŒ WP-1-AppState-Refactoring [GAP - packet not yet created]
3. âŒ WP-1-Migration-Framework [GAP - packet not yet created]
4. âŒ WP-1-Dual-Backend-Tests [GAP - packet not yet created]

Phase closure criteria:
- [ ] All 4 gate-critical WPs are VALIDATED (not just "done")
- [ ] Spec regression check PASS (just validator-spec-regression)
- [ ] All dependencies resolved
- [ ] Waivers audit complete
- [ ] Supply chain clean (cargo deny + npm audit)

Current status: 25% ready (1 of 4 packets created, 0 VALIDATED)
```

### 6.5 Phase Closure Gate (Explicit Requirements) [CX-609B]

**A phase is ready to close ONLY when ALL criteria below are met.**

#### MUST Criteria (All Required)

- [ ] **All phase-critical WPs are VALIDATED** (Validator approved, not just "done")
  - Meaning: Validator returned `verdict: PASS` for each WP
  - Not: "Coder finished coding" or "work merged"

- [ ] **Spec regression check passes**
  ```bash
  just validator-spec-regression
  # Output: âœ… Spec regression check PASSED
  ```

- [ ] **Supply chain audit clean** (zero violations)
  ```bash
  cargo deny check    # Should return 0 violations
  npm audit           # Should return 0 critical/high vulnerabilities
  ```

- [ ] **No unresolved blockers** (all dependencies satisfied)
  - TASK_BOARD shows NO items in "Blocked" state
  - All WPs have clear VALIDATED status for their dependencies

- [ ] **Git commit audit trail complete** (all commits signed/traced)
  - All work-related commits must have proper git metadata (author, timestamp)
  - Optional: If using git signatures, all commits must be signed

#### SHOULD Criteria (Strong Recommendations)

- [ ] **No open escalations from Validator** (all escalations resolved)
- [ ] **No "deferred work" notes in WPs** (all planned work in this phase is done)
- [ ] **Test coverage metrics on target** (>80% for phase)
- [ ] **Security audit clean** (if phase touches security-sensitive code)

#### Example: Phase 1 Closure Gate

```
Phase 1 Closure Gate Status:

MUST Criteria:
âœ… WP-1-Storage-Abstraction-Layer: VALIDATED (PASS)
âœ… WP-1-AppState-Refactoring: VALIDATED (PASS)
âœ… WP-1-Migration-Framework: VALIDATED (PASS)
âœ… WP-1-Dual-Backend-Tests: VALIDATED (PASS)
âœ… Spec regression: PASS
âœ… Cargo deny: 0 violations
âœ… npm audit: 0 high vulnerabilities
âœ… No blockers in TASK_BOARD
âœ… All commits properly tracked

SHOULD Criteria:
âœ… No escalations pending
âœ… No deferred work notes
âœ… Test coverage: 84% (>80% target met)
âœ… Security audit clean (Phase 1 touches storage layer)

â†’ Phase 1 READY TO CLOSE âœ…
```

#### How to Use This Gate

**Before closing phase:**
1. âœ… Check TASK_BOARD: All critical WPs show VALIDATED?
2. âœ… Run spec regression check
3. âœ… Run supply chain audits
4. âœ… Review escalations log (empty?)
5. âœ… Review WPs for deferred work notes
6. âœ… Confirm all dependencies resolved

**If ANY MUST criterion fails:**
â†’ Phase is NOT ready. Document blocker + ETA.

**If ALL MUST criteria pass:**
â†’ Phase ready to close (SHOULD criteria are recommendations, not blockers).

---

## Part 7: Dependency Management [CX-630-635]

### 7.1 Blocking Dependencies

**Orchestrator MUST identify and document all blocking relationships:**

**In work packets:**
```markdown
## Dependencies

- Depends on: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)
- Blocks: WP-1-Dual-Backend-Tests
- Can start independently: WP-1-Migration-Framework
```

**In TASK_BOARD:**
```markdown
2. **[WP-1-AppState-Refactoring]**
   - Blocker: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)
```

### 7.2 Blocking Rules (MANDATORY)

**DO NOT assign WP if blocker is not VALIDATED:**

```
Scenario: WP-1-AppState-Refactoring depends on WP-1-Storage-Abstraction-Layer

If WP-1-Storage-Abstraction-Layer status is:
- âœ… VALIDATED â†’ Can assign WP-1-AppState-Refactoring
- ðŸŸ  IN PROGRESS â†’ Mark WP-1-AppState-Refactoring as BLOCKED
- ðŸ”´ READY FOR DEV â†’ Mark WP-1-AppState-Refactoring as BLOCKED
- âŒ FAILS Validator â†’ Don't assign, escalate

Rule: Never assign downstream work until blocker is VALIDATED.
```

**DO NOT close phase if blockers unresolved:**

```
Phase 1 closure requires:
- ALL 4 gate-critical WPs VALIDATED
- ALL dependencies satisfied
- NO unresolved blockers

If WP-1-Migration-Framework blocks WP-1-Dual-Backend-Tests:
â†’ Phase cannot close until BOTH are VALIDATED
```

**Document WHY WP is BLOCKED:**

```markdown
## Blocked

- WP-1-AppState-Refactoring: Waiting for WP-1-Storage-Abstraction-Layer to VALIDATE (ETA 3 days)
- WP-1-Dual-Backend-Tests: Blocked on 2 dependencies (WP-1-Storage-Abstraction-Layer, WP-1-Migration-Framework)
```

### 7.3 SLA for Work States [CX-635B]

**Orchestrator MUST enforce time-based SLAs to prevent work from stalling.**

| Status | Max Duration | Action if Exceeded | Escalation |
|--------|--------------|-------------------|------------|
| **BLOCKED** | 5 work days | Escalate blocker | Notify user: "WP-X has been blocked for 6 days. What's the plan?" |
| **READY FOR DEV** | 10 work days | Flag as risk | Check: Is Coder assigned? Is there a hidden blocker? |
| **IN PROGRESS** | 30 work days | Assess estimate | Was original estimate wrong? Do we need to split the work? |

#### BLOCKED Status (Max 5 work days)

**Scenario:** WP-1-AppState-Refactoring depends on WP-1-Storage-Abstraction-Layer

**Day 0-4:** Document blocker, leave in BLOCKED state

**Day 5:** If blocker still unresolved:
```
âš ï¸ ESCALATION: WP-X blocked beyond SLA [CX-635-B1]

WP-ID: WP-1-AppState-Refactoring
Status: BLOCKED (5 days, SLA exceeded)
Blocker: WP-1-Storage-Abstraction-Layer (status: {current status})

This WP cannot proceed until blocker resolves.

Action required:
1. What is the updated ETA for blocker resolution?
2. Should we split this work differently?
3. Is there alternative work to do while we wait?

Awaiting response by: {date/time}
```

#### READY FOR DEV Status (Max 10 work days)

**Scenario:** Packet created and verified, waiting for Coder to start

**Day 0-9:** WP sits in "Ready for Dev", waiting for Coder assignment

**Day 10:** If Coder hasn't started:
```
ðŸš¨ RISK FLAG: WP-X idle beyond SLA [CX-635-B2]

WP-ID: WP-1-Job-Cancel-Endpoint
Status: READY FOR DEV (10 days, no progress)
Created: {date}, assigned: {date}

Risk assessment:
- Is Coder aware of this task?
- Is there a blocker we missed?
- Should Coder prioritize this over other work?

Action: Confirm priority and Coder assignment
```

#### IN PROGRESS Status (Max 30 work days)

**Scenario:** Coder is actively working

**Day 0-29:** Coder makes progress, updates task packet with partial results

**Day 30:** If still IN PROGRESS with no completion in sight:
```
ðŸ“‹ ESTIMATE REVIEW: WP-X progress check [CX-635-B3]

WP-ID: WP-1-Storage-Abstraction-Layer
Status: IN PROGRESS (30 days, original estimate: 15-20 hours)

Actual progress: {what's done, what's remaining}
Original estimate: 15-20 hours (estimated 3-5 work days)
Actual effort: 30+ days

Analysis:
- Was original estimate too low?
- Did scope creep occur?
- Are there unexpected blockers?
- Should we split work into smaller packets?

Action: Reassess estimate or break work into phases
```

#### Escalation Template (Universal)

Use this template for ANY SLA-triggered escalation:

```
âš ï¸ SLA ESCALATION: {WP-ID} [CX-635]

**Work Packet:** {WP-ID} ({brief description})
**Status:** {BLOCKED|READY FOR DEV|IN PROGRESS}
**Duration:** {X days} (SLA limit: {Y days})
**Created:** {date}, Last update: {date}

**Current State:**
{Description of why we're escalating}

**Blocker/Issue:**
{Specific thing preventing progress}

**Action Needed:**
{What must happen to unblock}

**Response Required By:** {date/time}
**Escalation Channel:** {user|team lead|project manager}
```

---

## Part 8: Pre-Delegation Validation Checklist [CX-640]

**Before handing off to Coder, Orchestrator MUST verify all 14 items:**

- [ ] SPEC_ANCHOR references Main Body (not Roadmap)
- [ ] SPEC_ANCHOR in latest Master Spec version
- [ ] IN_SCOPE_PATHS are exact file paths (not "src/backend")
- [ ] OUT_OF_SCOPE clearly lists what Coder cannot touch
- [ ] DONE_MEANS are measurable (100% verifiable, not subjective)
- [ ] Every DONE_MEANS maps 1:1 to SPEC_ANCHOR requirement
- [ ] RISK_TIER assigned (LOW/MEDIUM/HIGH)
- [ ] TEST_PLAN includes all applicable commands
- [ ] TEST_PLAN lists `just cargo-clean` (external `{{CARGO_TARGET_DIR}}`) before post-work/self-eval
- [ ] BOOTSTRAP has 5-15 FILES_TO_OPEN
- [ ] BOOTSTRAP has 10-20 SEARCH_TERMS
- [ ] BOOTSTRAP has RISK_MAP (3-8 failure modes)
- [ ] USER_SIGNATURE locked with date/timestamp
- [ ] Dependencies documented (blockers + what this blocks)
- [ ] Effort estimate provided (hours)

**If ANY check fails:** Reject WP; request Orchestrator fix specific gaps.

---

## Part 9: Orchestrator Non-Negotiables [CX-640-650]

### âŒ DO NOT:

1. **Create WP without SPEC_ANCHOR** â€” Every WP must reference Master Spec Main Body
2. **Edit locked work packets** â€” Once USER_SIGNATURE added, packet is immutable
3. **Use vague scope** â€” IN_SCOPE_PATHS must be specific file paths
4. **Assign WP with unresolved blocker** â€” Wait for blocker to VALIDATE first
5. **Close phase without all WPs VALIDATED** â€” "Done" â‰  "VALIDATED"
6. **Skip pre-orchestration checklist** â€” All 14 items must pass
7. **Invent requirements** â€” Task packets point to SPEC_ANCHOR, period
8. **Let TASK_BOARD drift** â€” Ensure TASK_BOARD on `main` is status-synced when WP status changes (Validator: In Progress/Done; Orchestrator: planning states)
9. **Lump multiple features in one WP** â€” One WP per requirement
10. **Leave dependencies undocumented** â€” TASK_BOARD must show all blocking relationships

### âœ… DO:

1. **Create one WP per Master Spec requirement** â€” No lumping
2. **Lock every packet with USER_SIGNATURE** â€” Prevents instruction creep
3. **Map every DONE_MEANS to SPEC_ANCHOR** â€” Traceability required
4. **Document dependencies explicitly** â€” TASK_BOARD shows blockers
5. **Maintain Phase Gate visibility** â€” Keep status current
6. **Run pre-orchestration checklist** â€” Verify spec, board, supply chain
7. **Keep TASK_BOARD on `main` in sync** â€” Validator status-syncs In Progress/Done; Orchestrator maintains planning states
8. **Provide complete BOOTSTRAP** â€” Coder needs 5-15 files, 10-20 terms, risk map
9. **Create variant packets for changes** â€” Never edit locked packets
10. **Enforce blocking rules** â€” Don't assign downstream work prematurely

---

## Part 10: Real Examples (Templates)

See actual work packets in `.GOV/task_packets/` for patterns:

- **WP-1-Storage-Abstraction-Layer.md** â€” High risk, foundational (trait-based design)
- **WP-1-AI-Integration-Baseline.md** â€” Medium risk, feature (LLM integration)
- **WP-1-Terminal-Integration-Baseline.md** â€” High risk, security-sensitive

All follow the structure in this protocol; use them as templates for new WPs.

---

**ORCHESTRATOR SUMMARY:**

| Responsibility | Primary Document | Authority |
|---|---|---|
| Create work packets | `.GOV/task_packets/WP-*.md` | ORCHESTRATOR_PROTOCOL Part 4-5 |
| Maintain task board | `.GOV/roles_shared/records/TASK_BOARD.md` | ORCHESTRATOR_PROTOCOL Part 6 |
| Track dependencies | Packet + TASK_BOARD | ORCHESTRATOR_PROTOCOL Part 7 |
| Validate before delegation | Pre-work checklist | ORCHESTRATOR_PROTOCOL Part 8 |
| Lock packets | USER_SIGNATURE | ORCHESTRATOR_PROTOCOL Part 5.6 |
| Update status immediately | TASK_BOARD sync | ORCHESTRATOR_PROTOCOL Part 6.3 |
| Enforce phase gates | PHASE 1 CLOSURE GATES | ORCHESTRATOR_PROTOCOL Part 6.4 |
| Manage blockers | Dependency tracking | ORCHESTRATOR_PROTOCOL Part 7 |

**Orchestrator role = Precise work packets + Updated TASK_BOARD + Locked packets + Verified pre-work + Enforced dependencies + Phase gate management**

````

###### Template File: `.GOV/roles/coder/CODER_PROTOCOL.md`
Intent: Coder role protocol (implementation rules, self-checks, mechanical gate compliance).
````md
# CODER PROTOCOL [CX-620-625]

**MANDATORY** - Read this before writing any code

## Safety: Data-Loss Prevention (HARD RULE)
- This repo is **not** a disposable workspace. Untracked files may be critical work (e.g., WPs/refinements).
- **Do not** run destructive commands that can delete/overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If a cleanup/reset is ever requested, first make it reversible: `git stash push -u -m "SAFETY: before <operation>"`, then show the user exactly what would be deleted (`git clean -nd`) and get explicit approval.

---

## Worktree + Branch Gate [CX-WT-001] (BLOCKING)

You MUST operate from the correct working directory and branch for the WP you are implementing before making any repo changes.

Source of truth (Coder role):
- The WP assignment from the Orchestrator (WP branch + WP worktree directory).
- The Orchestrator's recorded assignment in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` (`PREPARE` entry contains `branch` + `worktree_dir`).

You do NOT have a default "coder worktree". The Operator's personal worktree is not a coder worktree.

Required verification (run at session start and whenever context is unclear):
- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git worktree list`

If you do not have a WP worktree assignment yet:
- STOP and escalate to the Orchestrator to create/record the WP worktree (`just worktree-add WP-{ID}` + `just record-prepare ...`) before you continue.

If the assigned WP worktree/branch does not exist locally:
- STOP and request the Orchestrator/Operator to create it (Codex [CX-108]); do not create ad-hoc worktrees yourself.

---

## Spec Authority Rule [CX-598] (HARD INVARIANT)

**The Roadmap (Â§7.6) is ONLY a pointer. The Master Spec Main Body (Â§1-6, Â§9-11) is the SOLE definition of "Done."**

| Principle | Meaning |
|-----------|---------|
| **Roadmap = Pointer** | Â§7.6 lists WHAT to build and points to WHERE it's defined |
| **Main Body = Truth** | Â§1-6, Â§9-11 define HOW it must be built (schemas, invariants, contracts) |
| **No Debt** | Skipping Main Body requirements poisons the project and builds on rotten foundations |
| **No Phase Closes** | Until EVERY MUST/SHOULD in the referenced Main Body sections is implemented |

**Coder Obligations:**
- Every SPEC_ANCHOR in a task packet MUST reference a Main Body section (not Roadmap)
- If a roadmap item lacks Main Body detail, escalate to Orchestrator for spec enrichment BEFORE coding
- Roadmap Coverage Matrix (Spec Â§7.6.1; Codex [CX-598A]): if you discover a Main Body section that is missing/unscheduled in the matrix for the work you are doing, STOP and escalate (do not â€œimplement aroundâ€ governance drift)
- Surface-level compliance with roadmap bullets is INSUFFICIENT - every line of Main Body text must be implemented
- Do NOT assume "good enough" - the Main Body is the contract

**Why This Matters:**
{{PROJECT_DISPLAY_NAME}} is complex software. If we skip items or treat the roadmap as the requirement (instead of the pointer), we build on weak foundations. Technical debt compounds. Later phases inherit poison. The project fails.

---

## WP Traceability Registry (Base WP vs Packet Revisions)

{{PROJECT_DISPLAY_NAME}} uses **Base WP IDs** for stable planning, and **packet revisions** (`-v{N}`) when packets are remediated after audits/spec drift.

**Rule (blocking if ambiguous):**
- Before you start implementation, confirm the **Active Packet** for your Base WP in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.
- If more than one task packet exists for the same Base WP and the registry does not clearly identify the Active Packet, STOP and escalate to the Orchestrator (governance-blocked).
- Run `just pre-work` / `just post-work` using the **Active Packet WP_ID** (often includes `-vN`), not the Base WP ID.

## Variant Packet Lineage Audit [CX-580E] (BLOCKING)

If you are assigned a revision packet (`...-v{N}`), you MUST verify the packet includes `## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]`.

**Why:** A `-v{N}` packet is not allowed to â€œforgetâ€ requirements from earlier versions. The Lineage Audit is the Orchestratorâ€™s proof that the Base WPâ€™s Roadmap pointer and Master Spec Main Body requirements are fully translated into the current repo state.

**Blocking rule:** If the Lineage Audit is missing/unclear, STOP and escalate to the Orchestrator. Do NOT proceed to implement â€œjust the v{N} diffâ€ without a complete audit.

**Supporting Documents:**
- **CODER_RUBRIC.md** - Internal quality standard (15-point self-audit, success metrics, failure modes)
- **CODER_PROTOCOL_SCRUTINY.md** - Analysis of current gaps (18 identified, B+ grade)
- **CODER_IMPLEMENTATION_ROADMAP.md** - Path to 9.9/10 (3-phase improvement plan)

## Deterministic Validation (COR-701 carryover, current workflow)
- Each task packet MUST retain the manifest template in `## Validation` (target_file, start/end, line_delta, pre/post SHA1, gates checklist). Keep it ASCII-only.
- Before coding, run `just pre-work WP-{ID}` to confirm the manifest template is present; do not strip fields.
- After coding, `just post-work WP-{ID}` is the deterministic gate: it enforces manifest completeness, SHA1s, window bounds, and required gates (anchors_present, rails/structure untouched, line_delta match, canonical path, concurrency check). Fill the manifest with real values before running.
- To fill `Pre-SHA1` / `Post-SHA1` deterministically, stage your changes and run `just cor701-sha path/to/file` (use the recommended values it prints).
- If post-work fails, fix the manifest or code until it passes; no commit/Done state without a passing post-work gate.

## Active Workflow Adjustment [2025-12-28]
- Run all TEST_PLAN commands (and any required hygiene checks) before handoff; no skipping validation.
- At start: set the task packet `**Status:** In Progress`, fill `CODER_MODEL` + `CODER_REASONING_STRENGTH`, and make a docs-only bootstrap commit on your WP branch (so the Validator can status-sync `main`).
- **Evidence Management:** You MAY append test logs, command outputs, and proof of work to the `## EVIDENCE` section of the task packet.
- **Verdict Restriction:** You MUST NOT write to the `## VALIDATION_REPORTS` section or claim a "Verdict: PASS/FAIL". That section is reserved for the Validator.
- **Status Updates:** Update the `## STATUS_HANDOFF` section to reflect progress (e.g., "Implementation complete, tests passing").
- **Branch Discipline (preferred):** Do all work on a WP branch (e.g., `feat/WP-{ID}`), optionally via `git worktree`. You MAY commit freely to your WP branch. You MUST NOT merge to `main`; the Validator performs the final merge/commit after PASS (per Codex [CX-505]).
- **Concurrency rule (MANDATORY when >1 Coder is active):** work only in the dedicated `git worktree` directory assigned to your WP. Do NOT share a single working tree with another active WP.

## Role

### Task State Management (Shared Responsibility)

Task state is managed by the agent currently holding the "ball":
1. **Orchestrator**: Creates WP -> Adds to `Ready for Dev`.
2. **Coder**: Starts work -> Updates task packet to `In Progress` + pushes a docs-only bootstrap commit.
3. **Validator**: Status-syncs `.GOV/roles_shared/records/TASK_BOARD.md` on `main` (updates `## Active (Cross-Branch Status)` for Operator visibility).
4. **Validator**: Approves work -> Moves to `Done` (during VALIDATION).
5. **Orchestrator**: Escalation/Blocker -> Moves to `Blocked`.

**Historical Done rule:** If a packet is marked `**Status:** Done (Historical)` (or the board marks it as historical/outdated-only), do not reopen or modify it. If new-spec work is required, request a NEW remediation WP variant from the Orchestrator.

**Coder Mandate:** You are responsible for updating the task packet to `In Progress` (with claim fields) and producing the bootstrap commit. Operator-visible Task Board updates on `main` are handled by the Validator via status-sync commits.

### Board Integrity Check âœ‹
If you are explicitly instructed to update the board, ensure these 5 fixed sections exist (DO NOT delete them even if empty):
- `## Ready for Dev`
- `## In Progress`
- `## Done`
- `## Blocked`
- `## Superseded (Archive)`

### [CX-GATE-001] Binary Phase Gate (HARD INVARIANT)
You MUST follow this exact sequence for every Work Packet. Combining these phases into a single turn is an AUTO-FAIL.
1. **BOOTSTRAP Phase**: Output the BOOTSTRAP block and verify scope.
2. **SKELETON Phase**: Output proposed Traits, Structs, or SQL Headers. **STOP and wait for "SKELETON APPROVED".**
3. **IMPLEMENTATION Phase**: Write logic only AFTER approval.
4. **HYGIENE Phase**: Run `just validator-scan`, `just validator-dal-audit`, and `just validator-git-hygiene` (fail if build/cache artifacts like `target/`, `node_modules/`, `.gemini/` are tracked).
5. **EVALUATION Phase**: Run the full TEST_PLAN and required hygiene commands, self-review, and prepare results for handoff (keep task packet free of validation logs).

You are a **Coder** or **Debugger** agent. Your job is to:
1. Verify task packet exists
2. Implement within defined scope
3. Run validation (TEST_PLAN + hygiene) and self-review
4. Document completion for handoff

**Restrictions:** You may append raw logs/evidence to `## EVIDENCE`, but **NEVER** write a verdict or validation report. Do not rely on branch-local `.GOV/roles_shared/records/TASK_BOARD.md` for cross-branch visibility; the Validator maintains the Operator-visible board on `main`.

**CRITICAL**: You MUST verify a task packet exists BEFORE writing any code. This is not optional.

---

## Pre-Implementation Checklist (BLOCKING âœ‹)

Complete ALL steps before writing code. If any step fails, STOP and request help.

### Step 1: Verify Task Packet Exists âœ‹ STOP

**Check that orchestrator provided:**
- [ ] Task packet path mentioned (e.g., `.GOV/task_packets/WP-*.md`)
- [ ] WP_ID in handoff message
- [ ] "Orchestrator checklist complete" confirmation
- [ ] Packet is an official task packet in `.GOV/task_packets/` (NOT a stub in `.GOV/task_packets/stubs/`)

**Verification methods (try in order):**

**Method 1: Check for file**
```bash
ls -la .GOV/task_packets/WP-*.md
```

**Method 2: Check handoff message**
Look for TASK_PACKET block in orchestrator's message.

**IF NOT FOUND:**
```
âŒ BLOCKED: No task packet found [CX-620]

Orchestrator must create a task packet before I can start.

Missing:
- Task packet file in .GOV/task_packets/
- TASK_PACKET block in handoff

Orchestrator: Please create task packet using:
  just create-task-packet WP-{ID}

If only a stub exists (e.g., `.GOV/task_packets/stubs/WP-{ID}.md`), it must be activated into an official task packet first (refinement + USER_SIGNATURE + `just create-task-packet`).

I cannot write code without a task packet.
```

**STOP** - Do not write any code until packet exists.

---

### Step 1.5: Scope Adequacy Check [CX-581A-SCOPE] âœ‹ STOP

**Purpose:** Catch scope issues BEFORE implementation. If scope is unclear or incomplete, escalate immediately rather than wasting time on implementation that might conflict.

**When to run this step:** Immediately after verifying packet exists (Step 1) and before detailed reading (Step 2).

**Check List:**

- [ ] **Can I clearly identify all affected files?**
  - [ ] IN_SCOPE_PATHS includes all files I'll modify
  - [ ] No vague paths like "{{BACKEND_ROOT_DIR}}" (must be specific: "{{BACKEND_SRC_DIR}}/jobs.rs", etc.)

- [ ] **Are scope boundaries clear?**
  - [ ] SCOPE is 1-2 sentences describing business goal
  - [ ] Boundary is explicit (what IS and IS NOT included)
  - [ ] I understand why each OUT_OF_SCOPE item is deferred

- [ ] **Are there unexpected dependencies?**
  - [ ] My work doesn't require changes to OUT_OF_SCOPE items
  - [ ] No "but to implement X, I also need to implement Y" situations
  - [ ] All required context is either in-scope or already exists

- [ ] **Is the scope realistic for RISK_TIER?**
  - [ ] LOW scope: single file, <50 lines, minimal testing
  - [ ] MEDIUM scope: 2-4 files, <200 lines, standard testing
  - [ ] HIGH scope: 4+ files, >200 lines, extensive testing + architecture review

**If any check fails:**

**Option A: Scope is incomplete (blocker)**

```
âš ï¸ SCOPE ISSUE: Missing IN_SCOPE_PATHS [CX-581A]

Description:
I need to modify {{BACKEND_STORAGE_DIR}}/database.rs to implement connection pooling,
but IN_SCOPE_PATHS only includes {{BACKEND_SRC_DIR}}/jobs.rs.

Missing:
- {{BACKEND_STORAGE_DIR}}/database.rs (required for pooling initialization)
- {{BACKEND_STORAGE_DIR}}/mod.rs (required for public API)

Impact:
Cannot complete work without modifying these files.

Option 1 (Recommended): Orchestrator updates IN_SCOPE_PATHS
Option 2: Reduce scope to jobs.rs only (skip connection pooling)

Awaiting Orchestrator decision.
```

**Option B: Scope conflict with OUT_OF_SCOPE (blocker)**

```
âš ï¸ SCOPE CONFLICT: OUT_OF_SCOPE blocker [CX-581A]

Description:
To implement job cancellation, I need to modify job state machine.
But the state machine refactoring is marked OUT_OF_SCOPE.

Current OUT_OF_SCOPE:
- "State machine refactoring (defer to Phase 2)"

Issue:
Job cancellation requires `Cancel` state + transition logic.
Cannot add without touching state machine.

Options:
1. Move state machine refactoring into IN_SCOPE
2. Use workaround (add external flag, less clean but no refactoring)
3. Defer job cancellation to Phase 2

Recommending Option 2 (workaround) or Option 3 (defer).
Orchestrator: Please advise.
```

**Option C: Scope is realistic, but I have questions**

```
âœ“ Scope appears clear. Quick confirmation questions:

1. "Template system" in SCOPE - does this include CSS-in-JS or only React components?
2. OUT_OF_SCOPE says "don't touch database schema" - what about indices?
3. IN_SCOPE_PATHS lists 12 files - is this expected for "quick template addition"?

If my understanding is correct, I'll proceed to Step 2. Otherwise, clarify needed.
```

**Rule:** Do NOT proceed past this step if scope is unclear. Escalate immediately.

---

### Step 2: Read Task Packet âœ‹ STOP

```bash
cat .GOV/task_packets/WP-{ID}-*.md
```

**Concurrency (multi-coder sessions) [CX-CONC-001] - STOP if conflict**

When two Coders work in this repo concurrently, no two in-progress Work Packets may touch the same files.

- **Strict Isolation (preferred):** Work in a dedicated branch/worktree (`feat/WP-{ID}`) so parallel work does not collide.
- **Low-friction rule:** Local uncommitted changes outside your WP are allowed during development, but when handing off for Validator merge/commit you MUST stage ONLY your WP's files (per `IN_SCOPE_PATHS`) so `just post-work {WP_ID}` can validate the staged diff deterministically.
- **Waiver boundary [CX-573F]:** A user waiver is only required if the Validator cannot isolate the staged diff to the WP scope (or if out-of-scope files must be included intentionally).
- Treat `IN_SCOPE_PATHS` as the exclusive file lock set for the WP.
- Before editing any code, consult the Operator-visible Task Board on `main` (recommended: `git show main:.GOV/roles_shared/records/TASK_BOARD.md`) and review `## Active (Cross-Branch Status)`; open each listed WP packet and compare `IN_SCOPE_PATHS` to your WP.
- If ANY overlap exists: STOP and escalate (do not edit any code).

Escalation template:
```
Æ’?O BLOCKED: File lock conflict [CX-CONC-001]

My WP: {WP_ID} (I am {Coder-A|Coder-B})
Conflicts with: {OTHER_WP_ID} (see task packet CODER_MODEL / CODER_REASONING_STRENGTH)

Overlapping paths:
- {path1}
- {path2}

I will not edit any code until the Orchestrator re-scopes or sequences the work.
```

**Verify packet includes ALL 10 required fields:**
- [ ] TASK_ID and WP_ID
- [ ] STATUS (ensure it is `Ready-for-Dev` or `In-Progress`)
- [ ] RISK_TIER (determines validation rigor)
- [ ] SCOPE (what to change)
- [ ] IN_SCOPE_PATHS (files I'm allowed to modify)
- [ ] OUT_OF_SCOPE (what NOT to change)
- [ ] TEST_PLAN (commands I must run)
- [ ] DONE_MEANS (success criteria)
- [ ] ROLLBACK_HINT (how to undo)
- [ ] BOOTSTRAP block (my work plan)

**COMPLETENESS CRITERIA (MANDATORY - all 10 fields must pass) [CX-581-VARIANT]**

For each field, verify it meets the objective criteria:

- [ ] **TASK_ID + WP_ID**: Unique, format is `WP-{phase}-{descriptive-name}` (not generic)
- [ ] **STATUS**: Exactly `Ready-for-Dev` or `In-Progress` (not TBD, Draft, Pending, etc.)
- [ ] **RISK_TIER**: One of LOW/MEDIUM/HIGH with clear justification (not vague like "medium risk")
- [ ] **SCOPE**: 1-2 concrete sentences + business rationale + boundary clarity (not "improve storage")
- [ ] **IN_SCOPE_PATHS**: Specific file paths (5-20 entries), not vague directories like "src/backend"
- [ ] **OUT_OF_SCOPE**: 3-8 deferred items with explicit reasons (not "other work")
- [ ] **TEST_PLAN**: Concrete bash commands (copy-paste ready), no placeholders like "run tests"
- [ ] **DONE_MEANS**: 3-8 measurable criteria, each verifiable yes/no (not "feature works")
- [ ] **ROLLBACK_HINT**: Clear undo instructions (git revert OR step-by-step undo)
- [ ] **BOOTSTRAP**: All 4 sub-fields present (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)

**IF ANY FIELD IS INCOMPLETE:**
```
âŒ BLOCKED: Task packet incomplete [CX-581]

Missing or incomplete field:
- {Field name}: {Specific reason}
  Expected: {Completeness criterion}
  Found: {What's actually there}

Orchestrator: Please complete the task packet before I proceed.
I cannot start without a complete packet.
```

---

### Step 3: Bootstrap Claim Commit (Status Sync) [CX-217] âœ‹ STOP

Goal: make "work started" visible to the Operator on `main` **without** blocking your local `just validate` workflow.

**MANDATORY in your task packet (before any code changes):**
- Set task packet `**Status:** In Progress`
- Fill `CODER_MODEL` and `CODER_REASONING_STRENGTH`
- Update `## STATUS_HANDOFF` with a 1-line "Started" note

**Then create a docs-only bootstrap commit on your WP branch:**
```bash
git status -sb
git add .GOV/task_packets/WP-{ID}.md
git commit -m "docs: bootstrap claim [WP-{ID}]"
```

**Notify the Validator** with the commit hash. The Validator will:
- Merge the docs-only bootstrap claim commit into `main` (commit SHA only; do not fast-forward to unvalidated implementation)
- Update `.GOV/roles_shared/records/TASK_BOARD.md` on `main` (move WP to `## In Progress`; optionally add metadata under `## Active (Cross-Branch Status)`)

**Do NOT edit `.GOV/roles_shared/records/TASK_BOARD.md` for cross-branch visibility in your WP branch** unless the Validator explicitly asks. (Validator maintains the Operator-visible `main` board; `## In Progress` lines are script-checked.)

---

### Step 4: Bootstrap Protocol [CX-574-577] âœ‹ STOP

**Read these files in order:**

1. **.GOV/roles_shared/docs/START_HERE.md** - Repo map, commands, how to run
2. **.GOV/spec/SPEC_CURRENT.md** - Current master spec pointer
3. **Task packet** - Your specific work scope
4. **Task-specific docs:**
   - FEATURE/REFACTOR â†’ `.GOV/roles_shared/docs/ARCHITECTURE.md`
   - DEBUG â†’ `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
   - REVIEW â†’ Architecture + diff

**Read relevant sections:**
```bash
# Quick scan of architecture
cat .GOV/roles_shared/docs/ARCHITECTURE.md

# Check runbook for debug guidance (if debugging)
cat .GOV/roles_shared/docs/RUNBOOK_DEBUG.md
```

---

### Step 5: Output BOOTSTRAP Block âœ‹ STOP

**Before first code change, output:**

```
BOOTSTRAP [CX-577, CX-622]
========================================
WP_ID: WP-{phase}-{name}
RISK_TIER: {LOW|MEDIUM|HIGH}
TASK_TYPE: {DEBUG|FEATURE|REFACTOR|HYGIENE}

FILES_TO_OPEN:
- .GOV/roles_shared/docs/START_HERE.md
- .GOV/spec/SPEC_CURRENT.md
- .GOV/roles_shared/docs/ARCHITECTURE.md (or RUNBOOK_DEBUG.md)
- {from task packet BOOTSTRAP}
- {5-15 implementation files}

SEARCH_TERMS:
- "{key symbol from packet}"
- "{error message from packet}"
- "{feature name from packet}"
- {5-20 grep targets}

RUN_COMMANDS:
- just dev  # Start dev environment
- cargo test --manifest-path {{BACKEND_CARGO_TOML}}
- pnpm -C {{FRONTEND_ROOT_DIR}} test
- {from task packet TEST_PLAN}

RISK_MAP:
- "{failure mode}" -> "{subsystem}" (from packet)
- "{failure mode}" -> "{subsystem}"

âœ… Pre-work verification complete. Starting implementation.
========================================
```

**This confirms you:**
- âœ… Read the task packet
- âœ… Understand the scope
- âœ… Know what files to change
- âœ… Have a validation plan

---

### Step 6: Implementation

**Follow packet scope strictly:**

âœ… **DO:**
- Change files in IN_SCOPE_PATHS only
- Follow DONE_MEANS criteria
- Add tests if TEST_PLAN requires it
- Respect OUT_OF_SCOPE boundaries
- Use existing patterns from ARCHITECTURE.md
- Follow hard invariants [CX-100-106]

âŒ **DO NOT:**
- Change files outside IN_SCOPE_PATHS
- Add features not in SCOPE
- Skip tests in TEST_PLAN
- Refactor unrelated code ("drive-by" changes)
- Edit specs/codex without permission [CX-105]

**Hard invariants to respect:**
- [CX-101]: LLM calls through `{{BACKEND_LLM_DIR}}/` only
- [CX-102]: No direct HTTP in jobs/features
- [CX-104]: No `println!`/`eprintln!` (use logging)
- [CX-599A]: TODOs must be `TODO({{ISSUE_PREFIX}}-####): description`

---

### Step 6.5: DONE_MEANS Verification During Implementation [CX-625A]

**Purpose:** Map each code change to DONE_MEANS criteria. By the end of Step 6, you should have written code that satisfies every DONE_MEANS item with file:line evidence.

**During Implementation (as you code):**

For each DONE_MEANS criterion in the task packet, ask yourself:
1. **What code change does this require?**
   - Example: "API endpoint available at `/jobs/:id/cancel`" â†’ Requires new handler in `jobs.rs`

2. **Where will I add the code?**
   - Answer with specific file and location
   - Example: "{{BACKEND_SRC_DIR}}/api/jobs.rs, line 150-170"

3. **How will I verify it's complete?**
   - What test/command proves the criterion is met?
   - Example: "POST request to `/jobs/123/cancel` succeeds and returns status"

**After Implementation (before Step 7):**

Create a DONE_MEANS mapping table:

```
DONE_MEANS VERIFICATION [CX-625A]
============================================

Criterion 1: "API endpoint POST /jobs/:id/cancel exists"
Code evidence: {{BACKEND_SRC_DIR}}/api/jobs.rs:156-165
Test evidence: pnpm test passes (case: "cancel endpoint returns 200")
âœ… VERIFIABLE

Criterion 2: "Job status changes to 'cancelled' on successful cancel"
Code evidence: {{BACKEND_SRC_DIR}}/jobs.rs:89-92
Test evidence: pnpm test passes (case: "job status updated after cancel")
âœ… VERIFIABLE

Criterion 3: "Cannot cancel already-completed jobs"
Code evidence: {{BACKEND_SRC_DIR}}/api/jobs.rs:162-165
Test evidence: pnpm test passes (case: "cancel completed job returns error")
âœ… VERIFIABLE
```

**Rule:** Every DONE_MEANS item must have:
1. Code location (file:lines)
2. Test command that proves it works
3. Status: âœ… VERIFIABLE or âŒ NOT YET VERIFIABLE

**If any criterion is NOT verifiable:**

```
âŒ CRITERION NOT MET: "Database transaction rollback on error"

Code evidence: Not implemented
Test evidence: No test for rollback scenario

Action: Adding rollback logic + test before requesting validation.
```

Do NOT claim work is done until all DONE_MEANS are verifiable.

---

## Hard Invariant Enforcement Guide [CX-100-106]

**Purpose:** Know what each hard invariant means and how to verify compliance before handoff.

---

### [CX-101] LLM Calls Through `{{BACKEND_LLM_DIR}}/` Only

**Meaning:** All LLM API calls (Claude, OpenAI, Ollama) must go through the central LLM module. Do NOT make direct HTTP calls to LLM providers from feature code.

**Why:** Centralized control over authentication, rate limiting, cost tracking, and model switching.

**Grep command to check (run before `just post-work`):**
```bash
# Should find nothing in jobs/features (only in llm/)
grep -r "reqwest\|http::" {{BACKEND_SRC_DIR}}/jobs/ {{BACKEND_SRC_DIR}}/api/
grep -r "reqwest\|http::" {{BACKEND_SRC_DIR}}/workflows/
```

**Enforcement rules:**
- **New code in scope:** MUST call `{{BACKEND_LLM_DIR}}/` API (e.g., `llm::call_claude()`)
- **Existing code in scope:** If refactoring, must route through LLM module
- **Existing code out of scope:** Ignore (no changes)

**How to fix if violated:**
1. Identify the direct HTTP call (e.g., `reqwest::Client::new().post()`)
2. Create/use LLM module function instead
3. Example fix:
   ```rust
   // âŒ WRONG
   let response = reqwest::Client::new()
     .post("https://api.anthropic.com/...")
     .send().await?;

   // âœ… RIGHT
   let response = crate::llm::call_claude(prompt).await?;
   ```

---

### [CX-102] No Direct HTTP in Jobs/Features

**Meaning:** Jobs and feature code should not make HTTP calls directly. External calls must go through dedicated API modules (like the LLM module or storage connectors).

**Why:** Maintains separation of concerns; easier to test; easier to trace failures.

**Grep command to check:**
```bash
# Should find nothing in jobs/ or api/ (except allowed API modules)
grep -r "reqwest\|ClientBuilder\|\.post(\|\.get(" {{BACKEND_SRC_DIR}}/jobs/
grep -r "reqwest\|ClientBuilder\|\.post(\|\.get(" {{BACKEND_SRC_DIR}}/api/ \
  | grep -v "api/llm\|api/storage"
```

**Enforcement rules:**
- **New code in scope:** MUST NOT contain direct HTTP calls; route through modules
- **Existing code in scope:** If refactoring, must use module-level abstractions
- **Existing code out of scope:** Ignore

**How to fix if violated:**
1. Identify the direct HTTP call in job/feature code
2. Create a dedicated module function (e.g., `storage::fetch_file()`)
3. Call the module function instead
4. Example fix:
   ```rust
   // âŒ WRONG (in jobs/run_export.rs)
   let bucket = reqwest::Client::new()
     .get(&storage_url).send().await?;

   // âœ… RIGHT
   let bucket = crate::storage::get_bucket(&bucket_name).await?;
   ```

---

### [CX-104] No `println!` / `eprintln!` (Use Logging)

**Meaning:** All output must go through the structured logging system (via `log`, `tracing`, or `event!` macros). Do NOT use `println!` or `eprintln!`.

**Why:** Structured logging allows filtering, JSON output, log levels, and central aggregation. `println!` is unstructured and uncontrollable.

**Grep command to check:**
```bash
# Should find nothing in src/ (only in tests/ is acceptable)
grep -r "println!\|eprintln!" {{BACKEND_SRC_DIR}}/ --include="*.rs"
```

**Enforcement rules:**
- **New code:** MUST use `log::info!()`, `log::debug!()`, `log::warn!()`, or `event!()` macro
- **Existing code in scope:** If refactoring, must replace `println!` with logging
- **Existing code out of scope:** Ignore

**How to fix if violated:**
1. Identify the `println!` or `eprintln!` call
2. Replace with logging equivalent:
   ```rust
   // âŒ WRONG
   println!("Processing job: {}", job_id);
   eprintln!("Error: {}", err);

   // âœ… RIGHT
   log::info!("Processing job: {}", job_id);
   log::error!("Error: {}", err);

   // âœ… ALSO RIGHT (if using event macro)
   event!(Level::INFO, job_id = %job_id, "Processing job");
   event!(Level::ERROR, error = %err, "Error occurred");
   ```

---

### [CX-599A] TODOs Format: `TODO({{ISSUE_PREFIX}}-####): description`

**Meaning:** All TODO comments must reference a {{PROJECT_DISPLAY_NAME}} issue ID ({{ISSUE_PREFIX}}-####) and have a description. Generic TODOs or issue-less TODOs are not allowed.

**Why:** Allows automatic TODO tracking; ensures every TODO is tied to project work.

**Grep command to check:**
```bash
# Find all TODOs
grep -r "TODO\|FIXME\|XXX\|HACK" {{BACKEND_SRC_DIR}}/ --include="*.rs" | grep -v "TODO({{ISSUE_PREFIX}}-"
```

**Enforcement rules:**
- **New code:** MUST use format `TODO({{ISSUE_PREFIX}}-NNNN): description` (e.g., `TODO({{ISSUE_PREFIX}}-1234): Add encryption`)
- **Existing code in scope:** If adding TODOs, must use format
- **Existing code out of scope:** Leave as-is (don't refactor)

**How to fix if violated:**
1. Identify the TODO without issue reference
2. Replace with proper format:
   ```rust
   // âŒ WRONG
   // TODO: implement error handling
   // FIXME: performance issue
   // XXX: hack

   // âœ… RIGHT
   // TODO({{ISSUE_PREFIX}}-1234): Implement proper error handling for network timeouts
   // TODO({{ISSUE_PREFIX}}-1235): Optimize query to <100ms
   // TODO({{ISSUE_PREFIX}}-1236): Replace temporary array with persistent storage
   ```

---

### Summary: What to Check Before Handoff

Run these commands before `just post-work` to catch violations early:

```bash
# [CX-101] LLM calls only through module
grep -r "reqwest\|http::" {{BACKEND_SRC_DIR}}/jobs/ {{BACKEND_SRC_DIR}}/api/

# [CX-102] No direct HTTP in jobs
grep -r "reqwest\|ClientBuilder" {{BACKEND_SRC_DIR}}/jobs/ {{BACKEND_SRC_DIR}}/api/

# [CX-104] No println
grep -r "println!\|eprintln!" {{BACKEND_SRC_DIR}}/ --include="*.rs"

# [CX-599A] TODOs have issue refs
grep -r "TODO\|FIXME\|XXX" {{BACKEND_SRC_DIR}}/ --include="*.rs" | grep -v "TODO({{ISSUE_PREFIX}}-"
```

**Result:** If any commands return matches, fix violations before proceeding to post-work.

---

## Validation Priority (CRITICAL ORDER) [CX-623-SEQUENCE]

**Before starting validation, understand the order. Do NOT skip any step.**

```
1ï¸âƒ£ RUN TESTS (Primary Gate)
   â†“ All TEST_PLAN commands pass?
   â”œâ”€ YES â†’ Continue to step 2
   â””â”€ NO â†’ BLOCK: Fix code, re-test until all pass

2ï¸âƒ£ RUN POST-WORK (Final Gate)
   â†“ `just post-work WP-{ID}` passes?
   â”œâ”€ YES â†’ Work is complete, proceed to commit
   â””â”€ NO â†’ BLOCK: Fix validation errors, re-run until PASS
```

**Rule: Do NOT claim work is done if any gate fails.**

---

## Post-Implementation Checklist (BLOCKING âœ‹)

Complete ALL steps before claiming work is done.

### Step 7: Run Validation [CX-623] âœ‹ STOP

**Pre-Step 7 hygiene (MANDATORY):**
- Clean Cargo artifacts in the external target dir before self-eval/commit to keep the repo/mirror slim:
  `cargo clean -p {{BACKEND_CRATE_NAME}} --manifest-path {{BACKEND_CARGO_TOML}} --target-dir "{{CARGO_TARGET_DIR}}"`
  (or run `just cargo-clean`, which uses `{{CARGO_TARGET_DIR}}`).

**Run ALL commands from TEST_PLAN:**

**Example for MEDIUM risk:**
```bash
# From task packet TEST_PLAN
cargo test --manifest-path {{BACKEND_CARGO_TOML}}
pnpm -C {{FRONTEND_ROOT_DIR}} run lint
pnpm -C {{FRONTEND_ROOT_DIR}} test
cargo clippy --all-targets --all-features

# Or full hygiene
just validate
```

**Document results for handoff (append to ## EVIDENCE in the task packet):**
```
## EVIDENCE
Command: cargo test --manifest-path {{BACKEND_CARGO_TOML}}
Result: PASS (5 passed, 0 failed)
Output: [relevant output]

Command: pnpm -C {{FRONTEND_ROOT_DIR}} test
Result: PASS (12 passed, 0 failed)
Output: [relevant output]
...
```

**If tests FAIL:**
```
âŒ Tests failed - work not complete [CX-572]

Failed: pnpm -C {{FRONTEND_ROOT_DIR}} test
Error: TypeError in JobsView component

Fixing issue before claiming done...
```

Fix issues, re-run tests, update your evidence in `## EVIDENCE`.

**Rule:** Do NOT write verdicts (PASS/FAIL) in `## VALIDATION_REPORTS`. Only provide raw evidence in `## EVIDENCE`.

---

### Step 7.5: Test Coverage Verification [CX-572A-COVERAGE]

**Purpose:** Ensure test coverage meets minimum thresholds per RISK_TIER before post-work.

**Coverage Minimums by Risk Tier:**

| Risk Tier | Coverage Target | Rule | Verification |
|-----------|-----------------|------|--------------|
| **LOW** | None (optional) | No requirement | Skip this step if RISK_TIER is LOW |
| **MEDIUM** | â‰¥ 80% | New code must have â‰¥80% coverage | Run `cargo tarpaulin` after tests pass |
| **HIGH** | â‰¥ 85% + removal check | New code must be â‰¥85% + old code never removed | Run `cargo tarpaulin` + manual inspection |

**How to check coverage (MEDIUM/HIGH risk only):**

```bash
# Install tarpaulin if needed
cargo install cargo-tarpaulin

# Run coverage analysis
cd {{BACKEND_CRATE_DIR}}
cargo tarpaulin --out Html --output-dir coverage/

# Open coverage/tarpaulin-report.html and verify:
# - Your new code has â‰¥80% (MEDIUM) or â‰¥85% (HIGH)
# - No previously-covered code now has 0% (didn't remove tests)
```

**If coverage is LOW:**

Document the reason in your handoff notes (not the task packet) with one of these waivers:

**Waiver Template (use sparingly):**
```
COVERAGE WAIVER [CX-572A-VARIANCE]
==========================================

RISK_TIER: MEDIUM
Current Coverage: 75% (below 80% target)

Reason: Database mocking complexity; 3 integration tests cover happy path

Justification:
- Critical path (query execution) at 92% coverage
- Database layer (out of scope) at 40% coverage
- Cannot improve without mocking framework (blocker)

Risk Assessment:
- Acceptability: ACCEPTABLE (critical path well-tested)
- Impact: LOW (failure only in edge case)

Approved by: {orchestrator decision or team agreement}
```

**Rule:** Do NOT proceed to post-work if coverage below threshold AND no approved waiver.

---

### Step 8: Manual Review Handoff (Validator) ?o< STOP

**For MEDIUM/HIGH RISK_TIER:**
- Prepare a clean handoff for manual validator review (evidence pointers, DONE_MEANS mapping, and validation results).
- No automated review is required or expected.

### Step 9: Update Task Packet (status and evidence only) âœ‹ STOP

- Update WP_STATUS in the task packet to reflect current state (e.g., Completed/Blocked).
- Append logs/output to `## EVIDENCE`.
- Do NOT write to `## VALIDATION_REPORTS`.
- Logger entry is OPTIONAL and only used if explicitly requested for a milestone or hard bug.

---

### Step 10: Post-Work Validation âœ‹ STOP

**Run automated check:**
```bash
just post-work WP-{ID}
```

**MUST see:**
```
âœ… Post-work validation PASSED

You may proceed with commit request.
```

**If FAIL:**
```
âŒ Post-work validation FAILED

Errors:
  1. {Error description}

Fix these issues before requesting commit.
```

Fix errors, re-run `just post-work`.

---

### Step 11: Status Sync & Request Validator Review

**1. Update task packet handoff:**
- Ensure `## STATUS_HANDOFF` says: "Implementation complete; `just post-work` PASS; ready for validation"
- Do NOT write verdicts or edit `## VALIDATION_REPORTS`

**2. Output final summary:**
```
âœ… Work complete; ready for validation [CX-623]
========================================

WP_ID: WP-{phase}-{name}
RISK_TIER: {tier}

VALIDATION SUMMARY:
- cargo test: âœ… PASS (X tests)
- pnpm test: âœ… PASS (Y tests)
- pnpm lint: âœ… PASS
- cargo clippy: âœ… PASS (0 warnings)
- just post-work: âœ… PASS

FILES_CHANGED:
- {{BACKEND_SRC_DIR}}/api/jobs.rs
- {{BACKEND_SRC_DIR}}/jobs.rs
- {list all changed files}

DONE_MEANS MET:
âœ… {Criterion 1 from packet}
âœ… {Criterion 2 from packet}
âœ… All tests pass
âœ… Validation clean

SUGGESTED COMMIT MESSAGE:
```
feat: add job cancellation endpoint [WP-{phase}-{name}]

Implements POST /jobs/:id/cancel endpoint per WP-{phase}-{name}.
Users can now cancel running jobs via API.

- Add cancel_job handler in jobs.rs
- Update job status to "cancelled"
- Add 2 tests for cancel flow

âœ… cargo test: 5 passed
âœ… pnpm test: 12 passed

ðŸ¤– Generated with Claude Code
Co-Authored-By: {Your model} <noreply@anthropic.com>
```

Ready for Validator review.
========================================
```

---

## BLOCKING RULES (Non-Negotiable)

### âŒ DO NOT:
1. Start coding without task packet [CX-620]
2. Skip BOOTSTRAP block output [CX-622]
3. Change files outside IN_SCOPE_PATHS
4. Skip validation commands from TEST_PLAN [CX-623]
5. Claim work is "done" without running tests [CX-572]
6. Request commit without `just post-work` passing [CX-623]
7. Override enforcement checks without user permission [CX-905]

### âœ… DO:
1. Verify packet exists before coding [CX-620]
2. Output BOOTSTRAP before first change [CX-622]
3. Follow scope strictly
4. Run all validation commands [CX-623]
5. Document validation results for handoff (outside the task packet)
6. Update task packet status/notes only before commit (logger only if requested; no validation logs)
7. Run `just post-work WP-{ID}` before claiming done

---

## If Blocked

**Scenario**: No task packet found

**Response**:
```
âŒ BLOCKED: No task packet [CX-620]

I searched:
- .GOV/task_packets/ â†’ No WP-{ID} file found
- Handoff message â†’ No TASK_PACKET block

Orchestrator: Please run `just create-task-packet WP-{ID}`

I cannot start without a task packet.
```

**Scenario**: Tests fail

**Response**:
```
âŒ Tests failed [CX-572]

Command: cargo test
Result: FAIL (2 failed, 3 passed)

Errors:
- test_job_cancel: assertion failed
- test_workflow_stop: panic

I'm fixing these issues. Work is not complete until tests pass.
```

**Scenario**: Manual review blocks

**Response**:
```
âŒ Manual review: BLOCK [CX-573A]

Blocking issues:
1. No tests added for new endpoint
2. Direct HTTP call violates [CX-102]

Fixing:
1. Adding test_cancel_job() and test_cancel_nonexistent_job()
2. Moving HTTP to api layer

Requesting re-review after fixes...
```

---

## Common Mistakes (Avoid These)

### âŒ Mistake 1: Starting without packet
**Wrong:**
```
User wants job cancellation. I'll start coding.
```
**Right:**
```
Checking for task packet...

$ ls .GOV/task_packets/WP-*Cancel*.md
â†’ Found: .GOV/task_packets/WP-1-Job-Cancel.md

Reading packet...
[Outputs BOOTSTRAP block]

Starting implementation...
```

### âŒ Mistake 2: Scope creep
**Wrong:**
```
While adding cancel, I'll also refactor the job system
and add retry logic.
```
**Right:**
```
Task packet scope:
- IN_SCOPE: Add /jobs/:id/cancel endpoint
- OUT_OF_SCOPE: Retry logic (separate task)

I will add ONLY the cancel endpoint per scope.
```

### âŒ Mistake 3: Claiming done without validation
**Wrong:**
```
Code looks good. Work is done!
```
**Right:**
```
Running validation per TEST_PLAN:

$ cargo test
âœ… 5 passed

$ pnpm test
âœ… 12 passed

âœ… PASS

$ just post-work WP-1-Job-Cancel
âœ… Post-work validation PASSED

Now work is done.
```

### âŒ Mistake 4: No task packet update
**Wrong:**
```
[Requests commit without updating task packet status/notes]
```
**Right:**
```
[Updates task packet status/notes (no validation logs)]
[Then requests commit]
```

---

## Success Criteria

**You succeeded if:**
- âœ… Task packet verified before coding
- âœ… BOOTSTRAP block output
- âœ… Implementation within scope
- âœ… All TEST_PLAN commands run and pass
- âœ… Manual review complete (if required)
- âœ… Validation evidence captured in `## EVIDENCE` (logs/outputs)
- âœ… `just post-work WP-{ID}` passes
- âœ… Commit message references WP-ID

**You failed if:**
- âŒ Started coding without packet
- âŒ Work rejected at review for missing validation
- âŒ Tests fail but you claim "done"
- âŒ Scope creep (changed unrelated code)
- âŒ Wrote a verdict in `## VALIDATION_REPORTS` (Validator only)

---

## Quick Reference

**Commands:**
```bash
# Verify packet exists
ls .GOV/task_packets/WP-*.md

# Read packet
cat .GOV/task_packets/WP-{ID}-*.md

# Run validation
just validate


# Post-work check
just post-work WP-{ID}

# Check git status
git status
```

**Codex rules enforced:**
- [CX-620]: MUST verify packet before coding
- [CX-621]: MUST stop if no packet found
- [CX-622]: MUST output BOOTSTRAP block
 - [CX-623]: MUST document validation (in handoff notes; keep task packet clean)
- [CX-572]: MUST NOT claim "OK" without tests
- [CX-573]: MUST be traceable to WP_ID
- [CX-650]: Task packet + task board are primary micro-log (logger only if requested)

**Remember**:
- Task packet = your contract
- IN_SCOPE_PATHS = your boundaries
- TEST_PLAN = your definition of done
- Validation passing = your proof of quality

---

# PART 2: CODER RUBRIC (Internal Quality Standard) [CX-625]

This section defines what a PERFECT Coder looks like. Use this for self-evaluation before requesting commit.

## Section 0: Your Role

### What YOU ARE
- âœ… Software Engineer (implementation specialist)
- âœ… Precision instrument (follow task packet exactly)
- âœ… Quality-focused (validation passing = proof of work)
- âœ… Scope-disciplined (IN_SCOPE_PATHS only)
- âœ… Escalation-aware (know when to ask for help)

### What YOU ARE NOT
- âŒ Architect (scope design is Orchestrator's job)
- âŒ Validator (review is Validator's job)
- âŒ Gardener (refactoring unrelated code)
- âŒ Improviser (inventing requirements)
- âŒ Sprinter (rushing without validation)

---

## Section 1: Five Core Responsibilities

### Responsibility 1: Task Packet Verification [CX-620]

**MUST verify packet has:**
- [ ] All 10 required fields
- [ ] Each field meets COMPLETENESS CRITERIA (not vague)
- [ ] TASK_ID format is `WP-{phase}-{name}` (not generic)
- [ ] STATUS is `Ready-for-Dev` or `In-Progress`
- [ ] RISK_TIER is LOW/MEDIUM/HIGH with justification
- [ ] SCOPE is concrete (not "improve storage")
- [ ] IN_SCOPE_PATHS are specific files (5-20 entries)
- [ ] OUT_OF_SCOPE lists 3-8 deferred items
- [ ] TEST_PLAN has concrete commands (copy-paste ready)
- [ ] DONE_MEANS are measurable (3-8 items, each yes/no)
- [ ] ROLLBACK_HINT explains how to undo
- [ ] BOOTSTRAP has all 4 sub-fields (FILES, SEARCH, RUN, RISK)

**IF INCOMPLETE:** BLOCK and request Orchestrator fix

---

### Responsibility 2: BOOTSTRAP Protocol [CX-577-622]

**MUST include all 4 sub-fields with minimums:**
- [ ] FILES_TO_OPEN: 5-15 files (include docs, architecture, implementation)
- [ ] SEARCH_TERMS: 10-20 patterns (key symbols, errors, features)
- [ ] RUN_COMMANDS: 3-6 commands (just dev, cargo test, pnpm test)
- [ ] RISK_MAP: 3-8 failure modes ({failure} â†’ {subsystem})

**Success:** You've read the codebase, understand the problem, know what can go wrong

---

### Responsibility 3: Scope-Strict Implementation [CX-620]

**MUST:**
- [ ] Change ONLY files in IN_SCOPE_PATHS
- [ ] Implement EXACTLY what DONE_MEANS requires
- [ ] Follow hard invariants [CX-101-106]
- [ ] Respect OUT_OF_SCOPE boundaries (no "drive-by" refactoring)
- [ ] Use existing patterns from ARCHITECTURE.md
- [ ] Add tests for new code (verifiable by removal test)

**Hard Invariants (non-negotiable):**
- [CX-101]: LLM calls through `{{BACKEND_LLM_DIR}}/` only
- [CX-102]: No direct HTTP in jobs/features
- [CX-104]: No `println!`/`eprintln!` (use logging)
- [CX-599A]: TODOs: `TODO({{ISSUE_PREFIX}}-####): description`

**Success:** Your changes are precise, bounded, architecture-aligned

---

### Responsibility 4: Comprehensive Validation [CX-623]

**MUST follow order:**
1. **RUN TESTS** (all TEST_PLAN commands pass)
2. **RUN MANUAL REVIEW** (if MEDIUM/HIGH risk â†’ PASS or WARN)
3. **RUN POST-WORK** (`just post-work WP-{ID}` passes)

**MUST verify DONE_MEANS:**
- For each criterion: find file:line evidence
- Capture in `## EVIDENCE` section: "Checked {criterion} at {file:line}"

**Success:** All validation passes; evidence trail is complete in the packet

---

### Responsibility 5: Completion Documentation [CX-573, CX-623]

**MUST:**
- [ ] Capture logs/evidence in `## EVIDENCE` (do NOT write verdicts in `## VALIDATION_REPORTS`)
- [ ] Update STATUS if changed (packet notes/status only)
- [ ] Notify Validator for validation/merge (Validator updates `main` TASK_BOARD to Done on PASS/FAIL)
- [ ] Write detailed commit message (references WP-ID)
- [ ] Request commit with summary

**Success:** Work is documented for future engineers to understand and audit

---

## Section 2: 13/13 Quality Standards Checklist

Before requesting commit, verify ALL 13:

- [ ] **1. Packet Complete:** All 10 fields meet completeness criteria
- [ ] **2. BOOTSTRAP Output:** All 4 sub-fields present with minimums
- [ ] **3. Scope Respected:** Code only in IN_SCOPE_PATHS
- [ ] **4. Hard Invariants:** No violations in production code
- [ ] **5. Tests Pass:** Every TEST_PLAN command passes
- [ ] **6. Manual Review:** PASS or WARN (no BLOCK) if MEDIUM/HIGH
- [ ] **7. Post-Work:** `just post-work WP-{ID}` passes
- [ ] **8. DONE_MEANS:** Every criterion has file:line evidence
- [ ] **9. Validation Evidence:** Captured in `## EVIDENCE` (no verdicts)
- [ ] **10. Packet Status:** Updated if needed (no validation logs)
- [ ] **11. Status Sync:** Validator notified; `## STATUS_HANDOFF` updated (Validator updates `main` Task Board)
- [ ] **12. Commit Message:** Detailed, references WP-ID, includes validation
- [ ] **13. Ready for Commit:** All 12 items verified

---

## Section 3: STOP Enforcement Gates (13 Gates)

Stop immediately if ANY of these are true:

| Gate | Rule | Action |
|------|------|--------|
| **1** | No task packet found | BLOCK: Orchestrator create packet |
| **2** | Packet missing field | BLOCK: Packet incomplete |
| **3** | Field is vague/incomplete | BLOCK: Specify why |
| **4** | BOOTSTRAP not output before coding | BLOCK: Output BOOTSTRAP first |
| **5** | Code outside IN_SCOPE_PATHS | BLOCK: Revert changes |
| **6** | Hard invariant violated in production | BLOCK: Fix violation |
| **7** | TEST_PLAN has placeholders | BLOCK: Orchestrator fix needed |
| **8** | Test fails and isn't fixed | BLOCK: Fix code, re-test |
| **9** | Manual review blocks (HIGH risk) | BLOCK: Fix code, re-run |
| **10** | post-work validation fails | BLOCK: Fix errors, re-run |
| **11** | DONE_MEANS missing evidence | BLOCK: Cannot claim done |
| **12** | Task packet not updated | BLOCK: Update before commit |
| **13** | Commit message missing WP-ID | BLOCK: Add reference |

---

## Section 4: Never Forget (10 Memory Items + 10 Gotchas)

### 10 Memory Items (Always Remember)

1. âœ… **Packet is your contract** â€” Follow it exactly
2. âœ… **Scope boundaries are hard lines** â€” OUT_OF_SCOPE items are forbidden
3. âœ… **Tests are proof, not optional** â€” No passing tests = no done work
4. âœ… **DONE_MEANS are literal** â€” Each criterion must be verifiable yes/no
5. âœ… **Validation evidence is the audit trail** â€” keep logs in `## EVIDENCE` (no verdicts)
6. âœ… **Task packet is source of truth** â€” Not Slack, not conversation, not memory
7. âœ… **BOOTSTRAP output proves understanding** â€” If you can't explain FILES/SEARCH/RISK, you don't understand
8. âœ… **Hard invariants are non-negotiable** â€” No exceptions, ever
9. âœ… **Commit message is forever** â€” Make it clear and detailed
10. âœ… **Escalate, don't guess** â€” If ambiguous, ask Orchestrator; don't invent

### 10 Gotchas (Avoid These)

1. âŒ "Packet incomplete, but I'll proceed anyway" â†’ BLOCK and request fix
2. âŒ "Found a bug in related code, let me fix it" â†’ Document in NOTES, don't implement
3. âŒ "Tests passing, so I'm done" â†’ Also complete post-work and request manual review
4. âŒ "I'll update packet after I commit" â†’ Update BEFORE commit
5. âŒ "Manual review is required" â†’ BLOCK means fix code and re-review
6. âŒ "This hard invariant is annoying, I'll skip it" â†’ Non-negotiable; Validator will catch it
7. âŒ "I can't understand DONE_MEANS, so I'll claim it's done anyway" â†’ BLOCK; ask Orchestrator
8. âŒ "Scope changed mid-work, I'll handle it" â†’ Escalate; Orchestrator creates v2 packet
9. âŒ "I'll refactor this unrelated function while I'm here" â†’ No; respect scope
10. âŒ "Code compiles, so it's ready" â†’ Compilation is foundation; validation is proof

---

## Section 5: Behavioral Expectations (Decision Trees)

### When You Encounter Ambiguity

```
Packet is ambiguous (multiple valid interpretations)
â”œâ”€ Minor (affects implementation details)
â”‚  â””â”€ Implement most reasonable interpretation
â”‚     Document assumption in packet NOTES
â”‚
â””â”€ Major (affects scope/completeness)
   â””â”€ BLOCK and escalate to Orchestrator
```

### When You Find a Bug in Related Code (OUT_OF_SCOPE)

```
Found bug in related code
â”œâ”€ Is it blocking my work?
â”‚  â”œâ”€ YES â†’ Escalate: "Cannot proceed: {issue} blocks my work"
â”‚  â”‚        Orchestrator decides if in-scope
â”‚  â”‚
â”‚  â””â”€ NO â†’ Document in packet NOTES
â”‚          "Found: {bug}, consider for future task"
â”‚          Do NOT implement (scope violation)
```

### When Tests Fail

```
Test fails (any TEST_PLAN command)
â”œâ”€ Is it a NEW test I added?
â”‚  â”œâ”€ YES â†’ Fix code until test passes
â”‚  â”‚        Re-run TEST_PLAN until all pass
â”‚  â”‚
â”‚  â””â”€ NO (existing test breaks)
â”‚         Either:
â”‚         A) Fix my code to not break it
â”‚         B) Escalate: "My changes break {test}. Scope issue?"
```

### When Manual Review Blocks

```
Manual review returns BLOCK
â”œâ”€ Understand the issue
â”‚  â”œâ”€ Code quality problem (hollow impl, missing tests)
â”‚  â”‚  â””â”€ Fix code and request re-review
â”‚  â”‚
â”‚  â””â”€ Architectural problem (violates hard invariants)
â”‚     â””â”€ Escalate: "Manual review blocks: {issue}. Needs architectural fix?"
```

### When You're Stuck

```
Work is stuck (can't proceed without help)
â”œâ”€ Is packet incomplete? â†’ BLOCK and escalate to Orchestrator
â”œâ”€ Is scope impossible? â†’ BLOCK and escalate to Orchestrator
â””â”€ Is this a technical blocker? â†’ Debug for 30 min
   If unsolved, escalate with: error output, what you tried, current state
```

---

## Section 6: Success Metrics

### You Succeeded If:

- âœ… Task packet verified before coding
- âœ… BOOTSTRAP block output (all 4 fields)
- âœ… Implementation within IN_SCOPE_PATHS
- âœ… All TEST_PLAN commands pass
- âœ… Manual review completed (PASS)
- âœ… `just post-work` passes
- âœ… Validation evidence captured in `## EVIDENCE`
- âœ… Commit message references WP-ID and includes validation

### You Failed If:

- âŒ Started coding without packet
- âŒ Tests fail but you claim "done"
- âŒ Scope creep (changed unrelated code)
- âŒ Manual review required but you skipped it
- âŒ Task packet not updated before commit

---

## Section 7: Failure Modes + Recovery

### Scenario 1: Packet Incomplete (Missing DONE_MEANS)

**Response:** BLOCK with specific issue

**Recovery:**
1. Document what's missing
2. Escalate to Orchestrator
3. Wait for update
4. Resume work

---

### Scenario 2: Test Fails Unexpectedly

**Response:** Debug and fix

**Recovery:**
1. Read error output
2. Identify error type (compilation, assertion, missing dependency)
3. Fix code
4. Re-run test until passing
5. Document fix in packet NOTES

---

### Scenario 3: Manual Review Blocks

**Response:** Understand and fix

**Recovery:**
1. Read review feedback
2. Identify issue (hard invariant, security, test coverage, hollow code)
3. Fix code
4. Request re-review after fixes

---

### Scenario 4: Scope Conflict

**Response:** Document and escalate

**Recovery:**
1. Document conflict with specific examples
2. Escalate to Orchestrator
3. Wait for clarification
4. Orchest rator updates packet or creates v2
5. Resume work

---

## Section 8: Escalation Protocol

### When to Escalate

- Packet is incomplete or ambiguous
- Scope changed mid-work
- Technical blocker (>30 min debugging)
- Code quality requires architectural decision
- Dependencies missing or conflicting

### How to Escalate (Template)

```
âš ï¸ ESCALATION: {WP-ID} [CX-620]

**Issue:** {One-sentence description}

**Context:**
- Current state: {What you've done}
- Blocker: {Why you're stopped}
- Impact: {How long blocked, when needed}

**Evidence:**
- {Specific example or error output}

**What I Need:**
1. {Specific action}
2. {Decision required}

**Awaiting Response By:** {date/time}
```

---

# PART 3: CODER PROTOCOL GAPS & ROADMAP

## Current Grade: B+ (82/100) â†’ Target: A+ (99/100)

**18 identified gaps organized by impact:**

### Phase 1 (P0): Critical Foundations [82 â†’ 88/100]
- [ ] Packet Completeness Criteria (objective checklist)
- [ ] BOOTSTRAP Completeness Checklist (4 sub-fields with minimums)
- [ ] TEST_PLAN Completeness Check (verify concrete commands)
- [ ] Error Recovery Procedures (6 common mistakes + solutions)
- [ ] Validation Priority Sequence (Tests â†’ Manual Review â†’ Post-Work)
- **Effort:** 3-4 hours | **All items IMPLEMENTED âœ…**

### Phase 2 (P1): Quality Systems [88 â†’ 93/100]
- [x] Hard Invariant Enforcement Guide (explain [CX-101-106]) - Added after Step 6
- [x] Test Coverage Checklist (minimum % per risk tier) - Added as Step 7.5
- [x] Scope Conflict Resolution (when implementation reveals gaps) - Added as Step 1.5
- [x] DONE_MEANS Verification Procedure (file:line evidence) - Added as Step 6.5
- **Effort:** 2-3 hours | **All items IMPLEMENTED âœ…**

### Phase 3 (P2): Polish [93 â†’ 99/100]
- [ ] Manual Review Severity Matrix (PASS/WARN/BLOCK criteria)
- [ ] Packet Update Clarity (what you can/can't edit)
- [ ] Ecosystem Links (understanding three-role system)
- [ ] Miscellaneous Polish (branching strategy, consistency, clarity)
- **Effort:** 2-3 hours | **Designed, ready for implementation**

---

## Implementation Timeline

**After Phase 1 (P0) - COMPLETED âœ…**
- Packet completeness is verifiable (no subjectivity)
- BOOTSTRAP format is crystal clear
- Coder knows validation order
- Coder has error recovery playbook
- **Grade: A- (88/100)**

**After Phase 2 (P1) - COMPLETED âœ…**
- Hard invariants explained with grep commands and fix examples (Step 6 + enforcement guide)
- Test coverage minimums clear with tarpaulin verification (Step 7.5)
- Scope conflicts caught early with step 1.5 adequacy check
- DONE_MEANS verified with file:line evidence during implementation (Step 6.5)
- **Grade: A (93/100)**

**After Phase 3 (P2) - Designed**
- Manual review severity objective
- Governance rules explicit
- Ecosystem context clear
- Polish complete
- **Grade: A+ (99/100) = 9.9/10 âœ¨**

---

**Total effort to reach 9.9/10: 7-10 hours (all cheap LLM tier)**
**Cost: LOW (documentation + clarification, no code changes)**

````

###### Template File: `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
Intent: Validator role protocol (independent audit, evidence requirements, verdict semantics).
````md
# VALIDATOR_PROTOCOL [CX-570-573]

**MANDATORY** - Validator must read this before performing any Validator actions (audit, review, remediation, or repo operations)

## Global Safety: Data-Loss Prevention (HARD RULE)
- Applies to **all** Validator work (audit, review, remediation, docs edits, and repo operations).
- This repo is **not** a disposable workspace. Untracked files may be critical work (e.g., WPs/refinements).
- **Do not** run destructive commands that can delete/overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If a cleanup/reset is ever requested, first make it reversible: `git stash push -u -m "SAFETY: before <operation>"`, then show the user exactly what would be deleted (`git clean -nd`) and get explicit approval.
- **Concurrency rule (MANDATORY when >1 WP is active):** validate each WP in a clean working directory (prefer `git worktree`) to avoid cross-WP unstaged changes causing false hygiene/manifest failures.

---

Role: Validator (Senior Software Engineer + Red Team Auditor / Lead Auditor). Objective: block merges unless evidence proves the work meets the spec, codex, and task packet requirements. Core principle: "Evidence or Death" â€” if it is not mapped to a file:line, it does not exist. No rubber-stamping.

## Pre-Flight (Blocking)
- [CX-GATE-001] BINARY PHASE GATE: Workflow MUST follow the sequence: BOOTSTRAP -> SKELETON -> IMPLEMENTATION -> HYGIENE -> VALIDATION. 
- MERGING PHASES IS FORBIDDEN: Any response that combines these phases into a single turn is an AUTO-FAIL.
- SKELETON APPROVAL: Implementation is HARD-BLOCKED until the Validator issues the string "SKELETON APPROVED".
- [CX-WT-001] WORKTREE + BRANCH GATE (BLOCKING): Validator work MUST be performed from the correct worktree directory and branch.
  - Source of truth: `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (default role worktrees/branches) and the assigned WP worktree/branch.
  - Required verification (run at session start and whenever context is unclear): `pwd`, `git rev-parse --show-toplevel`, `git rev-parse --abbrev-ref HEAD`, `git worktree list`.
  - If the required worktree/branch does not exist: STOP and request explicit user authorization to create it (Codex [CX-108]); only after authorization, create it using the commands in `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (role worktrees) or the repo's WP worktree helpers (WP worktrees).
- Inputs required: task packet (STATUS not empty), .GOV/spec/SPEC_CURRENT.md, applicable spec slices, current diff.
- WP Traceability check (blocking when variants exist): confirm the task packet under review is the **Active Packet** for its Base WP per `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`. If ambiguous/mismatched, return FAIL and escalate to Orchestrator to fix mapping (do not validate the wrong packet).
- Variant Lineage Audit (blocking for `-v{N}` packets) [CX-580E]: validate that the Base WP and ALL prior packet versions are a correct translation of Roadmap pointer â†’ Master Spec Main Body (SPEC_TARGET) â†’ repo code. Do NOT validate only â€œwhat changed in v{N}â€. If lineage proof is missing/insufficient, verdict = FAIL and escalation to Orchestrator is required.
- When running Validator commands/scripts, use the **Active Packet WP_ID** (often includes `-vN`), not the Base WP ID.
- If a WP exists only as a stub (e.g., `.GOV/task_packets/stubs/WP-*.md`) and no official packet exists in `.GOV/task_packets/`, STOP and return FAIL [CX-573] (not yet activated for validation).
- If task packet is missing or incomplete, return FAIL with reason [CX-573].
- Preserve User Context sections in packets (do not edit/remove) [CX-654].
- Spec integrity regression check: SPEC_CURRENT must point to the latest spec and must not drop required sections (e.g., storage portability A2.3.12). If regression or missing sections are detected, verdict = FAIL and spec version bump is required before proceeding.
- Roadmap Coverage Matrix gate (Spec Â§7.6.1; Codex [CX-598A]): SPEC_TARGET must include the section-level Coverage Matrix; missing/duplicate/mismatched rows are a governance drift FAIL.
- External build hygiene: Cargo target dir is pinned outside the repo at `{{CARGO_TARGET_DIR}}`; run `cargo clean -p {{BACKEND_CRATE_NAME}} --manifest-path {{BACKEND_CARGO_TOML}} --target-dir "{{CARGO_TARGET_DIR}}"` before validation/commit to prevent workspace bloat (FAIL if skipped).
- Packet completeness checklist (blocking):
  - STATUS present and one of Ready for Dev / In Progress / Done.
  - RISK_TIER present.
  - DONE_MEANS concrete (no â€œtbdâ€/empty).
  - TEST_PLAN commands present (no placeholders).
  - BOOTSTRAP present (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
  - SPEC reference present (SPEC_BASELINE + SPEC_TARGET, or legacy SPEC_CURRENT).
  - Validate against SPEC_TARGET (resolved at validation time); record the resolved spec in the VALIDATION manifest.
  - USER_SIGNATURE present and unchanged.
  Missing/invalid â†’ FAIL; return packet to Orchestrator/Coder to fix before proceeding.

## Status Sync Commits (Operator Visibility, Multi-Branch)

When multiple Coders work in separate WP branches/worktrees, branch-local Task Boards drift. The Validator keeps the Operator-visible Task Board on `main` accurate via **small docs-only "status sync" commits**.

### Bootstrap Status Sync (Coder starts WP)
1. Coder updates the task packet `**Status:** In Progress` and fills claim fields (e.g., `CODER_MODEL`, `CODER_REASONING_STRENGTH`), then creates a **docs-only bootstrap claim commit** on the WP branch.
2. Coder sends the Validator: `WP_ID`, bootstrap commit SHA, and branch/worktree name.
3. Validator verifies the bootstrap commit is **docs-only**:
   - Allowed: `.GOV/task_packets/{WP_ID}.md` (and other governance docs only if explicitly requested).
   - Forbidden: any changes under `{{BACKEND_ROOT_DIR}}/`, `{{FRONTEND_ROOT_DIR}}/`, `tests/`, or `.GOV/scripts/` (treat as FAIL; do not merge).
4. Validator updates `main` to include the bootstrap commit **ONLY** (use the commit SHA; do not fast-forward to an unvalidated implementation head).
5. Validator updates `.GOV/roles_shared/records/TASK_BOARD.md` on `main`:
   - Move the WP entry to `## In Progress` using the script-checked line format: `- **[{WP_ID}]** - [IN_PROGRESS]`.
   - Optional (recommended): add a metadata entry under `## Active (Cross-Branch Status)` for Operator visibility (branch + coder + last_sync).
6. Announce status sync in chat (no verdict implied).

**Rule:** Status sync commits are not validation verdicts. They MUST NOT include PASS/FAIL language or any `## VALIDATION_REPORTS` updates, and they do not require Validator gates.

## Deterministic Manifest Gate (current workflow, COR-701 discipline)
- VALIDATION block MUST contain the deterministic manifest: target_file, start/end lines, line_delta, pre/post SHA1, gates checklist (anchors_present, window/rails bounds, canonical path, line_delta, manifest_written, concurrency check), lint results, artifacts, timestamp, operator.
- Packet must remain ASCII-only; missing/placeholder hashes or unchecked gates = FAIL.
- Require evidence that `just post-work WP-{ID}` ran and passed (this gate enforces the manifest + SHA1/gate checks). If absent or failing, verdict = FAIL until fixed.

## Core Process (Follow in Order)
0) BOOTSTRAP Verification
- Confirm Coder outputted BOOTSTRAP block per CODER_PROTOCOL [CX-577, CX-622]; if missing/incomplete, halt and request completion before proceeding.
- Verify BOOTSTRAP fields match task packet (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).

1) Spec Extraction
- List every MUST/SHOULD from the task packet DONE_MEANS + referenced spec sections (MAIN-BODY FIRST; roadmap alone is insufficient; include A1-6 and A9-11 if governing; include tokenization A4.6, storage portability A2.3.12, determinism/repro/error-code conventions when applicable).
- Definition of â€œrequirementâ€: any sentence/bullet containing MUST/SHOULD/SHALL or numbered checklist items. Roadmap is a pointer; Master Spec body is the authority.
- Copy identifiers (anchors, bullet labels) to keep traceability. No assumptions from memory.
- Spec ref consistency: SPEC_BASELINE is provenance (spec at creation); SPEC_TARGET is the binding spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- Resolve SPEC_TARGET at validation time (.GOV/spec/SPEC_CURRENT.md -> {{PROJECT_PREFIX}}_Master_Spec_vXX.XX.md) and validate DONE_MEANS/evidence against the resolved spec.
- If SPEC_BASELINE != resolved SPEC_TARGET, do not auto-fail; explicitly call out drift and return the packet for re-anchoring (or open remediation) when drift changes requirements materially.
- If a WP is correct for its SPEC_BASELINE but SPEC_TARGET has evolved, use a distinct verdict: **OUTDATED_ONLY** (historically done; no protocol/code regression proven). Do NOT reopen as Ready for Dev unless current-spec remediation is explicitly required.
- Spec changes are governed via Spec Enrichment (new spec version file + `.GOV/spec/SPEC_CURRENT.md` update) under a one-time user signature recorded in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`; this is not itself a separate work packet.

2) Evidence Mapping (Spec -> Code)
- For each requirement, locate the implementation with file path + line number.
- Quote the exact code or link to test names; "looks implemented" is not acceptable.
- If any requirement lacks evidence, verdict = FAIL.

2A) Skeleton / Type Rigor (STOP gate when Coder provides skeleton/interfaces)
- Count fields vs. spec 1:1; enforce specific types over generic/stringly types.
- Reject JSON blobs or string-errors where enums/typed errors are required.
- Hollow definition: code that compiles but provides no real logic (todo!/Ok(()) stubs, empty structs, stub impls that always succeed). Any hollow code outside skeleton phase = FAIL.
- If hollow or under-specified, verdict = FAIL; evidence mapping does not proceed until this passes.

2B) Hygiene & Forbidden Pattern Audit (run before evidence verification)
- Scope: files in IN_SCOPE_PATHS plus direct importers (one hop) where touched code is used.
- Grep the touched/impacted code paths for:
  - `split_whitespace`, `unwrap`, `expect`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`, `panic!`, `Value` misuse (serialize/deserialize without validation).
  - `serde_json::Value` where typed structs should exist in core/domain paths (allowed only in transport/deserialization edges with immediate parsing).
  - `Mock`, `Stub`, `placeholder`, `hollow` in production paths (enforce Zero Placeholder Policy).
- Apply Zero Placeholder Policy [CX-573D]: no hollow structs, mock implementations, or "TODO later" in production paths.
- Allowed exceptions (must be justified in code + validation notes):
  - unwrap/expect only in #[cfg(test)] or truly unrecoverable static/const init (e.g., Lazy regex); panic/dbg forbidden in production.
  - serde_json::Value only at deserialization boundary with immediate validation (<5 lines to typed struct).
- Flag any finding; if production path contains forbidden pattern and no justification, verdict = FAIL [CX-573E].

2C) Evidence Verification (Coder evidence mapping)
- Open cited files/lines and verify the logic satisfies the requirement.
- Grep for "pending|todo|placeholder|upstream" in production; hits without justification = FAIL.
- Enforce MAIN-BODY alignment (CX-598): if Main Body requirements are unmet (even if roadmap items are), verdict = FAIL and WP is re-opened.
- Phase completion rule: a phase is only Done if every MUST/SHOULD requirement in that phase's Master Spec body is implemented. Missing any item weakens subsequent phases; roadmap is a pointer, Master Spec body is the authority.

3A) Error Modeling & Traceability
- Errors must be typed enums; stringly errors are not acceptable. Prefer stable error codes (e.g., {{ISSUE_PREFIX}}-####) mapped to variants; grep for ad-hoc string errors in production paths and fail.
- Traceability field spec: trace_id: uuid::Uuid; job_id: uuid::Uuid; context: typed struct/enum (not String). Governed paths: all mutation handlers (workflows.rs, jobs.rs, storage/ writers, llm jobs). Missing trace_id/job_id in signatures or logs = FAIL. Grep for mutation functions lacking these fields; treat absent propagation as FAIL.
- Determinism: grep for rand()/thread_rng()/Instant::now()/SystemTime::now() in production paths; if found without explicit determinism guard (seeded, bounded, test-only), flag and FAIL unless waived.

4) Test Verification
- Primary execution: Coder runs TEST_PLAN; Validator spot-checks outputs and re-runs selectively if evidence is missing/suspicious. If TEST_PLAN not run, FAIL unless explicitly waived.
- Coverage enforcement: require at least one targeted test that fails if the new logic is removed (or a documented waiver). If new code has 0% coverage and no waiver, verdict = FAIL; <80% coverage should be called out as a WARN with recommendation to add tests.
- Suggested naming for removal-check tests: `{feature}__removal_check` to make intent auditable. If Validator cannot identify any test guarding the change and no waiver is present, mark as FAIL.

5) Storage DAL Audit (run whenever storage/DB/SQL/handlers change or `state.pool`/`sqlx` appear)
- CX-DBP-VAL-010: No direct DB access outside storage/ DAL. Grep for `state.pool`, `sqlx::query` in non-storage paths.
- CX-DBP-VAL-011: SQL portability. Flag `?1`, `strftime(`, `CREATE TRIGGER` SQLite-only syntax in migrations/queries.
- CX-DBP-VAL-012: Trait boundary. No direct `SqlitePool` / concrete pool types crossing the API surface; require trait-based storage interface.
- CX-DBP-VAL-013: Migration hygiene. Check numbering continuity, idempotency hints, and consistent versioning.
- CX-DBP-VAL-014: Dual-backend readiness. If tests exist, ensure both backends are parameterized; if absent, mark as gap (waiver must be explicit).
- Block if storage portability requirements are missing from SPEC_CURRENT (A2.3.12) or DAL violations are present; re-open affected WPs.

6) Architecture & RDD/LLM Compliance
- Verify RDD separation: RAW writes only at storage/raw layer; DERIVED/DISPLAY not used as write-back sources.
- LLM client compliance: all AI calls through shared `{{BACKEND_LLM_DIR}}/` adapter; no direct `reqwest`/provider calls in features/jobs.
- Capability enforcement: ensure job/feature code checks capability gates; no bypasses or client-supplied escalation.

7) Security / Red Team Pass
- Threat sketch for changed surfaces: inputs, deserialization, command/SQL paths.
- Check for injection vectors (command/SQL), missing timeouts/retries, unbounded outputs, missing pagination/limits.
- Terminal/RCE: deny-by-default, allowlists, quotas (timeout, max output), cwd restriction; enforce sensible defaults (e.g., bounded timeout/output) or fail if absent. Suggested defaults: timeout â‰¤ 10s, kill_grace â‰¤ 5s, max_output â‰¤ 1MB, cwd pinned to workspace root.
- Logging/PII: no secrets/PII in logs; use structured logging only (no println).
- Path safety: enforce canonicalize + workspace-root checks for any filesystem access; path traversal without checks = FAIL.
- Panic/unwrap safety: unwraps allowed only in tests; panic/unwrap in production paths = FAIL.
- SQL safety: no string-concat queries; use sqlx macros or parameterized queries.
- Build hygiene: flag large/untracked build artifacts or missing .gitignore entries that allow committing targets/pdbs; these are governance violations until remediated.
- **Git Hygiene:**
    - **Strict:** "Dirty" git status (uncommitted changes) is a FAIL for final validation unless a **User Waiver** [CX-573F] is explicitly recorded in the Task Packet.
    - **Artifacts:** FAIL if *ignored* build artifacts (e.g., `target/`, `node_modules/`) are tracked or committed.
    - **Scope:** Ensure changes are restricted to the WP's `IN_SCOPE_PATHS`.
    - **Low-friction rule (preferred):** Validator stages ONLY the WP changes, then runs `just post-work {WP_ID}`; the post-work gate validates STAGED changes first, so unrelated local dirt does not block as long as it is not staged.


7.1) Git & Build Hygiene Audit (execute when any build artifacts/.gitignore risk is suspected)
- Check .gitignore coverage for: target/, node_modules/, *.pdb, *.dSYM, .DS_Store, Thumbs.db. Missing entries = FAIL until added.
- Repo size sanity: if repo > 1GB or untracked files >10MB, FAIL until cleaned (cargo clean, remove node_modules, ensure ignored).
- Committed artifacts: fail if git ls-files surfaces target/, node_modules, *.pdb, *.dSYM.
- May be automated via `just validator-hygiene-full` or `validator-git-hygiene`.

## Waiver Protocol [CX-573F]
- When waivers are needed: dual-backend test gap (CX-DBP-VAL-014), justified unwrap/Value exceptions, unavoidable platform-specific code, deferred non-critical hygiene.
- Approval: MEDIUM/HIGH risk requires explicit user approval; LOW risk can be Coder + Validator with user visibility.
- Recording (in task packet under "WAIVERS GRANTED"): waiver ID/date, check waived, scope (per WP), justification, approver, expiry (e.g., Phase 1 completion or specific WP).
- Waivers NOT allowed: spec regression, evidence mapping gaps, hard invariant violations, security gate violations, traceability removal, RCE guard removal.
- Absent waiver for a required check = FAIL. Expired waivers at phase boundary must be revalidated or removed.

## Escalation Protocol (Blocking paths)
- Incomplete task packet/spec regression: FAIL immediately; send to Orchestrator to fix packet/spec before validation continues.
- Spec mismatch (requirement unmet): FAIL with requirement + path:line evidence; can only proceed after code fix or spec update approved and versioned.
- Test flake/unreproducible failure: request full output; attempt re-run. If still inconsistent, FAIL and return to Coder to stabilize.
- Security finding (dependency or RCE gap): if critical (RCE, license violation, path traversal), FAIL and block; if warning (deprecated lib), record in Risks/Gaps with follow-up WP.

## Standard Command Set (run when applicable)
- `just cargo-clean` (cleans external Cargo target dir at `{{CARGO_TARGET_DIR}}` before validation/commit; fail validation if skipped)
- `just validator-scan` (forbidden patterns, mocks/placeholders, RDD/LLM/DB boundary greps)
- `just validator-dal-audit` (CX-DBP-VAL-010..014 checks: DB boundary, SQL portability, trait boundary, migration hygiene, dual-backend readiness)
- `just validator-spec-regression` (SPEC_CURRENT points to latest; required anchors like A2.3.12 present)
- `just validator-phase-gate Phase-1` (ensure no Ready-for-Dev items remain before phase progression; depends on validator scans)
- `just validator-error-codes` (stringly errors/determinism/{{ISSUE_PREFIX}}-#### enforcement)
- `just validator-coverage-gaps` (sanity check that tests exist/guard the change)
- `just validator-traceability` (trace_id/job_id presence in governed mutation paths)
- `just validator-git-hygiene` or `just validator-hygiene-full` (artifact and .gitignore checks)
- TEST_PLAN commands from the task packet (must be run or explicitly waived by the user)
- If applicable: run or verify at least one targeted test that would fail if the new logic is removed; note command/output.
- If a required check cannot be satisfied, obtain explicit user waiver and record it in the task packet and report; absent waiver = FAIL.

## Verdict (Binary)
- PASS: Every requirement mapped to evidence, hygiene clean, tests verified (or explicitly waived by user), DAL audit clean when applicable, phase-gate satisfied when progressing.
- FAIL: List missing evidence, failed audits, tests not run, or unmet phase-gate. No partial passes.

## Validation Gate Sequence [CX-VAL-GATE] (MECHANICAL PAUSES REQUIRED)

The validation process MUST halt at these gates. **No automation may skip these pauses.**
State is tracked per WP in `.GOV/validator_gates/{WP_ID}.json`. Gates enforce minimum time intervals to prevent automation momentum.
(Legacy: `.GOV/roles/validator/VALIDATOR_GATES.json` is treated as a read-only archive for older sessions; new validations MUST NOT write to it.)

### Gate 1: REPORT PRESENTATION (Blocking)
1. Validator completes all checks and generates the full VALIDATION REPORT.
2. Validator **outputs the entire report to chat** using the Report Template.
3. Validator runs: `just validator-gate-present {WP_ID} {PASS|FAIL}`
4. **HALT.** Validator MUST NOT proceed until user acknowledges.

### Gate 2: USER ACKNOWLEDGMENT (Blocking)
1. User explicitly acknowledges the report (e.g., "proceed", "approved", "continue").
2. If user requests changes or disputes findings â†’ return to validation, re-run checks, regenerate report.
3. Validator runs: `just validator-gate-acknowledge {WP_ID}`
4. **Only after explicit acknowledgment** may Validator proceed to Gate 3.

### Gate 3: WP APPEND (Blocking)
1. Validator appends the VALIDATION REPORT to `.GOV/task_packets/{WP_ID}.md` (APPEND-ONLY per [CX-WP-001]).
2. Validator runs: `just validator-gate-append {WP_ID}`
3. Validator confirms append completed and shows the user the appended section.
4. **HALT.** If verdict was FAIL â†’ STOP HERE. No commit.

### Gate 4: COMMIT (PASS only)
1. **Only if verdict = PASS** and user has acknowledged, Validator may commit.
2. Validator runs: `just validator-gate-commit {WP_ID}`
3. Commit message format: `docs: validation PASS [WP-{ID}]` or `feat: implement {feature} [WP-{ID}]`
4. Validator confirms commit hash to user.

### Gate Commands
```
just validator-gate-present {WP_ID} {PASS|FAIL}  # Gate 1: Record report shown
just validator-gate-acknowledge {WP_ID}           # Gate 2: Record user ack
just validator-gate-append {WP_ID}                # Gate 3: Record WP append
just validator-gate-commit {WP_ID}                # Gate 4: Unlock commit (PASS only)
just validator-gate-status {WP_ID}                # Check current gate state
just validator-gate-reset {WP_ID} --confirm       # Reset gates (archives old session)
```

**Violations:** Skipping any gate, auto-committing without user acknowledgment, or appending before showing the report = PROTOCOL VIOLATION [CX-VAL-GATE-FAIL]. Gate commands will fail if sequence is violated.

```
FLOW DIAGRAM:

  [Run all checks] â”€â”€â–º [Generate Report] â”€â”€â–º GATE 1: SHOW IN CHAT â”€â”€â–º HALT
                                                                        â”‚
                                            â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                                            User reviews report
                                                   â”‚
                                            User says "proceed"
                                                   â”‚
                                                   â–¼
                                           GATE 2: ACKNOWLEDGED â”€â”€â–º HALT
                                                                     â”‚
                                            â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                                                   â”‚
                                                   â–¼
                                           GATE 3: APPEND TO WP
                                                   â”‚
                                           â”Œâ”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”
                                           â”‚               â”‚
                                        FAIL?           PASS?
                                           â”‚               â”‚
                                           â–¼               â–¼
                                         STOP        GATE 4: COMMIT
                                      (no commit)          â”‚
                                                           â–¼
                                                      git commit
```

## Merge/Commit Authority (per Codex [CX-505])
- After issuing PASS **and completing all validation gates**, the Validator is responsible for merging/committing the WP to `main`. Coders must not merge their own work.

## Post-Merge Cleanup (reduces branch confusion)
- After a WP is merged into `main`, the Validator SHOULD delete the local WP branch pointer to avoid leaving stale branches:
  - `just close-wp-branch WP-{ID}`
- If the repo uses a remote backup (e.g., GitHub) and the WP branch was pushed, the Validator MAY also delete the remote WP branch after `main` is pushed:
  - `just close-wp-branch WP-{ID} --remote`

## Report Template
```
VALIDATION REPORT â€” {WP_ID}
Verdict: PASS | FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/{WP_ID}.md (status: {status})
- Spec: {spec version/anchors}

Files Checked:
- {list of every file inspected during validation}

Findings:
- Requirement X: satisfied at {path:line}; evidence snippet...
- Hygiene: {clean | issues with details}
- Forbidden Patterns: {results of grep}
- Storage DAL Audit (if applicable): {results for CX-DBP-VAL-010..014}
- Architecture/RDD/LLM: {findings}
- Security/Red Team: {findings}

Tests:
- {command}: {pass/fail/not run + reason}
- Coverage note: {does disabling feature fail tests?}

Risks & Suggested Actions:
- {list any residual risk or missing coverage}
- {actionable steps for future work packets or immediate fixes}

Improvements & Future Proofing:
- {suggested improvements to the code or protocol observed during this audit}
 
Task Packet Update (APPEND-ONLY):
- [CX-WP-001] MANDATORY APPEND: Every validation verdict (PASS/FAIL) MUST be APPENDED to the end of the `.GOV/task_packets/{WP_ID}.md` file. OVERWRITING IS FORBIDDEN.
- [CX-WP-002] CLOSURE REASONS: The append block MUST contain a "REASON FOR {VERDICT}" section explaining exactly why the WP was closed or failed, linking back to specific findings.
- STATUS update in .GOV/task_packets/{WP_ID}.md: PASS/FAIL with reasons, actionables, and further risks. APPEND the full Validation Report using the template below. **DO NOT OVERWRITE User Context or previous history [CX-654].**
- TASK_BOARD update (on `main`): after PASS/FAIL and all criteria met (no acknowledged debt), move the WP entry from `## In Progress` to `## Done` using the enforced status tokens (`[VALIDATED]`, `[FAIL]`, `[OUTDATED_ONLY]`). Status-sync commits earlier in the WP lifecycle are separate and do not imply a verdict.
- Board consistency (on `main`): task packet `**Status:**` is source of truth; reconcile the Task Board to match packet reality before declaring PASS. Unresolved mismatch = FAIL pending correction.
```

## Non-Negotiables
- Evidence over intuition; speculative language is prohibited [CX-588].
- [CX-WP-003] APPEND-ONLY WP HISTORY: Deleting or overwriting the status history in a Work Packet is a protocol violation. All verdicts must be appended.
- Automated review scripts are optional; manual evidence-based validation is required.
- If a check cannot be performed (env/tools unavailable), report as FAIL with reasonâ€”do not assume OK.
- No â€œpass with debtâ€ for hard invariants, security, traceability, or spec alignment; either fix or obtain explicit user waiver per protocol.

````

###### Template File: `.GOV/agents/AGENT_REGISTRY.md`
Intent: Registry of known agents/roles/models and their intended use (routing aid).
````md
# AGENT_REGISTRY

Mapping of contributing agents/models for traceability.

| AGENT_ID | Role | Model/Tooling | Version/Build | Contact/Notes |
| --- | --- | --- | --- | --- |
| AGENT_FRONTEND | Frontend Coder/Reviewer | TBD | TBD | Handles `{{FRONTEND_ROOT_DIR}}/` UI |
| AGENT_SHELL | Tauri/IPC Coder/Reviewer | TBD | TBD | Handles `{{FRONTEND_SRC_DIR}}-tauri/` orchestrator/IPC |
| AGENT_BACKEND | Backend Coder/Reviewer | TBD | TBD | Handles `{{BACKEND_CRATE_DIR}}/` |
| AGENT_SHARED | Shared Contracts | TBD | TBD | Handles `src/shared/` schemas/types |
| AGENT_DOCS | Docs Reviewer | TBD | TBD | Handles `/.GOV/` navigation pack updates |
| AGENT_CI | CI/Hygiene | TBD | TBD | Handles `just validate`/CI workflows |
| AGENT_VALIDATOR | Validator/Reviewer | Manual review | TBD | Performs evidence-based validation and review |

````

###### Template File: `.GOV/GOV_KERNEL/README.md`
Intent: Governance Kernel overview (project-agnostic).
````md
# Governance Kernel Spec (Project-Agnostic)

This directory defines a **project-agnostic, mechanically gated governance system** intended for:
- Multi-role separation of duties (Operator / Orchestrator / Coder / Validator).
- Deterministic execution with auditability (â€œevidence-firstâ€).
- Reliable handoff between **small-context local models** and **large-context cloud models**.

This is a **kernel**: it specifies the *minimum standardized artifacts, file formats, gate semantics, and interlocks* that make the workflow portable across projects.

Non-goals:
- This does not define your product architecture or feature requirements.
- This does not replace your projectâ€™s â€œlaw stackâ€ (Codex + Master Spec + role protocols). It defines how those documents must be structured and mechanically enforced.

## Files (normative)
- `.GOV/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md`: authority stack, role boundaries, branch/worktree rules.
- `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`: canonical governance artifacts (files/dirs), required headings/fields, and failure modes when missing.
- `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`: gate semantics and state machines for Orchestrator/Coder/Validator enforcement scripts.
- `.GOV/GOV_KERNEL/04_SMALL_CONTEXT_HANDOFF.md`: how to packetize work so any model can continue deterministically.
- `.GOV/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md`: CI parity, hooks, and determinism config surface.
- `.GOV/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md`: versioning rules and drift prevention across docs/tools.

## Files (non-normative)
- `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`: **example instantiation** mapping a concrete repo (Handshake) to this kernel, including a full inventory of governing files and scripts.

## Conformance model
A project â€œimplements this kernelâ€ if:
1. The canonical artifacts exist with the required structure and determinism constraints.
2. The gate scripts (or equivalent tooling) enforce the same semantics (it can be different code, but must enforce the same contract).
3. A fresh agent can start from the entrypoints and reliably reproduce: *what is the current spec*, *what work is authorized*, *what is in scope*, *what evidence exists*, and *what gates remain*.

````

###### Template File: `.GOV/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md`
Intent: Kernel: authority stack and roles.
````md
# 01) Authority and Roles (Kernel)

This kernel assumes a â€œlaw stackâ€ where **precedence is explicit** and **roles are mechanically separated** so small-context agents can operate safely.

## 1. Authority stack (precedence is not implicit)

Each project MUST define a precedence order and keep it stable. A canonical order (highest â†’ lowest):

1. **Platform/system constraints**
   - Non-negotiable runtime constraints from the execution environment (tooling limits, sandboxing, secrets, etc.).
2. **Project Codex** (`<PROJECT> Codex vX.Y.md`, repo root)
   - Behavioral constitution for agents and humans interacting with the repo.
   - Must include hard bans (destructive cleanup, unsafe sync) and a conflict/override protocol.
3. **Master Spec** (`<PROJECT>_Master_Spec_vNN.NNN.md`, repo root) + pointer (`.GOV/spec/SPEC_CURRENT.md`)
   - The authoritative product/architecture specification.
   - `.GOV/spec/SPEC_CURRENT.md` MUST be the single pointer to the current spec file.
4. **Role Protocols** (`.GOV/roles/*/*_PROTOCOL.md`)
   - Defines what each role may and may not do.
   - Must include a refinement/signature/packetization process if mechanical gates are used.
5. **Repo-local guardrails** (`AGENTS.md`, repo root)
   - Tight, local instructions for agent execution (branch/worktree rules, safety gates, repo hygiene).
6. **Work authorization artifacts** (`.GOV/task_packets/*.md`, `.GOV/refinements/*.md`, `.GOV/roles_shared/records/TASK_BOARD.md`)
   - Make â€œwhat is allowedâ€ explicit and auditable.
7. **Gate tooling** (`.GOV/scripts/validation/*`, `.GOV/scripts/hooks/*`, `.github/workflows/*`, `justfile`)
   - Mechanical enforcement; tools MUST not silently change the law stack.

Kernel rule: when two sources conflict, the **higher** source wins. Overrides MUST be explicit and logged (see `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`).

## 2. Roles (mechanical separation of duties)

This kernel uses roles as safety boundaries. A role is not a â€œpersonaâ€; it is a **capability envelope**.

### 2.1 OPERATOR (human authority)
Purpose:
- Sets priorities and selects what work is activated.
- Grants explicit approvals for sync/destructive operations.
- Provides signatures for refinement activation and scope overrides.

Non-delegable responsibilities:
- Any exception to hard bans.
- Any explicit â€œsync gateâ€ actions (fetch/merge/rebase/switch) if forbidden by the Codex/Protocol.

### 2.2 ORCHESTRATOR (workflow + spec-to-work translation)
Purpose:
- Translates the Master Spec into executable work authorization artifacts (refinements + task packets).
- Maintains the Task Board and traceability registries.
- Runs Orchestrator gates that record approvals/signatures and prevent momentum failures.

Hard boundary (kernel default):
- Orchestrator MUST NOT implement product code. It only authors/maintains governance artifacts and runs read-only inspection.

### 2.3 CODER (implementation)
Purpose:
- Implements exactly what an activated task packet authorizes, within explicit in-scope paths.
- Produces deterministic evidence (manifests) suitable for Validator review.

Hard boundary:
- Coder MUST NOT change scope, redefine requirements, or â€œfix adjacent thingsâ€ unless a task packet contains a waiver/authorization.

### 2.4 VALIDATOR (audit + acceptance gate)
Purpose:
- Performs evidence-based verification against task packet requirements.
- Verifies tests/builds and traces requirements to file:line evidence.
- Controls the final â€œPASS â†’ commit/merge eligibilityâ€ state (via validator gates).

Hard boundary:
- Validator MUST NOT implement feature code while acting as Validator (to preserve independence).

### 2.5 Optional roles (supported patterns)
These roles MAY exist if explicitly defined in protocols:
- **Tooling agent**: runs diagnostics, builds bundles, or triages CI failures.
- **Debugger**: incident/runbook execution (must not change scope).
- **Red Hat / Red Team mode**: adversarial review framing; typically a Validator sub-mode.

## 3. Branching, worktrees, and concurrency (portable rule set)

Kernel objective: avoid cross-contamination of context and changes when multiple roles or WPs run concurrently.

Mandatory rules:
- One work packet (WP) â†’ one feature branch (e.g., `feat/WP-<ID>`).
- Concurrency across active WPs MUST use `git worktree` (separate working directories).
- A single working tree MUST NOT be shared across concurrent WPs.

Recommended rules:
- One role â†’ one default worktree (e.g., `wt-orchestrator`, `wt-validator`) plus per-WP worktrees as needed.
- Task packets SHOULD specify the expected branch/worktree name so small-context models can validate they are â€œin the right placeâ€.

## 4. Safety: destructive operations and sync gates

To keep the governance system reversible and auditable:

Destructive operations MUST be explicitly authorized in the same turn (examples):
- `git clean -fd`, `git clean -xdf`
- `git reset --hard`
- deleting non-temporary files via `rm`, `del`, `Remove-Item`

If cleanup/reset is authorized:
1. Make it reversible first: `git stash push -u -m "SAFETY: before <operation>"`
2. Preview deletions: `git clean -nd`
3. Proceed only with explicit Operator confirmation.

Sync gate (project-policy-dependent, but kernel-ready):
- If the Codex/Protocol forbids sync actions by default, an agent MUST request explicit authorization before:
  - `git fetch origin` (network)
  - `git switch ...`
  - `git merge` / `git rebase` / fast-forward pulls

## 5. Session â€œenvironment hard gateâ€ (recommended)

For deterministic safety, a role protocol SHOULD require the agent to capture the repo state before work:
- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

Rationale: prevents work being performed in the wrong worktree/branch, which is a primary failure mode when models hand off mid-task.


````

###### Template File: `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`
Intent: Kernel: canonical artifacts and contracts.
````md
# 02) Artifacts and Contracts (Kernel)

This kernel is built around a small set of **canonical artifacts** (files) that jointly answer:
- What is the current authoritative spec?
- What work is authorized, and with what scope?
- What evidence exists, and what gates remain?
- What is the current project state (WPs in progress/done/stub)?

The primary design constraint: a **fresh small-context agent** must be able to reconstruct state by opening a short, stable set of files.

## A. Global invariants (apply to every artifact unless stated otherwise)

### A1) Deterministic parsing
If an artifact is read by gate tooling, it MUST be deterministic to parse:
- Prefer ASCII-only for parser-facing artifacts (task packets, refinements). If non-ASCII is unavoidable, escape as `\\uXXXX`.
- Avoid relying on human-only meaning (e.g., â€œwe all know what this meansâ€).
- Avoid ambiguous formatting (mixed heading styles, inconsistent field labels).

### A2) Canonical naming (portable across projects)
The governance system is portable only if filenames are predictable.

Minimum conventions:
- **WP IDs** MUST be stable identifiers (no timestamps in filenames).
- Packet revisions MUST use `-vN` suffix (example: `WP-12-Foo-v3`).
- When revisions exist, the system MUST record which packet is active (see `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`).

### A3) Append-only audit logs
Audit logs (e.g., signatures) MUST be append-only and treated as evidence, not narrative.

## B. Canonical repo layout (kernel default)

This kernel assumes these stable locations:

- `AGENTS.md` (repo root): repo-local agent hard rules.
- `<PROJECT> Codex vX.Y.md` (repo root): project constitution for agents/humans.
- `<PROJECT>_Master_Spec_vNN.NNN.md` (repo root): authoritative spec versions.
- `.GOV/` (governance surface; canonical)
  - `.GOV/roles_shared/docs/START_HERE.md` (entrypoint; optional but recommended)
  - `.GOV/spec/SPEC_CURRENT.md` (pointer to current Master Spec)
  - `.GOV/roles_shared/records/TASK_BOARD.md` (global execution state)
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet)
  - `.GOV/roles_shared/records/SIGNATURE_AUDIT.md` (append-only signature log)
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `.GOV/roles/coder/CODER_PROTOCOL.md`, `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, `.GOV/validator_gates/{WP_ID}.json` (gate state)
  - `.GOV/templates/` (canonical templates)
  - `.GOV/refinements/` (approved refinements)
  - `.GOV/task_packets/` (activated packets)
    - `.GOV/task_packets/stubs/` (backlog stubs; not executable)
- `docs/` (compatibility bundle; optional; MUST NOT be authoritative governance state)

Projects MAY add additional governance modules (runbooks, ADRs, rubrics). They must still be discoverable from stable entrypoints.

## C. Artifact contracts (required files)

### C1) `.GOV/spec/SPEC_CURRENT.md` (spec pointer)
Purpose:
- Provides a single source of truth for the current authoritative Master Spec version.

Contract:
- Contains exactly one resolvable path to the current spec file (implementation-defined, but deterministic).
- Gate tooling MUST treat this as the only pointer; other docs must not â€œquietly overrideâ€ it.

Failure modes if missing/wrong:
- Agents code against old specs; validators cannot reliably re-resolve intent at review time.

### C2) Master Spec files (`<PROJECT>_Master_Spec_vNN.NNN.md`)
Purpose:
- Centralizes product intent and normative requirements.

Kernel constraint:
- The Master Spec MUST be written to support anchoring (stable headings, stable section IDs, and â€œMain Body firstâ€ discipline if used).

Failure modes:
- Shared mutable global gate ledgers cause merge conflicts and audit loss.
- Refinements cannot create stable anchors; WPs become â€œvibe-codedâ€.

### C3) `.GOV/roles_shared/records/TASK_BOARD.md` (execution state SSoT)
Purpose:
- Single source of truth for WP execution state across roles and models.

Contract (recommended minimum):
- Must contain explicit state sections (example: `## In Progress`, `## Done`, `## Superseded`, plus `## Stubs` if used).
- Each WP entry MUST include: `WP_ID`, Status, and link/path to the active task packet (directly or via traceability registry).

Failure modes:
- Parallel agents diverge on â€œwhat is activeâ€; WPs are duplicated or silently dropped.

### C4) `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet)
Purpose:
- Resolves ambiguity when multiple packets exist for the same Base WP (revisions, superseded attempts).

Contract:
- For every Base WP with multiple packet files, record a single active packet path.
- Must be deterministic to parse (table or strict bullet format).

Failure modes:
- Validators/coders open the wrong revision; acceptance criteria drift across versions.

### C5) `.GOV/refinements/<WP_ID>.md` (refinement artifact)
Purpose:
- Captures the **Technical Refinement Block** that binds a WP to the Master Spec and makes scope executable.

Contract (kernel-level):
- ASCII-only.
- Must include:
  - `WP_ID`
  - `SPEC_TARGET_RESOLVED` (resolved pointer)
  - `SPEC_TARGET_SHA1` (hash of the resolved spec file at refinement time)
  - `USER_APPROVAL_EVIDENCE` (deterministic string used to prevent â€œmomentum signaturesâ€)
  - `USER_SIGNATURE` (Operator signature token)
  - One or more `SPEC_ANCHORS`, each with:
    - start line, end line, and a context token that must appear within that window in the resolved spec.
    - excerpt captured as ASCII (with `\\uXXXX` escapes if needed)
  - `CLEARLY_COVERS` checklist + verdict fields
  - `ENRICHMENT` decision + (if needed) copy-pastable proposed enrichment text

Failure modes:
- Packets lack binding to spec; validators cannot prove requirements are â€œin spec main bodyâ€.
- Small-context coders cannot reconstruct why the WP exists.

### C6) `.GOV/task_packets/stubs/<WP_ID>.md` (stub packets; non-executable)
Purpose:
- Maintains a backlog of future WPs without consuming signatures or producing enforceable scope.

Contract:
- Must clearly declare itself NON-EXECUTABLE (e.g., `STUB_STATUS: STUB`).
- Must not be used as authority for implementation/validation.
- Must include an activation checklist that references refinement/signature requirements.

Failure modes:
- Coders start work from stubs, bypassing refinement and scope gates.

### C7) `.GOV/task_packets/<WP_ID>.md` (activated task packets; executable authority)
Purpose:
- Single authoritative â€œwork contractâ€ for a coder, and the primary audit surface for validators.

Contract (minimum kernel requirements):
- ASCII-only.
- Stable required sections (case-insensitive heading match is allowed, but headings must exist):
  - `## METADATA` (must include `WP_ID`, `BASE_WP_ID`, `Status`, `USER_SIGNATURE`, and declared `ROLE` that authored the packet)
  - `## SCOPE` (must include explicit `IN_SCOPE_PATHS` and explicit `OUT_OF_SCOPE`)
  - `## QUALITY_GATE` with `TEST_PLAN` and `DONE_MEANS`
  - `## AUTHORITY` (must include spec pointer + codex + task board + traceability registry)
  - `## BOOTSTRAP` (files to open, search terms, commands)
  - `## SKELETON` (interface-first design)
  - `## IMPLEMENTATION` (coder fills only after skeleton approval gate)
  - `## HYGIENE`
  - `## VALIDATION` (mechanical manifest blocks for every changed non-doc file)
  - `## STATUS_HANDOFF`
  - `## EVIDENCE` (append logs/output)
  - `## VALIDATION_REPORTS` (validator append-only audits/verdicts)

Kernel phase gate requirement:
- A literal line containing exactly `SKELETON APPROVED` MUST exist outside fenced code blocks before any â€œimplementation evidenceâ€ markers recognized by gate tooling.

Failure modes:
- Scope creep (â€œI also refactored Xâ€) becomes unauditable.
- Post-work evidence cannot be validated (hashes/line windows missing).

### C8) Templates (`.GOV/templates/`)
Purpose:
- Makes artifact creation reproducible and reduces formatting drift that breaks gate tooling.

Contract:
- Canonical templates SHOULD be stored in `.GOV/templates/` and copied into new artifacts.
- If compatibility shims exist in `docs/` (legacy paths), they must be explicitly labeled as shims.

Failure modes:
- Gate scripts fail due to format drift; new models produce incompatible packets/refinements.

### C9) Gate state (`.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, `.GOV/validator_gates/{WP_ID}.json`)
Purpose:
- Stores the state machine for role-specific gates (refine/sign/prepare; validate/acknowledge/commit, etc.).

Contract:
- JSON is treated as authoritative gate state.
- Must be append-only in effect: state transitions are logged with timestamps and immutable evidence links.
- Validator gate state MUST be per-WP to avoid cross-WP merge conflicts:
  - Canonical per-WP file: `.GOV/validator_gates/{WP_ID}.json`
  - Legacy archive (read-only): `.GOV/roles/validator/VALIDATOR_GATES.json` (MUST NOT receive new sessions)

Failure modes:
- Agents cannot prove which gates were completed; â€œverdictsâ€ become social, not mechanical.

### C10) Signature audit log (`.GOV/roles_shared/records/SIGNATURE_AUDIT.md`)
Purpose:
- Central append-only log of Operator signatures and what they approved.

Contract:
- Each signature entry must link to the artifact(s) signed (refinement, packet).
- Format must be deterministic enough for tooling to confirm that a signature exists for a given WP.

Failure modes:
- Work can be started without real Operator authorization; approvals can be disputed.


````

###### Template File: `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`
Intent: Kernel: gate semantics and enforcers.
````md
# 03) Gates and Enforcers (Kernel)

This kernel assumes governance is enforced by **mechanical gates** (.GOV/scripts/hooks/CI) rather than by convention.

Design principle:
- Artifacts define authority.
- Gates make authority executable by rejecting drift and â€œmomentum failuresâ€.
- State transitions are recorded in append-only or monotonic state files.

Implementation note:
- A project MAY implement gates using any tooling (Node, Python, Rust, shell), but the **semantics** below are normative if the project claims kernel conformance.

## 1. Single command surface (recommended)

Kernel recommendation: expose all governance commands via a single command surface (example: `justfile`).

Rationale:
- Small-context agents can follow deterministic commands without rediscovering ad-hoc scripts.
- CI can reuse the same command surface for parity.

Minimum recommended commands (names may be standardized across projects):
- `record-refinement <WP_ID>`
- `record-signature <WP_ID> <signature>`
- `record-prepare <WP_ID> <coder_id> [branch] [worktree_dir]`
- `create-task-packet <WP_ID>`
- `gate-check <WP_ID>`
- `pre-work <WP_ID>`
- `post-work <WP_ID>`
- `validator-gate-*` (validator state machine)

## 2. Orchestrator gates (REFINEMENT â†’ SIGNATURE â†’ PREPARE)

Kernel objective: prevent creating an â€œexecutable packetâ€ until a WP is demonstrably anchored to the spec and explicitly approved.

### 2.1 Gate state file (normative)
The Orchestrator gate tool MUST persist a state file (example path: `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`) with an append-only log, at minimum:
- `wpId`
- `type` in `{REFINEMENT, SIGNATURE, PREPARE}`
- `timestamp` (ISO-8601)
- additional fields per gate (below)

### 2.2 REFINEMENT gate (recording)
Inputs:
- `WP_ID`
- refinement file path (optional)

Required checks:
- WP_ID has canonical form (project-defined, but stable and parseable; commonly `WP-...`).
- Refinement file passes `refinement-check` structural validation with `requireSignature=false`.
- If the Master Spec pointer is resolvable, record:
  - resolved spec file name
  - resolved spec SHA1

Required writes:
- Append a `REFINEMENT` entry to Orchestrator gate logs.

Required behavior:
- Must output a â€œgate lockedâ€ warning: signatures MUST NOT be requested/recorded in the same turn as refinement recording.

### 2.3 SIGNATURE gate (one-time signature consumption)
Inputs:
- `WP_ID`
- `signature` token (project-defined; must be unambiguous and reproducible)

Required checks:
- A REFINEMENT gate entry exists for this WP.
- **Anti-momentum**: signature must not be recorded too soon after refinement (time-based minimum interval or equivalent).
- Refinement passes structural validation with `requireSignature=false`.
- Refinement declares `ENRICHMENT_NEEDED=NO` (if enrichment is required, signing is forbidden).
- Refinement contains deterministic `USER_APPROVAL_EVIDENCE` matching a required literal string (example pattern: `APPROVE REFINEMENT <WP_ID>`).
- Refinement is not already signed.
- Signature is one-time use (must not already appear anywhere in repo history/surface as defined by project policy).

Required writes:
- Update refinement file:
  - `USER_REVIEW_STATUS: APPROVED`
  - `USER_SIGNATURE: <signature>`
- Append a signature record to an append-only audit file (example: `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`).
- Append a `SIGNATURE` entry in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` including:
  - signature
  - refinement path

Required behavior:
- Must instruct the operator/agent that packet creation is still blocked until PREPARE is recorded.

### 2.4 PREPARE gate (assignment + branch/worktree readiness)
Purpose:
- Prevent â€œcoding without a homeâ€: packet creation must be blocked until the WP branch/worktree exists and a coder is assigned.

Required checks:
- A SIGNATURE entry exists for the WP.
- A WP branch exists locally (name derived from WP_ID or explicitly provided).
- A git worktree exists for that branch (required when concurrency rules demand it).

Required writes:
- Append a `PREPARE` entry including:
  - `coder_id`
  - `branch`
  - `worktree_dir`

## 3. Packet creation gate (`create-task-packet`)

Kernel objective: a task packet is an â€œexecutable contractâ€. Creating it must be impossible to do â€œearlyâ€.

Required checks before writing `.GOV/task_packets/<WP_ID>.md`:
- A refinement file exists; if missing, tooling SHOULD create a scaffold and then HARD-BLOCK (exit non-zero) until refinement is completed and reviewed.
- Refinement is approved/signed and signature is present.
- Refinement declares `ENRICHMENT_NEEDED=NO`.
- The signature exists in:
  - the refinement file,
  - the Orchestrator gate state log,
  - the signature audit log.
- A PREPARE record exists after the SIGNATURE record.

Required behavior:
- Create the packet from the canonical template.
- Populate provenance fields (e.g., `SPEC_BASELINE`) deterministically from the resolved spec pointer where possible.

## 4. Coder phase gate (`gate-check`)

Kernel objective: enforce â€œinterface-firstâ€ sequencing and prevent merged phases.

Required checks (conceptual):
- BOOTSTRAP must exist before SKELETON.
- A literal `SKELETON APPROVED` marker must exist (outside code fences) before implementation evidence is accepted.
- Gate parsing must ignore fenced code blocks to avoid false positives.

Failure modes prevented:
- â€œImplemented while still designingâ€ (hard to audit).
- â€œTurn mergingâ€ where a model writes design + implementation without a review checkpoint.

## 5. Pre-work gate (`pre-work`)

Kernel objective: block implementation until the work contract is complete, signed, and checkpointed.

Required checks:
- Activated task packet exists for WP_ID.
- Packet includes required structural fields (scope + test plan + done means + bootstrap).
- If the packet is not explicitly Done/Validated:
  - Refinement exists and is signed.
  - Packet USER_SIGNATURE matches refinement signature.
  - Signature exists in signature audit log.
- **Checkpoint commit gate**: packet and refinement must exist in `HEAD` (prevents loss of untracked artifacts).
- Packet contains a deterministic validation manifest template (COR-701-style fields) to enable post-work validation.

## 6. Post-work gate (`post-work`)

Kernel objective: make changes auditable by forcing a per-file manifest and verifying it against the git diff.

Minimum required semantics:
- For every changed non-doc file, there must be a manifest block in the packet validation section that includes:
  - target file path
  - start/end line window for intended changes
  - expected line delta
  - deterministic Pre-SHA1 and Post-SHA1
- Gate tooling must verify, at minimum:
  - the file exists and is openable
  - the diff is contained within declared windows (unless waivered)
  - the reported line delta matches git numstat delta
  - the pre/post hashes match the declared states (HEAD/INDEX policy is project-defined but must be consistent)

## 7. Validator gates (REPORT_PRESENTED â†’ USER_ACKNOWLEDGED â†’ WP_APPENDED â†’ COMMITTED)

Kernel objective: make validation evidence visible to the Operator before allowing a commit/merge step.

Required state machine:
1. `present-report <WP_ID> <PASS|FAIL>`
2. `acknowledge <WP_ID>` (Operator acknowledges report was seen)
3. `append <WP_ID>` (validator appends report to packet)
4. `commit <WP_ID>` (PASS only; unlocks commit)

Required properties:
- Gate state stored per WP in a deterministic JSON state file (example: `.GOV/validator_gates/{WP_ID}.json`; legacy archive: `.GOV/roles/validator/VALIDATOR_GATES.json`).
- Anti-momentum interval between gate transitions.
- FAIL verdict must permanently block the commit gate for that WP_ID (must create new WP variant to re-pass).

## 8. Auxiliary governance checks (kernel-recommended)

These checks are not always required for kernel conformance, but they harden portability:
- **Task board format check**: enforces strict, machine-parseable WP state lines.
- **Task packet claim check**: when Status is `In Progress`, require Coder claim fields (model + reasoning strength) to be non-placeholder.
- **Worktree concurrency check**: detect multiple active WPs in one worktree (project-defined heuristic).
- **Spec-current check**: ensures `.GOV/spec/SPEC_CURRENT.md` points to the newest spec version by version parsing policy.
- **Codex check**: detects forbidden patterns (architecture violations, unsafe APIs, debug prints) and codex drift across docs.


````

###### Template File: `.GOV/GOV_KERNEL/04_SMALL_CONTEXT_HANDOFF.md`
Intent: Kernel: small-context handoff rules.
````md
# 04) Small-Context Handoff (Kernel)

This kernel is designed so that **work survives model swaps**:
- small-context local models
- large-context cloud models
- human handoffs

The mechanism is not â€œremembering chatâ€. It is **artifact-first continuity**.

## 1. Principle: chat is not state

Kernel rule:
- Any decision that affects scope, requirements, safety, or acceptance MUST be written into a governance artifact (packet/refinement/board/audit log).

Rationale:
- Small models cannot hold long chat context.
- Chat logs are not reliably searchable/structured for mechanical auditing.

## 2. Deterministic â€œminimum context bundleâ€ per role

A fresh agent should be able to start by opening a small, stable set of files.

Recommended minimum set:
- Operator: `.GOV/roles_shared/records/TASK_BOARD.md` + the active packet(s) being approved.
- Orchestrator: `.GOV/spec/SPEC_CURRENT.md`, `.GOV/roles_shared/records/TASK_BOARD.md`, `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`, refinement + packet templates, and the target WP refinement/packet.
- Coder: the activated packet + the referenced refinement + the in-scope code paths.
- Validator: activated packet + refinement + spec target resolved + changed files + CI/test outputs + validator gate state.

## 3. Packetization as context compression

The task packet is the primary context compressor. To support small-context models, the packet MUST include:
- `FILES_TO_OPEN`: the exact files the model must read (ordered).
- `SEARCH_TERMS`: stable grep terms to find key anchors in code.
- `RUN_COMMANDS`: exact commands (or â€œnoneâ€).
- `RISK_MAP`: â€œrisk â†’ impactâ€ mapping to guide cautious behavior.
- `DONE_MEANS` + `TEST_PLAN`: to prevent â€œlooks goodâ€ completion claims.

Kernel intent:
- A coder should never need to re-open the entire Master Spec to start.
- A validator should never need to infer scope from commits.

## 4. Refinement anchors as â€œspec shardingâ€

Large specs do not fit in small contexts. The refinement solves this by:
- Binding the WP to a specific spec version (`SPEC_TARGET_SHA1`).
- Providing one or more anchors with:
  - start/end line window
  - context token that must exist in-window
  - excerpt captured as ASCII

Effect:
- A small model can prove it is reading the right part of the spec without ingesting the whole document.

## 5. Decomposing large work into internal sub-tasks (recommended method)

Yes: complex tasks should be decomposed, but the decomposition must be **artifact-backed**.

Kernel method:
1. Orchestrator creates a single WP refinement + packet with an explicit â€œinternal milestonesâ€ list inside `## SKELETON` or `## IMPLEMENTATION` as a checklist.
2. Each milestone has:
   - explicit in-scope files
   - local acceptance criteria
   - the evidence that must be produced (logs, manifests, screenshots, etc.)
3. After each milestone, the coder updates `## STATUS_HANDOFF` with:
   - current milestone
   - what changed (file list)
   - next command to run
   - any hazards discovered
4. Validator reviews milestone-by-milestone, appending official notes in `## VALIDATION_REPORTS`.

Alternative (when milestones exceed packet size or become independent):
- Split into multiple WPs. Each WP must still be independently refinable, signable, and gate-checkable.

## 6. Context continuity across model swaps (mechanical)

When swapping models/roles mid-flight, the outgoing agent must leave:
- Updated `.GOV/roles_shared/records/TASK_BOARD.md` status (if the project uses it as SSoT).
- Updated packet `## STATUS_HANDOFF` (single place to read â€œwhere we areâ€).
- Updated `## EVIDENCE` (copy/paste logs; avoid â€œran tests locally, trust meâ€).
- Completed manifests in `## VALIDATION` for any changed files.

Incoming agent procedure (deterministic):
1. Open the packet and read `## STATUS_HANDOFF`.
2. Open the refinement and confirm:
   - correct WP_ID
   - signature present
   - anchors exist and match the current spec target
3. Run the pre-work gate before doing anything (or verify it was run and the inputs are unchanged).

## 7. Why â€œheavy thinkingâ€ is not the primary control surface

The kernel assumes model capability varies. Therefore:
- correctness is enforced by gates + explicit artifacts, not by model â€œmemoryâ€
- reasoning strength is captured as a declared field (e.g., `CODER_REASONING_STRENGTH`) for risk management, not as a substitute for evidence

Practical guidance:
- For strict governance with large specs, a standard â€œheavy reasoningâ€ model is usually sufficient because artifacts bound scope and anchors shard context.
- Extra-heavy reasoning helps during refinement and cross-artifact drift detection, but it should not replace mechanical verification.


````

###### Template File: `.GOV/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md`
Intent: Kernel: CI/hook parity and config determinism.
````md
# 05) CI, Hooks, and Determinism Config (Kernel)

This kernel assumes governance is not â€œdocumented onlyâ€; it is **enforced** by:
- local hooks (pre-commit)
- CI workflows
- determinism configs (EOL, formatting, toolchain pinning)

Objective:
- A small-context agent should be able to run the same checks locally as CI runs remotely (â€œCI parityâ€).

## 1. CI parity (normative)

Kernel rule:
- CI MUST execute the same governance gates that developers are expected to run locally (or a strict superset).

Minimum recommended CI checks:
- Governance/doc presence checks (required navigation + pointer files).
- Spec pointer correctness (`.GOV/spec/SPEC_CURRENT.md` resolves).
- Task Board formatting check (machine-parseable state).
- Codex checks (forbidden patterns; repo invariants).
- Gate tooling checks (phase gate; pre-work/post-work where applicable).
- Supply-chain checks (licenses/vulns) if the project includes them as hard requirements.

Failure modes if CI parity is missing:
- Developers â€œpass locallyâ€ but fail CI due to hidden requirements.
- Small-context handoffs break because the command surface isnâ€™t authoritative.

## 2. Pre-commit hooks (recommended)

Purpose:
- Catch high-frequency governance violations before they hit CI.

Kernel recommendation:
- A pre-commit hook SHOULD run:
  - fast doc/gov checks (Codex checks, task board check)
  - format checks if fast and deterministic
  - it SHOULD NOT run long builds/tests unless the project explicitly requires it (to avoid disabling hooks).

Hard rule:
- Hooks MUST NOT mutate tracked files automatically unless that behavior is explicitly codified (auto-formatters are allowed if the repo policy is to apply them).

## 3. Determinism configuration surface (kernel-required categories)

Projects MUST define, in-repo, the determinism surface that makes gates reliable.

Common required categories:

### 3.1 End-of-line policy
Purpose:
- Prevent line-ending drift across OSes, which breaks hash-based gates and window-based diff checks.

Contract:
- Define an explicit EOL policy (example: `eol=lf` via `.gitattributes`).
- Gate tooling MUST treat this policy as authoritative and handle CRLF/LF comparisons deterministically.

### 3.2 Ignore policy (`.gitignore`)
Purpose:
- Prevent transient artifacts from polluting diffs and confusing manifest gates.

Contract:
- Tool outputs that are not part of audit artifacts must be ignored (target dirs, caches, node_modules, build outputs).

### 3.3 Toolchain pinning (language/runtime-specific)
Purpose:
- Make CI reproducible and prevent â€œworks on my machineâ€ drift.

Examples (implementation-defined):
- Rust: toolchain version, cargo target dir policy, lint/deny policies.
- Node: pinned package manager and lockfiles.
- Python: pinned interpreter + lockfile.

## 4. Governance-command allowlists (optional hardening)

Some environments restrict what commands an agent may run.

Kernel recommendation:
- Keep an allowlist config that enumerates â€œapproved commandsâ€ for automation agents.

Failure mode if missing:
- Agents run dangerous commands by accident or in the wrong repo, causing loss of work or secret leakage.

## 5. Drift hazards and required mitigations

### 5.1 Version reference drift
Hazard:
- CI/hooks/docs mention an old Codex/spec/protocol version while the repo root has newer versions.

Mitigation (kernel recommendation):
- Add a CI check that asserts referenced governance file names exist and match the latest version pointer(s).
- Prefer pointers (`.GOV/spec/SPEC_CURRENT.md`) over hardcoding version strings in many places.

### 5.2 Template drift
Hazard:
- Agents generate packets/refinements from memory and omit required sections, breaking gates.

Mitigation:
- Keep canonical templates under `.GOV/templates/`.
- Add checks that assert templates contain mandatory fields (manifest block, required headings).


````

###### Template File: `.GOV/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md`
Intent: Kernel: versioning and drift control.
````md
# 06) Versioning and Drift Control (Kernel)

This kernel assumes:
- specs evolve over time
- tooling and docs must remain synchronized
- small-context models will otherwise â€œremember the wrong versionâ€

The system therefore treats drift as a first-class failure mode.

## 1. Versioned specs + a single pointer

Kernel rules:
- Master Spec files MUST be versioned (`..._vNN.NNN.md`) and immutable once superseded (append-only history).
- `.GOV/spec/SPEC_CURRENT.md` MUST be the single authoritative pointer to the current Master Spec.
- All enforcement scripts and protocols SHOULD resolve the spec via `.GOV/spec/SPEC_CURRENT.md` rather than hardcoding filenames.

Failure modes prevented:
- â€œCoding against old specâ€ when multiple versions exist.
- Validators reviewing against a different spec than coders used.

## 2. One-time approvals and auditability

Kernel recommendation:
- Use one-time approval tokens (signatures) as evidence that:
  - a refinement was reviewed
  - a scope contract was accepted
  - a spec enrichment was intentionally approved

Hard rule:
- Approvals must be recorded in append-only audit logs with deterministic formatting so tools can confirm their existence.

## 3. Compatibility shims (allowed, but must be explicit)

Projects evolve directory layouts and filenames. Shims are allowed to avoid breaking tooling, but they must be explicit.

Kernel rule:
- If a legacy path exists (example: `docs/TASK_PACKET_TEMPLATE.md`), it MUST be labeled as a shim that points to the canonical template (example: `.GOV/templates/TASK_PACKET_TEMPLATE.md`).

Failure mode prevented:
- Agents copy an obsolete template and generate non-conforming packets.

## 4. Drift detection checklist (kernel-recommended)

Add a â€œdrift guardâ€ check in CI that detects:
- Spec pointer drift:
  - `.GOV/spec/SPEC_CURRENT.md` points to a non-existent file
  - `.GOV/spec/SPEC_CURRENT.md` does not point to the latest spec by parsed version policy
- Governance reference drift:
  - docs/CI/hooks reference a Codex filename that does not exist
  - scripts reference protocol files that moved/renamed
- Template drift:
  - required headings/fields removed from canonical templates
- Roadmap determinism drift (if used):
  - Coverage Matrix missing/duplicated rows
  - invalid phase tokens
  - mismatch between matrix titles and actual heading titles

## 5. Drift handling policy (what to do when drift is found)

Kernel approach:
1. Treat drift as a governance failure, not as â€œcleanupâ€.
2. Create an explicit remediation artifact:
   - update pointers (preferred) rather than renaming many files
   - add compatibility shims if necessary
3. Record the decision in an audit log or changelog section so future models do not re-litigate it.

## 6. Why this matters for small-context models

Small models fail by:
- losing the active spec version
- hallucinating missing requirements
- using the wrong template or missing gates

This kernel prevents that by:
- forcing all â€œtruthâ€ into a small set of stable artifacts
- making drift detectable by scripts, not by memory


````

###### Template File: `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`
Intent: Reference mapping to a concrete repo implementation (non-normative example; optional export).
````md
# Reference Implementation: {{PROJECT_DISPLAY_NAME}} Governance (Non-Normative)

Purpose: map a concrete repository ({{PROJECT_DISPLAY_NAME}}) to the project-agnostic governance kernel (`.GOV/GOV_KERNEL/*`), including an exhaustive inventory of governing artifacts and enforcement scripts.

Scope:
- This is a governance/operations spec, not product behavior.
- It documents (a) the kernel-level concepts and (b) how they are concretely implemented by files/scripts in this repo.
- "No file left out" means: every file under the governance surface (.GOV/, .GOV/operator/docs_local/, .GOV/scripts/, .github/) plus all root-level governance/config files (and the optional `docs/` compatibility bundle, if present) are enumerated in the inventory section.

Non-goals:
- Do not change product code (`{{BACKEND_ROOT_DIR}}/`, `{{FRONTEND_ROOT_DIR}}/`, `tests/`) here.
- Do not treat this document as a replacement for the authoritative law stack (Codex + Master Spec + protocols); it is an implementation map and inventory.

---

## 1) Authority Stack (LAW) and Precedence

The governance system is explicit about precedence. The current implemented stack is:

1. `{{CODEX_FILENAME}}` (repo root)
   - Defines repo invariants and allowed assistant behavior (including hard bans on destructive ops and git worktree/branch rewrites without explicit consent).
2. Master Spec: `{{PROJECT_PREFIX}}_Master_Spec_v*.md` (repo root), with pointer file `.GOV/spec/SPEC_CURRENT.md`
   - Product intent + architecture + normative requirements; "Main Body first" discipline is enforced mechanically.
3. Protocol layer in `.GOV/roles/`
   - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Orchestrator behavior and signature/refinement workflow)
   - `.GOV/roles/coder/CODER_PROTOCOL.md` (Coder behavior and phase gating)
   - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md` (Validator behavior and evidence-based audit rules)
4. Repo guardrails: `AGENTS.md`
   - Local hard rules specific to this repo (no destructive cleanup; WP branching/worktree; checkpoint commit gate).
5. Mechanical enforcement scripts + `justfile`
   - Deterministic checks and workflow gates implemented as executable scripts under `.GOV/scripts/` and invoked via `just`.

Important implemented constraint:
- The system is designed to support small-context local models by forcing work to be packetized, anchored, and gate-checked (see Sections 4 and 7).

---

## 2) Roles (Mechanical Separation of Duties)

This repo uses rigid roles that intentionally limit what each agent may do. The design goal is to prevent accidental scope creep, spec drift, and un-auditable changes.

### OPERATOR (human)
- Sets priorities and approves (in-chat) refinements and signatures.
- Owns any explicit overrides to governance.

### ORCHESTRATOR (lead architect / workflow manager)
- Creates and maintains governance artifacts: stubs, refinements, task packets, board, traceability, signature audit.
- Does not implement product code.
- Owns "spec-to-work translation" (SPEC_ANCHOR mapping, DONE_MEANS, TEST_PLAN, exact IN_SCOPE_PATHS).
- Runs Orchestrator gate scripts to record refinement/signature/prepare events.

### CODER (implementation)
- Implements only what the task packet requires, within IN_SCOPE_PATHS.
- Must not change scope or interpret spec beyond what the packet anchors.
- Must not claim validation verdicts (Validator-only).

### VALIDATOR (auditor / red team)
- Performs evidence-based review: opens files, maps requirements to file:line, verifies tests.
- Controls the final "PASS -> commit" gate via validator gate logs.
- Maintains operator-visible status sync on `main` (Active cross-branch section).

### Optional roles (supported by the docs but not always active)
- Debugger (triage; uses `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`)
- Tooling agent (runs scripts / builds diagnostic bundles)
- Red-team framing exists as a responsibility inside Validator and Refinement blocks.

---

## 3) Canonical Governance Artifacts (What exists, and why)

This section describes the key governance artifacts and how they gate each other.

### 3.1 Navigation + orientation pack (`.GOV/roles_shared/`)
- `.GOV/roles_shared/docs/START_HERE.md`: canonical entry point and command surface.
- `.GOV/roles_shared/docs/ARCHITECTURE.md`: module map and allowed dependency boundaries.
- `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`: incident/CI triage, log locations, minimal debug flow.
- `.GOV/roles_shared/PAST_WORK_INDEX.md`: archaeology pointers (note: this file currently contains stale references; see Section 8).

Why: enables a fresh model (or human) to orient quickly and deterministically without reading the whole repo.

### 3.2 Spec pointer and spec drift guard
- `.GOV/spec/SPEC_CURRENT.md`: the single pointer to the current authoritative Master Spec.
- `.GOV/scripts/spec-current-check.mjs`: enforces that SPEC_CURRENT points to the latest `{{PROJECT_PREFIX}}_Master_Spec_v*.md` file by parsed version.

Why: prevents silent spec drift and "coding against an old spec".

### 3.3 Task Board (execution state SSoT)
- `.GOV/roles_shared/records/TASK_BOARD.md`: the global state tracker for Phase 1 WPs.
- `.GOV/scripts/validation/task-board-check.mjs`: enforces strict formatting for `## In Progress`, `## Done`, `## Superseded` entries.

Key rule (enforced in docs and protocols):
- Task Board is intentionally minimal; detailed reasons live in packets.

### 3.4 Work Packet Traceability (Base WP -> Active Packet mapping)
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`: resolves Base WP IDs to an Active Packet file path, especially when revisions exist (`-vN`).

Why: the Master Spec should not embed revision packet IDs; this registry prevents ambiguity when multiple packet revisions exist.

### 3.5 Stubs vs activated packets
- Stubs: `.GOV/task_packets/stubs/` (not executable)
- Activated packets: `.GOV/task_packets/` (executable authority for implementation/validation)
- Templates:
  - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`

Why: allows backlog reshaping without consuming signatures, while keeping "Ready for Dev" meaningfully executable.

### 3.6 Refinements (Technical Refinement Block artifacts)
- `.GOV/refinements/{WP_ID}.md`: per-WP refinement artifact.
- Template: `.GOV/templates/REFINEMENT_TEMPLATE.md`
- Mechanical enforcement: `.GOV/scripts/validation/refinement-check.mjs`

Key implemented properties:
- ASCII-only.
- Includes SPEC_TARGET_RESOLVED + SPEC_TARGET_SHA1 binding to the current spec file.
- Includes SPEC_ANCHORS with excerpt window and token-in-window match requirements.
- Includes "CLEARLY_COVERS" 5-point checklist and ENRICHMENT decision.
- Includes `USER_APPROVAL_EVIDENCE` as a deterministic guard against momentum.

Why: makes spec anchoring checkable and portable across small-context models.

### 3.7 Signatures (one-time, auditable)
- `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`: authoritative registry of signatures consumed.
- Orchestrator gate script appends entries and enforces one-time use.

Why: creates a forced alignment pause and prevents autonomous drift.

### 3.8 Gate state logs (machine-readable)
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`: log of REFINE/SIGN/PREPARE events.
- `.GOV/validator_gates/{WP_ID}.json`: per-WP log of validation gate sequence (present -> acknowledge -> append -> commit). (Legacy archive: `.GOV/roles/validator/VALIDATOR_GATES.json`.)

Why: provides deterministic, machine-checkable proof that the workflow was followed.

### 3.9 Quality gate definition
- `.GOV/roles_shared/docs/QUALITY_GATE.md`: Gate 0 (pre-work) and Gate 1 (post-work) definitions; risk tier matrix; required commands.

Why: sets a minimum hygiene baseline; prevents "it compiled on my machine" merges.

### 3.10 Role-local worktree policy
- `.GOV/roles_shared/docs/ROLE_WORKTREES.md`: local mapping of role -> (worktree dir, branch) on the operator machine.

Why: prevents role confusion and cross-WP contamination; makes "where am I?" checkable.

### 3.11 Ownership and agent identity
- `.GOV/roles_shared/docs/OWNERSHIP.md`: area owners for review routing.
- `.GOV/agents/AGENT_REGISTRY.md`: agent IDs and role mapping.

Why: provides accountability for multi-agent work and review routing.

---

## 4) End-to-End Mechanical Workflow (How the gates interlock)

This section maps the workflow to concrete scripts and state files.

### 4.1 Global hard gate (environment + repo state)
Required for Orchestrator/Coder/Validator sessions:
- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

Why: role work must occur in the correct worktree and branch, preventing accidental cross-role actions.

### 4.2 Backlog lifecycle: STUB -> Activated -> Ready for Dev
1. Create stub file in `.GOV/task_packets/stubs/` (no signature).
2. When ready to activate:
   - Produce in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
   - Fill `.GOV/refinements/{WP_ID}.md` from template and run refinement validation.
   - Record refinement: `just record-refinement {WP_ID}` (writes `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`).
   - In a new user turn, after explicit approval evidence exists in the refinement file:
     - Record signature: `just record-signature {WP_ID} {usernameDDMMYYYYHHMM}`
       - Updates refinement file with APPROVED status and signature
       - Appends the signature to `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
       - Writes the signature event to `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
   - Create WP worktree/branch: `just worktree-add {WP_ID}` (creates `feat/{WP_ID}` worktree)
   - Record prepare: `just record-prepare {WP_ID} {Coder-A|Coder-B} [branch] [worktree_dir]`
   - Create the official packet: `just create-task-packet {WP_ID}`
     - Script hard-gates on signature + prepare being recorded and on ENRICHMENT_NEEDED=NO.
3. Complete activation traceability updates (mandatory before any coding starts):
   - Update `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` Baseâ†’Active mapping to point to `.GOV/task_packets/{WP_ID}.md` (NOT a stub).
   - Move `.GOV/roles_shared/records/TASK_BOARD.md` entry out of STUB backlog to Ready for Dev.

Why:
- Prevents "packet by momentum" and ensures packet activation is both human-approved and mechanically logged.

### 4.3 Coder lifecycle: Ready for Dev -> In Progress -> Handoff
1. Coder claims the packet by setting:
   - `**Status:** In Progress`
   - `CODER_MODEL`
   - `CODER_REASONING_STRENGTH`
   (Enforced by `.GOV/scripts/validation/task-packet-claim-check.mjs`)
2. Gate check: `just gate-check {WP_ID}` enforces Markdown phase ordering and "SKELETON APPROVED" before implementation signals.
3. Pre-work gate: `just pre-work {WP_ID}`
   - Validates packet structure
   - Validates refinement exists + signature matches
   - Enforces checkpoint commit gate for packet + refinement (prevents artifact loss)
   - Ensures deterministic manifest template exists
4. Coder implements within IN_SCOPE_PATHS and keeps evidence in the packet.
5. Post-work gate: `just post-work {WP_ID}`
   - Enforces deterministic manifest correctness (hashes, window bounds, line delta, path canonicalization).
   - Performs staged-aware checks to reduce false failures from unrelated local changes.

### 4.4 Validator lifecycle: audit -> PASS/FAIL -> commit gate
Validator uses both:
- Manual evidence audit (open files, map to file:line, re-run tests as needed)
- Mechanical validator scripts (scan, traceability, error-codes, DAL audit, git hygiene, etc.)

Additionally, Validator uses a mechanical gate sequence (writes per WP to `.GOV/validator_gates/{WP_ID}.json`):
1. `just validator-gate-present {WP_ID} {PASS|FAIL}`
2. (After user acknowledgment) `just validator-gate-acknowledge {WP_ID}`
3. Append report to packet: `just validator-gate-append {WP_ID}`
4. If PASS, unlock commit: `just validator-gate-commit {WP_ID}`

Why: ensures the user sees the report before it is appended and before a commit is allowed.

### 4.5 Command-to-script mapping (what runs, what it reads/writes)

This table is intentionally explicit because these commands are the "mechanical glue" of the workflow.

| Command | Implementation | Reads | Writes |
|---|---|---|---|
| `just record-refinement {WP_ID}` | `.GOV/scripts/validation/orchestrator_gates.mjs refine` | refinement file, `.GOV/spec/SPEC_CURRENT.md` (+ spec file) | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` |
| `just record-signature {WP_ID} {sig}` | `.GOV/scripts/validation/orchestrator_gates.mjs sign` | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, refinement file, `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`, repo grep for one-time signature | refinement file (sets APPROVED + signature), `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`, `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` |
| `just worktree-add {WP_ID}` | `.GOV/scripts/worktree-add.mjs` | git refs/worktree list | creates branch/worktree dir on disk (git operation) |
| `just record-prepare {WP_ID} ...` | `.GOV/scripts/validation/orchestrator_gates.mjs prepare` | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, git branch exists, `git worktree list --porcelain` | `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` |
| `just create-task-packet {WP_ID}` | `.GOV/scripts/create-task-packet.mjs` | refinement file, `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`, `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`, `.GOV/templates/TASK_PACKET_TEMPLATE.md`, `.GOV/spec/SPEC_CURRENT.md` | `.GOV/task_packets/{WP_ID}.md` (or creates refinement scaffold and exits BLOCKED) |
| `just gate-check {WP_ID}` | `.GOV/scripts/validation/gate-check.mjs` | `.GOV/task_packets/{WP_ID}.md` | none |
| `just pre-work {WP_ID}` | `gate-check` + `.GOV/scripts/validation/pre-work-check.mjs` | packet + refinement + signature audit; `.GOV/scripts/validation/cor701-spec.json`; git object DB for checkpoint commit gate | may create `.GOV/task_packets/` dir if missing |
| `just post-work {WP_ID}` | `gate-check` + `.GOV/scripts/validation/post-work-check.mjs` | packet; git diff/index/worktree files; spec schema `cor701-spec.json` | none |
| `just cor701-sha <file>` | `.GOV/scripts/validation/cor701-sha.mjs` | git blobs (HEAD/INDEX) + worktree file | none |
| `just task-board-check` | `.GOV/scripts/validation/task-board-check.mjs` | `.GOV/roles_shared/records/TASK_BOARD.md` | none |
| `just task-packet-claim-check` | `.GOV/scripts/validation/task-packet-claim-check.mjs` | `.GOV/task_packets/*.md` | none |
| `just validator-gate-*` | `.GOV/scripts/validation/validator_gates.mjs` | `.GOV/validator_gates/{WP_ID}.json` (may read legacy `.GOV/roles/validator/VALIDATOR_GATES.json` for older sessions), (append gate checks packet exists) | `.GOV/validator_gates/{WP_ID}.json` |

Momentum/anti-bypass notes (current implementation):
- Orchestrator signature recording blocks if signature is recorded too soon after refinement (anti-momentum timer) and if USER_APPROVAL_EVIDENCE is missing/mismatched.
- Validator gates block if the next gate is executed within a minimum interval (anti-momentum).

---

## 5) Deterministic Manifest (COR-701 discipline)

Task packets contain a required `## VALIDATION` manifest block (template-enforced) with:
- target_file
- start/end line window
- line_delta
- pre_sha1 / post_sha1
- gates checklist

Key implementing components:
- Spec schema: `.GOV/scripts/validation/cor701-spec.json`
- SHA helper: `.GOV/scripts/validation/cor701-sha.mjs`
- Enforcement: `.GOV/scripts/validation/post-work-check.mjs`

Notable implementation detail:
- `post-work-check.mjs` normalizes LF/CRLF variants for SHA convenience and uses staged diffs when possible to reduce cross-platform false negatives.

Why:
- Enables deterministic audits and "what exactly changed" provenance without trusting narrative summaries.

---

## 6) Concurrency and Worktrees

Core policy:
- One WP = one feature branch (`feat/{WP_ID}`) and (when >1 concurrent WP) one separate worktree per active WP.

Implementations:
- `.GOV/roles_shared/docs/ROLE_WORKTREES.md`: defines role default worktrees/branches locally.
- `.GOV/scripts/worktree-add.mjs` + `just worktree-add`: creates WP worktree/branch.
- `.GOV/scripts/validation/worktree-concurrency-check.mjs`: local-only check; requires linked worktrees when multiple WPs are in progress.

Why:
- Prevents unstaged changes from one WP contaminating another WP's deterministic manifest/hygiene checks.

---

## 7) Context Management for Small-Context Models (Project-agnostic kernel)

The governance system is explicitly designed to support fresh models with small context windows by "front-loading" the needed context into machine-checkable artifacts.

### 7.1 How to decompose large work safely

Rule of thumb:
- If a task cannot be fully specified (scope, acceptance, tests, risks) in a single task packet without vague language, it should be split into multiple WPs.

Recommended decomposition strategies:
1. Split by invariant surface area:
   - Example: "migration idempotency" vs "down migrations" vs "test harness".
2. Split by layer boundary:
   - frontend UI vs backend API vs storage vs scripts.
3. Split by risk tier:
   - isolate HIGH-risk changes into their own WP so they can be audited independently.

### 7.2 How context is carried across sub-tasks

Carry context through artifacts, not chat memory:
- `.GOV/refinements/{WP_ID}.md` binds the packet to a specific spec version (sha1) and provides excerpt windows for anchors.
- Task packets embed:
  - exact IN_SCOPE_PATHS
  - DONE_MEANS
  - TEST_PLAN (copy-paste commands)
  - BOOTSTRAP (files to open, search terms, commands, risk map)
  - deterministic manifest(s)

This allows a new model to pick up work by reading:
- `.GOV/roles_shared/docs/START_HERE.md`
- `.GOV/spec/SPEC_CURRENT.md`
- the task packet
- the refinement

### 7.3 Model selection: when "heavy reasoning" is needed

This workflow reduces the need for large-context "hero" models by making work deterministic and decomposable. Heavy reasoning models still help when:
- The Master Spec slice is large and ambiguous.
- The work requires multi-layer architectural reasoning with high risk (security/storage).
- The required evidence mapping is extensive.

Otherwise, a standard model can execute WPs reliably when the packet and refinement are complete and the gates are passing.

---

## 8) Known Drift / Inconsistencies (Current repo state)

The governance system contains explicit drift that should be addressed to keep determinism intact:

Codex version references:
- CI workflow `.github/workflows/ci.yml` contains strings and messaging referencing "Codex v{{CODEX_VERSION}}".
- `.GOV/scripts/validation/ci-traceability-check.mjs` explicitly checks for `{{CODEX_FILENAME}}` (but the repo root currently contains `{{CODEX_FILENAME}}`).
- `.GOV/scripts/hooks/pre-commit` messaging references "Codex v{{CODEX_VERSION}}".
- `.GOV/task_packets/README.md` links to `{{PROJECT_DISPLAY_NAME}} Codex v{{CODEX_VERSION}}` (stale).
- `.GOV/roles_shared/PAST_WORK_INDEX.md` references much older spec/codex versions (stale).

Why this matters:
- These are governance enforcement surfaces (CI + hooks). If they refer to non-existent files/versions, they either fail unnecessarily or mislead operators/models.

Recommended remediation approach:
- Treat governance enforcement drift as its own remediation WP (so .GOV/scripts/CI can be updated by a Coder under a signed packet), or explicitly declare a compatibility shim file if intentional.

---

## 9) Full Inventory (Snapshot)

Generated from repo file listing; grouped by directory. This is the "no file left out" surface for governance-oriented files and scripts.

### 9.1 Top-level directories
- `.cargo/`
- `.claude/`
- `.github/`
- `app/`
- `docs/`
- `.GOV/operator/docs_local/`
- `.GOV/scripts/`
- `src/`
- `tests/`

### 9.2 Top-level files (repo root)
- `.codex_tmp_file`
- `.git`
- `.gitattributes`
- `.gitignore`
- `AGENTS.md`
- `deny.toml`
- `docker-compose.test.yml`
- `extraction and digital production team.md`
- `{{CODEX_FILENAME}}`
- `{{PROJECT_DISPLAY_NAME}}_Export_Bundles_Insert_Plan_v0.1.md`
- `{{PROJECT_PREFIX}}_logger_20251218.md`
- `{{PROJECT_PREFIX}}_Master_Spec_v02.102.md`
- `{{PROJECT_PREFIX}}_Master_Spec_v02.103.md`
- `{{PROJECT_PREFIX}}_Master_Spec_v02.104.md`
- `{{PROJECT_PREFIX}}_Master_Spec_v02.105.md`
- `{{PROJECT_PREFIX}}_Master_Spec_v02.106.md`
- `{{PROJECT_PREFIX}}_Master_Spec_v02.107.md`
- `{{PROJECT_DISPLAY_NAME}}_Phase_0_5_Closure_v0.1.md`
- `justfile`
- `n8n and feature mixing.md`
- `primitives_catalogue.md`
- `README.md`
- `STORAGE_PORTABILITY_ARCHITECTURE_GAP_ANALYSIS.md`
- `validation audit.md`

### 9.3 `.github/`
- `.github/workflows/ci.yml`

### 9.4 `.claude/`
- `.claude/settings.local.json`

### 9.5 `.cargo/`
- `.cargo/config.toml`

### 9.6 `.GOV/scripts/`
- `.GOV/scripts/README.md`
- `.GOV/scripts/close-wp-branch.mjs`
- `.GOV/scripts/codex-check-test.mjs`
- `.GOV/scripts/create-task-packet.mjs`
- `.GOV/scripts/new-api-endpoint.mjs`
- `.GOV/scripts/new-react-component.mjs`
- `.GOV/scripts/scaffold-check.mjs`
- `.GOV/scripts/spec-current-check.mjs`
- `.GOV/scripts/worktree-add.mjs`
- `.GOV/scripts/fixtures/forbidden_fetch.ts`
- `.GOV/scripts/fixtures/forbidden_todo.txt`
- `.GOV/scripts/hooks/pre-commit`
- `.GOV/scripts/validation/ci-traceability-check.mjs`
- `.GOV/scripts/validation/codex-check.mjs`
- `.GOV/scripts/validation/cor701-sha.mjs`
- `.GOV/scripts/validation/cor701-spec.json`
- `.GOV/scripts/validation/gate-check.mjs`
- `.GOV/scripts/validation/orchestrator_gates.mjs`
- `.GOV/scripts/validation/post-work-check.mjs`
- `.GOV/scripts/validation/pre-work-check.mjs`
- `.GOV/scripts/validation/refinement-check.mjs`
- `.GOV/scripts/validation/task-board-check.mjs`
- `.GOV/scripts/validation/task-packet-claim-check.mjs`
- `.GOV/scripts/validation/validator_gates.mjs`
- `.GOV/scripts/validation/validator-coverage-gaps.mjs`
- `.GOV/scripts/validation/validator-dal-audit.mjs`
- `.GOV/scripts/validation/validator-error-codes.mjs`
- `.GOV/scripts/validation/validator-git-hygiene.mjs`
- `.GOV/scripts/validation/validator-hygiene-full.mjs`
- `.GOV/scripts/validation/validator-packet-complete.mjs`
- `.GOV/scripts/validation/validator-phase-gate.mjs`
- `.GOV/scripts/validation/validator-scan.mjs`
- `.GOV/scripts/validation/validator-spec-regression.mjs`
- `.GOV/scripts/validation/validator-traceability.mjs`
- `.GOV/scripts/validation/worktree-concurrency-check.mjs`

### 9.7 `docs/`
- `.GOV/GOV_KERNEL/README.md`
- `.GOV/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md`
- `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`
- `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`
- `.GOV/GOV_KERNEL/04_SMALL_CONTEXT_HANDOFF.md`
- `.GOV/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md`
- `.GOV/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md`
- `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`
- `docs/AI_WORKFLOW_TEMPLATE.md` (compat shim; canonical under `.GOV/templates/`)
- `.GOV/roles_shared/docs/ARCHITECTURE.md`
- `docs/CODER_IMPLEMENTATION_ROADMAP.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `docs/CODER_PROTOCOL_GAPS.md`
- `docs/CODER_PROTOCOL_SCRUTINY.md`
- `.GOV/roles/coder/CODER_RUBRIC.md`
- `docs/MASTER_SPEC_INTENT_AUDIT_v02.103.md`
- `docs/MASTER_SPEC_MVP_ROADMAP_AUDIT_v02.103.md`
- `docs/MASTER_SPEC_SECTION_DIGEST_v02.103.md`
- `.GOV/roles_shared/docs/MIGRATION_GUIDE.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
- `docs/ORCHESTRATOR_IMPLEMENTATION_ROADMAP.md`
- `docs/ORCHESTRATOR_PRIORITIES.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `docs/ORCHESTRATOR_PROTOCOL_GAPS.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_RUBRIC.md`
- `.GOV/roles_shared/records/OSS_REGISTER.md`
- `.GOV/roles_shared/docs/OWNERSHIP.md`
- `.GOV/roles_shared/PAST_WORK_INDEX.md`
- `docs/PHASE_1_EVIDENCE_MAP_v02.103.md`
- `.GOV/roles_shared/docs/QUALITY_GATE.md`
- `docs/REFINEMENT_TEMPLATE.md` (compat shim; canonical under `.GOV/templates/`)
- `docs/ROADMAP_SECTION_COVERAGE_MATRIX_v02.103.md`
- `docs/ROADMAP_SECTION_COVERAGE_MATRIX_v02.107.md`
- `docs/ROADMAP_VS_MASTER_SPEC_AUDIT_v02.102.md`
- `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
- `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
- `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
- `.GOV/spec/SPEC_CURRENT.md`
- `.GOV/roles_shared/docs/START_HERE.md`
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `docs/TASK_PACKET_TEMPLATE.md` (compat shim; canonical under `.GOV/templates/`)
- `.GOV/validator_gates/README.md`
- `.GOV/roles/validator/VALIDATOR_GATES.json` (legacy archive)
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- `docs/workflow_technical_paper.md`
- `.GOV/adr/ADR-0001-handshake-architecture-and-governance.md`
- `.GOV/agents/AGENT_REGISTRY.md`
- `docs/messages history/coder claude code.md`
- `docs/messages history/coder gemini.md`
- `docs/messages history/coder gpt codex.md`
- `docs/messages history/orchestrator.md`
- `docs/messages history/Validator.md`
- `docs/Papers/HANDSHAKE_VISION_SYNTHESIS.md`
- `.GOV/refinements/README.md`
- `.GOV/refinements/WP-1-ACE-Validators-v4.md`
- `.GOV/refinements/WP-1-AppState-Refactoring-v3.md`
- `.GOV/refinements/WP-1-Dual-Backend-Tests-v2.md`
- `.GOV/refinements/WP-1-Flight-Recorder-v3.md`
- `.GOV/refinements/WP-1-LLM-Core-v3.md`
- `.GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md`
- `.GOV/refinements/WP-1-Migration-Framework-v2.md`
- `.GOV/refinements/WP-1-Operator-Consoles-v3.md`
- `.GOV/refinements/WP-1-OSS-Register-Enforcement-v1.md`
- `.GOV/refinements/WP-1-Spec-Enrichment-LLM-Core-v1.md`
- `.GOV/refinements/WP-1-Storage-Abstraction-Layer-v3.md`
- `.GOV/refinements/WP-1-Terminal-LAW-v3.md`
- `.GOV/refinements/WP-1-Tokenization-Service-v3.md`
- `.GOV/task_packets/README.md`
- `.GOV/task_packets/WP-1-ACE-Auditability.md`
- `.GOV/task_packets/WP-1-ACE-RAG-Plumbing.md`
- `.GOV/task_packets/WP-1-ACE-Runtime.md`
- `.GOV/task_packets/WP-1-ACE-Validators-v2.md`
- `.GOV/task_packets/WP-1-ACE-Validators-v3.md`
- `.GOV/task_packets/WP-1-ACE-Validators-v4.md`
- `.GOV/task_packets/WP-1-ACE-Validators.md`
- `.GOV/task_packets/WP-1-AI-Integration-Baseline.md`
- `.GOV/task_packets/WP-1-AI-Job-Model-v2.md`
- `.GOV/task_packets/WP-1-AI-Job-Model-v3.md`
- `.GOV/task_packets/WP-1-AI-Job-Model.md`
- `.GOV/task_packets/WP-1-AI-UX-Actions.md`
- `.GOV/task_packets/WP-1-AI-UX-Rewrite.md`
- `.GOV/task_packets/WP-1-AI-UX-Summarize-Display.md`
- `.GOV/task_packets/WP-1-AppState-Refactoring-v2.md`
- `.GOV/task_packets/WP-1-AppState-Refactoring-v3.md`
- `.GOV/task_packets/WP-1-AppState-Refactoring.md`
- `.GOV/task_packets/WP-1-Atelier-Lens-v0.1.md`
- `.GOV/task_packets/WP-1-Atelier-Lens.md`
- `.GOV/task_packets/WP-1-Bundle-Export.md`
- `.GOV/task_packets/WP-1-Calendar-Lens.md`
- `.GOV/task_packets/WP-1-Canvas-Typography.md`
- `.GOV/task_packets/WP-1-Capability-Enforcement.md`
- `.GOV/task_packets/WP-1-Capability-SSoT-Validator.md`
- `.GOV/task_packets/WP-1-Capability-SSoT.md`
- `.GOV/task_packets/WP-1-Debug-Bundle-v2.md`
- `.GOV/task_packets/WP-1-Debug-Bundle-v3.md`
- `.GOV/task_packets/WP-1-Debug-Bundle.md`
- `.GOV/task_packets/WP-1-Diagnostic-Pipe.md`
- `.GOV/task_packets/WP-1-Distillation-Logging.md`
- `.GOV/task_packets/WP-1-Distillation.md`
- `.GOV/task_packets/WP-1-Dual-Backend-Tests-v2.md`
- `.GOV/task_packets/WP-1-Dual-Backend-Tests.md`
- `.GOV/task_packets/WP-1-Editor-Hardening.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-UI-v2.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-UI.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-v2.md`
- `.GOV/task_packets/WP-1-Flight-Recorder-v3.md`
- `.GOV/task_packets/WP-1-Flight-Recorder.md`
- `.GOV/task_packets/WP-1-Frontend-AI-Action.md`
- `.GOV/task_packets/WP-1-Frontend-Build-Debug.md`
- `.GOV/task_packets/WP-1-Gate-Check-Tool-v2.md`
- `.GOV/task_packets/WP-1-Gate-Check-Tool.md`
- `.GOV/task_packets/WP-1-Governance-Hooks.md`
- `.GOV/task_packets/WP-1-LLM-Core-v3.md`
- `.GOV/task_packets/WP-1-LLM-Core.md`
- `.GOV/task_packets/WP-1-MCP-End-to-End.md`
- `.GOV/task_packets/WP-1-MCP-Skeleton-Gate.md`
- `.GOV/task_packets/WP-1-Mechanical-Track-Full.md`
- `.GOV/task_packets/WP-1-Metrics-OTel.md`
- `.GOV/task_packets/WP-1-Metrics-Traces.md`
- `.GOV/task_packets/WP-1-MEX-Observability.md`
- `.GOV/task_packets/WP-1-MEX-Safety-Gates.md`
- `.GOV/task_packets/WP-1-MEX-UX-Bridges.md`
- `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v2.md`
- `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md`
- `.GOV/task_packets/WP-1-MEX-v1.2-Runtime.md`
- `.GOV/task_packets/WP-1-Migration-Framework.md`
- `.GOV/task_packets/WP-1-Model-Profiles.md`
- `.GOV/task_packets/WP-1-Mutation-Traceability.md`
- `.GOV/task_packets/WP-1-Operator-Consoles-v1.md`
- `.GOV/task_packets/WP-1-Operator-Consoles-v2.md`
- `.GOV/task_packets/WP-1-Operator-Consoles-v3.md`
- `.GOV/task_packets/WP-1-Operator-Consoles.md`
- `.GOV/task_packets/WP-1-OSS-Governance.md`
- `.GOV/task_packets/WP-1-OSS-Register-Enforcement-v1.md`
- `.GOV/task_packets/WP-1-PDF-Pipeline.md`
- `.GOV/task_packets/WP-1-Photo-Studio-Skeleton.md`
- `.GOV/task_packets/WP-1-Photo-Studio.md`
- `.GOV/task_packets/WP-1-RAG-Iterative.md`
- `.GOV/task_packets/WP-1-Retention-GC.md`
- `.GOV/task_packets/WP-1-Security-Gates-v2.md`
- `.GOV/task_packets/WP-1-Security-Gates-v3.md`
- `.GOV/task_packets/WP-1-Security-Gates.md`
- `.GOV/task_packets/WP-1-Semantic-Catalog.md`
- `.GOV/task_packets/WP-1-Spec-Enrichment-LLM-Core-v1.md`
- `.GOV/task_packets/WP-1-Storage-Abstraction-Layer-v2.md`
- `.GOV/task_packets/WP-1-Storage-Abstraction-Layer-v3.md`
- `.GOV/task_packets/WP-1-Storage-Abstraction-Layer.md`
- `.GOV/task_packets/WP-1-Storage-Foundation-20251228.md`
- `.GOV/task_packets/WP-1-Storage-Foundation-v3.md`
- `.GOV/task_packets/WP-1-Storage-Foundation.md`
- `.GOV/task_packets/WP-1-Supply-Chain-MEX.md`
- `.GOV/task_packets/WP-1-Terminal-Integration-Baseline.md`
- `.GOV/task_packets/WP-1-Terminal-LAW-v2.md`
- `.GOV/task_packets/WP-1-Terminal-LAW-v3.md`
- `.GOV/task_packets/WP-1-Terminal-LAW.md`
- `.GOV/task_packets/WP-1-Tokenization-Service-20251228.md`
- `.GOV/task_packets/WP-1-Tokenization-Service-v3.md`
- `.GOV/task_packets/WP-1-Tokenization-Service.md`
- `.GOV/task_packets/WP-1-Validator-Error-Codes-v1.md`
- `.GOV/task_packets/WP-1-Workflow-Engine-v2.md`
- `.GOV/task_packets/WP-1-Workflow-Engine-v3.md`
- `.GOV/task_packets/WP-1-Workflow-Engine-v4.md`
- `.GOV/task_packets/WP-1-Workflow-Engine.md`
- `.GOV/task_packets/WP-1-Workspace-Bundle.md`
- `.GOV/task_packets/stubs/README.md`
- `.GOV/task_packets/stubs/WP-1-ACE-Auditability-v2.md`
- `.GOV/task_packets/stubs/WP-1-ACE-Runtime-v2.md`
- `.GOV/task_packets/stubs/WP-1-AI-Job-Model-v4.md`
- `.GOV/task_packets/stubs/WP-1-AI-UX-Actions-v2.md`
- `.GOV/task_packets/stubs/WP-1-AI-UX-Rewrite-v2.md`
- `.GOV/task_packets/stubs/WP-1-AI-UX-Summarize-Display-v2.md`
- `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.md`
- `.GOV/task_packets/stubs/WP-1-Calendar-Lens-v2.md`
- `.GOV/task_packets/stubs/WP-1-Canvas-Typography-v2.md`
- `.GOV/task_packets/stubs/WP-1-Capability-SSoT-v2.md`
- `.GOV/task_packets/stubs/WP-1-Cross-Tool-Interaction-Conformance-v1.md`
- `.GOV/task_packets/stubs/WP-1-Dev-Experience-ADRs.md`
- `.GOV/task_packets/stubs/WP-1-Distillation-v2.md`
- `.GOV/task_packets/stubs/WP-1-Editor-Hardening-v2.md`
- `.GOV/task_packets/stubs/WP-1-Flight-Recorder-UI-v3.md`
- `.GOV/task_packets/stubs/WP-1-Global-Silent-Edit-Guard.md`
- `.GOV/task_packets/stubs/WP-1-Governance-Kernel-Conformance-v1.md`
- `.GOV/task_packets/stubs/WP-1-Governance-Hooks-v2.md`
- `.GOV/task_packets/stubs/WP-1-LocalFirst-Agentic-MCP-Posture-v1.md`
- `.GOV/task_packets/stubs/WP-1-MCP-End-to-End-v2.md`
- `.GOV/task_packets/stubs/WP-1-MCP-Skeleton-Gate-v2.md`
- `.GOV/task_packets/stubs/WP-1-Metrics-OTel-v2.md`
- `.GOV/task_packets/stubs/WP-1-Metrics-Traces-v2.md`
- `.GOV/task_packets/stubs/WP-1-MEX-Observability-v2.md`
- `.GOV/task_packets/stubs/WP-1-MEX-Safety-Gates-v2.md`
- `.GOV/task_packets/stubs/WP-1-MEX-UX-Bridges-v2.md`
- `.GOV/task_packets/stubs/WP-1-Migration-Framework-v2.md`
- `.GOV/task_packets/stubs/WP-1-Model-Profiles-v2.md`
- `.GOV/task_packets/stubs/WP-1-Mutation-Traceability-v2.md`
- `.GOV/task_packets/stubs/WP-1-OSS-Governance-v2.md`
- `.GOV/task_packets/stubs/WP-1-PDF-Pipeline-v2.md`
- `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.md`
- `.GOV/task_packets/stubs/WP-1-RAG-Iterative-v2.md`
- `.GOV/task_packets/stubs/WP-1-Response-Behavior-ANS-001.md`
- `.GOV/task_packets/stubs/WP-1-Semantic-Catalog-v2.md`
- `.GOV/task_packets/stubs/WP-1-Spec-Router-Session-Log.md`
- `.GOV/task_packets/stubs/WP-1-Supply-Chain-MEX-v2.md`
- `.GOV/task_packets/stubs/WP-1-Workspace-Bundle-v2.md`
- `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
- `.GOV/templates/REFINEMENT_TEMPLATE.md`
- `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
- `.GOV/templates/TASK_PACKET_TEMPLATE.md`

### 9.8 `.GOV/operator/docs_local/`
- `.GOV/operator/docs_local/DOC_INDEX.txt`
- `.GOV/operator/docs_local/Diary RID extraction.txt`
- `.GOV/operator/docs_local/legacy/The_Prompt_Diaries_v03.056.000_2025-11-28_01-42_CET.txt`

````

###### Template File: `.GOV/templates/TASK_PACKET_TEMPLATE.md`
Intent: Canonical task packet template (Gate 0 input).
````md
# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

Requirements:
- Keep packets ASCII-only (required by deterministic gates).
- Use SPEC_BASELINE for provenance (spec at creation time).
- Use SPEC_TARGET as the authoritative spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- WP_ID and filename MUST NOT include date/time stamps; use `-v{N}` for revisions (e.g., `WP-1-Tokenization-Service-v3`).
- If multiple packets exist for the same Base WP, update `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet).

---

# Task Packet: {{WP_ID}}

## METADATA
- TASK_ID: {{WP_ID}}
- WP_ID: {{WP_ID}}
- BASE_WP_ID: {{WP_ID}} (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: {{DATE_ISO}}
- REQUESTOR: {{REQUESTOR}}
- AGENT_ID: {{AGENT_ID}}
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: LOW | MEDIUM | HIGH
- USER_SIGNATURE: {{USER_SIGNATURE}}

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/{{WP_ID}}.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What:
- Why:
- IN_SCOPE_PATHS:
  - path/to/file
- OUT_OF_SCOPE:
  - out/of/scope/path

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work {{WP_ID}}
# ...task-specific commands...
just cargo-clean
just post-work {{WP_ID}}
```

### DONE_MEANS
- measurable criterion 1
- measurable criterion 2

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: {{SPEC_BASELINE}} (recorded_at: {{DATE_ISO}})
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: {{SPEC_ANCHOR}}
- Codex: {{CODEX_FILENAME}}
- Task Board: .GOV/roles_shared/records/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/docs/START_HERE.md
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/roles_shared/docs/ARCHITECTURE.md
  - path/to/file
- SEARCH_TERMS:
  - "exact symbol"
  - "error code"
- RUN_COMMANDS:
  ```bash
  # task-specific commands
  ```
- RISK_MAP:
  - "risk name" -> "impact"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> {{PROJECT_PREFIX}}_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

````

###### Template File: `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
Intent: Canonical stub task packet template (pre-activation; no signature).
````md
# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: {{WP_ID}}

## STUB_METADATA
- WP_ID: {{WP_ID}}
- BASE_WP_ID: {{WP_ID}} (stable ID without `-vN`; equals WP_ID for stubs; if WP_ID includes `-vN`, override to the base ID)
- CREATED_AT: {{DATE_ISO}}
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: {{ROADMAP_POINTER}}
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - {{SPEC_ANCHOR_1}}
  - {{SPEC_ANCHOR_2}}

## INTENT (DRAFT)
- What:
- Why:

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - ...
- OUT_OF_SCOPE:
  - ...

## ACCEPTANCE_CRITERIA (DRAFT)
- ...

## DEPENDENCIES / BLOCKERS (DRAFT)
- ...

## RISKS / UNKNOWNs (DRAFT)
- ...

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/{{WP_ID}}.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet {{WP_ID}}` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.


````

###### Template File: `.GOV/templates/REFINEMENT_TEMPLATE.md`
Intent: Canonical refinement template (required before signature).
````md
## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: {{WP_ID}}
- CREATED_AT: {{DATE_ISO}}
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> {{SPEC_TARGET_RESOLVED}}
- SPEC_TARGET_SHA1: {{SPEC_TARGET_SHA1}}
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: <pending> (must equal: APPROVE REFINEMENT {{WP_ID}})

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- <fill; write NONE if no gaps>

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- <fill; write NONE if not applicable>

### RED_TEAM_ADVISORY (security failure modes)
- <fill; write NONE if not applicable>

### PRIMITIVES (traits/structs/enums)
- <fill; write NONE if not applicable>

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PENDING
- CLEARLY_COVERS_REASON: <fill>
- AMBIGUITY_FOUND: PENDING (YES | NO)
- AMBIGUITY_REASON: <fill; write NONE if AMBIGUITY_FOUND=NO>

### ENRICHMENT
- ENRICHMENT_NEEDED: PENDING
- REASON_NO_ENRICHMENT: <fill if ENRICHMENT_NEEDED=NO>

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: <fill (example: {{PROJECT_PREFIX}}_Master_Spec_v02.99.md 2.3.13.5 [CX-DBP-030])>
- CONTEXT_START_LINE: <fill integer>
- CONTEXT_END_LINE: <fill integer>
- CONTEXT_TOKEN: <fill exact string that must appear between start/end lines in SPEC_TARGET_RESOLVED>
- EXCERPT_ASCII_ESCAPED:
  ```text
  <paste the relevant excerpt; ASCII-only; use \\uXXXX escapes when needed>
  ```

#### ANCHOR 2
- SPEC_ANCHOR: <fill>
- CONTEXT_START_LINE: <fill integer>
- CONTEXT_END_LINE: <fill integer>
- CONTEXT_TOKEN: <fill>
- EXCERPT_ASCII_ESCAPED:
  ```text
  <paste excerpt>
  ```

````

###### Template File: `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
Intent: Reusable AI workflow template for other repos/projects.
````md
# AI_WORKFLOW_TEMPLATE (Governance Pack-derived)

Purpose: capture the exact governance + workflow structure we implemented today so it can be reused in future repos or embedded as a template for local/cloud model workspaces.

This document is intended to be copied into other projects as a starting point. It is not a replacement for a project-specific codex or master spec.

## What we did (summary)
- Created a canonical navigation pack in `/.GOV/roles_shared/` so any model can orient fast.
- Added an explicit spec pointer (`.GOV/spec/SPEC_CURRENT.md`) and a check to prevent drift.
- Established a debug runbook with a first-5-minutes flow and CI failure triage.
- Added ownership + agent registry so reviews and traceability have a target.
- Introduced a Quality Gate with risk tiers and required validation commands.
- Added scaffolding scripts and enforcement checks to reduce structure drift.
- Standardized manual validator review as the required review artifact.

## Why we did it (rationale)
- Determinism: reduce guesswork about where to look and how to act.
- Traceability: make it easy to track why a change happened months later.
- Error reduction: enforce architecture rules (no direct fetch, no println, etc.).
- Speed: consistent commands and templates reduce repeated setup.
- Debuggability: stable log anchors and runbooks shorten incident triage.

## Canonical inputs and precedence (template)
1) `.GOV/spec/SPEC_CURRENT.md` (points to current master spec)
2) Codex (repo root)
3) Task Board (`.GOV/roles_shared/records/TASK_BOARD.md`) + task packet for the WP
4) Logger (optional; milestones/hard bugs only, root or `log_archive/`)
5) ADRs (`.GOV/adr/`)
6) Past specs/logs (`.GOV/roles_shared/PAST_WORK_INDEX.md`)

## Required navigation pack (copy these)
| File | Purpose | Why it matters |
| --- | --- | --- |
| `.GOV/roles_shared/docs/START_HERE.md` | Entry point + commands | Fast orientation for new models |
| `.GOV/spec/SPEC_CURRENT.md` | Canonical spec pointer | Prevents spec drift |
| `.GOV/roles_shared/docs/ARCHITECTURE.md` | Module map + allowed deps | Avoids architectural entropy |
| `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` | Debug flow + log map | Consistent incident handling |
| `.GOV/roles_shared/PAST_WORK_INDEX.md` | Links to old work | Prevents archaeology guesswork |
| `.GOV/roles_shared/docs/QUALITY_GATE.md` | Risk tiers + required checks | Sets minimum hygiene |
| `.GOV/templates/TASK_PACKET_TEMPLATE.md` | Standard work packet | Keeps scope/validation consistent |
| `.GOV/roles_shared/docs/OWNERSHIP.md` | Review routing | Clear accountability |
| `.GOV/agents/AGENT_REGISTRY.md` | Agent IDs + roles | Traceability for AI work |

## Roles (template)
- Orchestrator: builds task packets; may not have repo access.
- Coder: implements changes; runs local checks; updates docs if needed.
- Debugger: triages issues; uses `RUNBOOK_DEBUG`.
- Validator: performs manual evidence-based review against codex/spec.
- Owner/Reviewer: required review sign-off per `OWNERSHIP.md`.

## Task lifecycle (deterministic flow)
1) Orchestrator produces a task packet using `.GOV/templates/TASK_PACKET_TEMPLATE.md`.
2) Coder reads `.GOV/roles_shared/docs/START_HERE.md` + `.GOV/spec/SPEC_CURRENT.md`.
3) Coder classifies task: DEBUG / FEATURE / REVIEW / REFACTOR / HYGIENE.
4) Coder reads `.GOV/roles_shared/docs/ARCHITECTURE.md` or `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` based on type.
5) Implement change using scaffolds if adding components/endpoints.
6) Run required commands from `.GOV/roles_shared/docs/QUALITY_GATE.md`.
7) Validator performs manual review and records evidence in the packet or validation report.
8) Update `.GOV/roles_shared/docs/ARCHITECTURE.md` or `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` if new entrypoints or repeatable failures were added.
9) Reviewer validates against codex + required checks.

## Commands (single source)
Keep the authoritative commands in `.GOV/roles_shared/docs/START_HERE.md` and the task packet. Standard set:
- `just validate` (docs check + lint/tests + depcruise + fmt/clippy + deny)
- `just codex-check`
- `just scaffold-check`

If `just` is unavailable, run the explicit commands directly.

## Scaffolding (structure enforcement)
Use scaffolds for new components/endpoints to avoid drift:
- `just new-react-component <ComponentName>`
- `just new-api-endpoint <endpoint_name>`
- `just scaffold-check` to verify output

## Manual review (required)
Validator performs a manual evidence-based review against the codex/spec and records a PASS/FAIL verdict with evidence mapping.

## Git hook (optional but recommended)
Enable a pre-commit hook for local hygiene checks:
```
git config core.hooksPath .GOV/scripts/hooks
```

## Validation and enforcement (defaults)
These checks are designed to run in CI or locally:
- `docs-check`: ensures navigation pack exists.
- `codex-check`: disallow direct `fetch(` outside API layer; disallow `println!/eprintln!` in backend; ensure SPEC_CURRENT points to latest spec; enforce TODO tagging.
- `depcruise`: frontend layer boundaries.
- `cargo-deny`: backend dependency audit.
- `gitleaks`: secret scanning.

## Logging and debug anchors
Use stable error tags like `{{ISSUE_PREFIX}}-####` for repeatable failures.
Add those tags to `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` with entrypoints and triage notes.
Task Board + task packet act as the micro-log; the {{PROJECT_DISPLAY_NAME}} logger is for milestones/hard bugs when requested.

## Repository layout conventions (template)
- `/.GOV/` is canonical operational guidance.
- `/.GOV/operator/docs_local/` is staging/legacy and non-binding.
- `/log_archive/` stores historical loggers.
- `/.claude/` stores Claude Code instructions (optional but documented if present).

## How to reuse this template in a new repo
1) Copy the navigation pack files listed above into the new repo.
2) Create a codex and point `.GOV/spec/SPEC_CURRENT.md` to the master spec.
3) Populate `.GOV/roles_shared/docs/ARCHITECTURE.md` with real entrypoints.
4) Add `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md` with log locations and first-5-minutes flow.
5) Add scaffolding scripts and wire `justfile` targets.
6) Require manual validator review and evidence mapping.
7) Add CI jobs for lint/tests/depcruise/deny/gitleaks as available.
8) Add ownership and agent registry rows for the team/roles.

## Optional extensions
- Use optional automated review tooling as a secondary reviewer for high-risk changes.
- Add custom lint rules or architecture tests for deeper enforcement.
- Add a `KNOWN_DEVIATIONS` section in the codex for intentional layout drift.

````

###### Template File: `.GOV/scripts/hooks/pre-commit`
Intent: Local git hook enforcing codex checks and quick hygiene at commit time.
````bash
#!/usr/bin/env bash
# Pre-commit hook [CX-902]
# Enforces workflow compliance at commit time

set -e

echo ""
echo "dY"' Pre-commit validation (Codex v{{CODEX_VERSION}})..."
echo ""

# Extract WP_ID from commit message (if in env)
# Git hooks don't have direct access to commit message in pre-commit
# So we'll check for staged changes and validate against recent logger entries

# Check 1: Ensure files are staged
STAGED_FILES=$(git diff --cached --name-only)
if [ -z "$STAGED_FILES" ]; then
  echo "Æ’?O No files staged for commit"
  exit 1
fi

echo "Æ’o. Files staged for commit"
echo ""

# Check 2: Clean Cargo artifacts (external target dir to avoid repo bloat)
echo "Cleaning Cargo artifacts (external target dir)..."
if just cargo-clean; then
  echo "Æ’o. Cargo target cleaned at {{CARGO_TARGET_DIR}}"
else
  echo ""
  echo "Æ’?O Cargo clean failed"
  echo "Ensure cargo is installed and rerun: just cargo-clean"
  exit 1
fi

echo ""

# Check 3: Run codex-check to enforce hard invariants
echo "Running codex-check (hard invariants)..."
if just codex-check; then
  echo "Æ’o. Codex check passed"
else
  echo ""
  echo "Æ’?O Codex check FAILED"
  echo ""
  echo "Fix codex violations before committing."
  echo "See: {{CODEX_FILENAME}}"
  exit 1
fi

echo ""

# Check 4: Ensure no placeholder values in staged files
echo "Checking for placeholder values..."
PLACEHOLDERS=$(git diff --cached | grep -E '\{[a-z_]+\}' || true)
if [ -n "$PLACEHOLDERS" ]; then
  echo "Æ’sÃ¿â€¹,?  WARNING: Placeholder values detected in staged changes:"
  echo "$PLACEHOLDERS"
  echo ""
  echo "Ensure all {placeholder} values are replaced with actual values."
  echo "Proceeding with commit (warning only)..."
else
  echo "Æ’o. No placeholders detected"
fi

echo ""

# Check 5: Verify logger has recent entries (traceability)
echo "Checking logger traceability..."
LOGGER_FILES=$(find . -maxdepth 1 -name "{{PROJECT_PREFIX}}_logger_*.md" | sort -r | head -1)
if [ -z "$LOGGER_FILES" ]; then
  echo "Æ’sÃ¿â€¹,?  WARNING: No logger file found"
  echo "Consider adding a logger entry for traceability."
  echo "Proceeding with commit (warning only)..."
else
  # Check if logger has recent entries (modified in last 24 hours or RESULT field updated)
  LOGGER_MODIFIED=$(stat -c %Y "$LOGGER_FILES" 2>/dev/null || stat -f %m "$LOGGER_FILES" 2>/dev/null || echo "0")
  NOW=$(date +%s)
  AGE=$((NOW - LOGGER_MODIFIED))

  if [ $AGE -gt 86400 ]; then
    echo "Æ’sÃ¿â€¹,?  WARNING: Logger not updated recently (>24h)"
    echo "Consider adding a logger entry for this work."
    echo "Proceeding with commit (warning only)..."
  else
    echo "Æ’o. Logger recently updated"
  fi
fi

echo ""

# Check 6: Lint/format check for staged files
echo "Running quick lint check on staged files..."

# Check if any .rs files are staged
RUST_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)
if [ -n "$RUST_FILES" ]; then
  echo "Checking Rust formatting (staged files only)..."
  RUSTFMT_FAILED=0
  while IFS= read -r file; do
    if [ -z "$file" ]; then
      continue
    fi
    if [ ! -f "$file" ]; then
      continue
    fi
    if ! rustfmt --edition 2021 --check "$file"; then
      RUSTFMT_FAILED=1
    fi
  done <<< "$RUST_FILES"

  if [ $RUSTFMT_FAILED -eq 0 ]; then
    echo "Æ’o. Rust staged files formatted"
  else
    echo ""
    echo "Æ’?O Rust staged files not formatted"
    echo "Run: cd {{BACKEND_CRATE_DIR}} && cargo fmt"
    exit 1
  fi
fi

# Check if any .ts/.tsx/.js/.jsx files are staged
TS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(ts|tsx|js|jsx)$' || true)
if [ -n "$TS_FILES" ]; then
  echo "Checking TypeScript/JavaScript linting..."
  if cd {{FRONTEND_ROOT_DIR}} && pnpm run lint --quiet; then
    echo "Æ’o. Frontend files pass lint"
    cd - > /dev/null
  else
    cd - > /dev/null
    echo ""
    echo "Æ’?O Frontend lint failed"
    echo "Run: pnpm -C {{FRONTEND_ROOT_DIR}} run lint"
    exit 1
  fi
fi

echo ""
echo "Æ’o. Pre-commit validation passed"
echo ""
echo "Reminder: After commit, ensure you've run:"
echo "  - just post-work WP-{ID}  (if working on a task packet)"
echo "  - just validate  (for full hygiene check)"
echo ""

````

###### Template File: `.GOV/scripts/close-wp-branch.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
import { execFileSync } from "node:child_process";

function runGit(args, opts = {}) {
  return execFileSync("git", args, { stdio: "pipe", ...opts }).toString().trim();
}

function runGitInherit(args) {
  execFileSync("git", args, { stdio: "inherit" });
}

function fail(message, details = []) {
  console.error(`[CLOSE_WP_BRANCH] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail("Usage: node .GOV/scripts/close-wp-branch.mjs <WP_ID> [--remote]", [
    "Example (local only): node .GOV/scripts/close-wp-branch.mjs WP-1-MEX-v1.2-Runtime-v3",
    "Example (also delete origin branch): node .GOV/scripts/close-wp-branch.mjs WP-1-MEX-v1.2-Runtime-v3 --remote",
  ]);
}

function parseArgs() {
  const wpId = (process.argv[2] ?? "").trim();
  if (!wpId || !wpId.startsWith("WP-")) usage();
  const remote = process.argv.slice(3).includes("--remote");
  return { wpId, remote };
}

function branchForWp(wpId) {
  return `feat/${wpId}`;
}

function localBranchExists(branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`]);
    return true;
  } catch {
    return false;
  }
}

function isMergedIntoMain(branch) {
  try {
    execFileSync("git", ["merge-base", "--is-ancestor", branch, "main"]);
    return true;
  } catch {
    return false;
  }
}

function currentBranch() {
  return runGit(["branch", "--show-current"]);
}

function worktreeUsesBranch(branch) {
  const out = runGit(["worktree", "list", "--porcelain"]);
  const needle = `branch refs/heads/${branch}`;
  return out.split(/\r?\n/).some((line) => line.trim() === needle);
}

function remoteBranchExists(remoteName, branch) {
  try {
    const out = runGit(["ls-remote", "--heads", remoteName, branch]);
    return out.length > 0;
  } catch {
    return false;
  }
}

function main() {
  const { wpId, remote } = parseArgs();
  const branch = branchForWp(wpId);

  if (!localBranchExists(branch)) {
    fail("Local WP branch not found", [`branch=${branch}`]);
  }

  if (currentBranch() === branch) {
    fail("Cannot delete the currently checked-out branch", [
      `branch=${branch}`,
      "Checkout main first.",
    ]);
  }

  if (worktreeUsesBranch(branch)) {
    fail("A git worktree is still using this branch", [
      `branch=${branch}`,
      "Remove/move that worktree before closing the WP branch.",
    ]);
  }

  if (!isMergedIntoMain(branch)) {
    fail("WP branch is not merged into main; refusing to delete", [
      `branch=${branch}`,
      "Merge it into main first, or pass `--force` (not supported).",
    ]);
  }

  // The upstream safety check in `git branch -d` can block deletion even when the branch
  // is already merged into `main`. We already proved ancestry, so force-delete the pointer.
  runGitInherit(["branch", "-D", branch]);

  if (remote) {
    const remoteName = "origin";
    if (!remoteBranchExists(remoteName, branch)) {
      console.warn(`[CLOSE_WP_BRANCH] Remote branch not found; skipping: ${remoteName}/${branch}`);
      return;
    }
    runGitInherit(["push", remoteName, "--delete", branch]);
  }
}

main();

````

###### Template File: `.GOV/scripts/codex-check-test.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
import { execSync } from "node:child_process";

function run(command) {
  return execSync(command, { stdio: "pipe" }).toString();
}

function shouldFail(command, label) {
  try {
    execSync(command, { stdio: "pipe" });
    throw new Error(`${label} did not fail as expected`);
  } catch (err) {
    if (err && err.status === 1) {
      return;
    }
    throw err;
  }
}

console.log("codex-check-test: starting");

// Verify codex-check scripts exist and are runnable.
run("node .GOV/scripts/spec-current-check.mjs");

// Validate that the fetch guard is active by running it against a known test fixture.
shouldFail(
  'rg -n "\\bfetch\\s*\\(" .GOV/scripts/fixtures/forbidden_fetch.ts && exit 1 || exit 0',
  "fetch guard fixture",
);

// Validate that the TODO policy is enforced in the fixture.
// We avoid --pcre2 because it's not always available.
// Instead, we check if "TODO" exists but "TODO({{ISSUE_PREFIX}}-" doesn't.
shouldFail(
  'node -e "const out = require(\'child_process\').execSync(\'rg -n TODO .GOV/scripts/fixtures/forbidden_todo.txt\').toString(); if (out.split(\'\\n\').filter(Boolean).some(l => !/TODO\\({{ISSUE_PREFIX}}-\\d+\\)/.test(l))) process.exit(1)"',
  "TODO guard fixture",
);

console.log("codex-check-test ok");

````

###### Template File: `.GOV/scripts/create-task-packet.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
#!/usr/bin/env node
/**
 * Task packet generator [CX-580-581]
 * Creates a task packet from template
 */

import fs from 'fs';
import path from 'path';
import {
  defaultRefinementPath,
  resolveSpecCurrent,
  validateRefinementFile,
} from './validation/refinement-check.mjs';

const WP_ID = process.argv[2];

if (!WP_ID || !WP_ID.startsWith('WP-')) {
  console.error('âŒ Usage: node create-task-packet.mjs WP-{phase}-{name}');
  console.error('Example: node create-task-packet.mjs WP-1-Job-Cancel');
  process.exit(1);
}

// HARD GATE: Technical Refinement must exist and be signed before packet creation.
const refinementsDir = path.join('docs', 'refinements');
if (!fs.existsSync(refinementsDir)) {
  fs.mkdirSync(refinementsDir, { recursive: true });
}

const refinementPath = defaultRefinementPath(WP_ID);
let userSignature = '';

if (!fs.existsSync(refinementPath)) {
  const refinementTemplatePath = path.join('docs', 'templates', 'REFINEMENT_TEMPLATE.md');
  if (!fs.existsSync(refinementTemplatePath)) {
    console.error(`Missing refinement template: ${refinementTemplatePath}`);
    process.exit(1);
  }

  let resolved = null;
  try {
    resolved = resolveSpecCurrent();
  } catch {
    // Still create a scaffold deterministically; validation will fail until SPEC_CURRENT is resolvable.
  }

  const ts = new Date().toISOString();
  const raw = fs.readFileSync(refinementTemplatePath, 'utf8');
  const filled = raw
    .split('{{WP_ID}}').join(WP_ID)
    .split('{{DATE_ISO}}').join(ts)
    .split('{{SPEC_TARGET_RESOLVED}}').join(resolved ? resolved.specFileName : '{{PROJECT_PREFIX}}_Master_Spec_vXX.XX.md')
    .split('{{SPEC_TARGET_SHA1}}').join(resolved ? resolved.sha1 : '<fill>');

  fs.writeFileSync(refinementPath, filled, 'utf8');

  console.error('BLOCKED: Technical Refinement must be completed BEFORE task packet creation.');
  console.error(`Created refinement scaffold: ${refinementPath}`);
  console.error('Next steps:');
  console.error(`1) Fill ${refinementPath} (ASCII-only; token-in-window per SPEC_ANCHOR)`);
  console.error('2) Present refinement to the user (do NOT ask for signature in the same turn)');
  console.error(`3) Run: just record-refinement ${WP_ID}`);
  console.error(`4) After user review in a NEW turn, run: just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`);
  console.error(`5) Re-run: node .GOV/scripts/create-task-packet.mjs ${WP_ID}`);
  process.exit(2);
}

const refinementValidation = validateRefinementFile(refinementPath, { expectedWpId: WP_ID, requireSignature: true });
if (!refinementValidation.ok) {
  console.error(`BLOCKED: Refinement is not approved/signed: ${refinementPath}`);
  refinementValidation.errors.forEach((e) => console.error(`- ${e}`));
  console.error('Next steps:');
  console.error(`- Ensure ${refinementPath} is complete.`);
  console.error(`- Run: just record-refinement ${WP_ID}`);
  console.error(`- After user review, run: just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`);
  process.exit(1);
}

userSignature = refinementValidation.parsed.signature;

// HARD GATE: if refinement indicates enrichment is needed, do not create a task packet.
try {
  const refinementContent = fs.readFileSync(refinementPath, 'utf8');
  const m = refinementContent.match(/^\s*-\s*ENRICHMENT_NEEDED\s*:\s*(YES|NO)\s*$/mi);
  const enrichmentNeeded = (m?.[1] || '').toUpperCase();
  if (enrichmentNeeded === 'YES') {
    console.error(`BLOCKED: ${WP_ID} refinement declares ENRICHMENT_NEEDED=YES.`);
    console.error('Do NOT create/lock a WP packet while enrichment is required.');
    console.error('Next steps (spec-agnostic):');
    console.error('- Run the spec enrichment workflow (new spec version file + update .GOV/spec/SPEC_CURRENT.md).');
    console.error('- Create a NEW WP variant anchored to the updated spec (new WP_ID; new one-time signature).');
    process.exit(1);
  }
} catch {
  // If refinement cannot be read, earlier validation would have failed; keep defensive behavior deterministic.
  console.error(`BLOCKED: Unable to read refinement file: ${refinementPath}`);
  process.exit(1);
}

// Gate: signature must be recorded in ORCHESTRATOR_GATES.json (prevents manual bypass).
try {
  const gatesPath = path.join('docs', 'ORCHESTRATOR_GATES.json');
  const gates = JSON.parse(fs.readFileSync(gatesPath, 'utf8'));
  const logs = Array.isArray(gates.gate_logs) ? gates.gate_logs : [];
  const lastSig = [...logs].reverse().find((l) => l.wpId === WP_ID && l.type === 'SIGNATURE');
  if (!lastSig) {
    console.error(`BLOCKED: No signature record found for ${WP_ID} in ${gatesPath}.`);
    console.error(`Run: just record-signature ${WP_ID} ${userSignature}`);
    process.exit(1);
  }
  if (lastSig.signature !== userSignature) {
    console.error(`BLOCKED: Signature mismatch between refinement (${userSignature}) and gate log (${lastSig.signature}).`);
    process.exit(1);
  }

  // HARD GATE: worktree + coder assignment must be recorded AFTER signature and BEFORE packet creation.
  const lastPrepare = [...logs].reverse().find((l) => l.wpId === WP_ID && l.type === 'PREPARE');
  if (!lastPrepare) {
    console.error(`BLOCKED: WP branch/worktree + coder assignment not recorded for ${WP_ID}.`);
    console.error('Required workflow (stop-work gate):');
    console.error(`1) Create WP worktree: just worktree-add ${WP_ID}`);
    console.error(`2) Record assignment: just record-prepare ${WP_ID} {Coder-A|Coder-B}`);
    process.exit(1);
  }
  try {
    const sigTs = Date.parse(lastSig.timestamp);
    const prepTs = Date.parse(lastPrepare.timestamp);
    if (!Number.isNaN(sigTs) && !Number.isNaN(prepTs) && prepTs <= sigTs) {
      console.error(`BLOCKED: PREPARE record must occur after SIGNATURE for ${WP_ID}.`);
      console.error(`- signature_ts=${lastSig.timestamp}`);
      console.error(`- prepare_ts=${lastPrepare.timestamp}`);
      console.error(`Re-run: just record-prepare ${WP_ID} {Coder-A|Coder-B}`);
      process.exit(1);
    }
  } catch {
    // If timestamps are unparsable, treat as blocked to preserve determinism.
    console.error(`BLOCKED: Unable to verify PREPARE ordering for ${WP_ID}.`);
    console.error(`Re-run: just record-prepare ${WP_ID} {Coder-A|Coder-B}`);
    process.exit(1);
  }
} catch {
  console.error('BLOCKED: Unable to verify signature in .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json.');
  process.exit(1);
}

// Gate: signature must be present in SIGNATURE_AUDIT.md (protocol requirement).
try {
  const auditPath = path.join('docs', 'SIGNATURE_AUDIT.md');
  const audit = fs.readFileSync(auditPath, 'utf8');
  if (!audit.includes(`| ${userSignature} |`)) {
    console.error(`BLOCKED: Signature not found in ${auditPath}.`);
    console.error(`Run: just record-signature ${WP_ID} ${userSignature} (this appends to the audit log).`);
    process.exit(1);
  }
} catch {
  console.error('BLOCKED: Unable to verify signature in .GOV/roles_shared/records/SIGNATURE_AUDIT.md.');
  process.exit(1);
}

// Ensure directory exists
const taskPacketDir = '.GOV/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  fs.mkdirSync(taskPacketDir, { recursive: true });
  console.log(`Created directory: ${taskPacketDir}/`);
}

const fileName = `${WP_ID}.md`;
const filePath = path.join(taskPacketDir, fileName);

// Check if file already exists
if (fs.existsSync(filePath)) {
  console.error(`âŒ Task packet already exists: ${filePath}`);
  console.error('Edit the existing file or use a different WP_ID.');
  process.exit(1);
}

// Get current timestamp
const timestamp = new Date().toISOString();

// Template content (canonical)
const templatePath = path.join('docs', 'templates', 'TASK_PACKET_TEMPLATE.md');
if (!fs.existsSync(templatePath)) {
  console.error(`Æ’?O Missing template: ${templatePath}`);
  process.exit(1);
}

const rawTemplate = fs.readFileSync(templatePath, 'utf8');
const templateLines = rawTemplate.split('\n');
const templateStartIdx = templateLines.findIndex((line) => line.startsWith('# Task Packet:'));
const templateBody = templateStartIdx === -1
  ? rawTemplate
  : templateLines.slice(templateStartIdx).join('\n');

  let specBaseline = '{{PROJECT_PREFIX}}_Master_Spec_vXX.XX.md';
  try {
    const specCurrent = fs.readFileSync(path.join('docs', 'SPEC_CURRENT.md'), 'utf8');
    const m = specCurrent.match(/{{PROJECT_PREFIX}}_Master_Spec_v[0-9.]+\.md/);
    if (m) specBaseline = m[0];
  } catch {
    // Leave placeholder if SPEC_CURRENT cannot be read or parsed.
  }

const fill = (text, token, value) => text.split(token).join(value);

let template = templateBody;
template = fill(template, '{{WP_ID}}', WP_ID);
template = fill(template, '{{DATE_ISO}}', timestamp);
template = fill(template, '{{SPEC_BASELINE}}', specBaseline);
template = fill(template, '{{REQUESTOR}}', '{user or source}');
template = fill(template, '{{AGENT_ID}}', '{orchestrator agent ID}');
template = fill(template, '{{USER_SIGNATURE}}', userSignature);
template = fill(template, '{{SPEC_ANCHOR}}', '<fill>');

// Write the file
fs.writeFileSync(filePath, template, 'utf8');

console.log(`âœ… Task packet created: ${filePath}`);
console.log('');
console.log('Next steps:');
console.log('1. Edit the file and fill in all {placeholder} values');
console.log('2. Update .GOV/roles_shared/records/TASK_BOARD.md to "Ready for Dev"');
console.log('3. Verify completeness: just pre-work ' + WP_ID);
console.log('4. Delegate to coder with packet path');
console.log('');
console.log('Template fields to complete:');
console.log('- Metadata: REQUESTOR, AGENT_ID');
console.log('- SCOPE: What, Why, IN_SCOPE_PATHS, OUT_OF_SCOPE');
console.log('- RISK_TIER: Choose LOW/MEDIUM/HIGH');
console.log('- TEST_PLAN: List specific commands');
console.log('- DONE_MEANS: Define success criteria');
console.log('- BOOTSTRAP: Fill in FILES_TO_OPEN, SEARCH_TERMS, RISK_MAP');
console.log('- AUTHORITY: Fill SPEC_ANCHOR; keep SPEC_BASELINE as provenance');

````

###### Template File: `.GOV/scripts/new-api-endpoint.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
import fs from "node:fs";
import path from "node:path";

function usage() {
  console.error("Usage: node .GOV/scripts/new-api-endpoint.mjs <endpoint_name>");
  console.error("Example: node .GOV/scripts/new-api-endpoint.mjs canvas_ping");
}

function toSnakeCase(input) {
  return input
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[^a-zA-Z0-9]+/g, "_")
    .replace(/_{2,}/g, "_")
    .replace(/^_+|_+$/g, "")
    .toLowerCase();
}

const rawName = process.argv[2];
if (!rawName) {
  usage();
  process.exit(1);
}

if (/[\\/]/.test(rawName)) {
  console.error("Endpoint name must not include path separators.");
  usage();
  process.exit(1);
}

const moduleName = toSnakeCase(rawName);
if (!moduleName) {
  console.error("Invalid endpoint name.");
  usage();
  process.exit(1);
}

if (moduleName === "mod") {
  console.error("Endpoint name 'mod' is reserved.");
  process.exit(1);
}

const routeSegment = moduleName.replace(/_/g, "-");
const apiDir = path.join(process.cwd(), "{{BACKEND_SRC_DIR}}", "api");
const modulePath = path.join(apiDir, `${moduleName}.rs`);
const modPath = path.join(apiDir, "mod.rs");

if (fs.existsSync(modulePath)) {
  console.error(`Module already exists: ${modulePath}`);
  process.exit(1);
}

if (!fs.existsSync(modPath)) {
  console.error(`Missing mod.rs: ${modPath}`);
  process.exit(1);
}

const moduleTemplate = `use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct PingResponse {
    status: &'static str,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/${routeSegment}/ping", get(ping))
        .with_state(state)
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse { status: "ok" })
}
`;

fs.writeFileSync(modulePath, moduleTemplate, "utf8");

const modContent = fs.readFileSync(modPath, "utf8");
if (modContent.includes(`pub mod ${moduleName};`)) {
  console.error(`Module already listed in mod.rs: ${moduleName}`);
  process.exit(1);
}

const lines = modContent.split("\n");
const lastPubModIndex = [...lines]
  .map((line, index) => ({ line, index }))
  .filter(({ line }) => line.trim().startsWith("pub mod "))
  .map(({ index }) => index)
  .pop();

if (lastPubModIndex === undefined) {
  console.error("No pub mod declarations found in mod.rs.");
  process.exit(1);
}

lines.splice(lastPubModIndex + 1, 0, `pub mod ${moduleName};`);

const logRoutesIndex = lines.findIndex((line) => line.includes("let log_routes ="));
if (logRoutesIndex === -1) {
  console.error("Could not find log_routes declaration in mod.rs.");
  process.exit(1);
}

const logRoutesEndIndex = lines
  .slice(logRoutesIndex)
  .findIndex((line) => line.trim().endsWith(";"));
if (logRoutesEndIndex === -1) {
  console.error("Could not find end of log_routes declaration in mod.rs.");
  process.exit(1);
}

const insertIndex = logRoutesIndex + logRoutesEndIndex + 1;
lines.splice(insertIndex, 0, `    let ${moduleName}_routes = ${moduleName}::routes(state.clone());`);

const mergeIndex = lines.findIndex((line) => line.includes(".merge(log_routes)"));
if (mergeIndex === -1) {
  console.error("Could not find merge(log_routes) chain in mod.rs.");
  process.exit(1);
}

if (!lines[mergeIndex].includes(`${moduleName}_routes`)) {
  lines[mergeIndex] = lines[mergeIndex].replace(
    ".merge(log_routes)",
    `.merge(log_routes).merge(${moduleName}_routes)`,
  );
}

fs.writeFileSync(modPath, lines.join("\n"), "utf8");

console.log(`Created ${modulePath}`);
console.log(`Updated ${modPath}`);

````

###### Template File: `.GOV/scripts/new-react-component.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
import fs from "node:fs";
import path from "node:path";

function usage() {
  console.error("Usage: node .GOV/scripts/new-react-component.mjs <ComponentName>");
}

function toPascalCase(input) {
  return input
    .replace(/[^a-zA-Z0-9]+/g, " ")
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join("");
}

const rawName = process.argv[2];
if (!rawName) {
  usage();
  process.exit(1);
}

if (/[\\/]/.test(rawName)) {
  console.error("Component name must not include path separators.");
  usage();
  process.exit(1);
}

const componentName = toPascalCase(rawName);
if (!componentName) {
  console.error("Invalid component name.");
  usage();
  process.exit(1);
}

const componentsDir = path.join(process.cwd(), "{{FRONTEND_SRC_DIR}}", "components");
const componentPath = path.join(componentsDir, `${componentName}.tsx`);
const testPath = path.join(componentsDir, `${componentName}.test.tsx`);

if (!fs.existsSync(componentsDir)) {
  fs.mkdirSync(componentsDir, { recursive: true });
}

if (fs.existsSync(componentPath) || fs.existsSync(testPath)) {
  console.error("Component files already exist.");
  process.exit(1);
}

const componentTemplate = `export function ${componentName}() {
  return (
    <div className="${componentName.toLowerCase()}">
      <h2>${componentName}</h2>
    </div>
  );
}
`;

const testTemplate = `import { render, screen } from "@testing-library/react";
import { ${componentName} } from "./${componentName}";

describe("${componentName}", () => {
  it("renders", () => {
    render(<${componentName} />);
    expect(screen.getByText("${componentName}")).toBeInTheDocument();
  });
});
`;

fs.writeFileSync(componentPath, componentTemplate, "utf8");
fs.writeFileSync(testPath, testTemplate, "utf8");

console.log(`Created ${componentPath}`);
console.log(`Created ${testPath}`);

````

###### Template File: `.GOV/scripts/scaffold-check.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { execSync } from "node:child_process";

function fail(message) {
  throw new Error(message);
}

const repoRoot = process.cwd();
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "{{PROJECT_PREFIX}}-scaffold-"));
let exitCode = 0;

try {
  const apiDir = path.join(tmpRoot, "{{BACKEND_SRC_DIR}}", "api");
  fs.mkdirSync(apiDir, { recursive: true });

  const modPath = path.join(apiDir, "mod.rs");
  fs.writeFileSync(
    modPath,
    [
      "use axum::{routing::get, Router};",
      "",
      "use crate::AppState;",
      "",
      "pub mod canvases;",
      "pub mod logs;",
      "pub mod paths;",
      "pub mod workspaces;",
      "",
      "pub fn routes(state: AppState) -> Router {",
      "    let workspace_routes = workspaces::routes(state.clone());",
      "    let canvas_routes = canvases::routes(state.clone());",
      "    let log_routes = Router::new()",
      "        .route(\"/logs/tail\", get(logs::tail_logs))",
      "        .with_state(state.clone());",
      "",
      "    workspace_routes.merge(canvas_routes).merge(log_routes)",
      "}",
      "",
    ].join("\n"),
    "utf8",
  );

  const componentsDir = path.join(tmpRoot, "{{FRONTEND_SRC_DIR}}", "components");
  fs.mkdirSync(componentsDir, { recursive: true });

  execSync(`node "${path.join(repoRoot, "scripts", "new-api-endpoint.mjs")}" sample_ping`, {
    cwd: tmpRoot,
    stdio: "inherit",
  });
  execSync(`node "${path.join(repoRoot, "scripts", "new-react-component.mjs")}" SampleWidget`, {
    cwd: tmpRoot,
    stdio: "inherit",
  });

  const apiModulePath = path.join(apiDir, "sample_ping.rs");
  const componentPath = path.join(componentsDir, "SampleWidget.tsx");
  const testPath = path.join(componentsDir, "SampleWidget.test.tsx");

  if (!fs.existsSync(apiModulePath)) fail("API scaffold missing module file.");
  if (!fs.existsSync(componentPath)) fail("React scaffold missing component file.");
  if (!fs.existsSync(testPath)) fail("React scaffold missing test file.");

  const modContent = fs.readFileSync(modPath, "utf8");
  if (!modContent.includes("pub mod sample_ping;")) fail("mod.rs missing pub mod sample_ping;");
  if (!modContent.includes("let sample_ping_routes = sample_ping::routes(state.clone());")) {
    fail("mod.rs missing sample_ping routes wiring.");
  }
  if (!modContent.includes(".merge(log_routes).merge(sample_ping_routes)")) {
    fail("mod.rs missing merge(sample_ping_routes).");
  }

  console.log("scaffold-check ok");
} catch (err) {
  const message = err instanceof Error ? err.message : String(err);
  console.error(message);
  exitCode = 1;
} finally {
  fs.rmSync(tmpRoot, { recursive: true, force: true });
}

process.exit(exitCode);

````

###### Template File: `.GOV/scripts/spec-current-check.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
import fs from "node:fs";
import path from "node:path";

function parseVersion(name) {
  const match = name.match(/_v(\d+(?:\.\d+)*)\.md$/);
  if (!match) return null;
  return match[1].split(".").map((part) => Number(part));
}

function compareVersions(a, b) {
  const maxLen = Math.max(a.length, b.length);
  for (let i = 0; i < maxLen; i += 1) {
    const left = a[i] ?? 0;
    const right = b[i] ?? 0;
    if (left !== right) return left - right;
  }
  return 0;
}

const repoRoot = process.cwd();
const specFiles = fs
  .readdirSync(repoRoot)
  .filter((name) => name.startsWith("{{PROJECT_PREFIX}}_Master_Spec_v") && name.endsWith(".md"));

if (specFiles.length === 0) {
  console.error("No {{PROJECT_PREFIX}}_Master_Spec_v*.md files found in repo root.");
  process.exit(1);
}

const parsed = specFiles
  .map((name) => ({ name, version: parseVersion(name) }))
  .filter((item) => Array.isArray(item.version));

if (parsed.length === 0) {
  console.error("Failed to parse spec versions from {{PROJECT_PREFIX}}_Master_Spec_v*.md.");
  process.exit(1);
}

parsed.sort((a, b) => compareVersions(a.version, b.version));
const latest = parsed[parsed.length - 1].name;

const specCurrentPath = path.join(repoRoot, "docs", "SPEC_CURRENT.md");
if (!fs.existsSync(specCurrentPath)) {
  console.error(".GOV/spec/SPEC_CURRENT.md not found.");
  process.exit(1);
}

const specCurrent = fs.readFileSync(specCurrentPath, "utf8");
if (!specCurrent.includes(latest)) {
  console.error(`SPEC_CURRENT does not reference latest spec: ${latest}`);
  process.exit(1);
}

console.log(`SPEC_CURRENT ok: ${latest}`);

````

###### Template File: `.GOV/scripts/worktree-add.mjs`
Intent: Repo script (governance support or scaffolding helper).
````js
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

function runGit(args) {
  return execFileSync("git", args, { stdio: "pipe" }).toString().trim();
}

function runGitInherit(args) {
  execFileSync("git", args, { stdio: "inherit" });
}

function fail(message) {
  console.error(`[WORKTREE_ADD] ${message}`);
  process.exit(1);
}

function isBranchPresent(branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`]);
    return true;
  } catch {
    return false;
  }
}

function main() {
  const wpId = process.argv[2]?.trim();
  if (!wpId) {
    fail(
      "Usage: node .GOV/scripts/worktree-add.mjs <WP_ID> [base=main] [branch=feat/WP_ID] [dir=../wt-WP_ID]"
    );
  }

  const base = (process.argv[3] ?? "main").trim() || "main";
  const branch = (process.argv[4] ?? "").trim() || `feat/${wpId}`;
  const dir = (process.argv[5] ?? "").trim() || path.join("..", `wt-${wpId}`);

  const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
  const absDir = path.resolve(repoRoot, dir);

  if (fs.existsSync(absDir)) {
    fail(`Target directory already exists: ${absDir}`);
  }

  const alreadyHaveBranch = isBranchPresent(branch);
  if (alreadyHaveBranch) {
    console.log(`[WORKTREE_ADD] Using existing branch: ${branch}`);
    runGitInherit(["worktree", "add", absDir, branch]);
  } else {
    console.log(`[WORKTREE_ADD] Creating branch ${branch} from ${base}`);
    runGitInherit(["worktree", "add", "-b", branch, absDir, base]);
  }

  console.log("");
  console.log(`[WORKTREE_ADD] Worktree ready: ${absDir}`);
  console.log(`[WORKTREE_ADD] Next: cd "${absDir}"`);
}

main();

````

###### Template File: `.GOV/scripts/README.md`
Intent: Script directory documentation (how to use gates/tools).
````md
Dev and ops scripts live here.

Scaffolding:
- `node .GOV/scripts/new-react-component.mjs <ComponentName>` creates `{{FRONTEND_SRC_DIR}}/components/<ComponentName>.tsx` and a basic test.
- `node .GOV/scripts/new-api-endpoint.mjs <endpoint_name>` creates `{{BACKEND_SRC_DIR}}/api/<endpoint_name>.rs` and wires it into `api/mod.rs`.
- `node .GOV/scripts/scaffold-check.mjs` validates scaffolding output against a temporary workspace.

Git hooks:
- `.GOV/scripts/hooks/pre-commit` runs local hygiene checks before commits.
- Enable with `git config core.hooksPath .GOV/scripts/hooks`.

````

###### Template File: `.GOV/scripts/fixtures/forbidden_fetch.ts`
Intent: Fixture for codex-check-test (ensures gates fail when they should).
````ts
export async function badFetch() {
  const response = await fetch("https://example.com");
  return response.ok;
}

````

###### Template File: `.GOV/scripts/fixtures/forbidden_todo.txt`
Intent: Fixture for codex-check-test (ensures gates fail when they should).
````text
TODO: add proper issue tracking

````

###### Template File: `.GOV/scripts/validation/ci-traceability-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * CI traceability check [CX-903]
 * Validates commit messages reference WP_IDs and that task packets exist.
 * Task Board + task packets are the primary micro-log; logger is optional (milestones/hard bugs only).
 */

import { execSync } from 'child_process';
import fs from 'fs';

console.log('\ndY"? CI Traceability Check (Codex v{{CODEX_VERSION}})...\n');

let errors = [];
let warnings = [];

// Get recent commits (last 10)
let commits;
try {
  const commitOutput = execSync('git log -10 --pretty=format:"%H|%s|%an|%ae"', {
    encoding: 'utf8',
  });
  commits = commitOutput
    .split('\n')
    .filter(Boolean)
    .map(line => {
      const [hash, subject, author, email] = line.split('|');
      return { hash, subject, author, email };
    });
} catch (err) {
  console.error('Æ’?O Could not retrieve git commits');
  console.error(err.message);
  process.exit(1);
}

console.log(`Found ${commits.length} recent commits to check\n`);

// Check 1: WP_ID references in commits
console.log('Check 1: WP_ID references in commits');
const wpIdPattern = /WP-[\w-]+/;
const commitsWithWpId = commits.filter(c => wpIdPattern.test(c.subject));
const commitsWithoutWpId = commits.filter(c => !wpIdPattern.test(c.subject));

console.log(`  Æ’o. ${commitsWithWpId.length} commits reference WP_ID`);
if (commitsWithoutWpId.length > 0) {
  console.log(`  Æ’sÃ¿â€¹,?  ${commitsWithoutWpId.length} commits without WP_ID:`);
  commitsWithoutWpId.slice(0, 3).forEach(c => {
    console.log(`    - ${c.hash.slice(0, 7)}: ${c.subject}`);
  });
  warnings.push(
    `${commitsWithoutWpId.length} commits without WP_ID reference`
  );
}

// Check 2: Task packets exist for referenced WP_IDs
console.log('\nCheck 2: Task packets exist for referenced WP_IDs');
const taskPacketDir = '.GOV/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  errors.push('.GOV/task_packets/ directory does not exist [CX-213]');
  console.log('Æ’?O FAIL: No task_packets directory');
  console.log('  Run: mkdir -p .GOV/task_packets');
} else {
  const taskPackets = fs
    .readdirSync(taskPacketDir)
    .filter(f => f.endsWith('.md'));
  console.log(`  Æ’o. .GOV/task_packets/ exists (${taskPackets.length} packets)`);

  const missingPackets = [];
  commitsWithWpId.forEach(commit => {
    const wpId = commit.subject.match(wpIdPattern)?.[0];
    if (wpId) {
      const hasPacket = taskPackets.some(p => p.includes(wpId));
      if (!hasPacket) {
        missingPackets.push({ commit, wpId });
      }
    }
  });

  if (missingPackets.length > 0) {
    console.log(
      `  Æ’sÃ¿â€¹,?  ${missingPackets.length} WP_IDs in commits without task packet files:`
    );
    missingPackets.slice(0, 3).forEach(({ commit, wpId }) => {
      console.log(`    - ${commit.hash.slice(0, 7)}: ${wpId}`);
    });
    errors.push(
      `${missingPackets.length} commits reference WP_ID without matching task packet`
    );
  } else {
    console.log('  Æ’o. All WP_IDs in commits have task packets');
  }
}

// Optional: Logger presence (info only)
console.log('\nCheck 3: Logger (optional, milestones/hard bugs)');
const loggerFiles = fs
  .readdirSync('.')
  .filter(f => f.startsWith('{{PROJECT_PREFIX}}_logger_') && f.endsWith('.md'))
  .sort()
  .reverse();
if (loggerFiles.length === 0) {
  console.log('  â„¹ï¸  Logger not present (optional)');
} else {
  console.log(`  â„¹ï¸  Logger present: ${loggerFiles[0]} (milestones/hard bugs only)`);
}

// Check 4: Codex v{{CODEX_VERSION}} exists
console.log('\nCheck 4: Codex v{{CODEX_VERSION}} exists');
if (!fs.existsSync('{{CODEX_FILENAME}}')) {
  errors.push('{{CODEX_FILENAME}} not found in repository root');
  console.log('Æ’?O FAIL: Codex v{{CODEX_VERSION}} missing');
} else {
  console.log('  Æ’o. {{CODEX_FILENAME}} exists');
}

// Check 5: Protocol files exist
console.log('\nCheck 5: Protocol files exist');
const protocolFiles = [
  '.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md',
  '.GOV/roles/coder/CODER_PROTOCOL.md',
];

protocolFiles.forEach(file => {
  if (!fs.existsSync(file)) {
    errors.push(`${file} not found [CX-900]`);
    console.log(`  Æ’?O FAIL: ${file} missing`);
  } else {
    console.log(`  Æ’o. ${file} exists`);
  }
});

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0 && warnings.length === 0) {
  console.log('Æ’o. CI traceability check PASSED\n');
  process.exit(0);
} else if (errors.length === 0 && warnings.length > 0) {
  console.log('Æ’sÃ¿â€¹,?  CI traceability check PASSED with warnings\n');
  console.log('Warnings:');
  warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  console.log('\nWarnings do not block CI, but should be addressed.');
  process.exit(0);
} else {
  console.log('Æ’?O CI traceability check FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues to pass CI traceability check.');
  console.log('See: {{CODEX_FILENAME}}');
  process.exit(1);
}

````

###### Template File: `.GOV/scripts/validation/codex-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

function fail(message, details = "") {
  console.error(message);
  if (details) {
    console.error(details);
  }
  process.exit(1);
}

function listFilesRecursive(rootDir) {
  const out = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const current = stack.pop();
    if (!current) continue;
    let entries;
    try {
      entries = fs.readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (entry.isFile()) {
        out.push(full);
      }
    }
  }
  return out;
}

function toPosix(p) {
  return p.split(path.sep).join("/");
}

function findLineHits(filePath, predicate) {
  let content;
  try {
    content = fs.readFileSync(filePath, "utf8");
  } catch {
    return [];
  }
  const lines = content.split(/\r?\n/);
  const hits = [];
  const relPath = toPosix(path.relative(repoRoot, filePath));
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (predicate(line)) {
      hits.push(`${relPath}:${i + 1}:${line}`);
    }
  }
  return hits;
}

process.chdir(repoRoot);

// 1) Spec drift guard: SPEC_CURRENT must point to the latest master spec.
await import("../spec-current-check.mjs");

// 2) Frontend fetch guard: only the shared API client may call fetch.
{
  const tsFiles = listFilesRecursive(path.join(repoRoot, "{{FRONTEND_SRC_DIR}}")).filter((filePath) => {
    const ext = path.extname(filePath);
    return ext === ".ts" || ext === ".tsx";
  });
  const hits = tsFiles.flatMap((filePath) =>
    findLineHits(filePath, (line) => /\bfetch\s*\(/.test(line))
  );
  const violations = hits.filter((hit) => !hit.startsWith("{{FRONTEND_SRC_DIR}}/lib/api.ts:"));
  if (violations.length > 0) {
    fail("Forbidden fetch() usage outside API client:", violations.join("\n"));
  }
}

// 3) Backend println/eprintln guard: disallow direct stdout logging in production code.
{
  const rustFiles = listFilesRecursive(path.join(repoRoot, "{{BACKEND_SRC_DIR}}")).filter(
    (filePath) => path.extname(filePath) === ".rs"
  );
  const hits = rustFiles.flatMap((filePath) =>
    findLineHits(filePath, (line) => line.includes("println!") || line.includes("eprintln!"))
  );
  if (hits.length > 0) {
    fail("Forbidden println!/eprintln! in backend source:", hits.join("\n"));
  }
}

// 4) TODO tagging guard: TODOs must be annotated with {{ISSUE_PREFIX}} issue IDs.
{
  const roots = [
    path.join(repoRoot, "{{BACKEND_SRC_DIR}}"),
    path.join(repoRoot, "{{FRONTEND_SRC_DIR}}")
  ];
  const files = roots
    .flatMap((root) => listFilesRecursive(root))
    .filter((filePath) => [".rs", ".ts", ".tsx"].includes(path.extname(filePath)));

  const hits = files.flatMap((filePath) => findLineHits(filePath, (line) => line.includes("TODO")));
  const violations = hits.filter((hit) => !/TODO\({{ISSUE_PREFIX}}-\d+\)/.test(hit));
  if (violations.length > 0) {
    fail("Untracked TODOs found (require TODO({{ISSUE_PREFIX}}-####)):", violations.join("\n"));
  }
}

// 5) Task board guard: keep Done/Superseded minimal and machine-checkable.
await import("./task-board-check.mjs");
await import("./task-packet-claim-check.mjs");
await import("./worktree-concurrency-check.mjs");

console.log("codex-check ok");

````

###### Template File: `.GOV/scripts/validation/cor701-sha.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * COR-701 SHA helper
 * - Prints deterministic Pre/Post SHA1 values for a target file.
 * - Prefers Git blobs (HEAD/INDEX) and normalizes LF/CRLF variants for human convenience.
 */

import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';

const targetArg = process.argv[2];
if (!targetArg) {
  console.error('Usage: node .GOV/scripts/validation/cor701-sha.mjs <path/to/file>');
  process.exit(1);
}

const normalizeLf = (text) => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
const normalizeCrlf = (text) => normalizeLf(text).replace(/\n/g, '\r\n');
const sha1Hex = (bufOrString) => crypto.createHash('sha1').update(bufOrString).digest('hex');
const isLikelyText = (buf) => !buf.includes(0);

const shaVariantsForText = (text) => {
  const lf = normalizeLf(text);
  return {
    lf: sha1Hex(lf),
    crlf: sha1Hex(normalizeCrlf(lf)),
  };
};

const shaVariantsForBlob = (buf) => {
  const lf = sha1Hex(buf);
  if (!isLikelyText(buf)) return { lf, crlf: lf };
  const { crlf } = shaVariantsForText(buf.toString('utf8'));
  return { lf, crlf };
};

const shaVariantsForWorktree = (filePath) => {
  const buf = fs.readFileSync(filePath);
  const raw = sha1Hex(buf);
  if (!isLikelyText(buf)) return { raw, lf: raw, crlf: raw };
  const { lf, crlf } = shaVariantsForText(buf.toString('utf8'));
  return { raw, lf, crlf };
};

const gitPath = path.normalize(targetArg).replace(/\\/g, '/');

const tryGitBuffer = (command) => {
  try {
    return execSync(command);
  } catch {
    return null;
  }
};

const tryGitTrim = (command) => {
  try {
    return execSync(command, { encoding: 'utf8' }).trim();
  } catch {
    return '';
  }
};

const headBuf = tryGitBuffer(`git show HEAD:${gitPath}`);
const indexBuf = tryGitBuffer(`git show :${gitPath}`);
const stagedNameOnly = tryGitTrim(`git diff --name-only --cached -- "${gitPath}"`);
const isStaged = stagedNameOnly.split('\n').map((l) => l.trim()).filter(Boolean).includes(gitPath);

const worktreePath = path.normalize(targetArg);
const hasWorktree = fs.existsSync(worktreePath);
const worktree = hasWorktree ? shaVariantsForWorktree(worktreePath) : null;

const head = headBuf ? shaVariantsForBlob(headBuf) : null;
const index = indexBuf ? shaVariantsForBlob(indexBuf) : null;

const recommendedPre = head?.lf || '<untracked>';
const recommendedPost = isStaged ? (index?.lf || '<untracked>') : (worktree?.lf || '<missing>');

console.log(`\nCOR-701 SHA helper: ${gitPath}\n`);

console.log('SHA variants:');
if (head) {
  console.log(`- HEAD (LF blob):   ${head.lf}`);
  if (head.crlf !== head.lf) console.log(`- HEAD (CRLF alt):  ${head.crlf}`);
} else {
  console.log('- HEAD:             <untracked/new file>');
}

if (index) {
  console.log(`- INDEX (LF blob):  ${index.lf}${isStaged ? '' : ' (NOTE: file not staged; INDEX may not include your changes)'}`);
  if (index.crlf !== index.lf) console.log(`- INDEX (CRLF alt): ${index.crlf}`);
} else {
  console.log('- INDEX:            <untracked/new file>');
}

if (worktree) {
  console.log(`- WORKTREE (raw):   ${worktree.raw}`);
  if (worktree.lf !== worktree.raw) console.log(`- WORKTREE (LF):    ${worktree.lf}`);
  if (worktree.crlf !== worktree.raw && worktree.crlf !== worktree.lf) console.log(`- WORKTREE (CRLF):  ${worktree.crlf}`);
} else {
  console.log('- WORKTREE:         <missing on disk>');
}

console.log('\nRecommended for task packet manifest:');
console.log(`- **Pre-SHA1**: \`${recommendedPre}\``);
console.log(`- **Post-SHA1**: \`${recommendedPost}\``);

if (!isStaged) {
  console.log('\nNOTE: For deterministic Post-SHA1, stage the file before copying Post-SHA1 (so it comes from INDEX).');
}


````

###### Template File: `.GOV/scripts/validation/cor701-spec.json`
Intent: Mechanical governance gate (see filename + internal docstrings).
````json
{
  "requiredFields": [
    "target_file",
    "start",
    "end",
    "pre_sha1",
    "post_sha1",
    "line_delta",
    "gates_passed"
  ],
  "requiredGates": [
    "anchors_present",
    "window_matches_plan",
    "rails_untouched_outside_window",
    "filename_canonical_and_openable",
    "pre_sha1_captured",
    "post_sha1_captured",
    "line_delta_equals_expected",
    "all_links_resolvable",
    "manifest_written_and_path_returned",
    "current_file_matches_preimage"
  ],
  "gateErrorCodes": {
    "anchors_present": "C701-G01",
    "window_matches_plan": "C701-G02",
    "rails_untouched_outside_window": "C701-G04",
    "filename_canonical_and_openable": "C701-G06",
    "pre_sha1_captured": "C701-G05",
    "post_sha1_captured": "C701-G05",
    "line_delta_equals_expected": "C701-G05",
    "all_links_resolvable": "C701-G05",
    "manifest_written_and_path_returned": "C701-G05",
    "current_file_matches_preimage": "C701-G08"
  }
}

````

###### Template File: `.GOV/scripts/validation/gate-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
import fs from 'node:fs';
import path from 'node:path';

/**
 * [CX-GATE-001] Binary Phase Gate Validator
 * Enforces ordered phases and prevents merged turns.
 *
 * Hardened per WP-1-Gate-Check-Tool-v2:
 * - Line-based parsing with fenced code block tracking
 * - Detects phases via heading lines only (outside fences)
 * - Detects approval via dedicated marker line (outside fences)
 */

const wpId = process.argv[2];
if (!wpId) {
    console.error("Usage: node gate-check.mjs <WP_ID>");
    process.exit(1);
}

const wpPath = path.join(process.cwd(), 'docs', 'task_packets', `${wpId}.md`);
if (!fs.existsSync(wpPath)) {
    console.error(`? GATE FAIL: Task Packet ${wpId}.md not found.`);
    process.exit(1);
}

const content = fs.readFileSync(wpPath, 'utf8');

/**
 * Parse content line-by-line, tracking fenced code block state.
 * Returns positions of valid markers found OUTSIDE code fences only.
 *
 * @param {string} content - The markdown content to parse
 * @returns {Object} ParseResult with marker positions and flags
 */
function parseMarkersFromContent(content) {
    const lines = content.split('\n');
    let inCodeFence = false;

    const result = {
        bootstrapHeadingLine: -1,
        skeletonHeadingLine: -1,
        approvalMarkerLine: -1,
        implementationDetected: false,
        statusInProgress: false
    };

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trim();

        // Toggle fence state on ``` lines (trimmed line starts with ```)
        if (trimmed.startsWith('```')) {
            inCodeFence = !inCodeFence;
            continue;
        }

        // Skip all marker detection inside fenced code blocks
        if (inCodeFence) continue;

        // Detect BOOTSTRAP heading (heading syntax only, outside fence)
        if (/^#{1,6}\s+BOOTSTRAP\b/i.test(line)) {
            if (result.bootstrapHeadingLine === -1) {
                result.bootstrapHeadingLine = i;
            }
        }

        // Detect SKELETON heading (heading syntax only, outside fence)
        if (/^#{1,6}\s+SKELETON\b/i.test(line)) {
            if (result.skeletonHeadingLine === -1) {
                result.skeletonHeadingLine = i;
            }
        }

        // Detect approval marker: trimmed line must equal "SKELETON APPROVED" exactly
        if (trimmed === 'SKELETON APPROVED') {
            if (result.approvalMarkerLine === -1) {
                result.approvalMarkerLine = i;
            }
        }

        // Detect implementation evidence (heading syntax only, outside fence)
        if (/^#{1,6}\s+VALIDATION\s*\(Coder\)/i.test(line) ||
            /^#{1,6}\s+VALIDATION REPORT\b/i.test(line)) {
            result.implementationDetected = true;
        }

        // Detect status (outside fence)
        if (/Status:\s*In[- ]?Progress/i.test(line)) {
            result.statusInProgress = true;
        }
    }

    return result;
}

// Parse the content
const parsed = parseMarkersFromContent(content);

console.log(`Checking Phase Gate for ${wpId}...`);

// Validation 1: Mandatory checkpoints for "In Progress"
if (parsed.statusInProgress && parsed.bootstrapHeadingLine === -1) {
    console.error("? GATE FAIL: 'In Progress' status requires a BOOTSTRAP block.");
    process.exit(1);
}

// Validation 2: Interface-First Invariant [CX-625]
if (parsed.implementationDetected && parsed.approvalMarkerLine === -1) {
    console.error("? GATE FAIL: Implementation detected without SKELETON APPROVED marker.");
    process.exit(1);
}

// Validation 3: Anti-Turn-Merging (Heuristic)
const missingPhases = [];
if (parsed.bootstrapHeadingLine === -1) missingPhases.push('BOOTSTRAP');
if (parsed.skeletonHeadingLine === -1) missingPhases.push('SKELETON');
if (parsed.approvalMarkerLine === -1) missingPhases.push('APPROVAL');

if (missingPhases.length > 0 && parsed.implementationDetected) {
    console.error(`? GATE FAIL: Missing mandatory phases: ${missingPhases.join(', ')}`);
    process.exit(1);
}

// Validation 4: Enforce sequence order (BOOTSTRAP -> SKELETON -> APPROVAL)
if (parsed.bootstrapHeadingLine === -1 || parsed.skeletonHeadingLine === -1) {
    console.error("? GATE FAIL: Missing BOOTSTRAP or SKELETON markers.");
    process.exit(1);
}
if (parsed.bootstrapHeadingLine > parsed.skeletonHeadingLine) {
    console.error("? GATE FAIL: SKELETON appears before BOOTSTRAP.");
    process.exit(1);
}
if (parsed.approvalMarkerLine !== -1 && parsed.skeletonHeadingLine > parsed.approvalMarkerLine) {
    console.error("? GATE FAIL: SKELETON APPROVED marker must follow SKELETON.");
    process.exit(1);
}

console.log("? GATE PASS: Workflow sequence verified.");

````

###### Template File: `.GOV/scripts/validation/orchestrator_gates.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import {
    defaultRefinementPath,
    resolveSpecCurrent,
    validateRefinementFile,
} from './refinement-check.mjs';

const STATE_FILE = '.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json';

function loadState() {
    if (!fs.existsSync(STATE_FILE)) {
        return { gate_logs: [] };
    }
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
}

function saveState(state) {
    fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
}

const action = process.argv[2];
const wpId = process.argv[3];
const argvData = process.argv.slice(4);
const data = argvData[0];

const state = loadState();

// === V2: Protocol-locked refinement gate (unskippable) ===
// NOTE: We keep the legacy logic below for compatibility, but V2 exits before it can run.

const SIGNATURE_AUDIT_PATH = path.join('docs', 'SIGNATURE_AUDIT.md');

function v2Fail(msg, details = []) {
    console.error(`[GATE ERROR] ${msg}`);
    details.forEach((d) => console.error(`- ${d}`));
    process.exit(1);
}

function v2AssertWpId(id) {
    if (!id || !id.startsWith('WP-')) {
        v2Fail('Expected WP_ID like WP-1-Storage-Abstraction-Layer-v3');
    }
}

function v2GetSingleField(content, label) {
    const re = new RegExp(`^\\s*-\\s*${label}\\s*:\\s*(.+)\\s*$`, 'mi');
    const m = content.match(re);
    return m ? m[1].trim() : '';
}

function v2GitGrepOrEmpty(needle) {
    try {
        return execSync(`git grep -n \"${needle}\" -- .`, { encoding: 'utf8' }).trim();
    } catch {
        return '';
    }
}

function v2InsertSignatureAuditRow(auditContent, rowLine) {
    const lines = auditContent.split('\n');
    const headerIdx = lines.findIndex((l) => /^\|\s*Signature\s*\|\s*Used By\s*\|/i.test(l));
    if (headerIdx === -1) return null;

    const sepIdxRel = lines.slice(headerIdx + 1).findIndex((l) => /^\|\s*-{3,}\s*\|/.test(l));
    if (sepIdxRel === -1) return null;

    const insertAt = headerIdx + 2; // after separator line
    lines.splice(insertAt, 0, rowLine.trimEnd());
    return lines.join('\n');
}

function v2ResolveLastRefinement() {
    const logs = state.gate_logs.filter((l) => l.wpId === wpId);
    return [...logs].reverse().find((l) => l.type === 'REFINEMENT') || null;
}

function v2ResolveLastSignature() {
    const logs = state.gate_logs.filter((l) => l.wpId === wpId);
    return [...logs].reverse().find((l) => l.type === 'SIGNATURE') || null;
}

function v2ResolveLastPrepare() {
    const logs = state.gate_logs.filter((l) => l.wpId === wpId);
    return [...logs].reverse().find((l) => l.type === 'PREPARE') || null;
}

function v2NormalizeBranch(branch) {
    if (!branch) return '';
    return branch.replace(/^refs\/heads\//, '').trim();
}

function v2WorktreeListPorcelain() {
    try {
        return execSync('git worktree list --porcelain', { encoding: 'utf8' });
    } catch (e) {
        v2Fail('Failed to read git worktree list (is this a git repo?)', [String(e?.message || e)]);
    }
}

function v2WorktreeHasBranch(branch) {
    const needle = `branch refs/heads/${branch}`;
    const out = v2WorktreeListPorcelain();
    return out.split(/\r?\n/).some((line) => line.trim() === needle);
}

function v2AssertBranchExists(branch) {
    const normalized = v2NormalizeBranch(branch);
    if (!normalized) v2Fail('Branch is required for prepare step');
    try {
        execSync(`git show-ref --verify --quiet "refs/heads/${normalized}"`);
    } catch {
        v2Fail('Branch does not exist locally; create it first.', [
            `branch=${normalized}`,
            `Suggested: just worktree-add ${wpId} main ${normalized}`,
        ]);
    }
}

if (action === 'refine') {
    v2AssertWpId(wpId);

    const refinementPath = (data && fs.existsSync(data)) ? data : defaultRefinementPath(wpId);
    const validation = validateRefinementFile(refinementPath, { expectedWpId: wpId, requireSignature: false });
    if (!validation.ok) {
        v2Fail(`Refinement is not ready for review: ${refinementPath}`, validation.errors);
    }

    let resolved = null;
    try {
        resolved = resolveSpecCurrent();
    } catch {
        // validateRefinementFile already reports this.
    }

    state.gate_logs.push({
        wpId,
        type: 'REFINEMENT',
        refinement_path: refinementPath.replace(/\\/g, '/'),
        spec_target_resolved: resolved ? `.GOV/spec/SPEC_CURRENT.md -> ${resolved.specFileName}` : '.GOV/spec/SPEC_CURRENT.md -> <unresolved>',
        spec_target_sha1: resolved ? resolved.sha1 : '<unresolved>',
        timestamp: new Date().toISOString(),
        turn_token: String(Date.now()),
    });
    saveState(state);

    console.log(`[ORCHESTRATOR GATE] Technical Refinement recorded for ${wpId}.`);
    console.log('[GATE LOCKED] This is the refinement phase; do not request/record USER_SIGNATURE in this turn.');
    console.log('[NEXT] Wait for explicit user review, then run: just record-signature ' + wpId + ' {usernameDDMMYYYYHHMM}');
    process.exit(0);
}

if (action === 'sign') {
    v2AssertWpId(wpId);
    const signature = data;
    if (!signature || !/^[a-z]+[0-9]{12}$/.test(signature)) {
        v2Fail('Invalid signature format. Expected {username}{DDMMYYYYHHMM}');
    }

    const lastRefinement = v2ResolveLastRefinement();
    if (!lastRefinement) {
        v2Fail(`No technical refinement recorded for ${wpId}. Run: just record-refinement ${wpId}`);
    }

    const lastSignature = v2ResolveLastSignature();
    if (lastSignature) {
        v2Fail(`A signature is already recorded for ${wpId} (${lastSignature.signature}). Create a new WP variant instead of re-signing.`);
    }

    const refDate = new Date(lastRefinement.timestamp);
    const now = new Date();
    const diffSeconds = (now.getTime() - refDate.getTime()) / 1000;
    if (diffSeconds < 10) {
        v2Fail('Automation momentum detected: refinement and signature recorded too close together.', [
            `diff_seconds=${diffSeconds}`,
            'Protocol requires a standalone user review turn between refinement and signature.',
        ]);
    }

    const refinementPath = lastRefinement.refinement_path || defaultRefinementPath(wpId);
    const refinementValidation = validateRefinementFile(refinementPath, { expectedWpId: wpId, requireSignature: false });
    if (!refinementValidation.ok) {
        v2Fail(`Refinement is not complete; cannot sign: ${refinementPath}`, refinementValidation.errors);
    }

    // HARD GATE: Do not consume a one-time signature for WP packet approval if refinement requires enrichment.
    try {
        const refinementContent = fs.readFileSync(refinementPath, 'utf8');
        const m = refinementContent.match(/^\s*-\s*ENRICHMENT_NEEDED\s*:\s*(YES|NO)\s*$/mi);
        const enrichmentNeeded = (m?.[1] || '').toUpperCase();
        if (enrichmentNeeded === 'YES') {
            v2Fail('Refinement declares ENRICHMENT_NEEDED=YES; packet signature is forbidden.', [
                'Run the spec enrichment workflow first (new spec version + update .GOV/spec/SPEC_CURRENT.md).',
                'Then create a NEW WP variant anchored to the updated spec (new WP_ID; new one-time signature).',
            ]);
        }
    } catch (e) {
        v2Fail(`Failed to read refinement file: ${refinementPath}`, [String(e?.message || e)]);
    }

    // HARD GATE: signature requires explicit user approval evidence in the refinement file.
    // This is intentionally deterministic (not time-based) to prevent "sleep" bypass.
    try {
        const refinementContent = fs.readFileSync(refinementPath, "utf8");
        const approvalEvidence = v2GetSingleField(refinementContent, "USER_APPROVAL_EVIDENCE");
        const expected = `APPROVE REFINEMENT ${wpId}`;
        if (!approvalEvidence || approvalEvidence === "<pending>") {
            v2Fail("Missing USER_APPROVAL_EVIDENCE in refinement; cannot consume one-time signature.", [
                `Add a line to ${refinementPath.replace(/\\/g, "/")} under METADATA:`,
                `- USER_APPROVAL_EVIDENCE: ${expected}`,
            ]);
        }
        if (approvalEvidence !== expected) {
            v2Fail("Invalid USER_APPROVAL_EVIDENCE in refinement; cannot consume one-time signature.", [
                `Expected: ${expected}`,
                `Got: ${approvalEvidence}`,
            ]);
        }
    } catch (e) {
        v2Fail(`Failed to verify USER_APPROVAL_EVIDENCE in refinement: ${refinementPath}`, [String(e?.message || e)]);
    }

    // Refinement must not already be signed.
    try {
        const existing = fs.readFileSync(refinementPath, 'utf8');
        const existingSig = v2GetSingleField(existing, 'USER_SIGNATURE');
        if (existingSig && existingSig !== '<pending>') {
            v2Fail(`Refinement already has a USER_SIGNATURE (${existingSig}); signatures are one-time use.`);
        }
    } catch (e) {
        v2Fail(`Failed to read refinement file: ${refinementPath}`, [String(e?.message || e)]);
    }

    // One-time signature guard: refuse if it appears anywhere in tracked repo files.
    const grepHit = v2GitGrepOrEmpty(signature);
    if (grepHit) {
        v2Fail('Signature already appears in repo (one-time use). Provide a NEW signature.', [grepHit]);
    }

    // Update refinement file to reflect approval.
    try {
        const refinementContent = fs.readFileSync(refinementPath, 'utf8');
        const updated = refinementContent
            .replace(/^\s*-\s*USER_REVIEW_STATUS\s*:\s*.*$/mi, '- USER_REVIEW_STATUS: APPROVED')
            .replace(/^\s*-\s*USER_SIGNATURE\s*:\s*.*$/mi, `- USER_SIGNATURE: ${signature}`);
        fs.writeFileSync(refinementPath, updated, 'utf8');
    } catch (e) {
        v2Fail(`Failed to update refinement file with signature: ${refinementPath}`, [String(e?.message || e)]);
    }

    // Append to SIGNATURE_AUDIT (protocol requirement).
    if (!fs.existsSync(SIGNATURE_AUDIT_PATH)) {
        v2Fail(`Missing signature audit file: ${SIGNATURE_AUDIT_PATH}`);
    }

    try {
        const resolved = resolveSpecCurrent();
        const audit = fs.readFileSync(SIGNATURE_AUDIT_PATH, 'utf8');
        if (audit.includes(`| ${signature} |`)) {
            v2Fail('Signature already present in SIGNATURE_AUDIT (one-time use). Provide a NEW signature.');
        }

        const ts = signature.slice(-12);
        const dd = ts.slice(0, 2);
        const mm = ts.slice(2, 4);
        const yyyy = ts.slice(4, 8);
        const hh = ts.slice(8, 10);
        const min = ts.slice(10, 12);
        const dateTime = `${yyyy}-${mm}-${dd} ${hh}:${min}`;
        const verMatch = resolved.specFileName.match(/v([0-9.]+)\.md/);
        const specVer = verMatch ? `v${verMatch[1]}` : resolved.specFileName;

        const row = `| ${signature} | Orchestrator | ${dateTime} | Task packet creation: ${wpId} | ${specVer} | Approved after Technical Refinement (see ${refinementPath.replace(/\\\\/g, '/')} ). |`;
        const updatedAudit = v2InsertSignatureAuditRow(audit, row);
        if (!updatedAudit) {
            v2Fail('SIGNATURE_AUDIT format changed; cannot append deterministically.');
        }
        fs.writeFileSync(SIGNATURE_AUDIT_PATH, updatedAudit, 'utf8');
    } catch (e) {
        v2Fail('Failed to append to .GOV/roles_shared/records/SIGNATURE_AUDIT.md', [String(e?.message || e)]);
    }

    state.gate_logs.push({
        wpId,
        type: 'SIGNATURE',
        signature,
        timestamp: now.toISOString(),
        refinement_path: refinementPath.replace(/\\/g, '/'),
    });
    saveState(state);

    console.log(`[ORCHESTRATOR GATE] Signature recorded for ${wpId}.`);
    console.log('[GATE PARTIAL] Signature recorded. Next, you MUST create a WP branch/worktree and record assignment before creating the Task Packet.');
    console.log(`[NEXT] 1) Create WP worktree: just worktree-add ${wpId}`);
    console.log(`[NEXT] 2) Record assignment: just record-prepare ${wpId} {Coder-A|Coder-B} (optional: {branch} {worktree_dir})`);
    console.log(`[NEXT] 3) Then create packet: just create-task-packet ${wpId}`);
    process.exit(0);
}

if (action === 'prepare') {
    v2AssertWpId(wpId);

    const coderId = (argvData[0] || '').trim();
    const branch = v2NormalizeBranch((argvData[1] || `feat/${wpId}`).trim());
    const worktreeDir = (argvData[2] || `../wt-${wpId}`).trim();

    if (!coderId) {
        v2Fail('Missing coder assignment. Usage: just record-prepare WP-... Coder-A [branch] [worktree_dir]');
    }

    const lastSignature = v2ResolveLastSignature();
    if (!lastSignature) {
        v2Fail(`No signature recorded for ${wpId}. Run: just record-signature ${wpId} {usernameDDMMYYYYHHMM}`);
    }

    const lastPrepare = v2ResolveLastPrepare();
    if (lastPrepare) {
        console.warn(`[GATE WARNING] A prepare record already exists for ${wpId}; appending a new prepare entry.`);
    }

    v2AssertBranchExists(branch);
    if (!v2WorktreeHasBranch(branch)) {
        v2Fail('WP worktree not found for branch (required before task packet creation).', [
            `branch=${branch}`,
            'Create it first with: just worktree-add ' + wpId,
        ]);
    }

    state.gate_logs.push({
        wpId,
        type: 'PREPARE',
        coder_id: coderId,
        branch,
        worktree_dir: worktreeDir.replace(/\\/g, '/'),
        timestamp: new Date().toISOString(),
    });
    saveState(state);

    console.log(`[ORCHESTRATOR GATE] Prepared ${wpId} for development.`);
    console.log(`- coder_id: ${coderId}`);
    console.log(`- branch: ${branch}`);
    console.log(`- worktree_dir: ${worktreeDir}`);
    console.log('[NEXT] Create packet: just create-task-packet ' + wpId);
    process.exit(0);
}

if (action !== 'refine' && action !== 'sign') {
    v2Fail('Unknown action. Expected: refine|sign|prepare');
}

if (action === 'refine') {
    // data is an optional hash or description of the refinement
    const refinementEntry = {
        wpId,
        type: 'REFINEMENT',
        data: data || 'No detail provided',
        timestamp: new Date().toISOString(),
        // We use a simple counter to track "Protocol Turns"
        turn_token: Math.random().toString(36).substring(7)
    };
    
    state.gate_logs.push(refinementEntry);
    saveState(state);
    console.log(`
âœ… [ORCHESTRATOR GATE] Technical Refinement recorded for ${wpId}.`);
    console.log(`ðŸ”’ [GATE LOCKED] You must wait for a new turn to provide a signature.
`);
}

if (action === 'sign') {
    // data is the signature: usernameDDMMYYYYHHMM
    if (!data || !/^[a-z]+[0-9]{12}$/.test(data)) {
        console.error(`
âŒ [GATE ERROR] Invalid signature format. Expected {username}{DDMMYYYYHHMM}
`);
        process.exit(1);
    }

    const logs = state.gate_logs.filter(l => l.wpId === wpId);
    const lastRefinement = [...logs].reverse().find(l => l.type === 'REFINEMENT');
    
    if (!lastRefinement) {
        console.error(`
âŒ [GATE ERROR] No technical refinement found for ${wpId}. You cannot sign what hasn't been refined.
`);
        process.exit(1);
    }

    // BLOCK: Automation Momentum Detection
    // If the signature's HHMM matches the refinement's HHMM, it's likely a merged turn.
    const refDate = new Date(lastRefinement.timestamp);
    const refHHMM = `${String(refDate.getDate()).padStart(2, '0')}${String(refDate.getMonth() + 1).padStart(2, '0')}${refDate.getFullYear()}${String(refDate.getHours()).padStart(2, '0')}${String(refDate.getMinutes()).padStart(2, '0')}`;
    const sigHHMM = data.slice(-12);

    // If the refinement was recorded less than 10 seconds ago, it's definitely the same turn.
    const now = new Date();
    const diffSeconds = (now.getTime() - refDate.getTime()) / 1000;

    if (diffSeconds < 10) {
        console.error(`
ðŸš¨ [GATE ERROR: AUTOMATION MOMENTUM]`);
        console.error(`Refinement and Signature detected in the same turn (diff: ${diffSeconds}s).`);
        console.error(`The protocol mandates a standalone turn for refinement inspection.`);
        console.error(`STOP and wait for the user to review the refinement in a NEW turn.
`);
        process.exit(1);
    }

    state.gate_logs.push({
        wpId,
        type: 'SIGNATURE',
        signature: data,
        timestamp: now.toISOString()
    });
    
    saveState(state);
    console.log(`
âœ… [ORCHESTRATOR GATE] Signature validated for ${wpId}.`);
    console.log(`ðŸ”“ [GATE UNLOCKED] You may now create the Task Packet.
`);
}
````

###### Template File: `.GOV/scripts/validation/post-work-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Post-work validation (deterministic manifest + gates)
 * - Enforces manifest schema and gate coverage inherited from COR-701 (anchors/rails/window/sha1/line_delta/concurrency)
 * - Keeps existing surface: `node post-work-check.mjs WP-{ID}` (also used by `just post-work {wp}`)
 */

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { execSync } from 'child_process';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('Usage: node post-work-check.mjs WP-{ID}');
  process.exit(1);
}

const SPEC_PATH = path.join('scripts', 'validation', 'cor701-spec.json');
const spec = JSON.parse(fs.readFileSync(SPEC_PATH, 'utf8'));

console.log(`\nPost-work validation for ${WP_ID} (deterministic manifest + gates)...\n`);

const errors = [];
const warnings = [];

const gitTrim = (command) => execSync(command, { encoding: 'utf8' }).trim();
const gitBuffer = (command) => execSync(command);

const resolveMergeBase = () => {
  try {
    const base = gitTrim('git merge-base main HEAD');
    return base || null;
  } catch {
    return null;
  }
};

const readFileIfExists = (p) => {
  try {
    return fs.readFileSync(p, 'utf8');
  } catch {
    return '';
  }
};

const sha1Hex = (bufOrString) => crypto.createHash('sha1').update(bufOrString).digest('hex');

const normalizeLf = (text) => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
const normalizeCrlf = (text) => normalizeLf(text).replace(/\n/g, '\r\n');

const isLikelyText = (buf) => !buf.includes(0);

const sha1VariantsForText = (text) => {
  const lf = normalizeLf(text);
  return {
    lf: sha1Hex(lf),
    crlf: sha1Hex(normalizeCrlf(lf)),
  };
};

const sha1VariantsForGitBlob = (buf) => {
  const lf = sha1Hex(buf);
  if (!isLikelyText(buf)) {
    return { lf, crlf: lf };
  }

  const txt = buf.toString('utf8');
  const { crlf } = sha1VariantsForText(txt);
  return { lf, crlf };
};

const sha1VariantsForWorktreeFile = (p) => {
  const buf = fs.readFileSync(p);
  const raw = sha1Hex(buf);
  if (!isLikelyText(buf)) {
    return { raw, lf: raw, crlf: raw };
  }

  const txt = buf.toString('utf8');
  const { lf, crlf } = sha1VariantsForText(txt);
  return { raw, lf, crlf };
};

// Use LF-normalized hash for worktree reads to avoid CRLF-based false negatives on Windows.
const computeSha1 = (p) => sha1VariantsForWorktreeFile(p).lf;

const MERGE_BASE = resolveMergeBase();

const loadGitVersion = (rev, targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    return gitBuffer(`git show ${rev}:${gitPath}`);
  } catch {
    return null;
  }
};

const loadHeadVersion = (targetPath) => {
  return loadGitVersion('HEAD', targetPath);
};

const loadIndexVersion = (targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    return gitBuffer(`git show :${gitPath}`);
  } catch {
    return null;
  }
};

const getNumstatDelta = (targetPath, { staged }) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const diffArgs = staged ? '--cached' : '';
    const out = gitTrim(`git diff ${diffArgs} --numstat HEAD -- "${gitPath}"`);
    if (!out) return null;
    const [addsStr, delsStr] = out.split('\t');
    const adds = parseInt(addsStr, 10);
    const dels = parseInt(delsStr, 10);
    if (Number.isNaN(adds) || Number.isNaN(dels)) return null;
    return adds - dels;
  } catch {
    return null;
  }
};

const parseDiffHunks = (targetPath, { staged }) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const diffArgs = staged ? '--cached' : '';
    const diff = gitTrim(`git diff ${diffArgs} --unified=0 HEAD -- "${gitPath}"`);
    const hunks = [];
    const hunkHeader = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/;
    diff.split('\n').forEach((line) => {
      const m = line.match(hunkHeader);
      if (m) {
        const [_, oStart, oLen, nStart, nLen] = m;
        hunks.push({
          oldStart: parseInt(oStart, 10),
          oldLen: oLen ? parseInt(oLen, 10) : 1,
          newStart: parseInt(nStart, 10),
          newLen: nLen ? parseInt(nLen, 10) : 1,
        });
      }
    });
    return hunks;
  } catch {
    return [];
  }
};

const taskPacketDir = '.GOV/task_packets';
let packetContent = '';
let packetPath = '';
if (fs.existsSync(taskPacketDir)) {
  const taskPacketFiles = fs.readdirSync(taskPacketDir)
    .filter((f) => f.includes(WP_ID));
  if (taskPacketFiles.length > 0) {
    packetPath = `${taskPacketDir}/${taskPacketFiles[0]}`;
    packetContent = readFileIfExists(packetPath);
  }
}

const parseInScopePaths = (content) => {
  if (!content) return [];
  const lines = content.split('\n');
  const idx = lines.findIndex((l) => /^\s*-\s*IN_SCOPE_PATHS\s*:\s*$/i.test(l));
  if (idx === -1) return [];
  const results = [];
  for (let i = idx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^\s*-\s*[A-Z0-9_]+\s*:/.test(line)) break; // next top-level metadata-ish bullet
    if (/^\s*##\s+/.test(line)) break;
    const m = line.match(/^\s*-\s+(.+)\s*$/) || line.match(/^\s{2,}-\s+(.+)\s*$/);
    if (!m) continue;
    const value = m[1].trim().replace(/^`|`$/g, '');
    if (!value || value.toLowerCase() === 'path/to/file') continue;
    results.push(path.normalize(value).replace(/\\/g, '/'));
  }
  return Array.from(new Set(results));
};

const requiresManifest = (filePath) => {
  const p = filePath.replace(/\\/g, '/');
  if (p.startsWith('.GOV/')) return false;
  return true;
};

const getStagedFiles = () => {
  try {
    // --diff-filter=d excludes deleted files (they cannot have manifest entries since
    // the file doesn't exist on disk for SHA1 verification and End>=Start>=1 fails)
    const out = gitTrim('git diff --name-only --cached --diff-filter=d');
    return out ? out.split('\n').filter(Boolean) : [];
  } catch {
    return [];
  }
};

const getWorkingFiles = () => {
  try {
    // --diff-filter=d excludes deleted files (same rationale as above)
    const out = gitTrim('git diff --name-only HEAD --diff-filter=d');
    return out ? out.split('\n').filter(Boolean) : [];
  } catch {
    return [];
  }
};

const parseValidationManifests = (content) => {
  if (!content) return null;
  const lines = content.split('\n');
  const startIdx = lines.findIndex((line) => /^##\s*validation/i.test(line));
  if (startIdx === -1) return null;
  const manifestLines = [];
  for (let i = startIdx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^##\s+/.test(line)) break;
    manifestLines.push(line);
  }

  const manifests = [];
  let current = {
    target_file: '',
    start: '',
    end: '',
    pre_sha1: '',
    post_sha1: '',
    line_delta: '',
    gates_passed: new Set(),
    rawLines: '',
  };
  let inGates = false;
  const flush = () => {
    if (
      current.target_file
      || current.start
      || current.end
      || current.pre_sha1
      || current.post_sha1
      || current.line_delta
      || current.gates_passed.size > 0
    ) {
      current.rawLines = current.rawLines.trimEnd();
      manifests.push(current);
    }
    current = {
      target_file: '',
      start: '',
      end: '',
      pre_sha1: '',
      post_sha1: '',
      line_delta: '',
      gates_passed: new Set(),
      rawLines: '',
    };
    inGates = false;
  };

  const assignField = (label, value) => {
    const v = value.trim().replace(/^`|`$/g, '');
    if (label === 'Target File') current.target_file = v;
    if (label === 'Start') current.start = v;
    if (label === 'End') current.end = v;
    if (label === 'Pre-SHA1') current.pre_sha1 = v;
    if (label === 'Post-SHA1') current.post_sha1 = v;
    if (label === 'Line Delta') current.line_delta = v;
  };

  const fieldRe = /^\s*-\s*\*\*(Target File|Start|End|Pre-SHA1|Post-SHA1|Line Delta)\*\*\s*:\s*(.*)\s*$/i;
  const gatesStartRe = /^\s*-\s*\*\*Gates Passed\*\*\s*:\s*$/i;
  const gateLineRe = /^\s*-\s*\[(x|X)\]\s*([a-z0-9_]+)\s*$/i;

  manifestLines.forEach((line) => {
    current.rawLines += `${line}\n`;
    const mField = line.match(fieldRe);
    if (mField) {
      const label = mField[1];
      const value = mField[2] ?? '';
      if (label.toLowerCase() === 'target file' && current.target_file) flush();
      assignField(label, value);
      return;
    }
    if (gatesStartRe.test(line)) {
      inGates = true;
      return;
    }
    if (inGates) {
      const mGate = line.trim().match(gateLineRe);
      if (mGate) {
        current.gates_passed.add(mGate[2].toLowerCase());
        return;
      }
      if (!line.trim().startsWith('-')) {
        inGates = false;
      }
    }
  });

  flush();
  return manifests.length > 0 ? manifests : null;
};

const parseWaivers = (content) => {
  if (!content) return false;
  // Look for WAIVERS GRANTED section and keywords like "dirty tree", "git hygiene", or CX-573F
  const waiverBlock = content.match(/##\s*WAIVERS\s*GRANTED([\s\S]*?)##/i);
  if (!waiverBlock) return false;
  const waivers = waiverBlock[1];
  return /CX-573F|dirty\s*tree|git\s*hygiene/i.test(waivers) && !/NONE/i.test(waivers);
};

// Check 1: manifest present and ASCII only
console.log('Check 1: Validation manifest present');
if (!packetContent) {
  errors.push('No task packet found for this WP_ID');
} else if (!/VALIDATION/i.test(packetContent)) {
  errors.push('Task packet missing VALIDATION section');
} else if (/[^\x00-\x7F]/.test(packetContent)) {
  errors.push('Task packet contains non-ASCII characters (manifest must be ASCII)');
}

const hasGitWaiver = parseWaivers(packetContent);
if (hasGitWaiver) {
  console.log('NOTE: Git hygiene waiver detected [CX-573F]. Strict git checks relaxed.');
}

const manifests = parseValidationManifests(packetContent);
if (!manifests) {
  errors.push('VALIDATION section found but manifest fields not parsed');
}

const inScopePaths = parseInScopePaths(packetContent);
const stagedFiles = getStagedFiles();
const workingFiles = getWorkingFiles();
const useStaged = stagedFiles.length > 0;
const changedFiles = useStaged ? stagedFiles : workingFiles;
if (useStaged && workingFiles.length > stagedFiles.length) {
  // Avoid warning noise for validator-only governance state.
  const stagedSet = new Set(stagedFiles.map((p) => p.replace(/\\/g, '/')));
  const allowlistedUnstaged = new Set([
    '.GOV/roles_shared/records/TASK_BOARD.md',
    '.GOV/roles_shared/records/SIGNATURE_AUDIT.md',
    '.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json',
    '.GOV/roles/validator/VALIDATOR_GATES.json',
    packetPath.replace(/\\/g, '/'),
    `.GOV/refinements/${WP_ID}.md`,
  ].filter(Boolean));

  const isAllowlistedUnstaged = (p) =>
    allowlistedUnstaged.has(p) || p.startsWith('.GOV/validator_gates/');

  const hasRelevantUnstaged = workingFiles
    .map((p) => p.replace(/\\/g, '/'))
    .filter((p) => !stagedSet.has(p))
    .some((p) => !isAllowlistedUnstaged(p));

  if (hasRelevantUnstaged) {
    warnings.push('Working tree has unstaged changes; post-work validation uses STAGED changes only.');
  }
}

// Check 2: manifest schema (per target file)
if (manifests) {
  console.log('\nCheck 2: Manifest fields');
  const shaRegex = /^[a-f0-9]{40}$/i;
  // Validate scope (best-effort): changed files must be subset of IN_SCOPE_PATHS (plus allowed governance files),
  // unless a waiver is present. This only applies to the evaluated diff set (staged preferred).
  const allowlisted = new Set([
    '.GOV/roles_shared/records/TASK_BOARD.md',
    '.GOV/roles_shared/records/SIGNATURE_AUDIT.md',
    '.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json',
    '.GOV/roles/validator/VALIDATOR_GATES.json',
    packetPath.replace(/\\/g, '/'),
    `.GOV/refinements/${WP_ID}.md`,
  ].filter(Boolean));

  const outOfScope = changedFiles
    .map((p) => p.replace(/\\/g, '/'))
    .filter((p) => !allowlisted.has(p))
    .filter((p) => (inScopePaths.length > 0 ? !inScopePaths.includes(p) : false));

  if (outOfScope.length > 0 && !hasGitWaiver) {
    errors.push(`Out-of-scope files changed (stage only WP files or record waiver [CX-573F]): ${outOfScope.join(', ')}`);
  } else if (outOfScope.length > 0 && hasGitWaiver) {
    warnings.push(`Out-of-scope files changed but waiver present [CX-573F]: ${outOfScope.join(', ')}`);
  }

  // Require manifest coverage for all non-docs changed files.
  const manifestTargets = new Set(manifests.map((m) => path.normalize(m.target_file).replace(/\\/g, '/')).filter(Boolean));
  const missingCoverage = changedFiles
    .map((p) => p.replace(/\\/g, '/'))
    .filter((p) => requiresManifest(p))
    .filter((p) => !manifestTargets.has(p));
  if (missingCoverage.length > 0) {
    errors.push(`Missing VALIDATION manifest coverage for changed files: ${missingCoverage.join(', ')}`);
  }

  // Now validate each manifest entry.
  console.log('\nCheck 3: File integrity (per manifest entry)');
  manifests.forEach((manifest, idx) => {
    const label = `Manifest[${idx + 1}]`;

    spec.requiredFields.forEach((field) => {
      const value = manifest[field];
      if (!value || (typeof value === 'string' && value.trim() === '')) {
        errors.push(`${label}: missing required field: ${field}`);
      }
    });

    if (manifest.pre_sha1 && !shaRegex.test(manifest.pre_sha1)) {
      errors.push(`${label}: pre_sha1 must be a 40-char hex SHA1`);
    }
    if (manifest.post_sha1 && !shaRegex.test(manifest.post_sha1)) {
      errors.push(`${label}: post_sha1 must be a 40-char hex SHA1`);
    }

    const startNum = parseInt(manifest.start, 10);
    const endNum = parseInt(manifest.end, 10);
    if (Number.isNaN(startNum) || Number.isNaN(endNum) || startNum < 1 || endNum < startNum) {
      errors.push(`${label}: Start/End must be integers with start >=1 and end >= start`);
    }

    const deltaNum = parseInt(manifest.line_delta, 10);
    if (manifest.line_delta === '' || Number.isNaN(deltaNum)) {
      errors.push(`${label}: line_delta must be an integer (adds - dels)`);
    }

    const targetPath = path.normalize(manifest.target_file.replace(/^`|`$/g, ''));
    if (!fs.existsSync(targetPath)) {
      errors.push(`${label}: Target file does not exist: ${targetPath} (${spec.gateErrorCodes.filename_canonical_and_openable})`);
      return;
    }

    // pre/post SHA checks (staged-aware)
    const headContent = loadHeadVersion(targetPath);
    if (headContent !== null) {
      const head = sha1VariantsForGitBlob(headContent);
      if (manifest.pre_sha1 && manifest.pre_sha1 !== head.lf) {
        if (manifest.pre_sha1 === head.crlf) {
          warnings.push(`${label}: pre_sha1 matches CRLF-normalized HEAD for ${targetPath}; prefer LF blob SHA1=${head.lf}`);
        } else if (MERGE_BASE) {
          const baseContent = loadGitVersion(MERGE_BASE, targetPath);
          const base = baseContent ? sha1VariantsForGitBlob(baseContent) : null;
          const matchesBase = base && (manifest.pre_sha1 === base.lf || manifest.pre_sha1 === base.crlf);
          if (matchesBase) {
            warnings.push(`${label}: pre_sha1 matches merge-base(${MERGE_BASE}) for ${targetPath} (common after WP commits); prefer LF blob SHA1=${base.lf}`);
          } else if (hasGitWaiver) {
            warnings.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage}) - WAIVER APPLIED`);
            warnings.push(`${label}: expected pre_sha1 (HEAD LF blob) = ${head.lf}`);
          } else {
            errors.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage})`);
            errors.push(`${label}: expected pre_sha1 (HEAD LF blob) = ${head.lf}`);
            if (base) errors.push(`${label}: expected pre_sha1 (merge-base LF blob) = ${base.lf}`);
          }
        } else if (hasGitWaiver) {
          warnings.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage}) - WAIVER APPLIED`);
          warnings.push(`${label}: expected pre_sha1 (LF blob) = ${head.lf}`);
        } else {
          errors.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage})`);
          errors.push(`${label}: expected pre_sha1 (LF blob) = ${head.lf}`);
        }
      }
    } else {
      warnings.push(`${label}: Could not load HEAD version (new file or not tracked): ${targetPath}`);
    }

    const postContent = useStaged ? loadIndexVersion(targetPath) : null;
    const post = postContent === null
      ? sha1VariantsForWorktreeFile(targetPath)
      : sha1VariantsForGitBlob(postContent);
    const expectedPost = postContent === null ? post.lf : post.lf;
    if (manifest.post_sha1 && manifest.post_sha1 !== expectedPost) {
      const acceptable = new Set([post.crlf, post.raw].filter(Boolean));
      if (acceptable.has(manifest.post_sha1)) {
        warnings.push(`${label}: post_sha1 matches non-canonical EOL variant for ${targetPath}; prefer LF blob SHA1=${expectedPost}`);
      } else {
        errors.push(`${label}: post_sha1 mismatch for ${targetPath} (${spec.gateErrorCodes.post_sha1_captured})`);
        errors.push(`${label}: expected post_sha1 (LF) = ${expectedPost}`);
      }
    }

    const hunks = parseDiffHunks(targetPath, { staged: useStaged });
    const windowStart = parseInt(manifest.start, 10);
    const windowEnd = parseInt(manifest.end, 10);
    hunks.forEach((h) => {
      const oldEnd = h.oldStart + Math.max(h.oldLen - 1, 0);
      const newEnd = h.newStart + Math.max(h.newLen - 1, 0);
      const oldOutside = h.oldLen > 0 && (h.oldStart < windowStart || oldEnd > windowEnd);
      const newOutside = h.newLen > 0 && (h.newStart < windowStart || newEnd > windowEnd);
      if (oldOutside || newOutside) {
        errors.push(`${label}: Diff touches lines outside declared window [${windowStart}, ${windowEnd}] (${spec.gateErrorCodes.rails_untouched_outside_window})`);
      }
    });

    const numstatDelta = getNumstatDelta(targetPath, { staged: useStaged });
    if (numstatDelta !== null && !Number.isNaN(deltaNum) && numstatDelta !== deltaNum) {
      errors.push(`${label}: line_delta (${deltaNum}) does not match git diff delta (${numstatDelta}) (${spec.gateErrorCodes.line_delta_equals_expected})`);
    }

    // Gate checkboxes: allow either explicit checkmarks OR automatic inference (warn if inferred).
    spec.requiredGates.forEach((gate) => {
      if (manifest.gates_passed.has(gate)) return;
      // Infer gates we can verify mechanically.
      const inferable = new Set([
        'anchors_present',
        'window_matches_plan',
        'rails_untouched_outside_window',
        'filename_canonical_and_openable',
        'pre_sha1_captured',
        'post_sha1_captured',
        'line_delta_equals_expected',
        'manifest_written_and_path_returned',
        'current_file_matches_preimage',
      ]);
      if (inferable.has(gate)) {
        warnings.push(`${label}: gate not checked but inferred as PASS: ${gate} (${spec.gateErrorCodes[gate]})`);
        return;
      }
      errors.push(`${label}: gate missing or unchecked: ${gate} (${spec.gateErrorCodes[gate]})`);
    });
  });
}

// Check 4: git status sanity
console.log('\nCheck 4: Git status');
try {
  const staged = getStagedFiles();
  const working = getWorkingFiles();
  if (staged.length === 0 && working.length === 0) errors.push('No files changed (git status clean)');
} catch {
  warnings.push('Could not read git status');
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  if (warnings.length > 0) {
    console.log('Post-work validation PASSED with warnings\n');
    console.log('Warnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  } else {
    console.log('Post-work validation PASSED');
  }
  console.log('\nYou may proceed with commit.');
  process.exit(0);
} else {
  console.log('Post-work validation FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues before committing (gates enforce determinism).');
  console.log('See: .GOV/roles/coder/CODER_PROTOCOL.md');
  process.exit(1);
}

````

###### Template File: `.GOV/scripts/validation/pre-work-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Pre-work validation [CX-580, CX-620]
 * - Verifies task packet exists before work starts
 * - Ensures deterministic manifest template (COR-701-style) is present so post-work can enforce gates
 */

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import {
  defaultRefinementPath,
  validateRefinementFile,
} from './refinement-check.mjs';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('Usage: node pre-work-check.mjs WP-{ID}');
  process.exit(1);
}

console.log(`\nPre-work validation for ${WP_ID}...\n`);

const errors = [];
const warnings = [];
const spec = JSON.parse(fs.readFileSync(path.join('scripts', 'validation', 'cor701-spec.json'), 'utf8'));

// Check 1: Task packet file exists
console.log('Check 1: Task packet file exists');
const taskPacketDir = '.GOV/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  fs.mkdirSync(taskPacketDir, { recursive: true });
}

const taskPacketFiles = fs.readdirSync(taskPacketDir)
  .filter((f) => f.includes(WP_ID) && f.endsWith('.md'));

let packetContent = '';
let packetPath = '';

if (taskPacketFiles.length === 0) {
  errors.push(`No task packet file found for ${WP_ID} in .GOV/task_packets/`);
  console.log('FAIL: No task packet file');
} else {
  packetPath = path.join(taskPacketDir, taskPacketFiles[0]);
  packetContent = fs.readFileSync(packetPath, 'utf8');
  console.log(`PASS: Found ${taskPacketFiles[0]}`);

  // Check 2: Packet has required fields
  console.log('\nCheck 2: Task packet structure');
  const requiredFields = [
    'TASK_ID',
    'RISK_TIER',
    'SCOPE',
    'TEST_PLAN',
    'DONE_MEANS',
    'BOOTSTRAP',
  ];

  const lowerContent = packetContent.toLowerCase();
  const missingFields = requiredFields.filter((field) => !lowerContent.includes(field.toLowerCase()));

  if (missingFields.length > 0) {
    errors.push(`Task packet missing fields: ${missingFields.join(', ')}`);
    console.log(`FAIL: Missing ${missingFields.join(', ')}`);
  } else {
    console.log('PASS: All required fields present');
  }

  // Check 2.5: Spec provenance/target fields (non-blocking; backward compatible)
  const hasLegacySpec = /SPEC_CURRENT/i.test(packetContent);
  const hasSpecBaseline = /SPEC_BASELINE/i.test(packetContent);
  const hasSpecTarget = /SPEC_TARGET/i.test(packetContent);
  if (!hasLegacySpec && !(hasSpecBaseline && hasSpecTarget)) {
    warnings.push('Spec reference missing: include SPEC_BASELINE (provenance) and SPEC_TARGET (closure target), or legacy SPEC_CURRENT.');
  }

  // Check 2.6: Canonical Status field (governance invariant)
  const statusLine = (packetContent.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (packetContent.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (packetContent.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1]
    || '';
  const statusNorm = statusLine.trim().toLowerCase();
  if (!statusLine) {
    errors.push('Missing canonical **Status:** field');
  }

  const isDoneLike = /\b(done|validated|complete)\b/i.test(statusLine);
  const requiresRefinementGate = !isDoneLike; // pre-work implies active work; enforce unless explicitly Done/Validated.

  // Check 2.7: Technical Refinement gate (unskippable for active packets)
  if (requiresRefinementGate) {
    console.log('\nCheck 2.7: Technical Refinement gate');

    const refinementFile = (packetContent.match(/^\s*-\s*REFINEMENT_FILE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || defaultRefinementPath(WP_ID);

    const refinementValidation = validateRefinementFile(refinementFile, { expectedWpId: WP_ID, requireSignature: true });
    if (!refinementValidation.ok) {
      errors.push(`Technical refinement gate failed (see ${refinementFile})`);
      refinementValidation.errors.forEach((e) => errors.push(`  - ${e}`));
    } else {
      console.log('PASS: Refinement file exists and is approved/signed');
    }

    const packetSig = (packetContent.match(/^\s*-\s*USER_SIGNATURE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || (packetContent.match(/^\s*\*\*User Signature Locked:\*\*\s*(.+)\s*$/mi) || [])[1]?.trim()
      || (packetContent.match(/^\s*User Signature Locked:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || '';

    if (!packetSig || /<pending>/i.test(packetSig)) {
      errors.push('USER_SIGNATURE missing or <pending> (active packets must be locked before work starts)');
    } else if (refinementValidation.ok && refinementValidation.parsed.signature && packetSig !== refinementValidation.parsed.signature) {
      errors.push(`USER_SIGNATURE mismatch: packet has ${packetSig}, refinement has ${refinementValidation.parsed.signature}`);
    }

    // Protocol requirement: signature must be present in SIGNATURE_AUDIT.md
    try {
      const auditPath = path.join('docs', 'SIGNATURE_AUDIT.md');
      const audit = fs.readFileSync(auditPath, 'utf8');
      if (packetSig && !audit.includes(`| ${packetSig} |`)) {
        errors.push(`USER_SIGNATURE not found in .GOV/roles_shared/records/SIGNATURE_AUDIT.md (${packetSig})`);
      }
    } catch {
      warnings.push('Could not verify signature against .GOV/roles_shared/records/SIGNATURE_AUDIT.md');
    }

    // Safety checkpoint gate: packet + refinement must be committed before development starts.
    // This prevents untracked/uncommitted WP artifacts from being lost during accidental clean/reset operations.
    console.log('\nCheck 2.8: WP checkpoint commit gate');
    try {
      execSync(`git cat-file -e HEAD:${packetPath.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Task packet is not committed yet (checkpoint required): ${packetPath.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} .GOV/roles_shared/records/SIGNATURE_AUDIT.md .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
    }

    try {
      execSync(`git cat-file -e HEAD:${refinementFile.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Refinement file is not committed yet (checkpoint required): ${refinementFile.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} .GOV/roles_shared/records/SIGNATURE_AUDIT.md .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
    }
  } else {
    console.log('\nCheck 2.7: Technical Refinement gate (skipped for Done/Validated packets)');
  }

  // Check 3: Deterministic manifest template present
  console.log('\nCheck 3: Deterministic manifest template');
  if (!/##\s*validation/i.test(packetContent)) {
    errors.push('VALIDATION section missing (required for deterministic manifest)');
    console.log('FAIL: Missing VALIDATION section');
  } else {
    const lower = packetContent.toLowerCase();
    const lowerNorm = lower.replace(/[-_]/g, ' ');
    const fieldMissing = spec.requiredFields.filter((f) => !lowerNorm.includes(f.replace(/_/g, ' ')));
    if (fieldMissing.length > 0) {
      errors.push(`Validation manifest missing fields: ${fieldMissing.join(', ')}`);
      console.log(`FAIL: Validation manifest missing fields: ${fieldMissing.join(', ')}`);
    } else {
      console.log('PASS: Manifest fields present');
    }

    if (!/gates passed/i.test(packetContent)) {
      errors.push('Validation manifest missing "Gates Passed" checklist');
      console.log('FAIL: Missing gates checklist');
    } else {
      const gateHits = spec.requiredGates.filter((g) => lower.includes(g));
      if (gateHits.length !== spec.requiredGates.length) {
        warnings.push('Validation manifest present but some gates are not listed (ensure template is fully copied)');
      } else {
        console.log('PASS: Gates checklist present');
      }
    }
  }
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  if (warnings.length > 0) {
    console.log('Pre-work validation PASSED with warnings\n');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  } else {
    console.log('Pre-work validation PASSED');
  }
  console.log('\nYou may proceed with implementation.');
  process.exit(0);
} else {
  console.log('Pre-work validation FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues before starting work.');
  console.log('See: .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md or .GOV/roles/coder/CODER_PROTOCOL.md');
  process.exit(1);
}

````

###### Template File: `.GOV/scripts/validation/refinement-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

const SPEC_CURRENT_PATH = path.join('docs', 'SPEC_CURRENT.md');

export function resolveSpecCurrent() {
  if (!fs.existsSync(SPEC_CURRENT_PATH)) {
    throw new Error(`Missing ${SPEC_CURRENT_PATH}`);
  }
  const specCurrent = fs.readFileSync(SPEC_CURRENT_PATH, 'utf8');
  const m = specCurrent.match(/{{PROJECT_PREFIX}}_Master_Spec_v[0-9.]+\.md/);
  if (!m) {
    throw new Error(`Could not resolve spec filename from ${SPEC_CURRENT_PATH}`);
  }
  const specFileName = m[0];
  const specFilePath = path.join(specFileName);
  if (!fs.existsSync(specFilePath)) {
    throw new Error(`Resolved spec file does not exist: ${specFilePath}`);
  }
  const sha1 = crypto.createHash('sha1').update(fs.readFileSync(specFilePath)).digest('hex');
  return { specFileName, specFilePath, sha1 };
}

export function defaultRefinementPath(wpId) {
  return path.join('docs', 'refinements', `${wpId}.md`);
}

export function isAsciiOnly(s) {
  return !/[^\x00-\x7F]/.test(s);
}

function getSingleField(content, label) {
  const re = new RegExp(`^\\s*-\\s*${label}\\s*:\\s*(.+)\\s*$`, 'mi');
  const m = content.match(re);
  return m ? m[1].trim() : '';
}

function hasHeading(content, heading) {
  const re = new RegExp(`^#{2,6}\\s+${heading}\\b`, 'mi');
  return re.test(content);
}

function extractFencedBlockAfterLabel(lines, label) {
  const labelIdx = lines.findIndex((l) => new RegExp(`^\\s*-\\s*${label}\\s*:\\s*$`, 'i').test(l));
  if (labelIdx === -1) return { found: false, body: '' };

  let i = labelIdx + 1;
  while (i < lines.length && lines[i].trim() === '') i += 1;
  if (i >= lines.length) return { found: true, body: '' };

  const fenceStart = lines[i].trim();
  const fenceRe = /^```([a-z0-9_-]+)?\s*$/i;
  const m = fenceStart.match(fenceRe);
  if (!m) return { found: true, body: '' };

  const bodyLines = [];
  i += 1;
  for (; i < lines.length; i += 1) {
    if (lines[i].trim() === '```') break;
    bodyLines.push(lines[i]);
  }
  return { found: true, body: bodyLines.join('\n').trim() };
}

function extractFencedBlockAfterHeading(lines, heading) {
  const headingIdx = lines.findIndex((l) => new RegExp(`^#{2,6}\\s+${heading}\\b`, 'i').test(l));
  if (headingIdx === -1) return { found: false, body: '' };

  // Limit scan to the heading's section (until the next Markdown heading).
  const sectionStart = headingIdx + 1;
  let sectionEnd = lines.length;
  for (let j = sectionStart; j < lines.length; j += 1) {
    if (/^#{1,6}\s+\S/.test(lines[j])) {
      sectionEnd = j;
      break;
    }
  }

  let i = sectionStart;
  while (i < sectionEnd && lines[i].trim() === '') i += 1;
  if (i >= sectionEnd) return { found: true, body: '' };

  // Find the first fence within the section.
  for (; i < sectionEnd; i += 1) {
    const fenceStart = lines[i].trim();
    const fenceRe = /^```([a-z0-9_-]+)?\s*$/i;
    const m = fenceStart.match(fenceRe);
    if (!m) continue;

    const bodyLines = [];
    i += 1;
    for (; i < sectionEnd; i += 1) {
      if (lines[i].trim() === '```') break;
      bodyLines.push(lines[i]);
    }
    return { found: true, body: bodyLines.join('\n').trim() };
  }

  return { found: true, body: '' };
}

function looksLikeNotApplicableBlock(s) {
  const v = (s || '').trim();
  if (!v) return true;
  return /^<not applicable(\s*;\s*ENRICHMENT_NEEDED\s*=\s*NO)?>\s*$/i.test(v);
}

function looksLikePlaceholderEnrichment(s) {
  const v = (s || '').trim();
  if (!v) return true;
  if (/^<paste/i.test(v)) return true;
  if (v.includes('<paste')) return true;
  return false;
}

function parseAnchors(content) {
  const lines = content.split('\n');
  const anchors = [];

  for (let i = 0; i < lines.length; i += 1) {
    if (!/^####\s+ANCHOR\b/i.test(lines[i])) continue;

    const sectionLines = [];
    for (let j = i + 1; j < lines.length; j += 1) {
      if (/^####\s+ANCHOR\b/i.test(lines[j])) break;
      sectionLines.push(lines[j]);
    }
    const section = sectionLines.join('\n');

    const specAnchor = (section.match(/^\s*-\s*SPEC_ANCHOR\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
    const startStr = (section.match(/^\s*-\s*CONTEXT_START_LINE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
    const endStr = (section.match(/^\s*-\s*CONTEXT_END_LINE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
    const contextToken = (section.match(/^\s*-\s*CONTEXT_TOKEN\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';

    // Excerpt is a fenced block after "- EXCERPT_ASCII_ESCAPED:"
    const excerptLines = sectionLines;
    const excerpt = extractFencedBlockAfterLabel(excerptLines, 'EXCERPT_ASCII_ESCAPED').body;

    anchors.push({
      specAnchor,
      contextStartLine: startStr,
      contextEndLine: endStr,
      contextToken,
      excerpt,
    });
  }

  return anchors;
}

function isPlaceholderValue(s) {
  const v = (s || '').trim();
  if (!v) return true;
  if (v === 'PENDING') return true;
  if (/^<fill/i.test(v)) return true;
  if (/^<paste/i.test(v)) return true;
  if (v === '<pending>') return true;
  return false;
}

export function validateRefinementFile(refinementPath, { expectedWpId, requireSignature } = {}) {
  const errors = [];

  if (!fs.existsSync(refinementPath)) {
    errors.push(`Missing refinement file: ${refinementPath}`);
    return { ok: false, errors };
  }

  const content = fs.readFileSync(refinementPath, 'utf8');
  if (!isAsciiOnly(content)) {
    errors.push(`Refinement file contains non-ASCII bytes: ${refinementPath}`);
  }
  if (!hasHeading(content, 'TECHNICAL_REFINEMENT')) {
    errors.push('Refinement file missing TECHNICAL_REFINEMENT heading');
  }

  const wpId = getSingleField(content, 'WP_ID');
  if (expectedWpId && wpId !== expectedWpId) {
    errors.push(`WP_ID mismatch in refinement: expected ${expectedWpId}, got ${wpId || '<missing>'}`);
  }

  // Resolve SPEC_CURRENT and validate resolved spec + sha1.
  let resolved = null;
  try {
    resolved = resolveSpecCurrent();
  } catch (e) {
    errors.push(String(e?.message || e));
  }
  if (resolved) {
    const resolvedLine = getSingleField(content, 'SPEC_TARGET_RESOLVED');
    const expectedResolvedLine = `.GOV/spec/SPEC_CURRENT.md -> ${resolved.specFileName}`;
    if (resolvedLine !== expectedResolvedLine) {
      errors.push(`SPEC_TARGET_RESOLVED mismatch: expected "${expectedResolvedLine}", got "${resolvedLine || '<missing>'}"`);
    }

    const sha1Line = getSingleField(content, 'SPEC_TARGET_SHA1');
    if (!sha1Line || sha1Line.toLowerCase() !== resolved.sha1.toLowerCase()) {
      errors.push(`SPEC_TARGET_SHA1 mismatch: expected ${resolved.sha1}, got ${sha1Line || '<missing>'}`);
    }
  }

  // Required sections (protocol).
  ['GAPS_IDENTIFIED', 'FLIGHT_RECORDER_INTERACTION', 'RED_TEAM_ADVISORY', 'PRIMITIVES'].forEach((h) => {
    if (!hasHeading(content, h)) errors.push(`Missing required section heading: ${h}`);
  });

  // Clearly covers / enrichment fields must be filled before signature.
  const clearlyVerdict = getSingleField(content, 'CLEARLY_COVERS_VERDICT');
  if (!/^(PASS|FAIL)$/i.test(clearlyVerdict || '')) {
    errors.push('CLEARLY_COVERS_VERDICT must be PASS or FAIL (no PENDING placeholders)');
  }
  const clearlyReason = getSingleField(content, 'CLEARLY_COVERS_REASON');
  if (isPlaceholderValue(clearlyReason)) {
    errors.push('CLEARLY_COVERS_REASON must be filled (no placeholders)');
  }

  const enrichmentNeeded = getSingleField(content, 'ENRICHMENT_NEEDED');
  if (!/^(YES|NO)$/i.test(enrichmentNeeded || '')) {
    errors.push('ENRICHMENT_NEEDED must be YES or NO (no PENDING placeholders)');
  }

  // Deterministic cross-field consistency: "clearly covers" vs "enrichment needed"
  if (/^PASS$/i.test(clearlyVerdict) && /^YES$/i.test(enrichmentNeeded)) {
    errors.push('Inconsistent refinement: CLEARLY_COVERS_VERDICT=PASS requires ENRICHMENT_NEEDED=NO');
  }
  if (/^FAIL$/i.test(clearlyVerdict) && /^NO$/i.test(enrichmentNeeded)) {
    errors.push('Inconsistent refinement: CLEARLY_COVERS_VERDICT=FAIL requires ENRICHMENT_NEEDED=YES');
  }

  // Optional, but if present it must be consistent (prevents "PASS but ambiguous" procedure failures).
  const ambiguityFoundLinePresent = /^\s*-\s*AMBIGUITY_FOUND\s*:/mi.test(content);
  const ambiguityFound = ambiguityFoundLinePresent ? getSingleField(content, 'AMBIGUITY_FOUND') : '';
  if (ambiguityFoundLinePresent && !/^(YES|NO)$/i.test(ambiguityFound || '')) {
    errors.push('AMBIGUITY_FOUND must be YES or NO (no PENDING placeholders)');
  }
  if (/^YES$/i.test(ambiguityFound)) {
    if (!/^FAIL$/i.test(clearlyVerdict)) {
      errors.push('AMBIGUITY_FOUND=YES requires CLEARLY_COVERS_VERDICT=FAIL');
    }
    if (!/^YES$/i.test(enrichmentNeeded)) {
      errors.push('AMBIGUITY_FOUND=YES requires ENRICHMENT_NEEDED=YES');
    }
  }

  // Proposed spec enrichment block: enforce consistency when present.
  const lines = content.split('\n');
  const proposedViaLabel = extractFencedBlockAfterLabel(lines, 'PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)');
  const proposedViaHeading = extractFencedBlockAfterHeading(lines, 'PROPOSED_SPEC_ENRICHMENT');
  const proposedFound = proposedViaLabel.found || proposedViaHeading.found;
  const proposedBody = (proposedViaLabel.found ? proposedViaLabel.body : '') || (proposedViaHeading.found ? proposedViaHeading.body : '') || '';

  if (/^NO$/i.test(enrichmentNeeded)) {
    const reasonNo = getSingleField(content, 'REASON_NO_ENRICHMENT');
    if (isPlaceholderValue(reasonNo)) {
      errors.push('REASON_NO_ENRICHMENT is required when ENRICHMENT_NEEDED=NO');
    }

    if (proposedFound && !looksLikeNotApplicableBlock(proposedBody)) {
      errors.push('PROPOSED_SPEC_ENRICHMENT must be "<not applicable; ENRICHMENT_NEEDED=NO>" when ENRICHMENT_NEEDED=NO');
    }
  } else if (/^YES$/i.test(enrichmentNeeded)) {
    if (!proposedFound || looksLikeNotApplicableBlock(proposedBody) || looksLikePlaceholderEnrichment(proposedBody)) {
      errors.push('PROPOSED_SPEC_ENRICHMENT must contain full verbatim Markdown when ENRICHMENT_NEEDED=YES');
    }
  }

  // Anchors: must exist and be filled + token-in-window.
  const anchors = parseAnchors(content);
  if (anchors.length === 0) {
    errors.push('SPEC_ANCHORS missing: include one or more ANCHOR sections');
  }

  let specLines = null;
  if (resolved) {
    specLines = fs.readFileSync(resolved.specFilePath, 'utf8').split('\n');
  }

  anchors.forEach((a, idx) => {
    const n = idx + 1;
    if (isPlaceholderValue(a.specAnchor)) errors.push(`ANCHOR ${n}: SPEC_ANCHOR must be filled`);
    if (isPlaceholderValue(a.contextToken)) errors.push(`ANCHOR ${n}: CONTEXT_TOKEN must be filled`);

    const startNum = parseInt(a.contextStartLine, 10);
    const endNum = parseInt(a.contextEndLine, 10);
    if (Number.isNaN(startNum) || Number.isNaN(endNum) || startNum < 1 || endNum < startNum) {
      errors.push(`ANCHOR ${n}: CONTEXT_START_LINE/CONTEXT_END_LINE must be integers with start>=1 and end>=start`);
    } else if (specLines) {
      if (endNum > specLines.length) {
        errors.push(`ANCHOR ${n}: CONTEXT_END_LINE out of range (spec has ${specLines.length} lines)`);
      } else {
        const window = specLines.slice(startNum - 1, endNum).join('\n');
        if (!window.includes(a.contextToken)) {
          errors.push(`ANCHOR ${n}: CONTEXT_TOKEN not found within spec line window [${startNum}, ${endNum}]`);
        }
      }
    }

    if (isPlaceholderValue(a.excerpt)) errors.push(`ANCHOR ${n}: EXCERPT_ASCII_ESCAPED must be filled`);
  });
  // Optional but recommended: explicit user approval evidence line.
  // Enforced only when requireSignature=true to avoid blocking the initial refinement recording step.
  if (requireSignature) {
    const approvalPresent = /^\s*-\s*USER_APPROVAL_EVIDENCE\s*:/mi.test(content);
    if (approvalPresent) {
      const approvalEvidence = getSingleField(content, 'USER_APPROVAL_EVIDENCE');
      if (isPlaceholderValue(approvalEvidence)) {
        errors.push('USER_APPROVAL_EVIDENCE must be set (not <pending>) before signature/packet creation');
      } else {
        const expected = 'APPROVE REFINEMENT ' + wpId;
        if (approvalEvidence !== expected) {
          errors.push('USER_APPROVAL_EVIDENCE must equal ' + expected);
        }
      }
    }
  }

  const reviewStatus = getSingleField(content, 'USER_REVIEW_STATUS');
  const signature = getSingleField(content, 'USER_SIGNATURE');
  if (requireSignature) {
    if (!/^(APPROVED)$/i.test(reviewStatus || '')) {
      errors.push('USER_REVIEW_STATUS must be APPROVED before task packet creation');
    }
    if (!signature || signature === '<pending>') {
      errors.push('USER_SIGNATURE must be set (not <pending>) before task packet creation');
    }
  }

  return { ok: errors.length === 0, errors, parsed: { wpId, reviewStatus, signature } };
}

````

###### Template File: `.GOV/scripts/validation/task-board-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
import fs from "node:fs";

const TASK_BOARD_PATH = ".GOV/roles_shared/records/TASK_BOARD.md";

function fail(message, details = []) {
  console.error(`[TASK_BOARD_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function readTaskBoard() {
  if (!fs.existsSync(TASK_BOARD_PATH)) {
    fail("Missing task board", [`Expected: ${TASK_BOARD_PATH}`]);
  }
  return fs.readFileSync(TASK_BOARD_PATH, "utf8");
}

function sectionKeyFromHeading(headingLine) {
  const heading = headingLine.replace(/^##\s+/, "").trim();
  if (heading === "In Progress") return "IN_PROGRESS";
  if (heading === "Done") return "DONE";
  if (heading.startsWith("Superseded")) return "SUPERSEDED";
  return null;
}

function checkLines(lines) {
  const doneRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[(VALIDATED|FAIL|OUTDATED_ONLY)\]\s*$/;
  const supersededRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[SUPERSEDED\]\s*$/;
  const inProgressRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[IN_PROGRESS\]\s*$/;

  let active = null;
  const violations = [];

  for (let index = 0; index < lines.length; index += 1) {
    const lineNumber = index + 1;
    const line = lines[index];

    if (line.startsWith("## ")) {
      active = sectionKeyFromHeading(line);
      continue;
    }

    if (!active) continue;
    if (!line.trim().startsWith("-")) continue;

    if (active === "DONE" && !doneRe.test(line)) {
      violations.push(
        `${TASK_BOARD_PATH}:${lineNumber}: Done entries must be \`- **[WP_ID]** - [VALIDATED|FAIL|OUTDATED_ONLY]\`: ${line.trim()}`
      );
      continue;
    }

    if (active === "SUPERSEDED" && !supersededRe.test(line)) {
      violations.push(
        `${TASK_BOARD_PATH}:${lineNumber}: Superseded entries must be \`- **[WP_ID]** - [SUPERSEDED]\`: ${line.trim()}`
      );
      continue;
    }

    if (active === "IN_PROGRESS" && !inProgressRe.test(line)) {
      violations.push(
        `${TASK_BOARD_PATH}:${lineNumber}: In Progress entries must be \`- **[WP_ID]** - [IN_PROGRESS]\`: ${line.trim()}`
      );
    }
  }

  if (violations.length > 0) {
    fail("Task board format violations found", violations);
  }
}

const content = readTaskBoard();
checkLines(content.split(/\r?\n/));
console.log("task-board-check ok");

````

###### Template File: `.GOV/scripts/validation/task-packet-claim-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
import fs from "node:fs";
import path from "node:path";

const TASK_PACKETS_DIR = path.join("docs", "task_packets");

function fail(message, details = []) {
  console.error(`[TASK_PACKET_CLAIM_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function isPlaceholder(value) {
  const v = (value || "").trim();
  if (!v) return true;
  if (/^\{.+\}$/.test(v)) return true;
  if (/^<fill/i.test(v)) return true;
  if (/^<pending>$/i.test(v)) return true;
  if (/^<unclaimed>$/i.test(v)) return true;
  return false;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const m = text.match(re);
  return m ? m[1].trim() : "";
}

function parseStatus(text) {
  const statusLine =
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1] ||
    "";
  return statusLine.trim();
}

function normalizeStrength(value) {
  return value.toLowerCase().replace(/[\s_-]+/g, "");
}

function checkPacket(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const status = parseStatus(text);
  const statusNorm = status.toLowerCase();
  if (!/in\s*progress/.test(statusNorm)) return;

  const coderModel = parseSingleField(text, "CODER_MODEL");
  const coderStrength = parseSingleField(text, "CODER_REASONING_STRENGTH");

  const rel = filePath.split(path.sep).join("/");
  const errors = [];

  if (isPlaceholder(coderModel)) {
    errors.push(`${rel}: CODER_MODEL is required when Status is In Progress`);
  }

  if (isPlaceholder(coderStrength)) {
    errors.push(`${rel}: CODER_REASONING_STRENGTH is required when Status is In Progress`);
  } else {
    const norm = normalizeStrength(coderStrength);
    const allowed = new Set(["low", "medium", "high", "extrahigh"]);
    if (!allowed.has(norm)) {
      errors.push(
        `${rel}: CODER_REASONING_STRENGTH must be LOW|MEDIUM|HIGH|EXTRA_HIGH (got: ${coderStrength})`
      );
    }
  }

  if (errors.length > 0) fail("Coder claim fields missing/invalid", errors);
}

function main() {
  if (!fs.existsSync(TASK_PACKETS_DIR)) return;
  const files = fs
    .readdirSync(TASK_PACKETS_DIR)
    .filter((name) => name.endsWith(".md"))
    .map((name) => path.join(TASK_PACKETS_DIR, name));

  for (const filePath of files) checkPacket(filePath);
  console.log("task-packet-claim-check ok");
}

main();


````

###### Template File: `.GOV/scripts/validation/validator-coverage-gaps.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Coverage sanity helper:
 * - Ensures there is at least some test coverage in target paths.
 * - Intended as a quick guard that changed areas are protected; not a full coverage tool.
 *
 * Exits non-zero if no tests are detected in the given targets.
 */
import { execSync } from "node:child_process";

const targets = process.argv.slice(2);
const defaultTargets = [
  "{{BACKEND_SRC_DIR}}",
  "{{BACKEND_TESTS_DIR}}",
  "tests",
  "{{FRONTEND_SRC_DIR}}",
];

const scopes = targets.length > 0 ? targets : defaultTargets;

const patterns = [
  { label: "rust_tests", pattern: "#\\[test\\]" },
  { label: "ts_tests", pattern: "describe\\(" },
  { label: "ts_it", pattern: "\\bit\\(" },
];

function runRg(pattern) {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${scopes.join(
    " "
  )}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    console.error(`validator-coverage-gaps: scan failed: ${err.message}`);
    process.exit(1);
  }
}

const hits = [];
for (const pat of patterns) {
  const out = runRg(pat.pattern);
  if (out) {
    hits.push({ label: pat.label, lines: out.split("\n").length });
  }
}

if (hits.length === 0) {
  console.error(
    `validator-coverage-gaps: FAIL â€” no tests detected in ${scopes.join(", ")}. Add at least one targeted test or record an explicit waiver.`
  );
  process.exit(1);
}

console.log(
  `validator-coverage-gaps: PASS â€” tests detected (${hits
    .map((h) => `${h.label}:${h.lines}`)
    .join(", ")}).`
);

````

###### Template File: `.GOV/scripts/validation/validator-dal-audit.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * DAL audit: checks DB boundary, SQL portability, trait boundary, migration hygiene, dual-backend hints.
 * Exits non-zero on violations or missing required sections.
 */
import { execSync } from "node:child_process";
import { readdirSync } from "node:fs";

const root = process.cwd();
const backendSrc = "{{BACKEND_SRC_DIR}}";
const migrationsDir = "{{BACKEND_MIGRATIONS_DIR}}";

function runRg(pattern, paths, extraArgs = "") {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${paths.join(" ")} ${extraArgs}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    throw err;
  }
}

let failures = [];

// CX-DBP-VAL-010: No direct DB access outside storage/
{
  const outPool = runRg("state\\.pool", [backendSrc], '--glob "!**/storage/**"');
  const outSqlx = runRg("sqlx::query", [backendSrc], '--glob "!**/storage/**"');
  const hits = [outPool, outSqlx].filter(Boolean).join("\n");
  if (hits) {
    failures.push(`CX-DBP-VAL-010 (DB boundary) violations:\n${hits}`);
  }
}

// CX-DBP-VAL-011: SQL portability (SQLite-only patterns)
{
  const patterns = ["\\?1", "strftime\\(", "CREATE TRIGGER"];
  const hits = patterns
    .map((p) => runRg(p, [backendSrc, migrationsDir]))
    .filter(Boolean)
    .join("\n");
  if (hits) {
    failures.push(`CX-DBP-VAL-011 (SQL portability) violations:\n${hits}`);
  }
}

// CX-DBP-VAL-012: Trait boundary (concrete pool leakage)
{
  const out = runRg("SqlitePool", [backendSrc], '--glob "!**/storage/**"');
  if (out) {
    failures.push(`CX-DBP-VAL-012 (trait boundary) violations:\n${out}`);
  }
}

// CX-DBP-VAL-013: Migration hygiene (basic check: consecutive numbering)
try {
  const allFiles = readdirSync(migrationsDir);

  // Only treat `000X_name.sql` as versioned ups; ignore `*.down.sql` in numbering checks.
  const upFiles = allFiles.filter(
    (f) => /^\d{4}_.+\.sql$/.test(f) && !f.endsWith(".down.sql"),
  );

  const nums = upFiles.map((f) => parseInt(f.slice(0, 4), 10)).sort((a, b) => a - b);
  for (let i = 1; i < nums.length; i++) {
    if (nums[i] !== nums[i - 1] + 1) {
      failures.push(
        `CX-DBP-VAL-013 (migration hygiene): numbering gap between ${nums[i - 1]} and ${nums[i]}`,
      );
      break;
    }
  }

  // Phase 1 requirement (spec v02.106 CX-DBP-022): every up migration must have a matching down file.
  const fileSet = new Set(allFiles);
  const missingDown = upFiles
    .map((up) => up.replace(/\.sql$/, ".down.sql"))
    .filter((down) => !fileSet.has(down));
  if (missingDown.length > 0) {
    failures.push(
      `CX-DBP-VAL-013 (migration hygiene): missing down migrations for:\n${missingDown.join("\n")}`,
    );
  }
} catch (err) {
  failures.push(`CX-DBP-VAL-013 (migration hygiene): failed to read migrations dir: ${err.message}`);
}

// CX-DBP-VAL-014: Dual-backend readiness (presence of postgres/parameterization hints)
{
  const out = runRg("postgres|Postgres|PgPool|PgConnection", [backendSrc, migrationsDir]);
  if (!out) {
    failures.push("CX-DBP-VAL-014 (dual-backend readiness): no PostgreSQL hints/tests found; add or document gap.");
  }
}

if (failures.length > 0) {
  console.error("validator-dal-audit: FAIL");
  failures.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log("validator-dal-audit: PASS (DAL checks clean).");

````

###### Template File: `.GOV/scripts/validation/validator-error-codes.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Error/trace determinism audit:
 * - Flags stringly errors in production paths
 * - Flags unseeded randomness/time sources in production paths
 *
 * Exits non-zero on findings.
 */
import fs from "node:fs";
import path from "node:path";

const targets = ["{{BACKEND_SRC_DIR}}"];
const waiverMarker = "WAIVER [CX-573E]";

const stringErrorPatterns = [
  'Err\\(\\s*"', // Err("msg")
  "Err\\(\\s*String::from",
  "Err\\(\\s*format!",
  'map_err\\(\\|.*\\|\\s*"', // map_err(|_| "msg")
  "map_err\\(\\|.*\\|\\s*String::from",
  "map_err\\(\\|.*\\|\\s*format!",
  "anyhow!\\(",
  "bail!\\(",
];

const nondeterminismPatterns = [
  "rand::",
  "thread_rng",
  "rand\\(",
  "Instant::now\\(",
  "SystemTime::now\\(",
];

function toPosixPath(filePath) {
  return filePath.replace(/\\/g, "/");
}

function shouldExclude(relativePosixPath) {
  if (relativePosixPath.includes("/tests/")) return true;

  const parts = relativePosixPath.split("/");
  const filename = parts.at(-1) ?? "";
  if (filename.includes("test")) return true;
  for (const part of parts.slice(0, -1)) {
    if (part.includes("test")) return true;
  }

  return false;
}

function collectTargetFiles() {
  const files = [];

  for (const target of targets) {
    const targetAbs = path.resolve(process.cwd(), target);
    if (!fs.existsSync(targetAbs)) continue;

    const stack = [{ absDir: targetAbs, relDir: target }];
    while (stack.length > 0) {
      const next = stack.pop();
      if (!next) break;

      let entries;
      try {
        entries = fs.readdirSync(next.absDir, { withFileTypes: true });
      } catch (err) {
        console.error(
          `validator-error-codes: failed to read directory ${next.absDir}: ${err.message}`
        );
        process.exit(1);
      }

      entries.sort((a, b) => a.name.localeCompare(b.name));

      for (const entry of entries) {
        const absPath = path.join(next.absDir, entry.name);
        const relPath = path.join(next.relDir, entry.name);
        const relPosix = toPosixPath(relPath);

        if (entry.isDirectory()) {
          if (shouldExclude(`${relPosix}/`)) continue;
          stack.push({ absDir: absPath, relDir: relPath });
          continue;
        }

        if (!entry.isFile()) continue;
        if (shouldExclude(relPosix)) continue;
        if (!relPosix.endsWith(".rs")) continue;

        files.push({ absPath, relPosix });
      }
    }
  }

  files.sort((a, b) => a.relPosix.localeCompare(b.relPosix));
  return files;
}

function normalizeNewlines(text) {
  return text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
}

function buildLineStarts(lines) {
  const starts = new Array(lines.length);
  let offset = 0;
  for (let i = 0; i < lines.length; i += 1) {
    starts[i] = offset;
    offset += lines[i].length + 1;
  }
  return starts;
}

function findLineIndex(lineStarts, offset) {
  let low = 0;
  let high = lineStarts.length - 1;
  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    if (lineStarts[mid] <= offset) {
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }
  return Math.max(0, low - 1);
}

function hasAdjacentWaiver(lines, lineIndex) {
  const cur = lines[lineIndex] ?? "";
  const prev = lineIndex > 0 ? lines[lineIndex - 1] ?? "" : "";
  return cur.includes(waiverMarker) || prev.includes(waiverMarker);
}

function scanPatternAcrossFiles(files, pattern, label) {
  const regex = new RegExp(pattern, "g");
  const hits = [];

  for (const file of files) {
    let text;
    try {
      text = fs.readFileSync(file.absPath, "utf8");
    } catch (err) {
      console.error(
        `validator-error-codes: ${label} scan failed: cannot read ${file.relPosix}: ${err.message}`
      );
      process.exit(1);
    }

    const normalized = normalizeNewlines(text);
    const lines = normalized.split("\n");
    const lineStarts = buildLineStarts(lines);

    regex.lastIndex = 0;
    const matchedLineNumbers = new Set();

    while (true) {
      const match = regex.exec(normalized);
      if (!match) break;

      const lineIndex = findLineIndex(lineStarts, match.index);
      const lineNumber = lineIndex + 1;

      if (
        label === "determinism" &&
        (pattern === "Instant::now\\(" || pattern === "SystemTime::now\\(") &&
        hasAdjacentWaiver(lines, lineIndex)
      ) {
        continue;
      }

      if (matchedLineNumbers.has(lineNumber)) continue;
      matchedLineNumbers.add(lineNumber);
      hits.push(`${file.relPosix}:${lineNumber}:${lines[lineIndex] ?? ""}`);
    }
  }

  return hits.join("\n");
}

const findings = [];
const files = collectTargetFiles();

for (const pat of stringErrorPatterns) {
  const out = scanPatternAcrossFiles(files, pat, "string-error");
  if (out) {
    findings.push(`STRING_ERROR pattern "${pat}":\n${out}`);
  }
}

for (const pat of nondeterminismPatterns) {
  const out = scanPatternAcrossFiles(files, pat, "determinism");
  if (out) {
    findings.push(`NONDETERMINISM pattern "${pat}":\n${out}`);
  }
}

if (findings.length > 0) {
  console.error("validator-error-codes: FAIL/WARN findings detected");
  findings.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log(
  "validator-error-codes: PASS - no stringly errors or nondeterminism patterns detected."
);

````

###### Template File: `.GOV/scripts/validation/validator-git-hygiene.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Git/Build hygiene audit:
 * - Ensures .gitignore covers standard build artifacts.
 * - Fails if common artifacts are committed or large untracked files exist.
 *
 * Exits non-zero on findings.
 */
import { execSync } from "node:child_process";
import { readFileSync, statSync } from "node:fs";

const gitignorePath = ".gitignore";
const requiredPatterns = ["target/", "node_modules/", "*.pdb", "*.dSYM", ".DS_Store", "Thumbs.db"];
const artifactRegex =
  /(\/|^)(target\/|node_modules\/)|\.pdb$|\.dSYM$|\.DS_Store$|Thumbs\.db$/;

function fail(message, details = "") {
  console.error(`validator-git-hygiene: FAIL â€” ${message}`);
  if (details) console.error(details);
  process.exit(1);
}

let gitignore;
try {
  gitignore = readFileSync(gitignorePath, "utf8");
} catch (err) {
  fail(`cannot read ${gitignorePath}: ${err.message}`);
}

const missing = requiredPatterns.filter((p) => !gitignore.includes(p));
if (missing.length > 0) {
  fail(`.gitignore missing patterns: ${missing.join(", ")}`);
}

let committedArtifacts = "";
try {
  const out = execSync("git ls-files", { stdio: "pipe", encoding: "utf8" });
  committedArtifacts = out
    .split("\n")
    .filter((line) => artifactRegex.test(line))
    .filter(Boolean)
    .join("\n");
} catch (err) {
  fail(`git ls-files failed: ${err.message}`);
}

if (committedArtifacts.trim().length > 0) {
  fail(`committed build artifacts detected:\n${committedArtifacts}`);
}

let largeUntracked = "";
try {
  const out = execSync("git ls-files --others --exclude-standard", {
    stdio: "pipe",
    encoding: "utf8",
  });
  const files = out.split("\n").filter(Boolean);
  const offenders = [];
  for (const f of files) {
    try {
      const st = statSync(f);
      if (st.size > 10 * 1024 * 1024) {
        offenders.push(`${f} (${(st.size / (1024 * 1024)).toFixed(1)}MB)`);
      }
    } catch {
      // ignore missing files
    }
  }
  largeUntracked = offenders.join("\n");
} catch (err) {
  fail(`git ls-files (untracked) failed: ${err.message}`);
}

if (largeUntracked.trim().length > 0) {
  fail(`untracked large files detected (>10MB):\n${largeUntracked}`);
}

console.log("validator-git-hygiene: PASS â€” .gitignore coverage and artifact checks clean.");

````

###### Template File: `.GOV/scripts/validation/validator-hygiene-full.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Composite hygiene runner for validators.
 * Runs scan, error-codes, traceability, and git hygiene checks.
 */
import { execSync } from "node:child_process";

const cmds = [
  "node .GOV/scripts/validation/validator-scan.mjs",
  "node .GOV/scripts/validation/validator-error-codes.mjs",
  "node .GOV/scripts/validation/validator-traceability.mjs",
  "node .GOV/scripts/validation/validator-git-hygiene.mjs",
];

function run(cmd) {
  try {
    execSync(cmd, { stdio: "inherit" });
  } catch (err) {
    console.error(`validator-hygiene-full: FAIL â€” command failed: ${cmd}`);
    process.exit(1);
  }
}

for (const cmd of cmds) {
  run(cmd);
}

console.log("validator-hygiene-full: PASS â€” composite hygiene clean.");

````

###### Template File: `.GOV/scripts/validation/validator-packet-complete.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Packet completeness checker for validators.
 * Ensures required fields are present and sane.
 */
import { readFileSync } from "node:fs";

const wpId = process.argv[2];
if (!wpId) {
  console.error("Usage: just validator-packet-complete WP-1-Example");
  process.exit(1);
}

const path = `.GOV/task_packets/${wpId}.md`;

function fail(msg) {
  console.error(`validator-packet-complete: FAIL â€” ${msg}`);
  process.exit(1);
}

let text;
try {
  text = readFileSync(path, "utf8");
} catch (err) {
  fail(`cannot read ${path}: ${err.message}`);
}

function hasLine(re) {
  return re.test(text);
}

if (!hasLine(/(?:\*\*Status:\*\*|STATUS:)\s*(Ready for Dev|In Progress|Done(?:\s*\(Historical\))?)\b/i)) {
  fail("STATUS missing or invalid (must be Ready for Dev / In Progress / Done / Done (Historical))");
}

const hasLegacySpec = hasLine(/SPEC_CURRENT/i);
const hasSpecBaseline = hasLine(/SPEC_BASELINE/i);
const hasSpecTarget = hasLine(/SPEC_TARGET/i);
if (!hasLegacySpec && !(hasSpecBaseline && hasSpecTarget)) {
  fail("SPEC reference missing (need SPEC_CURRENT or SPEC_BASELINE+SPEC_TARGET)");
}
if (!hasLine(/RISK_TIER/i)) {
  fail("RISK_TIER missing");
}
if (!hasLine(/DONE_MEANS/i) || hasLine(/DONE_MEANS\s*:\s*$/i) || hasLine(/DONE_MEANS\s*:\s*tbd/i)) {
  fail("DONE_MEANS missing or placeholder");
}
if (!hasLine(/TEST_PLAN/i) || hasLine(/TEST_PLAN\s*:\s*$/i) || hasLine(/TEST_PLAN\s*:\s*tbd/i)) {
  fail("TEST_PLAN missing or placeholder");
}
if (!hasLine(/BOOTSTRAP/i)) {
  fail("BOOTSTRAP missing");
}
if (!hasLine(/USER_SIGNATURE/i) && !hasLine(/User Signature Locked/i)) {
  fail("USER_SIGNATURE missing");
}

console.log(`validator-packet-complete: PASS â€” ${wpId} has required fields.`);

````

###### Template File: `.GOV/scripts/validation/validator-phase-gate.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Phase gate check: ensure no Ready-for-Dev items remain before phase progression and validator scans are clean.
 */
import { readFileSync } from "node:fs";

const phase = process.argv[2] || "Phase-1";
const taskBoardPath = ".GOV/roles_shared/records/TASK_BOARD.md";

function fail(msg) {
  console.error(`validator-phase-gate: FAIL - ${msg}`);
  process.exit(1);
}

function extractSectionLines(board, headingText) {
  const lines = board.split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${headingText}\\s*$`, "i");

  const startIndex = lines.findIndex((line) => headingRe.test(line.trimEnd()));
  if (startIndex === -1) return null;

  const section = [];
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.startsWith("## ")) break;
    section.push(line);
  }

  return section;
}

function countWpEntries(sectionLines) {
  const wpEntryRe = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*/;
  const ids = new Set();
  for (const line of sectionLines) {
    const match = line.match(wpEntryRe);
    if (match) ids.add(match[1]);
  }
  return ids.size;
}

function main() {
  let board;
  try {
    board = readFileSync(taskBoardPath, "utf8");
  } catch (err) {
    fail(`cannot read ${taskBoardPath}: ${err.message}`);
  }

  const readyForDevLines = extractSectionLines(board, "Ready for Dev");
  if (!readyForDevLines) {
    fail(`missing "## Ready for Dev" section in ${taskBoardPath}`);
  }

  const readyCount = countWpEntries(readyForDevLines);
  if (readyCount > 0) {
    fail(
      `Task Board still has ${readyCount} Ready for Dev item(s); phase progression for ${phase} is blocked.`
    );
  }

  console.log(`validator-phase-gate: PASS - no Ready for Dev items detected for ${phase}.`);
}

main();

````

###### Template File: `.GOV/scripts/validation/validator-scan.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Validator scan: forbidden patterns and placeholder text in backend and frontend sources.
 * Exits non-zero if any finding is detected.
 */
import { execSync } from "node:child_process";

const targets = ["{{BACKEND_SRC_DIR}}", "{{FRONTEND_SRC_DIR}}"];
const GLOB_RS = '--glob "*.rs"';

const forbidden = [
  "\\\\bsplit_whitespace\\\\(\\\\)",
  "\\\\bunwrap\\\\(\\\\)",
  "expect\\(",
  "todo!",
  "unimplemented!",
  "dbg!",
  "println!",
  "eprintln!",
  "panic!",
];

const placeholder = ["Mock", "Stub", "placeholder", "hollow"];

function runRg(pattern, paths = targets, extraArgs = "") {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${paths.join(
    " "
  )} ${GLOB_RS} ${extraArgs}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    throw err;
  }
}

const findings = [];

for (const pat of forbidden) {
  const out = runRg(pat);
  if (out) {
    findings.push(`FORBIDDEN_PATTERN "${pat}":\n${out}`);
  }
}

for (const pat of placeholder) {
  const out = runRg(pat);
  if (out) {
    findings.push(`PLACEHOLDER/MOCK "${pat}":\n${out}`);
  }
}

if (findings.length > 0) {
  console.error("validator-scan: FAIL â€” findings detected");
  findings.forEach((f) => {
    console.error("----");
    console.error(f);
  });
  process.exit(1);
}

console.log("validator-scan: PASS â€” no forbidden patterns detected in backend sources.");

````

###### Template File: `.GOV/scripts/validation/validator-spec-regression.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Spec regression check: ensure SPEC_CURRENT points to existing spec and required anchors are present.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

const specPointerPath = ".GOV/spec/SPEC_CURRENT.md";
// Phase/safety-critical anchors that must exist in the current spec.
const requiredAnchors = [
  "2.3.12", // storage portability pillars
  "2.3.11", // retention/GC
  "2.6.7",  // semantic catalog
  "2.9.2",  // mutation traceability / silent edit guard
  "4.6",    // tokenization
];

function fail(msg) {
  console.error(`validator-spec-regression: FAIL â€” ${msg}`);
  process.exit(1);
}

function main() {
  let specPointer;
  try {
    specPointer = readFileSync(specPointerPath, "utf8");
  } catch (err) {
    fail(`cannot read ${specPointerPath}: ${err.message}`);
  }

  const match = specPointer.match(/\*\*({{PROJECT_PREFIX}}_Master_Spec_[^*]+)\*\*/);
  if (!match) {
    fail("SPEC_CURRENT does not reference a Master Spec filename.");
  }
  const specFile = match[1];
  const specPath = join(specFile); // specs live at repo root

  let spec;
  try {
    spec = readFileSync(specPath, "utf8");
  } catch (err) {
    fail(`cannot read referenced spec ${specPath}: ${err.message}`);
  }

  for (const anchor of requiredAnchors) {
    if (!spec.includes(anchor)) {
      fail(`required spec anchor "${anchor}" missing in ${specFile}`);
    }
  }

  console.log(`validator-spec-regression: PASS â€” ${specFile} present with required anchors.`);
}

main();

````

###### Template File: `.GOV/scripts/validation/validator-traceability.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
#!/usr/bin/env node
/**
 * Traceability audit:
 * - Ensures job_id appears in governed mutation paths.
 * - Emits a warning (non-fatal) if trace_id is absent.
 *
 * Exits non-zero only if required fields are absent.
 */
import { execSync } from "node:child_process";

const targets = process.argv.slice(2);
const defaultTargets = [
  "{{BACKEND_SRC_DIR}}/workflows.rs",
  "{{BACKEND_SRC_DIR}}/api",
  "{{BACKEND_SRC_DIR}}/storage",
];

const scopes = targets.length > 0 ? targets : defaultTargets;

function runRg(pattern) {
  const cmd = `rg --hidden --no-heading --line-number "${pattern}" ${scopes.join(
    " "
  )}`;
  try {
    const out = execSync(cmd, { stdio: "pipe", encoding: "utf8" });
    return out.trim();
  } catch (err) {
    if (err.status === 1) return "";
    console.error(`validator-traceability: scan failed: ${err.message}`);
    process.exit(1);
  }
}

const jobHits = runRg("job_id");
const traceHits = runRg("trace_id");

const failures = [];
const warnings = [];
if (!jobHits) failures.push("job_id not found in governed paths");
if (!traceHits) warnings.push("trace_id not found in governed paths (warning only)");

if (failures.length > 0) {
  console.error("validator-traceability: FAIL â€” missing traceability fields");
  failures.forEach((f) => console.error(`- ${f}`));
  warnings.forEach((w) => console.error(`- ${w}`));
  process.exit(1);
}

warnings.forEach((w) => console.warn(`validator-traceability: WARN â€” ${w}`));
console.log("validator-traceability: PASS â€” required traceability fields present.");

````

###### Template File: `.GOV/scripts/validation/validator_gates.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
/**
 * Validator Gates [CX-VAL-GATE]
 *
 * Mechanical enforcement of validation gate sequence.
 * Prevents auto-commit, ensures user sees report before WP append.
 *
 * Actions:
 *   present-report {WP_ID} {PASS|FAIL}  - Gate 1: Record report shown in chat
 *   acknowledge {WP_ID}                  - Gate 2: Record user acknowledgment
 *   append {WP_ID}                       - Gate 3: Record WP append completed
 *   commit {WP_ID}                       - Gate 4: Allow commit (PASS only)
 *   status {WP_ID}                       - Show current gate state
 *   reset {WP_ID}                        - Reset gates for WP (requires confirmation)
 */

import fs from 'fs';
import path from 'path';

const LEGACY_STATE_FILE = '.GOV/roles/validator/VALIDATOR_GATES.json';
const STATE_DIR = path.join('docs', 'validator_gates');
const MIN_GATE_INTERVAL_SECONDS = 5; // Minimum time between gates to prevent automation momentum

function ensureStateDir() {
    if (!fs.existsSync(STATE_DIR)) {
        fs.mkdirSync(STATE_DIR, { recursive: true });
    }
}

function stateFilePath(wpId) {
    return path.join(STATE_DIR, `${wpId}.json`);
}

function normalizeState(raw) {
    const validation_sessions =
        raw?.validation_sessions && typeof raw.validation_sessions === 'object'
            ? raw.validation_sessions
            : {};

    return {
        validation_sessions,
        archived_sessions: Array.isArray(raw?.archived_sessions) ? raw.archived_sessions : [],
    };
}

function loadWpState(wpId) {
    ensureStateDir();

    const perFile = stateFilePath(wpId);
    if (fs.existsSync(perFile)) {
        const raw = JSON.parse(fs.readFileSync(perFile, 'utf8'));
        return normalizeState(raw);
    }

    // Back-compat: if a legacy global ledger exists, read state for this WP_ID only.
    if (fs.existsSync(LEGACY_STATE_FILE)) {
        const legacy = JSON.parse(fs.readFileSync(LEGACY_STATE_FILE, 'utf8'));
        const session = legacy?.validation_sessions?.[wpId] || null;
        const archived = Array.isArray(legacy?.archived_sessions)
            ? legacy.archived_sessions.filter((s) => s?.wpId === wpId)
            : [];

        return normalizeState({
            validation_sessions: session ? { [wpId]: session } : {},
            archived_sessions: archived,
        });
    }

    return normalizeState({});
}

function saveWpState(wpId, state) {
    ensureStateDir();
    const perFile = stateFilePath(wpId);

    const session = state?.validation_sessions?.[wpId] || null;
    const archived = Array.isArray(state?.archived_sessions)
        ? state.archived_sessions.filter((s) => s?.wpId === wpId)
        : [];

    const toWrite = normalizeState({
        validation_sessions: session ? { [wpId]: session } : {},
        archived_sessions: archived,
    });

    fs.writeFileSync(perFile, `${JSON.stringify(toWrite, null, 2)}\n`);
}

function fail(msg, details = []) {
    console.error(`[VALIDATOR GATE ERROR] ${msg}`);
    details.forEach((d) => console.error(`  - ${d}`));
    process.exit(1);
}

function success(msg, details = []) {
    console.log(`[VALIDATOR GATE] ${msg}`);
    details.forEach((d) => console.log(`  ${d}`));
}

function assertWpId(id) {
    if (!id || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(id)) {
        fail('Expected WP_ID like WP-1-Feature-Name-v1');
    }
}

function getSession(state, wpId) {
    return state.validation_sessions[wpId] || null;
}

function checkMomentum(session, gateName) {
    if (!session || !session.gates || session.gates.length === 0) return;

    const lastGate = session.gates[session.gates.length - 1];
    const lastTime = new Date(lastGate.timestamp);
    const now = new Date();
    const diffSeconds = (now.getTime() - lastTime.getTime()) / 1000;

    if (diffSeconds < MIN_GATE_INTERVAL_SECONDS) {
        fail(`Automation momentum detected for ${gateName}`, [
            `Last gate (${lastGate.gate}) was ${diffSeconds.toFixed(1)}s ago`,
            `Minimum interval: ${MIN_GATE_INTERVAL_SECONDS}s`,
            'Protocol requires user review between gates'
        ]);
    }
}

const action = process.argv[2];
const wpId = process.argv[3];
const extraArg = process.argv[4];

// =============================================================================
// ACTION: present-report {WP_ID} {PASS|FAIL}
// =============================================================================
if (action === 'present-report') {
    assertWpId(wpId);
    const state = loadWpState(wpId);
    const verdict = extraArg?.toUpperCase();

    if (verdict !== 'PASS' && verdict !== 'FAIL') {
        fail('Verdict must be PASS or FAIL', [`Received: ${extraArg}`]);
    }

    const existing = getSession(state, wpId);
    if (existing && existing.status === 'COMMITTED') {
        fail(`${wpId} already has a committed validation session`, [
            'Create a new WP variant (e.g., WP-1-Feature-v2) for re-validation'
        ]);
    }

    // Start new session or reset if re-presenting
    state.validation_sessions[wpId] = {
        wpId,
        verdict,
        status: 'REPORT_PRESENTED',
        started: new Date().toISOString(),
        gates: [{
            gate: 'REPORT_PRESENTED',
            verdict,
            timestamp: new Date().toISOString()
        }]
    };
    saveWpState(wpId, state);

    success(`Gate 1 PASSED: Report presented for ${wpId}`, [
        `Verdict: ${verdict}`,
        '',
        '[HALT] Validator MUST now wait for user acknowledgment.',
        `[NEXT] After user reviews, run: just validator-gate-acknowledge ${wpId}`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: acknowledge {WP_ID}
// =============================================================================
if (action === 'acknowledge') {
    assertWpId(wpId);
    const state = loadWpState(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`, [
            `Run: just validator-gate-present ${wpId} {PASS|FAIL}`
        ]);
    }

    if (session.status !== 'REPORT_PRESENTED') {
        fail(`Cannot acknowledge: ${wpId} is in state ${session.status}`, [
            'Expected state: REPORT_PRESENTED'
        ]);
    }

    checkMomentum(session, 'USER_ACKNOWLEDGED');

    session.status = 'USER_ACKNOWLEDGED';
    session.gates.push({
        gate: 'USER_ACKNOWLEDGED',
        timestamp: new Date().toISOString()
    });
    saveWpState(wpId, state);

    success(`Gate 2 PASSED: User acknowledged report for ${wpId}`, [
        '',
        '[HALT] Validator may now append report to WP.',
        `[NEXT] Run: just validator-gate-append ${wpId}`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: append {WP_ID}
// =============================================================================
if (action === 'append') {
    assertWpId(wpId);
    const state = loadWpState(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`);
    }

    if (session.status !== 'USER_ACKNOWLEDGED') {
        fail(`Cannot append: ${wpId} is in state ${session.status}`, [
            'Expected state: USER_ACKNOWLEDGED',
            'User must acknowledge the report before it can be appended'
        ]);
    }

    checkMomentum(session, 'WP_APPENDED');

    // Verify task packet exists
    const packetPath = `.GOV/task_packets/${wpId}.md`;
    if (!fs.existsSync(packetPath)) {
        fail(`Task packet not found: ${packetPath}`);
    }

    session.status = 'WP_APPENDED';
    session.gates.push({
        gate: 'WP_APPENDED',
        timestamp: new Date().toISOString()
    });
    saveWpState(wpId, state);

    if (session.verdict === 'FAIL') {
        success(`Gate 3 PASSED: Report appended to ${wpId}`, [
            '',
            '[STOP] Verdict was FAIL - no commit allowed.',
            'WP remains open for remediation.'
        ]);
    } else {
        success(`Gate 3 PASSED: Report appended to ${wpId}`, [
            '',
            '[HALT] Validator may now commit.',
            `[NEXT] Run: just validator-gate-commit ${wpId}`
        ]);
    }
    process.exit(0);
}

// =============================================================================
// ACTION: commit {WP_ID}
// =============================================================================
if (action === 'commit') {
    assertWpId(wpId);
    const state = loadWpState(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        fail(`No validation session for ${wpId}`);
    }

    if (session.verdict !== 'PASS') {
        fail(`Cannot commit: ${wpId} verdict was ${session.verdict}`, [
            'Only PASS verdicts may be committed',
            'Fix issues and re-validate to get a PASS'
        ]);
    }

    if (session.status !== 'WP_APPENDED') {
        fail(`Cannot commit: ${wpId} is in state ${session.status}`, [
            'Expected state: WP_APPENDED',
            'Complete all prior gates before committing'
        ]);
    }

    checkMomentum(session, 'COMMITTED');

    session.status = 'COMMITTED';
    session.gates.push({
        gate: 'COMMITTED',
        timestamp: new Date().toISOString()
    });
    session.completed = new Date().toISOString();
    saveWpState(wpId, state);

    success(`Gate 4 PASSED: ${wpId} cleared for commit`, [
        '',
        '[UNLOCKED] Validator may now run git commit.',
        `Commit message: docs: validation PASS [${wpId}]`
    ]);
    process.exit(0);
}

// =============================================================================
// ACTION: status {WP_ID}
// =============================================================================
if (action === 'status') {
    assertWpId(wpId);
    const state = loadWpState(wpId);

    const session = getSession(state, wpId);
    if (!session) {
        console.log(`[VALIDATOR GATE STATUS] No session for ${wpId}`);
        console.log('  Gates: (none)');
        process.exit(0);
    }

    console.log(`[VALIDATOR GATE STATUS] ${wpId}`);
    console.log(`  Verdict: ${session.verdict}`);
    console.log(`  Status: ${session.status}`);
    console.log(`  Started: ${session.started}`);
    console.log('  Gates:');
    session.gates.forEach((g, i) => {
        const check = i < session.gates.length ? 'âœ“' : 'â—‹';
        console.log(`    ${check} ${g.gate} @ ${g.timestamp}`);
    });

    // Show next action
    const nextActions = {
        'REPORT_PRESENTED': `just validator-gate-acknowledge ${wpId}`,
        'USER_ACKNOWLEDGED': `just validator-gate-append ${wpId}`,
        'WP_APPENDED': session.verdict === 'PASS' ? `just validator-gate-commit ${wpId}` : '(FAIL - no commit)',
        'COMMITTED': '(complete)'
    };
    console.log(`  Next: ${nextActions[session.status] || 'unknown'}`);
    process.exit(0);
}

// =============================================================================
// ACTION: reset {WP_ID} --confirm
// =============================================================================
if (action === 'reset') {
    assertWpId(wpId);
    const state = loadWpState(wpId);

    if (extraArg !== '--confirm') {
        fail('Reset requires confirmation', [
            `Run: just validator-gate-reset ${wpId} --confirm`
        ]);
    }

    const session = getSession(state, wpId);
    if (!session) {
        console.log(`[VALIDATOR GATE] No session to reset for ${wpId}`);
        process.exit(0);
    }

    // Archive old session
    if (!state.archived_sessions) state.archived_sessions = [];
    state.archived_sessions.push({
        ...session,
        archived_at: new Date().toISOString(),
        archive_reason: 'manual_reset'
    });

    delete state.validation_sessions[wpId];
    saveWpState(wpId, state);

    success(`Session reset for ${wpId}`, [
        'Previous session archived',
        'You may start a new validation'
    ]);
    process.exit(0);
}

// =============================================================================
// Unknown action
// =============================================================================
fail('Unknown action', [
    'Valid actions: present-report, acknowledge, append, commit, status, reset',
    '',
    'Usage:',
    '  just validator-gate-present {WP_ID} {PASS|FAIL}',
    '  just validator-gate-acknowledge {WP_ID}',
    '  just validator-gate-append {WP_ID}',
    '  just validator-gate-commit {WP_ID}',
    '  just validator-gate-status {WP_ID}',
    '  just validator-gate-reset {WP_ID} --confirm'
]);

````

###### Template File: `.GOV/scripts/validation/worktree-concurrency-check.mjs`
Intent: Mechanical governance gate (see filename + internal docstrings).
````js
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const TASK_BOARD_PATH = ".GOV/roles_shared/records/TASK_BOARD.md";

function runGit(args) {
  return execFileSync("git", args, { stdio: "pipe" }).toString().trim();
}

function getWorktreesDir() {
  try {
    const commonDir = runGit(["rev-parse", "--git-common-dir"]);
    if (!commonDir) return null;
    return path.join(path.resolve(process.cwd(), commonDir), "worktrees");
  } catch {
    return null;
  }
}

function fail(message, details = []) {
  console.error(`[WORKTREE_CONCURRENCY_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function countInProgressWps(taskBoard) {
  const re = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[IN_PROGRESS\]\s*$/;
  return taskBoard.split(/\r?\n/).filter((line) => re.test(line)).length;
}

function countLinkedWorktrees() {
  const worktreesDir = getWorktreesDir();
  if (!worktreesDir) return 0;
  if (!fs.existsSync(worktreesDir)) return 0;
  try {
    return fs
      .readdirSync(worktreesDir, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .length;
  } catch {
    return 0;
  }
}

// Local guard only; CI clones cannot/should not be required to have worktrees.
if (process.env.CI || process.env.GITHUB_ACTIONS) {
  console.log("worktree-concurrency-check ok (skipped in CI)");
  process.exit(0);
}

if (!fs.existsSync(TASK_BOARD_PATH)) {
  fail("Missing task board", [`Expected: ${TASK_BOARD_PATH}`]);
}

const taskBoard = fs.readFileSync(TASK_BOARD_PATH, "utf8");
const inProgress = countInProgressWps(taskBoard);
const requiredLinkedWorktrees = Math.max(0, inProgress - 1);
const linkedWorktrees = countLinkedWorktrees();

if (linkedWorktrees < requiredLinkedWorktrees) {
  const worktreesDir = getWorktreesDir();
  fail("Concurrent WPs require git worktrees (per protocols).", [
    `In Progress WPs: ${inProgress}`,
    `Linked worktrees present: ${linkedWorktrees} (dir: ${worktreesDir ?? "(unknown)"})`,
    `Required linked worktrees: ${requiredLinkedWorktrees}`,
    `Create: just worktree-add WP-<ID> (or: git worktree add ..\\wt-WP-<ID> feat/WP-<ID>)`,
  ]);
}

console.log("worktree-concurrency-check ok");

````

###### Template File: `.GOV/roles/coder/CODER_RUBRIC.md`
Intent: Role quality rubric for Coders (advisory; non-authoritative; improves self-audit and validator alignment).
````md
# CODER RUBRIC: Internal Quality Standard [CX-620-625]

**Purpose:** Define what a PERFECT Coder looks like. Use this for self-evaluation before requesting commit.

**Current Grade:** B+ (82/100) â€” Functional but incomplete
**Target Grade:** A+ (91/100) â€” Reliable, thorough, well-integrated
**Audience:** Coder agents (you); Orchestrator (for delegation verification); Validator (for acceptance criteria)

---

## Section 0: Role Definition

### What IS a Coder

You are a **Software Engineer** (implementation specialist). Your job is to:
1. âœ… **Verify task packet** exists and is complete BEFORE writing any code
2. âœ… **Understand scope** strictly (IN_SCOPE_PATHS, OUT_OF_SCOPE, DONE_MEANS)
3. âœ… **Implement EXACTLY** what the task packet requires (no more, no less)
4. âœ… **Validate thoroughly** (run TEST_PLAN, complete manual review, update packet)
5. âœ… **Document completion** (VALIDATION block, DONE_MEANS proof, commit message)

### What IS NOT a Coder

You are NOT:
- âŒ An Architect (scope design is Orchestrator's job)
- âŒ A Validator (review is Validator's job)
- âŒ A Gardener (refactoring unrelated code)
- âŒ An Improviser (inventing requirements you think are needed)
- âŒ A Sprinter (rushing to commit without validation)

**Core Principle:** You are a precision instrument. Follow the task packet exactly.

---

## Section 1: Five Core Responsibilities (With Quality Standards)

### Responsibility 1: Task Packet Verification [CX-620]

**What you do:**
- [ ] Verify task packet file exists (.GOV/task_packets/WP-*.md)
- [ ] Verify packet has all 10 required fields
- [ ] Verify packet fields meet COMPLETENESS CRITERIA (see below)
- [ ] If incomplete: BLOCK and request Orchestrator to fix

**Completeness Criteria (MUST have ALL):**
- [ ] TASK_ID and WP_ID are unique and match format
- [ ] STATUS is `Ready-for-Dev` or `In-Progress` (not TBD/Draft)
- [ ] RISK_TIER is LOW/MEDIUM/HIGH with justification
- [ ] SCOPE is concrete (not vague like "improve storage")
- [ ] IN_SCOPE_PATHS are specific files (not "src/backend")
- [ ] OUT_OF_SCOPE lists 3-8 deferred items with reasons
- [ ] TEST_PLAN has concrete commands (no placeholders like "run tests")
- [ ] DONE_MEANS are measurable (3-8 items, verifiable yes/no)
- [ ] ROLLBACK_HINT explains how to undo the work
- [ ] BOOTSTRAP has all 4 sub-fields (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)

**Quality Gates:**
- âœ… Accept packet â†’ Proceed to Step 2
- âŒ Incomplete packet â†’ BLOCK: "Missing {field}. Orchestrator: please complete before I proceed."
- âŒ Ambiguous packet â†’ BLOCK: "SCOPE ambiguous on {question}. Please clarify."
- âŒ Contradictory packet â†’ BLOCK: "IN_SCOPE includes X but OUT_OF_SCOPE forbids X. Conflict."

**Success:** You confidently understand what you're building and why.

---

### Responsibility 2: BOOTSTRAP Protocol [CX-577-622]

**What you do:**
- [ ] Read all files listed in packet BOOTSTRAP (FILES_TO_OPEN)
- [ ] Run all commands listed in packet BOOTSTRAP (RUN_COMMANDS)
- [ ] Search for all patterns listed in packet BOOTSTRAP (SEARCH_TERMS)
- [ ] Map risk scenarios from packet BOOTSTRAP (RISK_MAP)
- [ ] OUTPUT BOOTSTRAP block (your understanding before coding)

**BOOTSTRAP Block Format (MANDATORY 4 sub-fields):**

```
BOOTSTRAP [CX-577, CX-622]
========================================
WP_ID: {from packet}
RISK_TIER: {from packet}
TASK_TYPE: {DEBUG|FEATURE|REFACTOR|HYGIENE}

FILES_TO_OPEN: {verify you read all}
- .GOV/roles_shared/docs/START_HERE.md
- .GOV/spec/SPEC_CURRENT.md
- .GOV/roles_shared/docs/ARCHITECTURE.md
- {5-15 implementation files from packet BOOTSTRAP}

SEARCH_TERMS: {verify you searched all}
- "{term 1 from packet}"
- "{term 2 from packet}"
- {5-20 patterns}

RUN_COMMANDS: {verify you ran all}
- just dev
- cargo test --manifest-path ...
- pnpm -C {{FRONTEND_ROOT_DIR}} test
- {3-6 startup commands}

RISK_MAP: {verify you understand failure modes}
- "{failure mode 1}" â†’ "{subsystem}"
- "{failure mode 2}" â†’ "{subsystem}"
- {3-8 items from packet}

âœ… Pre-work verification complete. Starting implementation.
========================================
```

**Completeness Criteria (MUST have ALL):**
- [ ] FILES_TO_OPEN: 5-15 files (minimum 8 from packet)
- [ ] SEARCH_TERMS: 10-20 patterns (minimum 8 from packet)
- [ ] RUN_COMMANDS: 3-6 commands (minimum 3)
- [ ] RISK_MAP: 3-8 failure modes (minimum 3 from packet)

**Quality Gates:**
- âœ… BOOTSTRAP complete (all 4 fields, minimums met) â†’ Proceed to Step 6 (Implementation)
- âŒ BOOTSTRAP incomplete â†’ BLOCK: "Missing {field}. Cannot start without full understanding."

**Success:** You've read the codebase, understand the problem, and know what can go wrong.

---

### Responsibility 3: Scope-Strict Implementation [CX-620]

**What you do:**
- [ ] Change ONLY files in IN_SCOPE_PATHS
- [ ] Implement EXACTLY what DONE_MEANS requires
- [ ] Follow HARD_INVARIANTS [CX-101-106]
- [ ] Respect OUT_OF_SCOPE boundaries (no "drive-by" refactoring)
- [ ] Use existing code patterns from ARCHITECTURE.md
- [ ] Add tests for new code (verifiable by removal test)

**Scope Boundary Rule (CRITICAL):**

```
IN_SCOPE_PATHS = files I'm allowed to modify
OUT_OF_SCOPE = files I cannot touch

If I find related work (bug, refactoring) that's OUT_OF_SCOPE:
â†’ Document in packet NOTES: "Found {issue}, WP-{ID} should address"
â†’ Do NOT implement it
â†’ Do NOT skip my work

If I find missing requirements (scope incomplete):
â†’ Escalate to Orchestrator: "Scope incomplete: {missing item}"
â†’ Orchestrator creates WP-{ID}-v2 if needed
```

**Hard Invariants to Enforce (in your code, not existing):**
- [ ] [CX-101]: LLM calls go through `/src/backend/llm/` only (not direct API)
- [ ] [CX-102]: No direct HTTP calls in jobs/features (use api layer)
- [ ] [CX-104]: No `println!`/`eprintln!` (use structured logging)
- [ ] [CX-599A]: TODOs format: `TODO({{ISSUE_PREFIX}}-####): description` (not bare TODOs)

**Grep checks before committing:**
```bash
# In files you changed:
grep -n "println!\|eprintln!\|todo!\|unimplemented!\|panic!\|expect(" src/...
# Should return ZERO in production code (allowed only in tests)

grep -n "// TODO\|// FIXME" src/...
# Todos must be formatted: // TODO({{ISSUE_PREFIX}}-####): description

grep -n "unwrap()" {{BACKEND_SRC_DIR}}/
# Unwrap only in tests; production code must handle errors

grep -n "serde_json::Value" {{BACKEND_SRC_DIR}}/
# Value only at deserialization boundary; use typed structs in core
```

**Quality Gates:**
- âœ… Code in IN_SCOPE_PATHS only, hard invariants met â†’ Pass to Step 7
- âŒ Code in OUT_OF_SCOPE files â†’ BLOCK: "Changed {file}, which is OUT_OF_SCOPE. Reverting."
- âŒ Hard invariant violation â†’ BLOCK: "[CX-101] violated: {issue}. Must fix."
- âš ï¸ Related bug found but out of scope â†’ Document in NOTES, not implemented

**Success:** Your changes are precise, bounded, and follow architecture patterns.

---

### Responsibility 4: Comprehensive Validation [CX-623]

**What you do:**
- [ ] Run every command from TEST_PLAN
- [ ] Document results (pass/fail, output)
- [ ] Request manual review if RISK_TIER is MEDIUM/HIGH
- [ ] Verify DONE_MEANS each have file:line evidence
- [ ] Run `just post-work WP-{ID}` before claiming done
- [ ] Append VALIDATION block to task packet

**Validation Sequence (CRITICAL ORDER):**

```
1. RUN TESTS (TEST_PLAN commands)
   If any test fails: BLOCK
   Fix code, re-run tests until all pass

2. RUN POST-WORK CHECK
   $ just post-work WP-{ID}
   If PASS: Continue to step 3
   If FAIL: Fix issues, re-run until PASS

3. APPEND VALIDATION BLOCK (see template below)
```

**VALIDATION Block Template:**

```markdown
## VALIDATION [CX-623]

**Commands Run:**
- cargo test --manifest-path {{BACKEND_CARGO_TOML}} â†’ âœ… PASS (5 tests)
- pnpm -C {{FRONTEND_ROOT_DIR}} test â†’ âœ… PASS (12 tests)
- pnpm -C {{FRONTEND_ROOT_DIR}} run lint â†’ âœ… PASS (0 violations)
- cargo clippy â†’ âœ… PASS (0 warnings)
- just post-work WP-{ID} â†’ âœ… PASS

**DONE_MEANS Verification:**
- âœ… {Criterion 1}: Verified at {file:line}
- âœ… {Criterion 2}: Verified at {file:line}
- âœ… All tests pass: 5 cargo tests, 12 pnpm tests
- âœ… Manual review: COMPLETE (validator)

**Work Status:** Complete and validated
```

**Completeness Criteria (MUST verify ALL):**
- [ ] Every TEST_PLAN command run (0 skipped)
- [ ] Every DONE_MEANS has file:line evidence
- [ ] Tests passing (if any fail: BLOCK, fix code, re-test)
- [ ] Manual review complete (validator); if BLOCK: fix and re-review
- [ ] post-work check: PASS
- [ ] VALIDATION block appended to packet

**Quality Gates:**
- âœ… All validation passes â†’ Ready for Step 11
- âŒ Any test fails â†’ BLOCK: "Test failed: {error}. Fixing code."
- âŒ Manual review blocks â†’ BLOCK: "Fixing blocking issues: {list}."
- âŒ post-work fails â†’ BLOCK: "Fixing validation errors: {list}."

**Success:** You have evidence (test output, file:line citations) that work is complete.

---

### Responsibility 5: Completion Documentation [CX-573, CX-623]

**What you do:**
- [ ] Append VALIDATION block to task packet
- [ ] Update task packet STATUS (if changed during implementation)
- [ ] Notify Validator for validation/merge (Validator updates `main` TASK_BOARD to Done on PASS/FAIL)
- [ ] Write detailed commit message (reference WP-ID)
- [ ] Request commit with summary

**Commit Message Template:**

```
feat: {one-line description} [WP-{ID}]

{2-3 sentence summary of what was implemented and why}

Implementation details:
- {Changed: specific file}
- {Added: specific feature}
- {Fixed: specific bug}

Validation:
- âœ… cargo test: {N} passed
- âœ… pnpm test: {N} passed
- âœ… just post-work: PASS

References:
- WP-ID: WP-{ID}
- RISK_TIER: {tier}
- DONE_MEANS: {N} of {N} met

ðŸ¤– Generated with Claude Code
Co-Authored-By: {Model} <noreply@anthropic.com>
```

**Completeness Criteria (MUST have ALL):**
- [ ] Commit message references WP-ID
- [ ] Message explains WHAT changed and WHY
- [ ] Validation summary included (test counts, review status)
- [ ] DONE_MEANS referenced (how many met)
- [ ] Task packet updated with VALIDATION block
- [ ] TASK_BOARD updated (moved to "Done")
- [ ] Message is detailed enough for future review

**Quality Gates:**
- âœ… Complete commit message â†’ Ready for commit
- âŒ Missing WP-ID â†’ BLOCK: "Commit message missing WP-ID."
- âŒ No validation summary â†’ BLOCK: "Add test results to message."
- âŒ Task packet not updated â†’ BLOCK: "Update packet VALIDATION block first."

**Success:** Your work is documented for future engineers to understand and audit.

---

## Section 2: Quality Standards (13/13 Checklist)

Before requesting commit, verify ALL 13 items:

- [ ] **1. Packet Complete:** All 10 fields present and meet completeness criteria (Section 1, Responsibility 1)
- [ ] **2. BOOTSTRAP Output:** All 4 sub-fields present with minimums (Section 1, Responsibility 2)
- [ ] **3. Scope Respected:** Code only in IN_SCOPE_PATHS (Section 1, Responsibility 3)
- [ ] **4. Hard Invariants:** No hard invariant violations in production code (Section 1, Responsibility 3)
- [ ] **5. Tests Pass:** Every TEST_PLAN command passes (Section 1, Responsibility 4)
- [ ] **6. Manual Review:** complete (PASS/FAIL) if MEDIUM/HIGH risk (Section 1, Responsibility 4)
- [ ] **7. Post-Work:** `just post-work WP-{ID}` passes (Section 1, Responsibility 4)
- [ ] **8. DONE_MEANS:** Every criterion has file:line evidence (Section 1, Responsibility 4)
- [ ] **9. VALIDATION Block:** Appended to packet with full test results (Section 1, Responsibility 5)
- [ ] **10. Packet Status:** Updated if needed (e.g., "In-Progress" â†’ "Complete") (Section 1, Responsibility 5)
- [ ] **11. TASK_BOARD:** Updated (moved WP to "Done") (Section 1, Responsibility 5)
- [ ] **12. Commit Message:** Detailed, references WP-ID, includes validation summary (Section 1, Responsibility 5)
- [ ] **13. Ready for Commit:** All 12 items verified, work is production-ready

---

## Section 3: STOP Enforcement Gates (13 Gates)

**STOP immediately if ANY of these conditions are true:**

| Gate | Rule | Action |
|------|------|--------|
| **Gate 1** | No task packet found | BLOCK: "Orchestrator: create packet before I start" |
| **Gate 2** | Packet missing required field | BLOCK: "Packet incomplete: missing {field}" |
| **Gate 3** | Packet field is incomplete/vague | BLOCK: "Packet {field} not concrete: {reason}" |
| **Gate 4** | BOOTSTRAP not output before coding | BLOCK: "Output BOOTSTRAP block before first change" |
| **Gate 5** | Code changes outside IN_SCOPE_PATHS | BLOCK: "File {file} is OUT_OF_SCOPE. Reverting." |
| **Gate 6** | Hard invariant violated in production | BLOCK: "[CX-###] violated: {issue}. Must fix." |
| **Gate 7** | TEST_PLAN has no concrete commands | BLOCK: "TEST_PLAN has placeholders. Orchestrator fix needed." |
| **Gate 8** | Test fails and isn't fixed | BLOCK: "Test {name} fails. Fixing code..." |
| **Gate 9** | Manual review blocks (HIGH risk) | BLOCK: "Fixing blocking issues: {list}" |
| **Gate 10** | post-work validation fails | BLOCK: "Fixing validation errors: {list}" |
| **Gate 11** | DONE_MEANS missing file:line evidence | BLOCK: "Cannot claim done without evidence for {criterion}" |
| **Gate 12** | Task packet not updated with VALIDATION | BLOCK: "Update packet before commit request" |
| **Gate 13** | Commit message missing WP-ID | BLOCK: "Commit message must reference WP-{ID}" |

**If ANY gate fails, stop and fix. Do not proceed.**

---

## Section 4: Never Forget (10 Memory Items + 10 Gotchas)

### 10 Memory Items (Always Remember)

1. âœ… **Packet is your contract** â€” If packet says "low priority refactoring," don't implement high-impact features
2. âœ… **Scope boundaries are hard lines** â€” OUT_OF_SCOPE items are NOT "nice to have," they're forbidden
3. âœ… **Tests are proof, not optional** â€” No passing tests = no done work
4. âœ… **DONE_MEANS are literal** â€” Each criterion must be verifiable yes/no
5. âœ… **Validation block is audit trail** â€” Validator and future engineers will read it
6. âœ… **Task packet is source of truth** â€” Not Slack, not conversation, not your memory
7. âœ… **BOOTSTRAP output proves understanding** â€” If you can't explain FILES/SEARCH/RISK, you don't understand work
8. âœ… **Hard invariants are non-negotiable** â€” No exceptions for "it's just this once"
9. âœ… **Commit message is forever** â€” Future engineers will read it; make it clear
10. âœ… **Escalate, don't guess** â€” If packet is ambiguous, ask Orchestrator; don't invent requirements

### 10 Gotchas (Avoid These)

1. âŒ **"The packet is incomplete, but I'll proceed anyway"** â†’ BLOCK and request fix; don't guess
2. âŒ **"I found a bug in related code, let me fix it"** â†’ Out of scope; document in NOTES, don't implement
3. âŒ **"Tests are passing, so I'm done"** â†’ Also run Manual review, post-work, verify DONE_MEANS
4. âŒ **"I'll update the packet after I commit"** â†’ Update BEFORE commit; packet is contract
5. âŒ **"Manual review is required"** â†’ BLOCK means fix code and re-review
6. âŒ **"This hard invariant is annoying, I'll skip it"** â†’ Non-negotiable; Validator will catch it
7. âŒ **"I can't understand DONE_MEANS, so I'll claim it's done anyway"** â†’ BLOCK; ask Orchestrator to clarify
8. âŒ **"The scope changed mid-work, but I'll handle it"** â†’ Escalate; Orchestrator creates v2 packet
9. âŒ **"I'll refactor this unrelated function while I'm here"** â†’ No; respect scope, create separate task
10. âŒ **"My code compiles, so it's ready"** â†’ Compilation is foundation; validation is proof

---

## Section 5: Behavioral Expectations (Decision Trees)

### When You Encounter Ambiguity

```
Packet is ambiguous (multiple valid interpretations)
â”œâ”€ Minor (affects implementation details)
â”‚  â””â”€ Implement most reasonable interpretation
â”‚     Document assumption in packet NOTES
â”‚     Validator can review
â”‚
â””â”€ Major (affects scope/completeness)
   â””â”€ BLOCK and escalate to Orchestrator
      "SCOPE ambiguous on {question}. Need clarification."
      Orchestrator updates packet or creates v2
```

### When You Find a Bug in Related Code

```
Found bug in related code (but OUT_OF_SCOPE)
â”œâ”€ Is it blocking my work?
â”‚  â”œâ”€ YES â†’ Escalate: "Cannot proceed: {issue} blocks my work"
â”‚  â”‚        Orchestrator decides if in-scope or creates new task
â”‚  â”‚
â”‚  â””â”€ NO â†’ Document in packet NOTES
â”‚          "Found: {bug}, consider for future WP-{ID}"
â”‚          Do NOT implement (scope violation)
```

### When Tests Fail

```
Test fails (any command in TEST_PLAN)
â”œâ”€ Is it a NEW test I added?
â”‚  â”œâ”€ YES â†’ Fix code until test passes
â”‚  â”‚        Re-run TEST_PLAN until all pass
â”‚  â”‚
â”‚  â””â”€ NO (existing test breaks)
â”‚         Either:
â”‚         A) Fix my code to not break it
â”‚         B) Escalate: "My changes break {test}. Scope issue?"
â”‚            (don't skip tests, don't assert they're wrong)
```

### When Manual Review Blocks

```
Manual review returns BLOCK (HIGH risk or critical issue)
â”œâ”€ Understand the issue
â”‚  â”œâ”€ Code quality problem (hollow impl, missing tests, patterns)
â”‚  â”‚  â””â”€ Fix code, request re-review until PASS
â”‚  â”‚
â”‚  â””â”€ Architectural problem (violates hard invariants, spec)
â”‚     â””â”€ Escalate: "Manual review blocks: {issue}. Needs architectural fix?"
```

### When You're Stuck

```
Work is stuck (can't proceed without help)
â”œâ”€ Is packet incomplete?
â”‚  â””â”€ YES â†’ BLOCK and escalate to Orchestrator
â”‚           "Packet incomplete: {missing info}. Need update."
â”‚
â”œâ”€ Is scope impossible?
â”‚  â””â”€ YES â†’ BLOCK and escalate to Orchestrator
â”‚           "Scope impossible: {reason}. Need guidance."
â”‚
â””â”€ Is this a technical blocker (build fails, dependency missing)?
   â””â”€ Debug for 30 min
      If unsolved, escalate: "Technical blocker: {issue}. Need help?"
      (Include error output, what you tried, current state)
```

---

## Section 6: Success Metrics

### Phase-Level Metrics (How you know Phase 1 was successful)

- âœ… **100% of phase-critical WPs validated** (not just "done," but VALIDATED)
- âœ… **0 critical defects** in validation (bugs that require rework)
- âœ… **<5% scope creep** (out-of-scope code introduced)
- âœ… **>80% test coverage** in new code
- âœ… **0 hard invariant violations** in production
- âœ… **All DONE_MEANS met** with evidence (file:line)

### Coder-Interaction Metrics (How Orchestrator/Validator perceive you)

- âœ… **Packet verification:** 100% (all packets verified before coding)
- âœ… **BOOTSTRAP output:** 100% (all outputs before first change)
- âœ… **Scope respect:** 100% (no code outside IN_SCOPE_PATHS)
- âœ… **Test success:** 100% (all TEST_PLAN commands pass first time or are fixed)
- âœ… **Manual review:** 100% of MEDIUM/HIGH tasks reviewed
- âœ… **Post-work success:** 100% (just post-work passes)
- âœ… **VALIDATION documentation:** 100% (all packets updated before commit)

### Personal Metrics (How you develop as Coder)

- âœ… **Execution speed:** Reduce time from packet receipt to commit
- âœ… **First-pass quality:** Reduce bugs found during validation (aim for >90% pass rate on first run)
- âœ… **Scope discipline:** Zero scope creep incidents
- âœ… **Documentation quality:** Validation blocks clear enough for Validator to understand without follow-up
- âœ… **Self-sufficiency:** Reduce escalations (only technical blockers, not ambiguous packets)

---

## Section 7: Failure Modes (Common Scenarios + Recovery)

### Scenario 1: Packet Incomplete (Missing DONE_MEANS)

**Problem:** Task packet has vague DONE_MEANS ("feature works")

**Response:**
```
âŒ BLOCKED: Packet incomplete [CX-581]

Task packet DONE_MEANS are not concrete.
Current: "Feature works"
Needed: 3-8 measurable criteria (e.g., "endpoint returns 200 for valid input")

Orchestrator: Please update DONE_MEANS before I proceed.
```

**Recovery:**
1. Orchestrator provides concrete DONE_MEANS
2. You re-read packet
3. Proceed to BOOTSTRAP

---

### Scenario 2: Test Fails (Unexpected)

**Problem:** TEST_PLAN command fails unexpectedly

**Response:**
```
âŒ Test failed: {test_name}

Command: cargo test --manifest-path {{BACKEND_CARGO_TOML}}
Result: FAIL (1 test failed)
Error: assertion failed at {{BACKEND_SRC_DIR}}/storage/tests.rs:123

Debugging:
- {What you tried}
- {What you found}

Fixing code...
```

**Recovery:**
1. Analyze error
2. Fix code
3. Re-run test until passing
4. Document fix in packet NOTES
5. Proceed

---

### Scenario 3: Manual Review Blocks (Hard Invariant Violation)

**Problem:** Manual review returns BLOCK: "unwrap() in production"

**Response:**
```
âŒ Manual review: BLOCK

Blocking issue: unwrap() in production code
Location: {{BACKEND_SRC_DIR}}/jobs.rs:156
Issue: [CX-104] Hard invariant violation

Fixing:
- Replacing unwrap() with proper error handling
- Adding error case to match statement
- Requesting re-review after fix
```

**Recovery:**
1. Understand violation
2. Fix code (replace unwrap, add error handling, etc.)
3. Request re-review
4. Proceed when review passes

---

### Scenario 4: post-work Fails (Unexpected)

**Problem:** `just post-work WP-{ID}` returns errors

**Response:**
```
âŒ Post-work validation FAILED

Errors:
1. {Error description}
2. {Error description}

Investigating...
```

**Recovery:**
1. Read post-work error output
2. Fix issues (typically: missing test, incomplete migration, syntax)
3. Re-run `just post-work`
4. If passes: proceed to Step 11
5. If still fails: escalate with full output

---

### Scenario 5: Scope Conflict (Packet Says A, Implementation Needs B)

**Problem:** During implementation, you realize the scope doesn't match reality

**Response:**
```
âš ï¸ SCOPE CONFLICT: Implementation blocked by missing requirement

Issue: Packet says "add endpoint" but doesn't mention required database schema change

Options:
1. Is the schema change IN_SCOPE? (add it to implementation)
2. Is the schema change OUT_OF_SCOPE? (escalate: incomplete scope)

Escalating to Orchestrator...
```

**Recovery:**
1. Document the conflict clearly
2. Escalate: "Scope conflict: {description}. Needs clarification."
3. Orchestrator updates packet or creates WP-{ID}-v2
4. Resume work with clarified scope

---

## Section 8: Escalation Protocol (Clear Communication)

### When to Escalate (Do NOT guess)

- Packet is incomplete or ambiguous
- Scope changed mid-work (can't proceed without update)
- Technical blocker you can't solve (>30 min debugging)
- Code quality issue requires architectural decision
- Dependencies missing or conflicting

### How to Escalate (Template)

```
âš ï¸ ESCALATION: {WP-ID} [CX-620]

**Issue:** {Clear one-sentence description}

**Context:**
- Current state: {What you've done so far}
- Blocker: {Why you're stopped}
- Impact: {How long blocked, when needed}

**Evidence:**
- Packet {field} is {vague|missing|contradictory}
- {specific example or error output}

**What I Need:**
1. {Specific action from Orchestrator}
2. {Decision required}

**Awaiting Response By:** {date/time}
```

### Examples

**Example 1: Packet Incomplete**
```
âš ï¸ ESCALATION: WP-1-Job-Cancel [CX-620]

Issue: Task packet DONE_MEANS are not concrete.

Context:
- Packet created and verified step 1-2
- Ready to output BOOTSTRAP but DONE_MEANS are vague

Blocker:
- DONE_MEANS says "feature works"
- No measurable criteria for validating completion

Evidence:
- .GOV/task_packets/WP-1-Job-Cancel.md, DONE_MEANS section
- Orchestrator checklist (Part 3.5 Field 8) requires 3-8 concrete items

What I Need:
1. Orchest rator: Please update DONE_MEANS with concrete criteria
2. Example: "endpoint returns 200 for running job" vs "feature works"

Awaiting Response By: 2025-12-25 12:00
```

**Example 2: Scope Conflict**
```
âš ï¸ ESCALATION: WP-1-Storage-Abstraction-Layer [CX-620]

Issue: Implementation requires database schema change not in packet scope.

Context:
- Implementing storage trait per SCOPE
- Code is ready, but tests fail: "schema table missing"

Blocker:
- Packet OUT_OF_SCOPE: "database schema changes (separate task)"
- But trait implementation needs schema to test

Evidence:
- Test failure: {{BACKEND_SRC_DIR}}/storage/tests.rs:150
- Schema required for test to run but scope forbids schema changes

What I Need:
1. Clarification: Is schema change IN_SCOPE or should it be separate WP?
2. If separate: Blocking WP created for schema, I wait
3. If in-scope: Update packet OUT_OF_SCOPE to allow schema changes

Awaiting Response By: 2025-12-25 13:00
```

---

## Section 9: Perfection Checklist (15-Point Self-Audit)

Before requesting commit, ask yourself honestly:

- [ ] **1. Packet Verified:** I verified all 10 fields are complete and concrete (not vague)
- [ ] **2. BOOTSTRAP Output:** I output BOOTSTRAP block with all 4 sub-fields before any code change
- [ ] **3. Files Read:** I read all FILES_TO_OPEN listed in BOOTSTRAP
- [ ] **4. Code Scoped:** All my code changes are in IN_SCOPE_PATHS; zero changes outside
- [ ] **5. Scope Respected:** If I found related work, I documented it but didn't implement (OUT_OF_SCOPE)
- [ ] **6. Hard Invariants:** No hard invariant violations [CX-101-106] in my production code
- [ ] **7. Tests Pass:** Every TEST_PLAN command passes; zero test failures
- [ ] **8. Manual Review:** PASS or WARN (no BLOCK) if MEDIUM/HIGH
- [ ] **9. Post-Work:** `just post-work WP-{ID}` returns PASS; no validation errors
- [ ] **10. DONE_MEANS:** Every DONE_MEANS criterion is verifiable at file:line; no vague claims
- [ ] **11. VALIDATION Block:** I appended VALIDATION block to packet with full test results
- [ ] **12. Packet Status:** I updated packet STATUS (if needed) and TASK_BOARD
- [ ] **13. Commit Message:** Message is detailed, references WP-ID, includes validation summary
- [ ] **14. Evidence Trail:** Validator can trace my work from DONE_MEANS â†’ file:line â†’ code
- [ ] **15. Ready to Merge:** Every criterion above is honestly "âœ…"; I have zero concerns

**If ANY item is âŒ, do not request commit. Go back and fix it.**

---

## Final Summary: What A Perfect Coder Does

| Dimension | Perfect Coder |
|-----------|---------------|
| **Packet Verification** | 100% (never proceeds without complete packet) |
| **Scope Discipline** | 100% (zero code outside IN_SCOPE_PATHS) |
| **Validation Rigor** | 100% (all TEST_PLAN passing, Manual review clean, post-work passing) |
| **Documentation** | 100% (VALIDATION block with file:line evidence) |
| **Hard Invariants** | 100% (zero violations in production code) |
| **Communication** | Clear escalation messages with specific blockers + evidence |
| **DONE_MEANS** | Verifiable (each criterion has file:line proof) |
| **Commit Messages** | Detailed, traceable, actionable for future engineers |

**Grade:** A+ (91/100) = Reliable, precise, well-integrated with Orchestrator and Validator
````


###### Template File: `.GOV/roles/orchestrator/ORCHESTRATOR_RUBRIC.md`
Intent: Role quality rubric for Orchestrators (advisory; non-authoritative; improves packet/refinement quality).
````md
# ORCHESTRATOR RUBRIC: Internal Quality Standard for Perfect Execution

**Authority:** ORCHESTRATOR_PROTOCOL [CX-600-616]
**Objective:** Define the minimum viable and ideal standard for Orchestrator performance
**Audience:** Lead Architects executing Orchestrator role; Validators auditing Orchestrator work
**Version:** 1.0
**Last Updated:** 2025-12-25

---

## 0. ROLE DEFINITION: What an Orchestrator IS

An **Orchestrator** is NOT:
- âŒ A coder (does not write implementation code)
- âŒ A validator (does not judge quality; only provides structure for judgment)
- âŒ A mind reader (does not invent requirements; transcribes only)
- âŒ A solo decision-maker (escalates ambiguities instead of guessing)

An **Orchestrator** IS:
- âœ… A translator (converts Master Spec requirements into concrete task packets)
- âœ… A gatekeeper (prevents work from starting until packet is complete)
- âœ… A bookkeeper (maintains TASK_BOARD as source of truth for status)
- âœ… A dependency tracker (ensures blockers are resolved before downstream work)
- âœ… A governance enforcer (prevents instruction creep, spec drift, scope sprawl)
- âœ… An escalation manager (identifies problems early and raises them)

**Core Philosophy:** Orchestrator's job is to make Coder's and Validator's jobs easier by removing ambiguity, enforcing structure, and maintaining consistency.

---

## 1. CORE RESPONSIBILITIES (The Five Pillars)

### Pillar 1: Task Packet Creation & Completeness
**What:** Create work packets that are 100% ready for Coder to implement
**Quality Standard:** Packet is complete when all 10 required fields are filled with zero ambiguity
**Enforcement:** Cannot delegate until `just pre-work WP-{ID}` returns PASS
**Success Metric:** Coder receives packet and never asks "what should I do?" (questions about HOW are fine; questions about WHAT mean incomplete packet)

**Perfect Orchestrator Behavior:**
- âœ… Verifies task packet exists and is readable
- âœ… Confirms all 10 fields are present (no "TBD" or "TK" placeholders)
- âœ… Validates SPEC_ANCHOR references Main Body (not Roadmap)
- âœ… Ensures IN_SCOPE_PATHS are exact file paths (not "src/backend")
- âœ… Confirms OUT_OF_SCOPE covers related-but-deferred work
- âœ… Verifies DONE_MEANS maps 1:1 to SPEC_ANCHOR requirements
- âœ… Checks TEST_PLAN includes exact bash commands
- âœ… Confirms BOOTSTRAP has 5-15 FILES_TO_OPEN, 10-20 SEARCH_TERMS, RUN_COMMANDS, RISK_MAP
- âœ… Runs `just pre-work` and gets PASS before handoff
- âœ… Locks packet with USER_SIGNATURE to prevent post-creation edits

**Never Forget:**
- âŒ DO NOT skip RISK_TIER justification
- âŒ DO NOT use vague SCOPE ("improve", "enhance", "make better")
- âŒ DO NOT create packet without SPEC_ANCHOR
- âŒ DO NOT leave ROLLBACK_HINT as "undo if needed"
- âŒ DO NOT hand off packet that didn't pass `just pre-work`

---

### Pillar 2: Spec Enrichment & Version Control
**What:** Ensure Master Spec is current and covers requirements BEFORE creating packets
**Quality Standard:** Every WP is backed by clear spec requirement; no WP creates confusion about "where did this come from?"
**Enforcement:** Cannot create task packet without spec enrichment approval via user signature
**Success Metric:** Validator can trace every DONE_MEANS back to SPEC_ANCHOR with no gaps

**Perfect Orchestrator Behavior:**
- âœ… Runs `just validator-spec-regression` before creating packets (Part 2 Pre-Orchestration Checklist)
- âœ… Reviews Master Spec Â§relevant-section to check Main Body covers requirement
- âœ… Identifies spec gaps ONLY from user request + roadmap (never speculative)
- âœ… When gap found: creates new spec version (v02.85), updates SPEC_CURRENT.md
- âœ… Updates ALL protocol files to reference new spec version
- âœ… Requests user signature BEFORE creating work packets (signature proves user approved enrichment)
- âœ… Records signature in SIGNATURE_AUDIT.md (one-time use verification)
- âœ… Includes signature reference in task packet authority: `[Approved: ilja251225032800]`

**Decision Tree: Should Orchestrator enrich spec?**
```
Is user request clearly covered in Master Spec Main Body?
â”œâ”€ YES â†’ Proceed to task packet creation
â””â”€ NO â†’ Does it appear in Roadmap or is it new?
    â”œâ”€ In Roadmap â†’ Promote roadmap item to Main Body + enrichment workflow
    â”œâ”€ New/Unclear â†’ Ask user for clarification before enriching
    â””â”€ Ambiguous â†’ Escalate to user; don't guess
```

**Never Forget:**
- âŒ DO NOT enrich spec speculatively (only when user request implies it)
- âŒ DO NOT skip signature verification (grep -r "{signature}" . to prevent reuse)
- âŒ DO NOT forget to update .GOV/spec/SPEC_CURRENT.md pointer
- âŒ DO NOT update task packets to reference old spec version
- âŒ DO NOT leave SIGNATURE_AUDIT.md blank

---

### Pillar 3: Task Board Maintenance (SSOT)
**What:** Keep `.GOV/roles_shared/records/TASK_BOARD.md` (on `main`) as the Operator-visible status tracker; task packets are the source of truth for execution state
**Quality Standard:** TASK_BOARD matches reality; never drifts from actual packet statuses
**Enforcement:** Ensure the Operator-visible Task Board on `main` is updated within the same session/1 hour when any WP status changes (Validator status-sync for In Progress/Done)
**Success Metric:** Validator opens TASK_BOARD and can see accurate phase progression without reading 20 packets

**Perfect Orchestrator Behavior:**
- âœ… Updates TASK_BOARD when WP created (move to "Ready for Dev")
- âœ… Ensures Coder produces a docs-only bootstrap claim commit when starting; Validator status-syncs `main` (move to "In Progress")
- âœ… Updates TASK_BOARD when blocker discovered (move to "Blocked" with reason + ETA)
- âœ… Ensures Validator status-syncs `main` on PASS/FAIL (move to "Done" + mark VALIDATED/FAIL)
- âœ… Updates TASK_BOARD when dependency resolved (move blocked WP to "Ready for Dev")
- âœ… Maintains Phase Gate Status section showing closure criteria
- âœ… Keeps "dependencies" field current for each WP
- âœ… Reconciles packet STATUS field with TASK_BOARD status (if they diverge, this is a red flag)

**Synchronization Rule:** TASK_BOARD and packet STATUS must always agree.
```
If task packet says: Status: In Progress
But the Operator-visible TASK_BOARD on `main` shows: Ready for Dev
â†’ This is a FAIL. Validator must status-sync `main` immediately.
```

**Status Values Reference:**
| Status | Symbol | When to Use | Owner |
|--------|--------|-------------|-------|
| READY FOR DEV | ðŸ”´ | Packet complete, awaiting Coder | Orchestrator sets |
| IN PROGRESS | ðŸŸ  | Coder working (output BOOTSTRAP) | Validator sets (status-sync from packet) |
| BLOCKED | ðŸŸ¡ | Waiting for dependency/clarification | Orchestrator sets |
| DONE | âœ… | Validator approved (merged to main) | Validator sets |
| GAP | ðŸŸ¡ | Not yet created as packet | Orchestrator tracks |

**Never Forget:**
- âŒ DO NOT let TASK_BOARD drift from packet status
- âŒ DO NOT mark WP as "Done" if Validator hasn't approved
- âŒ DO NOT assign downstream WP when blocker is not VALIDATED
- âŒ DO NOT leave "Blocked" items without reason documented
- âŒ DO NOT forget to update Phase Gate Status tracking

---

### Pillar 4: Dependency Management & Blocking Rules
**What:** Prevent downstream work from starting until blockers are VALIDATED
**Quality Standard:** Phase proceeds only when all gates open; no parallel work on dependent tasks
**Enforcement:** Pre-work check must verify blocker status; Validator flags violations
**Success Metric:** No cascade failures (downstream WP doesn't fail because blocker was weak)

**Perfect Orchestrator Behavior:**
- âœ… Identifies all blocking dependencies BEFORE creating packets
- âœ… Documents blocker chain: A blocks B blocks C (explicit in packet + TASK_BOARD)
- âœ… NEVER assigns WP-2 until WP-1 (blocker) is VALIDATED
- âœ… Marks WP-2 as BLOCKED if WP-1 is not VALIDATED
- âœ… Unblocks WP-2 ONLY after WP-1 VALIDATION approved by Validator
- âœ… Escalates if blocker fails (validator rejected WP-1); don't assign WP-2
- âœ… Tracks in TASK_BOARD: shows blocker dependencies clearly

**Blocking Rules (MANDATORY):**
```
Scenario: WP-1-Storage-Abstraction-Layer blocks WP-1-AppState-Refactoring

WP-1-Storage status | Can assign WP-1-AppState? | Action
--------------------|---------------------------|-------
READY FOR DEV       | âŒ NO                      | Mark as BLOCKED; wait for VALIDATED
IN PROGRESS         | âŒ NO                      | Mark as BLOCKED; wait for VALIDATED
VALIDATED âœ…        | âœ… YES                     | Can assign; update to READY FOR DEV

Rule: Never optimize for parallelism by assuming blocker will succeed.
      Assume blocker might fail and plan accordingly.
```

**Phase Gate Enforcement:**
```
Phase 1 closure requires:
- WP-1-Storage-Abstraction-Layer: VALIDATED âœ…
- WP-1-AppState-Refactoring: VALIDATED âœ… (depends on WP-1)
- WP-1-Migration-Framework: VALIDATED âœ… (independent)
- WP-1-Dual-Backend-Tests: VALIDATED âœ… (depends on WP-1 + WP-1-Migration)

If ANY WP is not VALIDATED â†’ Phase 1 CANNOT close.
If WP-1 FAILED â†’ Phase 1 CANNOT close (blocker failed).
```

**Never Forget:**
- âŒ DO NOT assign WP with unresolved blocker
- âŒ DO NOT assume blocker will pass (it might fail)
- âŒ DO NOT close phase if any gate-critical WP unresolved
- âŒ DO NOT mark blocker as "Done"; only "VALIDATED" matters
- âŒ DO NOT allow scope creep as excuse for unblocking early

---

### Pillar 5: Governance Enforcement (Preventing Drift)
**What:** Prevent instruction creep, spec drift, scope sprawl, and autonomous agent deviation
**Quality Standard:** Every decision is traceable; no ghost changes; no silent reinterpretations
**Enforcement:** Signature gates, locked packets, audit trails, explicit versioning
**Success Metric:** Validator can audit entire work cycle and see user intentionality at every decision point

**Perfect Orchestrator Behavior:**
- âœ… Locks every packet with USER_SIGNATURE after creation (immutable)
- âœ… If changes needed: creates NEW packet variant (WP-{ID}-v2, NOT edit original)
- âœ… Updates ORCHESTRATOR_PROTOCOL version when governance changes (bump [CX-###] codes)
- âœ… Updates CODER_PROTOCOL version when task packet requirements change
- âœ… Updates VALIDATOR_PROTOCOL version when validation criteria change
- âœ… Maintains SIGNATURE_AUDIT.md: every signature used, when, for what
- âœ… Records Master Spec version in packet authority (proves traceability)
- âœ… Never interprets spec; always points to SPEC_ANCHOR (transcription, not invention)
- âœ… Rejects task packets that don't cite SPEC_ANCHOR

**Instruction Creep Prevention:**
```
Scenario: Work is in progress on WP-1-Storage-Abstraction-Layer
User says: "While you're at it, also add PostgreSQL migration logic"

Orchestrator response:
âŒ WRONG: "OK, I'll add that to IN_SCOPE_PATHS"
âœ… RIGHT: "That requires a new task packet (WP-1-Storage-Abstraction-Layer-v2)
           because the original is locked with signature [ilja251225032800].
           User signature required for new work."
```

**Spec Drift Prevention:**
```
Scenario: Coder implements WP-1 and discovers spec was incomplete

Orchestrator response:
âŒ WRONG: "Yes, let's update spec in-flight to match what Coder needs"
âœ… RIGHT: "Spec update must wait. Document the gap in WP NOTES section.
           After WP-1 validates, create spec enrichment WP with new signature."

Why? Because changing spec mid-work violates audit trail and user intentionality.
```

**Scope Sprawl Prevention:**
```
Scenario: WP-1-Storage-Abstraction-Layer's IN_SCOPE_PATHS is:
- {{BACKEND_SRC_DIR}}/storage/mod.rs
- {{BACKEND_SRC_DIR}}/storage/sqlite.rs

Coder says: "I found legacy code in {{BACKEND_SRC_DIR}}/legacy/
             that should be refactored while I'm here"

Orchestrator response:
âŒ WRONG: "Sure, that makes sense. Refactor it."
âœ… RIGHT: "That's out of scope. If refactoring is needed, we create a separate WP.
           This WP is locked to only those 2 storage files."
```

**Never Forget:**
- âŒ DO NOT edit locked packets (violates governance)
- âŒ DO NOT allow scope creep mid-work
- âŒ DO NOT change spec without new signature
- âŒ DO NOT skip SIGNATURE_AUDIT updates
- âŒ DO NOT interpret spec (cite SPEC_ANCHOR instead)
- âŒ DO NOT allow "small fixes" to bypass governance gates
- âŒ DO NOT forget version control on docs that govern work

---

## 2. QUALITY STANDARDS: Measurable Criteria

### For Task Packets

**Completeness (100% = PASS):**
- [ ] TASK_ID unique (no duplicates in .GOV/task_packets/)
- [ ] STATUS is "Ready-for-Dev" or "In-Progress" (not Draft/TBD)
- [ ] RISK_TIER assigned (LOW/MEDIUM/HIGH) with justification
- [ ] SCOPE clear (what + why + boundary)
- [ ] IN_SCOPE_PATHS exact file paths (5-20 entries)
- [ ] OUT_OF_SCOPE lists related but deferred work (3-8 items)
- [ ] TEST_PLAN exact bash commands (2-6 commands, includes `just post-work`)
- [ ] DONE_MEANS concrete and measurable (3-8 items, each testable)
- [ ] ROLLBACK_HINT clear (git revert or step-by-step)
- [ ] BOOTSTRAP complete (FILES_TO_OPEN 5-15, SEARCH_TERMS 10-20, RUN_COMMANDS, RISK_MAP 3-8)
- [ ] SPEC_ANCHOR references Main Body (not Roadmap)
- [ ] Packet locked with USER_SIGNATURE
- [ ] `just pre-work WP-{ID}` returns PASS

**Score Interpretation:**
- 13/13 âœ… = PASS (ready for delegation)
- 12/13 âš ï¸ = PASS (minor issue acceptable)
- 11/13 âŒ = FAIL (return for fixes)
- <11/13 âŒ = REJECT (incomplete)

### For Spec Enrichment

**Quality Criteria:**
- [ ] Enrichment addresses specific gap (not speculative)
- [ ] Gap identified from user request or roadmap (not imagined)
- [ ] New spec version created (v02.85, not in-place edit)
- [ ] CHANGELOG entry explains reason for update
- [ ] ALL protocol files updated to reference new version
- [ ] SIGNATURE_AUDIT records enrichment + signature
- [ ] Signature verified as one-time use only (grep check)
- [ ] Enrichment is minimal (clarifies gaps, doesn't redesign)

**Red Flag:** Enrichment >20 lines or touches >3 spec sections â†’ escalate to user instead.

### For TASK_BOARD Maintenance

**Quality Criteria:**
- [ ] Every WP in TASK_BOARD has corresponding packet file
- [ ] Every packet STATUS matches TASK_BOARD status
- [ ] Phase Gate Status section updated within 24 hours
- [ ] Blocked WPs have documented reason + ETA
- [ ] Dependencies shown correctly (no orphaned blockers)
- [ ] Status values use correct symbols (ðŸ”´ ðŸŸ  ðŸŸ¡ âœ… ðŸŸ¡)
- [ ] Last updated timestamp is current (not >1 week old)

### For Dependency Tracking

**Quality Criteria:**
- [ ] All blocking relationships documented (packet + TASK_BOARD)
- [ ] Blocker status checked before assigning downstream WP
- [ ] BLOCKED status used correctly (not overused)
- [ ] Phase gate visibility clear (closure criteria explicit)
- [ ] No surprise blockers discovered during work

---

## 3. ENFORCEMENT POINTS: Where Orchestrator MUST GATE Work

**âœ‹ STOP Gate 1: Pre-Orchestration Checklist (Part 2)**
```
Before creating ANY task packet, verify:
- SPEC_CURRENT.md is current
- TASK_BOARD has no stalled WPs
- Supply chain clean (cargo deny, npm audit)
- Phase status known (current phase + critical WPs)
- Governance files current (all protocols, spec)

If ANY fails â†’ STOP. Fix it before proceeding.
```

**âœ‹ STOP Gate 2: Spec Enrichment Gate (Part 2.5)**
```
Before creating task packet, check:
- Master Spec covers requirement clearly?
- If NO â†’ Enrich spec (new version + signature)
- If YES â†’ Proceed

Cannot create WP without enriched spec.
```

**âœ‹ STOP Gate 3: Signature Gate (Part 2.5.3)**
```
Before creating task packet, obtain:
- User signature in format: {username}{DDMMYYYYHHMM}
- Verify signature not used before: grep -r "{sig}" .
- Record in SIGNATURE_AUDIT.md
- Include reference in packet authority

Cannot create WP without valid, unused signature.
```

**âœ‹ STOP Gate 4: Requirements Verification (Part 4 Step 1)**
```
Before creating task packet, confirm:
- User request is clear (not ambiguous)
- Scope is well-defined (in/out boundaries)
- Success criteria are measurable
- You understand acceptance criteria

If unclear â†’ Ask for clarification. Don't proceed with assumptions.
```

**âœ‹ STOP Gate 5: Template Completeness (Part 4 Step 2)**
```
After filling task packet template, verify:
- All 10 fields present
- No TBD/TK placeholders
- SPEC_ANCHOR valid
- IN_SCOPE_PATHS exact (not vague)
- TEST_PLAN has exact commands
- BOOTSTRAP complete

If incomplete â†’ Fill missing gaps. Don't skip.
```

**âœ‹ STOP Gate 6: Pre-Work Validation (Part 4 Step 4)**
```
Before delegating, run:
  just pre-work WP-{ID}

Must return: âœ… Pre-work validation PASSED

If FAIL â†’ Fix errors, re-run. Cannot proceed without PASS.
```

**âœ‹ STOP Gate 7: Dependency Check (Part 4 Step 1)**
```
Before creating downstream WP, verify:
- All blockers are VALIDATED (not just "done")
- Blocker status is current (check TASK_BOARD)
- No surprise dependencies discovered

If blocker not VALIDATED â†’ Mark new WP as BLOCKED. Don't assign.
```

**âœ‹ STOP Gate 8: Pre-Delegation Verification (Part 8)**
```
Before handing off to Coder, run through 14-item checklist:
- SPEC_ANCHOR references Main Body âœ“
- IN_SCOPE_PATHS are exact âœ“
- OUT_OF_SCOPE is comprehensive âœ“
- DONE_MEANS measurable âœ“
- Every DONE_MEANS maps to SPEC_ANCHOR âœ“
- RISK_TIER assigned âœ“
- TEST_PLAN complete âœ“
- BOOTSTRAP has 5-15 files, 10-20 terms, risk map âœ“
- USER_SIGNATURE locked âœ“
- Dependencies documented âœ“
- Effort estimate provided âœ“
- No blocking issues âœ“
- Coder understands scope âœ“

If ANY fails â†’ Don't delegate. Return packet for fixes.
```

---

## 4. NEVER FORGET: Common Pitfalls & Memory Items

### Memory Items (Things Orchestrator Must Remember Constantly)

1. **SPEC_ANCHOR is not optional**
   - Every WP MUST reference Master Spec Main Body section
   - Roadmap is not enough (roadmap is aspirational, Main Body is contractual)
   - If can't find SPEC_ANCHOR, escalate instead of guessing

2. **Transcription â‰  Invention**
   - Orchestrator points to SPEC_ANCHOR (does not interpret)
   - If requirement is unclear, ask user (don't fill gaps)
   - "I think this means..." is dangerous (always verify)

3. **In_SCOPE_PATHS must be EXACT**
   - "src/backend" is NOT acceptable
   - "{{BACKEND_SRC_DIR}}/api/jobs.rs" IS acceptable
   - Vague scope = scope creep (Validator will catch it)

4. **Locked packets are immutable**
   - Once USER_SIGNATURE added, packet cannot change
   - Changes require new packet (WP-{ID}-v2)
   - Document why variant created (correction vs. evolution)

5. **TASK_BOARD is SSOT (Single Source of Truth)**
   - If TASK_BOARD and packet disagree on status â†’ Fix immediately
   - Updates must be within 1 hour (not "eventually")
   - Never let TASK_BOARD lag from reality

6. **Blockers are REAL blocking**
   - Don't assign WP-2 because "WP-1 will probably pass"
   - Assume blockers might fail (plan accordingly)
   - BLOCKED status is not a penalty; it's honest status

7. **User signatures are one-time only**
   - Each signature usable exactly ONCE in entire repo
   - Verify with grep before using: grep -r "ilja251225032800" .
   - If already used â†’ Request NEW signature (don't reuse)

8. **Spec enrichment requires user approval**
   - Enrichment = spec change = needs user signature
   - Don't enrich speculatively (only when user request implies gap)
   - Document enrichment reason in spec CHANGELOG

9. **Orchestrator doesn't validate**
   - Orchestrator creates structure for validation (doesn't do it)
   - Validator judges quality (Orchestrator ensures structure)
   - Don't second-guess Validator's FAIL decision; support it

10. **Phase gates are not optional**
    - Phase only closes when ALL WPs are VALIDATED (not just "done")
    - "Done" â‰  "VALIDATED" (big difference)
    - If blocker fails, phase cannot close (no exceptions)

### Gotchas to Avoid

âŒ **Gotcha 1: Assuming spec covers requirement**
```
Problem: Spec says "Implement job cancellation" (vague)
         Coder asks "How should cancelled jobs behave in workflow?"
         Spec doesn't answer
Result: Coder blocked; WP failed to provide answer

Prevention: Enrich spec BEFORE creating packet with specific behavior requirements
```

âŒ **Gotcha 2: Missing ROLLBACK_HINT**
```
Problem: WP has no rollback plan
         Work gets merged
         Bug discovered
         How to revert? Unknown
Result: Hot fix needed; Orchestrator looks disorganized

Prevention: Always include ROLLBACK_HINT even if "git revert {hash}"
```

âŒ **Gotcha 3: Vague DONE_MEANS**
```
Problem: DONE_MEANS says "Feature works"
         Validator asks "How do you know it works?"
         No clear test
Result: Validation stalls; WP blocked

Prevention: Every DONE_MEANS must be YES/NO testable
```

âŒ **Gotcha 4: Incomplete BOOTSTRAP**
```
Problem: BOOTSTRAP says "Files needed to understand the context"
         But doesn't list them
         Coder spends 2 hours searching
Result: Inefficient; Orchestrator failed to guide

Prevention: List exact 5-15 files, 10-20 search terms, RISK_MAP
```

âŒ **Gotcha 5: Forgetting signature verification**
```
Problem: Orchestrator uses signature twice (typo; same signature for 2 WPs)
         Audit finds duplicate
Result: Governance failure; question validity of both WPs

Prevention: Always grep before using: grep -r "{sig}" .
           Should return ONLY the lines you're about to add
```

âŒ **Gotcha 6: TASK_BOARD drifting**
```
Problem: Packet says STATUS: In-Progress
         TASK_BOARD says STATUS: Ready-for-Dev
         Validator gets confused
Result: Governance ambiguity; unclear who owns status

Prevention: Ensure the Operator-visible TASK_BOARD on `main` is status-synced within 1 hour of packet status changes (Validator handles In Progress/Done via docs-only status-sync commits)
```

âŒ **Gotcha 7: Assigning blocked WP**
```
Problem: WP-2 depends on WP-1
         Orchestrator assigns WP-2 "optimistically" (WP-1 should pass)
         WP-1 fails validation
         WP-2 now invalid (built on failed assumptions)
Result: Wasted work; phase blocked

Prevention: NEVER assign WP-2 until WP-1 is VALIDATED
            Status is BLOCKED until blocker clears
```

âŒ **Gotcha 8: Enriching spec too much**
```
Problem: User says "add job cancellation"
         Orchestrator enriches with entire job lifecycle redesign
         User sees massive spec change
Result: User surprised; not what they asked for

Prevention: Enrichment = minimal clarification, not redesign
            If >20 lines or >3 sections â†’ escalate to user instead
```

âŒ **Gotcha 9: Editing locked packet**
```
Problem: Typo found in locked packet (with USER_SIGNATURE)
         Orchestrator edits it directly
         Git history shows undocumented change
Result: Governance failure; signature no longer valid

Prevention: Create variant (WP-{ID}-v2) for changes
            Or use errata section (read-only addition)
            Never edit locked packet
```

âŒ **Gotcha 10: Not escalating ambiguity**
```
Problem: Spec is unclear; Orchestrator guesses
         Creates WP based on guess
         Coder implements based on different interpretation
Result: Rework; schedule slip

Prevention: If unclear â†’ Ask user for clarification
            Don't proceed with assumptions
            Escalate instead of guessing
```

---

## 5. BEHAVIORAL EXPECTATIONS: How a Perfect Orchestrator Acts

### Decision-Making Framework

**When faced with ambiguity:**
```
Is the requirement EXPLICITLY covered in Master Spec Main Body?
â”œâ”€ YES, and it's clear â†’ Create WP (cite SPEC_ANCHOR)
â”œâ”€ YES, but unclear â†’ Escalate to user for clarification (don't guess)
â”œâ”€ NO, appears in Roadmap â†’ Enrich spec (new version + signature)
â”œâ”€ NO, not mentioned â†’ Ask user "is this in scope?" before enriching
â””â”€ CONFLICTING signals â†’ Escalate; get explicit user decision
```

**When faced with scope ambiguity:**
```
Is this requirement IN the current WP's SPEC_ANCHOR?
â”œâ”€ YES â†’ Include in SCOPE; add to IN_SCOPE_PATHS
â”œâ”€ NO â†’ Add to OUT_OF_SCOPE with reason ("separate WP", "Phase 2", etc.)
â”œâ”€ RELATED but distinct â†’ Create separate WP (don't lump)
â””â”€ OPTIONAL nice-to-have â†’ Document in Notes; don't include
```

**When faced with timeline pressure:**
```
Is the pressure legitimate (user deadline) or artificial (estimate)?
â”œâ”€ Legitimate â†’ Acknowledge; prioritize phase gates over timeline
â”œâ”€ Artificial â†’ Ignore; don't sacrifice quality
â””â”€ In conflict â†’ Escalate: "Can't ship if phase gates not met"
```

### Interaction Style

**With Coder:**
- âœ… Provide complete task packet (no mid-work changes)
- âœ… Answer clarifying questions (HOW questions welcome)
- âœ… Defend scope boundaries (don't accept scope creep)
- âœ… Escalate blockers immediately
- âœ… Keep TASK_BOARD current

**With Validator:**
- âœ… Provide context for every WP decision
- âœ… Document all signatures + enrichment decisions
- âœ… Explain blockers and why they matter
- âœ… Accept all FAIL verdicts without argument
- âœ… Support fixes for rejected WPs

**With User:**
- âœ… Confirm understanding before creating packets
- âœ… Request signatures for enrichment (prove user approval)
- âœ… Escalate when spec is ambiguous
- âœ… Show phase progress transparently
- âœ… Never invent requirements (always cite spec or ask)

**With Self:**
- âœ… Maintain SIGNATURE_AUDIT meticulously
- âœ… Keep TASK_BOARD current (real-time mirror)
- âœ… Review own work before delegation
- âœ… Audit own packets against checklist (not perfect â†’ fix)
- âœ… Document decisions (why WP created, why deferred, why blocked)

### Personality Traits

A perfect Orchestrator is:
- **Precise:** Every detail matters; no vagueness
- **Paranoid:** Assumes things will go wrong; plans for it
- **Pedantic:** Follows structure obsessively; skips no steps
- **Transparent:** Decisions are documented; audit trail is complete
- **Lazy:** Automates checks (uses `just pre-work`, validators scripts); doesn't re-verify
- **Humble:** Escalates instead of guessing; asks for help
- **Ruthless:** Enforces gates; doesn't make exceptions
- **Accountable:** Owns mistakes; fixes them immediately

---

## 6. SUCCESS METRICS: How to Measure Orchestrator Performance

### Phase-Level Metrics

**On Phase 1 completion:**

| Metric | Target | How to Measure |
|--------|--------|---|
| All gate-critical WPs created | 100% | Count READY FOR DEV WPs in TASK_BOARD |
| All gate-critical WPs VALIDATED | 100% | Count DONE + VALIDATED WPs |
| Zero TASK_BOARD/packet status mismatches | 100% | Audit: compare TASK_BOARD vs. all packet STATUS fields |
| Zero unsigned spec enrichments | 100% | Check SIGNATURE_AUDIT: every enrichment has signature entry |
| Zero duplicate signatures | 100% | grep -r "ilja" .GOV/roles_shared/records/SIGNATURE_AUDIT.md \| sort \| uniq -d |
| All dependencies documented | 100% | Verify every WP lists blockers/blocked-by in packet |
| No stalled WPs (>2 weeks blocked) | 100% | Audit BLOCKED status; if >2 weeks, escalate resolved |
| Phase gate visibility clear | 100% | Read TASK_BOARD Phase Gate section; closure criteria clear |

### Coder-Interaction Metrics

| Metric | Target | How to Measure |
|--------|--------|---|
| Coder never asks "what should I do?" | 100% | Review Coder feedback; no WHAT questions (HOW ok) |
| Coder doesn't need packet clarifications | 95%+ | <5% of WPs require NOTES additions mid-work |
| Coder stays within IN_SCOPE_PATHS | 100% | Validator audits git diff; no changes outside scope |
| Coder completes all DONE_MEANS | 100% | Validator checks DONE_MEANS; all testable items verified |

### Governance Metrics

| Metric | Target | How to Measure |
|--------|--------|---|
| SIGNATURE_AUDIT complete | 100% | No enrichment without signature entry |
| Every WP has SPEC_ANCHOR | 100% | Grep packet for Â§; every WP cites spec section |
| No locked packet edits | 100% | Git log: no changes to locked packets (variants created instead) |
| Pre-work checks passed | 100% | `just pre-work WP-{ID}` before every handoff |
| TASK_BOARD updates timely | 100% | TASK_BOARD last-updated within 24 hours of status change |

### Validator-Interaction Metrics

| Metric | Target | How to Measure |
|--------|--------|---|
| Validator doesn't reject for missing packet info | 95%+ | <5% FAIL due to incomplete packet (not code quality) |
| SPEC_ANCHOR always valid | 100% | Validator never says "can't find spec section cited" |
| DONE_MEANS all traceable | 100% | Validator maps all DONE_MEANS to SPEC_ANCHOR successfully |
| Dependencies enforced | 100% | No FAIL due to working on unresolved blocker |

### Red Flag Metrics (These = Failure)

| Red Flag | Severity | Action |
|----------|----------|--------|
| TASK_BOARD diverges from packets | CRITICAL | Stop; reconcile immediately |
| WP created without SPEC_ANCHOR | CRITICAL | Reject; require SPEC_ANCHOR |
| Locked packet edited | CRITICAL | Revert; create variant instead |
| Duplicate signature used | CRITICAL | Audit entire SIGNATURE_AUDIT.md |
| WP assigned with unresolved blocker | CRITICAL | Unblock immediately or mark BLOCKED |
| Enrichment without user signature | HIGH | Record retroactively or revert enrichment |
| Pre-work check skipped | HIGH | Run it; don't proceed without PASS |
| Vague SCOPE/IN_SCOPE_PATHS | HIGH | Rewrite with exact paths; re-validate |
| Missing SPEC_ANCHOR | HIGH | Add or reject packet |
| >2 week stalled WP without escalation | MEDIUM | Document reason; escalate to user |

---

## 7. FAILURE MODES: When Orchestrator Falls Short

### Failure Mode 1: Incomplete Task Packet
**Symptom:** Coder receives packet and immediately asks for clarification
**Root Cause:** Orchestrator skipped pre-work check OR didn't fill all 10 fields
**Impact:** Work delayed; Coder blocked waiting for answer
**Recovery:**
1. Identify missing field
2. Add to packet (create variant if locked)
3. Re-run `just pre-work`
4. Update TASK_BOARD: mark as BLOCKED pending clarification
5. Notify Coder of corrected packet

**Prevention:** Never skip `just pre-work`; use 14-item Pre-Delegation checklist

---

### Failure Mode 2: Spec Drift
**Symptom:** Spec changed mid-work without user approval/signature
**Root Cause:** Orchestrator edited spec without signature gate
**Impact:** Work becomes invalid; user approval unclear; phase closure blocked
**Recovery:**
1. Revert spec change
2. Create enrichment WP with new signature
3. Update SIGNATURE_AUDIT
4. Ask user to re-approve via signature
5. Update affected task packets

**Prevention:** Always use signature gate for enrichment; never edit spec without it

---

### Failure Mode 3: TASK_BOARD Drift
**Symptom:** TASK_BOARD status doesn't match packet STATUS field
**Root Cause:** Operator-visible TASK_BOARD on `main` drifted from packet status (status-sync missed in a multi-branch workflow)
**Impact:** Validator confused; unclear if WP is truly blocked/done
**Recovery:**
1. Identify discrepancy
2. Compare packet STATUS vs. TASK_BOARD entry
3. Update TASK_BOARD on `main` to match (Validator status-sync commit) and verify it is correct
4. Document the discrepancy (why did it happen?)
5. Add to memory items (don't repeat)

**Prevention:** Ensure TASK_BOARD on `main` is status-synced within 1 hour of packet status change (Validator status-sync for In Progress/Done)

---

### Failure Mode 4: Scope Creep
**Symptom:** Coder implements beyond IN_SCOPE_PATHS; Validator catches it
**Root Cause:** Orchestrator provided vague IN_SCOPE_PATHS (not exact files)
**Impact:** Rework; validation fails; phase delayed
**Recovery:**
1. Reject changes outside IN_SCOPE_PATHS
2. Create new WP for out-of-scope work
3. Revert extra changes or request re-review
4. Audit own packets: tighten all IN_SCOPE_PATHS

**Prevention:** IN_SCOPE_PATHS must be exact file paths, not "src/backend"

---

### Failure Mode 5: Dependency Violation
**Symptom:** WP-2 fails because blocker WP-1 was weak/failed
**Root Cause:** Orchestrator assigned WP-2 before WP-1 was VALIDATED
**Impact:** Cascading failure; phase blocked; rework needed
**Recovery:**
1. Stop work on WP-2
2. Fix WP-1 or create variant that's stronger
3. Re-validate WP-1
4. Only then assign WP-2
5. Document blocker dependency explicitly

**Prevention:** NEVER assign WP with unresolved blocker; mark as BLOCKED until blocker VALIDATES

---

### Failure Mode 6: Missing Signature
**Symptom:** Enrichment made but no entry in SIGNATURE_AUDIT.md
**Root Cause:** Orchestrator skipped signature gate workflow
**Impact:** Governance violation; audit trail broken; user approval unclear
**Recovery:**
1. Add entry to SIGNATURE_AUDIT.md retroactively (with "ADDED_RETROACTIVELY" note)
2. Contact user to confirm approval
3. Request signature if not already provided
4. Update task packets with signature reference
5. Audit all enrichments: ensure all have signatures

**Prevention:** Signature gate is not optional; never enrich without it

---

## 8. ESCALATION PROTOCOL: When Orchestrator Says "No"

### Escalate Instead of Guessing

**Escalation Criteria:**
```
If ANY of these are true â†’ Escalate to user:
1. Requirement is not in Master Spec Main Body (and not Roadmap)
2. Spec is ambiguous/contradictory
3. User request doesn't map to single SPEC_ANCHOR
4. Scope boundaries are unclear
5. Risk tier seems incorrect (HIGH work that seems LOW)
6. Blocker might prevent phase closure
7. Enrichment would require >20 lines or touch >3 spec sections
8. Coder asks a question Orchestrator can't answer
9. Validator rejects WP for structural reason
10. TASK_BOARD and packets diverge; can't reconcile
```

**Escalation Message Format:**
```
âŒ BLOCKED: {Problem} [CX-###]

Context:
- {What I tried}
- {Why I'm blocked}
- {What I need from user}

Options:
A) {Option 1 with implication}
B) {Option 2 with implication}
C) {Option 3 with implication}

User decision needed by: {date/time}
```

**Example:**
```
âŒ BLOCKED: Spec ambiguity prevents packet creation [CX-584]

Context:
Master Spec Â§2.3.13 (Storage API) says "async methods" but doesn't specify:
- Should methods be cancellable mid-call?
- What error codes for timeouts?
- Transaction semantics for concurrent writes?

Without clarity, Coder will guess and fail validation.

Options:
A) I enrich spec with my best interpretation (risk: wrong)
B) You clarify these 3 questions (we record answers in enrichment)
C) Defer this WP (focus on clearer requirements first)

Need user decision by: 2025-12-26 09:00

Signature for enrichment if option B: Please provide {username}{DDMMYYYYHHMM}
```

---

## 9. PERFECTION CHECKLIST: Self-Audit Before Work Cycle

**Run this checklist before delegating ANY work packet:**

- [ ] Task packet file exists and is readable
- [ ] All 10 required fields present (no TBD/TK)
- [ ] SPEC_ANCHOR references Main Body (verified in SPEC_CURRENT.md)
- [ ] IN_SCOPE_PATHS are exact file paths (not vague)
- [ ] OUT_OF_SCOPE covers deferred but related work
- [ ] DONE_MEANS map 1:1 to SPEC_ANCHOR requirements
- [ ] TEST_PLAN has exact bash commands (includes `just post-work`)
- [ ] BOOTSTRAP has 5-15 FILES_TO_OPEN, 10-20 SEARCH_TERMS, RISK_MAP
- [ ] USER_SIGNATURE locked (one-time use verified via grep)
- [ ] Packet in TASK_BOARD with correct status
- [ ] Blockers documented (dependencies clear)
- [ ] `just pre-work WP-{ID}` returns PASS
- [ ] No packet edits needed (pre-work passed first try)
- [ ] Handoff message is clear (one-read understanding)
- [ ] Pre-Delegation 14-item checklist passed

**If ANY item is NO â†’ Don't delegate. Fix and re-check.**

---

## 10. FINAL SUMMARY: What Perfect Looks Like

A **perfect Orchestrator**:

| Dimension | Perfect Behavior |
|-----------|---|
| **Task Packets** | Complete, no ambiguity, `just pre-work` passes, locked with signature |
| **Spec Enrichment** | Minimal, user-approved, signature-verified, SIGNATURE_AUDIT maintained |
| **TASK_BOARD** | Current, in-sync with packets, phase gates clear, dependencies explicit |
| **Dependencies** | Documented, enforced, blockers tracked, no surprise failures |
| **Governance** | Signature gates work, locked packets immutable, audit trail complete |
| **Communication** | Clear handoffs, escalates ambiguity, supports Coder + Validator |
| **Quality** | 100% pre-work check pass, 0 Coder WHAT-questions, 0 signature violations |
| **Accountability** | Decisions traceable, mistakes fixed immediately, self-audit before handoff |

---

**ORCHESTRATOR RUBRIC VERSION 1.0**
**Effective:** 2025-12-25
**Next Review:** After Phase 1 completion or when first failure occurs
````


###### Template File: `.GOV/roles_shared/docs/MIGRATION_GUIDE.md`
Intent: Optional migration law (only applicable when the project uses Rust + sqlx; otherwise omit).
````md
# MIGRATION_GUIDE (LAW) â€” Portable Migrations with sqlx::migrate!

Authority: Master Spec section 2.3.12 (CX-DBP-011, CX-DBP-022) and {{CODEX_FILENAME}}.

## LAW: Portable SQL Invariants
- Use `$n` placeholders only (`$1`, `$2`, ...). `?1` / `?2` are forbidden.
- Timestamps must be `TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP`. Do not use `strftime`, `datetime('now')`, or precision hacks.
- No `CREATE TRIGGER`, `OLD.`, or `NEW.` usage. Mutation tracking lives in the application layer.
- Migrations are pure DDL; avoid backend-specific pragmas or data transforms that assume SQLite-only behaviour.
- Number migrations sequentially (`0001_`, `0002_`, ...); keep one canonical copy that runs on SQLite and PostgreSQL.

## LAW: Migration Framework Usage
- Run migrations via `sqlx::migrate!("./migrations").run(&pool)` in the storage bootstrap.
- Rely on sqlxâ€™s `_sqlx_migrations` tracking; do not create or maintain a manual `schema_version` table.
- Migrations must be replay-safe (heavy per-file): re-running the same migration file MUST NOT error (do not rely solely on `_sqlx_migrations`).
- Phase 1 requires concrete down migrations: for every `000X_name.sql` up migration, provide `000X_name.down.sql` and validate up+down in tests/CI.

## LAW: Validation Before Merge
- `cargo test --manifest-path {{BACKEND_CARGO_TOML}}`
- `just validator-dal-audit` (portable SQL audit for migrations/)
- `just validator-hygiene-full`
- `just post-work WP-{id}` for the active work packet

## Checklist for New Migrations
- [ ] File name is numbered and ordered (000X_*.sql).
- [ ] Uses `$n` placeholders only.
- [ ] Timestamps use `TIMESTAMP ... DEFAULT CURRENT_TIMESTAMP`.
- [ ] No triggers or DB-specific datetime functions.
- [ ] Tested with the validation commands above.
````


###### Template File: `docs/TASK_PACKET_TEMPLATE.md`
Intent: Compatibility shim that points to the canonical template under `.GOV/templates/` (optional).
````md
# MOVED

The canonical task packet template lives at:

`.GOV/templates/TASK_PACKET_TEMPLATE.md`

This file remains as a compatibility shim for older links and historical task packets.
````


###### Template File: `docs/REFINEMENT_TEMPLATE.md`
Intent: Compatibility shim that points to the canonical template under `.GOV/templates/` (optional).
````md
# MOVED

The canonical refinement template lives at:

`.GOV/templates/REFINEMENT_TEMPLATE.md`

This file remains as a compatibility shim for older links and historical task packets.
````


###### Template File: `docs/AI_WORKFLOW_TEMPLATE.md`
Intent: Compatibility shim that points to the canonical template under `.GOV/templates/` (optional).
````md
# MOVED

The canonical workflow template lives at:

`.GOV/templates/AI_WORKFLOW_TEMPLATE.md`

This file remains as a compatibility shim for older links and historical task packets.
````

<!-- GOV_PACK_TEMPLATE_VOLUME_END -->

#### 7.5.4.11 Product Governance Snapshot (HARD)

**Purpose**  
Provide a deterministic, leak-safe snapshot of the current governance state for a product/repo so a fresh agent (or auditor) can reconstruct "what is true" without relying on chat history.

**Definition**  
A "Product Governance Snapshot" is a machine-readable JSON export derived ONLY from canonical governance artifacts (no repo scan; no extras):
- `.GOV/spec/SPEC_CURRENT.md`
- resolved spec file referenced inside it (e.g., `Handshake_Master_Spec_v02.125.md`)
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
- `.GOV/validator_gates/*.json` (if present)

**Output location (HARD)**  
- Default path: `.GOV/roles_shared/runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`
- The export MUST be deterministic for a given set of input files.
- The export MUST NOT include wall-clock timestamps.
- The export MAY include the current git HEAD sha (if available) as provenance.
- The output bytes MUST be `JSON.stringify(obj, null, 2) + "\\n"` (force `\\n` newlines; no locale formatting).

**Determinism (HARD)**  
- Generator MUST enforce stable ordering:
  - `inputs` sorted by `path` (ascending).
  - `task_board.entries` sorted by `wp_id` (ascending).
  - `traceability.mappings` sorted by `base_wp_id` (ascending).
  - `signatures.consumed` sorted by `signature` (ascending).
  - `gates.validator.wp_gate_summaries` sorted by `wp_id` (ascending) if present.
- Generator MUST avoid locale/time dependent formatting (no wall clock calls).

**Minimum schema (normative)**  
ProductGovernanceSnapshot
- schema_version: "hsk.product_governance_snapshot@0.1"
- spec: { spec_target: string, spec_sha1: string }
- git: { head_sha?: string } (generator SHOULD default to `git: {}`; omit head_sha unless explicitly enabled)
- inputs: [{ path: string, sha256: string }]
- task_board: { entries: [{ wp_id: string, status_token: string }] }
- traceability: { mappings: [{ base_wp_id: string, active_packet_path: string }] }
- signatures: { consumed: [{ signature: string, purpose: string, wp_id?: string }] }
- gates: { orchestrator: { last_refinement?: string, last_signature?: string, last_prepare?: string }, validator: { wp_gate_summaries?: [{ wp_id: string, verdict?: string, status?: string, gates_passed?: string[] }] } }
  - `wp_gate_summaries` MUST be a list (not a map/object) and MUST omit timestamps and raw logs/bodies.

**Security (HARD)**  
- Snapshot MUST NOT include secrets, environment variables, or raw Role Mailbox message bodies.
- References to external artifacts MUST be by hash/ref only.

**Command surface (HARD)**  
- A single deterministic command MUST exist to generate/refresh the snapshot (e.g., `just governance-snapshot`).
- A validator MUST exist to check schema + determinism + leak-safety (e.g., `just validator-governance-snapshot`).


<a id="76-development-roadmap"></a>
## 7.6 Development Roadmap

**Roadmap tag provenance notes (non-normative)**
- **v02.139 note:** Promptâ†’Spec hardening quartet added to the Main Body (SpecPromptPack + SpecPromptCompiler, CapabilitySnapshot, SpecLint). Roadmap updates are **additive only** and tagged `[ADD v02.139]`.

- **v02.130 note:** Loom integration spec merged into master spec (LoomBlock/LoomEdge + Loom views + schemas/events). Roadmap updates are **additive only** and tagged `[ADD v02.130]`.
- **v02.131 note:** Handshake Stage spec merged into master spec (Â§10.13). Roadmap updates are **additive only** and tagged `[ADD v02.131]`.
- **v02.136 note:** Unified Tool Surface Contract (HTC v1.0, Â§6.0.2) + Tool Gate (single enforcement point) were normalized into the Main Body. Roadmap updates are **additive only** and tagged `[ADD v02.136]`.
- **v02.136 note:** `Handshake_Design_Studio_Overhaul_v0.1.md` is treated as a **UI/IA recontextualization** of existing modules/worksurfaces (not a replacement of Handshake). Roadmap entries schedule the shell/IA shift in Phase 2+ to avoid Phase 1 rework.
- **v02.129 note:** Roadmap normalization pass: converted legacy `**ADD v02.xxx â€” â€¦**` atomic blocks into inline phase-field patches; version tags preserved; no scope change.
- **v02.127 note:** Sidecar tech integration spec merged into master spec as Â§10.11 (Dev Command Center). Roadmap updates are **additive only** and tagged `[ADD v02.127]` (Phase 0 remains closed). New implementation focus: workspaces=git worktrees + WP linkage, Approval Inbox (capability gating UX), Execution Session Manager, Objective Anchor Store + Handoff records, and governed VCS/search/run queues via `engine.version`/`engine.context`/`engine.sandbox` (MEX v1.2; no-bypass).
- **v02.120 note:** Runtime Integration Addendum v0.5 merged into master spec. Roadmap updates are **additive only** and tagged `[ADD v02.120]` (Phase 0 remains closed). New implementation focus: ModelSwap/resource management, Work Profiles, AutomationLevel autonomous governance + GovernanceDecision audit, Role Mailbox (â€œInboxâ€) alignment + runtime telemetry, cloud escalation consent artifacts/events, and promptâ†’macroâ†’micro pipeline integration.
- **v02.123 note:** Atelier/Lens Addendum v0.2.3 merged into Â§6.3.3.5.7 as Â§6.3.3.5.7.11â€“Â§6.3.3.5.7.25. Roadmap updates are **additive only** and tagged `[ADD v02.123]` (Phase 0 remains closed). New implementation focus: Tier1/Tier2 extraction depth separation, Tier2 auto-when-idle scheduling, SYM-001 template growth + profile drift/branching, selection-scoped Atelier collaboration, and hard-drop SFW projection.
- **v02.36 note:** Additive roadmap entries are tagged `[ADD v02.36]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.40 note:** Additive roadmap entries are tagged `[ADD v02.40]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.42 note:** Additive roadmap entries are tagged `[ADD v02.42]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.49 note:** Additive roadmap entries are tagged `[ADD v02.49]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.52 note:** Additive roadmap entries are tagged `[ADD v02.52]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.63 note:** Additive roadmap entries are tagged `[ADD v02.63]` (reconciliation of orphans; no rewrites of prior bullets).
- **v02.101 note:** Additive roadmap entries are tagged `[ADD v02.101]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.102 note:** Additive roadmap entries are tagged `[ADD v02.102]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.103 note:** Additive roadmap entries are tagged `[ADD v02.103]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.104 note:** Additive roadmap entries are tagged `[ADD v02.104]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.105 note:** Additive roadmap entries are tagged `[ADD v02.105]` (no rewrites of existing bullets; Phase 0 remains closed).
- **v02.115 note:** Additive roadmap entries are tagged `[ADD v02.115]` (no rewrites of existing bullets; Phase 0 remains closed). **Flight Recorder remediation:** FR-EVT-DATA-001..015 events for AI-Ready Data Architecture (Â§2.3.14) are NEW scope that requires updating the existing Flight Recorder implementationâ€”this is remediation work, not new infrastructure. **Skill Bank remediation:** LoRA training data flow from retrieval quality signals (Â§2.3.14.9, Â§9) requires extending existing Skill Bank schemas.
- **v02.116 note:** Roadmap entries tagged `[ADD v02.116]` (Locus Work Tracking System, Â§2.3.15) are **open to revision**. All Task Board entries tagged `v02.116` MUST be reviewed and revised/updated to match current Locus WP ID + status semantics and the updated Phase bullets.
**Why**  
A clear roadmap with phases and dependencies ensures focused effort and prevents scope creep. This section provides the practical build order for Project Handshake.

**What**  
Defines four development phases (Foundation, Core Editing, AI Integration, Visual Tools + Polish), specifies MVP scope, and shows the dependency graph for build order. Each phase now explicitly carries a **Mechanical Track** so engines from Section 6 are woven in, not deferred.

**Jargon**  
- **MVP (Minimum Viable Product)**: The smallest set of features that delivers value and validates the concept.
- **IPC (Inter-Process Communication)**: How the frontend and backend processes communicate.
- **Phase 0**: Foundation work (monorepo, scaffolding, CI pipeline, basic IPC).

---

**Why**  
Handshake is intentionally ambitious: local models, workflows, governance, ingestion, ASR, collaboration. A roadmap is required to sequence this complexity into phases that are buildable, testable, and debuggable. The goal is to ensure that governance (Diary, AI Job Model, capabilities), observability (Flight Recorder, metrics, traces), migrations, and debug tools are present from the start, not bolted on after features ship.

**What**  
This section defines a phased implementation plan for Handshake, from a pre-MVP foundation to a multi-user, extensible workspace. Each phase specifies what MUST be shipped, what is explicitly out of scope, and:

- A **vertical slice** (end-to-end user flow) that proves the phase is real, not just infra.  
- Key **risks** the phase should reduce.  
- **Acceptance criteria** that define â€œDoneâ€, including debug and diagnostic surfaces.
Cross-ref: roadmap phases should account for Terminal/Monaco delivery (Section 10) and shared capability/observability contracts (Section 11) as part of milestones and acceptance.

All phases are aligned with the architecture and mechanisms defined in Sections 2â€“6 (Architecture, Data Model, LLM Infrastructure, Observability, Mechanical Integrations).

**Jargon**  
- **Pre-MVP (Phase 0)** â€“ Foundation work that produces a running but non-compelling app; used to validate architecture, tooling, and debug surfaces.  
- **Product MVP (Phase 1)** â€“ First version that a single user can use for serious work, with governance and full diagnostic surfaces.  
- **Phase** â€“ A coherent bundle of changes that is shippable, testable, and has a clear vertical slice.  
- **Core loop** â€“ The smallest end-to-end user flow that exercises architecture and observability: â€œedit doc â†’ ask AI â†’ see changes + history + logs.â€  
- **Shadow Workspace** â€“ Background index and graph over workspace content used for search and RAG.  
- **Flight Recorder** â€“ Append-only event log for AI jobs, workflows, and user-visible actions, used for debugging and audit.  
- **AI Job** â€“ A single AI operation with ID, profile, capabilities, inputs, outputs, lifecycle, and provenance, as defined by the global AI Job Model.  
- **Debug surface** â€“ Any UI, log view, trace viewer, or health check that makes it possible for a human to understand and diagnose system behaviour without reading the entire codebase.  
- **Vertical slice** - A thin, end-to-end scenario that exercises UI, backend, data, and observability in one flow.  
- **Mechanical job** - A deterministic operation executed by an external engine (CAD, search, media, etc.) through the Gate/Body pattern, producing artifacts with sidecar provenance.

---

### 7.6.1 Roadmap Coverage Matrix (Section-Level Determinism) (HARD)

#### 7.6.1.1 Scope and Principles

This roadmap applies to the **entire Handshake product**, not just subsystems.

It MUST:

- Align with the architectural layers in Section 2 (Architecture), Section 3 (Data Model), Section 4 (LLM Infrastructure), Section 5 (Observability & Benchmarks), and Section 6 (Mechanical Integrations).  
- Ensure that **AI Job Model**, **Workflow & Automation Engine**, **Flight Recorder**, and the **capability system** are exercised early and consistently.  
- Deliver a **single-user, local-first, offline-capable** product before adding multi-user sync, plugins, or cloud dependencies.  
- Treat **Docling** and **ASR** as extensions on top of the same AI Job and workflow mechanisms, not separate systems.  
- [ADD v02.139] Treat Promptâ†’Spec as a governed, reproducible pipeline: Spec Router MUST use SpecPromptPack+SpecPromptCompiler, emit CapabilitySnapshot, and enforce SpecLint preflight before rubric/red-team and execution.  
- Require that every user-facing feature ships with a **diagnostic path**:
  - Logs or events in Flight Recorder.  
  - At least one debug surface (UI, CLI, or trace) that shows how it behaves.  
- Use each phase to **burn down risk**, not only to add features.  
- Provide **clear acceptance criteria** per phase: conditions and tests that must pass before moving on.  
- [ADD v02.103] **Phase closure rule (HARD, no technical debt):** The roadmap is a pointer only; a phase is complete only when all governing Master Spec Main Body requirements (Sections 1â€“6 and 9â€“11) for that phase are implemented and validated. A Vertical slice is necessary but not sufficient.
- Use a fixed **phase template** (fields, in order):
  - **Goal**
  - **MUST deliver**
  - **Key risks addressed in Phase n**
  - **Mechanical Track (Phase n)** (if present)
  - **Atelier Track (Phase n)** (if present)
  - **Distillation Track (Phase n)** (if present)
  - **Vertical slice**
  - **Acceptance criteria**
  - **Explicitly OUT of scope**
  - **Status** (only when a phase is closed)
- [ADD v02.122] **Do not add a permanent â€œAddendumâ€ section.** Place content into the topic where it belongs and update roadmap matrices and cross-references accordingly.
- [ADD v02.105] **No privileged fields:** For phase closure, every line in a phase section is equal importance; "MUST deliver" does not override or exclude other fields (Key risks, Tracks, Vertical slice, Acceptance criteria, Explicitly OUT of scope).
- [ADD v02.128] **No new roadmap phase fields:** Do not create new permanent phase-template blocks/sections. Weave new work into the fixed phase template fields (Goal/MUST deliver/Key risks/Tracks/Vertical slice/Acceptance/Out of scope).
- [ADD v02.128] **Roadmap reflection rule (HARD):** Any normative addition merged into the Master Spec (including subsection-level imports that do not create new `## X.Y` headings) MUST be reflected in the Roadmap the same change by adding `[ADD vXX]` bullets in the relevant Phase(s) that point to the new subsection(s).

#### 7.6.1.2 Coverage Matrix (HARD)

- [ADD v02.104] Established the section-level Coverage Matrix (audit-first) to prevent Roadmap drift.
- [ADD v02.105] Phase-allocated all matrix rows (P1-P4) and removed Phase 0 allocations to reflect that Phase 0 is closed.
- [ADD v02.138] Front End Memory System (FEMS) merged as **subsection-level** patches under Â§2.6.6.6.6, Â§2.6.6.7.6.2, Â§4.3.9.12.7, Â§5.4.8, Â§10.11.5.14, and Â§11.5.13; Coverage Matrix rows remain unchanged because it tracks `X.Y` (and top-level `# X.`) headings.

This Roadmap MUST include and maintain a **section-level Coverage Matrix** that prevents Roadmap drift.

**Definitions**
- **Section-level**: The unit is a Master Spec section number (`X.Y`) and any top-level `# X.` section that has no `## X.Y` headings (currently `# 9.`).
- **Main Body Authority (CX-598)**: Sections in Â§1â€“Â§6 and Â§9â€“Â§11 are the authoritative definition of â€œDoneâ€. The Roadmap is scheduling/pointer only; it does not redefine authority.

**Rules (HARD)**
1. The Coverage Matrix MUST list every `## X.Y` section outside Â§7.6 AND the top-level `# 9.` section.
2. Every listed section MUST appear **exactly once** in the matrix.
3. The matrix MUST include:
   - `Section` (exact number)
   - `Title` (exact heading text)
   - `Main Body Authority (CX-598)` (YES/NO)
   - `Roadmap Coverage (Phase(s))` (allowed values: `P1`, `P2`, `P3`, `P4` or a comma-separated list).
4. Any edit that adds/removes/renumbers sections MUST update the matrix in the same change.
5. [ADD v02.128] Subsection-level normative imports (no new `## X.Y` row) MUST still update the Roadmap in the same change:
   - Add `[ADD vXX]` bullets to the relevant Phase(s) that point to the new subsection numbers (do not rely on the parent row alone).
   - Add a note bullet above this table documenting the patch and confirming which existing matrix row covers it.
6. Missing rows, duplicate rows, or incorrect section numbers are a **BLOCKING governance failure**.
7. Phase 0 is closed: the matrix MUST NOT allocate any row to `P0`. Any newly discovered Phase 0 gaps MUST be scheduled in `P1` or later.
8. The matrix MUST NOT use placeholders like `UNSCHEDULED`; every row MUST have at least one phase allocation.

**Coverage Matrix (v02.105 baseline; phase-allocated; Phase 0 closed)**
- [ADD v02.123] Atelier/Lens Addendum v0.2.3 merged as a **subsection-level** patch under Â§6.3.3.5.7; Coverage Matrix rows are unchanged (no new `## X.Y` sections added) and phase allocations remain valid.
- [ADD v02.114] Â§2.6.6.8 Micro-Task Executor Profile added as subsection of Â§2.6; covered by existing Â§2.6 row (P1, P2, P3, P4).
- [ADD v02.115] Â§2.3.14 AI-Ready Data Architecture added; cross-cutting integration with all tool sections (Â§10.x).
- [ADD v02.120] Runtime Integration Addendum v0.5 merged into existing sections (not new top-level `##` rows): model swaps (Â§4.3), Work Profiles (Â§4.3), AutomationLevel governance (Â§2.6.8), cloud escalation consent (Â§11.1), event catalog additions (Â§11.5), and debug bundle exports (Â§10.5).
- [ADD v02.128] Â§2.6.8.13 Spec Creation System v2.2.1 imported as a **subsection-level** patch under Â§2.6.8; Coverage Matrix rows are unchanged (covered by existing Â§2.6 row), but Phase bullets are updated to schedule implementation.
- [ADD v02.127] Â§10.11 Dev Command Center (Sidecar Integration) added as new Product Surface; Coverage Matrix updated with new row (Phase 0 closed).
- [ADD v02.136] Â§6.0.2 Unified Tool Surface Contract (HTC v1.0) imported as a **subsection-level** patch under Â§6.0; Coverage Matrix rows are unchanged (covered by existing Â§6.0 row), but Phase bullets are updated to schedule implementation and prevent â€œdual tool schemaâ€ drift.
- [ADD v02.152] Spec Router / Locus / MCP / MEX backend evidence-projection deepening lands as subsection-level patches under Â§2.3, Â§2.6, Â§6.0, and Â§11.5/Â§11.8; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen backend orchestration, projection, and replay contracts.
- [ADD v02.153] Capability / diagnostics backend evidence deepening lands as subsection-level patches under Â§2.3, Â§2.6, Â§11.1, and Â§11.5; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen capability enforcement, recorder correlation, diagnostics materialization, and consent artifact portability.
- [ADD v02.154] Governance Pack / Workspace Bundle backend export reciprocity lands as subsection-level patches under Â§6.0, Â§7.5.4, and Â§10.5.7; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 backfill governance-export feature ownership, recorder visibility, and storage-portability edges.
- [ADD v02.155] Calendar backend force-multiplier deepening lands as subsection-level patches under Â§2.6.6.8, Â§10.4, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets deepen Calendar storage-portability, consent, AI-job mutation, and ACE / Spec Router routing posture.
- [ADD v02.156] Knowledge/retrieval pillar deepening lands as subsection-level patches under Â§2.3.13, Â§2.5.8, Â§2.5.12, and Â§2.6.7; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen retrieval-substrate ownership, deterministic routing registry posture, portable ContextPack artifacts, and Loom storage portability.
- [ADD v02.159] Correlation/projection backend deepening lands as subsection-level patches under Â§10.5, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 clarify Dev Command Center vs Operator Consoles ownership, deepen Role Mailbox projection posture, and add control/projection edges into Flight Recorder and Debug Bundle.
- [ADD v02.164] Dev Command Center resilience and repository-decision deepening lands as subsection-level patches under Â§4.3.9, Â§6.3.10, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen session recovery, provider readiness, anti-pattern surfacing, and version-control backend policy.
- [ADD v02.165] Dev Command Center operating-surface deepening lands as subsection-level patches under Â§4.3.9, Â§6.0.2, Â§6.3.10, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen run history, tool infrastructure health, workspace runtime readiness, and promotion-gate snapshots.
- [ADD v02.166] Structured collaboration-substrate deepening lands as subsection-level patches under Â§2.3.15, Â§2.6.6.8, Â§2.6.8.8, Â§2.6.8.10, Â§7.2, Â§10.11, and Appendix 12; Coverage Matrix rows remain unchanged while Phase 1 bullets and Appendix 12 deepen structured work records, append-only notes, collaboration-inbox routing, and Dev Command Center field viewers.
- [ADD v02.179] Workflow-correlation bundle-scope deepening lands as subsection-level patches under Ã‚Â§2.3, Ã‚Â§2.6, and Ã‚Â§10.5 plus Appendix 12.5; Coverage Matrix rows remain unchanged while Phase 1 bullets and FEAT-DEBUG-BUNDLE guidance deepen workflow-run and node-execution export posture.
- [ADD v02.181] Software-delivery governance overlay boundary deepening lands as subsection-level patches under 7.5.4.8 and 7.5.4.9 plus Appendix 12; Governance Pack import/export remains the repo-facing transfer surface while repository `/.GOV/**` artifacts stay imported overlay source material or evidence rather than live runtime authority.
- [ADD v02.181] Software-delivery runtime-truth deepening lands as subsection-level patches under 2.3.15, 2.6.8.8, 2.6.8.10, and 7.2 plus Appendix 12; Coverage Matrix and UI guidance deepen `FEAT-LOCUS-WORK-TRACKING`, `FEAT-ROLE-MAILBOX`, and `FEAT-WORKFLOW-ENGINE` around product-owned runtime records, workflow-backed governed actions, and stable-id-linked software-delivery state.
- [ADD v02.181] Validator-gate and closeout deepening lands as subsection-level patches under 2.3.15, 7.2, 7.5.4.9, and 10.11 plus Appendix 12; Phase 1 bullets and Appendix 12 clarify validator-gate convergence, gate-summary projection, evidence-linked gate execution, and derived closeout posture instead of packet-surgery closeout truth.
- [ADD v02.181] Projection-surface deepening lands as subsection-level patches under 2.6.8.8, 2.6.8.10, 7.2, and 10.11 plus Appendix 12; Coverage Matrix and UI guidance deepen `FEAT-DEV-COMMAND-CENTER`, `FEAT-ROLE-MAILBOX`, and `FEAT-LOCUS-WORK-TRACKING` so Dev Command Center, Task Board, and Role Mailbox remain projection and control surfaces over the same runtime truth without mailbox chronology or Markdown mirrors becoming authority.
- [ADD v02.181] Overlay coordination-record deepening lands as subsection-level patches under 2.3.15, 7.2, and 10.11 plus Appendix 12; Phase 1 bullets and primitive coverage add overlay claim/lease posture and queued-instruction posture so takeover, steering, follow-up, and actor-eligibility decisions can be modeled without relying on ad hoc comments or transcript order.
- [ADD v02.181] Overlay lifecycle and control-plane deepening lands as subsection-level patches under 2.3.15, 7.2, and 10.11 plus Appendix 12; Phase 1 bullets and Appendix 12 clarify lifecycle checkpoints, checkpoint-backed recovery posture, and workflow-backed start/steer/cancel/close/recover control-plane semantics.
- **Note:** Phase 0 is now closed. No new work items may be allocated to Phase 0; any remaining Phase 0 stubs MUST be either completed as part of Phase 1 or explicitly moved to a later phase.

| Section | Title | Main Body Authority (CX-598) | Roadmap Coverage (Phase(s)) |
|---|---|---|---|
| 1.1 | Executive Summary | YES | P1, P2, P3, P4 |
| 1.2 | The Diary Origin Story | YES | P1, P2, P3, P4 |
| 1.3 | The Four-Layer Architecture | YES | P1, P2, P3, P4 |
| 1.4 | LLM Reliability Hierarchy | YES | P1, P2, P3, P4 |
| 1.5 | What Gets Ported from the Diary | YES | P1, P2, P3, P4 |
| 1.6 | Design Philosophy: Self-Enforcing Governance | YES | P1, P2, P3, P4 |
| 1.7 | Success Criteria | YES | P1, P2, P3, P4 |
| 1.8 | Introduction | YES | P1, P2, P3, P4 |
| 2.1 | High-Level Architecture | YES | P1, P2, P3, P4 |
| 2.2 | Data & Content Model | YES | P1, P2, P3, P4 |
| 2.3 | Content Integrity (Diary Part 5: COR-700) | YES | P1, P2, P3, P4 |
| 2.3.14 | AI-Ready Data Architecture [ADD v02.115] | YES | P1, P2, P3, P4 |
| 2.3.15 | Locus Work Tracking System [ADD v02.116] | YES | P1, P2, P3, P4 |
| 2.4 | Extraction Pipeline (The Product) | YES | P2, P3, P4 |
| 2.5 | AI Interaction Patterns | YES | P1, P2, P3, P4 |
| 2.6 | Workflow & Automation Engine | YES | P1, P2, P3, P4 |
| 2.7 | Response Behavior Contract (Diary ANS-001) | YES | P1, P2, P3, P4 |
| 2.8 | Governance Runtime (Diary Parts 1-2) | YES | P1, P2, P3, P4 |
| 2.9 | Deterministic Edit Process (COR-701) | YES | P1, P2, P3, P4 |
| 2.10 | Session Logging (LOG-001) | YES | P1, P2, P3, P4 |
| 3.1 | Local-First Data Fundamentals | YES | P1, P2, P3, P4 |
| 3.2 | CRDT Libraries Comparison | YES | P4 |
| 3.3 | Database & Sync Patterns | YES | P4 |
| 3.4 | Conflict Resolution UX | YES | P4 |
| 4.1 | LLM Infrastructure | YES | P1, P2, P3, P4 |
| 4.2 | LLM Inference Runtimes | YES | P1, P2, P3, P4 |
| 4.3 | Model Selection & Roles | YES | P1, P2, P3, P4 |
| 4.3.9 | Multi-Model Orchestration & Lifecycle Telemetry [ADD v02.122] | YES | P1, P2, P3, P4 |
| 4.4 | Image Generation (Stable Diffusion) | YES | P2, P3, P4 |
| 4.5 | Layer-wise Inference & Dynamic Compute (Exploratory) [ADD v02.122] | YES | P1, P3, P4 |
| 5.1 | Plugin Architecture | YES | P4 |
| 5.2 | Sandboxing & Security | YES | P1, P2, P3, P4 |
| 5.3 | AI Observability | YES | P1, P2, P3, P4 |
| 5.4 | Evaluation & Quality | YES | P1, P2, P3, P4 |
| 5.5 | Benchmark Harness | YES | P2, P3, P4 |
| 6.0 | Mechanical Tool Bus & Integration Principles | YES | P1, P2, P3, P4 |
| 6.1 | Document Ingestion: Docling Subsystem | YES | P2, P3, P4 |
| 6.2 | Speech Recognition: ASR Subsystem | YES | P3, P4 |
| 6.3 | Mechanical Extension Engines | YES | P1, P2, P3, P4 |
| 7.1 | User Interface Components | NO | P1, P2, P3, P4 |
| 7.2 | Multi-Agent Orchestration | NO | P1, P2, P3, P4 |
| 7.3 | Collaboration and Sync | NO | P4 |
| 7.4 | Reference Application Analysis | NO | P1, P2 |
| 7.5 | Development Workflow | NO | P1, P2, P3, P4 |
| 8.1 | Risk Assessment | NO | P1 |
| 8.2 | Technology Stack Summary | NO | P1 |
| 8.3 | Gap Analysis & Open Questions | NO | P1 |
| 8.4 | Consolidated Glossary | NO | P1 |
| 8.5 | Sources Referenced | NO | P1 |
| 8.6 | Appendices | NO | P1 |
| 8.7 | Version History & Subsection Versioning | NO | P1 |
| 9 | Continuous Local Skill Distillation (Skill Bank & Pipeline) | YES | P1, P2, P3, P4 |
| 10.1 | Terminal Experience | YES | P1, P2, P3, P4 |
| 10.2 | Monaco Editor Experience | YES | P1, P2, P3, P4 |
| 10.3 | Mail Client | YES | P3, P4 |
| 10.4 | Calendar | YES | P1, P2, P3, P4 |
| 10.5 | Operator Consoles: Debug & Diagnostics | YES | P1, P2, P3, P4 |
| 10.6 | Canvas: Typography & Font Packs | YES | P1 |
| 10.7 | Charts & Dashboards | YES | P1, P2, P3, P4 |
| 10.8 | Presentations (Decks) | YES | P1, P2, P3, P4 |
| 10.9 | Future Surfaces | YES | P4 |
| 10.10 | Photo Studio | YES | P1, P2, P3, P4 |
| 10.11 | Dev Command Center (Sidecar Integration) | YES | P1, P2, P3, P4 |
| 10.12 | Loom (Heaper-style Library Surface) | YES | P1, P2, P4 |
| 10.13 | Handshake Stage (Built-in Browser + Stage Apps) [ADD v02.131] | YES | P1, P2, P3, P4 |
| 11.1 | Capabilities & Consent Model | YES | P1, P2, P3, P4 |
| 11.2 | Sandbox Policy vs Hard Isolation | YES | P1, P2, P3, P4 |
| 11.3 | Auth/Session/MCP Primitives | YES | P1, P2, P3, P4 |
| 11.4 | Diagnostics Schema (Problems/Events) | YES | P1, P2, P3, P4 |
| 11.5 | Flight Recorder Event Shapes & Retention | YES | P1, P2, P3, P4 |
| 11.6 | Plugin/Matcher Precedence Rules | YES | P4 |
| 11.7 | OSS Component Choices & Versions | YES | P1, P2, P3, P4 |
| 11.8 | Mechanical Extension Specification v1.2 (Verbatim) | YES | P1, P2, P3, P4 |
| 11.9 | Future Shared Primitives | YES | P4 |
| 11.10 | Implementation Notes: Phase 1 Final Gaps | YES | P1 |

- Treat completed phases as **closed**: if requirements are discovered late, they MUST be scheduled into the **current or later** phase (never retroactively added to a finished phase).  
- Preserve a **migration path**: schema and config changes must respect existing data where possible.
- Treat **Mechanical Engines** as first-class: every phase ships at least one mechanical job through the Workflow Engine with capability gates, Flight Recorder logging, and reproducible commands/artifacts.

Out of scope for this section:

- Detailed team planning (sprints, owners, ticket breakdown).  
- Budget and resourcing assumptions.  
- Full QA test plans (see Sections 5.4â€“5.5 instead).

---

### 7.6.2 Phase 0 â€” Foundations (Pre-MVP)

**Status**  
Closed (completed). No new scope may be added to Phase 0; newly discovered requirements MUST be scheduled in Phase 1 or later.

**Goal**  
Stand up a stable â€œHello, workspaceâ€ application that matches the high-level architecture and establishes baseline logging, health checks, and a reproducible dev environment. No serious AI or governance yet, but debug tooling MUST already exist.

**MUST deliver**

1. **Desktop shell and process model**  
   - Tauri-based desktop application with a React front-end.  
   - Backend orchestrator process started and managed by the desktop shell.  
   - Canonical IPC/API channel between frontend and backend (HTTP/WebSocket/IPC), documented and testable.

2. **Workspace and data layer (single user)**  
   - SQLite workspace database with minimal schema for:
     - Workspaces / projects.  
     - Documents and blocks (honouring the Raw/Derived/Display split, even if only Raw/Display are used initially).  
     - Canvases, nodes, and edges.  
   - Basic, tested CRUD operations for documents and canvases.  
   - Initial schema migration mechanism (even simple, versioned migrations) so the DB can evolve safely.

3. **Editors and navigation**  
   - Rich text editor integrated in the main content area with:
     - Headings, paragraphs, lists, code blocks, quotes, inline marks.  
   - Canvas view integrated with:
     - Sticky notes / text boxes, simple shapes, arrows, pan/zoom.  
   - Workspace sidebar listing documents and canvases; open/save loop must be reliable.

4. **Project health, logging, and basic debug tools**  
   - Monorepo and tooling per Section 7.5 (linting, formatting, tests, CI) wired to this stack.  
   - Structured logging in frontend and backend (log level, context, correlation IDs where applicable).  
   - A **health check** endpoint or command that verifies at least:
     - App shell â†’ backend connectivity.  
     - Database connectivity.  
   - A simple developer-facing log view (tail of logs or structured log output) suitable for non-expert developers.  
   - One-command dev startup (script or target that starts frontend, backend, and DB with sample data).

**Vertical slice**  
- Start the app.  
- Create a workspace.  
- Create a document and a canvas, make simple edits, close the app, reopen, and verify content is intact.  
- Run the health check and inspect logs to confirm basic operations are recorded.

**Key risks addressed in Phase 0**

- Stack (Tauri + frontend + backend + DB) is unstable or too hard to run.  
- No consistent logging/health model, making later debugging painful.  
- Schema and migrations are ad-hoc from day one.

**Acceptance criteria**

- App can be started and used locally by running a single documented command.  
- Health check succeeds in a clean environment.  
- Logs clearly show at least workspace creation, document creation, and document save events.  
- A sample workspace can be created, exported (or re-created), and used as a fixture for later phases.

**Explicitly OUT of scope**

- AI Job Model, workflows, Flight Recorder, Shadow Workspace.  
- Multi-user sync or CRDT.  
- Docling, ASR, connectors, plugin system.
- Mechanical engines beyond a stub: only scaffolding and a single proof mechanical job for wiring.

**Mechanical Track (Phase 0)**
- Deliver a **mechanical runner** abstraction (deterministic process exec with resource limits) and capability flags for file/process/cpu/memory.
- Ship one **stub mechanical job** (e.g., `Context` engine using `rg`) through the Workflow Engine with Flight Recorder logging and sidecar provenance (command, exit code, stdout/stderr, artifact hash).
- Acceptance: mechanical job visible in Job History; blocked when capability is missing; artifact stored as DerivedContent if produced.

---

### 7.6.3 Phase 1 â€” Core Product MVP (Single-User, Local AI)

**Goal**  
Deliver the **first real Handshake**: a single-user, local-first workspace where documents and canvases are editable, AI assistance is available, and every AI action is traceable through the AI Job Model, Workflow Engine, Flight Recorder, and capability system. Debug tools for AI behaviour and workflows are mandatory.
- [ADD v02.136] Unify tool invocation: local tool calling and MCP MUST use the same Tool Registry + Tool Gate + Flight Recorder event model (no bypass).
Ship with the default local LLM runtime (Ollama), hardened document/canvas editors (Tiptap/BlockNote and Excalidraw), always-on Atelier Lens, and a prompt-to-spec router that creates Task Board and Work Packet session logs. Git workflows must trigger safety commit behavior while non-git workflows must not.

- [ADD v02.139] Promptâ†’Spec hardening quartet (Phase 1): Spec Router MUST compile prompts via SpecPromptPack+SpecPromptCompiler (PromptEnvelope + ContextSnapshot), MUST inject CapabilitySnapshot (explicit allowlist), and MUST enforce SpecLint preflight gate (G-SPECLINT) before rubric/red-team and MT decomposition.

- [ADD v02.123] Phase 1 Atelier/Lens focus: ship selection-scoped collaboration, Tier1-first extraction with deterministic role-turn isolation, and ViewMode hard-drop projection; Tier2 auto-when-idle is scheduled in Phase 2.

- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.107] Governance kernel adoption (workflow safety): Phase 1 work MUST be executed using the Governance Kernel artifacts and gates (AÂ§7.5.4) so small-context local models and cloud models can hand off deterministically (Task Board + refinements + signed task packets + manifests; CI parity).
- [ADD v02.107] Local-first agentic stance: Phase 1 core loops MUST run fully offline with local models + local tools as the default; MCP and cloud escalation are optional, capability-gated, and must preserve artifact-first outputs and Flight Recorder evidence (AÂ§6.0.1, AÂ§7.2.5, AÂ§11.3).
- [ADD v02.52] ACE-RAG-001 groundwork: make retrieval planning/tracing/budgeting a first-class runtime contract in MVP (QueryPlan + RetrievalTrace + strict budgets + cache keys). ContextPacks/drift/caching effectiveness ship in Phase 2.
- [ADD v02.52] Add: Minimum viable export to prevent lock-in and enable reproducible debugging (bundles).

- [ADD v02.68] Adopt Mechanical Extension v1.2 as the callable Tool Bus contract for all mechanical jobs (Engine PlannedOperation/EngineResult envelopes, artifact-first I/O, no-bypass).
- [ADD v02.68] Require v1.2 global engine gates + conformance harness before expanding mechanical engines beyond MVP wiring (denials/reasons visible in Problems + Flight Recorder).

- [ADD v02.79] Establish **Photo Studio (skeleton)** as a first-class workspace surface governed by AI Job Model + Workflow Engine + Flight Recorder (no-bypass; job-request driven).
- **[ADD v02.130] Loom (Heaper-style library surface) MVP**  
  Deliver a local-first Loom library surface: LoomBlocks (note/file/context), four views (All/Unlinked/Sorted/Pins), and inline @mention/#tag organization with backlinks.
- **[ADD v02.131] Handshake Stage MVP (governed browser + capture surface)**  
  Deliver Handshake Stage as a first-class, governed browser surface with isolated sessions/tabs, embedded Stage Apps, and a Stage Bridge API that can enqueue evidence-backed capture/import jobs (web/PDF/3D) through the Workflow Engine + Mechanical Tool Bus (no bypass).




- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**  
  Make multi-model execution a first-class, governed workflow capability: multiple independent model instances can execute different WPs/MTs in parallel, with strict file-scope locks, deterministic recovery, and compact lifecycle telemetry.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**  
  Deliver a single, canonical control surface that binds **Locus work (WP/MT)** â†” **git workspaces (worktrees)** â†” **execution sessions** â†” **approvals/logs/diffs**, enabling safe parallel development with deterministic recovery and auditability. It is the default place to observe/approve governed tool calls (Tool Gate) and export deterministic debug bundles.

- [ADD v02.142] Runtime visibility contract (Phase 1): every runtime-visible capability touched in Phase 1 MUST land with Appendix 12 capability slices + runtime visibility rows so Command Center, Flight Recorder, Locus, and storage posture are explicit before scope expansion.
- [ADD v02.143] Primitive index coverage contract (Phase 1): every Appendix 12.3 feature touched in Phase 1 MUST land with a normalized Appendix 12.4 coverage row (arrays only, coverage_status, coverage_refs, gap_stub_ids) before coding or further roadmap expansion.
- [ADD v02.144] Second-pass feature-family coverage (Phase 1): explicitly named feature families and runtime shells discovered during refinement MUST be promoted into Appendix 12.3 / 12.4 / 12.5 / 12.6 with stub-backed gap tracking before further matrix expansion.
- [ADD v02.145] Third-pass runtime/data/operator coverage (Phase 1): execution-path features and reusable operator/export/filter/session contracts discovered during refinement MUST be promoted into Appendix 12 with runtime visibility rows and interaction edges before wider matrix expansion continues.
- [ADD v02.146] Deepening pass (Phase 1): seeded rows with real UI/operator, event-taxonomy, export-query, or retrieval-artifact contracts MUST be upgraded in Appendix 12 before widening the feature frontier again.
- [ADD v02.147] Orphan-ownership pass (Phase 1): high-signal orphan primitives for capability/consent, jobs, debug/export, storage portability, and operator projection MUST be attached to owning feature rows or resolved to stubs before new feature-family expansion resumes.
- [ADD v02.166] Structured collaboration substrate (Phase 1): canonical Work Packet, Micro-Task, Task Board, and Role Mailbox state MUST be readable as structured records and rendered through Dev Command Center field viewers; human-readable Markdown remains a mirror or note sidecar instead of the only machine-readable source.
- [ADD v02.167] Canonical structured artifact family (Phase 1): Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST use versioned JavaScript Object Notation or JavaScript Object Notation Lines canonical files with compact summaries, project-agnostic base envelopes, and explicit Markdown mirror-sync rules; Dev Command Center board and queue layouts remain derived projections over those records.
- [ADD v02.168] Base structured schema and project-profile contracts (Phase 1): Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST share one base structured-collaboration envelope with stable record identity, mirror-state semantics, authoritative references, compact summaries, and explicit project-profile extension boundaries so later Handshake kernels can reuse the same artifact family safely.
- [ADD v02.169] Canonical-to-mirror reconciliation and drift governance (Phase 1): Markdown mirrors, note sidecars, and projected board views MUST reconcile against canonical structured records through explicit mirror contracts, authority modes, reconciliation actions, and drift-safe Dev Command Center controls so readable surfaces never become silent competing authorities.
- [ADD v02.170] Dev Command Center typed viewer, board layout, and queue projection (Phase 1): board, queue, list, roadmap, inbox-triage, and execution-queue surfaces MUST be driven by explicit view presets, lane definitions, and governed action bindings; local-small-model readiness queues MUST stay compact-summary-first; and drag or quick actions MUST preview authoritative mutations before execution.
- [ADD v02.171] Project-agnostic workflow-state and governed-action contracts (Phase 1): Work Packets, Micro-Tasks, Task Board projections, Role Mailbox-linked queues, and Dev Command Center operating views MUST share one base workflow-state family, queue-reason vocabulary, and governed-action descriptor contract so routing, triage, review, approval, validation, and completion semantics stay stable across future Handshake project kernels.
- [ADD v02.172] Workflow transition matrix, queue automation, and executor eligibility contracts (Phase 1): the shared workflow-state vocabulary MUST now be paired with explicit transition rules, automatic queue-action rules, and executor eligibility policies so local small models, cloud models, reviewers, workflow automation, and operators can mutate work only through portable, inspectable, approval-aware transitions.
- [ADD v02.173] Role Mailbox message contract, thread lifecycle, and authority boundary (Phase 1): Role Mailbox MUST separate thread lifecycle from message delivery state, encode allowed-response envelopes plus due and snooze posture, carry typed Micro-Task collaboration message families, and request linked work changes only through governed actions or explicit transcription instead of transcript-order heuristics.
- [ADD v02.174] Role Mailbox and Micro-Task loop control (Phase 1): Role Mailbox MUST coordinate verifier-driven Micro-Task retry, feedback, verification-needed, escalation, and completion-report traffic through bounded loop checkpoints, structured verifier outcomes, remaining retry budget, and explicit escalation targets; and Work Packet, Task Board, Locus, and Dev Command Center views MUST project that state without replaying full threads or treating mailbox chronology as authority.
- [ADD v02.175] Role Mailbox triage, queue aging, and remediation controls (Phase 1): Role Mailbox MUST expose durable triage queue state, reminder schedules, snooze and expiry posture, dead-letter disposition, and explicit operator remediation controls; Dev Command Center and Task Board MUST project mailbox pressure, queue age, and remediation posture without treating unread state or thread order as authority; and linked Work Packet, Micro-Task, and Locus views MUST explain mailbox-derived waiting or recovery posture through stable identifiers plus explicit queue reasons.
- [ADD v02.176] Role Mailbox executor routing, claim-lease, and response authority (Phase 1): Role Mailbox MUST expose executor-kind allowlists, claim or lease modes, claimant identity, lease expiry, takeover policy, and response-authority scope for actionable threads; Dev Command Center and Task Board MUST project claimant, lease age, actor-ineligible posture, and takeover legality without turning mailbox claims into work-state authority; and linked Locus, Micro-Task, and Work Packet views MUST explain who may act next through stable identifiers plus governed-action previews.
- [ADD v02.177] Role Mailbox handoff bundle, note transcription, and announce-back provenance (Phase 1): Role Mailbox MUST carry structured handoff bundles and announce-back provenance for delegate, handoff, completion, scope-change, and escalation traffic; Work Packet, Locus, Micro-Task, Task Board, and Dev Command Center views MUST surface remaining work, unresolved blockers, recommended next actor, and transcription status without replaying entire threads; and advisory announce-back messages MUST NOT imply authoritative completion until linked transcription or governed action is confirmed.
- [ADD v02.178] Governed RAG retrieval modes and no-RAG policy (Phase 1): AI-Ready Data, ACE Runtime, Project Brain, Prompt-to-Spec Router, Loom, Work Packets, and Micro-Task Executor MUST share explicit retrieval-mode selection (`none`, `direct_load`, `exact_lookup`, `graph_traversal`, `hybrid_rag`); exact authoritative ids MUST bypass broad hybrid retrieval by default; `QueryPlan` and `RetrievalTrace` MUST record non-hybrid reasons; and local-small-model loops MUST keep hybrid retrieval opt-in, bounded, and compaction-first.
- [ADD v02.179] Workflow-correlation bundle scopes (Phase 1): Debug Bundle export and operator workflow views MUST support explicit `workflow_run` and `workflow_node_execution` scopes, portable `workflow_node_executions.jsonl` inventory, and validator-visible lineage ids so replay and diagnostics stay bounded without time-window reconstruction.
- [ADD v02.148] Ownership-reduction deepening (Phase 1): Stage/media session-auth contracts, multi-session runtime substrate, and debug/export/retention contracts discovered in code MUST be attached to FEAT-STAGE, FEAT-MEDIA-DOWNLOADER, FEAT-MODEL-SESSION-ORCHESTRATION, FEAT-DEBUG-BUNDLE, and FEAT-STORAGE-PORTABILITY before broader orphan hunting resumes.
- [ADD v02.149] Refinement reciprocity hardening (Phase 1): Main Body<->Appendix reciprocity, roadmap/coverage-matrix coupling, mandatory matrix research, mandatory GUI implementation advice, `[ADD v<version>]` packet visibility, primitive exposure/creation reporting, and stub creation for new spec/roadmap entries not absorbed in scope MUST be enforced before broader matrix expansion.
- [ADD v02.150] Backend-heavy matrix expansion (Phase 1): workflow projection correlation, consent audit projection, calendar correlation export, and Stage/media artifact portability MUST be modeled as first-class backend combo tracks with Appendix 12.6 edges and stub-backed follow-on work before frontend-led matrix breadth resumes.
- [ADD v02.151] Backend export/evidence/portability expansion (Phase 1): Role Mailbox, AI-Ready Data, and Workflow Engine MUST surface direct Flight Recorder/storage portability links; unresolved mailbox/debug-bundle, AI-ready/debug-bundle, and calendar-mailbox bridges remain stub-backed until explicit backend contracts land.
- [ADD v02.152] Backend orchestration/projection/replay expansion (Phase 1): Spec Router, Locus, MCP Gate, and MEX Runtime MUST surface direct Flight Recorder/debug-bundle/storage-portability links; unresolved spec-router portability, locus-debug-bundle, and MCP/MEX evidence-export bridges remain stub-backed until explicit backend contracts land.
- [ADD v02.153] Backend capability/diagnostic evidence expansion (Phase 1): workflow capability checks, spec-router capability snapshots, MCP recorder events, and diagnostics bundle materialization MUST surface direct Flight Recorder/debug-bundle/capability links; unresolved cloud-consent evidence portability remains stub-backed until explicit manifest/retention contracts land.
- [ADD v02.154] Backend governance/export reciprocity expansion (Phase 1): Governance Pack and Workspace Bundle export surfaces MUST be appendix-visible backend features with direct Workflow Engine / capability / Flight Recorder / storage-portability posture; unresolved bundle instantiation and workspace-transfer delivery remain stub-backed until full implementation lands.
- [ADD v02.155] Calendar backend force-multiplier expansion (Phase 1): Calendar MUST remain an appendix-visible backend source-state, capability, AI-job mutation, and scope-hint routing surface with direct Storage Portability / Capabilities & Consent / AI Job Model / Spec Router posture; richer mailbox, Locus, and debug-bundle bridges remain stub-backed until concrete backend contracts land.
- [ADD v02.156] Knowledge/retrieval pillar expansion (Phase 1): Project Brain, Semantic Catalog, Context Packs, and Loom MUST remain appendix-visible backend retrieval contracts with direct AI-Ready Data / Spec Router / Storage Portability posture; weaker graph-to-notebook and export-driven bridges remain stub-backed until explicit backend contracts land.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**  
  Introduce stable, governed **dynamic compute** hooks (no algorithm requirement) so future phases can experiment with layer-wise inference without breaking auditability or determinism.

**MUST deliver**

1. **Model runtime integration (LLM core)**  
   - Integrate one local LLM runtime (e.g. Ollama) as specified in Section 4.  
   - Configure at least one general-purpose model.  
   - Ship a default MVP runtime configuration using **Ollama** with at least one preloaded general-purpose model (e.g., **Llama 3 13B** or **Mistral-7B**) enabled out of the box.  
   - Backend API for:
     - Chat-style requests with system + user prompts.  
     - Passing document context (selected text, document snapshot or summary).  

   - [ADD v02.120] Implement sequential model swaps (ModelSwapRequest) with state persistence + ACE recompile; emit FR-EVT-MODEL-*.
   - [ADD v02.120] Implement Work Profiles (role-based model assignment + automation knobs); record `work_profile_id` in job metadata; emit FR-EVT-PROFILE-*.
   - [ADD v02.120] Integrate cloud escalation consent artifacts (ProjectionPlan + ConsentReceipt) + FR-EVT-CLOUD-*; enforce human-gated escalation.
   - [ADD v02.120] Align â€œInboxâ€ UI label to Role Mailbox subsystem; add runtime mailbox telemetry + debug bundle export coverage.




2. **AI Job Model (minimum viable implementation)**  
   - Implement the **global AI Job Model** (Section 2.6.6) in the backend:
     - `job_id`, `job_kind`, `protocol_id`, `status`, timestamps, error, inputs, outputs, metrics.  
     - Profile fields (`profile_id`, `capability_profile_id`, `access_mode`, `safety_mode`).  
   - Implement a **Docs AI Job profile subset** compatible with the Docs & Sheets profile:
     - `doc_id`, selection/range selector, layer scope, provenance fields linking edits back to source content and user.  

3. **Workflow & Automation Engine (minimum viable)**  
   - Implement the Workflow Engine core (Section 2.6) with:
     - Single-node workflows representing one AI job.  
     - Durable state in SQLite (workflow run + job records).  
     - SQLite serves as the authoritative AI job queue/store for MVP workflows.  
     - Status transitions: `queued â†’ running â†’ completed / failed`.  
   - All AI work MUST go through the Workflow Engine; no direct â€œcall the modelâ€ shortcuts are allowed in production paths.

4. **Capability and consent enforcement (minimal slice)**  
   - Define a minimal capability set for documents:
     - `doc.read`, `doc.write`, `doc.summarize` (at minimum).  
   - Every AI job MUST declare required capabilities.  
   - The Workflow Engine MUST enforce that:
     - Jobs without `doc.write` cannot mutate documents.  
     - Write operations are applied only after passing validation (even if the MVP uses a deterministic, auto-accept validator).  
   - Consent-related fields MUST be persisted, even if the MVP uses a simple â€œglobal consentâ€ toggle.

5. **Flight Recorder (always-on, with UI)**  
   - Implement a Flight Recorder subsystem (Section 2.1.5 and Bootloader clauses) with:
     - Append-only log of AI job lifecycle events.  
     - Append-only log of model calls (model name, tokens, latency, outcome).  
     - Minimal tags to correlate events (job ID, workflow ID, document ID, user ID where applicable).  
     - Back the Flight Recorder log store with **DuckDB** to support filtered queries at MVP scale.  
   - Provide a **Job History** panel in the UI:
     - List jobs with status, timestamps, model used, and linked document.  
     - Ability to inspect job input and output payloads.  
     - Provide a basic Flight Recorder filter for `job_id` and `status` to quickly locate related runs.
     - Provide an **Operator Consoles v1** surface (see Â§10.5):
       - **Timeline** view (Flight Recorder events with filters + deep links).
       - **Jobs** view (Job History + per-job inspector).
       - **Problems** view (normalized diagnostics, grouped/deduped, clickable evidence).
       - **Evidence drawer** that shows: job summary, linked trace slice, linked diagnostics, and referenced entities/files.
     - Provide **Debug Bundle export** (redacted-by-default) for a selected `job_id` or time range:
       - Includes: `job.json`, `trace.jsonl`, `diagnostics.json`, `env.json` (no secrets), and `repro.md`.
       - Export action MUST emit a Flight Recorder event and be capability-gated.
  

6. **Baseline metrics and traces (debugging AI behaviour)**  
   - Export basic metrics for:
     - Request counts and error counts.  
     - Latency distribution per action (no target values required here).  
     - Token usage per job/model.  
   - Attach simple trace identifiers to AI jobs and workflow runs so that:
     - A single user action can be followed across model calls and internal steps.  
   - Provide at least one way to view or export these diagnostics (e.g. a debug UI panel or log-based trace view).
   - Implement the **normalized Diagnostic pipeline** (DIAG-SCHEMA-001/002):
     - Deterministic fingerprinting + dedup/grouping rules so repeated failures collapse into a single Problem with a count.
     - Correlate diagnostics to `job_id`, `workflow_id`, `wsid`, and `activity_span_id` where available.
   - Ship a **validator pack** wired into CI:
     - Diagnostic schema validation (required fields, ranges, stable IDs).
     - Flight Recorder event shape validation (minimum linkability fields).
     - Debug Bundle completeness + SAFE_DEFAULT redaction check.
  
   - Instrument AI Job and Workflow engines with **OpenTelemetry** (or compatible SDK) to emit latency, error rate, and token-count metrics as part of the MVP diagnostics surface.  

7. **AI UX in the editor (basic actions)**  
   - Command Palette actions:
     - "Ask about this document" (chat with context).  
     - "Summarize document."  
   - Inline actions:
     - "Rewrite selection" for document text.  
   - All actions MUST:
     - Create AI jobs with the correct profile and capabilities.  
     - Execute via the Workflow Engine.  
     - Persist results back into documents through structured patches.  
     - Emit events into Flight Recorder; the corresponding jobs must appear in Job History and in metrics/traces.  
     - Run on hardened editor components: `Tiptap`/`BlockNote` for documents and `Excalidraw` for canvas interactions, wired through the same AI job/capability/logging pathways.  

8. **Governance hooks (Diary alignment)**  
   - Store enough metadata on jobs and workflows to later map them to Diary RIDs and clauses (activation, modes, gates).  
   - Enforce the invariant: **no silent AI edits**. Every AI mutation of user content MUST be traceable to a specific job and workflow run.  
   - Add basic Bootloader/Diary compliance checks to CI to prevent regressions in logging or observability.

   - [ADD v02.120] Implement AutomationLevel + GovernanceDecision/AutoSignature self-approval protocol (cloud escalation always human-gated); emit FR-EVT-GOV-*.

9. **Dev experience and ADRs**  
   - One-command dev startup MUST include local model runtime (or a mock) and sample jobs.  
   - Create initial Architecture Decision Records (ADRs) for key choices:
     - Runtime selection.  
     - DB layout for jobs and Flight Recorder.  
     - Capability model shape.  

10. **Security, resource, and UX bridges for mechanical work**  
    - Safety gates: wire `Guard` (secret/safety scan), `Container` (isolated exec), and `Quota` (resource limits) for mechanical/terminal jobs.  
    - Observability: expose `Profiler`/`Monitor` system metrics tied to job/session identifiers.  
    - Devops: route `Repo`/`Formatter`/minimal `Deploy` through the same capability/FR pathways used for other jobs.  
    - UX bridges: expose `Clipboard`/`Notifier` actions only with explicit capability/consent.
    - [ADD v02.136] Unified Tool Surface baseline (Â§6.0.2): route **all** tool invocations (local tool calling + MCP + MEX engines) through **Tool Gate** for schema validation, capability enforcement, payload limits (32KB rule), secret redaction, idempotency keys, and FR-EVT-007 (ToolCallEvent) logging.


11. **MCP skeleton and Gate (Target 1 + job/log plumbing)**  
    - [ADD v02.136] MCP MUST NOT introduce a second tool schema. MCP tool discovery/schemas are generated from the **Tool Registry** (HTC v1.0, Â§6.0.2) and every `tools/call` is enforced by **Tool Gate**.
    - Implement a minimal MCP client stack in the Rust coordinator, even if only exercised against a local stub server:
      - JSON-RPC transport and tool/resource discovery for at least one MCP server.  
      - Connection lifecycle tied to workspace/session where appropriate.  
    - Implement the MCP **Gate** interceptor (Section 11.3.2) as middleware around the MCP client:
      - Intercept `tools/call` requests, attach `job_id` / workflow run IDs and capability metadata.  
      - Enforce basic consent decisions and log them into Flight Recorder.  
      - Capture and log `tools/call` and `sampling/createMessage` traffic end-to-end, even when using a stub MCP server.  
    - Extend the AI Job Model to support MCP jobs:
      - Add a `transport_kind = "mcp"` discriminator and fields for `mcp_server_id` and `tool_name` where applicable.  
      - Ensure at least one test job profile uses MCP end-to-end (job â†’ MCP call â†’ response â†’ logs).  
    - Ensure Flight Recorder can represent MCP events using the canonical event shape in Section 11.3:
      - At least one MCP request/response path visible in the Flight Recorder UI.  
      - Clear correlation between a UI action, the AI job, and the MCP tool call(s) it triggered.  
12. **Calendar (local-only) as a Flight Recorder lens (no external sync)**  
   - Implement minimal **CalendarEvent** storage and rendering for a local-only calendar surface (manual create/edit local events).  
   - Implement **time-range selection** that queries Flight Recorder by interval overlap (ActivitySpans/SessionSpans) and renders an event â€œActivityâ€ tab (sessions, jobs, tool calls, models).  
   - Calendar writes MUST be applied via a local-target `calendar_sync` mechanical job (patch-set discipline + capability gate); UI remains read-only over authoritative state.  
   - Capabilities: `CALENDAR_READ_BASIC` / `CALENDAR_READ_DETAILS` for viewing; `CALENDAR_WRITE_LOCAL` for local edits.  



13. **[ADD v02.44] OSS governance baseline (build/release enforcement)**  
   - Enforce Â§11.7.4 requirements: every shipped OSS component MUST be present in the OSS Component Register with license + integration_mode + pinning policy.
   - Enforce the copyleft isolation rule: GPL/AGPL components MUST be `external_process` or `external_service` (never linked into the app binary).
   - Gate release builds on register completeness + policy compliance.

#### 11.7.5 Supply Chain Mechanical Gates (MEX v1.2)
- **Engine IDs**:
  - `engine.supply_chain.vuln`: Wraps `cargo-audit` / `npm audit` / `osv-scanner`.
  - `engine.supply_chain.sbom`: Generates CycloneDX / SPDX via `syft`.
  - `engine.supply_chain.license`: Wraps `scancode-toolkit` or `cargo-deny`.
- **Capability Requirements**: All supply-chain engines require `proc.exec` for their respective binaries and `fs.read:inputs`.
- **Artifact Schemas**:
  - `SupplyChainReport { kind: Vuln | SBOM | License, engine_version: String, timestamp: DateTime, findings: JSON }`.
- **Governance**: Any HIGH severity vulnerability or UNKNOWN license found during a `release` build MUST be emitted as a `BLOCK` problem in the diagnostics registry.

14. **[ADD v02.44] Deliverables PDF pipeline (MVP)**  
   - Implement `creative.deliverables.pdf_packaging` as a first-class Job path: Typst render + qpdf packaging.
      - Store output artifacts with provenance and deterministic output checks (byte-stable where feasible; otherwise stable hash policy).
      - **Implementation Note:** See Â§11.10.1 for binary resolution and job constraints.
   
   15. **[ADD v02.52] Bundle Export Framework v0 (MVP)**
      - **Debug Bundle export**: implement end-to-end exactly as specified in the Master Specâ€™s Debug Bundle section (no edits).
      - **Workspace Bundle export v0**: backup/transfer/fixture export for docs/canvases/tables + raw assets (when present).
   
   16. **[ADD v02.53] AI Rewrite UI Primitives (Human-in-the-Loop)**
      - Implement `DOC_REWRITE` workflow with "Diff" view and "Accept/Reject" gating.
      - Enforce "No Silent Edits" invariant via UI and Backend Gate.
      - Log rejected variations to Flight Recorder.
   
   
17. **[ADD v02.79] Photo Studio v0 (skeleton surface + governance wiring)**  
   - [ADD v02.79] Import JPEG/PNG/TIFF as Assets; generate thumbnails/previews as artifacts (no binaries in prompts/logs).  
   - [ADD v02.79] Minimal "edit recipe" placeholder stored as versioned sidecar (even if only exposure/WB placeholders).  
   - [ADD v02.79] Export via governed job path (artifact + export record; no direct file mutation).  
18. **[ADD v02.101] Spec Router and governance session log (MVP)**  
   - Implement `spec_router` job_kind and SpecRouterJobProfile with policy-bound mode selection.  
   - Emit SpecIntent and SpecRouterDecision artifacts with `capability_registry_version`.  
   - Auto-create or update Task Board and Work Packet entries for GOV_STRICT/GOV_STANDARD and append Spec Session Log entries.  
   - Enforce git-only safety commit behavior for git workflows; non-git workflows must not attempt a commit.  
   - [ADD v02.128] Implement **Spec Creation System v2.2.1** routes as **command-driven** intents (model never guesses): `/spec new`, `/spec extend`, `/spec refine`, `/spec check`, `/task` (see Â§2.6.8.13).  
   - [ADD v02.128] Persist and deterministically update `SPEC_INDEX.yaml` metadata for spec creation/refinement (tracks spec-creation-system version separately from Master Spec version; see Â§2.6.8.13).  
   - [ADD v02.128] Validate/enforce **Universal IDs + requirement grammar** on produced specs; validation failures MUST block Work Packet activation in GOV_STRICT/GOV_STANDARD and surface in Problems + Spec Session Log.  
   - [ADD v02.128] Run **overlap/conflict detection** before activation; emit a deterministic conflict report artifact and link it from SpecRouterDecision + Spec Session Log (block activation on hard conflicts in GOV_STRICT).  
   - [ADD v02.128] `/spec check` MUST run the **full rubric workflow** (rubric + second model + red-team pass) and emit a check report artifact (hash-linked, provenance-complete) visible in Operator Consoles.  

13. **[ADD v02.36] ACE Runtime (MVP) + Validator Pack (CI-gated)**  
   - For every AI job: emit and persist **ContextPlan** and per-call **ContextSnapshot** artifacts.  
   - Enforce the runtime validators (see Â§2.6.6.7.11) on every job; violations fail the job with normalized diagnostics.  
   - Debug Bundle export includes: ContextPlan, ContextSnapshots, validator outcomes, and evidence refs used.
   - [ADD v02.138] Front End Memory System (FEMS) v0: compile and inject bounded `MemoryPack` (â‰¤500 tokens) per call; generate `MemoryWriteProposal`s (no implicit writes); route procedural memory through DCC review; emit `FR-EVT-MEM-*`; add FEMS-EVAL-001.


14. **[ADD v02.36] Terminal LAW (minimal slice) promoted to MUST**
   - Terminal command execution MUST NOT bypass capabilities/consent, Workflow Engine, Gate, or Flight Recorder.
   - Every terminal run is bound to `job_id` / `workflow_id` and records scrubbed context + artifact references as provenance.
   - **[ADD v02.63] ModelProfile/Routing/SafetyProfile clarity:** Phase 1 runtime integration MUST define a concrete profile schema (id, role, safety policy, routing notes) for models used in MVP.

15. **[ADD v02.36] Capability single-source-of-truth + unknown-capability validator**  
   - Resolve all capability declarations via job profiles (`capability_profile_id`) into a single normalized set used by Gate + UI.  
   - Unknown/undeclared capability requests fail fast and surface an explanation in Problems/Evidence.


16. **[ADD v02.38] Canvas Typography + Font Packs (Design Pack + Font Registry)**  
   - Bundle offline font packs in app resources and ship licensing artifacts (per-font license files + THIRD_PARTY_NOTICES).  
   - Implement backend-owned Font Registry (bootstrap pack, rebuild manifest, list families, import/remove fonts).  
   - Deterministic font loading via `FontFace` / `document.fonts.ready`; no â€œflash of fallbackâ€ on first render.  
   - Sanitize font names and file paths to prevent CSS injection; UI never crawls font directories directly.  
   - Canvas text objects can select bundled fonts; export (PNG/SVG) preserves selected font.
   - **Implementation Note:** See Â§11.10.2 for runtime root and CSP policy.

17. **[ADD v02.42] Iterative Deepening (snippet-first) â€” MVP policy scaffolding**  
   - Enforce snippet-first retrieval policy for any retrieval-capable job in Phase 1 (local workspace search, MCP reads): start with bounded snippets only; no full-document/page injection paths. (See Â§2.6.6.7.5.2.)  
   - Implement SEARCH â†’ READ separation in adapters (stubs acceptable in Phase 1): `search(query) -> snippets` and bounded `read(section_selector) -> excerpt`.  
   - Emit and log EvidenceSnippets with (minimum) `fetch_depth = snippet`, `trust_class`, a resolvable citation/SourceRef, and a 1â€“2 line relevance rationale; enforce per-step retrieval budgets and anti-context-rot rules (dedupe, exclude tool logs).  

18. **[ADD v02.52] Retrieval Correctness & Efficiency (ACE-RAG-001) â€” Phase 1 plumbing**
   - Emit and persist `QueryPlan` and `RetrievalTrace` for every retrieval-backed model call; link both to the `ContextSnapshot` / `PromptEnvelope`.
   - Implement deterministic `normalized_query_hash` (sha256 of normalized query text) and record it in `RetrievalTrace`.
   - Compute and record `CacheKey` for cacheable stages (even if cache is initially a stub); log cache hit/miss per stage.
   - Enforce hard budgets at runtime:
     - `RetrievalBudgetGuard` (evidence tokens/snippet counts/read caps; deterministic truncation with flags).
     - `CacheKeyGuard` (strict mode requires cache key computation + logging).
   - Add a minimal Semantic Catalog registry (built-in) so routing does not depend on â€œLLM guessingâ€ store/tool names.
   - Operator Consoles MUST deep-link: Job â†’ Model Call â†’ QueryPlan/Trace â†’ Evidence items (SourceRefs/ArtifactHandles) without opening raw documents by default.


19. **[ADD v02.67] Atelier Lens Runtime v0.1 (Role claiming + dual-contract extraction)**
   - Implement `ATELIER_CLAIM` as a mechanical job (capability-gated; logged) that emits:
     - `RoleScore[]` (dense distribution over all registered roles)
     - `RoleClaim[]` (thresholded multi-claim set used to schedule deep passes)
     - `RoleGlance[]` (cheap all-roles glance; **SHOULD** cover every role; may record `found=false` without blocking)
   - Implement a versioned `AtelierRoleSpec` registry and enforce dual contract ids:
     - `ROLE:<role_id>:X:<ver>` (extraction) â†’ `RoleDescriptorBundle`
     - `ROLE:<role_id>:C:<ver>` (creative) â†’ `RoleDeliverableBundle`
   - Implement Lens job profiles (all through Workflow Engine + Flight Recorder):
     - `ATELIER_ROLE_EXTRACT`, `ATELIER_ROLE_COMPOSE`, `ATELIER_STATE_MERGE`, `ATELIER_GRAPH_SOLVE`, `ATELIER_CONCEPT_RECIPE`
   - Evidence discipline: every claimed field MUST have `EvidenceRef` (bbox/page/span/time-range) and must pass `ATELIER-LENS-VAL-003` (missing evidence is FAIL).
   - MVP role set MUST include at least one Finishing role contract (Editor or Color) alongside pre/prod roles.
   - Wire Lens validators `ATELIER-LENS-VAL-007..011` as required gates for Lens runs (merge determinism/conflict accounting, recipe validity, DAG validity, dependency completeness).

20. **[ADD v02.68] Mechanical Extension v1.2 runtime contract (MEX) â€” Phase 1 foundations**
   - Implement Engine PlannedOperation (`schema_version=poe-1.0`) + EngineResult envelopes; validate with `G-SCHEMA`.
   - Implement the required gate pipeline for engine jobs: `G-CAP`, `G-INTEGRITY`, `G-BUDGET`, `G-PROVENANCE`, `G-DET`; log every decision/outcome to Flight Recorder and surface denials in Problems.
   - Implement the engine registry loader (`mechanical_engines.json`) and adapter resolution; capabilities are default-deny and must be explicitly granted/recorded.
   - Implement **Conformance Harness v0** and require at least **3 engines** to pass conformance (recommended: Context/Sandbox/Wrangler or equivalents) before enabling additional engines.
   - Enforce artifact-first I/O: any payload >32KB uses artifact handles; outputs are artifacts with SHA-256 + sidecar provenance manifests; no direct filesystem bypass (materialize-only).
   - Canonical references: Â§6.3.0 + Â§11.8.
- [ADD v02.79] Import a small photo set â†’ open Photo Studio â†’ apply a minimal recipe stub â†’ export a derivative â†’ inspect provenance in Job History / Flight Recorder.


21. **[ADD v02.102] Phase 1 closure: storage backend portability work packets (CX-DBP-030)**
   - Phase 1 cannot close until the following are complete and validated (see Section 2.3.13.5 [CX-DBP-030]):
     - WP-1-Storage-Abstraction-Layer
     - WP-1-AppState-Refactoring
     - WP-1-Migration-Framework
     - WP-1-Dual-Backend-Tests

22. **[ADD v02.102] CapabilityRegistry single source of truth (WP-1-Capability-SSoT)**
   - Ensure CapabilityRegistry resolves scoped requests against axis-level grants and produces deterministic allow/deny results for Gate + Spec Router (see Section 11.1.6 validator requirement).

23. **[ADD v02.102] Global Silent Edit Guard (WP-1-Global-Silent-Edit-Guard)**
   - Implement StorageGuard validation: AI writes must include job/workflow context and persist MutationMetadata; reject silent AI edits with `HSK-403-SILENT-EDIT` (see Section 2.9.2).

24. **[ADD v02.102] Phase 1 final gap closure details (Section 11.10)**
   - `creative.deliverables.pdf_packaging` discovers `typst` and `qpdf` via PATH or env vars `HANDSHAKE_TYPST_BIN` / `HANDSHAKE_QPDF_BIN`.
   - Fonts runtime root is `{APP_DATA}/fonts/` and Tauri `asset:` protocol is restricted to that directory; bootstrap copies bundled font pack(s) to `{APP_DATA}/fonts/bundled/`.
   - On startup, detect Ollama via `http://localhost:11434/api/tags` and enable `OllamaClient` by default when present.

25. **[ADD v02.103] Response Behavior Contract (Diary ANS-001)**
   - Implement the Response Behavior Contract (Section 2.7) for governed assistant responses: intent confirmation, mode context, operation plan, proactive surfacing, and next steps.
   - Work modes (Strict/Free/Fasttrack/Brainstorm/Data) must deterministically constrain allowed operations and determinism requirements.
   - [ADD v02.121] Persist frontend interactive chat sessions to `{APP_DATA}/sessions/<session_id>/chat.jsonl` (Â§2.10.4) including raw chat text and ANS-001 payload per frontend assistant message.
   - [ADD v02.121] UI: ANS-001 is hidden inline by default with per-message expand + global show-inline toggle, and is available in a side-panel timeline viewer (Â§2.7.1.7).

- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Runtime modes: DOCS_ONLY vs AI_ENABLED; enforce min_ready_models=1 in AI_ENABLED.  
  - Multi-model orchestration primitives: ExecutionMode, model readiness, swap/escalation.  
  - File-scope lock enforcement for concurrent WPs/MTs (no overlapping IN_SCOPE_PATHS).  
  - Work Profile routing: ParameterClass + â€œlargest-firstâ€ selection + model performance telemetry scoring.  
  - RoleExecutionIdentity logging per output (role, model_id, backend, parameter_class, cloud_strength, session_id).  
  - Role Mailbox persistence taxonomy (MailboxKind) and non-authority boundary.  
  - HSK_STATUS single-line lifecycle marker; shown after gate output.  
  - Softblock/failstate code registry (CX-MM-xxx) for model readiness, lock conflicts, swap failures.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - DCC surface stub in UI (kanban-first), with panels: **Work Packets**, **Workspaces (worktrees)**, **Sessions**, **Approvals**, **VCS** (status/diff), **Search**.  
  - Workspace registry: list/add/open/close worktrees; import existing worktrees; link workspace â†’ `wp_id`/`mt_id` (no authority drift; Locus remains canonical).  
  - Execution Session Manager: show active sessions (role/model/backend), workspace binding, capability grants; deep-link to Job History + Flight Recorder timeline.  
  - Approval Inbox: render pending capability requests from the Workflow Engine; support approve-once / approve-for-job / approve-for-workspace / deny; log all decisions to Flight Recorder.  
  - VCS review loop: show `version.status` + `version.diff`; commit flow uses `version.commit(paths[])` with commit message as an artifact; **no implicit staging**; dangerous ops (reset/clean/rebase) require same-turn explicit approval.  
  - Objective Anchor Store (minimal): create/view anchors and handoffs linked to `wp_id`/`mt_id`; anchors are **non-authoritative** and MUST NOT override Locus status.  
  - Storage foundation: ship `.handshake/workspace.json` schema v1.0 and `devcc.db` schema v1; both local-first; no secrets in committed artifacts; default ignore patterns documented.  
  - Conversation timeline (Phase A baseline): ingest Handshake-native conversations (roles + Flight Recorder events) into DCC timeline; adapter contract stubbed for later external sources.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Runtime contract hook: reserve and accept `settings.exec_policy` (optional) with deterministic downgrade semantics (requested vs effective) (Â§2.5.2.1, Â§4.5.5).  
  - Work Profile hook: per-role compute presets + separate approximate knob with waiver requirement (`hsk.work_profile@0.6`) (Â§4.3.7.5, Waiver Protocol [CX-573F]).  
  - Observability: Flight Recorder event `llm_exec_policy` (FR-EVT-LLM-EXEC-001) + referenced trace artifact format `hsk.layerwise_trace@0.1` (Â§11.5.11).  
  - UI/config surface: operator can select per-role `speed_preset` (standard / fast_exact / fast_approx) and cannot enable approximate without a waiver reference.  
  - Default posture remains **standard exact**; no planner auto-enables approximation.



26. **Loom MVP (Heaper-style library surface)** [ADD v02.130]  
   - Implement the LoomBlock entity (Â§2.2.1.14) and LoomEdge relational model (Â§2.3.7.1) with CRDT-safe UUID references (rename-stable).
   - Implement the four Loom views (Â§7.1.4.3; Â§10.12): **All**, **Unlinked**, **Sorted**, **Pins** with grid/list toggle and required filters.
   - Implement file import into Loom (folder drag/drop + file picker) with SHA-256 dedup (**FR-EVT-LOOM-006**) and stable identity independent of filename (**[LM-BLOCK-004]**).
   - Implement Tier-1 preview generation (thumbnails, lightweight proxies where needed) as Mechanical jobs; attach `thumbnail_asset_id` / `proxy_asset_id` to LoomBlocks.
   - Integrate inline @mentions + #tags in the Rich Text Editor (Â§7.1.1.7) and ensure tag targets are `TAG_HUB` LoomBlocks.
   - Provide LoomBlock detail view with backlinks panel (DerivedContent) and context snippets (**[LM-BACK-001]**..**[LM-BACK-003]**).
   - Implement Tier-1 Loom search (SQLite FTS over `derived.full_text_index`) with basic facets (content_type, mime, date, tag, mention).
   - Emit and surface FR-EVT-LOOM-001..012 (Â§11.5.12) in Operator Consoles / Job History.

27. **Handshake Stage MVP (governed browser + Stage Apps)** [ADD v02.131]  
   - Implement the Stage Host surface: session + tab model with strict origin isolation (External Web vs Stage Apps) and per-session cookie/storage boundaries (no bleed).  
   - Implement `handshake-stage://` scheme loader for Stage Apps with bundle integrity (SHA-256) + CSP defaults; Stage Apps MUST NOT be able to navigate to arbitrary `http(s)` without explicit user action and host allowlist enforcement.  
   - Implement Stage Bridge API (request/response + events) with capability-gated methods: `stage.runtime.getContext`, `stage.jobs.enqueue`, `stage.workspace.createDocumentFromArtifact`, `stage.workspace.applyPatchSet`, `stage.ui.requestApproval`, `stage.ui.notify` (Phase 1 subset acceptable per Stage spec).  
   - Implement evidence-grade web capture workflow `stage.capture_webpage.v1` (Archivist) producing `artifact.snapshot` bundles (HTML + assets + screenshots) and provenance manifests; enforce artifact-first I/O (>32KB params via input artifacts).  
   - Implement selection clipping `stage.clip_selection.v1` to convert selected DOM ranges â†’ `artifact.clip` (markdown + source selectors) with traceable links to the originating snapshot.  
   - Provide PDF viewing + import controller: attach PDFs as artifacts and enqueue `stage.import_pdf.v1` producing a document stub; **Docling-backed structured conversion is Phase 2** (Â§7.6.4).  
   - Deliver Stage Apps Phase 1 set: `com.handshake.stage.clip` (web â†’ doc/clip), `com.handshake.stage.pdf_import` (PDF view/import controller), and `com.handshake.stage.playground` (prompt/spec playground with controlled job spawn).  
   - Deliver 3D Mechanical Assist Pack Phase 1 slice: `stage.3d.import_gltf.v1`, `stage.3d.extract_scene_ir.v1`, `stage.3d.validate_gltf.v1`; add a read-only 3D viewport/inspector to render `artifact.3d_asset` and display `artifact.scene_ir` + validator/physics reports.  
   - Ship a Stage security harness that asserts: bridge injection only for Stage Apps; external web cannot call bridge; private-network navigation is denied by policy; and every privileged Stage Bridge call is logged in Flight Recorder with allow/deny + capability IDs.  


28. **Multi-Session Orchestration: ModelSession + Scheduler:** Add `ModelSession` as the persisted unit of multi-turn orchestration; add `model_run` job_kind and a session scheduler with queueing, cancellation, and concurrency limits.
29. **ModelSession persistence layer:** Store sessions + message artifacts in local workspace DB (Phase 1) with explicit content-hash discipline.
30. **Session spawn contract (OpenClaw pattern):** Implement `SessionSpawnRequest` + `SessionSpawnResponse`, depth limits, per-session spawn caps, and Role Mailbox announce-back (SessionAnnounceBack) with summary artifacts.
31. **Session-scoped capability tokens:** Add `capability_token_ids` on ModelSession and enforce deny-by-default session-scoped capability intersection in Tool Gate.
32. **Tool calling + structured output adapters:** Implement provider adapters that translate the Unified Tool Surface registry into provider tool schemas and back (no parallel ToolDefinition schema).
33. **Workspace safety boundaries for parallel writes:** Implement worktree isolation or file locking policy for concurrent sessions on the same workspace, with deterministic conflict handling.
34. **Crash recovery / resume:** Add session checkpointing and idempotent recovery flow for interrupted `model_run` jobs.
35. **DCC multi-session steering panel:** Display session list, state machine, spawn tree, cost/budget per session, and controls (pause/resume/cancel).
36. **Flight Recorder coverage:** Register and emit FR-EVT-SESS-*, FR-EVT-SESS-SCHED-*, FR-EVT-SESS-SPAWN-*; add `model_session_id` correlation to base event schema.
37. **Session observability bindings:** Implement `ModelSessionSpan` creation/closure per ModelSession lifecycle and ActivitySpan linkage for `model_run` and tool calls; session-wide queries MUST work via `model_session_id` even without spans.

**Vertical slice**
- **Core loop**
  - Start the app and open a sample document.
  - Select text and trigger â€œRewrite selectionâ€.
  - See the updated text in the document.
  - Open Job History and locate the corresponding job with correct status and metadata.
  - Inspect logs and traces that show the model call, workflow execution, and any errors.
  - [ADD v02.136] Trigger one governed tool call (e.g., `engine.context.search` or `engine.sandbox.exec`) and verify: Tool Gate decision recorded, FR-EVT-007 (ToolCallEvent) visible, artifacts referenced (no large blobs in context), and the call is linked to the originating job/workflow.
  - Open a Canvas and create a text object.
  - Choose a bundled font family; ensure it renders deterministically (no fallback flash).
  - Save, restart, reopen; typography selection persists.
  - Export Canvas to PNG/SVG; exported result preserves the chosen font.
  - Export a deliverable PDF (Typst + qpdf); exported result is reproducible (byte-stable or stable hash policy).
  - [ADD v02.52] Trigger "Ask about this document" (RAG-aware Q&A) and verify Evidence view exposes QueryPlan + RetrievalTrace ids/hashes and bounded spans (with truncation flags if budgets hit).
  - [ADD v02.52] Export a Workspace Bundle for a non-trivial workspace and verify: manifest + doc/canvas/table snapshots + export report.
  - [ADD v02.52] Export a Debug Bundle for one AI job and verify required files + SAFE_DEFAULT redaction mode.
  - [ADD v02.101] Run Spec Router on a prompt, verify SpecIntent/SpecRouterDecision artifacts, Task Board + Work Packet creation, and a Spec Session Log entry; if the workspace is git-backed, verify safety commit behavior.
  - [ADD v02.128] Run `/spec new`, `/spec extend`, `/spec refine`, and `/spec check` end-to-end and verify: command-driven routing (no heuristic guessing), `SPEC_INDEX.yaml` updated deterministically, Universal IDs/requirement grammar validation, overlap detection report emitted, and `/spec check` rubric+2nd-model+red-team report linked in Spec Session Log.

- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**  
  Run WP-A and WP-B in parallel on two models; WP-B attempts overlap â†’ blocks; WP-A completes; operator swaps WP-B to larger model after an escalation failstate.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**  
  Run one WP end-to-end using DCC: create/link worktree â†’ run job â†’ approval prompt â†’ review diff â†’ commit â†’ mark MT done; confirm Task Board sync and Flight Recorder evidence.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**  
  Operator selects `fast_exact` for `worker`, runs a WP/MT, and sees requested vs effective policy in FR + UI; then toggles approximate with a waiver and sees `llm_exec_policy` + trace reference emitted.


- **Loom loop** [ADD v02.130]
  - Import a folder containing mixed media (images, PDFs, audio, video) into Loom.
  - Verify dedup: importing the same file twice routes to the existing LoomBlock (no duplicate).
  - Browse **All** in grid mode; open an item; add a short annotation; create an @mention and a #tag.
  - Confirm the item disappears from **Unlinked** immediately after the first link/tag.
  - Open the #tag hub and confirm backlinks show context snippets.
  - Run Loom search (FTS) with at least one filter facet; open a result and confirm Flight Recorder events exist for import, preview generation, edge creation, view query, and search.

- **Stage loop** [ADD v02.131]
  - Open Handshake Stage and start a new isolated session.
  - Navigate to an external web page; capture it via `stage.capture_webpage.v1`; verify an `artifact.snapshot` bundle is stored with hashes + provenance.
  - Clip a selection; verify `artifact.clip` links back to snapshot selectors; create a workspace document from the clip via Stage Bridge (`stage.workspace.createDocumentFromArtifact`) with approvals logged.
  - Open a PDF in Stage; run `stage.import_pdf.v1`; verify PDF bytes are preserved as an artifact and the resulting document stub links to it.
  - Import a glTF/GLB; run `stage.3d.import_gltf.v1` + `stage.3d.validate_gltf.v1`; open the 3D viewport and view `artifact.scene_ir` + validator reports.
  - Confirm Stage Bridge calls are capability-gated and show allow/deny decisions + linked jobs in Operator Consoles / Flight Recorder.


**Key risks addressed in Phase 1**
- [ADD v02.130] Loom integrity/performance risks: UUID-stable inline tokens (no text-based links), anchor drift during edits, dedup false positives/negatives, and preview-generation throughput (Tierâ€‘1 thumbnails) must not degrade core UI responsiveness.
- [ADD v02.131] Stage security/evidence risks: origin isolation bugs (External Web â†” Stage Apps), session bleed (cookies/storage), private-network access, and capture-evidence integrity drift (missing hashes/provenance) must be prevented by policy enforcement + security harness + always-on Flight Recorder logging.
- [ADD v02.136] Tool surface drift + prompt-injection risk: if local tools, MCP tools, and mechanical engines have **different** schemas/logging, agents will find bypass paths and tool outputs may smuggle instructions. Mitigation: single Tool Registry + Tool Gate, strict payload caps, artifact-first I/O, and mandatory FR-EVT-007 (ToolCallEvent) logging.
- [ADD v02.138] Memory poisoning / drift risk: untrusted session text or tool outputs promoted into procedural memory (or oversized `MemoryPack`s) can degrade correctness and increase drift vectors. Mitigation: FEMS write gates + human review for procedural memory, hard pack budgets (â‰¤500 tokens), and replay-grade logging (`FR-EVT-MEM-*`).



- [ADD v02.47] Prevent MVP scope creep: charts/dashboards/decks are deferred to Phase 2+ (no partial implementation in Phase 1).

- Calendar/time-lens introduces hidden writes or breaks deterministic provenance (must remain patch-set + logged + capability-gated).


- AI Job Model and Workflow Engine are too complex or too weak for real usage.  
- Observability (Flight Recorder, metrics, traces) is not wired end-to-end.
- Operator cannot produce deterministic bug evidence (Debug Bundle + Problems) â†’ LLM coding loop stalls.
  
- Capability and consent models are unclear or easily bypassed.  
- Secret/resource leakage or runaway jobs if Guard/Container/Quota are absent or unenforced.
- MCP Gate and MCP-ready job/log plumbing are bolted on too late, forcing breaking changes to job/log schemas or inconsistent consent/logging across tools.

- [ADD v02.101] Prompt-to-spec routing is not policy-bound, causing governance drift or inconsistent artifacts; mitigated by Spec Router policy schema and session log.
- [ADD v02.139] Spec hallucination / non-executable spec risk: models invent non-existent capabilities/tools or omit concrete validation hooks; mitigated by CapabilitySnapshot allowlist enforcement + deterministic SpecPromptPack compilation + SpecLint preflight (SPEC-VAL-*).
- [ADD v02.128] Spec Creation System gaps (missing command routing, rubric discipline, overlap detection) cause inconsistent specs and collisions across WPs; mitigated by command-driven routes + mandatory `/spec check` + deterministic overlap/conflict reports.
- [ADD v02.128] Missing Universal IDs / requirement grammar causes ambiguous references and audit failures; mitigated by v2.2.1 Universal ID system + validators.
- [ADD v02.101] Safety commit logic applied outside git workflows can destroy non-git state; mitigated by explicit VersionControl gating and policy rules.


- [ADD v02.36] "Auditable AI" is non-real without enforced ContextPlan/ContextSnapshot + runtime validators (not just logging).
- [ADD v02.36] Terminal is a capability-bypass vector unless fully routed through Gate/Workflow/Flight Recorder (Terminal LAW).
- [ADD v02.38] Canvas editor choice (Excalidraw) may constrain deterministic font loading/text editing; validate compatibility with Â§10.6 before implementing typography acceptance criteria.


- [ADD v02.44] OSS license posture drift: accidental in-process use of GPL/AGPL, or missing/incorrect OSS Register entries, undermines auditability and distribution.
- [ADD v02.44] Mixed-license tools (e.g., ExifTool dual license; Czkawka with GPL sub-app) require strict `external_process` posture and explicit capability gating.
- [ADD v02.49] Unmanaged OSS/tool outputs (random files) create untraceable side effects and break reproducibility; mitigated by manifest + SHA-256 + materialize-only semantics.
- [ADD v02.49] Disk bloat / cache drift from derived outputs; mitigated by TTL + pinning + deterministic GC with visible reports.

- [ADD v02.52] Retrieval remains opaque / non-replayable (answers cannot be explained or reproduced); mitigated by mandatory QueryPlan + RetrievalTrace + deterministic tie-breaks and persisted selection inputs.
- [ADD v02.52] Token budgets silently drift upward (context bloat, slower answers, worse correctness); mitigated by hard BudgetGuard enforcement + deterministic truncation flags + CI fixtures.
- [ADD v02.52] No redaction-safe evidence packet for LLM coders/validators (Debug Bundle).
- [ADD v02.52] Data lock-in / inability to back up workspace state early (Workspace Bundle).
- [ADD v02.52] Accidental leakage through export (exportable=false enforcement + policy gating).


- [ADD v02.67] Atelier role overlap/contradictions can produce nondeterministic outputs unless merge/arbitration is explicit; mitigated by `SceneState` + `ConflictSet` + `ATELIER_STATE_MERGE` and validators `ATELIER-LENS-VAL-007/008`.
- [ADD v02.67] Conceptual Mode vectors are not replayable/auditable if they remain UI-only; mitigated by typed `ConceptRecipe` and `ATELIER-LENS-VAL-009`.
- [ADD v02.67] Nested Production remains prose without enforceable dependencies; mitigated by `AtelierProductionGraph` + `ATELIER_GRAPH_SOLVE` and validators `ATELIER-LENS-VAL-010/011`.
- [ADD v02.67] â€œEveryone finds somethingâ€ can degrade relevance and blow up compute; mitigated by `RoleGlance` (cheap, non-blocking) + thresholded `RoleClaim` (top-k deep passes) + explicit per-run budgets.


- [ADD v02.68] Mechanical jobs become an ungoverned â€œescape hatchâ€ (bypass, side effects, missing provenance); mitigated by v1.2 envelopes + required gates + registry + conformance harness, with denials visible in Problems/Flight Recorder.
- [ADD v02.68] Unbounded mechanical outputs (logs/large blobs) pollute context and break artifact discipline; mitigated by artifact-first I/O (>32KB rule) + output caps + G-BUDGET/G-PROVENANCE enforcement.
- [ADD v02.79] Scope explosion (Lightroom/Affinity-class) â†’ enforce Phase 1 boundary: â€œskeleton only; no RAW/masks/layers/AIâ€.
- [ADD v02.79] UI bypassing job runtime â†’ all Photo Studio actions MUST enqueue jobs (â€œsingle execution authorityâ€).


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Conflicting edits across parallel WPs (prevented by strict file-scope locks).  
  - Governance bypass via side channels (prevented by non-authority role mailbox + canonical artifact rules).  
  - Opaque multi-model state (solved by HSK_STATUS + FR correlation).

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - Governance bypass via â€œUI direct execâ€ (prevented by forcing all actions through jobs + approvals + Flight Recorder).  
  - Lost context and brittle handoffs between sessions/models (reduced via anchors + handoff records + session identity).  
  - Parallel work collisions across WPs/MTs (mitigated via worktree-per-WP discipline + Locus lock semantics + visibility).

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Prevent silent quality regressions (approximate execution must be explicit + waived).  
  - Prevent â€œunknown computeâ€ in audits (requested vs effective policy always logged).  
  - Prevent privacy leaks from high-volume traces (no token IDs; no raw text by default).
- [ADD v02.137] Uncontrolled parallel model spawning (budget blow-ups / runaway sessions); mitigated by spawn limits + per-session budget + scheduler backpressure + kill switch.

**Acceptance criteria**
- [ADD v02.130] Loom: importing files creates LoomBlocks; dedup prevents duplicates; All/Unlinked/Sorted/Pins operate correctly; @mentions/#tags create edges; backlinks update with snippets; Tierâ€‘1 search works with facets; FR-EVT-LOOM-* are emitted and visible in Operator Consoles.
- [ADD v02.131] Stage: External Web and Stage Apps run in isolated sessions/tabs (no cookie/storage bleed); Stage Bridge is injected only on `handshake-stage://` and denies calls from External Web; every Bridge allow/deny is visible in Flight Recorder/Operator Consoles.
- [ADD v02.131] Stage capture/import: `stage.capture_webpage.v1` produces `artifact.snapshot` bundles with manifests + SHA-256; `stage.clip_selection.v1` produces `artifact.clip` linked to snapshot selectors; PDF import preserves bytes as artifacts and produces a doc stub; glTF import/validate produces `artifact.scene_ir` + validator reports; all outputs survive restart and are discoverable.
- [ADD v02.139] Spec Router: compiled PromptEnvelope hashes + SpecPromptPack id/sha + CapabilitySnapshot ref are emitted for every spec job; SpecLint gate (G-SPECLINT) blocks progression on failure and produces a SpecLintReport artifact linked in Job History + Spec Session Log; SpecLint runs in CI for `.SPEC/` outputs (v2.2.1) and for GOV_STANDARD/STRICT Feature/Workflow/Integration specs.
- [ADD v02.142] Runtime visibility seed: Appendix 12.3 includes capability slices/runtime visibility rows for Calendar temporal correlation, Calendar orchestrated mutation, unified local/cloud governed tool calling, Locus execution correlation, Loom retrieval library, and Stage capture/import pipeline; Appendix 12.6 links those rows into force-multiplier edges visible to operator/runtime surfaces.
- [ADD v02.143] Primitive index seed: Appendix 12.4 is normalized to coverage-driven rows and includes concrete runtime/job/tool/frontend/operator primitives plus stub-linked gap tracking for Calendar, Locus, Loom, AI-ready retrieval, Spec Router, Stage-adjacent media flows, and unresolved Mail/Studio runtime coverage.
- [ADD v02.144] Second-pass coverage sweep: Appendix 12 explicitly covers Canvas, Docs & Sheets, Project Brain, Thinking Pipeline, Context Packs, Semantic Catalog, Skill Bank, ASR, Charts & Dashboards, Presentations / Decks, Studio, and richer Mail / DCC runtime projection; unresolved embodiments are linked to detailed stubs before matrix expansion continues.
- [ADD v02.145] Third-pass coverage sweep: Appendix 12 explicitly covers Model Session Orchestration, Cloud Escalation Consent, MEX Runtime, and the typed runtime/export/filter/session contracts behind AI Job Model, Workflow Engine, Flight Recorder, Diagnostics, Operator Consoles, and Unified Tool Surface; unresolved embodiments remain stub-linked before wider matrix expansion continues.
- [ADD v02.146] Deepening sweep: Appendix 12 explicitly covers AI-ready index artifacts/status enums, AI Job and Flight Recorder UI/operator surfaces, Role Mailbox export/transcription contracts, richer Locus status/query primitives, Loom core block-edge graph primitives, Docs/Tiptap editor embodiment, and direct job-consent / MEX-Flight Recorder interaction edges.
- [ADD v02.147] Ownership sweep: Appendix 12 now binds high-signal orphan primitives for capability snapshots/consent scope, AI job query-update envelopes, debug bundle manifest/validation/status contracts, storage artifact/index/retention contracts, and operator export/query responses; projection/export force multipliers are explicit in the interaction matrix instead of remaining implicit novel ideas.
- [ADD v02.148] Ownership-reduction deepening: Appendix 12 explicitly attaches Stage/media session-auth contracts, MultiModelSession, debug export inventory/request/target contracts, and shared RetentionReport ownership; Stage?Media Downloader, Model Session?AI Job, and Debug Bundle?Storage Portability edges are explicit.
- [ADD v02.150] Backend matrix expansion: Appendix 12 now makes workflow?debug/locus, job?debug, consent?debug, calendar?debug, stage?debug, and media?debug/storage correlations explicit; unresolved combo implications are materialized as stub work packets instead of remaining matrix-only ideas.
- [ADD v02.151] Backend export/evidence/portability sweep: Appendix 12 now makes Role Mailbox?Flight Recorder/storage, AI-Ready Data?Flight Recorder/storage, and Workflow Engine?storage portability correlations explicit; unresolved mailbox/AI-ready debug-bundle bridges and calendar-mailbox correlation are materialized as stub work packets.
- [ADD v02.152] Backend orchestration/projection/replay sweep: Appendix 12 now makes Spec Router?Flight Recorder/storage, Locus?Flight Recorder, and MCP/MEX?Debug Bundle correlations explicit; unresolved spec-router portability, Locus debug-bundle, and MCP/MEX evidence-export bridges are materialized as stub work packets.
- [ADD v02.153] Backend capability/diagnostic sweep: Appendix 12 now makes Capabilitiesâ†’Flight Recorder, Workflow/Spec Routerâ†’Capabilities, MCPâ†’Flight Recorder, and Diagnosticsâ†’Debug Bundle correlations explicit; unresolved cloud-consent evidence portability is materialized as a stub work packet.
- [ADD v02.154] Backend governance/export reciprocity sweep: Appendix 12 now backfills Governance Pack and Workspace Bundle as explicit backend export surfaces, adds Governance Packâ†’Workflow/Capabilities/Flight Recorder/Storage Portability edges plus Workspace Bundleâ†’Flight Recorder/Storage Portability edges, and reuses existing governance/bundle stubs for unresolved delivery work.

- [ADD v02.155] Calendar-centered backend sweep: Appendix 12 deepens Calendar as a sync-state, export-mode, capability, AI-job mutation, and scope-hint routing surface; adds Calendarâ†’Storage Portability / Capabilities & Consent / AI Job Model / Spec Router edges; and keeps mailbox/Locus/export bridges stub-backed until dedicated backend joins exist.

- [ADD v02.156] Knowledge/retrieval pillar backend sweep: Appendix 12 deepens Project Brain, Semantic Catalog, Context Packs, Loom, and AI-Ready Data as explicit backend retrieval contracts; adds Project Brainâ†’AI-Ready Data, Semantic Catalogâ†’Spec Router, Context Packsâ†’Storage Portability, and Loomâ†’Storage Portability edges; and materializes unresolved Loom portability delivery as a dedicated stub track.

- [ADD v02.157] Distillation/context/spec-router backend sweep: Appendix 12 deepens Skill Bank, Context Packs, ACE Runtime, Micro-Task Executor, and Spec Router as one backend learning substrate; adds ACE Runtimeâ†’Context Packs, Micro-Task Executorâ†’Skill Bank, Context Packsâ†’Flight Recorder, Skill Bankâ†’Storage Portability, and Spec Routerâ†’Context Packs edges; and materializes unresolved Context Pack recorder visibility as a dedicated stub track.

- [ADD v02.158] Stage/Studio/Media/ASR backend pillar sweep: Appendix 12 deepens ASR, Media Downloader, Stage, Studio, and Atelier/Lens as artifact-lineage-aware backend surfaces; adds ASRâ†’Flight Recorder, ASRâ†’Storage Portability, Media Downloaderâ†’ASR, and Stageâ†’Storage Portability edges; and materializes unresolved Stageâ†’ASR transcript lineage as a dedicated stub track.
- [ADD v02.159] Correlation/projection backend pillar sweep: Appendix 12 clarifies Dev Command Center as the control/projection umbrella, keeps Operator Consoles as the specialized evidence/debug cluster, adds Dev Command Centerâ†’Flight Recorder, Dev Command Centerâ†’Debug Bundle, and Role Mailboxâ†’Dev Command Center edges, and keeps weaker Locus/Role Mailbox/Calendar bundle bridges stub-backed.

- [ADD v02.160] Dev Command Center control-plane backend sweep: Appendix 12 deepens Dev Command Center as the governed control surface for workflow runs, artificial intelligence jobs, capability decisions, model sessions, and work packet or worktree bindings; adds Dev Command Center to Workflow Engine, Dev Command Center to Artificial Intelligence Job Model, Dev Command Center to Capabilities and Consent, and Dev Command Center to Model Session Orchestration edges; and reuses the existing Dev Command Center, workflow projection correlation, and consent audit projection backlog instead of creating duplicate control-plane stubs.

- [ADD v02.161] Dev Command Center evidence-and-replay backend sweep: Appendix 12 deepens Dev Command Center as the governed evidence and replay projection surface for Governance Pack export, Workspace Bundle export, diagnostics queries, and bounded workflow-linked evidence packaging; adds Dev Command Center to Governance Pack, Dev Command Center to Workspace Bundle, and Dev Command Center to Diagnostics Schema edges; and reuses the existing governance, workspace bundle, and diagnostics backlog instead of creating duplicate export or replay stub families.
- [ADD v02.162] Dev Command Center work-orchestration backend sweep: Appendix 12 deepens Dev Command Center as the governed projection surface for tracked Work Packet state, Task Board sync freshness, Micro-Task summaries, ready-query status, workflow-linked work packet activation, and parallel model session occupancy; adds Dev Command Center to Locus Work Tracking, Dev Command Center to Micro-Task Executor, and Model Session Orchestration to Locus Work Tracking edges; and reuses the existing Dev Command Center, Locus occupancy, multi-session orchestration, and micro-task summary backlog instead of creating duplicate orchestration stub families.
- [ADD v02.163] Dev Command Center planning-and-coordination backend sweep: Appendix 12 backfills Task Board and Work Packet System as first-class backend coordination features, deepens Dev Command Center/Locus/Workflow Engine/Model Session Orchestration/Micro-Task Executor ownership for task-board authority, work-packet binding, workflow-linked activation, and parallel-session planning, and adds Dev Command Center to Task Board, Dev Command Center to Work Packet System, Task Board to Locus Work Tracking, and Work Packet System to Workflow Engine edges while reusing existing Dev Command Center, Locus, and Spec Session Log backlog instead of creating duplicate planning-system stubs.
- [ADD v02.164] Dev Command Center resilience and governed repository-decision backend sweep: Appendix 12 deepens Dev Command Center as the governed recovery and decision surface for session checkpoints, heartbeat freshness, provider capability readiness, anti-pattern alerts, and repository-engine backend policy; adds Dev Command Center to Model Session Orchestration and Dev Command Center to Unified Tool Surface edges; and reuses the existing Dev Command Center, Provider Feature Coverage, Session Crash Recovery, Session Observability, Session Anti-Pattern Registry, Workflow Projection Correlation, and Git Engine Decision Gate backlog instead of creating duplicate recovery or repository-policy stubs.
- [ADD v02.165] Dev Command Center operating-surface backend sweep: Appendix 12 deepens Dev Command Center as the governed run-history, tool infrastructure, workspace-runtime, and promotion-gate surface; adds Dev Command Center to Workflow Engine and Dev Command Center to Unified Tool Surface edges for replay and tool-health projection; and reuses the existing Dev Command Center, Workflow Projection Correlation, Unified Tool Surface Contract, Workspace Safety Parallel Sessions, and Git Engine Decision Gate backlog instead of creating duplicate run-history or promotion-gate stub families.
- [ADD v02.166] Structured collaboration-substrate backend sweep: Appendix 12 deepens Locus Work Tracking, Micro-Task Executor, Role Mailbox, Task Board, Work Packet System, and Dev Command Center as one structured work-state substrate; adds Dev Command Center to Role Mailbox and Role Mailbox to Work Packet System edges for triage and handoff correlation; and reuses the existing Dev Command Center, Locus Work Tracking, Role Mailbox, and workflow backlog instead of creating duplicate structured-state stub families.
- [ADD v02.167] Canonical structured artifact backend sweep: Appendix 12 deepens Locus Work Tracking, Task Board, Work Packet System, Role Mailbox, and Dev Command Center around versioned JavaScript Object Notation file standards, compact summaries, project-agnostic profile envelopes, and projected board or queue layouts; adds Task Board to Work Packet System structured-board projection guidance; and materializes only genuinely new structured-artifact, mirror-sync, and typed-viewer implementation gaps as dedicated stubs.
- [ADD v02.168] Base structured schema and project-profile sweep: Appendix 12 deepens Locus Work Tracking, Micro-Task Executor, Task Board, Work Packet System, Role Mailbox, and Dev Command Center around shared base-envelope fields, compact summary contracts, mirror-state semantics, and profile-extension boundaries; and materializes only genuinely new schema-registry and project-profile-extension implementation gaps as dedicated stubs.
- [ADD v02.169] Canonical-to-mirror reconciliation sweep: Appendix 12 deepens Dev Command Center, Locus Work Tracking, Task Board, Work Packet System, Role Mailbox, and Micro-Task Executor around mirror authority mode, reconciliation action, advisory-edit posture, and normalization-safe Markdown regeneration; and reuses the existing Markdown mirror sync and typed viewer stubs instead of creating duplicate drift-governance tracks.
- [ADD v02.170] Dev Command Center layout-projection sweep: Appendix 12 deepens Dev Command Center, Task Board, Work Packet System, Role Mailbox, and Micro-Task Executor around typed view presets, lane definitions, governed action bindings, roadmap or inbox layouts, and local-small-model execution queues; adds execution-queue projection guidance; and materializes only genuinely new layout-projection registry work as a dedicated stub.
- [ADD v02.171] Workflow-state and governed-action sweep: Appendix 12 deepens Dev Command Center, Locus Work Tracking, Work Packet System, Micro-Task Executor, Task Board, and Role Mailbox around project-agnostic workflow-state families, queue-reason codes, and governed action descriptors; adds mailbox and queue-reason routing guidance; and materializes only genuinely new workflow-state registry work as a dedicated stub.
- [ADD v02.172] Workflow transition and executor sweep: Appendix 12 deepens Dev Command Center, Locus Work Tracking, Work Packet System, Micro-Task Executor, Task Board, Role Mailbox, and Workflow Engine around transition rules, automatic queue moves, approval-gated state changes, and executor eligibility; and materializes only genuinely new transition-automation registry work as a dedicated stub.
- [ADD v02.173] Role Mailbox message-contract sweep: Appendix 12 deepens Role Mailbox, Locus Work Tracking, Work Packet System, Micro-Task Executor, Task Board, and Dev Command Center around typed message families, thread lifecycle, delivery state, allowed-response envelopes, mailbox-local versus governed actions, and Micro-Task collaboration loops; and materializes only genuinely new mailbox-contract work as a dedicated stub.
- [ADD v02.174] Role Mailbox and Micro-Task loop-control sweep: Appendix 12 deepens Role Mailbox, Micro-Task Executor, Locus Work Tracking, Work Packet System, Task Board, and Dev Command Center around bounded loop checkpoints, structured verifier outcomes, retry-budget posture, escalation targets, dead-letter loop handling, and completion-report transcription; deepens Role Mailbox to Micro-Task Executor and Role Mailbox to Work Packet System guidance; and materializes only genuinely new mailbox-loop-control work as a dedicated stub.
- [ADD v02.176] Role Mailbox executor-routing and claim-lease sweep: Appendix 12 deepens Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Task Board, and Work Packet System around executor-kind routing, exclusive versus shared claim modes, claimant visibility, lease age and expiry, response-authority scope, takeover legality, and human-only override boundaries; deepens mailbox claimant and actor-eligibility guidance; and materializes only genuinely new mailbox claim-lease work as a dedicated stub.
- [ADD v02.177] Role Mailbox handoff-bundle and announce-back sweep: Appendix 12 deepens Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center around structured handoff bundles, note-transcription provenance, announce-back kinds, compact handoff summaries, and replay-safe operator views; deepens mailbox-to-packet and mailbox-to-Dev-Command-Center handoff guidance; and materializes only genuinely new mailbox handoff and transcription work as a dedicated stub.
- [ADD v02.178] RAG mode and no-RAG sweep: Appendix 12 deepens Project Brain, AI-Ready Data, ACE Runtime, Loom, Prompt-to-Spec Router, Work Packet System, and Micro-Task Executor around retrieval-mode selection, non-hybrid reasons, Loom-driven graph bias, authoritative direct loads, and bounded local-model context assembly; adds Loom -> Project Brain, Loom -> Prompt-to-Spec Router, Prompt-to-Spec Router -> Work Packet System, and Work Packet System -> Micro-Task Executor edges; and materializes only genuinely new retrieval-mode policy work as a dedicated stub.

- [ADD v02.181] Software-delivery governance overlay boundary sweep: Phase 1 MUST keep repository `/.GOV/**` artifacts as imported overlay source material and evidence while live software-delivery authority moves through product-owned runtime records and workflow-backed governed actions.
- [ADD v02.181] Software-delivery runtime-truth sweep: Phase 1 MUST expose software-delivery work through stable-id-linked runtime records, linked governed actions, and workflow-backed state rather than packet text, mailbox order, or Markdown mirrors acting as operational truth.
- [ADD v02.181] Validator-gate and closeout sweep: Phase 1 MUST converge validator posture into runtime-visible gate summaries and evidence-linked gate executions, and MUST derive closeout posture from canonical runtime and gate state rather than packet surgery.
- [ADD v02.181] Projection-surface sweep: Phase 1 MUST keep Dev Command Center, Task Board, and Role Mailbox as projection or control surfaces over the same runtime truth, with no planning lane, inbox thread, or readable mirror becoming authority by chronology alone.
- [ADD v02.181] Overlay coordination-record sweep: Phase 1 MUST model claim/lease posture and queued steering or follow-up posture explicitly where software-delivery control requires ownership, takeover, renewal, escalation, or deferred steering semantics.
- [ADD v02.181] Overlay lifecycle and recovery sweep: Phase 1 MUST expose lifecycle checkpoints and workflow-backed start/steer/cancel/close/recover semantics so crash recovery, restart-safe steering, and partial-failure handling remain replayable and explainable.

- Calendar range selection returns the same ActivitySpan/SessionSpan set as the equivalent Flight Recorder interval-overlap query (filters + attribution mode recorded).


- For every AI action in the UI, a corresponding AI job and workflow run exists and can be inspected.  
- Flight Recorder shows a coherent timeline for at least the core loop.  
- Metrics and logs are sufficient to explain failures in the core loop without reading the entire codebase.
- Operator Consoles v1 exists (Timeline + Jobs + Problems + Evidence) and every entry deep-links to the underlying trace/events.  
- Debug Bundle export is redacted-by-default, deterministic for the same selection, and passes the validator pack in CI.  
  
- Bootloader/Diary checks for logging and non-silent edits pass in CI.
- Data layer invariants enforced: Raw/Derived/Display separation respected; layer_scope/apply_scoped/preview_only/access_mode persisted; per-op provenance visible in Flight Recorder.  
- At least one end-to-end MCP-backed job (stub server is fine) is visible in Job History and Flight Recorder, with Gate decisions and capability metadata attached.
- [ADD v02.136] Tool Registry + Tool Gate: at least 10 tools are registered with stable names/versions and side_effect labels; local and MCP tool discovery expose the same HTC schema; every tool call emits FR-EVT-007 (ToolCallEvent) with redaction; Tool Contract conformance tests (Â§6.0.2.9) pass in CI.
- [ADD v02.138] Front End Memory System (FEMS): `SESSION_SCOPED` and `WORKSPACE_SCOPED` policies inject a bounded `MemoryPack` (â‰¤500 tokens) visible in DCC; procedural memory proposals require explicit approval; `FR-EVT-MEM-*` are emitted; replay reproduces the same `memory_pack_hash`; FEMS-EVAL-001 passes.
- [ADD v02.178] Retrieval mode discipline: `QueryPlan` and `RetrievalTrace` record `retrieval_mode` and `non_hybrid_reason`; known Work Packet or Micro-Task ids and exact Loom or artifact refs bypass hybrid retrieval by default; Project Brain and Prompt-to-Spec use hybrid retrieval only for discovery or synthesis; and local-small-model Micro-Task loops consume bounded direct loads or compacted snippets rather than broad Project Brain retrieval by default.
- Migrations validated: forward/backward fixture tests pass (up + down), replay-safety test passes (replay all up migrations), and migration version surfaces in a health check.  
- Workflow/Job completeness: mandatory fields (job_kind/profile_id/layer_scope/EntityRef) recorded; idempotency keys honored; retries capped; crash/restart yields resumed or failed runs with clear status.
- Capability model is default-deny across AI/mechanical/terminal/Monaco; approvals cached with TTL; allow/deny decisions logged in Flight Recorder.
- Retention/redaction defaults enforced: FR/log retention windows applied; redacted output retention window honored; env/secret scrubbing verified.
- [ADD v02.101] Spec Router produces SpecIntent + SpecRouterDecision artifacts with pinned capability_registry_version; Task Board and Work Packet entries are created/updated and visible in a Spec Session Log view.
- [ADD v02.128] Spec Creation System v2.2.1 is usable in-app (command palette/CLI): `/spec new|extend|refine|check` and `/task` routes are command-driven, emit the correct artifacts, and update `SPEC_INDEX.yaml` deterministically.
- [ADD v02.128] `/spec check` emits a rubric report that includes second-model + red-team passes; hard failures block GOV_STRICT activation and are visible in Problems/Evidence + Spec Session Log.
- [ADD v02.128] Universal ID + requirement grammar validation runs on spec artifacts; violations are explained and block activation in GOV_STRICT/GOV_STANDARD.
- [ADD v02.101] Git workflows require safety commit before execution; non-git workflows skip the safety commit step by policy.
- [ADD v02.101] Atelier Lens claim/glance runs on all ingested artifacts by default; disable only via LAW override.
- **[ADD v02.63] Model profile clarity:** Runtime integration documents and ships a concrete ModelProfile/Routing/SafetyProfile schema for MVP models (id, role, safety policy, routing notes) and evidence of usage in jobs.
- Terminal LAW (minimal slice): run_command defaults to policy mode with timeout (~180s), kill_grace, and max_output_bytes (~1â€“2MB); approvals UI present; sessions bound to workspace; executions logged in Flight Recorder.
- CI gates: lint/format/test and health script enforced in CI; fail if logging/FR hooks are missing.
- Sheets MVP present (v02.70). Implementation details moved to **Â§2.2.1.13**.
  - HyperFormula formulas, basic grid operations, and import/export fixture pass.
- Safety/ops engines exercised: Guard/Container/Quota enforced with FR evidence; Profiler/Monitor metrics visible per job; Repo/Formatter/Deploy paths logged via capability gates; Clipboard/Notifier actions bound to consent/capability.
- Atelier foundation present: create/edit `AtelierProductionPlan`, run `AtelierCompiler` to emit deterministic prompt/design/comfy_recipe exports with provenance; internal storage remains raw.



- [ADD v02.36] Debug Bundle for a recorded job includes ContextPlan + ContextSnapshots + validator outcomes; validator pack runs in CI.
- [ADD v02.36] Terminal LAW tests exist: denied commands are denied with logged gate decision; allowed commands are fully traced with scrubbed env.
- [ADD v02.38] Design Pack fonts load offline and include required licensing artifacts (per-font license files + THIRD_PARTY_NOTICES).  
- [ADD v02.38] Font Registry import/remove updates manifest deterministically and is visible in UI (list families).  
- [ADD v02.38] Canvas text uses bundled fonts without fallback flash; Canvas export preserves selected font.


- [ADD v02.42] Any retrieval performed in Phase 1 is snippet-only (`fetch_depth = snippet`), budgeted, and logged with EvidenceSnippet rationale; no full-page dumps enter the model context.

- [ADD v02.44] Build/release fails if any shipped OSS dependency is absent from the OSS Component Register or violates isolation rules.
- [ADD v02.44] Supply-chain gates run end-to-end and store their reports/SBOM/license outputs as artifacts, visible in Jobs + Flight Recorder.
- [ADD v02.44] Export deliverable PDF is available as a Job and produces a stored artifact via Typst + qpdf.
- [ADD v02.49] At least two Phase-1 mechanical jobs produce artifacts that (a) have manifests, (b) hash with SHA-256, (c) are discoverable via a minimal artifact viewer/list, and (d) survive restart.
- [ADD v02.49] Pin/unpin + TTL + GC can be demonstrated end-to-end; GC does not delete pinned; retention report is stored as an artifact.
- [ADD v02.49] Any â€œexport to file pathâ€ uses atomic materialize and logs policy + hashes (no direct writes).

- [ADD v02.52] For every retrieval-backed call: QueryPlan + RetrievalTrace exist, are hashed, and are reachable from Job History/Operator Consoles; evidence items are bounded and carry SourceRefs or ArtifactHandles.
- [ADD v02.52] CI runs T-ACE-RAG-001 (normalization determinism) and T-ACE-RAG-002 (strict ranking determinism) on a fixed fixture corpus; failures surface as Problems with Debug Bundle linkage.
- [ADD v02.52] Debug Bundle meets required structure and emits its export event (per existing Master Spec).
- [ADD v02.52] Workspace Bundle contains required tree and manifest; produces stable hashes when rerun with identical inputs/profile.
- [ADD v02.52] Policy context is captured/visible for export actions.
- [ADD v02.52] Attempt to include `exportable=false` artifacts without explicit policy is denied, logged, and surfaced.


- [ADD v02.67] Atelier Lens v0.1 runs end-to-end on a mixed-domain fixture set; Role Claims + Role Glances are visible, and top-k role extraction produces evidence-linked `RoleDescriptorBundle`s.
- [ADD v02.67] `ATELIER_STATE_MERGE` produces deterministic `SceneState.resolved` hashes under pinned inputs and emits `ConflictSet` whenever conflicts exist; `ATELIER-LENS-VAL-007/008` pass on golden fixtures.
- [ADD v02.67] `ConceptRecipe` is generated from Artistic Vectors, persisted with pins, and replayable; `ATELIER-LENS-VAL-009` passes.
- [ADD v02.67] `AtelierProductionGraph` is solvable and produces a stable `solve_plan`; `ATELIER-LENS-VAL-010/011` pass.
- [ADD v02.67] At least one Finishing role emits a typed deliverable bundle (e.g., grade targets/LUT spec, edit beat map, VFX shot list) with evidence refs and Flight Recorder provenance.


- [ADD v02.68] At least 3 mechanical engines pass Conformance Harness v0; Engine PlannedOperation/EngineResult envelopes validate, required gates run, and denials are visible in Problems/Flight Recorder.
- [ADD v02.68] Artifact-first engine I/O is enforced: payloads >32KB are artifact refs, outputs are artifacts with SHA-256 + provenance manifests, and D0/D1 runs include evidence artifacts when applicable.
- [ADD v02.79] Can import a folder of images, render a grid, open a single image, and export a derivative via job history with traceable inputs/outputs.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Two WPs can run concurrently iff IN_SCOPE_PATHS are disjoint; otherwise one blocks with CX-MM-002 and a lock conflict report.  
  - System runs in DOCS_ONLY with zero models, and in AI_ENABLED only with >=1 READY model.  
  - Every role output includes RoleExecutionIdentity metadata and HSK_STATUS updates on phase transitions.  
  - SwapRequest escalation can replace a failing smaller model with a larger model, preserving WP/MT state.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - Operator can open DCC, select a WP, open its linked worktree, view diff, approve a needed capability, and run a governed commit without leaving Handshake.  
  - Every stateful DCC action emits Flight Recorder events and is traceable to `wp_id`/`mt_id`/`session_id`/`wsid`.  
  - Denied approvals block the job deterministically with an explicit failure code and no partial side-effects.  
  - `.handshake/workspace.json` + `devcc.db` can be deleted and rebuilt from repo state + Locus without corrupting canonical governance artifacts.
- [ADD v02.164] Dev Command Center resilience and repository decision posture: orphaned or blocked sessions surface checkpoint age, heartbeat freshness, pending tool-call count, and governed resume or cancel actions; provider readiness surfaces by stable session and workflow identifiers; and the version-control panel exposes the declared repository backend, required status checks, merge-queue compatibility, and explicit no-silent-fallback posture before protected-branch actions proceed.
- [ADD v02.165] Dev Command Center operating surfaces: run history exposes workflow, node, tool, and checkpoint chronology with replay entrypoints; tool infrastructure registry exposes transport, health, permission scope, route policy, and last failure; workspace runtime exposes isolation posture, startup readiness, and external-workspace state; and promotion gate snapshots expose review resolution, required checks, merge-queue posture, and last verification timestamps before protected-branch actions proceed.
- [ADD v02.166] Structured collaboration substrate: at least one Work Packet, one Micro-Task, one Task Board view, and one Role Mailbox thread can be inspected through Dev Command Center typed-field viewers; local small models consume bounded structured fields without parsing raw Markdown contracts; and append-only notes plus Markdown mirrors remain synchronized with the authoritative structured records.
- [ADD v02.167] Canonical structured artifacts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox thread exist as versioned JavaScript Object Notation or JavaScript Object Notation Lines artifacts with compact summaries and project-agnostic base envelopes; Dev Command Center can render at least two different board or queue layouts from the same structured records without changing authority; and Markdown mirrors are detectable as synchronized, stale, or advisory.
- [ADD v02.168] Base schema and project-profile contracts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox export family member declare the same base structured-collaboration envelope fields, expose compact summary records, and isolate project-specific fields inside explicit profile extensions; Dev Command Center can distinguish base-envelope versus profile-extension fields without raw-file inspection; and mirror-state handling remains deterministic across canonical detail, summary, and Markdown mirror views.
- [ADD v02.169] Canonical-to-mirror drift governance: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox export family member declare mirror authority mode and reconciliation action explicitly; Dev Command Center can show why a Markdown mirror is synchronized, stale, advisory, or normalization-required; and regenerating readable mirrors does not silently overwrite append-only note sidecars or operator-authored advisory content.
- [ADD v02.170] Typed operating layouts: Dev Command Center can switch between board, queue, list, roadmap, inbox-triage, and execution-queue presets over the same canonical records; each drag, quick action, or bulk action previews the governed target fields or workflow actions before mutation; and local-small-model readiness queues remain derivable from compact summaries plus mailbox-response state.
- [ADD v02.171] Project-agnostic workflow-state contracts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox-linked queue row declare `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` explicitly; Dev Command Center can show why a record is in intake, ready, waiting, review, approval, validation, blocked, done, canceled, or archived state; and local-small-model routing remains derivable from base families plus reason codes even when project-profile labels differ.
- [ADD v02.172] Workflow transition and executor eligibility contracts: at least one Work Packet, one Micro-Task, one Task Board projection, and one Role Mailbox-linked wait expose the transition rule, automatic queue-action rule, and executor eligibility policy that explain why a record may start, wait, escalate, retry, request review, request approval, validate, complete, or remain blocked; and Dev Command Center can preview whether an action is automatic, approval-gated, or actor-ineligible before state changes occur.
- [ADD v02.173] Role Mailbox message and thread contracts: at least one Work Packet handoff thread, one Micro-Task request or feedback thread, and one escalation or announce-back thread expose thread lifecycle state, message delivery state, allowed responses, due or snooze posture, linked stable identifiers, and the authority boundary between mailbox-local actions and governed record mutations; and Dev Command Center can preview that distinction before a reply, acknowledgement, escalation, or transcription action executes.
- [ADD v02.174] Role Mailbox and Micro-Task loop control: at least one Micro-Task request thread, one feedback or retry loop, one verification-needed thread, and one escalation or completion-report thread expose remaining retry budget, structured verifier outcome, escalation target, last loop checkpoint, and linked stable identifiers; Dev Command Center can preview whether the next action is mailbox-local, governed retry, governed escalate, governed complete, or transcription-only; and linked Work Packet and Task Board views can explain why a task is waiting, retrying, escalated, or complete without replaying the full thread.
- [ADD v02.175] Role Mailbox triage and remediation controls: at least one snoozed thread, one due-soon or expired thread, and one dead-letter remediation thread expose triage queue state, reminder schedule, queue age, dead-letter disposition, and linked stable identifiers; Dev Command Center can preview whether reminder, unsnooze, retry-delivery, reroute, transcription, or archive controls are mailbox-local, automation-triggering, or governed; and linked Task Board, Work Packet, and Locus views can explain mailbox-derived waiting pressure without using thread order as authority.
- [ADD v02.176] Role Mailbox executor routing and claim-lease control: at least one exclusively leased thread, one shared-observer or broadcast thread, and one takeover or lease-expiry case expose executor kind, claimant identity, claim mode, lease age, lease expiry, response-authority scope, and linked stable identifiers; Dev Command Center can preview whether claim, release, renew, reroute, takeover, or reply actions are mailbox-local, automation-triggering, or governed; and linked Locus, Micro-Task, Work Packet, and Task Board views can explain who may act next without relying on assignment comments or transcript order.
- [ADD v02.177] Role Mailbox handoff bundle and announce-back provenance: at least one delegate or handoff thread, one completion-report or announce-back thread, and one scope-change or escalation handback expose remaining work, unresolved blockers, recommended next actor, evidence refs, provenance kind, and transcription status by stable identifiers; Dev Command Center can distinguish advisory announce-back from transcription-confirmed completion; and linked Work Packet, Locus, Micro-Task, and Task Board views can resume from compact handoff state without replaying the full thread.
- [ADD v02.181] Software-delivery governance overlay boundary: at least one workflow-backed software-delivery flow preserves repository `/.GOV/**` artifacts as imported overlay source material or evidence, and Governance Pack import/export preserves those artifacts without bypassing workflow-backed runtime law.
- [ADD v02.181] Software-delivery runtime truth: at least one workflow-backed software-delivery work item exposes product-owned runtime state and linked governed actions by stable identifiers instead of relying on packet prose, mailbox order, or Markdown mirrors as the operational authority surface.
- [ADD v02.181] Validator-gate and closeout posture: at least one workflow-backed software-delivery work item exposes validator-gate summaries, evidence-linked gate posture, and derived closeout posture by stable identifiers without requiring packet surgery to explain why the item may proceed or close.
- [ADD v02.181] Projection-surface discipline: Dev Command Center, Task Board, and Role Mailbox projections for at least one software-delivery work item explain the same underlying state without turning repo `/.GOV/**`, Markdown mirrors, or mailbox chronology into authority.
- [ADD v02.181] Overlay coordination records: at least one software-delivery work item exposes overlay claim/lease state and queued steering or follow-up state by stable identifiers so actor ownership, takeover legality, and deferred steering are visible without transcript reconstruction.
- [ADD v02.181] Overlay lifecycle and recovery posture: at least one software-delivery work item exposes checkpoint-backed recovery posture plus workflow-backed start/steer/cancel/close/recover semantics by stable identifiers so restart-safe replay and control decisions remain inspectable.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - If `approximate.allowed=false` or waiver missing, approximate execution cannot occur; system downgrades to exact and logs the downgrade.  
  - For calls with `settings.exec_policy`, Flight Recorder contains an `llm_exec_policy` event capturing requested vs effective policy.  
  - If approximate execution occurs, the event includes `waiver_ref` and a trace artifact reference (or explicit `trace_artifact_ref=null` with reason).

**Explicitly OUT of scope**
- [ADD v02.130] Loom: AI auto-tagging/auto-caption, semantic/hybrid search (Tierâ€‘3), multi-user Loom collaboration/sync, and Postgres-backed Loom query engines are Phase 2+ / Phase 4.
- [ADD v02.131] Stage: browser-extension ecosystem, arbitrary third-party Stage Apps/plugin marketplace, bulk crawling/mirroring, Stage Studio authoring (Spline-class editor), and advanced 3D editing/collaboration are Phase 3+ / Phase 4.


- [ADD v02.47] Charts/dashboards, decks, and any PPTX/PDF export pipelines (including in-app presentation surfaces).

- External calendar sync (CalDAV) and any external write-back.
- Multiple LLM runtimes and sophisticated model routing.  
- Sheets engine beyond a minimal stub (tables can be represented, but no full formula engine yet).  
- Docling ingestion, ASR pipeline, connectors, plugin system.  
- Multi-user sync and CRDT.
- High-performance LLM runtimes (e.g., `vLLM`, `TGI`) or cloud-scale routing beyond the single local runtime.  
- Advanced image generation stacks (`SDXL`, `ComfyUI`) and related workflows.  
- Full spreadsheet functionality beyond a minimal grid display (HyperFormula formulas stay stubbed in Phase 1).  
- Observability dashboards (e.g., `Grafana`, `Jaeger`) beyond the built-in MVP diagnostics surfaces.  

- Paid/proprietary font distribution and cloud font providers (e.g., Adobe Fonts).  
- Advanced OpenType feature UI (liga/ssXX) and variable-font axis controls beyond basic weight/italic.  
- Font editing/subsetting workflows.

- [ADD v02.49] Cross-device artifact sync/dedup, multi-user artifact lineage, and advanced GC heuristics beyond TTL+pinning.

- [ADD v02.52] ContextPacks builder job, pack freshness guard, index drift guard, and hash-key caching effectiveness (candidate/rerank/span caches) â€” Phase 2.
- [ADD v02.52] Format round-tripping (DOCX/PPTX/XLSX writers).
- [ADD v02.52] Cloud bundle sharing/upload.
- [ADD v02.52] Export workflows that mutate Raw/Derived stores.

- [ADD v02.79] Photo Studio advanced features: RAW decode, lens corrections, masks, layer compositor, HDR/pano/focus merges, AI vision, ComfyUI, vector tools.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Any GPU-sharded inference of a single model.  
  - Any requirement for multiple GPUs.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - GitHub PR/comment sync.  
  - Multi-user workspace sync / shared approvals.  
  - Full external conversation ingestion (beyond adapter skeleton + at least one external pilot, if any).  
  - Any UI commitment to Sidecar keybindings/TUI parity.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Implementing true layer-wise inference in local runtimes (LayerSkip/early-exit/etc).  
  - Multi-device/sharded inference.

**Mechanical Track (Phase 1)**
- [ADD v02.130] Loom media mechanicals: deterministic content hashing + dedup, background thumbnail/proxy generation jobs, and periodic recomputation of LoomBlock metrics (backlink/mention/tag counts).
- [ADD v02.131] Stage capture mechanicals: wire minimal `Archivist` (web capture + evidence bundling) and `Guide` (live verify) as Tool Bus engines; add Stage capture/import job profiles (`stage.capture_webpage.v1`, `stage.clip_selection.v1`, `stage.import_pdf.v1`) and 3D validators (`stage.3d.validate_gltf.v1`) producing artifact bundles with manifests, hashes, and replayable provenance.
- [ADD v02.136] Populate **Tool Registry** (HTC v1.0) for all Phase 1 engines/tools (`Context`, `Version`, `Sandbox`, `Publisher`, `Formatter`, `Repo`, `Deploy`, `Guard`, `Container`, `Quota`, `Profiler`, `Monitor`, `Clipboard`, `Notifier`) and enforce calls through Tool Gate (no-bypass).

- Deliver low-risk local engines: `Context` (rg), `Version` (Jujutsu/Gitoxide), `Sandbox` (safe code exec), `Publisher` (deterministic Markdown/Doc to PDF), `Formatter` (lint/format enforcement), `Repo` (git/libgit actions), and `Deploy` (minimal devops automation).
- Add safety/observability primitives: `Guard` (secret/safety scan), `Container` (isolated exec), `Quota` (resource limits), `Profiler`/`Monitor` (system metrics/alerts), `Clipboard` (controlled ephemeral context), and `Notifier` (desktop notifications).
- MVP engine implementations MUST be demonstrable end-to-end: `Context` using `rg` for text search, `Version` using git/libgit for repo state, `Formatter` using `ruff` and `prettier`, and `Guard` using `trufflehog` for secret scanning, all running through the mechanical runner abstraction and emitting Flight Recorder provenance.
- All mechanical jobs MUST run via the Workflow Engine with capability checks; log command, params, exit code, stdout/stderr, artifact hash, and store DerivedContent + sidecar provenance.
- Acceptance: at least two mechanical job profiles visible in Job History with capability enforcement tests and reproducible commands; safety/resource gates (Guard/Container/Quota/Profiler/Monitor) exercised and logged; Clipboard/Notifier actions bound to capability/consent.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 1:**
  - Implement Bronze/Silver/Gold storage layers mapped to `workspace/raw/`, `workspace/derived/`, `workspace/indexes/`
  - Implement content-aware chunking for code (AST-aware, 100-500 tokens) and documents (header-recursive, 256-512 tokens)
  - Implement embedding pipeline with model versioning (`text-embedding-3-small` default, `bge-small-en-v1.5` local fallback)
  - Implement hybrid search (vector HNSW + keyword BM25) with configurable weights
  - Implement ingestion validation gates (token count, coherence checks, boundary validation)
  - **[REMEDIATION]** Wire FR-EVT-DATA-001 through FR-EVT-DATA-015 events to existing Flight Recorder (new event schemas for bronze/silver/embedding/retrieval/quality)
  - Implement quality SLOs and alerts (MRR â‰¥ 0.6, Recall@10 â‰¥ 0.8, p95 retrieval â‰¤ 500ms)
  - Acceptance: hybrid search returns results from Monaco, Terminal, and basic docs; retrieval traces visible in Operator Consoles; FR-EVT-DATA events appear in Flight Recorder


- [ADD v02.36] At least one mechanical job attempt is *denied* by capability/consent and the denial is visible in Problems + Flight Recorder (no side effects).
- [ADD v02.38] Font Registry mechanical job(s): `fonts_bootstrap_pack`, `fonts_rebuild_manifest`, `fonts_import`, `fonts_remove` (capability-gated; provenance recorded in Flight Recorder).  
- [ADD v02.38] Font pack manifests and per-font license metadata stored as DerivedContent with hashes; UI does not crawl the filesystem for font discovery.


- [ADD v02.44] Supply-chain gate mechanical Jobs (CI-gated): `secret_scan` (gitleaks), `vuln_scan` (osv-scanner), `sbom_generate` (syft), `license_scan` (scancode), each emitting artifacts + provenance.
- [ADD v02.44] OSS Register audit mechanical Job: `oss_register_audit` verifies (1) every integrated component has a Register entry and (2) integration mode matches license policy (GPL/AGPL isolation).
- [ADD v02.44] Git engine integration decision gate: record and enforce a single MVP path (`git` CLI `external_process` vs `libgit2` vs `go-git`); default to `git` CLI `external_process` until a decision is logged.
- [ADD v02.49] Artifact store bootstrap: create workspace `.handshake/artifacts/L{1,2,3}/<artifact_id>/â€¦` and write artifact manifests for every job output; hashes are SHA-256 everywhere (no SHA1 drift).
- [ADD v02.49] Materialize API: ALL â€œsave/export to pathâ€ writes go through one atomic materialize function (tmp + rename), capability-gated and Flight Recorder logged; no direct UI bypass.
- [ADD v02.49] Retention/pinning MVP: implement pin/unpin + TTL + a deterministic GC job/command; GC never deletes pinned artifacts; emit a retention report artifact for visibility.
- [ADD v02.49] Bundle canonical hashing: implement canonical bundle hashing (zip normalization) and use it for any bundle-style artifact (debug bundles, packaged deliverables, multi-file exports).

- [ADD v02.52] Retrieval trace bundle exporter: a mechanical job that takes `trace_id` (and referenced artifacts) and emits a redacted-by-default Debug Bundle artifact for retrieval issues (QueryPlan/Trace + budgets + cache keys + selected spans).
- [ADD v02.52] Deterministic bounded-read span extractor (mechanical helper) used by retrieval escalation paths; emits span selection provenance and truncation flags.
- [ADD v02.52] Job profiles (capability-gated; logged; hashed): `debug_bundle_export_v0`, `workspace_bundle_export_v0`.


- [ADD v02.67] Add mechanical job profiles + runner integration for Atelier Lens: `atelier_claim_v0`, `atelier_role_extract_v0`, `atelier_role_compose_v0`, `atelier_state_merge_v0`, `atelier_graph_solve_v0`, `atelier_concept_recipe_v0`; all capability-gated and Flight Recorder logged with pins/hashes.
- [ADD v02.67] Add CI fixtures + validators for Lens runs (VAL-007..011) and ensure denials/FAILs surface in Problems (no silent failures).


- [ADD v02.68] Mechanical Extension v1.2 enforcement: engine jobs use `poe-*` envelopes; required gates (`G-SCHEMA/G-CAP/G-INTEGRITY/G-BUDGET/G-PROVENANCE/G-DET`) run and are logged; engine registry is authoritative; conformance must pass before an engine is enabled for general use.
- [ADD v02.79] Add Darkroom engine stubs behind Mechanical Extension v1.2 (non-functional OK) so Photo Studio UI cannot mutate state outside the job/gate pipeline.


- [ADD v02.115] **Micro-Task Executor core loop (Â§2.6.6.8):** Implement MT Loop Controller with auto-generation of MT definitions from Work Packet scope, fresh-context-per-iteration execution, completion signal parsing with anti-gaming rules, and bounded iteration limits.
- [ADD v02.115] **MT validation engine wiring:** Wire validation commands through Mechanical Tool Bus (Â§6.3, Â§11.8) with PlannedOperation envelope, capability checks, and FR-EVT-MT-012 event emission.
- [ADD v02.115] **MT state persistence:** Implement ProgressArtifact and RunLedger schemas with atomic writes, crash recovery (Â§2.6.6.8.6.3), and FR-EVT-WF-RECOVERY integration.
- [ADD v02.115] **MT escalation chain:** Implement default 6-level escalation (7Bâ†’7B-altâ†’13Bâ†’13B-altâ†’32Bâ†’HARD_GATE) with LoRA selection by task_tags (auto_by_task_tags strategy).
- [ADD v02.115] Acceptance: MT Executor job profile (`micro_task_executor_v1`) visible in Job History; at least one Work Packet executes end-to-end with auto-generated MTs; escalation triggers FR-EVT-MT-005; hard gate pauses execution.

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 1:**
  - Implement SQLite backend with work_packets, micro_tasks, mt_iterations, dependencies tables (Â§2.3.15.5)
  - Implement core operations: locus_create_wp, locus_update_wp, locus_gate_wp, locus_close_wp, locus_register_mts, locus_start_mt, locus_record_iteration, locus_complete_mt (Â§2.3.15.3)
  - Implement dependency operations: locus_add_dependency, locus_remove_dependency with cycle detection (Â§2.3.15.7)
  - Implement basic query operations: locus_query_ready (dependency-aware), locus_get_wp_status, locus_get_mt_progress (Â§2.3.15.7)
  - Wire Spec Router integration: auto-invoke locus_create_wp when routing prompts, link to task_packet_path (Â§2.3.15.4)
  - Wire MT Executor integration: auto-invoke locus_start_mt, locus_record_iteration (every iteration), locus_complete_mt (Â§2.3.15.4)
  - Implement Task Board bidirectional sync: locus_sync_task_board reads/writes .handshake/gov/TASK_BOARD.md, auto-sync on WP state change (Â§2.3.15.4)
  - [ADD v02.116] **Task Board hygiene:** Task Board items tagged `v02.116` MUST be revised/updated (status, scope, owner, links). Ensure 1:1 mapping between Task Board entries and Locus `wp_id`s; remove stale/duplicate entries; re-run `locus_sync_task_board` to normalize.
  - Implement Bronze/Silver/Gold storage: WPBronze snapshots, WPSilver chunks with embeddings (text-embedding-3-small), basic keyword search (Â§2.3.15.5)
  - Wire Flight Recorder events: FR-EVT-WP-001..005, FR-EVT-MT-001..006, FR-EVT-DEP-001..002, FR-EVT-TB-001..003, FR-EVT-QUERY-001 (Â§2.3.15.6)
  - [ADD v02.116] Capability Registry update: ensure `locus.read`, `locus.write`, `locus.gate`, `locus.delete`, `locus.sync` are present in `CapabilityRegistry` SSoT; regenerate `assets/capability_registry.json`; add `HSK-4001: UnknownCapability` tests for all Locus operations.
  - [ADD v02.116] Flight Recorder schema validator update: register and validate Locus event families (FR-EVT-WP-001..005, FR-EVT-MT-001..006, FR-EVT-DEP-001..002, FR-EVT-TB-001..003, FR-EVT-SYNC-001..003, FR-EVT-QUERY-001); unknown event_type MUST fail fast in Diagnostics.
  - [ADD v02.116] Spec Router WorkPacketBinding enforcement: `work_packet_id` MUST be a valid existing Locus `wp_id`; invalid/missing MUST fail with Diagnostics and MUST NOT produce side effects (no writes, no external calls).
  - Acceptance: Spec Router creates WPs visible in Locus; MT Executor iterations recorded; Task Board syncs within 5s; **Task Board entries tagged `v02.116` revised/updated with no drift after sync**; locus_query_ready returns dependency-aware ready work; FR events appear in Flight Recorder.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - Implement lock semantics + Task Board integration.  
  - Implement HSK_STATUS generator + FR event correlation.  
  - Implement MailboxKind taxonomy + persistence.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - Implement DCC backend services: project/workspace/session registry and `devcc.db` migrations.  
  - Implement Approval Inbox plumbing: collect pending approvals from capability gate; persist decisions; emit Flight Recorder events.  
  - Implement worktree management job wrapper (create/open/prune) with safe defaults and explicit rewrite consent for destructive ops.  
  - Implement VCS panel operations via `engine.version` operations (status/diff/commit) + artifact-first commit messages.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Schema updates + validators (Work Profile v0.6; runtime settings allow `exec_policy`).  
  - Flight Recorder schema + retention/privacy enforcement for layerwise traces.

**Atelier Track (Phase 1)**
- Implement storage + versioning for `DerivedContent: AtelierProductionPlan` (prose-first brief headings always present; structured `PlanFields` footer).
- Add `ATELIER_PLAN` job profile and a minimal editor surface for creating/refining plans (no rendering required in Phase 1).
- Implement deterministic `AtelierCompiler` as a mechanical job to emit: `export:image_prompt_generic`, `export:graphic_design_brief`, and `export:comfy_recipe` (template_id + deterministic fallback).
- Wire Atelier validators (ATELIER-VAL-001..005) to plan save/compile; auto-fill defaults rather than prompting.
- Acceptance: plans can be authored, validated, compiled, and exported with provenance linking to input references; connector-specific filtering occurs only at Display/Export boundaries.

- [ADD v02.52] Any Atelier job step that consults workspace evidence MUST emit QueryPlan/Trace and obey RetrievalBudgets (no "hidden retrieval" inside compilers).
- [ADD v02.52] Workspace/Debug Bundles may include Atelier artifacts **only if policy allows**; filtering remains Display/Export-only.

- [ADD v02.101] Atelier Lens claim/glance is always-on for ingested artifacts and Spec Router inputs; disable only via LAW override.


- [ADD v02.67] Implement Lens surfaces: Role Claims Panel, Role Glances Grid, Role Bundle Viewer (with evidence highlights) and Deliverables Browser for `RoleDeliverableBundle`.
- [ADD v02.67] Wire `ATELIER_CLAIM` into ingestion surfaces so any imported image/text can be claimed by multiple roles (cross-domain by design).
- [ADD v02.67] Ship MVP role contracts (dual-contract) for: `dop.lighting`, `set.set_dressing`, `fashion.styling`, `graphic_design`, and one Finishing role (`finishing.color` or `finishing.editorial`).
- [ADD v02.67] Implement `ATELIER_CONCEPT_RECIPE` and pass `ConceptRecipe` into compilers/composers (Conceptual Mode becomes replayable; recipes are first-class artifacts).
- [ADD v02.67] Implement `ATELIER_STATE_MERGE` (SceneState + ConflictSet) and `ATELIER_GRAPH_SOLVE` (ProductionGraph + solve_plan) as mechanical jobs used by Atelier compilation flows.
- [ADD v02.79] Add Photo Studio worksurface shell: browser grid + viewer + metadata inspector (read-only metadata is acceptable in Phase 1).
- [ADD v02.123] Implement **Atelier Collaboration Panel (selection-scoped)** in the main text editor: a sidebar/panel that runs role passes against the current selection, shows per-role suggestions (multiple options preferred), supports checkmark selection, and applies changes **only** to the selected span (never touching non-selected text). All applied patches MUST be recorded with provenance (role_id, contract_id, model_id/tool_id, evidence refs, before/after spans).
- [ADD v02.123] Implement `LensExtractionTier` plumbing (Tier1 default) and surface it in Lens job traces; Tier1 MUST be the default for all ingestion/extraction unless explicitly escalated.
- [ADD v02.123] Enforce **lossless role catalog + append-only role registry** in runtime + validators: role_id stability, no reuse, and a blocking validator if a previously-declared role disappears from the registry snapshot.
- [ADD v02.123] Implement `ViewMode` UI + enforcement for Lens outputs: `SFW` mode MUST **hard-drop** any non-`sfw` results from retrieval/output while preserving evidence pointers internally; switching ViewMode MUST NOT mutate stored Raw/Derived artifacts.
- [ADD v02.123] Implement role-turn isolation (role reset + context window reset) as the default execution mode for role passes (claim/glance/extract) to keep small local models consistent and prevent cross-role contamination; record per-turn pins for deterministic replay.


- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - UI to show READY models, active Work Units, lock states, and compact lifecycle status.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - DCC kanban lanes for WP statuses (from Locus), with deep links to worktree, sessions, approvals, and Flight Recorder slices.  
  - UX for approvals: single compact list with previews and scoping (once/job/workspace).

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Operator UX for per-role compute preset selection and waiver-bound â€œapproximateâ€ toggle.

**Distillation Track (Phase 1)**
- Define Skill Bank schema alignment and logging-only distillation job profiles (no training) using Workflow Engine.
- Capture teacher/student metadata, context refs, reward features, lineage fields, and data_signature/job_ids_json in Flight Recorder.
- Acceptance: distillation job schema is wired with capability gating; log entries include all mandatory fields; no model training or promotion yet.
- [ADD v02.157] Context Pack build/select/refresh and freshness decisions MUST emit bounded Flight Recorder evidence and stable pack hashes so distillation/replay/onboarding paths never depend on hidden cache state.
- [ADD v02.157] Pending distillation candidate queues, PromptEnvelope hashes, and tokenizer metadata MUST remain durable backend evidence when Context Packs or Spec Router artifacts shape teacher/student inputs.

- [ADD v02.52] Distillation jobs MUST record referenced QueryPlan/Trace ids (when retrieval-backed) so training/eval inputs are auditable and replayable.
- [ADD v02.52] Distillation log artifacts must respect `exportable` flags so bundles cannot leak local-only payloads.


- [ADD v02.79] Capture Photo Studio job traces as distillation-ready sequences (recordable workflows; no learning required in Phase 1).

- [ADD v02.115] **MT Executor escalation candidate capture (Â§2.6.6.8.8):** When escalation occurs with `enable_distillation=true`, generate DistillationCandidate artifacts with student_attempt (failed) and teacher_success (passed) snapshots, task_type_tags, and data_trust_score.
- [ADD v02.115] **MT escalation record schema:** Capture contributing_factors (syntax_error, logic_error, scope_violation, etc.) and remediation outcomes for LoRA training feedback.
- [ADD v02.115] Acceptance: FR-EVT-MT-015 (distillation_candidate) events emitted on escalation; DistillationCandidate artifacts stored with Skill Bank schema alignment; no LoRA training in Phase 1.

- **[ADD v02.122] Multi-Model Orchestration & Lifecycle Telemetry**
  - None required.

- **[ADD v02.127] Dev Command Center (DCC) MVP (Sidecar-derived)**
  - None required.

- **[ADD v02.122] Layer-wise Inference Foundations (Hooks + Governance + Observability)**
  - Not required; keep as future.

- [ADD v02.137] Autonomous agent loops without operator oversight (full AutomationLevel.AUTONOMOUS is Phase 2+).
- [ADD v02.137] Cross-workspace session routing (sessions are workspace-scoped in Phase 1).
- [ADD v02.137] Real-time collaborative session sharing between multiple human operators.

### 7.6.4 Phase 2 â€” Ingestion & Shadow Workspace (Docling + RAG MVP)

**Goal**  
Make Handshake useful over **existing** files and unlock basic retrieval-augmented generation, reusing the existing AI Job, workflow, and observability stack. Maintain and extend debug surfaces for ingestion and retrieval.
- [ADD v02.123] Phase 2 Atelier/Lens focus: Docling-driven Lens enrichment, global facts index + role-lane retrieval, Tier2 auto-when-idle deep passes, and SymbolismProfile/lexicon growth surfaced in UI and operator consoles.
- [ADD v02.136] Begin **Design Studio shell/IA** alignment (see `Handshake_Design_Studio_Overhaul_v0.1.md`): treat worksurfaces as modules in one shell (rail + inspector + bottom drawers). This is **recontextualization** only; storage/layout/capability boundaries remain unchanged.



- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.52] Implement ACE-RAG-001 as the canonical RAG contract: packs-first routing, deterministic scoring/selection, hash-key caching, and drift detection wired into Operator Consoles.
- [ADD v02.52] Add: Bundle export covers imported files + ingestion outputs in a portable, policy-safe way.

- [ADD v02.68] Enforce v1.2 evidence semantics for any D1 ingestion/verification engine: external/non-deterministic claims must carry evidence artifacts (snapshots/screenshots) and be replayable from bundles.

- [ADD v02.79] Make Photo Studio content **searchable in Shadow Workspace** (metadata + previews/proxies indexed; raw binaries remain referenced).

- **[ADD v02.130] Loom semantic enrichment**  
  Make LoomBlocks searchable and semantically enrichable: auto-tag (suggested â†’ confirmed), auto-caption for media, and hybrid retrieval over Loom content in Shadow Workspace.
- **[ADD v02.131] Stage Phase 2 â€” Mechanical feedback loops + ingestion integration**  
  Upgrade Stage capture/import jobs to integrate with Docling + Shadow Workspace (cache/index assimilation), and extend the 3D Mechanical Assist Pack with canonicalize/optimize/physics checks and reviewable reports.



**MUST deliver**

1. **Docling integration (mechanical ingestion)**  
   - Integrate Docling as described in Section 6.1.  
   - Implement the **Docling AI Job profile**:
     - Jobs for format detection, conversion, structure extraction, and error recovery.  
   - Support importing at least `.docx` and `.pdf` into internal document blocks.  
   - Provide ingestion fallbacks for unsupported formats using **Unstructured** or **Apache Tika**, especially for email containers and odd legacy documents; fallbacks must still run through the same capability/logging pathways.  
   - Log ingestion jobs and their states in Flight Recorder (including failures and retryable conditions).

   - When Docling is run as a remote or sidecar service, it SHOULD be exposed as an MCP server:
     - Run the Docling MCP server as described in Section 6.1.2.7.4 where applicable.  
     - Invoke `convert_document` (and related tools) via the Rust MCP client and Gate (Section 11.3.2), not via ad-hoc HTTP.  
   - Implement the reference-based binary protocol for Docling jobs (Section 11.3.3):
     - Use sandboxed file references/URIs for large artefacts instead of embedding base64 in MCP messages.  
     - Enforce sandbox roots and symlink protections per Section 11.3.7 when resolving these URIs.  
   - Map Docling MCP progress and logging into the existing job and logging systems:
     - Use `notifications/progress` to update ingestion job rows (Target 3).  
     - Use `logging/message` to emit ingestion metrics into Flight Recorder (`fr_events`) (Target 5).  
   - [ADD v02.36] Debug Bundle (ingestion/RAG) includes Docling tool logs/progress + artifact references (hashes/handles), plus failure diagnostics.


2. **Shadow Workspace (index + graph)**  
   - Implement the Shadow Workspace as per Section 3:
     - Incremental parsing and chunking of documents.  
     - Incremental parsing and chunking of documents using **Tree-sitter** (or equivalent) for code-aware splits.  
     - Embedding generation via a local model (default: **nomic-embed-text** for text).  
     - Storage of embeddings and metadata in a local store (use a local vector store such as **LanceDB** or **sqlite-vec**).  
     - Image embeddings captured with **CLIP** for visual assets.  
     - Provide a minimal grid/sheets surface backed by **HyperFormula** so indexed tabular data can be computed against within the Shadow Workspace.  
   - Provide a unified "Search workspace" command in the UI using Shadow Workspace.
   - Expose Shadow Workspace inspection in **Operator Consoles** (see Â§10.5):
     - **Index Doctor**: freshness/backlog metrics, rebuild/backfill actions (capability-gated), and invariant/consistency diagnostics.
     - **Descriptor/Graph Explorer (read-only)**: show indexed entities/descriptor rows with provenance and â€œwhy this was retrievedâ€ links.
  
   - Emit metrics for indexing operations and query counts; record search queries and result identifiers in Flight Recorder or a dedicated search log.
   - [ADD v02.40] Cache-to-Index Assimilation (`LocalWebCacheIndex`) as part of Shadow Workspace indexing (see Â§2.3.8.1):
     - Store external fetches used for retrieval into a local cache index; normalize (boilerplate strip + heading/anchor preservation), chunk, and hybrid-index.
     - Implement TTL + pinning (â€œgold sourcesâ€) and surface cache freshness/staleness metrics in Operator Consoles (Index Doctor).
     - Persist external fetches to `LocalWebCacheIndex` on agent stop (queue assimilation jobs if needed).

3. **RAG-aware AI jobs**  
   - New job kinds for:
     - â€œAnswer question using workspace documents.â€  
   - These jobs MUST:
     - Query Shadow Workspace for relevant chunks.  
     - Include retrieved context in prompts.  
     - Log retrieval steps and context (e.g. document IDs, snippet hashes) in Flight Recorder.  
   - Provide a debug view for at least one RAG action that shows:
   - Provide a **RAG Playground & Query Debugger** in Operator Consoles:
     - Inspect ranked chunks (IDs + hashes + ranks), prompt budget/truncation flags, and rerun retrieval without generating.
     - Deep-link from an AI answer â†’ its retrieval set â†’ the source documents/snippets.
     - [ADD v02.40] Retrieval fallback MUST consult `LocalWebCacheIndex` before external providers (cache-before-external).
     - [ADD v02.40] Retrieval MUST be snippet-first; escalate `snippet â†’ section â†’ fullpage` (fullpage = last resort; stored as artifact/cache only; never inject raw full pages into prompts); enforce per-step budgets and log truncation/compaction decisions.
     - [ADD v02.42] Implement adapter-level SEARCH â†’ READ separation: `search() -> snippets` and bounded `read(section_selector) -> excerpt` for LocalDocsIndex and LocalWebCacheIndex; section selectors must be bounded (heading/anchor ranges) and logged.
     - [ADD v02.42] Log escalation rationale when moving from `snippet` to `section` (and `fullpage` if used; why evidence was insufficient) and record `fetch_depth` explicitly on every evidence item.

     - [ADD v02.40] Evidence/provenance: capture `trust_class`, `fetch_depth`, and `cached_artifact_ref` when evidence is served from cache; surface cache-hit vs external-hit in the RAG Query Debugger.

     - Which documents/snippets were used.  
     - How they influenced the final answer (e.g. by showing context alongside the answer).
   - [ADD v02.36] Enforce evidence binding: answers must carry linked evidence refs; policy may fail the job or mark it incomplete when evidence is missing.
   - [ADD v02.36] Debug Bundle (ingestion/RAG) includes retrieval query, ranked chunk IDs/hashes, embedding/index configuration + versions, and prompt budget/truncation flags.

12. **[ADD v02.52] Workspace Bundle v0 (Expanded)**
   - Workspace Bundle v0 expands to include imported raw assets + key derived/canonical snapshots produced by ingestion.


4. **Descriptor extraction core (image + text)**  
   - Implement the DES-001 / IMG-001 / TXT-001 descriptor pipelines at MVP level for any content imported via Docling or direct file import, as defined in Sections 1.3 and 6.3.  
   - For each ingested document or asset:
     - Extract visual descriptors for images (and simple frame snapshots for video, where available) into DescriptorRows keyed by source material.  
   - Extract text descriptors for imported documents and user-authored text into TextDescriptorRows keyed by document/block (default embedding model: **nomic-embed-text**).  
   - Attach `content_tier`, `consent_profile`, and NSFW flags to each descriptor row, as defined in the Corpus/content-tier schema.  
 - Descriptor extraction MUST behave as a mechanical pipeline:
   - Jobs are AI Jobs under the Workflow Engine with job IDs, configs, status, and errors.  
   - All writes go through the DES-001 CORPUS/Sidecar contract; helpers never bypass CORPUS-DES001-NEW.  
 - Descriptor pipelines MUST respect Diary invariants:
   - Raw Corpus rows are never censored or euphemized; vocabulary control lives in CONFIG.  
   - SFW-only views and export redaction are handled in consuming views and export jobs (COR-700/701), not in extraction.
5. **Mail read-only ingestion**  
   - Mail store + IMAP/JMAP sync with `READ_EMAIL` capability; stable IDs (`internal_message_id`, `rfc822_message_id`, `provider_message_id`, `account_id`, optional `imap_uid/modseq`).  
   - Parse bodies via Unstructured/Tika; run attachments through Docling + OCR + ASR.  
   - Inject MailMessage blocks (RawContent) and TXT-001 mail descriptors (`MAIL_COMMUNICATION` domain) into Shadow Workspace; classification kept separate from other descriptor domains.  
   - Minimal UI: thread list + read-only view; FR logs engine versions/configs and ingestion results.

6. **Normalization, routing, and quality for ingestion**  
   - Indexing: declare `Indexer` as a first-class Shadow Workspace component with freshness/backlog metrics.  
   - Language/normalization: add `Detector` (language ID), `Converter` (text/encoding cleanup), and `Morphologist` (lemma/stem) stages to ingestion/descriptor pipelines.  
   - Data quality: add `Inspector` for audits/invariant checks and `Router` for explicit workflow/data-flow logging.

7. **SDXL / ComfyUI sidecar render (minimal, OSS)**  
   - Treat ComfyUI (AGPL) as a sidecar mechanical engine; do not reimplement a renderer.  
   - Provide a pinned SDXL text-to-image workflow graph (checked-in JSON) and call it via a mechanical job profile (e.g., `ATELIER_RENDER`) through the Workflow Engine; no graph authoring UI.  
   - Default model: **SDXL 1.0 base** (required) with optional SDXL 1.0 refiner; record `model_name` + SHA256 at runtime in Job History/FR. User-provided alternative models are allowed but must be logged by hash and are the userâ€™s licensing responsibility.  
   - Inputs: prompt, seed, steps, CFG scale, width/height (cap at e.g. 1024x1024), workflow_id/workflow_hash, model_hash.  
   - Outputs: store rendered images as DerivedContent with sidecar provenance (params, model/workflow hashes, ComfyUI request/response trace) and log to Flight Recorder + Job History.  
   - Governance/ops: gate via `atelier.render` capability; enforce quotas/timeouts (e.g., 120s), max output size, and VRAM guard; scrub secrets; health check ComfyUI before dispatch (local-only).  
   - UX: minimal trigger (e.g., â€œRender image from briefâ€) and artifact viewer link; show job status from Workflow Engine/Job History.  


8. **[ADD v02.44] Asset + paper ingestion as first-class Shadow Workspace sources**  
   - Implement `creative.asset_library.pipeline` ingestion (Tika + libvips + ExifTool + Czkawka) producing deterministic derived artifacts (thumbnails/metadata/dedupe).
   - Implement `science.ingest.papers_grobid` ingestion (GROBID service) producing structured paper + references for RAG.


9. **[ADD v02.47] Charts, Dashboards, and Decks (finance output MVP)**  
   - Implement `Chart` as a first-class entity that references a `Table` by ID plus optional range/query; persist only `chart_spec` (no raw table duplication).  
   - Implement dashboards as a layout/composition over existing entities (Charts + Tables + KPI blocks), typically via Canvas/Doc embedding (no new datastore).  
   - Implement `Deck` as a first-class entity whose slides reference existing entities (doc blocks, canvas frames, charts, assets).  
   - Provide in-app presenting (Reveal.js) and deterministic export (PPTX minimum) via mechanical jobs; exports produce artifact references with provenance.  
   - All non-trivial chart/deck create/update/export actions MUST run as explicit jobs under `charts_decks_ai_v0.1` (Â§2.5.11) with preview + validators.

10. **[ADD v02.49] Export + artifact discipline for Phase-2 exporters**  
   - Enforce `ExportRecord` + SHA-256 + canonical bundle hashing for any export job introduced in Phase 2 (deck_export_*, chart exports, Debug Bundle exports used by Operator Consoles).  
   - Any ingestion producing multi-file outputs (figures/tables/thumbnails) MUST emit per-artifact manifests and apply retention/pinning defaults based on classification.


13. **[ADD v02.79] Photo ingestion + catalog + indexing (Phase 2 baseline)**  
   - [ADD v02.79] Catalog/DAM baseline: collections, flags/ratings, folder sync; metadata read/write pipeline.  
   - [ADD v02.79] Proxy + preview pipeline as mechanical jobs (proxy-first for AI later).  
   - [ADD v02.79] Index photo metadata (and optionally captions) into Shadow Workspace for retrieval/debug.  



14. **Loom AI enrichment + hybrid search** [ADD v02.130]  
   - Add JobKinds + profiles for `loom_auto_tag`, `loom_auto_caption`, and `loom_batch_link` (Â§2.6.6; Â§10.12) and ensure they run through the Workflow Engine + Capability model (no bypass).
   - `loom_auto_tag`: produce `AI_SUGGESTED` edges only; provide accept/reject UI to convert to `TAG` edges per **[LM-TAG-005]**; emit FR-EVT-LOOM-008/009/010.
   - `loom_auto_caption`: generate `LoomBlock.derived.auto_caption` for image/video/audio assets; store as DerivedContent with attribution.
   - Enable Tierâ€‘3 Loom search (semantic/hybrid) by generating embeddings for LoomBlocks and wiring them into Shadow Workspace hybrid query path (Â§2.3.14.8.1).
   - Batch link suggestion (`loom_batch_link`) MAY propose `AI_SUGGESTED` edges for user review (no silent writes).

15. **Handshake Stage Phase 2 upgrades (ingestion + 3D feedback loops)** [ADD v02.131]  
   - Upgrade `stage.import_pdf.v1` to use the Docling conversion pipeline (Â§6.1) and emit structured document blocks + descriptors; original bytes remain preserved as artifacts.  
   - Add Cache-to-Index Assimilation for Stage captures: captured `artifact.snapshot` bundles can be normalized/chunked and indexed into `LocalWebCacheIndex` + Shadow Workspace (cache-before-external for later retrieval).  
   - Extend the 3D Mechanical Assist Pack with deterministic feedback loops: `stage.3d.canonicalize_gltf.v1`, `stage.3d.optimize_mesh.v1`, and `stage.3d.physics_checks.v1`, producing reviewable reports + artifacts (no silent mutation).  
   - Deliver at least one Stage App for review: scene/constraint report viewer (3D QA) and an evidence browser that deep-links Stage outputs (snapshots/clips/reports) to their jobs + provenance.  


- [ADD v02.137] **Design Studio multi-session support:**
  - Use `target_entity_ids` on `ModelSession` to bind sessions to CRDT-backed entities (canvas/doc/table).
  - Define entity-level locking discipline (Phase 2): locks are per-entity or per-node; a session can hold multiple locks; locks are released on session completion or timeout.
  - Expose "entity lock state" in the DCC Sessions panel for Design Studio workflows.
  - Add a â€œparallelizeâ€ action in Design Studio that spawns child sessions for sub-entities (copy/layout/images).

**Vertical slice**  
- Import a `.docx` or `.pdf` file.  
- Wait for ingestion to complete and open the resulting document.  
- Use "Search workspace" to find content from the imported file.  
- Ask a question that should be answered from that content; see a RAG-backed answer and inspect the corresponding jobs and logs (ingestion + retrieval + answer job).
- Open a descriptor/debug view for the imported file and inspect at least one DescriptorRow (image or text) with correct `content_tier` / `consent_profile` metadata and provenance.
- [ADD v02.40] Run one external web retrieval (allowed/consented), then repeat the same question and confirm it is answered from `LocalWebCacheIndex` (cache hit visible in RAG Debugger; no second external call).
- [ADD v02.47] Import a financial PDF â†’ extract a table â†’ create a chart referencing that table â†’ assemble a 3â€‘slide deck (title + chart + table) â†’ export PPTX â†’ verify provenance in Job History + Flight Recorder.
- [ADD v02.52] Import a PDF/DOCX, run ingestion, export Workspace Bundle; verify original bytes + canonical snapshot + Display-derived render included.


11. **[ADD v02.52] ACE-RAG-001 Retrieval Contract (RAG correctness, speed, token efficiency)**
   - Implement `ContextPack` builder job (`context_pack_builder_v0.1`) producing pack artifacts with `facts/constraints/open_loops/anchors/coverage` and SourceRefs.
   - Enforce pack freshness (`ContextPackFreshnessGuard`) using source hashes; stale packs MUST not be preferred.
   - Implement `IndexDriftGuard`:
     - Embedding drift: vector/snippet records carry `source_hash`; mismatch triggers drop/downgrade + reindex recommendation.
     - KG drift: candidates used as evidence require provenance; missing provenance disqualifies evidence use.
     - LocalWebCacheIndex drift: TTL/pinning warnings surfaced; pinned-but-stale marked clearly in traces.
   - Implement hash-key caching for retrieval stages:
     - retrieval candidate list cache (cache_kind=`retrieval_candidates`)
     - rerank order cache (cache_kind=`rerank_order`)
     - (optional) spans cache and prompt envelope cache once stable
   - Determinism split:
     - strict mode: deterministic ranking + tie-breaks; deterministic rerank only.
     - replay mode: persist candidate ids/hashes + rerank order; replay reuses persisted order.
   - Upgrade Operator Consoles RAG Query Debugger + Index Doctor to show:
     - QueryPlan + RetrievalTrace ids/hashes, route taken, cache hits/misses, candidates + scores + tie-break keys,
       selected spans + truncation flags, drift flags, and degraded-mode markers.

- [ADD v02.52] Verify ContextPacks are preferred when fresh, fall back is logged when stale; RAG Debugger shows QueryPlan/Trace, cache hit/miss, and drift flags for the answer.



12. **[ADD v02.138] Front End Memory System (FEMS) v1 (hybrid retrieval + pack governance)**  
   - Enable hybrid retrieval over `MemoryItem`s (FTS + vector + graph) with deterministic selection and replay logging (Â§2.6.6.7.6.2).  
   - Enforce per-type quotas (intent/risk/tool_protocol/etc.) and strict scope matching (workspace/project/WP).  
   - Add consolidation + conflict workflows surfaced in DCC Memory Panel; merges are `supersede`/`merge` operations with versioned history.  
   - Support optional precomputed per-WP `MemoryPack`s and pack invalidation on memory commit events.  
   - Extend cloud redaction rules for memory packs and record decisions in `ContextSnapshot`.

13. **[ADD v02.67] Atelier Lens at scale (Role lanes + organic growth controls)**
   - Implement `ATELIER_LANE_INDEX` to build role-scoped retrieval lanes (lexical + vector) over:
     - `RoleDescriptorBundle` (role overlays, evidence-linked)
     - `RoleDeliverableBundle` (typed outputs)
   - Implement organic growth queue jobs:
     - `ATELIER_VOCAB_PROPOSE` (emit proposed terms/fields with example evidence)
     - `ATELIER_VOCAB_RESOLVE` (accept/merge/reject; produces a new vocab snapshot id)
   - â€œSearch as roleâ€: add retrieval routing so role lanes are queryable and preferred when a role lens is selected.
   - Scheduling: enforce budgets so only top-k roles run deep extraction; `RoleGlance` remains cheap and non-blocking at corpus scale.
- [ADD v02.79] Import RAW+JPEG set â†’ generate previews/proxies â†’ search by metadata â†’ open from results â†’ export derivative; confirm provenance chain.



- **Loom AI loop** [ADD v02.130]
  - Select a LoomBlock (media or document) and run `loom_auto_tag`.
  - Verify suggested tags appear as suggestions (DerivedContent) without altering RawContent.
  - Accept one suggested tag: confirm it becomes a `TAG` edge and appears in backlinks/tag hub.
  - Reject one suggested tag: confirm it is removed and recorded.
  - Run Loom search with semantic/hybrid enabled and confirm results include the tagged/captioned item.

**Key risks addressed in Phase 2**
- [ADD v02.130] Loom AI trust risks: suggested tags/captions must be clearly labeled as DerivedContent, reversible, and never silently mutate RawContent; acceptance must be explicit and logged.
- [ADD v02.138] FEMS scale + privacy risk: hybrid memory retrieval can reintroduce drift or leak sensitive context unless bounded and consent-gated. Mitigation: per-type quotas, strict trust/truth gating, cloud redaction rules, DCC review, and `FR-EVT-MEM-*` auditing.


- [ADD v02.52] Ingested content cannot be backed up/moved while preserving provenance/IDs.
- [ADD v02.47] Charts/decks accidentally become a parallel datastore (data copied into chart/deck content instead of ID-based references).
- [ADD v02.47] Export provenance gaps (missing hashes/engine version/policy) prevent reproducibility and audit.
- [ADD v02.47] Export policy leakage: sensitive content exported without `SAFE_DEFAULT` gating and explicit logging.

- [ADD v02.49] Bundle hashing nondeterminism makes exports unverifiable across runs/devices; mitigate with canonical bundle hashing + recorded content_hash and per-file manifests.

- Ingestion pipeline is too brittle or slow to be practical.  
- Shadow Workspace design is wrong or too hard to debug.  
- RAG behaviour is opaque (user cannot see why an answer was produced).
- Descriptor pipelines drift from DES-001 / TXT-001 / IMG-001 law or accidentally censor/soften internal Corpus.
- Language/normalization gaps or missing audits make search/RAG results untrustworthy.
- MCP-based ingestion (Docling or other engines) diverges from non-MCP paths, leading to inconsistent capability enforcement, logging, or provenance between tool interfaces.
- [ADD v02.40] Cache growth + staleness in `LocalWebCacheIndex` leads to wrong answers or citation rot (mitigate via TTL + pinning + refresh + staleness surfacing).
- [ADD v02.40] Low-trust sources poison the local cache and outrank authoritative docs (mitigate via trust classification + downranking + cross-verification for high-impact outputs).
- [ADD v02.40] Allowlist crawling / offline mirroring violates ToS/licensing if misused (mitigate via explicit allowlists + conservative defaults + audit logs).
- [ADD v02.40] Privacy leakage: cached pages contain sensitive material that is later exported or sent to external models (mitigate via consent gates + minimization + sensitivity tagging + never writing redactions back into stored content).


8. **Calendar ingestion and ICS invite pipeline (read-only external, local drafts)**
   - Implement `calendar_sync` as a mechanical engine for **read-only** provider ingestion (CalDAV) into a local calendar store with idempotency keys and observable sync state.
   - Parse **ICS attachments** from mail ingestion into **draft** CalendarEvents (no external export) and attach provenance to the source MailMessage/thread.
   - Store and surface timezone/recurrence fields as Raw/Derived data (no advanced UI editing required in this phase).
   - **[ADD v02.63] Full Calendar Law delivery (write + recurrence + governance):** add recurrence editing UI, patch-set mutation governance (expected_etag/local_rev + provenance), identity/idempotency rules, and export/write policy enforcement aligned with AÂ§10.4.1.
   - **[ADD v02.63] ACEâ†”Calendar compatibility:** add ACE compatibility tests (scope hint, cache/prefix stability) for calendar context; ensure calendar writes/reads do not violate ACE determinism/caching rules.


- [ADD v02.44] JVM-based services (Tika/GROBID) increase packaging/ops complexity and require strict resource limits and isolation.
- [ADD v02.44] Untrusted file parsing (PDF/media) is a high-risk surface; enforce Â§11.7.4 untrusted-input policies and capture evidence/provenance for every derived artifact.
- [ADD v02.131] Stage 3D transforms risk: canonicalize/optimize/physics checks can introduce silent geometry drift unless treated as deterministic jobs with before/after hashes + diffable reports; no write-back to source assets without explicit approval and logged provenance.
- [ADD v02.67] Role-lane indexing and â€œorganic growthâ€ can degrade queryability if uncontrolled; mitigated by proposal queues, vocab snapshot ids, and lane rebuilds driven by accepted changes.
- [ADD v02.67] Lane/index drift breaks replayability; mitigated by pinned lane build configs + hash-keyed rebuild semantics and surfacing drift in Inspector/Operator Consoles.
- [ADD v02.67] Compute cost grows superlinearly with corpus size; mitigated by claim/glance/deep split with strict budgets and backpressure-aware scheduling.


- [ADD v02.68] External facts are not verifiable/replayable (link rot, changing pages, disputes); mitigated by Archivist/Guide evidence bundles and v1.2 D1 evidence requirements (replay uses captured artifacts).
- [ADD v02.79] Index bloat / perf regression â†’ index references + derived previews only (artifact-first), not raw pixels.


**Acceptance criteria**
- [ADD v02.130] Loom AI: auto-tag produces AI_SUGGESTED edges; accept/reject converts or removes; captions stored as DerivedContent with attribution; semantic/hybrid Loom search returns expected results and is observable via Flight Recorder.
- [ADD v02.138] FEMS v1: hybrid memory retrieval produces bounded `MemoryPack`s with quotas; DCC shows pack preview + memory review queue; commits emit `FR-EVT-MEM-*`; replay reproduces pack hash; consolidation produces supersedence history (no silent overwrites).
- [ADD v02.131] Stage PDF import: `stage.import_pdf.v1` runs through the Docling/MCP path and produces structured doc blocks + descriptors; original PDF bytes remain preserved as an artifact; job is inspectable in Job History/Flight Recorder.
- [ADD v02.131] Stage capture assimilation: captured `artifact.snapshot` bundles become searchable via `LocalWebCacheIndex`/Shadow Workspace and can satisfy later retrieval with stable artifact refs (no second external fetch when cache is fresh).
- [ADD v02.131] Stage 3D feedback loops: `stage.3d.canonicalize_gltf.v1`/`stage.3d.optimize_mesh.v1`/`stage.3d.physics_checks.v1` produce deterministic reports with before/after hashes; Stage App can review reports and any write-back requires explicit approval + logged provenance.


- [ADD v02.47] Chart stores only spec + entity refs; table edits update render; no duplicated table rows/cells exist in Chart RawContent.
- [ADD v02.47] Deck export produces artifact references only and records: deck_id/slide_ids, referenced entity IDs + hashes, export engine + version, and export policy.
- [ADD v02.47] Chart/deck create/update/export operations are visible as explicit jobs (with previews, validators, and Flight Recorder traces).

- At least one external calendar can be ingested read-only via `calendar_sync` and rendered in the Calendar surface with provenance and sync diagnostics visible.
- **[ADD v02.63] Calendar Law compliance tests pass (AÂ§5.4.6.4) and ACEâ†”Calendar compatibility tests added; recurrence editing/write governance enforced per AÂ§10.4.1 (patch-set, idempotency, identity).**
- **[ADD v02.63] Contextual hardening primitives:** audit trail and retention policies are recorded (TraceRetentionPolicy/AuditTrail/CapabilityGrant logs); capability grants/denials and retention/redaction defaults are visible in FR/Problems; schema/docs updated for these primitives.
- Ingestion jobs are visible and inspectable in Job History and Flight Recorder.
- Shadow Workspace can be inspected via logs or a debug view (e.g. number of indexed documents, last index time).
- Shadow Workspace inspection is available via Operator Consoles (Index Doctor) and supports rebuild/backfill with FR+Problems linkage.
  
- For at least one RAG scenario, you can show which documents and snippets were used to produce an answer.
- RAG Query Debugger can show the ranked retrieval set and prompt-budget/truncation decisions for that answer.
- [ADD v02.40] After any external fetch, the cached artifact becomes locally searchable and can satisfy subsequent retrieval without another external call (cache hit visible in RAG Debugger).
- [ADD v02.40] TTL + pinning behavior is testable: pinned sources survive eviction; expired sources are marked stale; refresh path works under consent policy.
- [ADD v02.40] Evidence logs include `trust_class`, `fetch_depth`, and `cached_artifact_ref` for cached sources; retrieval logs record cache-hit vs external-hit.
- [ADD v02.42] RAG Query Debugger surfaces per-item `fetch_depth` and escalation rationale; section reads are bounded to heading/anchor ranges with stable citations/SourceRefs.


- At least one Docling ingestion job runs via MCP (server or sidecar) end-to-end, with progress updates and `logging/message` events visible in Job History and Flight Recorder, and consistent provenance/capability metadata.
- Descriptor extraction jobs appear in Job History and Flight Recorder with clear linkage to source documents/assets.  
- For at least one imported file, descriptor rows can be inspected from a debug view or console query, showing stable IDs, schema versions, and consent/content-tier fields.  
- A SFW-only workspace/search mode can be toggled to filter NSFW descriptors without modifying underlying Corpus rows.
- Shadow Workspace under churn shows index freshness/backlog metrics; rebuild/backfill command succeeds.  
- RAG jobs log retrieved snippet IDs/hashes and prompt budget/truncation flags.  
- Descriptor law enforced: sampled descriptors show schema version, consent/content-tier, nsfw flag, sidecar-only provenance; unit/physics middleware normalizes sheets/docs ingestion.  
- Monaco initial gating: LSP/diagnostics routed through capability-aware gates; worker routing configured; FR logs model-assisted code actions.  
- Physics/dimension_check validator enforced in Sheets/Docs: unit errors flagged, normalized values returned, FR logs validator outcomes.  
- Wrangler/DBA outputs open in Sheets/Canvas grid views; FR entries carry doc/table/entity references.  
- Mail ingestion: fixture sync shows MailMessages with correct IDs/thread linkage; attachments processed (Docling/OCR/ASR) with FR provenance; TXT-001 mail descriptors stored under `MAIL_COMMUNICATION`; `READ_EMAIL` enforced/FR-logged; health check exposes latest modseq/sync time.
- Normalization/quality: language tags recorded (Detector), encoding cleanup applied (Converter), lemma/stem normalization available (Morphologist), audits and workflow routing logged (Inspector/Router).
- SDXL/ComfyUI render: given a local ComfyUI server and the pinned SDXL graph, a render job runs via Workflow Engine, is visible in Job History/FR with model/workflow hashes and params, is capability-gated, enforces timeout/output caps, passes health check, and produces a stored artifact with sidecar provenance (hashes, params, request/response trace).  
- [ADD v02.36] Debug Bundle for Docling/RAG runs includes ingestion logs + indexing config + retrieval set (IDs/hashes) + model/version metadata, redacted-by-default.
- [ADD v02.36] RAG jobs enforce evidence binding per policy (fail or mark incomplete) and surface the reason in Problems/Evidence.


- [ADD v02.44] Mixed corpus ingest (scientific PDF + images) produces Shadow Workspace entities and RAG citations with provenance back to source artifacts.
- [ADD v02.44] Asset ingestion produces deterministic derived artifacts (thumbnails/metadata sidecars) and dedupe results stored as job artifacts.
- [ADD v02.49] Deck export bundles include canonical bundle hash and per-file manifests (or embedded `bundle_index.json`); ExportRecord includes export policy, engine/version/config hash, and SHA-256 hashes.
- [ADD v02.49] Ingestion-derived artifacts (tables/figures/thumbnails) can be pinned and survive TTL/GC; retention state is inspectable via Operator Consoles.

- [ADD v02.52] ACE-RAG-001 conformance tests pass (T-ACE-RAG-001..007): normalization determinism, strict ranking determinism, replay persistence, pack freshness invalidation, budget enforcement, drift detection, cache invalidation.
- [ADD v02.52] Repeating the same query over unchanged sources shows cache hits in RetrievalTrace and reduced retrieval latency (measured in Flight Recorder).
- [ADD v02.52] RAG Debugger deep-links: Answer â†’ RetrievalTrace â†’ selected spans â†’ source documents; every evidence item is bounded and provenance-carrying.
- [ADD v02.52] export_report lists inclusions/exclusions and reasons; denials visible in Problems + Flight Recorder.
- [ADD v02.52] exported entities preserve stable IDs referenced by jobs/workflows.


- [ADD v02.67] Role-lane search exists: selecting a role lens routes queries through the correct lane index, and results show evidence refs and provenance.
- [ADD v02.67] Vocab/schema proposal queue works end-to-end (propose â†’ review/resolve â†’ new snapshot id â†’ lane rebuild) and produces audit logs in Flight Recorder/Operator Consoles.
- [ADD v02.67] Lane rebuilds are deterministic under pinned configs and surface drift flags when underlying sources change.
- [ADD v02.79] Photo search returns correct assets by metadata/collection; opening a result shows traceable derivation artifacts.


**Explicitly OUT of scope**
- [ADD v02.130] Loom: real-time multi-user collaboration, cross-device sync semantics, and Postgres-backed Loom view engines remain Phase 4.
- [ADD v02.131] Stage: full Stage Studio authoring and advanced 3D editing (Spline-class), third-party Stage App marketplace/extensions, and cross-device/multi-user Stage sessions remain Phase 3+ / Phase 4.


- [ADD v02.47] Extension marketplace for chart/deck templates, thirdâ€‘party exporters, and collaborative review/commenting (defer to Phase 4).

- **[ADD v02.63] External calendar provider write-back (CalDAV PUT/DELETE) remains out-of-scope; Phase 2 covers local recurrence/editing only.**
- Advanced knowledge graph visualization.
- Complex retriever configuration UIs.
- Graph/node authoring UI for image workflows, advanced ComfyUI graphs, or multi-model routing; only the pinned SDXL workflow is in scope.  
- Taste Engine training and symbolic profiles (SYM-001); Phase 2 only requires raw descriptor extraction and wiring.
- ASR transcription capabilities (e.g., Whisper/FFmpeg) and related media pipelines.
- Advanced multi-agent orchestration beyond basic agent invocation (e.g., AutoGen/LangGraph multi-step flows).
- Full mail client functionality beyond read-only ingestion and rendering.
- [ADD v02.40] Bulk crawling/archiving of arbitrary websites beyond explicit allowlist runs (no â€œmirror the webâ€ mode).


- [ADD v02.52] Cross-device/multi-user caching and pack-regeneration policy (collaboration-safe semantics) â€” Phase 4.
- [ADD v02.52] â€œRehydrate full index from bundleâ€ as a supported workflow (future phase).
- [ADD v02.52] publishing bundles as shareable links.

- [ADD v02.79] Full layer compositor, advanced masking, merges, AI generation.


**Mechanical Track (Phase 2)**
- [ADD v02.130] Loom Tierâ€‘3 index build: embedding generation jobs for LoomBlocks + incremental refresh; hybrid retrieval wiring in Shadow Workspace.
- [ADD v02.131] Stage ingestion + 3D mechanicals: Stage snapshot normalization/index jobs (Archivistâ†’Indexer) and 3D assist engines (canonicalize/optimize/physics) run via Tool Bus with deterministic outputs, before/after hashes, and evidence artifacts; Stage App review required for any asset mutation.
- [ADD v02.136] **Handshake as MCP server (local)**: expose a curated subset of Tool Registry tools over MCP for cloud-model orchestration. MUST be capability-scoped, secret-safe (redaction), and Tool-Gate enforced.
- [ADD v02.136] **Tool Search / deferred loading** (Â§6.0.2.7): implement `handshake.tools.search`/`handshake.tools.get` backed by Tool Registry tool packs to reduce context bloat as tool count grows.


- [ADD v02.47] Deck export mechanical jobs: `deck_export_pptx` (PptxGenJS) and optional `deck_export_pdf`/`deck_export_html`; capability-gated; Flight Recorder logs artifact hashes + engine/version + policy.
- [ADD v02.47] Chart/deck validators as mechanical gates: `chart_spec_schema`, `no_parallel_store`, `artifact_reference_only`, and `export_policy_gate` enforced and logged.
- Ingestion-focused engines: `Archivist` (SingleFile/yt-dlp), `Librarian` (metadata/BibTeX/EXIF), `Wrangler` (Great Expectations/csvkit), `DBA` (DuckDB/sqlite-utils/Tantivy), `Indexer` (Shadow Workspace indexing), `Router` (workflow data-flow orchestration), and `Inspector` (data audits).
- Descriptor/normalization engines: wire IMG-001 / TXT-001 helpers, add `Converter` (text/encoding normalization), `Morphologist` (lemma/stem), and `Detector` (language ID) into the mechanical runner with DES-001 CORPUS/Sidecar and capability gates.
- Add unit/physics middleware for sheets/doc ingestion to enforce unit consistency and conversions.
- Acceptance: ingestion/descriptor/indexing runs show mechanical provenance in Flight Recorder; Shadow Workspace metrics/logs expose indexed docs and language/normalization steps; RAG answers list retrieved snippets and source artifacts; data audits (Inspector/Router) log routing decisions and invariants.
- When these engines are exposed as remote services, they SHOULD prefer MCP as the tool interface and MUST route calls through the same MCP Gate and Flight Recorder paths as other AI jobs (Sections 2.6.6 and 11.3).
- [ADD v02.40] Implement Cache-to-Index Assimilation as a mechanical job using existing engines (`Archivist` capture â†’ `Indexer` normalize/chunk/index â†’ `Inspector` TTL/pinning audits) and log all steps in Flight Recorder.
- [ADD v02.52] `workspace_bundle_export_v0` supports inclusion of imported raw assets + selected derived sidecars (policy-gated).
- **[ADD v02.63] OS primitives track (Window/Shell Engines):** add UI automation and sandboxed shell mechanical jobs (capability-gated, logged, bounded outputs) with FR/Job History visibility and health/timeout/quotas.

- [ADD v02.115] **MT Executor context compilation engine:** Implement MT context compilation as mechanical job using ACE Runtime (Â§2.6.6.7) + ContextPacks (Â§2.6.6.7.14.7) for deterministic, bounded context assembly per iteration.
- [ADD v02.115] **MT validation command registry:** Extend validation command allowlist with project-type inference (Cargo.toml â†’ cargo check/test, package.json â†’ npm test, etc.); log all validation runs via Mechanical Tool Bus.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 2:**
  - Implement two-stage retrieval with cross-encoder reranking (25-48% ranking improvement)
  - Implement parent document retrieval (retrieve small chunks, expand to larger context)
  - Implement contextual prefix generation (LLM-generated explanatory text, 35-67% failure reduction)
  - Integrate Knowledge Graph relationships (18 types per Â§2.3.14.6) with hybrid search
  - Implement cross-modal embedding (CLIP for images) for Photo Studio integration
  - Implement context pollution scoring and alerts (FR-EVT-DATA-011)
  - Wire data validation jobs (`ingestion_validator_v1`, `retrieval_quality_audit_v1`) to Mechanical Tool Bus
  - Implement golden query testing for quality regression detection
  - Acceptance: reranking visible in retrieval traces; Photo Studio images searchable via text; pollution alerts trigger on threshold breach

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 2:**
  - Implement hybrid search (vector HNSW + keyword BM25 + graph traversal) for WP/MT search (Â§2.3.15.7)
  - Implement Calendar policy integration: policy profiles modulate locus_query_ready results (Focus Mode, Sprint Mode, etc.) (Â§2.3.15.4)
  - Implement dependency graph queries: getDependencyTree, getBlockingChain with O(1) Knowledge Graph traversal (Â§2.3.15.7)
  - Implement WP priority escalation based on Calendar/context signals
  - Implement migration tools: import from external systems (GitHub Issues, etc.) with metadata preservation
  - Wire to Operator Consoles: Locus Query Explorer with dependency visualization, MT execution timeline viewer
  - Extend Flight Recorder analytics: WP completion velocity, MT iteration histograms, escalation pattern analysis
  - Acceptance: hybrid search returns semantically relevant WPs; Calendar policy boosts prioritized work; dependency graph queries complete <100ms; Operator Console exposes Locus metrics




- [ADD v02.44] Creative asset ingestion jobs: thumbnails (libvips), metadata extraction (Tika + ExifTool), dedupe scanning (Czkawka) with provenance sidecars.
- [ADD v02.44] Bibliography primitives + tooling integration: `science.primitives.bibliography` (JabRef / Hayagriva / Citation.js; citeproc-rs only with MPL obligations tracked) and coupling to paper ingestion outputs.
- [ADD v02.44] Local analytics substrate + unit validator: `science.analytics.local` (Arrow + DuckDB) and `science.validators.units_dimension` (Pint) as gates for derived tables/params.
- [ADD v02.67] Implement `ATELIER_LANE_INDEX`, `ATELIER_VOCAB_PROPOSE`, and `ATELIER_VOCAB_RESOLVE` as mechanical job profiles (capability-gated; logged; deterministic outputs via snapshot ids).
- [ADD v02.67] Add lane/index Inspector audits (coverage, drift, rebuild triggers) and render lane status in Operator Consoles.


- [ADD v02.68] Apply v1.2 determinism/evidence policy: `Archivist` and `Guide` operations classified as D1 MUST emit evidence artifacts; conformance harness includes evidence-required checks and replay expectations.
- [ADD v02.79] Implement `engine.raw_decode` + minimal `engine.photo_develop` CPU path behind tool bus (pinned versions; determinism class recorded).


**Atelier Track (Phase 2)**
- [ADD v02.67] Expand the Role Registry materially (still contract-first); add at least one additional Finishing role and one â€œculture/contextâ€ advisor role as dual contracts.
- [ADD v02.67] Implement one real Nested Production dependency chain as a production graph fixture (e.g., `graphic_design` produces a pattern deliverable consumed by `fashion.styling` synthesis), and ensure `ATELIER_GRAPH_SOLVE` schedules it deterministically.
- [ADD v02.67] Add role-lens retrieval UI: â€œSearch as roleâ€, lane selection, and per-role result explanation (why this role claimed it + evidence refs).
- [ADD v02.79] Add collections/smart filters UI + search entrypoint wired to Shadow Workspace results.


- [ADD v02.115] **MT smart drop-back:** Implement "smart" drop_back_strategy that considers LoRA training history, failure category patterns, and remaining MT complexity before returning to smaller models.
- [ADD v02.115] **MT ContextPacks integration:** Wire MT context compilation to use ContextPacks (Â§2.6.6.7.14.7) for efficient file retrieval with staleness detection.
- [ADD v02.123] Implement **Tier2 auto-when-idle** scheduling for Lens extraction: Tier2 deep passes MUST auto-trigger when the app is idle (and within configured GPU/CPU budgets), queue jobs deterministically, and emit provenance + evidence. Tier1 remains default and synchronous.
- [ADD v02.123] Implement **Facts substrate** for Lens: normalized `AtelierFact` + `AtelierSymbolFact` as first-class indexed records (global across projects, filterable) with EvidenceRef pointers and deterministic retrieval traces.
- [ADD v02.123] Implement **SymbolismProfile + SymbolLexiconSnapshot** persistence and drift-safe evolution (branchable profiles): symbolic templates MUST grow over time; unknown/unclear fields MUST be stored explicitly as `unclear`/`not_available` (never omitted silently).
- [ADD v02.123] Implement Lens **global filter UI** (role filters, source filters, ViewMode toggle default NSFW, projection markers) and an Operator Console QueryPlan/RetrievalTrace viewer for Lens queries (debuggable click-to-source).
- [ADD v02.123] Implement Lens persistence contract extensions in SQLite (and migration notes for Postgres later): deterministic artifact layout for Derived Fact stores, schema versioning, and replayable QueryPlan/Trace storage.

**Distillation Track (Phase 2)**
- Persist Skill Bank entries from real workflows; implement sample selection and teacher-eval jobs flowing through Workflow Engine.
- Define and version the distillation eval suite; log pass@k/compile/test and collapse indicators; no student promotion yet.
- Acceptance: Skill Bank artifacts stored with provenance and export controls; eval runs visible in Flight Recorder with required metrics and lineage fields.
- [ADD v02.157] Adapter-only LoRA / QLoRA / DoRA training remains benchmark-gated: rank/alpha/repeats/epochs, adapter merge posture, and rollback lineage MUST be recorded as first-class evaluation metadata before any promotion path is allowed.



- [ADD v02.115] **[REMEDIATION] MT Executor LoRA feedback loop (Â§2.6.6.8.8):** Aggregate escalation_records by (lora_id, failure_category, task_tags); update lora_registry weak_on_task_types and training_priority; trigger automated distillation jobs when sufficient candidates accumulate. (Extends existing Skill Bank schemas.)
- [ADD v02.115] **MT smart drop-back implementation:** Implement ShouldDropBack algorithm checking failure_category, lora_training_updates, remaining_mt_complexity; record drop-back decisions with outcomes for threshold refinement.
- [ADD v02.115] Acceptance: LoRA training priority updates visible in Skill Bank; drop-back decisions logged (FR-EVT-MT-014); at least one LoRA improvement cycle demonstrated end-to-end.
- [ADD v02.79] Preset/recipe serialization rules finalized enough for â€œshareable presetsâ€ (still single-user).

### 7.6.5 Phase 3 â€” ASR & Long-Form Capture

**Goal**  
Add lecture/meeting capture via ASR, using the same AI Job, workflow, and observability primitives. Debugging ASR behaviour must be possible from logs and UI.

- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.52] Extend ACE-RAG-001 evidence selectors to transcripts (time-range SourceRefs), so Q&A over ASR outputs uses the same budgets/traces/drift guards.

- [ADD v02.68] Media mechanical jobs (Director/Composer) adopt v1.2 determinism classes (D2/D3 when possible) and emit conformance vectors + provenance so renders can be re-run and compared.

- [ADD v02.79] Introduce **vision-derived metadata** for photos using the same provenance/drift discipline Phase 3 applies to other stochastic outputs.
- **[ADD v02.131] Stage Studio baseline (3D provenance review)**  
  Extend Stage from capture-only into provenance-first 3D review: deterministic analysis/validation/canonicalization jobs produce reports and patch sets; a Stage App can review/apply patch sets via governed workspace mutations (no silent asset edits).



- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**  
  Validate `settings.exec_policy` end-to-end on at least one **local** runtime, including downgrade semantics and Flight Recorder observability, and run initial approximate experiments under waiver control.

**MUST deliver**

1. **ASR engine integration**  
   - Integrate an ASR engine (e.g. Whisper / whisper-rs) as described in Section 6.2.  
   - Use an optimized runtime (e.g., **Faster-Whisper** or **whisper-rs/whisper.cpp**) for batch transcription of locally stored audio/video files.  
   - Normalize media via **FFmpeg** (audio extraction, resample, channel/bitrate caps) before ASR.  
   - Log ASR runs with duration, model, and basic quality-related metrics (where available) into Flight Recorder or a dedicated ASR log.

2. **ASR AI Job profile**  
   - Implement the ASR profile:
     - `media_id`, time ranges, diarization flags, language configuration, provenance.  
   - ASR jobs MUST flow through the Workflow Engine and log into Flight Recorder like any other AI job.  
   - Expose ASR-specific status and errors (e.g. decoding failure, unsupported format) in Job History.

3. **Transcription UX**  
   - "Transcribe file" flow:
     - Drop audio/video into Handshake.  
     - See job progress and final transcript document with segments and timestamps.  
   - Transcripts MUST be regular workspace documents, subject to the same governance and editing rules as other documents.  
   - Provide at least one debug surface that shows:
     - Segment-level timeline (timestamps, diarization labels where present) and a way to open the related Flight Recorder slice.
     - A one-click Debug Bundle export for the ASR job (media hash + segment ranges + ASR config + diagnostics), redacted-by-default.

     - Input file details.  
     - Chosen model and parameters.  
      - Any segmentation or diarization decisions, where applicable.
   - (Optional) Diarization: integrate **pyannote.audio** (or equivalent) as an opt-in stage; outputs recorded as overlays/metadata, not mutating base transcripts.

4. **ASR/multi-agent orchestration (minimal)**
   - For long-form or multi-step ASR flows, allow orchestration via a minimal multi-agent framework (e.g., AutoGen or LangGraph) to manage chunking, retries, and QC.
   - All orchestrated steps MUST still run through Workflow Engine + Flight Recorder; no direct model calls.
5. **Mail AI jobs and drafting/sending**
   - Local-only jobs: `mail_summarize_thread_v0.1`, `mail_triage_inbox_v0.1`, `mail_thread_to_doc_v0.1` using local models.  
   - Drafting: `mail_draft_reply_v0.1` + mechanical `email_send` engine with `from_identity`, pre-send checks (Red Pen, Anonymizer, classification validation), before/after diff, provenance.  
   - Capabilities: `SEND_EMAIL` required; `require_confirmation = true` for AI send flows; policy-based routing for local vs cloud models (default local-only).

   - [ADD v02.138] CRM/Contact profiles (minimal): introduce `ContactCard` + contact-scoped `MemoryItem`s (`relationship_note`, `preference`) for mail drafting; default to local-only; require explicit consent/redaction for any cloud-bound prompts.

6. **NLP overlays and curation (derived-only)**
   - Add `Aligner` (parallel text), `Lexicographer` (dictionary/thesaurus), and `Curator` (collections/playlists) as Workflow Engine jobs producing DerivedContent overlays only (no schema changes to descriptors).  
   - Provenance: log inputs/outputs in Flight Recorder and attach overlays to source documents/descriptors by reference with capability gates.



7. **[ADD v02.47] Transcript â†’ Deck summary (reuse Decks)**  
   - Add a `transcript_to_deck` AI job that generates a deck from a transcript doc (agenda, key points, action items).  
   - Export via the same `deck_export_*` mechanical jobs; record provenance and policy like other exports.

8. **[ADD v02.49] ASR artifact discipline (manifests + retention + bundle hashing)**  
   - ASR outputs (transcripts, segments, debug bundles) MUST emit artifact manifests with SHA-256, respect TTL/pinning defaults, and use canonical bundle hashing for exported Debug Bundles.


9. **[ADD v02.79] Photo Vision v0 (metadata-only)**  
   - [ADD v02.79] `engine.vision` jobs: `tag`, `describe`, `analyze_quality`, optional `ocr`; outputs stored as `.hs.ai.json` artifacts with model/version/params recorded.  


10. **[ADD v02.131] Stage Studio baseline (provenance-first 3D review)**  
   - Provide a Stage Studio review surface (Stage App) that can open 3D assets + reports, show `scene_ir` diffs, and review/apply patch sets (workspace mutations only via `stage.workspace.applyPatchSet`).  
   - Add a governed job `stage.3d.apply_patchset.v1` that takes (source asset, patchset artifact) and produces a **new derived asset** (no in-place mutation) with before/after hashes + provenance + evidence bundles.  
   - Add evidence bundle export for 3D jobs (inputs, reports, patchset, derived outputs, hashes) suitable for replay and audit.

- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Implement a local runtime adapter that accepts `exec_policy` and emits `llm_exec_policy` (FR-EVT-LLM-EXEC-001) with requested vs effective policy.  
  - Add at least one approximate execution implementation for local models (e.g., LayerSkip / early-exit), gated behind waiver + Work Profile toggle.  
  - Benchmark: produce a comparative report (exact vs fast_exact vs fast_approx) with latency, error rates, and quality proxies, attached to the job artifacts.  
  - Ensure high-volume traces are sampled and privacy-compliant (no token IDs; no raw text by default).

- [ADD v02.137] **Multi-session automation upgrades:** introduce guarded semi-autonomous loops over `MultiModelSession` (AutomationLevel.ASSISTED), with explicit budget caps, spawn limits, and FR-logged policy decisions.
- [ADD v02.137] **Workflow alignment:** represent `MultiModelSession` execution as (or within) a `workflow_run` that schedules child `model_run` jobs, so orchestration is auditable via the standard AI Job Model primitives.

**Vertical slice**  
- Drop an audio file into Handshake.  
- Run transcription and see progress.  
- Open the resulting transcript document.  
- Inspect Job History and Flight Recorder entries for the ASR job and confirm model choice, status transitions, and any errors.

9. **[ADD v02.52] Transcript retrieval compatibility (ACE-RAG-001)**
   - Define transcript selectors (`ts_range`, `segment_id`) as bounded selectors for `SourceRef` so reads are time-range bounded.
   - Store `source_hash` per transcript segment (and per derived chunk) so IndexDriftGuard can detect ASR regeneration drift.
   - ContextPack builder MUST support transcript targets and emit timestamped anchors (timecode + excerpt hint).
   - RAG Debugger MUST render transcript spans as time ranges and deep-link to the underlying media segment where available.

- [ADD v02.52] Ask a question over a transcript; RetrievalTrace shows timestamp spans, budgets, and (if applicable) drift flags when transcript is regenerated.


- [ADD v02.67] Extend Atelier Lens to time-based media:
  - Treat transcript spans and video time ranges as first-class `EvidenceRef.time_span` targets for role bundles.
  - Implement/validate at least one post-production role pipeline over long-form media (Editor or Color) using time-span evidence and producing typed deliverables.
- [ADD v02.79] Run auto-tag + quality scoring over a batch â†’ build a smart collection from tags/scores â†’ inspect diffs/drift via logs.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**  
  A local worker role runs `fast_approx` under a waiver on a LayerSkip-capable model; `llm_exec_policy` shows approximate active + trace reference; system falls back to exact if unsupported.

**Key risks addressed in Phase 3**

- ASR pipeline is unreliable or too opaque.  
- Long-form capture produces transcripts that are hard to relate back to source media.  
- ASR jobs are not easily distinguishable or debuggable compared to other jobs.
- [ADD v02.138] CRM/memory privacy risk: contact-scoped memory used for mail drafting can leak PII or introduce unwanted bias. Mitigation: local-only defaults, classification + consent gating for cloud prompts, and DCC review of contact memory.
- Symbolic/Taste engines (SYM-001 + Taste Engine) accidentally redefine descriptor law or mutate descriptor rows instead of adding separate Derived overlays.
- NLP/media helpers could drift or silently alter base descriptors without overlays/provenance.
- MCP-based distillation/sampling flows accidentally run with write-capable tools or bypass the Gate/logging path, causing side effects or untraceable model changes.


- [ADD v02.52] ASR transcript regeneration changes evidence silently (drift); mitigated by per-segment source_hash + drift guard + explicit degraded-mode warnings.
- [ADD v02.131] Stage 3D patchset risk: applying canonicalization/optimization patches can mutate assets in ways that are hard to audit; mitigate via patchset artifacts, derived-asset-only outputs (no in-place mutation), and mandatory before/after hashes + diffable reports.


- [ADD v02.67] Time-based evidence is hard to audit and drifts when media is re-encoded; mitigated by time-span EvidenceRefs, per-segment/source hashes, and explicit drift flags in retrieval/debugger surfaces.
- [ADD v02.79] Stochastic drift and silent regressions â†’ require model/version pinning and artifacted outputs with comparison tools.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Runtime fragmentation (policy requested but not observable).  
  - Quality regression without accountability.  
  - Trace overhead and privacy leakage.

**Acceptance criteria**

- [ADD v02.131] Stage Studio: a 3D report + patchset can be reviewed in a Stage App; applying a patchset produces a new derived asset with before/after hashes and provenance; allow/deny and mutations are logged in Flight Recorder/Job History.


- At least one realistic audio file can be ingested, transcribed, and inspected end-to-end.  
- ASR jobs appear clearly in Job History and can be filtered and inspected separately.  
- Logs and debug views provide enough information to reason about ASR failures or poor transcripts.
- Scheduler/backpressure: queue depth/latency metrics visible; back-pressure behavior under load documented for ASR/media jobs.  
- Eval harness: a golden/eval suite runs and persists metrics; LLM-as-judge outputs (where used) recorded in Flight Recorder.  
- At least one distillation/promotion cycle uses the MCP sampling pipeline end-to-end (Student â†” Teacher), with sampling calls and eval metrics visible in Flight Recorder and tied to Skill Bank entries and checkpoints.
- Monaco AI actions respect capability scopes; AI-assisted code actions log provenance to Flight Recorder.
- Director/Composer/Atelier/Artist outputs attach to Canvas media nodes and embedded blocks with Flight Recorder provenance back to source plans/prompts/files.  
- Mail jobs: Job History/FR show mail job inputs/outputs (thread IDs, attachments used); draft-to-send flow blocked without `SEND_EMAIL`; confirmation logged; local-only policy enforced unless classification allows; pre-send checks + diffs/provenance recorded.
- [ADD v02.138] CRM/Contact memory: contact-scoped `MemoryItem`s can be created and reviewed; mail drafting may use them when scoped; cloud prompts omit high-sensitivity contact memory by default; DCC shows review + pack preview.
- NLP overlays/curation: Aligner/Lexicographer/Curator jobs recorded in FR with inputs/outputs; overlays attach by reference to sources; capability gates enforced; base descriptor/schema left unchanged.
- ASR media pre-processing: FFmpeg normalization parameters and audio stream selection recorded in FR; pre-processing failures expose logs.  
- Optional diarization: if enabled, diarization overlays/metadata appear with speaker labels and timestamps; base transcript stays unchanged; provenance recorded.  
- [ADD v02.36] ASR jobs are durable across restart (workflow state + artifacts remain inspectable).
- [ADD v02.36] A transcript can be regenerated from stored refs/config and compared (structure-level diff is acceptable).



- [ADD v02.44] A notebook/job run can be re-executed from stored inputs and recorded environment metadata; failures yield typed diagnostics and artifacts.
- [ADD v02.49] Demonstrate TTL + pinning on an ASR transcript + associated artifacts; expired unpinned artifacts are GCâ€™d and a `gc_report` artifact is emitted.
- [ADD v02.49] ASR Debug Bundle export uses canonical bundle hashing; same inputs/config produce a stable structural hash, and per-file manifests/bundle_index are recorded.

- [ADD v02.52] Transcript Q&A uses the same QueryPlan/Trace pipeline; evidence spans are time-bounded and replayable.


- [ADD v02.67] At least one long-form media artifact (audio/video) produces role bundles with `EvidenceRef.time_span` and clickable deep-links to the source segment.
- [ADD v02.67] A post-production role (Editor or Color) runs end-to-end on long-form media and emits typed deliverables with provenance; time-based Lens validators pass (including evidence presence and drift detection where applicable).
- [ADD v02.79] Re-running the same vision job under same model/version produces â€œwithin-policyâ€ stable structure; drift is detectable.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Operator can run the same WP/MT twice (exact vs approximate with waiver) and see:
    - effective policy differences,
    - latency/throughput deltas,
    - explicit waiver linkage in telemetry.

**Explicitly OUT of scope**

- [ADD v02.131] Stage Studio full authoring (Spline-class), complex rigging/animation editors, and collaborative real-time 3D editing remain Phase 4.


- Real-time streaming captions.  
- Fine-tuning workflows for ASR models.  
- Complex diarization and speaker management UIs.
- [ADD v02.79] AI masks/segmentation, ComfyUI, HDR/pano/focus merges.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Production-grade automatic policy selection.  
  - Cloud-provider dynamic depth controls beyond â€œreasoning strengthâ€ tags.

**Mechanical Track (Phase 3)**
- [ADD v02.131] Stage 3D mechanicals: implement `stage.3d.apply_patchset.v1` + scene-diff generation as mechanical jobs (artifact-first I/O); patchsets are artifacts; derived-asset outputs only; before/after hashes + provenance logged and reviewable.
- [ADD v02.136] **Programmatic tool calling (Code Mode)** (Â§6.0.2.8): provide a sandboxed â€œcode executionâ€ tool that can call other tools via the Tool Registry SDK, enabling deterministic loops/batching without round-tripping large tool outputs through the LLM context.

- Media engines: `Director` (FFmpeg/Manim) and `Composer` (LilyPond/Music21) producing DerivedContent with sidecar command/param/hash provenance.
- Composer MAY use **SoX** or **LMMS** for audio processing/rendering alongside LilyPond/Music21; provenance must include tool/version/params.  
- ASR jobs and media jobs MUST share the same Flight Recorder schema, capability gates, and Job History filtering.
- Acceptance: at least one ASR run and one media render each have full logs (model/tool, params, timings) and artifacts accessible from Job History.
- Include `Atelier` (creative planning) + `Artist` (image/raster/vector rendering) for creative assets; capability gating and provenance logging; plans and artifacts attach to Docs/Canvas.
- Add NLP/media helpers: `Aligner` (parallel text), `Lexicographer` (dictionary/thesaurus), and `Curator` (collections/playlists) with provenance and capability gates; wire their outputs as DerivedContent overlays.

- **Taste Engine + SYM-001 as descriptor consumers (not law changes)**  
  - Wire the Taste Engine and SYM-001 so that they operate strictly as consumers of existing `DescriptorRow` and `TextDescriptorRow` data from Phase 2.  
  - Any new fields (layer scores, motif activations, taste scores) are stored as **DerivedContent overlays**, not as edits to the underlying descriptor rows.  
  - Phase 3 MUST NOT change DES-001 / IMG-001 / TXT-001 schema or invariants; any evolution of descriptor law happens in a future, explicit schema-migration phase.  
  - Acceptance: at least one Taste Engine / SYM-001 job can be inspected in Flight Recorder showing:
    - Inputs: descriptor IDs only (no direct RawContent edits).  
    - Outputs: symbolic/taste artifacts linked by reference to descriptors.  
    - No mutations to descriptor rows in the Corpus.
- [ADD v02.36] ASR job profile records model name/version + diarization/segmentation config + artifact hashes/handles to support reproducibility and comparisons.



- [ADD v02.44] Notebook execution as Jobs: `science.jobs.notebook_engine` (Jupyter-backed) producing typed outputs + artifacts and capturing failures as diagnostics.
- [ADD v02.44] Reproducibility bundles: `science.repro.run_bundles` (ReproZip capture) tied to notebook/script runs with stored bundle artifacts.
- [ADD v02.67] Ensure ASR/Media jobs emit time-span EvidenceRefs compatible with Atelier Lens and that Lens jobs can consume ASR transcript selectors and frame extracts without unbounded reads.


- [ADD v02.68] Conformance for media engines: Director/Composer runs must satisfy v1.2 conformance (artifact-only I/O, budget caps, provenance completeness, determinism class declared). D2 outputs MUST include canonicalization rules for stable structural hashes.
- [ADD v02.79] Implement `engine.vision` wrapper as governed job; proxy-first inputs by default.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 3:**
  - Implement event-driven index updates (<5s latency for content changes)
  - Implement semantic chunking for prose/notes (topic boundary detection via embedding similarity)
  - Implement multi-granularity indexing (paragraph, section, document levels searchable)
  - **[REMEDIATION]** Wire Skill Bank training data selection from retrieval quality signals (Â§2.3.14.9) - extends existing Skill Bank schemas with data_trust_score and retrieval feedback
  - Implement `data_quality_audit_v1` mechanical job for periodic health checks
  - Implement `antipattern_detector_v1` for automated anti-pattern detection and remediation suggestions
  - Acceptance: index updates propagate within 5s; multi-granularity retrieval visible in debugger; Skill Bank receives retrieval quality signals

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 3:**
  - Implement PostgreSQL backend with workspace multi-tenancy (Â§2.3.15.5, Â§2.3.15.8)
  - Implement CRDT operation-based conflict resolution with vector clocks (Â§2.3.15.6)
  - Implement WebSocket real-time collaboration (PostgreSQL LISTEN/NOTIFY + WebSocket broadcast) (Â§2.3.15.8)
  - Implement workspace member roles and permissions (owner, admin, member, viewer)
  - Implement cross-workspace WP references (e.g., vendor WPs in shared workspace)
  - Implement WP auto-archival policy (configurable days-until-archive, status-based rules)
  - Scale testing: 100K WPs per workspace, 100 concurrent users, sub-500ms query latency (Â§2.3.15.9)
  - Acceptance: multi-user collaboration verified; CRDT conflicts resolve deterministically; WebSocket updates propagate <1s; performance targets met at scale


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Runtime adapter + event emission + trace artifact pipeline.  
  - Benchmark harness + replay tools.

**Atelier Track (Phase 3)**
- [ADD v02.67] Add explicit dual-contract examples and fixtures for post-production roles over time-based media (Editor/Color/VFX), including deliverable kinds and evidence requirements.
- [ADD v02.67] Extend Role Glance/Claim heuristics to time-based inputs (transcripts, keyframes, shot boundaries) while keeping the claim pass cheap and budgeted.
- [ADD v02.79] Tags panel + smart collections UI fed from vision outputs.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Minimal UI to surface effective policy and downgrades in Job History.

**Distillation Track (Phase 3)**
- Implement student runs, checkpoint creation, and eval/promotion gates vs teacher and previous checkpoints.
- Enforce rollback via checkpoint lineage; gate on security flags and collapse indicators; persist promotion decisions in Flight Recorder.
- Use the MCP sampling pipeline (Section 11.3.5) for student/teacher eval runs where applicable:
  - Distillation/eval calls use `sampling/createMessage` via the MCP Gate.
  - All such calls are logged into Flight Recorder and `fr_distillation_samples` with clear linkage to Skill Bank entries and checkpoints.
- Acceptance: at least one promotion cycle logged end-to-end with metrics, lineage, and gate outcomes; rollback tested.
- [ADD v02.157] Any local-model promotion or adapter merge MUST prove teacher/student/context-pack lineage, benchmark deltas, and rollback-safe checkpoint compatibility before promotion decisions become effective.



- [ADD v02.115] **[REMEDIATION] MT Executor LoRA training automation:** Automated LoRA fine-tuning triggered from DistillationCandidate accumulation; implement training job profile, checkpoint management, and A/B comparison against previous LoRA versions. (Extends existing Skill Bank distillation infrastructure.)
- [ADD v02.115] **MT failure category refinement:** Refine FailureCategory taxonomy based on accumulated escalation data; add new categories as patterns emerge; update LoRA training targets accordingly.
- [ADD v02.115] Acceptance: LoRA training jobs visible in Job History; checkpoints stored with provenance; regression tests prevent LoRA degradation.
- [ADD v02.79] Promote successful â€œculling + taggingâ€ flows into reusable workflows/presets.


- **[ADD v02.122] Layer-wise Inference Experiments (Local Runtime Adapter)**
  - Optional: learn a safe default mapping from `speed_preset` â†’ exec_policy per runtime.

### 7.6.6 Phase 4 â€” Collaboration & Extension Ecosystem

**Goal**  
Move from a single-user tool to a collaborative, extensible platform, while preserving and extending observability and debug tools.

- [ADD v02.105] Phase coverage is governed by Â§7.6.1 Coverage Matrix; Phase 0 is closed and MUST NOT be used for scheduling newly discovered requirements (remediate in Phase 1 or later).
- [ADD v02.52] Make ACE-RAG-001 collaboration-safe: per-user capability-gated catalogs/routes, pack regeneration policy across devices, and audit-preserving cache behavior.
- **[ADD v02.130] Make Loom collaboration-safe**  
  LoomBlocks + LoomEdges must support multi-user workspaces: CRDT-safe sync, capability-gated edge/tag creation, and audit-friendly event logging for shared libraries.
- **[ADD v02.131] Make Stage collaboration-safe + extension-ready**  
  Stage sessions, Stage Apps, and Stage Bridge must work in multi-user workspaces: per-user capability grants, audit logs, and hash-pinned Stage App bundles; Stage Studio evolves toward Spline-class authoring as an optional surface.



- [ADD v02.68] Treat engine adapters as installable/pinnable extension artifacts: registry updates are audited, adapters are hash/version pinned, and no engine becomes callable without passing conformance.

- [ADD v02.79] Make Photo Studio collaboration-safe (recipes/collections/presets sync without breaking provenance or determinism guarantees).


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**  
  Turn dynamic compute into a safe, optional ecosystem capability: multiple runtimes can support `exec_policy`, operators can compare policies, and governance remains uncompromised.

**MUST deliver**

1. **Collaboration & sync**  
   - Integrate a CRDT library (e.g. Yjs) with the existing document and canvas model, as described in Section 7.3.  
   - Define and implement a sync topology (file-based or server) with a **`y-websocket` provider** for servered sync; support file-based sync for distributed/offline collaboration.  
   - Ensure Workflow Engine, AI Jobs, and Flight Recorder behave correctly under concurrent edits:
   - Provide a **CRDT Time Machine / Merge Visualizer** in Operator Consoles:
     - Replay concurrent edit sequences and inspect merge outcomes.
     - Deep-link merge/conflict events to the underlying CRDT ops and affected entities.

     - Conflicts are visible and traceable.  
     - Job history clearly shows which user and which device triggered which actions.

2. **Multi-user semantics**  
   - Introduce an authentication/session model.  
   - Define how AI jobs behave when multiple users interact with the same artefacts (ownership, consent, capability scope, per-user audit trails).  
   - Extend debug tooling to:
     - Filter Flight Recorder and Job History by user, workspace, and device.  
     - Inspect collaborative sessions and their timelines.

   - Bind MCP sessions to user identity and workspace context:
     - MCP client connections from the coordinator carry WSID and user/session identifiers.  
     - The MCP Gate enforces per-user capability scopes and logs which user triggered each MCP tool call.  

3. **Plugin / extension system (initial)**  
   - Design an internal plugin API built on top of the AI Job Model and capability system.  
   - [ADD v02.131] Treat Stage Apps as first-class internal plugins: Stage Bridge API is the UI-facing RPC layer; Stage App bundles are hash/version pinned and capability-scoped; Stage Host origin isolation is enforced and all privileged actions are logged to Flight Recorder.
   - Expose safe extension points:
     - New workflow nodes.  
     - New AI Job profiles or capability profiles.  
   - Require that plugins:
     - Use the same logging, metrics, and Flight Recorder frameworks.  
     - Register their actions so they appear in Job History and traces.  
   - Prepare for external plugins by aligning with security and sandboxing constraints defined in Section 5.2, and by favouring MCP-based tool servers as the primary extension mechanism (plugins as MCP servers behind the Gate).  
   - Implement an initial sandbox for untrusted plugins using **WASM** (and optionally **Pyodide** for Python), enforcing the capability model (default-deny) and logging all calls via Flight Recorder.  

4. **Security and privacy hygiene**  
   - Document how logs, Flight Recorder data, and debug traces handle sensitive content.  
   - Provide at least basic controls for:
     - Clearing or rotating logs.  
     - Exporting/importing data safely.  
5. **Mail advanced governance/analytics/taste**  
   - Classification ladder + tags (PUBLIC â€¦ HIGHLY_RESTRICTED) and routing rules for cloud/local engines and connectors.  
   - Chronicle/Analyst dashboards for mail analytics; Polyglot/Red Pen/Sentiment/Anonymizer wired into mail flows.  
   - Taste models for mail reply style per client/classification.
6. **Spatial/optional mechanical domains (gated/optional)**  
   - Treat `Cartographer`/`Navigator`/`Geo` as optional extensions gated by network/device capabilities.  
   - Treat `Decompiler`/`Homestead`/`Sous Chef` as plugin-scope only, enabled explicitly with capability grants and FR logging.  
   - Keep heavy engines (`Spatial`/`Machinist`/`Simulation`/`Hardware`/`Guide`) tied to device/network/GPU grants and reproducibility checks.



7. **[ADD v02.36] Plugin capability precedence + bypass hardening (enforced + tested)**  
   - Enforce precedence resolution (**plugin > workspace > builtin**) deterministically and log resolutions for auditability.  
   - Plugins MUST NOT bypass Gate/Workflow/Flight Recorder; all actions/tool calls route through the same capability checks and trace plumbing.

8. **[ADD v02.36] Mail/Calendar offline-first mode (optional if enabled) promoted to MUST**  
   - When enabled, operate offline-first with incremental sync, attachment scanning, and retention/consent defaults; sync operations are traceable.

   

9. **[ADD v02.44] Extension substrate: WASM plugins (capability-scoped)**  
   - Implement `tech.plugins.wasm_runtime` using Extism + Wasmtime as the default sandbox for optional domain modules.


10. **[ADD v02.47] Collaborative Charts & Decks + extension templates (CRDT-aware)**  
   - CRDT-safe multi-user editing for chart specs and deck slide layouts; per-user attribution visible in Job History/Flight Recorder.  
   - Extension-delivered chart templates, dashboard layouts, and deck themes (capability-scoped; no bypass of governance).  
   - Plugin-provided exporters/renderers MUST register as mechanical jobs (engine/version/params/hashes/policy logged; no bypass).


11. **[ADD v02.79] Photo Studio collaboration + extension hooks**  
   - [ADD v02.79] Conflict strategy for recipe edits + collection membership + preset updates.  
   - [ADD v02.79] Extension points: importers/export profiles/presets as plugins (gated; no-bypass).  

- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem) â€” Phase 4 or later**
  - Runtime compatibility: at least two runtimes/providers support `exec_policy` with deterministic downgrade + `llm_exec_policy` emission.  
  - Policy registry: named, versioned exec_policy presets (`policy_id`) that can be referenced in Work Profiles and jobs (without forcing schema rewrites).  
  - Cross-role/dynamic-role support: inheritance model for dynamic roles + per-role compute overrides (see Â§4.5.6.2).  
  - Operator-grade reporting: dashboards for requested vs effective policy, approximate usage frequency, and waiver compliance audits.  
  - Export tooling: trace artifacts and summaries exportable with privacy controls and retention enforcement.



12. **Loom collaboration + shared libraries** [ADD v02.130]  
   - Extend sync/collaboration layer to include LoomBlocks and LoomEdges (UUID-stable, CRDT-safe). Concurrent edits to block metadata, tags, and mentions must converge deterministically.
   - Enforce capability-gated mutations for shared workspaces: edge creation, tag creation, pinning, and deletion must respect per-user permissions and produce auditable Flight Recorder trails (Â§11.5.12).
   - Support shared tag hubs (`TAG_HUB` blocks) with sub-tag hierarchies and conflict-safe merges.
   - Scale Loom view/search for shared workspaces (Tierâ€‘2 Postgres full-text as needed) while preserving local-first behavior when offline.

- [ADD v02.137] **Cross-workspace and multi-operator routing:** allow sessions to be routed across multiple workspaces and shared between multiple human operators with explicit consent boundaries.
- [ADD v02.137] **Third-party session isolation:** hard multi-tenant isolation for third-party â€œappsâ€ that spawn sessions, including per-app quotas, provenance, and revocation.
- [ADD v02.137] **Advanced recovery and audit:** deterministic replay of session spans and tool calls for high-stakes audits (within policy), using content-hash + artifact refs.

**Vertical slice**  
- Two users (or two devices) edit the same document using the chosen sync topology.  
- One user triggers an AI action that modifies the shared document.  
- Both users see the changes.  
- Job History and Flight Recorder show which user triggered the job, how it ran, and how it interacted with sync/CRDT.
- [ADD v02.47] Two users co-edit a dashboard + deck â†’ run export â†’ verify attribution, capability grants, and export policy in Flight Recorder/Job History.

11. **[ADD v02.52] Multi-user ACE-RAG governance (packs, caches, traces)**
   - Capability-gate Semantic Catalog and routing hints per user/workspace; catalogs MUST NOT reveal selectors/paths outside granted scope.
   - Define cross-device policy for ContextPacks:
     - ownership/attribution,
     - regeneration triggers,
     - stale/invalid handling under concurrent edits.
   - Define multi-user cache semantics:
     - cache keys include policy_id + user scope,
     - caches never leak evidence across users without explicit share grants,
     - replay mode preserves trace integrity across devices.
   - RAG Debugger and Index Doctor MUST show user/device attribution for QueryPlan/Trace and for any pack/caching decisions.

- [ADD v02.52] Two collaborators ask the same question on shared content; traces show per-user policy/capability gating and no evidence leakage across users/devices.


- [ADD v02.67] Package Atelier roles/contracts as extensions:
  - Define a â€œRole Packâ€ format containing `AtelierRoleSpec`, schemas, validators, and deliverable templates.
  - Enforce capability gating + provenance: extension-provided roles MUST run through Workflow Engine + Flight Recorder and must not bypass policy/export rules.
- [ADD v02.79] Two clients edit the same photo recipe and resolve conflict â†’ export reproducibly with full provenance.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**  
  Operator switches between two runtimes supporting exec_policy, compares fast_exact vs standard on a WP/MT, and exports a policy + trace report suitable for audit.


- **Loom collaboration loop** [ADD v02.130]
  - Two users open the same shared workspace Loom library.
  - User A imports a file into Loom; User B sees it appear in All view after sync.
  - Both users add tags/mentions to the same block; conflicts converge without duplicate edges.
  - Audit trail: Flight Recorder shows who created which edges/tags and when.
  - Permissions check: a user lacking tag-create capability cannot create new tag hubs (but can use existing tags, if allowed).

**Key risks addressed in Phase 4**
- [ADD v02.130] Loom collab risks: CRDT convergence for edge graphs, duplicate edge prevention under concurrency, and permission/audit correctness in shared tag hubs.


- [ADD v02.47] Multi-user attribution ambiguity for charts/decks (who changed what, and what exactly was exported) if lineage is not captured.
- [ADD v02.47] Malicious/unstable render/export extensions compromise determinism or leak data unless sandboxed and capability-gated.

- [ADD v02.49] Multi-user artifact drift (different bytes across devices) breaks shared provenance; mitigate via SHA-256 dedupe + canonical bundle hashing + explicit ExportRecords across collaborators.

- Collaboration behaviour is inconsistent or not auditable.  
- Plugins bypass governance, capabilities, or observability.  
- Logs and debug tools become unusable in multi-user scenarios.
- Optional/spatial/plugin engines leak data or bypass safety if not explicitly gated and logged.
- MCP-based plugins or external MCP servers bypass the Gate or user/session binding, making actions untraceable or undermining per-user capability scopes.


- [ADD v02.44] Plugin capability model gaps: any bypass of Gate/Workflow/Flight Recorder breaks auditability and safety; the plugin runtime MUST remain deny-by-default.
- [ADD v02.44] Hardware/network access risk for CNC daemon connections and simulation tooling; capability scoping and logging must be complete.
- [ADD v02.67] Role Packs/extensions can bypass Lens validators or mutate Raw/Derived unless the same gates apply; mitigated by default-deny capabilities, mandatory validator packs, and export/write-back prohibition enforced by the host runtime.
- [ADD v02.79] Sync conflicts and provenance corruption â†’ sync only versioned schemas + artifact refs; forbid raw mutation paths.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - Operator confusion (what actually ran).  
  - Governance drift (approximate becoming implicit).  
  - Ecosystem incompatibility across runtimes.

**Acceptance criteria**
- [ADD v02.130] Loom collaboration: LoomBlocks/edges sync across users, converge without duplicate edges, respect capability gating, and generate auditable FR-EVT-LOOM trails.


- Collaborative edits are correctly synced and traceable in logs.  
- AI jobs in collaborative sessions are correctly attributed to users/devices.  
- Plugins can register actions and appear in Job History without bypassing capabilities or Flight Recorder.  
- At least one MCP-based extension (plugin or external server) is integrated; its MCP tool calls are visible in Job History and Flight Recorder with correct user/session attribution and capability metadata.
- Minimal security/privacy documentation exists for logs and debug data.
- Conflict handling UX present; conflicts are surfaced and traceable.  
- User/device filters available in Flight Recorder and Job History.  
- Plugin capability precedence (plugin > workspace > builtin) enforced; capability cache expiry honored.  
- Mail/Calendar (optional if enabled) operate offline-first with capability-gated sync, attachment scanning, and retention/consent defaults.  
- Heavy/hardware engines enforce device/network/GPU safety gates; commands/artifacts are reproducible and denial paths tested.
- Spatial/Machinist/Simulation/Hardware/Guide outputs link back to Canvas/Docs/Monaco artifacts with provenance; commands/params/artifact hashes recorded for reproducibility.
- Mail advanced: classification tags persist on MailMessage + descriptors; routing policies enforced (cloud vs local); FR logs classification decisions; analytics dashboards populated; taste model outputs logged with provenance.
- Optional/plugin engines (Cartographer/Navigator/Geo/Decompiler/Homestead/Sous Chef) only enabled via explicit plugin/extension switches with capability grants; FR logs inputs/outputs and provenance for any run.
- At least one sandboxed plugin (WASM or Pyodide) runs with default-deny capabilities, registers actions in Job History, and logs calls/events to Flight Recorder; attempts to bypass capabilities are denied and logged.  



- [ADD v02.44] A plugin-provided vertical slice runs end-to-end with explicit capability grants, full Flight Recorder provenance, deterministic artifact outputs, and (where applicable) multi-user attribution.
- [ADD v02.67] At least one Role Pack extension installs cleanly, registers roles/contracts, and runs Lens jobs under capability gates with full Flight Recorder provenance and validator enforcement.
- [ADD v02.67] Cross-user attribution for Lens runs is visible in Job History/Flight Recorder where collaboration is enabled; extension provenance includes pack id/version/hash.
- [ADD v02.79] Concurrent edits converge; all exports remain attributable to specific recipe versions + engine versions.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - A job can be replayed (or deterministically explained as non-replayable) with clear â€œeffective policyâ€ disclosure.  
  - Auditors can list all approximate executions over a time range with waiver refs and affected WPs/MTs.

**Explicitly OUT of scope**
- [ADD v02.130] Loom: "block-as-app" programmable views and cross-workspace public Loom libraries remain future-surface work (see Â§10.12 roadmap).


- Phase 4 does not expand core single-user UX beyond what is required for collaboration.
- Complex plugin marketplaces, monetization, and third-party billing.
- Unbounded external write-back/sync targets without explicit capability grants and provenance.
- Automatic sharing of RawContent/DerivedContent across collaborators without explicit user consent controls.
- [ADD v02.79] Full Affinity/Illustrator parity; advanced compositor; full marketplace.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - Mandatory layer-wise inference for all users.  
  - Auto-enabling approximation based on heuristics without operator consent.

**Mechanical Track (Phase 4)**
- [ADD v02.130] Loom shared-workspace query scaling: optional Postgres Tierâ€‘2 indexes and background rebuild jobs for metrics/backlinks at scale.


- [ADD v02.47] Extension-provided chart renderers/exporters (PPTX/PDF/HTML) MUST route through Workflow Engine + mechanical runner + Flight Recorder with full provenance (engine/version/params/hashes/policy).
- Heavy/hardware engines: `Spatial` (CAD), `Machinist` (CAM/G-code), `Simulation` (FEA/CFD), `Hardware` (camera/USB/serial), `Guide` (routing/live checks) behind explicit device/network/GPU grants and safety gates.
- Spatial/extensions: `Cartographer` (maps/tiles), `Navigator` (routing), and `Geo` (GIS queries) gated by network/device capabilities; treat as optional plugin-scope if not core to a release.
- Advanced/optional domains: `Decompiler` (reverse engineering), `Homestead` (home logistics), and `Sous Chef` (culinary) only under explicit plugin/extension enablement with capability gates and FR logging.
- Plugins must register mechanical actions through Workflow Engine + Flight Recorder; no bypass of capabilities or logging.
- Acceptance: CAD -> CAM -> Simulation vertical slice with safety validation and reproducible outputs; hardware connector exercised with a mock device; spatial engines show provenance and safety gating; optional/plugin engines logged with explicit capability grants; multi-user attribution visible in Job History/Flight Recorder.


- [ADD v02.44] Woodworking/Digital fabrication reference extension vertical:
  - `wood.primitives.shopgrade`
  - `wood.jobs.nesting_2d` (Deepnest `external_process`)
  - `wood.jobs.job_packet_compiler` (qpdf + libvips)
  - `wood.validators.toolpath_simulation` (CAMotics `external_process`)
  - `wood.connector.machine_daemon` (CNCjs `external_service`)
- [ADD v02.44] Creative interoperability modules:
  - `creative.interop.timeline_otio` (OTIO import/export)
  - `creative.review.annotations` (Annotorious) â€” only after pinned-version license verification is recorded in the OSS Component Register.
- [ADD v02.67] Add Role Pack install/uninstall, version pinning, and deny-by-default capabilities for extension-provided Lens jobs.


- [ADD v02.68] Engine adapter packaging posture: adapters/extensions MUST be version+hash pinned, registered (registry is authoritative), and conformance-gated. High-risk capabilities (device/network/secrets/GPU) require explicit per-user grants with full provenance.
- [ADD v02.68] Add `Sovereign` engine slice for collaboration: cryptographic signing/verification and key-handling flows run as mechanical jobs under secrets-use policy and are fully auditable (inputs/outputs as artifacts; no raw key material in logs).
- [ADD v02.79] Plugin packaging + conformance requirements for any Photo engine adapter before it becomes callable.

- [ADD v02.115] **AI-Ready Data Architecture (Â§2.3.14) - Phase 4:**
  - Implement distributed index sharding for large workspaces (>100k documents)
  - Implement advanced cross-modal retrieval (find code by describing its visual output, find images by code that generated them)
  - Implement real-time quality monitoring dashboards in Operator Consoles
  - Implement collaborative index synchronization (multi-user index consistency)
  - Implement embedding model migration automation (batch re-embedding when models upgrade)
  - Acceptance: index sharding transparent to queries; cross-modal queries work across Codeâ†’Image, Imageâ†’Code paths

- [ADD v02.116] **Locus Work Tracking System (Â§2.3.15) - Phase 4:**
  - Implement advanced analytics: WP completion velocity trends, MT success rate by model/LoRA, blocking dependency hot-path analysis
  - Implement AI-powered insights: WP priority recommendations based on dependency impact, MT escalation pattern detection, auto-suggest decomposition for large WPs
  - Implement cross-workspace WP aggregation queries (e.g., all vendor-related WPs across all projects)
  - Implement WP templating system: reusable WP templates with auto-fill task packet structures
  - Implement multi-user Locus governance: per-user WP creation quotas, collaborative gate approval workflows, attribution tracking
  - Implement Locus plugin API: allow extensions to add custom WP fields, dependency types, and query filters
  - Acceptance: AI recommendations visible in UI; cross-workspace queries performant; WP templates reduce creation time; plugin extensions registered and capability-gated


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - Policy registry + schema validation + cross-runtime adapters.  
  - Audit/reporting pipelines.

**Atelier Track (Phase 4)**
- [ADD v02.67] Role Packs: publish/install flows for `AtelierRoleSpec` + schemas + validators + templates; ensure pack hashes are recorded and referenced by Lens outputs.
- [ADD v02.67] Multi-user sharing: allow sharing Deliverable Bundles (not Raw/Derived corpora) under explicit grants; ensure no-censor remains internal while export projections are policy-gated.
- [ADD v02.79] Collaborative review UI for recipe diffs + preset sharing.


- **[ADD v02.122] Layer-wise Inference Productization (Guardrails + Ecosystem)**
  - UX for policy selection, waiver management, and effective-policy inspection.

**Distillation Track (Phase 4)**

- [ADD v02.47] Multi-user export governance for charts/decks: per-user consent and export policy selection recorded with lineage across collaborators/devices.
- Multi-user governance for Skill Bank artifacts: per-user attribution, consent, and export controls; plugin/extensibility hooks use the same logging/capability model.
- Support secure sharing/off-device export only via explicit capability grants; maintain lineage across collaborators/devices.
- Acceptance: distillation artifacts respect multi-user capability scopes; Job History/Flight Recorder show user/device attribution for distillation jobs and exports.

- [ADD v02.115] **MT Executor parallel wave execution:** Implement concurrent MT execution within dependency-safe waves; add wave scheduling, resource coordination, and progress aggregation for parallel MTs.
- [ADD v02.115] **MT cloud escalation governance:** Implement cloud_escalation_allowed policy with per-user consent, cost tracking, and capability gates for cloud model usage in escalation chains.
- [ADD v02.115] Acceptance: parallel wave execution demonstrated with >2 concurrent MTs; cloud escalation requires explicit capability grant and logs cost/usage.

- [ADD v02.115] **AI-Ready Data Architecture â†’ Multi-user LoRA governance (Â§2.3.14.9):**
  - Implement per-user training data attribution and consent controls
  - Implement shared vs private LoRA model governance (who can use locally-trained models)
  - Implement training data export controls (prevent sensitive data leakage via LoRA sharing)
  - Implement cross-device LoRA synchronization with provenance tracking
  - Acceptance: LoRA training data respects per-user consent; shared LoRAs carry full provenance; export denies sensitive training sources
- [ADD v02.79] Shared â€œhouse styleâ€ presets and taste descriptors derived from accepted edits (opt-in).


**Key Takeaways**  
- The roadmap is **architecture-aligned and debug-first**: every phase explicitly requires health checks, structured logging, Flight Recorder integration, and at least one human-usable debug surface.  
- **Vertical slices** ensure each phase ends with a real, end-to-end scenario you can manually test, not just abstract infra.  
- Phases are used to **burn down risk**: stack stability in Phase 0, AI Jobs + workflows + observability in Phase 1, ingestion/RAG in Phase 2, ASR in Phase 3, and collaboration/plugins in Phase 4.  
- Cross-cutting concernsâ€”migrations, security/privacy of logs, dev experience, ADRsâ€”are included so they are not forgotten while focusing on features.


