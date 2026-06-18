---
file_id: cloth-02-md-feature-map
topic_id: T-MD-FEATURES
title: "Marvelous Designer Full Feature Map"
status: draft
depends_on: []
summary: "Complete taxonomy of Marvelous Designer 2025.x/2026.0 features grouped by domain, with implementation difficulty ratings and moat flags — the requirements baseline for a fully-featured Handshake-native Tailor engine."
sources: 28
updated_at: "2026-06-17"
---

## [T-MD-FEATURES] Marvelous Designer Full Feature Map

This document catalogs the complete Marvelous Designer feature set as of the 2026.0 release, drawing on the 2025.0/2025.1/2025.2/2026.0 release notes, official support documentation, and third-party reviews. It is the requirements baseline that downstream design documents reference when stating which features the Handshake-native Tailor engine must implement, phase, or consciously defer.

**Difficulty key**

| Tag | Meaning |
|-----|---------|
| `[D1]` | Straightforward — well-documented OSS prior art, clear algorithm |
| `[D2]` | Moderate — requires careful implementation but no research gap |
| `[D3]` | Hard — non-trivial algorithm, sparse OSS prior art, or tight integration requirement |
| `[D4]` | Moat-hard — MD's core competitive moat; little or no open-source equivalent at production quality |
| `[MOAT]` | Feature whose combination creates a hard-to-replicate competitive advantage |

---

### [T-MD-FEATURES.group-1-pattern-authoring] Group 1: 2D Pattern Authoring

The 2D pattern editor is the primary authoring surface in MD. Designers draw flat patterns (analogous to real sewing-pattern pieces) and the solver drapes them on the avatar in 3D. Edits made in 2D immediately update the 3D sim and vice versa.

#### Shape creation tools

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Create Rectangle | legacy | `[D1]` | Axis-aligned rect with corner rounding; creates a pattern panel |
| Create Circle | legacy | `[D1]` | Ellipse/circle panel |
| Create Polygon | legacy | `[D1]` | N-sided polygon outline via click-to-place vertices |
| Create Internal Polygon / Line | legacy | `[D2]` | Lines drawn inside an existing panel; creates sub-edges for darts, stress seams, fold lines, baselines, pleat lines |
| Add Point / Split Line | legacy | `[D1]` | Subdivides an existing edge at a clicked point |
| Edit Curvature | legacy | `[D2]` | Converts straight edge segments to cubic Bezier; control-handle drag |
| Edit Curve Point | legacy | `[D2]` | Moves individual Bezier handles for fine curvature control |
| Reverse Seam Line | legacy | `[D1]` | Flips edge direction (affects sewing direction notches) |

#### High-level sketch-to-pattern tools

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| 3D Pen (Avatar) | 2025.0 improved | `[D3]` | Draw curves on avatar surface, in 3D space, or along a plane; converts to editable 2D patterns; requires avatar SDF projection + curve flattening |
| 3D Pencil | 2026.0 | `[D3]` | Draw outlines of pattern parts directly onto or around a 3D avatar and auto-generate the 2D pattern; successor mode to 3D Pen |
| Pattern Drafter (Beta) | 2025.1 | `[D3]` | AI-assisted wizard-driven drafting from text prompts or schematic sketches; currently T-shirt only; requires LLM + pattern template library |

#### Pattern manipulation

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Edit Pattern (select/move points+lines) | legacy | `[D1]` | Standard 2D selection and translate |
| Transform Pattern (scale/rotate whole panel) | legacy | `[D1]` | Affine transform with pivot control |
| Fold Seam Line | legacy | `[D2]` | Interior line marked as a fold rather than a join; controls fold direction and angle |
| Dart | legacy | `[D3]` | Internal converging lines removing material to create 3D form; requires constraint system for tip convergence |
| Pleat Tool | 2025.2 | `[D3]` | Knife, box, and accordion pleat types; parameters: count, depth, interval, fold angle; non-destructive edit without re-sim |
| Lacing Tool | 2026.0 | `[D3]` | Lacing through rows of eyelets; auto-routes cord and generates editable pattern; combines rigid eyelet mesh + soft lace constraint |
| Pattern Grading / 2D Scaling Preserving 3D Shape | 2026.0 | `[D3]` | Scale a 2D pattern for a different character proportion while preserving the draped 3D garment silhouette |
| Pattern Archive | 2026.0 | `[D1]` | Per-pattern versioning/snapshotting for non-destructive iteration |
| Auto Seal | 2025.0 | `[D2]` | Automatically closes open seam ends |

**[MOAT] Bidirectional 2D↔3D edit loop:** Every 2D pattern edit updates the 3D drape in real time; draping feeds accurate edge lengths back to the 2D panel (so a seam edge reports its simulated length). This tight loop is MD's core authoring moat. No OSS cloth tool provides it at production fidelity without external roundtrips. `[D4]`

---

### [T-MD-FEATURES.group-2-sewing] Group 2: Sewing and Seam System

Sewing joins pattern edges by defining constraint pairs. MD's sewing system operates in both 2D (click edges in 2D window) and 3D (click edges in 3D window), and supports ratio-based gathering.

#### 2D sewing tools

| Feature | Difficulty | Notes |
|---------|------------|-------|
| Segment Sewing (2D) | `[D1]` | Click two pre-split edge segments in sequence to create a 1:1 distance-constraint seam |
| Free Sewing (2D) | `[D2]` | Mark arbitrary sub-sections of edges regardless of existing split points; exact seam length settable numerically |
| 1:N Segment Sewing (2D) | `[D3]` | Sew one segment to N segments; rest-length ratio distributes evenly |
| 1:N Free Sewing (2D) | `[D3]` | Ratio sewing with freely designated sub-sections |
| M:N Segment Sewing (2D) | `[D4]` | Sew M segments to N segments; N segments sub-divided proportionally when total lengths differ — models gathering and pleating at constraint level |
| M:N Free Sewing (2D) | `[D4]` | Free-sewing variant of M:N; most general gather/pleat constraint form |

#### 3D sewing tools (mirror of 2D variants)

| Feature | Difficulty | Notes |
|---------|------------|-------|
| 3D Segment Sewing | `[D2]` | Same as 2D Segment but operated from 3D viewport with spatial selection |
| 3D Free Sewing | `[D2]` | Same as 2D Free from 3D viewport |
| 3D 1:N Segment Sewing | `[D3]` | 3D variant |
| 3D 1:N Free Sewing | `[D3]` | 3D variant |
| 3D M:N Segment Sewing | `[D4]` | 3D variant |
| 3D M:N Free Sewing | `[D4]` | 3D variant |

#### Seam management

| Feature | Difficulty | Notes |
|---------|------------|-------|
| Edit Sewing Line Length | `[D2]` | Adjust seam length ratios numerically for gather control |
| Reverse Sewing | `[D1]` | Flip which face of a seam is inside or outside |
| Delete Sewing | `[D1]` | Remove defined seam lines |
| Fold Seam Line | `[D2]` | Mark seam as a fold constraint rather than a join (topstitching-style) |
| Tack | `[D2]` | Point constraints between trim objects and garment panels; animatable since 2025.2 |
| Auto Sewing | 2025.0 `[D3]` | Automatically detects and sews darts and pleats based on pattern placement relative to default avatar body form |

**[MOAT] Ratio sewing (M:N gather):** Per-seam rest-length scaling for arbitrary gather ratios is absent from all major OSS XPBD solvers, which support only 1:1 edge-to-edge distance constraints. Implementing this correctly in a GPU constraint buffer requires per-constraint rest-length metadata. `[D4]`

---

### [T-MD-FEATURES.group-3-fabric-physics] Group 3: Fabric and Material Physical Properties

MD's fabric model is anisotropic: stretch and bending properties are specified per grain direction (weft = horizontal, warp = vertical). This is the primary physical accuracy differentiator versus isotropic OSS cloth solvers.

#### Stretch and shear

| Property | Description | Difficulty |
|----------|-------------|------------|
| Stretch-Weft (%) | Resistance to stretch in the horizontal grain direction | `[D2]` |
| Stretch-Warp (%) | Resistance to stretch in the vertical grain direction | `[D2]` |
| Shear (%) | Resistance to diagonal stretch / crease propensity | `[D3]` |

These map to three independent XPBD stretch constraint compliance values oriented to the fabric grain matrix.

#### Bending

| Property | Description | Difficulty |
|----------|-------------|------------|
| Bending-Weft | Stiffness against bending in horizontal grain; high = denim/leather, low = silk | `[D2]` |
| Bending-Warp | Stiffness against bending in vertical grain | `[D2]` |
| Buckling Ratio (%) | Ratio of bending stiffness at buckling corners; controls wrinkle frequency; ~100% → easily foldable (silk/jersey) | `[D3]` |
| Buckling Stiffness | Absolute stiffness at the buckling corner; controls crispness vs. soft fold | `[D3]` |

True anisotropic bending requires separate dihedral-angle constraints for weft and warp grain axes. Most OSS XPBD cloth uses a single isotropic bending stiffness.

#### Dynamics

| Property | Description | Difficulty |
|----------|-------------|------------|
| Internal Damping | Velocity damping per particle; damps vibration/jitter | `[D1]` |
| Density (g/m²) | Cloth mass per unit area driving inertia and gravity response | `[D1]` |

#### Collision properties

| Property | Description | Difficulty |
|----------|-------------|------------|
| Collision Thickness (mm) | Gap maintained between cloth and collision objects; default ~2.5 mm | `[D2]` |
| Friction Coefficient | Cloth-avatar and cloth-cloth friction; prevents garment slide-off during animation | `[D2]` |

#### Simulation-time fabric properties (keyframeable since 2025.2)

| Property | Description | Difficulty |
|----------|-------------|------------|
| Pressure | Inflatable-object mode; keyframeable pressure scalar | `[D3]` |
| Solidify | Stiffness blend between rigid and soft; animatable stiffening | `[D3]` |
| Shrinkage-Weft | Horizontal shrinkage fraction; keyframeable since 2025.2 | `[D3]` |
| Shrinkage-Warp | Vertical shrinkage fraction; keyframeable since 2025.2 | `[D3]` |
| Trim Weight | Rigid trim mass contribution; keyframeable since 2025.2 | `[D3]` |
| Tack Strength | Point-constraint attachment force between trim and garment; keyframeable since 2025.2 | `[D3]` |

#### Preset library

| Feature | Difficulty | Notes |
|---------|------------|-------|
| Fabric preset library (cotton, denim, silk, jersey, leather, satin, etc.) | `[D2]` | Named property bundles; per-pattern override for independent properties per panel |
| Washed denim presets (stonewashed, acid washed, bleach washed) | 2025.1 `[D2]` | Texture + parametric controls |
| Print on Fabric | 2025.1 `[D1]` | Printed pattern application to fabric material |
| PBR Map Generator | 2024.1+ `[D2]` | Auto-generate Normal, Opacity, Roughness, Displacement, Metalness maps from base color texture |

**[MOAT] Anisotropic PBW property model:** Weft/warp/shear split with separate per-axis stretch and bending parameters is not present in any major OSS XPBD solver (ccincotti3, jspdown, xpbdrs all use isotropic constraints). Implementing correctly requires a 2×2 anisotropic compliance tensor per constraint. `[D4]`

**[MOAT] Keyframeable physical properties:** Time-varying stiffness, pressure, shrinkage during simulation requires per-substep GPU uniform upload. No OSS GPU cloth solver exposes this. `[D4]`

---

### [T-MD-FEATURES.group-4-simulation-engine] Group 4: Simulation Engine

MD's solver is proprietary and not publicly documented as PBD vs XPBD vs FEM. Its behavior is consistent with a modified XPBD core plus post-processing strain limiting. GPU acceleration via NVIDIA CUDA is the major performance path.

#### Core solver parameters

| Feature | Difficulty | Notes |
|---------|------------|-------|
| Particle Distance (mm) | `[D2]` | Sets triangle edge length / mesh resolution; range ~0.8 mm (ultra-high) to 700 mm (draft); lower = higher fidelity, higher cost |
| Substep count | `[D2]` | Number of simulation sub-steps per frame; more substeps = more accurate collision handling at higher CPU/GPU cost |
| Iteration count | `[D2]` | Constraint solver iterations per substep; more iterations = stiffer constraints |
| Fitting mode vs Animation mode | `[D2]` | Fitting (Accurate Fabric) mode: higher physical accuracy; Animation (Stable) mode: sacrifices some accuracy for temporal stability |

#### Acceleration

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| GPU Simulation (NVIDIA CUDA) | 2024.0+ | `[D3]` | Major cloth solver acceleration; GPU collision accuracy reached CPU parity in 2024.2 for cloth panels |
| GPU-Accelerated Trim Simulation | 2026.0 | `[D4]` | Extends GPU path to rigid trim/accessory simulation (was CPU-only in 2025.x); buttons/buckles now GPU-accelerated |
| CPU fallback | legacy | `[D1]` | Always available; required for trim collision in 2025.x; still needed for edge cases |

#### Self-collision

| Feature | Difficulty | Notes |
|---------|------------|-------|
| Self-collision (sphere-based) | `[D3]` | Disabled by default; enables cloth-cloth contact for multi-layer garments; sphere proxy approach |
| Precise self-collision | `[D4]` | Higher-accuracy mode via additional solver iterations; expensive; required for tight garment stacking |

#### Scene dynamics

| Feature | Difficulty | Notes |
|---------|------------|-------|
| Gravity (direction + magnitude) | `[D1]` | Global scene property; configurable axis |
| Wind (position, angle, strength) | `[D2]` | Positioned wind source; keyframeable in animation mode; real-time adjustment during recording |
| Avatar friction | `[D2]` | Per-avatar friction setting to prevent garment sliding during animation |
| Soft-body simulation | 2025.1 extended `[D3]` | Cloth physics applied to arbitrary mesh volumes; extended to custom imported OBJ/FBX avatars and 3D props in 2025.1 |

**[MOAT] GPU collision accuracy parity:** Reaching CPU-quality collision handling for complex multi-layer garments on the GPU is a hard engineering problem. As of 2024.2 MD achieved parity; open-source GPU XPBD solvers still compromise on collision quality or handling of deep interpenetrations. `[D4]`

---

### [T-MD-FEATURES.group-5-avatar-body-fitting] Group 5: Avatar System and Body Fitting

MD's avatar system provides parameterized body models and supports custom avatar import, critical for producing garments that fit specific characters.

#### Default avatar system

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Parameterized avatar library | legacy | `[D2]` | Multiple base body types with anthropometric measurement sliders (height, chest, waist, hip, shoulder, inseam, etc.) driven by real body-scan data |
| Avatar Editor (improved 2025.0) | 2025.0 | `[D2]` | More realistic avatar sizes; manual joint editing via IK Mode |
| IK Joint Mapping | 2025.0 | `[D3]` | Automatic conversion of imported characters (Daz 3D, Mixamo, Character Creator, MetaHuman) to MD format; IK-based pose editing |
| IK Mode for Avatar Joint | 2025.0 | `[D2]` | Natural pose editing by dragging end-effector joints; IK solver drives parent chain |

#### Custom avatar import

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Custom OBJ/FBX avatar import | legacy | `[D1]` | Rigged or unrigged meshes as collision targets or avatars |
| Blend Shape Avatar | 2026.0 | `[D3]` | Import avatars with blend shape / morph target data; real-time body shape changes drive cloth re-fit |
| OBJ Morphing (AVT files) | legacy | `[D2]` | Change avatar pose or shape via morph target AVT files |
| MetaHuman DNA import | 2025.2 | `[D3]` | Direct import of Epic Games MetaHumans in native .dna format; preserves proportions, mesh fidelity, rigging; garments auto-resize on export back to MetaHuman Creator |
| glTF / VRM avatar import | 2026.0 | `[D2]` | Import avatars in glTF or VRM format |

#### Fitting tools

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Auto-Fitting (improved 2025.0) | 2025.0 | `[D3]` | Automatically refit a garment from one avatar body to another; Preserve Topology option added 2025.1 |
| Garment Fit Properties | 2025.0 | `[D2]` | Per-garment fitting behavior controlling tightness of wrap on specific avatar; visualize Stress, Strain, Pressure on garment when fitted |
| Fit Map Display on 2D Patterns | 2026.0 | `[D2]` | Display garment fit maps (stress/strain/pressure) projected back onto the 2D pattern panels |
| Fabric-Aware Strain Maps | 2026.0 | `[D3]` | Strain visualization that accounts for per-fabric anisotropic properties |
| Sculpt Tool | 2025.0 | `[D3]` | Manual mesh sculpting of draped garment surface |

#### AI-assisted body tools

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| AI Pose Generator (Beta) | 2025.1 | `[D4]` | VLM-powered pose generation from keyword or reference image; poses avatar automatically; requires cloud inference |
| Auto Convert to Motion | 2025.0 | `[D2]` | Automatically converts imported FBX animations to MD's MTN format |

---

### [T-MD-FEATURES.group-6-animation-dynamics] Group 6: Animation and Dynamics

MD's animation system evolved from a recording-only playback system to a full keyframe timeline with animated fabric properties as of 2025.0-2025.2.

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Keyframe Animation system | 2025.0 | `[D2]` | Timeline-based keyframe authoring for avatar pose, wind, and now fabric properties |
| Keyframeable fabric properties | 2025.2 | `[D4]` | Shrinkage Weft/Warp, Solidify, Pressure, Tack Strength, Trim Weight all animatable on timeline |
| Tack constraint animation | 2025.2 | `[D3]` | Tack constraints between trims and garments can be keyframed to animate stitching/unstitching |
| Animation Timeline Markers | 2026.0 | `[D1]` | Named markers on timeline for navigation and organization |
| Animation recording | legacy | `[D2]` | Playback-and-record with real-time wind/pose adjustment during the take |
| Morph animation | 2026.0 extended | `[D2]` | Alembic import/export carries morph/blend shape channel animation |
| Trim/Accessory simulation during animation | 2025.0 | `[D3]` | Rigid trims simulate alongside cloth during animation playback |
| Wind keyframing | 2025.0 | `[D2]` | Wind position, angle, and strength keyframeable |
| Animation Range Export | 2026.0 | `[D1]` | Export specific frame ranges to FBX or glTF |
| FBX Joint Animation auto-keys | 2026.0 | `[D1]` | Automatically generates keyframes on empty frames during FBX joint-animation export |

**[MOAT] Keyframeable physical properties during simulation:** Time-varying fabric stiffness, pressure, shrinkage within a single simulation run requires per-substep parameter upload to the GPU. This is absent from all OSS GPU cloth solvers and enables animated inflatables, dynamic stiffening, and wash-simulation effects. `[D4]`

---

### [T-MD-FEATURES.group-7-trims-accessories] Group 7: Trims, Accessories, and Hardware

Trims are rigid or semi-rigid 3D objects (buttons, buckles, zippers, rivets) attached to garment patterns. In 2025.2 the pattern-to-trim conversion extended this to armor and hard-surface costume authoring.

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Trim import (OBJ/FBX) | legacy | `[D1]` | Import arbitrary 3D meshes as rigid or semi-rigid overlays |
| Glue tool | legacy | `[D2]` | Click-to-place trims at any position/angle on garment surface; gizmo for precise placement |
| Tack tool | legacy | `[D2]` | Apply trim to multiple attachment points across pattern pieces as point constraints |
| Stiffness control for trim | legacy | `[D2]` | Active-state trim stiffness parameter |
| Trim Simulation with Collision (CPU) | 2024.2 | `[D3]` | Trims physically collide with garment and body mesh during simulation; CPU-only in 2025.x |
| GPU-Accelerated Trim Simulation | 2026.0 | `[D4]` | Buttons, buckles, accessories now GPU-accelerated; required for trim collision at scale |
| Pattern-to-Trim conversion | 2025.2 | `[D4]` | Convert cloth pattern pieces into Trims or Accessories (rigid bodies); enables armor and hard-surface objects authored inside MD within the same session |
| Two-Way Zippers | 2025.0 | `[D3]` | Bidirectional zipper construction with OBJ teeth and preset |
| Default trim library | legacy | `[D1]` | Buttons, zippers, rivets, hardware, fasteners, notions |
| Register Accessory | 2025.0 | `[D2]` | Register imported objects as reusable accessories in the project library |
| Isolate Selection | 2026.0 | `[D1]` | Display only selected objects in viewport for inspection of individual garments in complex scenes |

**[MOAT] Pattern-to-rigid-body conversion:** Converting a draped cloth panel into a simulation-coupled rigid trim inline, within the same session and solver state, is unique to MD. OSS cloth pipelines require external DCC roundtrips. `[D4]`

**[MOAT] Mixed cloth-rigid simulation graph:** Cloth particles colliding with rigid trim meshes (which are themselves simulated) in the same solver pass requires a mixed rigid+soft constraint graph. All OSS GPU XPBD solvers are cloth-only; they use only static collision proxies. `[D4]`

---

### [T-MD-FEATURES.group-8-uv-texturing-rendering] Group 8: UV, Texturing, and Rendering

MD's UV layout is physically meaningful: UV islands are the exact flattened 2D pattern pieces, preserving fabric grain direction automatically.

#### UV system

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| UV Editor mode | legacy | `[D2]` | UV layout view synced with 2D pattern editor; each pattern piece auto-places as its own UV island matching the 2D shape |
| Automatic UV Packing | legacy | `[D2]` | Packs pattern UV islands without manual rearrangement |
| UV Editor Filtering for Selected Garments | 2026.0 | `[D1]` | Isolate UV view to selected garment only |
| Side and Back UV Expansion | 2025.0 | `[D2]` | Auto-generates UVs for garment sides and back, not just front panels |
| Grain Direction | legacy | `[D2]` | Per-pattern grainline angle entry; rotates texture image with grain |
| UV Bake Textures | legacy | `[D2]` | Export UV islands as Diffuse, Normal, Opacity maps |
| Edit Texture 2D | legacy | `[D2]` | Surface editor for placing graphic layers on 2D pattern pieces |

#### Material system

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Fabric texture + graphic layer | legacy | `[D1]` | Texture and graphic decal per pattern; transfers to 3D UV correctly |
| PBR material (Metallic, Roughness, Emissive, Normal, Opacity) | legacy | `[D2]` | Per-fabric PBR channels |
| PBR Map Generator | 2024.1 | `[D2]` | Auto-generates Displacement, Opacity, Roughness, Metalness maps from base color |
| Blend Graphic with Fabric | 2025.0 | `[D1]` | Blend texture graphic with underlying fabric color |
| Recolor Base Color Map on Export | 2025.0 | `[D1]` | Recolor base map during PBR export |
| Fur Strand Material (experimental) | 2025.0 | `[D4]` | Particle-based fur/hair strands on garment; follows fabric movement; now respects seams/graphics (2026.0) |
| Toon Shader (Express Cartoon) | 2026.0 | `[D2]` | Shading bands, outline, rim lighting, opacity, emissive, MatCap support; stylized viewport rendering |
| MatCap material | 2026.0 | `[D1]` | MatCap material mode |
| Schematic Render | 2025.0 | `[D2]` | Technical-drawing viewport render mode |

#### All-quad mesh

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| All-Quad Mesh Conversion | 2025.0 | `[D3]` | Auto-conversion of simulated triangular mesh to all-quad topology for downstream DCC tools and subdivision |
| Polygon Optimization for Topstitch | 2025.0 | `[D2]` | Reduces polygon count around topstitch geometry |
| Smooth Corner for Topstitch | 2025.0 | `[D2]` | Beveled corner rendering on topstitch paths |

**[MOAT] UV-from-pattern accuracy:** MD's UV islands are exact flattened 2D pattern pieces. Fabric texture grain direction is always physically accurate because the UV matches the sewing pattern, not an unwrapped 3D mesh. Replicating this requires an unfurl/flatten post-simulation pass that no other tool performs automatically. `[D4]`

---

### [T-MD-FEATURES.group-9-garment-library-assets] Group 9: Garment Library and Asset Management

MD's library system evolved in 2025.0 to a modular, block-based reusable component system, integrated with the CLO-SET cloud platform.

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| Modular Library | 2025.0 | `[D2]` | Save garment groups and block sets as reusable modular building blocks; mix-and-match style variations; categories: Group, Category, Style, Block |
| Block system | 2025.0 | `[D3]` | A Block = patterns sewn to Block Boxes; blocks with matching Block Box types are interchangeable; constraints-as-interfaces |
| New Library Window | 2025.0 | `[D2]` | Integrated with CLO-SET collaboration platform and CONNECT asset marketplace; browse local + marketplace assets in one UI |
| User-defined preset fabric files | legacy | `[D1]` | Save/load physical property sets as named presets |
| Import and Export Preset system | legacy | `[D1]` | Save/load import/export parameter sets for repeatable pipeline configurations |
| Object Browser | 2025.0 | `[D1]` | Hierarchical scene object tree for complex multi-garment scenes |
| Pattern Archive | 2026.0 | `[D1]` | Per-pattern snapshot/archiving for non-destructive iteration |
| Garment file format (.zpac/.zprj) | legacy | `[D1]` | Project container with all pattern, sewing, simulation, material, avatar data |

**[MOAT] Modular garment library with CLO-SET marketplace:** The combination of a local modular block library and a cloud marketplace where simulation parameters (fabric presets, garment blocks) are pre-embedded is a proprietary network-effect moat. There is no OSS equivalent. `[D4]`

---

### [T-MD-FEATURES.group-10-import-export-interop] Group 10: Import / Export and Pipeline Interoperability

MD targets VFX, games, AR, and fashion production simultaneously, requiring a wide format matrix.

#### 3D geometry formats

| Format | Direction | Version | Difficulty | Notes |
|--------|-----------|---------|------------|-------|
| OBJ + .mtl | in/out | legacy | `[D1]` | With material sidecar; avatar and prop import; garment export with UVs |
| FBX | in/out | legacy | `[D2]` | Material names synced; joint animation in/out; auto-key on empty frames (2026.0); EveryWear-optimized local FBX export (2025.2) |
| Alembic (.abc) | in/out | legacy | `[D2]` | Full animation cache; morph animation support (2026.0); export of specific animation ranges |
| USD / USDZ | in/out | 2025.0+ | `[D3]` | USD export; USDZ for AR apps; USD to Unreal Engine Chaos Cloth workflow |
| glTF / GLB | in/out | 2026.0 | `[D2]` | Full garment + avatar export in glTF 2.0; animation range export |
| VRM | in | 2026.0 | `[D2]` | VRM avatar import (VTuber/virtual character format) |
| MDD / PC2 | out | legacy | `[D1]` | Simulation cache formats for Maya/Houdini pipelines |
| Maya cache | out | legacy | `[D1]` | Maya fluid/geometry cache for deformable cloth in Maya |
| DXF | out | legacy | `[D1]` | 2D sewing pattern export for physical production |
| OBJ sequence | out | legacy | `[D1]` | Per-frame OBJ export for simulation cache |

#### MetaHuman pipeline

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| MetaHuman .dna native import | 2025.2 | `[D3]` | Direct import without intermediate conversion; preserves proportions, mesh fidelity, rigging |
| Export to MetaHuman Creator with auto-resize | 2025.2 | `[D3]` | Garments adapt automatically to body adjustments in MetaHuman Creator |
| USD MetaHuman Garment Integration Workflow | 2025.0+ | `[D3]` | Documented USD path for Marvelous Designer → MetaHuman → Unreal Engine |

#### Real-time engine integration

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| LiveSync (Unreal Engine) | legacy/ongoing | `[D4]` | Live link to Unreal Engine; one-click mesh/material/animation sync; changes in MD visible in UE in real-time; eliminates manual export |
| USD → Chaos Cloth workflow | 2025.0+ | `[D4]` | Physical properties (shrinkage, collision) readable by UE Chaos Cloth; simulation state exported for real-time re-simulation in UE |
| EveryWear system | legacy/ongoing | `[D4]` | Automated garment-to-game pipeline: auto-fits, rigs, bakes LODs, creates textures; Rig Template system (2026.0 experimental) automates joint creation for wide garments; blendshape support (2026.0) |
| EveryWear local FBX export | 2025.2 | `[D2]` | Optimized FBX directly to local machine after garment prep, rigging, UV, texture baking |

**[MOAT] EveryWear + LiveSync pipeline:** Fully automated cloth-to-game pipeline with Chaos Cloth property preservation requires knowledge of the internal solver state. No OSS cloth pipeline provides equivalent game-engine handoff quality. `[D4]`

---

### [T-MD-FEATURES.group-11-ai-model-steerability] Group 11: AI / Model-Steerability Features

This group is the primary Handshake differentiator. As of 2026.0, MD's in-product AI is limited to pose generation and texture generation. LLM-steerable pattern authoring inside the tool does not exist. The academic community has built the enabling infrastructure independently.

#### MD's current in-product AI

| Feature | Version | Difficulty | Notes |
|---------|---------|------------|-------|
| AI Pose Generator (Beta) | 2025.1 | `[D4]` | VLM-powered avatar pose from text keyword or reference photo; processing occurs online; not local inference |
| Pattern Drafter (Beta) | 2025.1 | `[D3]` | Wizard-driven pattern drafting from text or schematic; T-shirt only; AI-assisted draft iteration with history |
| AI Image Generator | 2025+ | `[D2]` | Separate plugin; generate texture/fabric images via AI; image applied to garment material |

#### External AI garment research (current 2025-2026 landscape)

These projects supply the algorithmic basis for the Handshake model-first garment authoring pipeline:

| Project | Origin | Approach | Handshake relevance |
|---------|--------|----------|---------------------|
| GarmentCode | ETH Zurich (MIT-adjacent) | Parametric sewing-pattern DSL; JSON-based; Edge/Panel/Component/Interface primitives; Python; GitHub: `maria-korosteleva/GarmentCode` | Reference schema for garment authority JSON; round-trip interop with ChatGarment-style tools |
| ChatGarment | CVPR 2025, MPI | Fine-tunes LLaVA VLM to output GarmentCode JSON from text or image; three dialog modes (estimate/generate/edit); 350-token compressed GarmentCode representation; site: chatgarment.github.io | Direct model for Handshake's LLM-steerable garment authoring via ModelAdapter |
| GarmentDiffusion | IJCAI 2025, open-source | Diffusion transformer on edge-oriented tokens; 100× faster than SewingGPT; text+image+incomplete-pattern input; GitHub: `Shenfu-Research/GarmentDiffusion` | Fast generation backbone alternative to ChatGarment for sketch-to-pattern path |
| Design2GarmentCode | CVPR 2025, Style3D | LMM generates GarmentCode pattern-making programs from images/text/sketches; program synthesis approach | Validates that LMM→structured JSON is production-ready for garment authoring |
| NGL-Prompter | MPI, Feb 2026 | Training-free; Natural Garment Language restructures GarmentCode for VLM legibility; deterministic mapping to valid GarmentCode; handles multi-layer outfits; arXiv: 2602.20700 | No-training LLM prompting path for Handshake garment authoring; zero finetuning cost |
| Dress-1-to-3 | 2025 | Single image → separated garments with sewing patterns; differentiable XPBD for pattern refinement via gradient descent; arXiv: 2502.03449 | Image-based garment capture feeding the authoring pipeline |
| AIpparel | 2025, ResearchGate | Multimodal foundation model for digital garments | Emerging foundation model approach; track for future integration |
| Garment Particles | arXiv 2605.26391, 2026 | 2D-3D symmetric garment representation for generation and editing | Emerging symmetric representation that may inform the 2D↔3D authority schema |
| Bolt (NVIDIA, Apr 2025) | NVIDIA Research | Automated garment transfer, draping, rigging at scale; XPBD-based draping; optimizes 2D patterns for new body proportions; output = rigged garment meshes; arXiv: 2504.17614 | Solver pipeline architecture reference; auto-rigging output target |

**[MOAT] LLM/VLM steerability gap in MD:** As of 2026.0, Marvelous Designer has no LLM-steerable sewing pattern authoring or simulation parameter estimation. This is the Tailor engine's key differentiator: owning ChatGarment-style JSON-to-pattern generation + parameter estimation via ModelAdapter + sandbox/validation/promotion gate. `[D4]`

---

### [T-MD-FEATURES.moat-summary] Moat Feature Summary

The eight MD moat features that define "fully featured" and drive the hardest parts of the Handshake design:

| ID | Moat Feature | MD Group | Difficulty | Handshake approach |
|----|-------------|----------|------------|-------------------|
| MOAT-1 | Bidirectional 2D↔3D edit loop | Group 1 | `[D4]` | CRDT-tracked panel geometry; solver reports back edge lengths to authority; 2D view is a projection |
| MOAT-2 | Ratio sewing (M:N gather/pleat) | Group 2 | `[D4]` | Per-seam rest-length scaling in WGSL constraint buffer; constraint stored in PostgreSQL seam rows |
| MOAT-3 | Anisotropic fabric model (weft/warp/shear) | Group 3 | `[D4]` | 2×2 compliance tensor per constraint pair in XPBD solver; fabric params stored in authority JSON |
| MOAT-4 | Keyframeable physical properties | Groups 3 & 6 | `[D4]` | Per-substep GPU uniform upload in WGSL pipeline; keyframes stored in EventLedger receipts |
| MOAT-5 | Mixed cloth-rigid simulation (trim collision) | Group 7 | `[D4]` | Mixed constraint graph: XPBD cloth + Rapier/parry rigid proxies for trims in same solver pass |
| MOAT-6 | UV-from-pattern accuracy | Group 8 | `[D4]` | Post-simulation unfurl/flatten pass to generate UV islands matching 2D pattern geometry |
| MOAT-7 | Automated game-engine rigging (EveryWear) | Group 10 | `[D4]` | Solver-state-aware skinning weight baking; deferred to post-v1; requires simulation state access |
| MOAT-8 | LLM steerability (Handshake differentiator) | Group 11 | `[D4]` | ChatGarment-style JSON-to-pattern via ModelAdapter; sandbox validation; PromotionGate authority |

**Phase recommendation:** MOAT-1 through MOAT-6 are required for a solver/authoring tool that matches MD production capability. MOAT-7 (EveryWear-equivalent automated game rigging) is a post-v1 stretch goal. MOAT-8 is Handshake's unique differentiator and should be designed in from day one, even if the LLM authoring path ships before the manual authoring UI.

---

### [T-MD-FEATURES.difficulty-distribution] Difficulty Distribution Summary

| Difficulty | Feature count | Notes |
|-----------|--------------|-------|
| `[D1]` | ~38 | Straightforward to implement; no algorithmic risk |
| `[D2]` | ~42 | Moderate; clear algorithm, requires careful implementation |
| `[D3]` | ~31 | Hard; sparse OSS prior art or tight integration requirement |
| `[D4]` | ~18 | Moat-hard; competitive differentiators; highest engineering risk |

Total catalogued features: ~129 across 11 groups.

The D4 features are concentrated in: anisotropic physics, GPU acceleration paths, the M:N sewing system, keyframeable properties, the trim collision system, UV-from-pattern, and LLM steerability. These are the design decisions that differentiate a production cloth engine from a demo solver.

---

### [T-MD-FEATURES.sources] Sources

1. https://support.marvelousdesigner.com/hc/en-us/articles/55837641308313-Marvelous-Designer-2026-0-New-Feature-List
2. https://support.marvelousdesigner.com/hc/en-us/articles/47358120307353-Marvelous-Designer-2025-0-2025-1-2025-2-New-Feature-List
3. https://www.cgchannel.com/2026/04/clo-virtual-fashion-releases-marvelous-designer-2026-0/
4. https://www.cgchannel.com/2025/04/clo-virtual-fashion-releases-marvelous-designer-2025-0/
5. https://www.cgchannel.com/2025/08/clo-virtual-fashion-releases-marvelous-designer-2025-1/
6. https://www.cgchannel.com/2025/11/clo-virtual-fashion-releases-marvelous-designer-2025-2/
7. https://digitalproduction.com/2026/04/15/marvelous-designer-2026-0-adds-3d-pencil-and-lacing/
8. https://digitalproduction.com/2025/11/18/marvelous-designer-2025-2-metahumans-pleats-keyframes/
9. https://digitalproduction.com/2025/08/21/marvelous-designer-2025-1-draw-wash-repeat/
10. https://digitalproduction.com/2025/05/01/marvelous-2025-now-with-extra-fur-fewer-triangles/
11. https://cgpress.org/archives/marvelous-designer-2025-2-introduces-improved-animation-controls-trim-conversion-fbx-export-metahuman-support-and-a-new-pleat-tool.html
12. https://support.marvelousdesigner.com/hc/en-us/articles/47358125463321-Simulation-Properties
13. https://support.marvelousdesigner.com/hc/en-us/articles/47358268602777-Particle-Distance-Setting
14. https://support.marvelousdesigner.com/hc/en-us/articles/47358420908313-FABRIC-PHYSICAL-PROPERTIES-Adjust-Buckling-Ratio
15. https://support.marvelousdesigner.com/hc/en-us/articles/47358443144985-FABRIC-PHYSICAL-PROPERTIES-Adjust-Buckling-Stiffness
16. https://support.marvelousdesigner.com/hc/en-us/articles/47358430412441-FABRIC-PHYSICAL-PROPERTIES-Adjust-Collision-Thickness
17. https://support.marvelousdesigner.com/hc/en-us/articles/47358460243737-FABRIC-PHYSICAL-PROPERTIES-Adjust-Density-Weight-Ver-2024-1
18. https://support.marvelousdesigner.com/hc/en-us/articles/47358444242457-FABRIC-PHYSICAL-PROPERTIES-Adjust-Friction-Coefficient
19. https://support.marvelousdesigner.com/hc/en-us/articles/47358199517209-Segment-Sewing-2D
20. https://support.marvelousdesigner.com/hc/en-us/articles/47358146261913-Trims
21. https://support.marvelousdesigner.com/hc/en-us/articles/47358198817689--Mode-UV-EDITOR
22. https://support.marvelousdesigner.com/hc/en-us/articles/47358157799961-Modular-Library-Ver-2025-0
23. https://support.marvelousdesigner.com/hc/en-us/articles/48920289920537-AI-Pose-Generator-Beta
24. https://support.marvelousdesigner.com/hc/en-us/articles/48920347302297-Pattern-Drafter-Beta
25. https://chatgarment.github.io/
26. https://github.com/maria-korosteleva/GarmentCode
27. https://arxiv.org/abs/2602.20700
28. https://support.marvelousdesigner.com/hc/en-us/articles/51752244831897-MetaHuman-DNA-Importer
