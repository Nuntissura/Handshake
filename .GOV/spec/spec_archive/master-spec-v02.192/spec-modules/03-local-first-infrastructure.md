---
schema: handshake.indexed_spec.module@1
spec_version: "v02.192"
bundle_id: "master-spec-v02.192"
module_id: "03"
section_id: "3"
title: "3. Local-First Infrastructure"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "d74c6ba68048b2f0dba83b0d0ad6f1b8d8fce3327b1a73a31081e59d8090cc6c"
body_sha256: "3deb08c98ea4ccbae1468c7b99b33c6143861c4448cb7c0404336a692e9cd5dc"
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

#### 3.3.0.2 PostgreSQL/EventLedger Authority + CRDT Draft State [UPDATED v02.190]

**Kernel V1 boundary [ADD v02.184] [UPDATED v02.190]:** The earlier SQLite guidance in this section is superseded for Handshake product authority. Runtime authority, cache, offline mode, compatibility mode, local fallback, bootstrap convenience, and test fixtures MUST use PostgreSQL/EventLedger authority, CRDT/write-box draft state, ArtifactStore materialization, or rebuildable projections derived from those authorities. Kernel V1 scheduling, promotion, replay, validation, session brokering, and trace reconstruction MUST use the PostgreSQL/EventLedger authority defined in Section 2.3.13.8.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              PostgreSQL/EventLedger Authority                â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ Type:         Managed PostgreSQL + EventLedger authority    â”‚
â”‚ Storage:      Product-managed runtime state                 â”‚
â”‚ Performance:  Concurrent, replayable, queryable             â”‚
â”‚ Features:     SQL, events, manifests, provenance            â”‚
â”‚ Boundary:     No SQLite fallback or Docker proof default    â”‚
â”‚ Reliability:  Fail-closed authority and replay              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**Why PostgreSQL/EventLedger for this project:**
- Authoritative records are replayable, attributable, and shareable across parallel model sessions.
- EventLedger provides mutation history and recovery evidence.
- ArtifactStore materializes files and projections without turning local folders into authority.
- CRDT/write-box records preserve draft collaboration before promotion.
- Docker, SQLite fallback, and manually launched services are not core proof paths.

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
â”‚        â”‚ â€¢ Update PostgreSQL/EventLedger projections        â”‚
â”‚        â”‚ â€¢ Update indexes                                   â”‚
â”‚        â–¼                                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                              â”‚
â”‚  â”‚ Postgres  â”‚  â—„â”€â”€â”€ Handles: Authority, Query, Events     â”‚
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

In PostgreSQL/EventLedger + rebuildable projections:
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
   â”œâ”€â”€â–º PostgreSQL/EventLedger authority/projections updated
   â”‚
   â””â”€â”€â–º CRDT update sent to sync server (or peer)

2. SYNC UPDATE RECEIVED (Device B)
   â”‚
   â”œâ”€â”€â–º CRDT merges incoming update
   â”‚
   â”œâ”€â”€â–º PostgreSQL/EventLedger projections reflect merged state
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
- CRDTs handle sync/merge; PostgreSQL/EventLedger handles authority, query, replay, and provenance.
- CRDT stores draft content that needs to sync; PostgreSQL/EventLedger stores promoted metadata, search inputs, and derived authority.
- On every promoted CRDT change, update PostgreSQL/EventLedger and rebuild projections from authority.
- Start with file-based sync (OneDrive/Dropbox + CRDT); add WebSocket server later for real-time collaboration.
- PostgreSQL/EventLedger is the required authority path; local-first ergonomics come from Handshake-managed lifecycle and rebuildable projections, not SQLite fallback.

---

Kernel V1 is excluded from the old SQLite local-first recommendation: its authority path is PostgreSQL/EventLedger only.

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

## 3.5 Sandbox Adapter Layer (Normative) [ADD v02.186] [ENRICHED v02.187]

**Why**
Handshake spawns untrusted code and model engines on every governed run. The mechanical runner sandbox in §5.2.5 names the *what*; this section defines the *how* — a stable Rust trait surface that lets KERNEL-004 route through Handshake-native managed isolation first, with WSL2+Podman, Windows-native jail, and Docker retained only as explicit compatibility adapters where they are already installed and operator-selected. The adapter layer is what makes the operator-locked invariant in §3.6 enforceable across every model process, mechanical engine, ASR worker, ComfyUI worker, plugin process, and helper subprocess Handshake ever launches.

The three adapters added in v02.186 are all **shared-kernel** (container or Win32 process-jail). v02.187 adds an explicit **isolation-tier model** (§3.5.5) and a **strong-isolation tier** (§3.5.6) — syscall-interception (gVisor) and microVM (Cloud Hypervisor) — because shared-kernel containers are not a sufficient boundary for genuinely untrusted, agent-written code: a kernel escape from a shared-kernel container is a host compromise. Untrusted agent-written code MUST run at the strong tier.

**What**
Defines the `SandboxAdapter` trait, the required adapter implementations across three isolation tiers, the trust-keyed per-job selection policy, the sandbox interface patterns the trait must follow, and the machine-readable capability declarations that downstream surfaces (ProcessOwnershipLedger §5.7, ModelRuntime §4.6, Work Profile §4.3.7) consume. The default implementation path is Handshake-native managed tooling; outside apps are compatibility adapters, not prerequisites for core operation.

**Jargon**
- **SandboxAdapter**: Rust trait exposing a uniform spawn/exec/fs-bind/net-policy/kill/status surface over heterogeneous OS-level isolation primitives.
- **WSL2+Podman**: Rootless Podman containers running inside WSL2; near-native GPU passthrough on NVIDIA via WSL2 CUDA driver.
- **WindowsNativeJailAdapter**: Win32 Job Objects + AppContainer + Restricted Tokens, wrapping an existing field-tested Rust crate (`codex-windows-sandbox` / `rappct`) per the OpenAI Codex 2026 pattern; never hand-rolled.
- **Isolation tier**: the strength class of an adapter's boundary — Tier 1 container/process-jail (shared host kernel), Tier 2 syscall-interception (user-space guest kernel, e.g. gVisor), Tier 3 microVM (dedicated guest kernel + hypervisor boundary, e.g. Cloud Hypervisor / libkrun).
- **microVM**: a lightweight virtual machine with its own guest kernel driven by a minimal Rust VMM (Cloud Hypervisor / libkrun / Firecracker); on a Windows 11 host these require Linux+KVM and therefore run inside WSL2.

---

### 3.5.1 `SandboxAdapter` trait contract

The `SandboxAdapter` trait lives in `src/backend/handshake_core/src/sandbox/mod.rs` and is the single authority surface every Handshake-spawned process passes through. The trait MUST expose:

- `spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle>` — launches a process under the adapter's isolation domain.
- `exec(&self, handle: &ProcessHandle, cmd: Command) -> Result<ExecResult>` — runs an additional command inside an existing isolation context.
- `fs_bind(&self, handle: &ProcessHandle, host_path: PathBuf, guest_path: PathBuf, mode: BindMode) -> Result<()>` — bind-mounts host paths (required for large GGUF model files).
- `net_policy(&self, handle: &ProcessHandle, policy: NetPolicy) -> Result<()>` — applies network isolation (deny-all / loopback-only / allowlist).
- `kill(&self, handle: &ProcessHandle, signal: Signal) -> Result<()>` — terminates and reclaims.
- `status(&self, handle: &ProcessHandle) -> Result<ProcessStatus>` — current state (Running / Exited / Killed / Orphaned).
- `exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>>` — final exit code once stopped.
- `capabilities(&self) -> AdapterCapabilities` — machine-readable capability declaration (see §3.5.4).

The trait MUST NOT expose adapter-specific types in its public surface; callers reason only in `ProcessSpec` / `ProcessHandle` / `AdapterCapabilities`.

### 3.5.2 Tier-1 adapter implementations (container / process-jail) [UPDATED v02.190]

KERNEL-004 ships three shared-kernel (Tier-1) adapters. The strong-isolation Tier-2/Tier-3 adapters are defined in §3.5.6; the tier model is §3.5.5.

- **`HandshakeNativeSandboxAdapter` (DEFAULT)** — Handshake-managed native isolation path built from integrated open-source components, product-managed subprocess lifecycle, typed capability discovery, and no operator-launched outside app prerequisite. It is the required default target for core Handshake operation.
- **`WSL2PodmanAdapter` (compat-only-not-default)** — rootless Podman inside WSL2. Cross-machine portable (any Win11/WSL2 host), near-native GPU on NVIDIA, supports bind-mount for large GGUF without copy. Retained only for explicit operator-selected compatibility and development environments where Podman is already present.
- **`WindowsNativeJailAdapter`** — Win32 Job Objects + AppContainer + Restricted Tokens, **wrapping an existing Rust crate** (`codex-windows-sandbox` or `rappct`). DO NOT hand-roll Win32 isolation. Required path when a job demands Win32 native fidelity (registry isolation, native DirectX path, no WSL2 round-trip).
- **`DockerAdapter`** — preserves the Docker integration shipped by KERNEL-003 (DocketAdapter). Tagged **compat-only-not-default**; selected only when operator explicitly opts in (existing operator workflow, Docker Desktop already installed).

### 3.5.3 Selection policy

- Operator default = `HandshakeNativeSandboxAdapter`.
- Per-job override via the Work Profile (§4.3.7) `sandbox.adapter` knob OR via the `ProcessSpec.required_capabilities` set when a job declares a requirement only one adapter satisfies.
- `WindowsNativeJailAdapter` is selected when `required_capabilities` includes `win32_native_fidelity`.
- `WSL2PodmanAdapter` is selected only when the Operator explicitly opts in (no implicit fallback).
- `DockerAdapter` is selected only when operator explicitly opts in (no implicit fallback).
- **Trust dimension [ADD v02.187]:** `ProcessSpec.trust_class` (`trusted` | `reviewed` | `untrusted_agent`) drives the MINIMUM isolation tier. `untrusted_agent` (code produced by a model and not yet operator/validator-reviewed) MUST select a Tier-3 microVM adapter; if no Tier-3 adapter is available on the host it MAY degrade to a Tier-2 (`GvisorAdapter`) and MUST record a typed `SandboxTierDowngrade` event; it MUST NOT run at Tier 1. `trusted`/`reviewed` MAY run at Tier 1.
- **Nested-virt / GPU guard [ADD v02.187]:** selection MUST check the host advertises `requires_nested_virt` capability before choosing a Tier-3 adapter, and MUST NOT route a GPU-required job to an adapter declaring `gpu_passthrough: none`. A GPU-required `untrusted_agent` job that cannot obtain a GPU-capable strong-isolation adapter MUST fail loudly — never silently drop to Tier 1.
- If no adapter satisfies the declared capabilities (including the trust-class minimum tier), the spawn MUST fail loudly with a typed `SandboxSelectionFailure` event; silent fallback is forbidden.

### 3.5.4 Adapter capability declarations

Each adapter MUST publish a machine-readable `AdapterCapabilities` struct that the frontend reads via Tauri IPC. Fields:

- `filesystem_isolation_strength`: enum {weak, strong, very_strong} — how strictly the guest is prevented from reading host paths outside binds.
- `network_isolation_strength`: enum {weak, strong, very_strong} — granularity of net-policy enforcement.
- `gpu_passthrough`: enum {none, nvidia_cuda, vendor_agnostic} — declared GPU support.
- `stdio_throughput_class`: enum {low, medium, high} — observed sustained stdin/stdout streaming throughput class (used by ModelRuntime to pick streaming engines).
- `win32_native_fidelity`: bool — whether the guest sees a true Win32 environment (true only for `WindowsNativeJailAdapter`).
- `cross_machine_portable`: bool — whether the adapter is portable across hosts. `HandshakeNativeSandboxAdapter` is the default portability target; `WSL2PodmanAdapter` and `DockerAdapter` are compatibility adapters only; `WindowsNativeJailAdapter` is Win32-native fidelity, not the portable default.
- `isolation_tier`: enum {tier1_container, tier2_syscall, tier3_microvm} [ADD v02.187] — the adapter's isolation tier per §3.5.5; consumed by the trust-keyed selection policy (§3.5.3).
- `requires_nested_virt`: bool [ADD v02.187] — whether the adapter needs nested virtualization (true for Tier-3 microVM adapters running inside WSL2). Selection MUST verify host support and degrade with a recorded `SandboxTierDowngrade` event when unavailable.
- `supports_snapshot`: bool [ADD v02.187] — whether the adapter implements `snapshot`/`restore` (§3.5.7) for the validate-then-promote flow.

These fields are consumed by ModelRuntime (§4.6) when picking an engine adapter and by ProcessOwnershipLedger (§5.7) when recording `sandbox_adapter_id` per process row.

### 3.5.5 Isolation tier model [ADD v02.187]

The `SandboxAdapter` layer is organised into three isolation tiers. Every adapter declares its tier via `AdapterCapabilities.isolation_tier` (§3.5.4). Stronger tiers cost more startup time and overhead; the selection policy (§3.5.3) picks the weakest tier that satisfies the job's `trust_class`.

- **Tier 1 — container / process-jail (shared host kernel).** `HandshakeNativeSandboxAdapter` is the default Tier-1 target; `WSL2PodmanAdapter`, `DockerAdapter`, and `WindowsNativeJailAdapter` are compatibility or fidelity-specific adapters. Suitable for trusted or already-reviewed code and for GPU-bound local-model boxing when the native path declares the required GPU capability. NOT sufficient for untrusted agent-written code: a shared-kernel escape is a host compromise.
- **Tier 2 — syscall-interception (user-space guest kernel).** `GvisorAdapter` — intercepts and re-implements a vetted subset of host syscalls so guest syscalls never reach the host kernel directly. Stronger than Tier 1, lighter than a microVM (~10-30% I/O overhead).
- **Tier 3 — microVM (dedicated guest kernel + hypervisor boundary).** `CloudHypervisorAdapter` (primary), `LibkrunAdapter` (embeddable alternative). The strongest boundary; an escape requires a hypervisor breakout. Required for untrusted agent-written code.

**HARD:** untrusted, agent-written code MUST run at Tier 3 (or Tier 2 when no Tier-3 adapter is available on the host, with a recorded `SandboxTierDowngrade`); it MUST NOT run at Tier 1.

### 3.5.6 Strong-isolation adapters (Tier 2 + Tier 3) [ADD v02.187]

- **`CloudHypervisorAdapter` (Tier 3, PRIMARY)** — wraps Cloud Hypervisor (rust-vmm, Apache-2.0): a Rust KVM/MSHV virtual machine monitor with VFIO GPU passthrough and a typed Rust control client (`cloud-hypervisor-client`). Each sandbox is a microVM with its own Linux kernel, driven over the Cloud Hypervisor REST/socket API behind the `SandboxAdapter` trait (out-of-process control).
- **`LibkrunAdapter` (Tier 3, embeddable alternative)** — wraps libkrun (Rust, Apache-2.0): a microVM built as an embeddable in-process C-API library (the model proven by microsandbox). Preferred when tighter in-process embedding is wanted over socket control; GPU is paravirtualized (venus/Vulkan) rather than VFIO.
- **`GvisorAdapter` (Tier 2)** — wraps gVisor `runsc` (Apache-2.0) as an OCI runtime providing a user-space guest kernel; the middle tier when a microVM is unavailable or unnecessary.

**Windows-host reality (NORMATIVE constraint) [UPDATED v02.190].** Cloud Hypervisor, libkrun, and gVisor all require Linux + KVM. On a Windows 11 host, WSL2 may be used as an explicit compatibility substrate for strong-isolation engines, but it is not a core outside-app prerequisite. The default product target remains Handshake-native managed isolation; strong-isolation adapters that require WSL2 must declare that capability and fail closed when it is unavailable. Therefore:
- The Windows-native (no-WSL2) fallback is **`QemuWhpxAdapter`** — QEMU driven via the Windows Hypervisor Platform (WHPX) accelerator, the only engine that boots a VM natively on Windows without WSL2. Tagged **fallback-only** (C codebase, GPL-2.0 → driven as a subprocess, never statically linked; weaker native-Windows GPU passthrough).
- Microsoft **OpenVMM** (Rust, native Windows + Hyper-V) is the intended future Windows-native microVM backend and is a tracked **watch item**, NOT shipped while it is pre-stable.

**GPU caveat (NORMATIVE) [UPDATED v02.190].** Nested GPU passthrough from a Tier-3 microVM through WSL2 is not yet proven reliable. Until it is, GPU-bound local-model boxing (§3.6) MUST use the first Handshake-native managed adapter that truthfully declares GPU support. WSL2/Podman GPU paths are compatibility-only opt-ins, not defaults. Tier-3 microVM isolation is reserved for code execution that does not require the GPU unless GPU-capable strong isolation is proven. Adapters declare GPU support truthfully via `gpu_passthrough`; the selection policy enforces the GPU guard in §3.5.3.

### 3.5.7 Sandbox interface patterns [ADD v02.187]

The `SandboxAdapter` trait (§3.5.1) and its `ProcessHandle` MUST follow the agent-sandbox interface patterns the 2026 field (microsandbox, Modal, SWE-ReX, E2B, Daytona) has converged on, so backends are swappable without changing callers:

1. **Backend-agnostic trait, swappable implementations** — callers depend only on `SandboxAdapter` + `ProcessSpec`/`ProcessHandle`/`AdapterCapabilities`; the same caller code runs against any tier.
2. **`exec` returns a process handle, not a string** — streamed stdout/stderr plus `exit_code()`/`status()` (§3.5.1).
3. **Stateful sessions vs one-shot exec** — `spawn` establishes a reusable isolation context; `exec` runs additional commands inside it (persistent shell/REPL) distinct from a one-shot spawn.
4. **First-class filesystem namespace** — `fs_bind` plus explicit `copy_in`/`copy_out`; callers never shell out to `cat`/`cp` to move bytes across the boundary.
5. **Network policy declared at create time** — `NetPolicy` (deny-all / loopback-only / allowlist) is set when the context is created, not toggled mid-run.
6. **Hard timeout + idle auto-kill** — `ProcessSpec` carries a hard `timeout` and an `idle_timeout`; the adapter auto-kills and reclaims on either so orphaned sandboxes self-reap (reinforces CX-503D).
7. **Snapshot / restore-as-new** — adapters declaring `supports_snapshot` expose `snapshot(handle) -> SnapshotRef` and `restore(SnapshotRef) -> ProcessHandle` that restores into a FRESH sandbox; this backs the validate-then-promote flow (a validated state is captured and re-spawned, never mutated in place as authority).
8. **Discovery + ownership tracking** — every spawned process is attributable and recoverable across restarts via ProcessOwnershipLedger (§5.7) `sandbox_adapter_id` + handle id; the adapter MUST support enumerating its live handles for reclaim.
9. **Explicit teardown returning status** — `kill` reclaims deterministically and `status`/`exit_code` report the terminal state; no leaked VMs/containers (CX-503D).

Trait methods added in v02.187 (additive to §3.5.1): `copy_in`, `copy_out`, and OPTIONAL `snapshot`/`restore` (gated by `AdapterCapabilities.supports_snapshot`). `ProcessSpec` gains `timeout`, `idle_timeout`, and `trust_class` (default `untrusted_agent`, network default deny-all). Existing callers are unaffected; new fields take safe defaults.

**Cross-references:** §5.2.5 Mechanical Runner Sandbox (high-level scope); §5.7 ProcessOwnershipLedger (consumes adapter id per spawned process); §4.6 ModelRuntime (delegates engine boxing to SandboxAdapter); §3.6 LocalModel Process Boxing (GPU-bound work stays Tier-1); CX-503R (no SQLite); CX-503D (terminal hygiene & reclaim); KERNEL-003 DocketAdapter (DockerAdapter preserves this integration); WP-KERNEL-004 refinement acceptance criteria AC-SANDBOX-ADAPTER-TRAIT, AC-SANDBOX-ADAPTER-IMPLS, AC-SANDBOX-CAP-DECL; v02.187 strong-isolation acceptance criteria AC-SANDBOX-ISOLATION-TIERS, AC-SANDBOX-MICROVM-ADAPTER, AC-SANDBOX-TRUST-SELECTION, AC-SANDBOX-INTERFACE-PATTERNS. Research basis: `.GOV/operator/docs_local/external_research_papers/sandbox-engine-research-basis-v1.md`.

---

## 3.6 LocalModel Process Boxing Invariant (Normative) [ADD v02.186]

**Why**
Pre-Kernel-V1 Handshake leaned on Ollama as an out-of-process daemon — a third-party app whose process lifecycle Handshake could not own, whose model registry sat outside Handshake's authority, and whose updates could silently change inference behavior. KERNEL-004 retires that dependency. This section [ADD v02.186] makes the boxing invariant explicit, so that no future spec or work packet can re-introduce a third-party local-model daemon as an authoritative runtime.

**What**
States the hard invariant that every local-model process Handshake runs MUST be spawned, isolated, and reclaimed inside Handshake; defines the on-disk layout for GGUF + tokenizer caches; carves out a narrow `ExternalEngineImport` lane for operator-pointed compatibility.

---

### 3.6.1 Hard invariant: no third-party model-server wrapper

**HARD**: No Ollama daemon, no LM Studio process, no third-party model-server wrapper is an authoritative ModelRuntime instance under KERNEL-004 and onward. ModelRuntime (§4.6) owns every model process spawned by Handshake. The §4.2.4 narrative is rewritten in v02.186 to remove the Ollama-as-primary recommendation; §4.6 ModelRuntime + LocalModelAdapter is the normative path.

### 3.6.2 Every model process is a sandboxed, ledger-tracked child

Every model process Handshake spawns MUST satisfy both:

1. It is a child of a `SandboxAdapter` (§3.5) — no bare `std::process::Command` model spawns.
2. It registers START + STOP rows in `ProcessOwnershipLedger` (§5.7) via the perf-mitigated batched/async/capped write path (no silent loss).

A model process that violates either condition is a spec defect and MUST fail the §5.6 HBR gate.

### 3.6.3 On-disk layout

The GGUF on-disk store and tokenizer cache live under repo-relative, configured roots only — no user-profile drive letters, no machine-local mount points, no hardcoded paths ([GLOBAL-PORTABILITY-004], [GLOBAL-PORTABILITY-005]). Defaults:

- GGUF store root: `${HANDSHAKE_DATA_ROOT}/models/gguf/` (configured via `settings.local_models.gguf_root`).
- Tokenizer cache root: `${HANDSHAKE_DATA_ROOT}/models/tokenizers/` (configured via `settings.local_models.tokenizer_cache_root`).
- Both roots are bind-mounted into the SandboxAdapter guest in read-only mode at model-load time.

### 3.6.4 ExternalEngineImport lane (compatibility, NOT authority)

Operator MAY point Handshake at a model file path on disk OR at a local OpenAI-compatible HTTP endpoint (e.g., a separately operator-launched llama-server instance) **for compatibility**. This is the `ExternalEngineImport` lane. Such pointers are NOT authoritative ModelRuntime instances:

- Handshake does NOT manage that process's lifecycle.
- The endpoint is never written into the ProcessOwnershipLedger as a Handshake-owned row.
- Capability declarations are inferred (best-effort) from the OpenAI-compatible introspection surface; the model is flagged `provider="external_compat"` in surfaces.
- LlmClient (§4.2.3, CX-101) still mediates every call.

This lane exists so operators can experiment with engines Handshake does not yet adapt, without breaking the boxing invariant for production runtime.

**Cross-references:** §3.5 SandboxAdapter; §4.2.3 LlmClient + CX-101 HARD_LLM_CLIENT; §4.2.4 rewrite (Ollama-as-primary removed v02.186); §4.6 ModelRuntime + LocalModelAdapter; §5.6 HBR gate; §5.7 ProcessOwnershipLedger; CX-102 (no direct HTTP to model endpoints outside LlmClient); WP-KERNEL-004 refinement acceptance criteria AC-LOCAL-MODEL-BOXING, AC-NO-OLLAMA-DAEMON, AC-EXTERNAL-ENGINE-IMPORT.

---


<a id="4-llm-infrastructure"></a>
