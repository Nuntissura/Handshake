---
schema: handshake.indexed_spec.module@1
spec_version: "v02.185"
bundle_id: "master-spec-v02.185"
module_id: "05"
section_id: "5"
title: "5. Security & Observability"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "e3019d9565e98421293edf35dda918ea27e94fd61f7906aac758d8373948763c"
body_sha256: "75a1f696f33ffd50b4edd8e6ae0e1fa03cb8df773af79ea3ae0666f9926eed5f"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 5. Security & Observability

## 5.1 Plugin Architecture

**Why**  
Plugins transform a static application into a living platform. Understanding plugin architecture patternsâ€”both successful and cautionaryâ€”informs how to build extensibility that balances power with safety.

**What**  
Analyzes existing plugin systems (VS Code, Figma, Browser Extensions, Obsidian), designs manifest format with permission declarations, defines plugin types (automation, UI, AI tool), and specifies API patterns for registration and workspace access.
See Sections 10/11 for surface-specific hooks (Terminal/Monaco) and shared capability/sandbox/diagnostics contracts plugins must honor.

**Jargon**  
- **Plugin Manifest**: JSON file declaring plugin metadata, permissions, and contributions.
- **Contributes**: Section of manifest declaring what UI elements (commands, menus) the plugin adds.
- **Activation Events**: Triggers that cause a plugin to load (lazy loading).
- **Declarative Contributions**: UI elements defined in manifest rather than code (commands, menus, views).
- **Extension Host**: Separate process running plugin code (VS Code pattern).
- **Capability Model**: Permission system where plugins declare required access and users consent.

---

### 5.1.1 Why Plugins Matter

#### 5.1.1.1 The Power of Extensibility

**Plugins let your users (and you) add features without changing the core application.**

```
Without Plugins:
  â€¢ Every feature request requires core development
  â€¢ One-size-fits-all: everyone gets everything or nothing
  â€¢ Slow iteration: changes go through your release cycle
  â€¢ Limited: can only do what YOU thought of

With Plugins:
  â€¢ Users can add their own integrations
  â€¢ Personalization: each user's setup is unique
  â€¢ Community innovation: features you never imagined
  â€¢ Faster: plugins ship independently of core app
```

#### 5.1.1.2 Examples of Plugin Value

```
Your app with no plugins:
  â””â”€â”€ Basic AI chat + documents
  
Your app with plugins:
  â”œâ”€â”€ Todoist integration (someone's plugin)
  â”œâ”€â”€ Custom AI model loader (power user)
  â”œâ”€â”€ Citation manager (academic user)
  â”œâ”€â”€ Code formatter (developer)
  â”œâ”€â”€ Voice commands (accessibility)
  â””â”€â”€ [Hundreds more possibilities]
```

---

### 5.1.2 Learning from Existing Systems

#### 5.1.2.1 VS Code: The Gold Standard

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                VS CODE EXTENSION MODEL                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  Runtime:     Separate "Extension Host" process              â”‚
â”‚  Language:    JavaScript/TypeScript                          â”‚
â”‚  Manifest:    package.json with "contributes" section        â”‚
â”‚  Security:    No sandboxâ€”full Node.js access                â”‚
â”‚  Trust:       "Trust this publisher?" prompt                 â”‚
â”‚                                                              â”‚
â”‚  What they got right:                                        â”‚
â”‚    âœ“ Rich API for extending UI                              â”‚
â”‚    âœ“ Lazy loading (activation events)                       â”‚
â”‚    âœ“ Declarative contributions (commands, menus)            â”‚
â”‚    âœ“ Huge ecosystem (50,000+ extensions)                    â”‚
â”‚                                                              â”‚
â”‚  What we'd do differently:                                   â”‚
â”‚    â€¢ Add sandboxing (they have none)                        â”‚
â”‚    â€¢ Require permission declarations                        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.1.2.2 Figma: Security-First

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                   FIGMA PLUGIN MODEL                         â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  Runtime:     Sandboxed JavaScript (no DOM, no XHR)          â”‚
â”‚  UI:          Separate iframe for plugin UI                  â”‚
â”‚  API:         Only Figma document access via figma.*         â”‚
â”‚  Network:     Must whitelist domains in manifest             â”‚
â”‚                                                              â”‚
â”‚  What they got right:                                        â”‚
â”‚    âœ“ True sandboxâ€”plugins can't escape                      â”‚
â”‚    âœ“ UI separated from logic                                â”‚
â”‚    âœ“ Explicit network permissions                           â”‚
â”‚    âœ“ User can cancel runaway plugins                        â”‚
â”‚                                                              â”‚
â”‚  What we'd adapt:                                            â”‚
â”‚    â€¢ Similar sandbox model                                   â”‚
â”‚    â€¢ Manifest-declared network permissions                   â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.1.2.3 Browser Extensions: Permission Model

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚             BROWSER EXTENSION MODEL (Manifest V3)            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  Key Innovation: Explicit permissions                        â”‚
â”‚                                                              â”‚
â”‚  manifest.json:                                              â”‚
â”‚  {                                                           â”‚
â”‚    "permissions": ["storage", "tabs"],                      â”‚
â”‚    "host_permissions": ["https://api.example.com/*"]        â”‚
â”‚  }                                                           â”‚
â”‚                                                              â”‚
â”‚  User sees at install:                                       â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                   â”‚
â”‚  â”‚ "MyExtension" wants to:              â”‚                   â”‚
â”‚  â”‚ â€¢ Read and change your browsing data â”‚                   â”‚
â”‚  â”‚   on api.example.com                 â”‚                   â”‚
â”‚  â”‚ â€¢ Store data locally                 â”‚                   â”‚
â”‚  â”‚                                      â”‚                   â”‚
â”‚  â”‚  [Add Extension]  [Cancel]           â”‚                   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                   â”‚
â”‚                                                              â”‚
â”‚  What we'd copy:                                             â”‚
â”‚    âœ“ Manifest-declared permissions                          â”‚
â”‚    âœ“ User consent at install                                â”‚
â”‚    âœ“ Clear permission descriptions                          â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.1.2.4 Obsidian: Cautionary Tale

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                   OBSIDIAN PLUGIN MODEL                      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  Runtime:     Main Electron process (no isolation!)          â”‚
â”‚  Access:      Full Node.jsâ€”plugins can do ANYTHING          â”‚
â”‚  Trust:       Community ratings + open source review         â”‚
â”‚                                                              â”‚
â”‚  âš ï¸ Security Issue:                                         â”‚
â”‚  "Obsidian plugins have all the same permissions you do     â”‚
â”‚  to read/write all the files in your vault"                 â”‚
â”‚                                                              â”‚
â”‚  A malicious plugin could:                                   â”‚
â”‚    â€¢ Read any file on your computer                         â”‚
â”‚    â€¢ Send data to external servers                          â”‚
â”‚    â€¢ Install malware                                         â”‚
â”‚    â€¢ Encrypt your files (ransomware)                        â”‚
â”‚                                                              â”‚
â”‚  What NOT to copy:                                           â”‚
â”‚    âœ— No sandboxing                                          â”‚
â”‚    âœ— Full system access                                     â”‚
â”‚    âœ— Trust based only on community review                   â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 5.1.3 Plugin Manifest & Registration

#### 5.1.3.1 Plugin Manifest Format

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

#### 5.1.3.2 Key Manifest Sections Explained

| Section | Purpose |
|---------|---------|
| `id` | Unique identifier (reverse domain style) |
| `main` | Entry point JavaScript file |
| `ui` | Optional HTML file for plugin UI panel |
| `permissions` | What the plugin is allowed to access |
| `contributes` | What UI elements the plugin adds |

---

### 5.1.4 Plugin Types & Categories

#### 5.1.4.1 Three Main Categories

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                      PLUGIN TYPES                            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  1. AUTOMATION PLUGINS                                       â”‚
â”‚     â€¢ Background tasks and macros                           â”‚
â”‚     â€¢ Triggered by events or commands                       â”‚
â”‚     â€¢ May not have UI                                        â”‚
â”‚     Example: "Auto-backup to Dropbox"                       â”‚
â”‚                                                              â”‚
â”‚  2. UI PLUGINS                                               â”‚
â”‚     â€¢ Add panels, views, or widgets                         â”‚
â”‚     â€¢ Render custom interfaces                               â”‚
â”‚     â€¢ Interact with user directly                           â”‚
â”‚     Example: "Kanban board view"                            â”‚
â”‚                                                              â”‚
â”‚  3. AI TOOL PLUGINS                                          â”‚
â”‚     â€¢ Add new AI capabilities                               â”‚
â”‚     â€¢ May integrate external models or APIs                 â”‚
â”‚     â€¢ Often combine UI + automation                         â”‚
â”‚     Example: "AI image generator", "Translation tool"       â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 5.1.5 API Design Patterns

#### 5.1.5.1 Registration API Example

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

#### 5.1.5.2 Workspace Data API

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

#### 5.1.5.3 Key Design Principles

ðŸ“Œ **Explicit Registration:** Plugins declare what they contribute via manifest AND register at runtime

ðŸ“Œ **Namespaced:** All plugin commands/views prefixed with plugin ID (`myplugin.command`)

ðŸ“Œ **Promise-based:** All async operations return Promises

ðŸ“Œ **Observable:** Plugins can subscribe to app events

ðŸ“Œ **Permission-gated:** API calls check permissions before executing

---

**Key Takeaways**  
- Plugins transform applications into platforms with community innovation and personalization.
- VS Code shows rich APIs and lazy loading; Figma shows true sandboxing; Browser extensions show permission models.
- Obsidian is a cautionary tale: no sandboxing means plugins can do anything, including ransomware.
- Plugin manifest declares metadata, permissions, and UI contributions; user consents at install.
- Three plugin types: automation (background tasks), UI (views/widgets), AI tools (model integrations).
- API design: explicit registration, namespaced commands, promise-based, observable, permission-gated.

---

## 5.2 Sandboxing & Security

**Why**  
Plugins are a major attack vector. Without sandboxing, any plugin can read files, steal data, or install malware. This section specifies how to run untrusted code safely.

**What**  
Explains why sandboxing is essential, compares sandboxing technologies (WASM, Pyodide, OS subprocess, containers), defines permission categories (filesystem, network, AI, workspace), and recommends a phased security architecture.
Cross-ref: Section 11.2 defines policy vs hard isolation defaults; Section 10.1 documents terminal sandbox expectations.

**Jargon**  
- **Sandbox**: Isolated environment that restricts what code can do.
- **WASM (WebAssembly)**: Binary instruction format that runs in a secure sandbox with no system access unless explicitly granted.
- **Pyodide**: Full Python interpreter compiled to WASM, enabling Python plugins with sandbox security.
- **seccomp/AppArmor/AppContainer**: OS-level sandboxing mechanisms (Linux/Windows).
- **Capability Model**: Security pattern where code receives explicit permission tokens for specific resources.
- **Default Deny**: Security stance where nothing is permitted unless explicitly granted.

---

### 5.2.1 Why Sandbox Untrusted Code

#### 5.2.1.1 The Risk

**Any code you run can do anything your user can do** (unless sandboxed).

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              WHAT UNSANDBOXED CODE CAN DO                    â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  âš ï¸ A malicious plugin WITHOUT sandboxing could:            â”‚
â”‚                                                              â”‚
â”‚  â€¢ Read ANY file on the computer                            â”‚
â”‚    - Browser passwords                                       â”‚
â”‚    - SSH keys                                                â”‚
â”‚    - Financial documents                                     â”‚
â”‚                                                              â”‚
â”‚  â€¢ Send data to external servers                            â”‚
â”‚    - Steal personal information                             â”‚
â”‚    - Exfiltrate business documents                          â”‚
â”‚                                                              â”‚
â”‚  â€¢ Modify or delete files                                   â”‚
â”‚    - Ransomware (encrypt and demand payment)                â”‚
â”‚    - Destroy data                                            â”‚
â”‚                                                              â”‚
â”‚  â€¢ Install malware                                          â”‚
â”‚    - Keyloggers                                              â”‚
â”‚    - Cryptocurrency miners                                   â”‚
â”‚                                                              â”‚
â”‚  This is NOT hypotheticalâ€”it happens regularly              â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.2.1.2 Defense Layers

```
Security = Multiple Layers

Layer 1: PERMISSION MODEL
  â€¢ Plugin declares what it needs
  â€¢ User consents at install
  â€¢ App only grants what was approved

Layer 2: SANDBOX
  â€¢ Plugin code runs in isolation
  â€¢ Cannot access system outside sandbox
  â€¢ Even if code is malicious, damage is limited

Layer 3: REVIEW PROCESS
  â€¢ Marketplace review before listing
  â€¢ Automated security scanning
  â€¢ Community reporting

Layer 4: MONITORING
  â€¢ Track plugin behavior
  â€¢ Alert on suspicious activity
  â€¢ Ability to remotely disable malicious plugins
```

---

### 5.2.2 Sandboxing Technologies Compared

#### 5.2.2.1 Overview Table

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ Technology   â”‚ Security   â”‚ Performance â”‚ Complexity   â”‚ Best For     â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ WASM         â”‚ â­â­â­â­â­    â”‚ â­â­â­â­      â”‚ â­â­â­ Medium  â”‚ Most plugins â”‚
â”‚ Pyodide      â”‚ â­â­â­â­â­    â”‚ â­â­â­        â”‚ â­â­â­ Medium  â”‚ Python AI    â”‚
â”‚ OS Subprocessâ”‚ â­â­â­â­      â”‚ â­â­â­â­â­     â”‚ â­â­ Complex  â”‚ Legacy code  â”‚
â”‚ Containers   â”‚ â­â­â­â­â­    â”‚ â­â­          â”‚ â­ Very High  â”‚ Heavy/risky  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.2.2.2 WebAssembly (WASM) â€” Recommended

**What it is:** A binary instruction format that runs in a secure sandbox.

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    CORE CONCEPT: WASM Sandbox
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

  Plugin code compiles to WASM (from Rust, C++, AssemblyScript)
  
  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
  â”‚                    YOUR APPLICATION                      â”‚
  â”‚                                                          â”‚
  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
  â”‚  â”‚              WASM SANDBOX                        â”‚    â”‚
  â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚    â”‚
  â”‚  â”‚  â”‚         PLUGIN CODE                       â”‚  â”‚    â”‚
  â”‚  â”‚  â”‚  â€¢ Cannot access filesystem               â”‚  â”‚    â”‚
  â”‚  â”‚  â”‚  â€¢ Cannot make network requests           â”‚  â”‚    â”‚
  â”‚  â”‚  â”‚  â€¢ Cannot read memory outside sandbox     â”‚  â”‚    â”‚
  â”‚  â”‚  â”‚  â€¢ Can ONLY call functions YOU expose     â”‚  â”‚    â”‚
  â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚    â”‚
  â”‚  â”‚                                                  â”‚    â”‚
  â”‚  â”‚  Exposed Functions (your API):                  â”‚    â”‚
  â”‚  â”‚  â€¢ readDocument(id) â†’ document                  â”‚    â”‚
  â”‚  â”‚  â€¢ saveDocument(id, content)                    â”‚    â”‚
  â”‚  â”‚  â€¢ showUI(html)                                 â”‚    â”‚
  â”‚  â”‚  â€¢ [nothing elseâ€”no system access]             â”‚    â”‚
  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
  â”‚                                                          â”‚
  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
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

#### 5.2.2.3 Pyodide (Python in WASM)

**What it is:** Full Python interpreter compiled to WASM.

```
Pyodide gives you Python plugins with WASM security.

Pros:
  âœ“ Full Python ecosystem (numpy, pandas, etc.)
  âœ“ Inherits WASM sandbox properties
  âœ“ Plugin authors write normal Python

Cons:
  âœ— Slower than native Python
  âœ— Large initial download (~10MB+)
  âœ— Startup time can be significant
```

**Best for:** AI/data plugins that need Python libraries.

#### 5.2.2.4 OS Subprocess Sandboxing

**What it is:** Running plugins as separate OS processes with restricted permissions.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                 OS-LEVEL SANDBOXING                          â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  Main App                    Plugin Process                  â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”              â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚           â”‚  IPC/Pipes   â”‚ Restricted by:            â”‚   â”‚
â”‚  â”‚   Your    â”‚â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–ºâ”‚ â€¢ seccomp (Linux)         â”‚   â”‚
â”‚  â”‚   App     â”‚              â”‚ â€¢ AppArmor (Linux)        â”‚   â”‚
â”‚  â”‚           â”‚              â”‚ â€¢ sandbox-exec (macOS)    â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜              â”‚ â€¢ AppContainer (Windows)  â”‚   â”‚
â”‚                             â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                                              â”‚
â”‚  Can block:                                                  â”‚
â”‚    â€¢ File access outside allowed paths                      â”‚
â”‚    â€¢ Network access                                          â”‚
â”‚    â€¢ Process spawning                                        â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 5.2.3 Permission Models

#### 5.2.3.1 Capability Categories

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                  PERMISSION CATEGORIES                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  FILESYSTEM SCOPES                                           â”‚
â”‚  â”œâ”€â”€ fs.read[/workspace/*]     Read specific paths          â”‚
â”‚  â”œâ”€â”€ fs.write[/workspace/out]  Write to specific paths      â”‚
â”‚  â””â”€â”€ fs.none                   No filesystem access         â”‚
â”‚                                                              â”‚
â”‚  NETWORK SCOPES                                              â”‚
â”‚  â”œâ”€â”€ net.none                  No network (default)         â”‚
â”‚  â”œâ”€â”€ net.host[api.example.com] Specific domains only        â”‚
â”‚  â””â”€â”€ net.any                   Unrestricted (dangerous)     â”‚
â”‚                                                              â”‚
â”‚  AI/MODEL SCOPES                                             â”‚
â”‚  â”œâ”€â”€ ai.none                   Cannot use AI                â”‚
â”‚  â”œâ”€â”€ ai.local                  Local models only            â”‚
â”‚  â”œâ”€â”€ ai.cloud                  Can call cloud APIs          â”‚
â”‚  â””â”€â”€ ai.budget[10000]          Token limit per day          â”‚
â”‚                                                              â”‚
â”‚  WORKSPACE DATA SCOPES                                       â”‚
â”‚  â”œâ”€â”€ workspace.read            Read documents/boards        â”‚
â”‚  â”œâ”€â”€ workspace.write           Modify data                  â”‚
â”‚  â””â”€â”€ workspace.none            No access to user data       â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.2.3.2 Install-Time Permission Dialog

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                                                              â”‚
â”‚  Install "AI Writing Assistant"?                            â”‚
â”‚                                                              â”‚
â”‚  This plugin requests:                                       â”‚
â”‚                                                              â”‚
â”‚  ðŸ“ Read your documents                                      â”‚
â”‚     To analyze and improve your writing                     â”‚
â”‚                                                              â”‚
â”‚  ðŸŒ Network access to api.grammarly.com                     â”‚
â”‚     To check grammar and spelling                           â”‚
â”‚                                                              â”‚
â”‚  ðŸ¤– Use local AI models                                      â”‚
â”‚     To generate writing suggestions                         â”‚
â”‚                                                              â”‚
â”‚  â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€              â”‚
â”‚                                                              â”‚
â”‚  âš ï¸ This plugin cannot:                                     â”‚
â”‚     â€¢ Access files outside your workspace                   â”‚
â”‚     â€¢ Access other websites                                 â”‚
â”‚     â€¢ Modify system settings                                â”‚
â”‚                                                              â”‚
â”‚         [Cancel]                [Install]                   â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 5.2.4 Recommended Security Architecture

```
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
                    DECISION POINT: Security Architecture
â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

RECOMMENDED: WASM-First with Permission Model

Phase 1 (Internal Plugins):
  â””â”€â”€ Simple process isolation
      â€¢ Run plugins as subprocesses
      â€¢ Limit via OS mechanisms where easy
      â€¢ Internal plugins are trusted (from your team)

Phase 2 (Community Plugins):
  â””â”€â”€ WASM sandbox for all third-party code
      â€¢ Compile plugins to WASM
      â€¢ Expose only necessary APIs
      â€¢ Manifest-declared permissions
      â€¢ User consent dialog at install

Phase 3 (Marketplace):
  â””â”€â”€ Full security pipeline
      â€¢ Automated security scanning
      â€¢ Manual review for sensitive permissions
      â€¢ Code signing
      â€¢ Remote disable capability

DEFAULT STANCE: Deny Everything
  â€¢ No filesystem access by default
  â€¢ No network by default
  â€¢ No AI access by default
  â€¢ Plugin must request; user must grant

â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
```

---

**Key Takeaways**  
- Unsandboxed plugins can read any file, steal data, install malware, or ransomware your files.
- Defense requires multiple layers: permission model + sandbox + review process + monitoring.
- WASM provides strong, proven isolation with near-native performanceâ€”recommended for most plugins.
- Permission categories: filesystem, network, AI/models, workspace dataâ€”each with scoped grants.
- Default deny: plugins get nothing unless they request it in manifest and user approves.
- Phased approach: start with subprocess isolation for internal plugins, add WASM for community plugins.

---

### 5.2.5 Mechanical Runner Sandbox
- Mechanical engines run via a constrained runner: explicit allowlist per engine, resource limits (CPU/GPU/mem/time), and capability gates (file/process/network/device).
- Log command, params, cwd, exit code, stdout/stderr, artifact hashes; refuse/abort when capability is missing or bounds exceeded.
- Provide refusal paths and tests to ensure engines cannot bypass Workflow/Flight Recorder or capabilities.

## 5.3 AI Observability

**Why**  
AI systems are probabilisticâ€”the same input can produce different outputs. Traditional debugging doesn't apply. This section defines what to monitor and how to debug AI behavior.

**What**  
Explains why AI needs different observability, defines key metrics (performance, resource, quality, cost), compares tools (OpenTelemetry + Prometheus vs Langfuse vs LangSmith), covers privacy-sensitive logging, and provides dashboard/instrumentation examples.
See Sections 10/11 for terminal/editor Flight Recorder events, diagnostics schema, and capability-linked logging policies.

**Jargon**  
- **Observability**: The ability to understand internal system state from external outputs (metrics, logs, traces).
- **OpenTelemetry (OTel)**: Vendor-neutral standard for collecting metrics, logs, and traces.
- **Prometheus**: Time-series database for storing metrics.
- **Grafana**: Visualization tool for metrics dashboards.
- **Langfuse**: Open-source LLM observability platform (self-hostable).
- **Trace**: End-to-end record of a request's path through the system.
- **Span**: A single unit of work within a trace.

---

### 5.3.1 What to Monitor in AI Apps

#### 5.3.1.1 Why AI Needs Different Observability

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

#### 5.3.1.2 Key Metrics to Track

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    AI OBSERVABILITY METRICS                  â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  PERFORMANCE METRICS                                         â”‚
â”‚  â”œâ”€â”€ Latency (p50, p95, p99)   How long requests take       â”‚
â”‚  â”œâ”€â”€ Tokens per second         Throughput measure           â”‚
â”‚  â”œâ”€â”€ Time to first token       Perceived responsiveness     â”‚
â”‚  â””â”€â”€ Queue depth               Backlog of requests          â”‚
â”‚                                                              â”‚
â”‚  RESOURCE METRICS                                            â”‚
â”‚  â”œâ”€â”€ GPU memory usage          Are we close to OOM?         â”‚
â”‚  â”œâ”€â”€ GPU utilization %         Is GPU being used?           â”‚
â”‚  â”œâ”€â”€ CPU/RAM usage             System health                â”‚
â”‚  â””â”€â”€ Model load/unload events  Memory management working?   â”‚
â”‚                                                              â”‚
â”‚  QUALITY SIGNALS                                             â”‚
â”‚  â”œâ”€â”€ Error rate                Model failures               â”‚
â”‚  â”œâ”€â”€ Retry rate                Had to try again             â”‚
â”‚  â”œâ”€â”€ Fallback rate             Localâ†’cloud switches         â”‚
â”‚  â”œâ”€â”€ User feedback             Thumbs up/down               â”‚
â”‚  â””â”€â”€ Task completion           Did user accomplish goal?    â”‚
â”‚                                                              â”‚
â”‚  COST METRICS (if using cloud APIs)                         â”‚
â”‚  â”œâ”€â”€ Tokens consumed           Input + output               â”‚
â”‚  â”œâ”€â”€ API spend                 Actual money                 â”‚
â”‚  â””â”€â”€ Local vs cloud ratio      How much offloaded?         â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 5.3.2 Tools Comparison

#### 5.3.2.1 Overview

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ Tool        â”‚ Type           â”‚ Local-First? â”‚ Best For      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ OTel+Prom   â”‚ General obs.   â”‚ âœ“ Yes        â”‚ Core metrics  â”‚
â”‚ Langfuse    â”‚ LLM-specific   â”‚ Self-hosted  â”‚ Full tracing  â”‚
â”‚ LangSmith   â”‚ LLM-specific   â”‚ Cloud only   â”‚ LangChain     â”‚
â”‚ Helicone    â”‚ LLM proxy      â”‚ Self-hosted  â”‚ Caching       â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.3.2.2 OpenTelemetry + Prometheus + Grafana â€” Recommended Core

**What it is:** Industry-standard observability stack.

```
The "boring but reliable" choice:

â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                                                              â”‚
â”‚  Your App â”€â”€â–º OpenTelemetry â”€â”€â–º Prometheus â”€â”€â–º Grafana     â”‚
â”‚  (metrics)    (collection)      (storage)     (dashboards) â”‚
â”‚                                                              â”‚
â”‚  Also:                                                       â”‚
â”‚  Your App â”€â”€â–º OTel â”€â”€â–º Jaeger/Tempo â”€â”€â–º Grafana            â”‚
â”‚  (traces)                (storage)      (visualization)     â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**Pros:**
- â­ Fully localâ€”no data leaves your machine
- â­ Vendor-neutral standard
- â­ Works with any backend (vLLM, TGI expose Prometheus metrics)
- â­ Flexibleâ€”you define what to track

**Cons:**
- âš ï¸ No LLM-specific features out of box
- âš ï¸ Must design your own metrics/spans
- âš ï¸ Setup requires several components

#### 5.3.2.3 Langfuse â€” Best LLM-Specific (Self-Hosted)

**What it is:** Open-source LLM observability platform.

```
Langfuse tracks:
  â€¢ Every prompt and response
  â€¢ Token counts and costs
  â€¢ Latency breakdowns
  â€¢ Tool calls within agents
  â€¢ User feedback
```

**Pros:**
- â­ Open-source, self-hostable
- â­ Purpose-built for LLM debugging
- â­ Tracks costs and tokens automatically
- â­ Integrates via OpenTelemetry

**Cons:**
- âš ï¸ Requires running Postgres + Langfuse server
- âš ï¸ Heavier setup than plain OTel

---

### 5.3.3 Privacy-Sensitive Logging

#### 5.3.3.1 The Problem

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
  â€¢ Email address (PII)
  â€¢ Salary information (sensitive)
  â€¢ Professional context (private)
```

#### 5.3.3.2 Best Practices

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                PRIVACY-SAFE LOGGING                          â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  1. REDACT BEFORE LOGGING                                    â”‚
â”‚     â€¢ Use regex/libraries to detect PII                     â”‚
â”‚     â€¢ Replace: "john.doe@company.com" â†’ "[EMAIL]"           â”‚
â”‚     â€¢ Tools: llm-guard Anonymize scanner                    â”‚
â”‚                                                              â”‚
â”‚  2. LOG METADATA, NOT CONTENT                                â”‚
â”‚     Good: { task: "email_draft", tokens_in: 50, success: T }â”‚
â”‚     Bad:  { prompt: "Write email to john...", ... }         â”‚
â”‚                                                              â”‚
â”‚  3. SAMPLE, DON'T LOG EVERYTHING                             â”‚
â”‚     â€¢ Log 10% of interactions for debugging                 â”‚
â”‚     â€¢ Full logs only with explicit user consent             â”‚
â”‚                                                              â”‚
â”‚  4. SHORT RETENTION                                          â”‚
â”‚     â€¢ Delete detailed logs after 7-30 days                  â”‚
â”‚     â€¢ Keep aggregated metrics longer                        â”‚
â”‚                                                              â”‚
â”‚  5. LOCAL ONLY                                               â”‚
â”‚     â€¢ Never send raw prompts to cloud services              â”‚
â”‚     â€¢ If cloud needed, anonymize first                      â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.3.3.3 Safe Logging Schema

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

### 5.3.4 Metrics & Dashboards

#### 5.3.4.1 Essential Dashboard Panels

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    GRAFANA DASHBOARD                         â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  ROW 1: HEALTH AT A GLANCE                                   â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”            â”‚
â”‚  â”‚ Requests/minâ”‚ â”‚ Error Rate  â”‚ â”‚ p95 Latency â”‚            â”‚
â”‚  â”‚    42       â”‚ â”‚   0.5%      â”‚ â”‚   850ms     â”‚            â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜            â”‚
â”‚                                                              â”‚
â”‚  ROW 2: LATENCY OVER TIME                                    â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”           â”‚
â”‚  â”‚  â”€â”€â”€â”€p50   â”€â”€â”€â”€p95   â”€â”€â”€â”€p99                 â”‚           â”‚
â”‚  â”‚     â•­â”€â”€â”€â”€â”€â”€â•®      â•­â”€â”€â”€â”€â”€â”€â”€â”€â”€â•®                â”‚           â”‚
â”‚  â”‚  â”€â”€â”€â•¯      â•°â”€â”€â”€â”€â”€â”€â•¯         â•°â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€    â”‚           â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜           â”‚
â”‚                                                              â”‚
â”‚  ROW 3: RESOURCES                                            â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”          â”‚
â”‚  â”‚ GPU Memory           â”‚ â”‚ GPU Utilization      â”‚          â”‚
â”‚  â”‚ â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–‘â–‘â–‘ 75%    â”‚ â”‚ â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–‘â–‘â–‘â–‘ 67%     â”‚          â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜          â”‚
â”‚                                                              â”‚
â”‚  ROW 4: BY MODEL                                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”           â”‚
â”‚  â”‚ Model      â”‚ Requests â”‚ Avg Latency â”‚ Errors â”‚           â”‚
â”‚  â”‚ mistral-7b â”‚ 1,234    â”‚ 340ms       â”‚ 0.2%   â”‚           â”‚
â”‚  â”‚ codellama  â”‚ 567      â”‚ 520ms       â”‚ 0.8%   â”‚           â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜           â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.3.4.2 Instrumentation Example

```python
from opentelemetry import trace, metrics

tracer = trace.get_tracer(__name__)
meter = metrics.get_meter(__name__)

```
#  Define metrics
request_counter = meter.create_counter(
    "llm_requests_total",
    description="Total LLM requests"
)
latency_histogram = meter.create_histogram(
    "llm_latency_seconds",
    description="LLM request latency"
)

```
#  Instrument a function
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

**Key Takeaways**  
- AI apps are probabilistic; traditional debugging doesn't workâ€”observability is essential.
- Track four metric categories: performance (latency, throughput), resources (GPU/memory), quality signals (errors, feedback), cost (tokens, API spend).
- OpenTelemetry + Prometheus + Grafana is the recommended local-first stack; add Langfuse for LLM-specific tracing.
- Privacy: log metadata not content; redact PII; sample instead of logging everything; short retention.
- Build dashboards showing health at a glance, latency over time, resource usage, and per-model breakdowns.

---

### 5.3.6 Distillation Observability Requirements
- Distillation jobs MUST emit Flight Recorder events for each stage (select, teacher run, student run, score, checkpoint, eval, promote/rollback) with trace IDs.
- Required fields: model/tokenizer ids, inference params, context refs (files/spec sections/tools), metrics (pass@k, compile/test rates, collapse indicators), reward features, lineage (parent_checkpoint_id), data_signature, job_ids_json, promotion decisions.
- PII/secret handling: apply log-time redaction and pre-training scrubbing; enforce capability-based export controls for Skill Bank artifacts.
- Dashboards/traces should surface promotion gates vs teacher/previous checkpoints and collapse indicators for regression detection.
- [ADD v02.157] Distillation observability MUST also record Context Pack hashes/freshness decisions, PromptEnvelope hashes, and pending-candidate queue transitions whenever Context Packs or Spec Router artifacts shape teacher/student inputs.

### 5.3.7 Log Privacy & Retention
- Apply redaction rules for PII/secrets on prompts/outputs before logging; enable user/ workspace opt-out flags.
- Set retention periods and export controls for Flight Recorder/log artifacts; require capability grants for export/off-device.
- Add CI checks to prevent accidental logging of sensitive fields; document redaction coverage and gaps.

## 5.4 Evaluation & Quality

**Why**  
LLM outputs are non-deterministicâ€”traditional unit tests with exact expected values don't work. This section defines testing strategies for AI systems.

**What**  
Addresses the challenge of testing non-deterministic outputs, introduces testing strategies (golden test suites, property-based tests, LLM-as-judge), and covers multi-agent tracing for complex workflows.

**Jargon**  
- **Golden Test Suite**: Set of representative prompts with expected properties or keywords to verify.
- **Property-Based Test**: Test that checks structural properties (valid JSON, contains keys) rather than exact content.
- **LLM-as-Judge**: Using another LLM to rate output quality on criteria like correctness and helpfulness.
- **Multi-Agent Tracing**: Tracking request flow through multiple agents/models to debug complex systems.

---

### 5.4.1 Testing LLM Outputs

#### 5.4.1.1 The Challenge

**LLM outputs are non-deterministic.** Traditional unit tests expect exact outputs:

```python
#  Traditional test (deterministic)
def test_add():
    assert add(2, 3) == 5  # Always passes or fails consistently

```
#  LLM test (non-deterministic)
def test_poem():
    poem = llm("Write a haiku about code")
    # TODO: define golden output for this test case
    assert poem == "<EXPECTED_POEM>"
```

#### 5.4.1.2 Testing Strategies

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                  LLM TESTING STRATEGIES                      â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  1. GOLDEN TEST SUITES                                       â”‚
â”‚     â€¢ Define representative test prompts                    â”‚
â”‚     â€¢ For deterministic tasks: check exact output           â”‚
â”‚     â€¢ For generative tasks: check key properties            â”‚
â”‚                                                              â”‚
â”‚  Example:                                                    â”‚
â”‚    Prompt: "What is 2+2?"                                   â”‚
â”‚    Assert: "4" in response.lower()                          â”‚
â”‚                                                              â”‚
â”‚  2. PROPERTY-BASED TESTS                                     â”‚
â”‚     â€¢ Check structural properties, not exact content        â”‚
â”‚     â€¢ Response length in expected range                     â”‚
â”‚     â€¢ Contains required keywords                            â”‚
â”‚     â€¢ Valid JSON/format                                     â”‚
â”‚                                                              â”‚
â”‚  Example:                                                    â”‚
â”‚    Prompt: "Write JSON with name and age"                   â”‚
â”‚    Assert: valid JSON, has "name" key, has "age" key        â”‚
â”‚                                                              â”‚
â”‚  3. LLM-AS-JUDGE                                             â”‚
â”‚     â€¢ Use another LLM to evaluate output quality            â”‚
â”‚     â€¢ Rate on criteria: correctness, coherence, helpfulness â”‚
â”‚     â€¢ Scalable but adds latency/cost                        â”‚
â”‚                                                              â”‚
â”‚  Example:                                                    â”‚
â”‚    Ask GPT-4: "Rate this response 1-5 for helpfulness: ..." â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.4.1.3 Golden Test Example

```python
#  tests/test_llm_golden.py

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

### 5.4.2 Multi-Agent Tracing

#### 5.4.2.1 The Complexity

**Multi-agent systems have many components talking to each other.** Debugging requires seeing the full flow.

```
User Request: "Summarize this document and create action items"

Agent Flow:
  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
  â”‚ Orchestratorâ”‚â”€â”€â–º "This needs summarization + extraction"
  â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜
         â”‚
    â”Œâ”€â”€â”€â”€â”´â”€â”€â”€â”€â”
    â”‚         â”‚
    â–¼         â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚Summaryâ”‚ â”‚Extractorâ”‚
â”‚ Agent â”‚ â”‚  Agent  â”‚
â””â”€â”€â”€â”¬â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”˜
    â”‚          â”‚
    â–¼          â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚Mistralâ”‚ â”‚CodeLlamaâ”‚
â”‚  LLM  â”‚ â”‚   LLM   â”‚
â””â”€â”€â”€â”¬â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”˜
    â”‚          â”‚
    â””â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”˜
         â”‚
         â–¼
    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”
    â”‚ Combine â”‚
    â”‚ Results â”‚
    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 5.4.2.2 Tracing with OpenTelemetry

```python
#  Each agent action becomes a span
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

#### 5.4.2.3 Trace Visualization

```
In Jaeger/Tempo, you'd see:

user_request                     [â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•] 2.5s
  â””â”€ orchestrator_decision       [â•â•]                               0.1s
  â””â”€ summary_agent               [â•â•â•â•â•â•â•â•â•â•â•â•â•â•]                   1.2s
       â””â”€ llm_call_mistral       [â•â•â•â•â•â•â•â•â•â•â•â•]                     1.0s
  â””â”€ extractor_agent             [â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•]                 1.5s
       â””â”€ llm_call_codellama     [â•â•â•â•â•â•â•â•â•â•â•â•â•â•]                   1.3s
  â””â”€ combine_results             [â•â•]                               0.1s
```

---

**Key Takeaways**  
- LLM outputs are non-deterministic; traditional exact-match tests don't work.
- Three strategies: golden test suites (check key properties/keywords), property-based tests (check structure), LLM-as-judge (rate quality).
- Multi-agent systems need full flow tracingâ€”use OpenTelemetry spans to track each agent and model call.
- Trace visualization shows timing breakdown and helps identify bottlenecks in complex workflows.

---


### 5.4.5 Kernel V1 Authority Observability Boundary [ADD v02.184]

Flight Recorder remains mandatory append-only observability, but Kernel V1 replay and promotion authority MUST come from the Postgres EventLedger defined in Section 2.3.13.9. A Flight Recorder record, provider trace, log line, terminal transcript, DCC projection, or generated Markdown file MUST NOT be treated as the authoritative source for a Kernel V1 state transition unless it references the durable EventLedger event ID and run IDs that carry the authority.

Kernel V1 observability MUST expose enough structured fields for no-context debugging:

- `kernel_task_run_id`
- `session_run_id`
- `event_ledger_id`
- `event_type`
- `actor`
- `causation_id`
- `correlation_id`
- `context_bundle_id`
- `artifact_proposal_id` when applicable
- `validation_run_id` when applicable
- `promotion_gate_id` when applicable

Security and observability tests for Kernel V1 MUST prove that replay still works when Flight Recorder or provider trace history is unavailable and EventLedger rows remain intact.

### 5.4.6 Governance Compliance Tests

These tests verify that Handshake correctly implements the governance rules from the Bootloader and Execution Charter.

#### 5.4.6.1 Bootloader Compliance Tests

```rust
// tests/bootloader_compliance.rs

#[cfg(test)]
mod bootloader_tests {
    use super::*;

    /// BL-30: Challenge hidden assumptions before planning
    #[test]
    fn test_bl_30_challenge_assumptions() {
        let job = Job::non_trivial().with_no_assumptions();
        let validator = ChallengeFirstValidator;
        let result = validator.validate(&job, &context());
        
        assert!(result.has_warning("BL-30"));
    }
    
    /// BL-104: Flight Recorder is append-only
    #[test]
    fn test_bl_104_append_only() {
        let mut recorder = FlightRecorder::new();
        recorder.append(test_entry()).unwrap();
        
        // These methods should not exist (compile-time guarantee)
        // recorder.delete(step_id);  // Compile error
        // recorder.update(step_id, new_entry);  // Compile error
        
        // Can only read and append
        assert!(recorder.entries().len() == 1);
    }
    
    /// BL-272: No unanchored operations
    #[test]
    fn test_bl_272_no_unanchored() {
        let result = PlannedOperation::new(
            OperationType::Insert,
            EntityRef::empty(),  // Unanchored - should fail
            LocationSelector::default(),
            None,
            ContentSnapshot::new("content"),
            "reason",
        );
        
        assert!(matches!(result, Err(Error::UnanchoredOperation { clause: "BL-272" })));
    }
    
    /// BL-274: Deletions must be explicit with before content
    #[test]
    fn test_bl_274_explicit_delete() {
        let op = PlannedOperation::delete(
            entity_ref(),
            location(),
            ContentSnapshot::new("old content"),
            "reason for deletion",
        );
        
        assert_eq!(op.operation_type, OperationType::Delete);
        assert!(op.before.is_some());
        assert!(op.after.is_empty());
        assert!(!op.risks.is_empty());
    }
    
    /// BL-24: Flight Recorder cannot be disabled
    #[test]
    fn test_bl_24_recorder_mandatory() {
        let runtime = Runtime::new(config());
        
        // Flight Recorder is always present
        assert!(runtime.flight_recorder().is_some());
        
        // Cannot disable it
        let result = runtime.disable_flight_recorder();
        assert!(result.is_err());
    }
}
```

#### 5.4.6.2 Execution Charter Compliance Tests

```rust
// tests/execution_charter_compliance.rs

#[cfg(test)]
mod execution_charter_tests {
    use super::*;

    /// EXEC-003: Default to activated mode
    #[test]
    fn test_exec_003_default_activated() {
        let mode = ProjectMode::default();
        assert_eq!(mode, ProjectMode::Activated);
    }
    
    /// EXEC-024/028: Layer 1 is immutable
    #[test]
    fn test_exec_024_l1_immutable() {
        let guard = LayerGuard;
        let operation = PlannedOperation::write_to(Layer::L1);
        
        let result = guard.check_write(Layer::L1, &operation);
        assert!(result.is_err());
        assert!(result.unwrap_err().code == "EXEC-024/028");
    }
    
    /// EXEC-043: No structural edits in non-structural modes
    #[test]
    fn test_exec_043_no_structural_in_chat() {
        let engine = EscalationEngine;
        let operation = PlannedOperation::structural();
        
        for mode in [WorkMode::Chat, WorkMode::Data, WorkMode::Brainstorm] {
            let result = engine.validate_mode_allows_structural(mode, &operation);
            assert!(result.is_err());
            assert!(result.unwrap_err().code == "EXEC-043");
        }
    }
    
    /// EXEC-050: Cannot fabricate non-existent references
    #[test]
    fn test_exec_050_no_fabrication() {
        let mut discovery = ReferenceDiscovery::new();
        discovery.full_scan(&empty_index()).unwrap();
        
        let result = discovery.get_reference(&RefId::new("FAKE-001"));
        assert!(result.is_err());
        assert!(result.unwrap_err().code == "EXEC-050");
    }
    
    /// EXEC-073: No governed tasks during drift state
    #[test]
    fn test_exec_073_drift_blocks_tasks() {
        let mut handler = DriftHandler::new();
        handler.set_drift_detected(true);
        
        let result = handler.can_execute_governed_task();
        assert!(result.is_err());
        assert!(result.unwrap_err().code == "EXEC-073");
    }
}
```

#### 5.4.6.3 COR-701 Compliance Tests

```rust
// tests/cor701_compliance.rs

#[cfg(test)]
mod cor701_tests {
    use super::*;

    /// C701-50: Anchors must be present
    #[test]
    fn test_c701_50_anchors_present() {
        let step = Step::new()
            .with_anchor("nonexistent-anchor");
        let context = Context::with_empty_document();
        
        let gate = AnchorsPresent;
        let result = gate.check(&step, &context);
        
        assert!(!result.unwrap().passed);
    }
    
    /// C701-52: Content outside window unchanged
    #[test]
    fn test_c701_52_boundary_protection() {
        let mut document = Document::with_content("AAABBBCCC");
        let step = Step::new()
            .with_window(Window::new(3, 6))  // Target "BBB"
            .with_after("XXX");
        
        // Simulate edit that also modifies outside window
        document.apply_corrupt_edit(&step, true);  // Corrupts boundary
        
        let gate = ContentUntouchedOutsideWindow;
        let result = gate.check(&step, &document.context());
        
        assert!(!result.unwrap().passed);
        assert!(result.unwrap().error_code == "C701-52");
    }
    
    /// C701-60: Concurrency check
    #[test]
    fn test_c701_60_concurrency_check() {
        let document = Document::with_content("original");
        let pre_hash = document.compute_hash();
        
        let step = Step::new()
            .with_pre_hash(&pre_hash);
        
        // Simulate concurrent modification
        document.modify_externally("modified");
        
        let gate = CurrentMatchesPreimage;
        let result = gate.check(&step, &document.context());
        
        assert!(!result.unwrap().passed);
        assert!(result.unwrap().error_code == "C701-60");
    }
    
    /// C701-47: Automatic rollback on failure
    #[test]
    fn test_c701_47_rollback() {
        let mut document = Document::with_content("original content");
        let original_hash = document.compute_hash();
        
        let step = Step::new()
            .with_after("new content")
            .with_failing_gate();  // Will fail a gate
        
        let executor = MicroStepExecutor::new();
        let result = executor.execute(&step, &mut document.context());
        
        // Edit failed
        assert!(result.is_err());
        
        // But content was rolled back
        assert_eq!(document.compute_hash(), original_hash);
    }
    
    /// C701-181: Governed edit requires manifest
    #[test]
    fn test_c701_181_manifest_required() {
        let operation = PlannedOperation::governed()
            .without_manifest();
        
        let behavior = EditBehavior;
        let result = behavior.validate_has_manifest(&operation);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().code == "C701-181");
    }
}
```


---


### 5.4.8 Front End Memory System Test Suite (FEMS-EVAL-001) (Normative) [ADD v02.138]

This suite validates that FEMS provides continuity **without** context bloat, poisoning, or irreproducibility.

**FEMS-EVAL-001.1 Budget + truncation**
- `MemoryPack.token_estimate` MUST be â‰¤ 500 (default) and MUST honor Work Profile overrides.
- If truncation occurs, it MUST be deterministic and MUST set a truncation warning flag.

**FEMS-EVAL-001.2 Provenance + bounded selectors**
- Every committed `MemoryItem` MUST carry bounded `SourceRef`s.
- Every `MemoryPackItem` MUST carry bounded `source_refs` (no â€œwhole documentâ€ selectors).

**FEMS-EVAL-001.3 Anti-poisoning / instruction suppression**
- Untrusted content (tool output, external web, user messages) MUST NOT be promotable into procedural memory without review.
- Memory extraction MUST reject â€œdo this tool callâ€ / â€œignore previous rulesâ€ style instructions as invalid memory content.

**FEMS-EVAL-001.4 Determinism & replay**
- In `replay` mode, the `MemoryPack` hash and selected `memory_id`s MUST match on repeated runs given pinned indices + identical inputs.

**FEMS-EVAL-001.5 Cloud redaction correctness**
- When `provider=cloud`, high-sensitivity memory MUST be excluded or redacted unless a consent receipt explicitly permits inclusion.
- Decisions MUST be visible in the `ContextSnapshot` and in DCC.

**FEMS-EVAL-001.6 Consolidation + conflict behavior**
- Dedupe merges MUST be stable and explainable.
- Conflicts MUST create superseded versions or conflict sets; no silent overwrites.

**Performance check (non-CI optional)**
- p95 `retrieve+pack.build` â‰¤ 500ms on a warmed cache for a medium workspace fixture.

## 5.5 Benchmark Harness

**Why**  
Reproducible performance testing enables informed decisions about runtimes, models, and configurations. This section specifies a systematic benchmarking system.

**What**  
Describes benchmark harness architecture (config files, adapters, runners, output), provides example configurations and adapter interface, and shows reporting format for comparing runtimes/models.

**Jargon**  
- **Benchmark Harness**: Framework for running reproducible performance tests.
- **Adapter**: Code that translates generic benchmark calls to specific runtime APIs (Ollama, vLLM, TGI).
- **Scenario**: A defined test configuration (prompts, models, concurrency levels, iterations).
- **Load Sweep**: Running tests at increasing concurrency levels to measure scaling behavior.

---


#### 5.4.6.4 Calendar Law Compliance Tests

These tests verify that the **Calendar Law** (see Â§10.4.1) is enforced by validators and cannot be bypassed by UI code, tool calls, or direct storage mutations.

Key invariants covered:

- **RBC is view-only**: UI may render calendar state, but MUST NOT write to calendar tables directly.
- **All mutations are patch-sets**: changes flow through the AI Job Model + Workflow Engine, then `calendar_sync` applies them.
- **External writes are gated**: any provider-side mutation requires explicit capabilities + consent prompts.
- **Outbox is idempotent**: every outbound change has a stable idempotency key; retries must not duplicate events.
- **Full observability**: every calendar mutation emits Flight Recorder spans and links back to `job_id`.

```rust
// tests/calendar_law_compliance.rs
// NOTE: implementation will differ, but these assertions are normative.

#[test]
fn test_calendar_rbc_view_only() {
    // Attempting to write calendar state from UI layer must fail.
    // e.g., CalendarViewModel::write_direct(...) => Err(ViewOnlyViolation)
}

#[test]
fn test_calendar_mutations_require_job_and_capability() {
    // Any patch apply must carry job_id + capability profile and pass consent gates.
    // e.g., calendar_sync.apply_patch(patch, ctx_without_job) => Err(MissingJobId)
    // e.g., calendar_sync.apply_patch(patch, ctx_without_cap) => Err(MissingCapability)
}

#[test]
fn test_calendar_outbox_requires_idempotency_key() {
    // Outbox entries must include idempotency_key; replays must be no-ops.
    // e.g., outbox.enqueue(change_without_key) => Err(MissingIdempotencyKey)
    // e.g., outbox.replay_same_key_twice() => single provider mutation
}

#[test]
fn test_calendar_ics_golden_fixtures_roundtrip() {
    // Golden ICS fixtures: parse -> canonicalize -> serialize must be stable.
    // Recurrence rules, timezones, exceptions must not drift across versions.
}

#[test]
fn test_calendar_rrule_property_expansion_is_deterministic() {
    // Property tests: expanding RRULEs over a time window yields deterministic results.
    // Same inputs -> same outputs; ordering stable; no hidden dependence on locale/system time.
}

#[test]
fn test_calendar_provider_sync_simulation() {
    // Simulated provider harness: CalDAV adapter + outbox retries + conflict cases.
    // Ensures convergence, no duplication, and correct conflict surfacing.
}

#[test]
fn test_calendar_mutations_emit_flight_recorder_spans() {
    // Every patch apply and every external sync write must emit trace spans linked to job_id.
}
```


### 5.4.7 Photo Stack Test Suite (Determinism + Golden Fixtures)

This section defines Photo Stack-specific testing requirements used by **Photo Studio (Â§10.10)** and the **Darkroom engine set (Â§6.3.3.6)**.

#### 5.4.7.1 Schema Validation
- All JSON artifacts MUST validate against versioned JSON Schema
- Schema evolution MUST maintain backward compatibility
- Schema files MUST be versioned in repository

#### 5.4.7.2 Golden Fixture Tests
**RAW Development:**
- 10+ camera manufacturers Ã— 3 exposure conditions
- Verify: WB, exposure, tone curve, HSL, detail
- Comparison: SSIM â‰¥ 0.995 for CPU, â‰¥ 0.99 for GPU

**Compositing:**
- All blend modes against Photoshop reference
- Layer masks, clipping, groups
- Comparison: SSIM â‰¥ 0.999 for blend modes

**Merge Operations:**
- HDR: 3/5/7 bracket sets with known output
- Panorama: 3-image, 9-image, 360Â° sets
- Focus: Macro stack reference

**Proxy Workflow:**
- Verify proxy generation matches settings
- Verify AI result scaling accuracy
- Verify metadata preservation

#### 5.4.7.3 Determinism Tests
- Replay test: Re-execute job, compare output hash
- Cross-platform: Same input â†’ same output (within determinism class)
- Version tracking: Engine version changes require revalidation

#### 5.4.7.4 AI Model Tests
- Vision model output consistency (same image â†’ similar tags)
- LLM output quality (manual review of samples)
- ComfyUI workflow reproducibility (with fixed seeds)

### 5.5.1 Benchmark Architecture

#### 5.5.1.1 Why Build a Benchmark Harness?

**Reproducible performance testing** lets you:
- Compare runtimes (Ollama vs vLLM)
- Compare models (Mistral-7B vs Llama2-7B)
- Measure impact of configuration changes
- Track performance over time

#### 5.5.1.2 System Design

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                  BENCHMARK HARNESS                           â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  CONFIG FILES (YAML)                                         â”‚
â”‚  â”œâ”€â”€ models.yml      Model endpoints and settings           â”‚
â”‚  â”œâ”€â”€ scenarios.yml   Test scenarios to run                  â”‚
â”‚  â””â”€â”€ prompts.yml     Standard prompts for testing           â”‚
â”‚                                                              â”‚
â”‚  ADAPTERS                                                    â”‚
â”‚  â”œâ”€â”€ OllamaAdapter   Talks to Ollama                        â”‚
â”‚  â”œâ”€â”€ VLLMAdapter     Talks to vLLM                          â”‚
â”‚  â”œâ”€â”€ TGIAdapter      Talks to TGI                           â”‚
â”‚  â””â”€â”€ ImageAdapter    Talks to ComfyUI                       â”‚
â”‚                                                              â”‚
â”‚  RUNNERS                                                     â”‚
â”‚  â”œâ”€â”€ SingleLLMRunner      One model, one prompt             â”‚
â”‚  â”œâ”€â”€ ConcurrentRunner     Multiple parallel requests        â”‚
â”‚  â””â”€â”€ MixedWorkloadRunner  LLM + Image together              â”‚
â”‚                                                              â”‚
â”‚  OUTPUT                                                      â”‚
â”‚  â”œâ”€â”€ results.jsonl   Raw timing data                        â”‚
â”‚  â””â”€â”€ report.md       Summary statistics                     â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 5.5.2 Scenarios & Adapters

#### 5.5.2.1 Example Configuration

```yaml
#  models.yml
models:
  - id: mistral-7b-ollama
    type: ollama
    endpoint: http://localhost:11434
    model_name: mistral
    
  - id: mistral-7b-vllm
    type: vllm
    endpoint: http://localhost:8000
    model_name: mistralai/Mistral-7B-v0.1

```
#  scenarios.yml
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

```
#  prompts.yml
prompts:
  - id: short_qa
    text: "What is the capital of France?"
    max_tokens: 50
    
  - id: medium_qa
    text: "Explain how photosynthesis works in 3 paragraphs."
    max_tokens: 300
```

#### 5.5.2.2 Adapter Interface

```python
#  adapters.py
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

### 5.5.3 Reporting & Analysis

#### 5.5.3.1 Output Format

```
```
#  Benchmark Report - 2024-01-15

### 5.5.4 Summary

| Scenario       | Model             | Avg Latency | p50    | p95    | Tokens/sec |
|----------------|-------------------|-------------|--------|--------|------------|
| single_chat    | mistral-7b-ollama | 340ms       | 320ms  | 450ms  | 88         |
| single_chat    | mistral-7b-vllm   | 310ms       | 300ms  | 420ms  | 97         |
| concurrent_8   | mistral-7b-vllm   | 180ms       | 170ms  | 250ms  | 620        |

### 5.5.5 Findings

- vLLM is ~10% faster for single requests
- vLLM scales much better under load (620 vs ~100 tokens/sec at 8 concurrent)
- Ollama shows consistent latency regardless of load (no batching)

### 5.5.6 Recommendations

- Use Ollama for development/single-user
- Use vLLM for production/multi-user scenarios
```

---

**Key Takeaways**  
- Benchmark harness enables reproducible comparison of runtimes (Ollama vs vLLM), models, and configurations.
- Architecture: YAML config files (models, scenarios, prompts) + adapters (translate to runtime APIs) + runners (execute tests) + output (raw data + summary report).
- Scenarios include single requests, concurrent load sweeps, and mixed workloads (LLM + image).
- Output reports provide summary tables with latency percentiles and tokens/sec, plus findings and recommendations.
- Key insight from example: vLLM scales ~15Ã— better under load (620 vs 41 tokens/sec at 8 concurrent).

---

---

