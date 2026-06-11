---
schema: handshake.indexed_spec.module@1
spec_version: "v02.185"
bundle_id: "master-spec-v02.185"
module_id: "03"
section_id: "3"
title: "3. Local-First Infrastructure"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "d74c6ba68048b2f0dba83b0d0ad6f1b8d8fce3327b1a73a31081e59d8090cc6c"
body_sha256: "dde0ff22f54de69ca20b78231f020fdbe9b5eaa24d9659304bcb2e7e716e3b92"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 3. Local-First Infrastructure

## 3.1 Local-First Data Fundamentals

**Why**  
Local-first is a core principle, not just a feature. Understanding what it means technically prevents design mistakes that would compromise user sovereignty.

**What**  
Explains what "local-first" really means, why concurrent editing is hard, how CRDTs solve it, and what CRDTs don't solve.

**Jargon**  
- **Local-first**: Data lives on your device first; cloud is optional.
- **Concurrent Editing**: Multiple participants modifying the same data simultaneously.
- **CRDT (Conflict-free Replicated Data Type)**: Data structure that can be merged without conflicts.
- **Eventual Consistency**: All replicas converge to the same state given enough time.

---

#### 3.1.0.1 The Promise

**Local-first software keeps your data on your devices, with optional cloud sync.** This gives you:

- **Ownership:** Your files are literally on your computer
- **Speed:** No network round-trip for every action
- **Offline:** Works without internet
- **Privacy:** Data doesn't have to touch company servers

#### 3.1.0.2 The Contrast

```
CLOUD-FIRST (Google Docs):                LOCAL-FIRST (What we're building):
                                          
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚   Google's Servers  â”‚ â† "Real" data     â”‚   YOUR Computer     â”‚ â† "Real" data
â”‚   (the cloud)       â”‚                   â”‚                     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
          â”‚                                         â”‚
          â–¼                                         â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚   Your Browser      â”‚ â† Just a window   â”‚   Cloud (optional)  â”‚ â† Backup/sync
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.1.0.3 Why Local-First for This Project?

ðŸ“Œ **Privacy:** AI processes your documents locally. Your private notes never leave your machine.

ðŸ“Œ **Speed:** No waiting for server round-trips. The AI model is right there on your GPU.

ðŸ“Œ **Ownership:** Your data is literally files on your computer. No company can lock you out.

ðŸ“Œ **Offline:** Works on airplanes, in basements, anywhere. No "you're offline" errors.

âš ï¸ **The Tradeoff:** Syncing between devices becomes much harder. When two devices edit the same document offline, we need special technology (CRDTs) to merge the changes.

---

### 3.1.1 The Problem: Concurrent Editing

#### 3.1.1.1 Why This is Hard

**Traditional databases assume one "source of truth."** When you save a document, you overwrite what was there. If two people edit simultaneously, one overwrites the other.

```
Traditional Approach (Google Docs style):
  
  [Device A]                    [Server]                    [Device B]
      â”‚                            â”‚                            â”‚
      â”‚â”€â”€â”€â”€ Edit: "Hello" â”€â”€â”€â”€â”€â”€â”€â”€>â”‚                            â”‚
      â”‚                            â”‚<â”€â”€â”€â”€ Edit: "World" â”€â”€â”€â”€â”€â”€â”€â”€â”‚
      â”‚                            â”‚                            â”‚
      â”‚                   Server decides order                  â”‚
      â”‚                   "Hello" then "World"                  â”‚
      â”‚                   OR "World" then "Hello"               â”‚
      â”‚                            â”‚                            â”‚
      
  Problem: Server is required. No offline support.
```

```
Local-First Challenge:
  
  [Device A - Offline]                              [Device B - Offline]
      â”‚                                                  â”‚
      â”‚ Edit: "Hello"                                    â”‚ Edit: "World"
      â”‚     (no server to ask!)                          â”‚     (no server!)
      â”‚                                                  â”‚
      â–¼                                                  â–¼
  Local state: "Hello"                          Local state: "World"
  
  Later, when both reconnect... now what?
```

#### 3.1.1.2 Timeline of a Conflict

```
Monday 9am:  Both laptop and tablet sync â†’ same document state
Monday 10am: You go offline on both devices
Monday 11am: On laptop, you add paragraph A
Monday 11am: On tablet, you add paragraph B
Monday 2pm:  Both come online again

QUESTION: What should the document look like now?

  Option 1: Last-write-wins â†’ One person's work is LOST âŒ
  Option 2: Keep both versions, ask user to choose â†’ Annoying âŒ  
  Option 3: Automatically merge both changes â†’ âœ“ This is what CRDTs do
```

---

### 3.1.2 Solution: CRDTs Explained

#### 3.1.2.1 Jargon Glossary

| Term | Plain English | Why It Matters |
|------|---------------|----------------|
| **CRDT** | Conflict-free Replicated Data Typeâ€”a special data structure that can merge automatically | The technology that makes local-first sync possible |
| **Merge** | Combining two versions into one | CRDTs guarantee merges always produce the same result |
| **Eventual Consistency** | All devices eventually have the same data, even if they're temporarily different | What CRDTs guarantee |
| **Operation-based (Op-based)** | A CRDT style that syncs by sharing operations ("insert 'A' at position 3") | One approach |
| **State-based** | A CRDT style that syncs by sharing entire state snapshots | Another approach |

#### 3.1.2.2 The Magic of CRDTs

**CRDTs are data structures designed so that merging always works and always produces the same result.**

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    CORE CONCEPT: How CRDTs Work
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

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
  â€¢ Each operation has a unique ID
  â€¢ We can replay ALL operations in a deterministic order
  â€¢ Both devices end up with: "HelloWorld" (or "WorldHello")
  â€¢ The SAME result regardless of which device syncs first!

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
```

#### 3.1.2.3 Types of CRDT Data Structures

```
For Text Documents:
  â€¢ Tracks each character insertion/deletion
  â€¢ Handles concurrent typing in different places
  â€¢ Libraries: Yjs (Y.Text), Automerge (Text type)

For JSON-like Objects:
  â€¢ Tracks changes to keys and values
  â€¢ Handles concurrent edits to different fields
  â€¢ Libraries: Yjs (Y.Map), Automerge (objects)

For Lists/Arrays:
  â€¢ Tracks insertions, deletions, moves
  â€¢ Handles concurrent list modifications
  â€¢ Libraries: Yjs (Y.Array), Loro (MovableList)

For Rich Text:
  â€¢ Tracks formatting (bold, italic, etc.)
  â€¢ Handles concurrent formatting changes
  â€¢ Libraries: Yjs + editor bindings
```

#### 3.1.2.4 What CRDTs DON'T Solve

âš ï¸ **CRDTs merge automatically, but "automatic" doesn't mean "smart."**

```
Example: Two users both edit the SAME sentence:

Original:        "The quick brown fox"
User A changes:  "The fast brown fox"      (quick â†’ fast)
User B changes:  "The quick red fox"       (brown â†’ red)

CRDT merge:      "The fast red fox"        (both changes applied)

Is this right? Maybe! But maybe User A wanted to keep "brown" and 
User B wanted to keep "quick". The CRDT doesn't understand INTENT,
it just merges the characters.
```

ðŸ’¡ **Key insight:** CRDTs prevent data loss and conflicts, but users may still need to review merged results for semantic correctness.

---

**Key Takeaways**  
- Local-first means your device holds authoritative data; cloud is secondary.
- The core challenge is concurrent editing when devices are offline.
- CRDTs solve this by tracking operations (not just state), enabling deterministic merges.
- CRDTs guarantee eventual consistencyâ€”all devices converge to the same state.
- CRDTs merge mechanically; they don't understand semantic intent, so users may need to review results.

---
## 3.2 CRDT Libraries Comparison

**Why**  
Choosing the right CRDT library affects performance, features, and ecosystem. This comparison helps make an informed decision.

**What**  
Deep dives into Yjs, Automerge, and Loro with pros/cons and recommendations.

**Jargon**  
- **Yjs**: Mature JavaScript CRDT library with rich ecosystem.
- **Automerge**: Rust-first CRDT with strong formal foundations.
- **Loro**: Emerging Rust CRDT combining best features of both.

---

### 3.2.1 Yjs Deep Dive

#### 3.2.1.1 What is Yjs?

**Yjs is the most popular CRDT library for JavaScript/TypeScript applications.** It's battle-tested, fast, and has excellent editor integrations.

#### 3.2.1.2 Key Features

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                          Yjs                                 â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ Language:     JavaScript/TypeScript                         â”‚
â”‚ Also:         Rust port (Yrs), Python, Swift, Ruby          â”‚
â”‚ Data Types:   Y.Text, Y.Map, Y.Array, Y.XmlFragment         â”‚
â”‚ Performance:  Excellent (~260K inserts: 1s, 10MB memory)    â”‚
â”‚ History:      No full history (snapshots optional)          â”‚
â”‚ Sync:         WebSocket, WebRTC, custom providers           â”‚
â”‚ Editors:      ProseMirror, TipTap, Monaco, Quill, more      â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.2.1.3 How Yjs Works

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

#### 3.2.1.4 Pros and Cons

**Pros:**
- â­ Best performance and memory efficiency
- â­ Rich editor integrations (drop-in for popular editors)
- â­ Large community, many examples
- â­ Multiple sync options (WebSocket, WebRTC, file-based)
- â­ Cross-platform via ports (Yrs for Rust/Tauri)

**Cons:**
- âš ï¸ No built-in full history (only current state)
- âš ï¸ Learning curve for understanding shared types
- âš ï¸ Need to manually handle persistence

---

### 3.2.2 Automerge Deep Dive

#### 3.2.2.1 What is Automerge?

**Automerge is an academically rigorous CRDT library with full history tracking.** Version 2 is written in Rust with JavaScript bindings.

#### 3.2.2.2 Key Features

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                       Automerge                              â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ Language:     Rust core, JS/WASM bindings                   â”‚
â”‚ Data Types:   JSON-like objects, lists, text, counters      â”‚
â”‚ Performance:  Slower (~260K inserts: 1.8s, 44MB memory)     â”‚
â”‚ History:      Full operation history (like Git)             â”‚
â”‚ Sync:         Custom sync protocol                          â”‚
â”‚ Best For:     When you need complete version history        â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.2.2.3 Pros and Cons

**Pros:**
- â­ Full version historyâ€”can reconstruct any past state
- â­ Cleaner APIâ€”works like normal JS objects
- â­ Academic backingâ€”provably correct
- â­ Good for debugging (can replay history)

**Cons:**
- âš ï¸ Higher memory usage (~4x more than Yjs)
- âš ï¸ Slower for large documents
- âš ï¸ Larger storage requirements (keeps all operations)
- âš ï¸ Fewer editor integrations

---

### 3.2.3 Loro and Emerging Options

#### 3.2.3.1 What is Loro?

**Loro is a new CRDT library aiming to combine the best of Yjs and Automerge.** It offers high performance AND full history.

#### 3.2.3.2 Key Features

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                          Loro                                â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ Language:     Rust core, JS/WASM bindings                   â”‚
â”‚ Data Types:   MovableList, Map, Tree, Text, Counter         â”‚
â”‚ Performance:  Very high (designed to beat both Yjs/Automerge)â”‚
â”‚ History:      Full version DAG (like Git)                   â”‚
â”‚ Unique:       Movable trees (great for outlines/kanban)     â”‚
â”‚ Maturity:     Newer, less battle-tested                     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.2.3.3 Pros and Cons

**Pros:**
- â­ Rust-native (great for Tauri)
- â­ Full history like Automerge, speed like Yjs (claimed)
- â­ Movable trees perfect for hierarchical data (outlines, kanban)
- â­ Time-travel debugging possible

**Cons:**
- âš ï¸ Newer, less proven in production
- âš ï¸ Smaller community and fewer integrations
- âš ï¸ API may still change

---

### 3.2.4 Recommendation: Which CRDT Library?

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    DECISION POINT: CRDT Library Choice
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

FOR ELECTRON + TypeScript:
  â””â”€â”€ Use Yjs
      â€¢ Best performance and editor integrations
      â€¢ Largest community, most resources
      â€¢ Add snapshots for version history if needed

FOR TAURI + Rust:
  â””â”€â”€ Consider Loro or Yrs (Yjs Rust port)
      â€¢ Loro: If you need movable trees and version history
      â€¢ Yrs: If you want Yjs compatibility across platforms

RECOMMENDATION FOR THIS PROJECT (starting):
  â””â”€â”€ Start with Yjs
      â€¢ Proven, fast, well-documented
      â€¢ Works in both Electron and Tauri (via Yrs)
      â€¢ Easiest path to editor integration
      â€¢ Migrate to Loro later if needed for hierarchical data

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
```

#### 3.2.4.1 Comparison Summary Table

| Aspect | Yjs | Automerge | Loro |
|--------|-----|-----------|------|
| Performance | â­â­â­â­â­ | â­â­â­ | â­â­â­â­â­ (claimed) |
| Memory | â­â­â­â­â­ (10MB) | â­â­â­ (44MB) | â­â­â­â­ |
| Full History | âŒ (snapshots only) | âœ… | âœ… |
| Editor Integration | â­â­â­â­â­ | â­â­ | â­â­ |
| Rust Native | Via Yrs | Via WASM | âœ… Native |
| Maturity | â­â­â­â­â­ | â­â­â­â­ | â­â­ |
| Movable Trees | âŒ | âŒ | âœ… |

---

**Key Takeaways**  
- Yjs is the performance and ecosystem leaderâ€”best choice for starting out.
- Automerge offers full history but at 4Ã— memory cost and slower performance.
- Loro is a promising emerging option combining Yjs speed with Automerge history, plus movable trees for hierarchical data.
- For Tauri/Rust: use Yrs (Yjs port) or Loro; for TypeScript: use Yjs directly.
- Start with Yjs; consider migration to Loro later if hierarchical sync or version DAG becomes critical.

---
## 3.3 Database & Sync Patterns

**Why**  
Understanding how CRDTs integrate with databases enables efficient local storage and sync.

**What**  
Covers SQLite integration, combining CRDT and database, and sync topologies.

**Jargon**  
- **SQLite**: Embedded relational database.
- **Sync Topology**: How devices connect for data synchronization (peer-to-peer, hub, hybrid).

---

#### 3.3.0.1 Why Use a Database with CRDT?

**CRDTs handle sync, but databases handle queries.** You often need both:

```
CRDT alone:
  âœ“ Sync across devices
  âœ“ Merge concurrent edits
  âœ— "Find all documents containing 'budget'" â†’ Slow (must scan all)
  âœ— "Sort documents by date" â†’ Not built-in
  âœ— Complex queries â†’ Difficult

Database alone:
  âœ“ Fast queries with indexes
  âœ“ Sort, filter, aggregate
  âœ— Sync across devices â†’ Conflicts!
  âœ— Offline merge â†’ Data loss

CRDT + Database:
  âœ“ Sync via CRDT
  âœ“ Query via database
  âœ“ Best of both worlds
```

#### 3.3.0.2 SQLite: The Recommended Choice

**Kernel V1 boundary [ADD v02.184]:** The SQLite guidance in this section applies to local-first document, metadata, search, index, and rebuildable projection surfaces. It does not authorize SQLite for Kernel V1 runtime authority, cache, offline mode, compatibility mode, local fallback, bootstrap convenience, or test fixtures. Kernel V1 scheduling, promotion, replay, validation, session brokering, and trace reconstruction MUST use the Postgres EventLedger authority defined in Section 2.3.13.9.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                        SQLite                                â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ Type:         Embedded SQL database                         â”‚
â”‚ Storage:      Single file on disk                           â”‚
â”‚ Performance:  Very fast for local operations                â”‚
â”‚ Features:     Full SQL, indexes, full-text search (FTS)     â”‚
â”‚ Size:         Tiny (library is ~1MB)                        â”‚
â”‚ Reliability:  Extremely battle-tested                       â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**Why SQLite for this project:**
- â­ Standard across all platforms (Windows, macOS, Linux)
- â­ Works great with Electron AND Tauri
- â­ Full-text search for finding documents
- â­ ACID guarantees (data integrity)
- â­ Single file = easy backup

---

### 3.3.1 Combining CRDT and Database

#### 3.3.1.1 Architecture Pattern

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    HYBRID ARCHITECTURE                      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                             â”‚
â”‚    User Edit                                                â”‚
â”‚        â”‚                                                    â”‚
â”‚        â–¼                                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                              â”‚
â”‚  â”‚   CRDT    â”‚  â—„â”€â”€â”€ Handles: Sync, Merge, Collaboration   â”‚
â”‚  â”‚  (Yjs)    â”‚                                              â”‚
â”‚  â””â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”˜                                              â”‚
â”‚        â”‚                                                    â”‚
â”‚        â”‚ On every CRDT change:                              â”‚
â”‚        â”‚ â€¢ Update SQLite                                    â”‚
â”‚        â”‚ â€¢ Update indexes                                   â”‚
â”‚        â–¼                                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                              â”‚
â”‚  â”‚  SQLite   â”‚  â—„â”€â”€â”€ Handles: Queries, Search, Indexes     â”‚
â”‚  â”‚           â”‚                                              â”‚
â”‚  â””â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”˜                                              â”‚
â”‚        â”‚                                                    â”‚
â”‚        â”‚ Query results                                      â”‚
â”‚        â–¼                                                    â”‚
â”‚    UI Display                                               â”‚
â”‚                                                             â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.3.1.2 What Goes Where?

```
In CRDT (Yjs):
  â€¢ Document content (text, rich text)
  â€¢ Board/canvas positions
  â€¢ List ordering
  â€¢ Everything that needs to sync and merge

In SQLite:
  â€¢ Document metadata (title, dates, tags)
  â€¢ Search indexes
  â€¢ User preferences
  â€¢ Derived/computed data
  â€¢ Anything that needs fast querying

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

#### 3.3.1.3 Sync Flow

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    COMPLETE SYNC FLOW                        â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜

1. USER MAKES EDIT (Device A)
   â”‚
   â”œâ”€â”€â–º CRDT update applied locally
   â”‚
   â”œâ”€â”€â–º SQLite updated with new content/metadata
   â”‚
   â””â”€â”€â–º CRDT update sent to sync server (or peer)

2. SYNC UPDATE RECEIVED (Device B)
   â”‚
   â”œâ”€â”€â–º CRDT merges incoming update
   â”‚
   â”œâ”€â”€â–º SQLite updated to reflect merged state
   â”‚
   â””â”€â”€â–º UI refreshes to show changes

3. CONFLICT HANDLED AUTOMATICALLY
   â”‚
   â””â”€â”€â–º CRDT merge is deterministic
       â”‚
       â””â”€â”€â–º Same SQLite state on all devices (eventually)
```

---

### 3.3.2 Sync Topologies

#### 3.3.2.1 Options for Syncing Data

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    SYNC TOPOLOGY OPTIONS                     â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  OPTION A: Peer-to-Peer                                      â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”         â”Œâ”€â”€â”€â”€â”€â”                                     â”‚
â”‚  â”‚Dev Aâ”‚â—„â”€â”€â”€â”€â”€â”€â”€â–ºâ”‚Dev Bâ”‚   Direct device-to-device          â”‚
â”‚  â””â”€â”€â”€â”€â”€â”˜         â””â”€â”€â”€â”€â”€â”˜                                     â”‚
â”‚     â”‚               â”‚                                        â”‚
â”‚     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                                        â”‚
â”‚  Pros: No server needed, private                             â”‚
â”‚  Cons: Both devices must be online simultaneously            â”‚
â”‚                                                              â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  OPTION B: Central Server                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”       â”Œâ”€â”€â”€â”€â”€â”€â”       â”Œâ”€â”€â”€â”€â”€â”                       â”‚
â”‚  â”‚Dev Aâ”‚â”€â”€â”€â”€â”€â”€â–ºâ”‚Serverâ”‚â—„â”€â”€â”€â”€â”€â”€â”‚Dev Bâ”‚                       â”‚
â”‚  â””â”€â”€â”€â”€â”€â”˜       â””â”€â”€â”€â”€â”€â”€â”˜       â””â”€â”€â”€â”€â”€â”˜                       â”‚
â”‚  Pros: Works when only one device online                    â”‚
â”‚  Cons: Requires running/paying for server                   â”‚
â”‚                                                              â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  OPTION C: File Sync (OneDrive/Dropbox)                     â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”       â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”       â”Œâ”€â”€â”€â”€â”€â”                     â”‚
â”‚  â”‚Dev Aâ”‚â”€â”€â”€â”€â”€â”€â–ºâ”‚OneDriveâ”‚â—„â”€â”€â”€â”€â”€â”€â”‚Dev Bâ”‚                     â”‚
â”‚  â””â”€â”€â”€â”€â”€â”˜       â””â”€â”€â”€â”€â”€â”€â”€â”€â”˜       â””â”€â”€â”€â”€â”€â”˜                     â”‚
â”‚  Pros: No custom server, leverages existing sync             â”‚
â”‚  Cons: File-level conflicts, coarse merging                  â”‚
â”‚        (need CRDT on top to handle conflicts)               â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.3.2.2 Recommendation

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    DECISION POINT: Sync Topology
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

PHASE 1 (MVP): File Sync + CRDT
  â€¢ Store CRDT updates as files in a synced folder
  â€¢ Let OneDrive/Dropbox/iCloud handle file sync
  â€¢ CRDT handles merge when files conflict
  â€¢ Zero server infrastructure needed

PHASE 2 (Multi-user): Central Sync Server
  â€¢ Build or use WebSocket sync server
  â€¢ Real-time collaboration possible
  â€¢ More complex but better UX

Libraries that help:
  â€¢ y-indexeddb: Local persistence for Yjs
  â€¢ y-websocket: WebSocket sync for Yjs
  â€¢ ElectricSQL: Postgres â†” SQLite sync
  â€¢ Replicache: Client-server sync framework

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
```

---

**Key Takeaways**  
- CRDTs handle sync/merge; SQLite handles queries/indexesâ€”use both together.
- CRDT stores content that needs to sync; SQLite stores metadata, search indexes, and derived data.
- On every CRDT change, update SQLite to keep the query layer in sync.
- Start with file-based sync (OneDrive/Dropbox + CRDT); add WebSocket server later for real-time collaboration.
- SQLite is the recommended local database: portable, reliable, full-featured, single-file.

---

Kernel V1 is excluded from the SQLite local-first recommendation: its first authority path is Postgres EventLedger only.

Kernel V1 CRDT workspace addendum [ADD v02.185]: Kernel V1 may use CRDT updates, snapshots, and state vectors for draft workspace collaboration, but those records remain pre-promotion working state. PostgreSQL remains mandatory for EventLedger authority, promotion receipts, replay, and validation. CRDT storage MUST be restart-replayable, snapshot-safe, and joinable to write-box and promotion ids; it MUST NOT become a hidden SQLite authority path.


---

## 3.4 Conflict Resolution UX

**Why**  
Even with CRDTs, users sometimes need to understand what changed. Good conflict UX builds trust.

**What**  
Patterns for showing sync status, version history, and when to surface conflicts to users.

**Jargon**  
- **Version History**: Record of document states over time.
- **Sync Status**: Visual indicator of synchronization state.

---

### 3.4.1 User-Facing Conflict Patterns

#### 3.4.1.1 The Good News

**Most of the time, users shouldn't see conflicts at all.** CRDTs merge automatically, and if users edit different parts of a document, everything "just works."

#### 3.4.1.2 When to Show Something

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              WHEN TO SHOW SYNC FEEDBACK                      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  ALWAYS SHOW:                                                â”‚
â”‚    â€¢ Sync status indicator (synced âœ“, syncing â†», offline âš¡)â”‚
â”‚    â€¢ "X minutes ago" last sync time                         â”‚
â”‚                                                              â”‚
â”‚  SHOW ON EVENT:                                              â”‚
â”‚    â€¢ "Document updated by another device" notification      â”‚
â”‚    â€¢ Highlight recently changed sections (briefly)          â”‚
â”‚                                                              â”‚
â”‚  SHOW ON POTENTIAL ISSUE:                                    â”‚
â”‚    â€¢ "This section was edited while you were offline.       â”‚
â”‚       Review the changes?" (when same paragraph edited)     â”‚
â”‚                                                              â”‚
â”‚  DON'T BOTHER USER WITH:                                     â”‚
â”‚    â€¢ Every automatic merge (too noisy)                      â”‚
â”‚    â€¢ Technical details ("CRDT vector clock updated")        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.4.1.3 Simple Sync Status UI

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ My Document.md                    âœ“ Synced 2 min ago    â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤

OR when syncing:

â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ My Document.md                    â†» Syncing...          â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤

OR when offline:

â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ My Document.md                    âš¡ Working offline     â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
```

---

### 3.4.2 Version History UI

#### 3.4.2.1 Why Provide Version History

Even with automatic merging, users want:
- **Safety net:** "I accidentally deleted something, can I get it back?"
- **Audit trail:** "What changed since yesterday?"
- **Comparison:** "What's different from the old version?"

#### 3.4.2.2 Implementation Approach

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                 VERSION HISTORY PANEL                        â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  â—„ Document History                                          â”‚
â”‚                                                              â”‚
â”‚  TODAY                                                       â”‚
â”‚  â”œâ”€â”€ 3:45 PM - You edited (current)                        â”‚
â”‚  â”œâ”€â”€ 2:30 PM - Synced from MacBook                         â”‚
â”‚  â””â”€â”€ 10:15 AM - You edited                                  â”‚
â”‚                                                              â”‚
â”‚  YESTERDAY                                                   â”‚
â”‚  â”œâ”€â”€ 8:00 PM - You edited                                   â”‚
â”‚  â””â”€â”€ 2:00 PM - Created                                      â”‚
â”‚                                                              â”‚
â”‚  â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€                  â”‚
â”‚  [Preview Selected] [Restore to This Version]               â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 3.4.2.3 Technical Implementation

```
For Yjs (no built-in history):
  â€¢ Take periodic snapshots (every N minutes or on significant changes)
  â€¢ Store snapshots in SQLite with timestamps
  â€¢ To restore: load old snapshot, create new CRDT state
  
For Automerge/Loro (built-in history):
  â€¢ History is automatically tracked
  â€¢ Can "time travel" to any past state
  â€¢ Trade-off: larger storage requirements
```

---

**Key Takeaways**  
- CRDTs handle most conflicts invisiblyâ€”don't over-communicate to users.
- Always show sync status (synced/syncing/offline) and last sync time.
- Highlight concurrent edits from other devices briefly; prompt review only when the same paragraph was edited.
- Provide version history as a safety net: periodic snapshots for Yjs, built-in history for Automerge/Loro.
- Version history UI enables restore, audit, and comparison without burdening users with technical details.

---


<a id="4-llm-infrastructure"></a>
