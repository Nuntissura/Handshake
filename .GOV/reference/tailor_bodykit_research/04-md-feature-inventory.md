---
file_id: tailor-bodykit-research-04-md-feature-inventory
topic_id: T-BK-MD-INVENTORY
title: "Independent Exhaustive Marvelous Designer Feature Inventory (MD 10 → 2026.0, full manual corpus)"
status: non_normative_research
normative_status: non_normative_context_only
research_lane: "web-research sub-agent, session 2026-07-08. Mined the full MD Zendesk help-center corpus (709 articles, 52 sections, via API) + all official New Feature Lists MD 10→2026.0 + developer.marvelousdesigner.com + cgchannel release coverage."
purpose: "Independent external parity checklist for the second-pass WP-KERNEL-010 Cloth gap analysis (the first parity pass audited only against the repo-internal 02-md-feature-map.md)."
key_negative_facts: "VERIFIED CLO-only (NOT in MD): DXF-AAMA/ASTM import/export; grading/auto-grading; seam-allowance tool; notch tool; pattern annotation; colorways; offline renderer (V-Ray); print layout; down/padding fill; custom fabric measurement kit."
updated_at: "2026-07-08"
---

# Marvelous Designer Exhaustive Feature Inventory (independent second-pass source)

## Area 1: 2D Pattern Design

| Feature | What it does (1 line) | MD version introduced | Class | Notes |
|---|---|---|---|---|
| Polygon tool | Draw freeform pattern outlines point-by-point (click=line, drag/curve for curves) | legacy | Authoring | Core drafting tool |
| Rectangle tool | Create rectangular pattern with drag or numeric W/H dialog | legacy | Authoring | |
| Ellipse/Circle tool | Create elliptical/circular patterns | legacy | Authoring | |
| Spiral tool | Create spiral patterns (for flounces/ruffles) | 12 | Authoring | Obscure; key for ruffles |
| Internal Polygon/Line | Draw internal lines/shapes inside a pattern | legacy | Authoring | Internal lines drive folds, sewing targets |
| Internal Rectangle | Internal rectangle shape inside pattern | legacy | Authoring | |
| Internal Ellipse | Internal ellipse shape inside pattern | legacy | Authoring | |
| Dart tool | Create darts by drag or numeric dialog (W left/right, H up/down, position to outline/center) | legacy (dialog v2.5.0) | Authoring | |
| Segment Darts | Create darts on a pattern outline segment | legacy | Authoring | Distinct from interior Dart tool |
| Convert to Hole/Internal Shape | Toggle internal shape between cut-out hole and shape | legacy | Authoring | Holes = vents, lacing eyelets |
| Create Multiple Patterns/Internal Shapes with Intervals | Array-duplicate patterns/shapes with numeric spacing | legacy | Authoring | Obscure batch tool |
| Create Smooth Curve | Draw smooth freehand curve; select desired point (2026.0 improvement) | legacy / 2026.0 | Authoring | |
| Create AI Bezier Curve | Illustrator-style bezier drawing mode for outlines/internal lines | v3.0.0 | Authoring | Two curve systems coexist (MD curve points vs bezier) |
| Edit AI Bezier Curve | Edit bezier handles | v3.0.0 | Authoring | |
| Convert to Curve Point (AI Bezier) | Convert MD curve points to bezier | legacy | Authoring | |
| Convert Curve↔Segment Points | Convert point types both ways (improved 2024.2) | legacy | Authoring | |
| Edit Curvature | Drag whole segment into a curve | legacy | Authoring | |
| Edit Curve Point | Move individual curve points (Ctrl-edit MD10) | legacy | Authoring | |
| Set Editing Range for Curves | Adjust curvature of a bounded sub-range of a line | 2025.0 | Authoring | Obscure |
| Delete Curve Point / Delete All Overlapping Points | Point cleanup operations | legacy | Authoring | |
| Optimize Curve Points | Reduce curve point count (all-pattern dialog 12.2) | MD10 | Authoring | Critical for pattern hygiene |
| Add Point/Split Line | Insert point / split segment (numeric ratio/length) | legacy | Authoring | |
| Add Point to Intersection | Add points where lines intersect | legacy | Authoring | Obscure |
| Add Perpendicular Internal Line | Drop perpendicular internal line from a point | legacy | Authoring | Obscure |
| Extend/Trim & Add Point | Extend or trim internal line to outline/another internal line | MD10 | Authoring | CAD-style trim |
| Divide Internal Lines | Split internal lines into N parts | legacy | Authoring | |
| Merge Points on Internal Lines | Merge coincident internal points | legacy | Authoring | |
| Select/Move Point/Segment; Transform Point/Segment | Point/segment-level editing with numeric transforms | legacy | Authoring | |
| Select/Move/Transform Pattern | Pattern-level move/rotate/scale | legacy | Authoring | |
| Preserve 3D Garment Shape on 2D Scaling | 2D scale without destroying draped 3D state | 2026.0 | Core-sim | Workflow-critical |
| Polygon Lasso Selection | Lasso select in 2D | legacy | Authoring | |
| Efficient partial selection (Ctrl+drag) | Partial marquee pattern selection | 2024.0 | Authoring | |
| Invert Selection | Invert 2D selection | legacy | Authoring | |
| Align / Align Points / Distribute / Order | Alignment, distribution, z-order of patterns | legacy | Authoring | |
| Change Length | Set exact segment length (lock curve points option MD10) | legacy | Authoring | |
| Rotate / Flip Horizontally/Vertically | Pattern rotate/flip ops | legacy | Authoring | |
| Cut / Cut & Sew | Split pattern along internal line, optionally auto-sew the cut | legacy | Authoring | Workflow-critical |
| Merge patterns | Merge two patterns across a shared seam | legacy | Authoring | |
| Merge Symmetric Patterns | Merge mirrored halves into symmetric pattern | legacy | Authoring | |
| Unfold | Reflect half-pattern across a fiducial segment into full pattern | legacy | Authoring | |
| Unfold Symmetric Editing (with Sewing) | Half-symmetry editing where mirror half stays linked incl. sewing | v5.1.0 | Authoring | "Sewing symmetry" |
| Adjust Center Line on Half Symmetry | Move symmetry axis of half pattern | MD10 | Authoring | Obscure |
| Unlink Internal Line Symmetry | Break symmetry link per internal line | 2026.0 | Authoring | Obscure |
| Clone as Pattern/Internal Shape | Duplicate pattern or demote/promote to internal shape | legacy | Authoring | |
| Clone as Pattern (with Sewing) | Clone keeps sewing relations | 12.2 | Authoring | |
| Layer Clone (Over/Under) | Instance pattern sewn edge-to-edge above/below original (linings, padding) | legacy | Authoring | Workflow-critical for padded garments |
| Linked Editing | Symmetric/instance clones edited simultaneously (with-sewing variant) | v3.0.0 | Authoring | |
| Mirror Paste | Paste mirrored copy | legacy | Authoring | |
| Copy a Part of Internal Shape | Copy sub-portion of internal shape | MD10 | Authoring | Obscure |
| Replace as Pattern Outline | Promote internal line to become the outline | legacy | Authoring | Obscure |
| Trace tool | Convert baselines/lines/areas to Pattern or Internal Shape; half-symmetry aware (2026.0) | legacy | Authoring | |
| Slash & Spread | Cut-and-rotate pattern spreading for flare (improved MD10) | legacy | Authoring | Classic patternmaking op |
| Pleats tool (wizard) | Generate Knife/Box/Accordion pleats: count, depth, interval, fold angles, auto-sew | 2025.2 | Authoring | New consolidated wizard |
| Pleats Fold | Assign pleat fold angles along a direction line (type presets) | v2.3.0 | Authoring | |
| Pleats Sewing | Sew pleats to target line in 3-segment steps | v2.3.0 | Authoring | |
| Roll Up | Auto-fold hems (sleeve/pant openings) N times at distance | 12.2 | Authoring | Obscure, workflow-critical |
| Roll Up Selected Area (3D mesh) | Roll selected 3D mesh area | legacy | Authoring | Separate from 2D Roll Up |
| Offset Pattern Outline | Offset outline outward/inward (Retract option MD10) | legacy | Authoring | MD's seam-allowance substitute |
| Offset as Internal Line | Create parallel internal line(s) at offset (improved 2025.0) | legacy | Authoring | |
| Group Internal Lines and Shapes | Group internals for joint transform | 2025.1 | Authoring | |
| Seam Taping | Reinforce segments; extend modes (None/To Outline/To Seam Taping 2024.2); physical presets (Fusible x4, Reinforcement x2, custom) | legacy / 2024.2 / 2025.0 | Core-sim | Fusible interfacing sim |
| Elastic | Per-line elastic with Strength, Ratio (≤200%), Length, Entire Length | legacy | Core-sim | Shirring mechanism |
| Shirring | Gathering via elastic/fabric properties (tutorials) | technique | Core-sim | No dedicated tool; smocking also technique-only (UNVERIFIED as tool) |
| Steam brush | Brush-shrink/stretch fabric locally (shrinkage, size, hardness; Add/Remove modes 2026.0) | legacy / 2026.0 | Core-sim | Steam Eraser added 2026.0 |
| Shrink Pattern (Shrinkage Weft/Warp) | Per-pattern % shrink/stretch in 3D only | legacy | Core-sim | Keyframable since 2025.2 |
| Fold Pattern (Fold Angle/Strength/Rendering) | Fold internal lines 0–360°, strength 0–20, soft/sharp rendering | legacy | Core-sim | |
| Bond | Stiffen pattern/internal-shape area like fused interlining (preset physicals) | v2.3.0 | Core-sim | |
| Skive | Soften/thin area 0–100% | v2.3.0 | Core-sim | Obscure leather op |
| Curved Side Geometry | Curvature % on pattern edge thickness sides | v2.2.0 | Authoring | Thick-mesh edge profile |
| Add Pattern Thickness | Extrusion direction (both/fwd/back), front/back/seam face toggles (2025.0), side resolution | legacy | Authoring | Drives Thick export |
| 2D Measurements (internal shapes) | Live dimension readouts to outline/center | v2.5.0 | Authoring | Pattern annotation substitute |
| Show 2D Measurements | 2D window measurement display (revised 2025.0) | legacy | Authoring | |
| Match 2D Pattern Measurements | Match segment lengths between patterns numerically | 2024.0 | Authoring | |
| Match Up (To Start/Center/End, points) | Align patterns by segments/points | v3.0.0 | Authoring | |
| Snap to Grid / Snap to Pattern / Smart Guide | 2D snapping systems, toggleable | legacy | Authoring | |
| Fix Grid Display | Lock grid display density | legacy | Authoring | Obscure |
| 2D Pattern Window Ruler | On-canvas ruler | 2024.1 | Authoring | |
| Lock patterns (2D) | Lock patterns against editing | legacy | Authoring | |
| Move with Arrow Keys (2D) | Nudge with configurable step | legacy | Authoring | |
| 2D Background image | Reference image underlay in 2D window | legacy | Authoring | Trace-from-scan workflow |
| Show Layer Depth in 2D | Color+number overlay of sim layer values | 2024.1 | Authoring | |
| 2D Random Color Display | Random per-pattern colors | MD10 | Authoring | |
| 2D Snapshot | Export image of 2D window | 2025.0 | Authoring | |
| Zoom Extents All (2D) | Frame all patterns | legacy | Authoring | |
| Basic arithmetic in input fields | Math expressions in numeric inputs (real-time in ghost fields 2026.0) | 2024.0 | Authoring | |
| Pattern Drafter (Beta) | Parametric block generator from measurement lists (shirts; pants/skirts 2025.2; re-edit parameters 2026.0) | 2025.1 | Authoring | Parametric pattern blocks |
| AI Pattern Drafter (Beta) | Generate drafted pattern measurements from a sketch image | 2025.1 | Ecosystem (AI) | |
| Pattern Archive | Stash patterns aside out of the scene, restorable | 2026.0 | Authoring | Pattern versioning aid |
| Styleline Editing | Move seams/stylelines directly in 3D; 2D updates live | 12 | Authoring | Workflow-critical |
| Grading (graded sizes) | Multi-size grading rules | ABSENT in MD | — | CLO-only (incl. CLO Auto Grading); zero mentions in MD manual |
| Seam allowance tool | Dedicated seam-allowance object | ABSENT in MD | — | CLO-only; MD uses Offset Pattern Outline + Fold |
| Pattern notch tool | Physical notch marks on outlines | ABSENT in MD (UNVERIFIED) | — | CLO has Notch tool; MD only has sewing-direction notches |
| Round corners | Fillet pattern corners | (UNVERIFIED) | Authoring | Present in CLO; MD availability unconfirmed |
| Pattern annotation text | Free text annotation on patterns | (UNVERIFIED) | Authoring | CLO has Pattern Annotation; not found in MD manual |

## Area 2: Sewing

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Segment Sewing (2D) | Sew whole segment to segment | legacy | Authoring | |
| Free Sewing (2D) | Sew arbitrary point-to-point ranges across segments | legacy | Authoring | |
| 1:N Segment/Free Sewing (2D) | One segment sewn to N segments (Shift-hover) | legacy | Authoring | |
| M:N Segment/Free Sewing (2D) | M segments to N segments (Enter to confirm) | legacy | Authoring | |
| 3D Sewing (segment/free/1:N/M:N) | All sewing modes performed directly in 3D window | v4.2.0 | Authoring | Often missed by clones |
| Directional notches (sewing direction) | Direction indicators; crossed notches = flipped sewn result; Reverse Sewing flips | legacy | Authoring | |
| Reverse Sewing | Reverse a sewing line's direction | legacy | Authoring | |
| Edit Sewing / Select-Move Sewing Lines | Select, move, re-target sewing lines | legacy | Authoring | |
| Edit Sewing Line Length | Adjust sewn range endpoints numerically | legacy | Authoring | |
| Check Sewing Length | Compare lengths of paired sewing lines (mismatch warnings) | legacy | Authoring | |
| Sewing Line Type: Turned | Two-ply turned seam; disables fold angle | v4.1.0 | Core-sim | |
| Fold Seam Line (Fold Angle/Strength on seams) | Press effect at seams 0–360°, strength | legacy | Core-sim | |
| Seamline Property panel | Name, tension (Ease/Stretch + strength/ratio), sublayer, reverse, topstitch, puckering, 3D seamline normal map; save/open .ssp | 2025.0 | Core-sim | Seam tension = obscure, critical |
| Set Sublayer | Order self-folding sewing (seam allowances, pleats) for stability | legacy | Core-sim | Obscure |
| Activate/Deactivate Sewing Lines | Temporarily disable seams | legacy | Authoring | |
| Show/Hide Sewing | Display toggle | legacy | Authoring | |
| Delete Sewing | Remove seams | legacy | Authoring | |
| Add Point to Pattern on Start/End | Auto-add points where free sewing starts/ends | legacy | Authoring | Obscure |
| Select All Sewn Patterns | Select connected garment via sewing graph | 12.2 | Authoring | |
| Auto Sewing | AI/arrangement-point-based automatic sewing of tops/pants/skirts (front type, collar seam options; auto darts/pleats) | 2025.0 | Authoring | Default arrangement points only |
| 3D Seamline display | Show seamlines on 3D garment | legacy | Authoring | |
| Weld Turned Seamlines | Weld turned seams at export/mesh level | 2026.0 | Authoring | |
| Seamline Sync during Export | Keep seamlines synced in exports | 2025.2 | Authoring | |
| Topstitch: Segment / Free / Seamline | Three creation modes; seamline topstitch OBJ-based (MD10), outline alignment improved 12.2 | legacy/MD10 | Authoring | |
| Texture Stitch | Texture-based (non-geometry) topstitch | 11 | Authoring | |
| Topstitch styles & properties | # of lines, thread thickness (mm default 2025.2), SPI/length, offset, corner type (smooth 2025.0), extend, flip, exaggerate, face setting, custom color, texture, normal map, custom OBJ topstitch assets (2025.0), polygon optimization (2025.0); .sst open/save | legacy→2025 | Authoring | |
| Register Topstitch | Register selection as reusable topstitch | 2025.0 | Authoring | |
| Puckering (Segment/Free/Seamline) | Seam pucker normal/color maps with placement control; style window 2025.0 | 2024.0 | Authoring | |
| Piping | Rounded piping along garment edges (2D window creation MD10); edit length; Closed End option 2024.2; property setting; show/hide | legacy/MD10/2024.2 | Authoring | |
| Binding | Bind edges Under/Over/Both; width 1–50mm, length %, particle distance, fabric, grain Bias/Warp/Custom+angle, topstitch, sewing type, extend | 11 | Authoring | |
| Tack (cloth-to-cloth) | Tack garment points together; to multiple vertices/patterns and Trims (2025.0) | legacy | Core-sim | |
| Tack on Avatar | Pin garment point to avatar surface | legacy | Core-sim | |
| Line Tack on Avatar / on Garment | Tack a whole line to avatar or garment | legacy | Core-sim | Obscure |
| Edit/Delete/Copy/Paste Tack | Tack management | legacy | Authoring | |
| Pins (Box/Lasso) | Pin mesh vertices in place; on segments/patterns/internal lines; W-click quick pin | legacy | Core-sim | |
| Attach Pin to / Detach from Avatar | Convert pins into avatar-following anchors | legacy | Core-sim | "Pin-to-body" |
| Duplicate Pins to Symmetric Pattern | Mirror pins | 2024.2 | Authoring | |
| Pin Group Selection | Double-click selects all pins on a pattern | 2025.2 | Authoring | |
| Lacing tool | Click eyelet holes in sequence; auto-generates editable lace pattern; reverse per-hole entry; curvature slider | 2026.0 | Authoring | |
| Notch Sewing (Modular) | Save sewing relationships via notch placements in modular blocks | v3.0.0 | Authoring | Obscure |

## Area 3: Simulation & Physics

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Simulate presets: Normal (Default) | Fastest working-quality sim | v4.2.0 (presets) | Core-sim | |
| Simulate: Animation (Stable) | Higher-quality preset for recording caches (toolbar 2024.2) | v4.2.0 | Core-sim | |
| Simulate: Fitting (Accurate Fabric) | Most accurate stretch, slowest | v4.2.0 | Core-sim | |
| Complete Nonlinear Simulation | Nonlinear solver for accurate elongation (long garments) | v3.0.0 | Core-sim | |
| CPU / GPU simulation | Selectable processor; GPU collision parity with CPU since 2024.2 | legacy / 2024.2 | Core-sim | GPU limits: trims/softbody (lifted 2025.2/2026.0) |
| GPU trim simulation | Trims simulated on GPU | 2026.0 | Core-sim | |
| Soft Body Simulation | Soft-body objects (props/avatars); custom avatars 2025.1; GPU 2025.2 | 2024.0 | Core-sim | |
| Instant simulation stop | Abort sim instantly under load | 2025.2 | Core-sim | |
| Interactive editing | 2D point/segment edits reflected live during sim | v4.2.0 | Core-sim | |
| Simulation Properties (.smp) | Time step, simulations/frame (substeps), CG iteration/residual finish condition, self-collision iteration count, air damping, gravity (-9800 default), CPU core count, nonlinear toggle; open/save | legacy | Core-sim | Full solver exposure |
| Collision detection toggles | Avatar-cloth / self-collision / proximity, each Triangle-vertex + Edge-Edge; avoidance stiffness | legacy | Core-sim | |
| Intersection Resolution | Resolve intersecting mesh via normal/flipped-normal | legacy | Core-sim | Obscure |
| Layer Based Collision Detection | Layer/sublayer-ordered collision | legacy | Core-sim | |
| Particle Distance per pattern | Mesh density 0.8–700mm | legacy | Core-sim | |
| Layer (Use Layer / drape additional garment) | Per-pattern layer int (±) ordering for multi-layer dressing | legacy | Core-sim | Reset to 0 after drape |
| Pressure (Express Air Pressure) | Inflation for pillows/padded items; keyframable 2025.2 | legacy | Core-sim | |
| Shrinkage Weft/Warp | Per-pattern shrink; keyframable 2025.2 | legacy | Core-sim | |
| Simulation thickness (collision) | Default 1.5mm/side collision envelope per pattern | legacy | Core-sim | "Skin offset" per pattern |
| Add'l Thickness Rendering | Visual/export thickness on top of fabric thickness | legacy | Authoring | |
| Freeze/Unfreeze | Freeze patterns as static colliders | legacy | Core-sim | |
| Deactivate (Pattern Only) / (Pattern & Sewing) | Exclude from sim, two scopes | legacy | Core-sim | Two distinct modes |
| Strengthen/Unstrengthen | Temporary stiffening; Partial Strengthen (2024.0) | legacy | Core-sim | |
| Solidify | Lock drape state per pattern w/ strength; Partial Solidify (11); keyframable 2025.2 | v2.5.0 | Core-sim | Quilting/shaping |
| Press tool | Flatten sewn double layers (auto-sets Turned) | legacy | Core-sim | |
| Wind Controller | Gizmo wind: activate, unlimited bound, Spherical/Planar type, strength, decay; multiple controllers (2024.0); keyframable (2025.0) | legacy / 2024.0 | Core-sim | |
| Gravity setting | Adjustable per scene (0 = weightless) | legacy | Core-sim | |
| Ground Setting | Ground plane collision config | legacy | Core-sim | |
| Avatar Friction Setting | Per-avatar friction | legacy | Core-sim | |
| Avatar Skin Offset | Collision offset around avatar | legacy | Core-sim | |
| Scene & Props Collision | Props as colliders w/ collision thickness 0–100 | 2024.0 | Core-sim | |
| Trim simulation with collision | Trims collide during sim (CPU only until 2026.0) | 2024.2 | Core-sim | |
| Quick Pinching | Drag-point pinch during live sim (default Q) | legacy (named 2025.0) | Core-sim | |
| Advanced Pinching with Soft Selection | Falloff shape/distance/power, surface vs straight distance, limit-to-pattern | 2024.2 / 2025.0 UI | Core-sim | |
| Sculpt mode | Wrinkle/Release (MD10 Dynamic Wrinkle Brush), Sculpt, Smooth, Grab, Stamp, Pinch brushes w/ alpha, custom material preview; warns on sim start (2025.1) | MD10 / 2025.0 | Authoring | Sculpt layer separate from sim mesh |
| Morph Target (OBJ morphing) | Morph avatar between OBJs over N frames (dress rigid items) | legacy | Core-sim | Also "Morph Target with AVT Files" |
| Fold Arrangement | Pre-fold patterns along internal lines before sim (symmetric folding 11) | legacy | Authoring | Critical for collars/plackets |
| Superimpose (Over/Under/Side) | Arrange sewn pattern directly onto counterpart | legacy | Authoring | |
| Smart Arrangement | Arrange new sewn pattern respecting drape | v3.0.0 | Authoring | |
| Garment Fit Maps | Stress map (kPa), Strain map (%), Fit map (can't wear/very tight/tight), Pressure points | legacy | Core-sim | Strain fabric-aware 2026.0; 2D-pattern display 2026.0 |
| Garment Fit Properties | Fit-related property set | 2025.0 | Core-sim | |
| Keyframable sim properties | Keyframe shrinkage/solidify/pressure/fabric/wind over time | 2025.0–2025.2 | Core-sim | Animation-driven physics |
| Down/Padding fill | Volumetric down fill | ABSENT in MD | — | CLO-only; MD uses Pressure+Layer Clone |
| Auto Seal | Auto-seal function (listed 2025.0 All-in-One) | 2025.0 | Core-sim | (UNVERIFIED detail) |

## Area 4: Fabric System

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Fabric objects per garment | Multiple fabrics; apply per pattern; copy/rename/delete/reset | legacy | Authoring | |
| Physical Property Presets | Library of preset fabric physics (~77 types incl. silk, denim, leather...) | legacy | Core-sim | |
| Physical Property Detail | Stretch-Weft/Warp/Shear, Bending-Weft/Warp, Buckling Ratio, Buckling Stiffness, Internal Damping, Density→Weight gsm (renamed 2024.1), Friction Coefficient | legacy | Core-sim | Full anisotropic cloth model |
| Collision Thickness (fabric) | Per-fabric collision envelope | legacy | Core-sim | |
| Rendering Thickness (fabric) | Visual thickness | legacy | Authoring | |
| Open/Save Fabric (.zfab) / Physical Property (.psp) | Fabric asset files | legacy | Authoring | |
| Physical Property Creator info | Guidance for making custom measured properties | 2025.0 | Authoring | Real fabric measurement kit = CLO-only |
| Fabric Front/Back/Side Setting | Different textures/materials per face and side | legacy | Authoring | |
| Texture maps | Base color, Normal, Displacement (clipping/particle distance/continuity), Opacity (+Alpha map RGB/Alpha modes MD10), Roughness, Metalness (MD10) | legacy/MD10 | Authoring | |
| PBR Map Generator | Auto-generate normal/displacement/roughness/metal/opacity from one image | 2024.1 | Authoring | |
| Import All PBR Maps at Once | Batch-load map set from folder | 2025.0 | Authoring | |
| Texture Repeat Type: Unified Map | UV-layout-based unified texturing vs tiled | MD10 (renamed 11) | Authoring | |
| Edit Texture / Edit Texture Size | Transform textures on patterns (back texture MD10) | legacy | Authoring | |
| Print on Fabric | Print layer on fabric: face, repeat (Block/Half Drop/Brick/Diamond/Stripe), space/shift, PBR blending (2025.1), .prt save | 11-era (UNVERIFIED exact) | Authoring | Half-drop repeats = obscure |
| Denim Wet Wash | Procedural denim wash: strength, direction, masking, blend multiple washes | 2025.1 | Authoring | |
| Fur material (Beta) | Fur strands in 3D window; graphics+seams maintained on fur (2026.0) | 2025.0 | Authoring | |
| Express Cartoon / Toon material | MToon-style toon: shade color/hardness/shift, emission, matcap, rim light, outline | 2026.0 | Authoring | |
| Substance SBSAR support | Procedural substance materials on fabrics, trims, buttons, avatars; bake channels | 11/12 | Ecosystem | |
| Opacity Setting | Per-fabric opacity (sheer) | legacy | Authoring | |
| Color Palette / Swatches | Palette library, add/edit swatches, open/save palette, Search Color (11), Desaturation, Eyedropper | legacy | Authoring | |
| Default Color Setting | Default garment color | legacy | Authoring | |
| Fabric List/Style windows | Fabric management UI (Object Browser 2025.0) | legacy | Authoring | |
| Preview Fabric Information | Hover fabric info | legacy | Authoring | |
| Blend Graphic/Print with Fabric | Blend modes between graphic/print and fabric maps | 2025.0/2025.1 | Authoring | |
| Show/Hide Texture, Refresh Texture | Display/reload | legacy | Authoring | |
| Render-only fabric types display | Shows CLO/CONNECT-only fabric types in MD projects | 2024.2 | Ecosystem | CLO-compat |
| Custom fabric measurement kit | Measure real fabric → digital properties | CLO-only | — | Not in MD |

## Area 5: Avatars

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Default avatars | Mia & Luka (12.2 defaults), kid avatars Melody/Oliver, legacy sets | legacy | Authoring | |
| 18 MetaHuman body types | Encrypted .avte MetaHuman bodies in library | 2024.0.173 | Ecosystem | |
| Avatar Editor | Tabs: Avatar Size, Measure, Arrangement, Fitting Suit, IK Joint | 2025.0 (rework; Size Editor MD10) | Authoring | |
| Avatar Size editing | Preset sizes (Curvy/Straight/Petite/Plus; men 34–52) + custom measurements, link/unlink proportional algorithm from body-scan data, .avs files | MD10 / 2025.0 | Authoring | Body-scan-driven sizing |
| Avatar Tape Measures | Linear tape, Circumference tape (5.1.4), Basic/Surface variants, height measure, edit/delete/rearrange/show/hide, open/save .mea, symmetric tape (11), type change | v5.1.4+ | Authoring | |
| 3D Garment Measure | Create/edit measures on garment itself | legacy | Authoring | Obscure |
| Arrangement Points | Blue placement points; add/delete/open/save (.arr) | legacy | Authoring | |
| Arrangement Bounding Volumes | Joint-attached cylinders (+Cuboid MD10); add/delete/reset/fit-to-avatar; open/save (.pan) | legacy | Authoring | |
| Arrange with points / Flip Wrap Direction | Snap patterns around body; wrap direction flip | legacy | Authoring | |
| Arrangement property setting | Offset/orientation per arranged pattern | legacy | Authoring | |
| Direct Positioning | Click-place trims/patterns onto surface | legacy | Authoring | |
| Symmetrical 3D State | Mirror 3D arrangement state | 2024.0 | Authoring | |
| Reset 2D/3D Arrangement | Reset placement | legacy | Authoring | |
| X-Ray Joints / Show Avatar Joints | See joints through mesh; hide selected joint, adjust joint size (2024.2) | 12 | Authoring | |
| Adjust Avatar Pose (joint drag) | Pose via joint rotation; IK mode (2025.0) | legacy | Authoring | |
| Avatar IK Joint Mapping | Map imported skeletons to IK (Daz/Mixamo/CC/MetaHuman naming auto) | 2025.0 | Authoring | |
| Pose files (.pos) open/save | Poses; auto-saved with garment | legacy | Authoring | |
| Motion files (.mtn) open/save | Joint motions | legacy | Authoring | |
| Play Motion | Play avatar motion (drives drape animation) | legacy | Core-sim | |
| Register Pose/Motion | Register to library from scene | 2024.2 | Authoring | |
| Auto Convert to Avatar | Convert OBJ/FBX/AVT/Daz mesh to rigged MD avatar (gender, auto/custom rig, MD skin/rigging-only/size-editable modes, exclude hands/feet) | 2024.0 | Authoring | |
| Auto Convert to Motion | Convert imported animation to MD motion | 2025.0 | Authoring | |
| MetaHuman DNA Importer | Import MetaHuman DNA rigs | 2025.2 | Ecosystem | |
| Blend Shape Avatar | Avatar blendshape support | 2026.0 | Authoring | |
| Change Pose by COLLADA joints / OBJ morphing | Legacy pose-change routes | legacy | Authoring | |
| Morph Target with AVT Files | Morph between avatar files | legacy | Core-sim | Layered-dressing trick |
| Register Accessory (.zacs) | Hair/Shoes/Glasses/Earring/Hat (Hat 2025.2); dummy-mesh conventions; Rigged type (2026.0) | 2025.0 / 2026.0 | Authoring | Parent-child accessories |
| Avatar: Change Heels | Heel/shoe foot posing | MD10 | Authoring | Obscure |
| Style Configurator for Kid Avatars | Kid avatar styling | 2025.2 | Authoring | |
| Avatar Materials / Object Property Editor | Avatar material editing; Eye Control (2025.0) | legacy | Authoring | |
| Scale Avatars | Uniform avatar scaling | MD10 | Authoring | |
| Avatar Smooth & Division | Subdivide/smooth avatar mesh; smooth template avatars (2024.2) | v4.2.0 | Authoring | |
| Deactivate/Activate Avatar | Toggle collider | legacy | Core-sim | |
| Show/Hide avatar, Show All Avatars | Display control (12.1) | legacy | Authoring | |
| Show Wireframe for Avatars | Avatar wireframe | 2025.1 | Authoring | |
| Avatar UV Display | Show avatar UVs | 2026.0 | Authoring | |
| Move Avatars & Garments to Default Position | Reset scene placement | legacy | Authoring | |
| Animation Import (avatar) | Import motions for avatars | legacy | Authoring | |
| Soft body for custom avatars | Jiggle/soft body on imported avatars | 2025.1 | Core-sim | |
| AI Pose Generator (Beta) | Pose from text prompt or reference image → .pos | 2025.1 | Ecosystem (AI) | |

## Area 6: Fitting & Grading

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Create Fitting Suit | Draw reference lines on custom avatar → fitting suit saved into .avt | MD10 | Authoring | Prereq for auto fitting |
| Automatically Create Fitting Suit at Import | Auto-generate fitting suit when importing avatar | 12 | Authoring | |
| Auto Fitting | Resize garment to different avatar (maintain curvature %, graphic size, texture size; Maintain Topology 2025.1; texture size kept 12.2) | MD10 | Core-sim | |
| Re-Target Draping / Re-Drape 3D Arrangement | Re-drape garment on new avatar WITHOUT resizing | 2025.0 | Core-sim | |
| Show Avatar/Garment Fitting Suit | Inspect fitting suits | 11 | Authoring | |
| Fit Maps (stress/strain/fit/pressure) | See Area 3 | legacy | Core-sim | On 2D patterns 2026.0 |
| Flatten (3D→2D) | Flatten 3D pen/avatar-surface areas into 2D patterns; multiple areas as one merged pattern (2025.1); optimize flattened outline points (2026.0) | legacy | Authoring | 3D-to-2D patternmaking |
| Flattening as Straight Line | Flatten with straightened edge constraint | legacy | Authoring | Obscure |
| 3D Pen (Avatar) / Create Line (Avatar) | Draw curves on avatar surface (off-avatar drawing 2025.1) | legacy / 2025.1 | Authoring | |
| 3D Pencil (Avatar) | Freehand sketch on avatar w/ symmetry+eraser → convert to 3D Pen → patterns; front/back auto-sew | 2026.0 | Authoring | |
| 3D Pen (Garment) / Edit lines | Draw/edit lines on garment; flatten from them | legacy | Authoring | |
| Add Point or Curve Point to Line (Avatar) | Edit avatar lines | legacy | Authoring | |
| Show Line (Avatar) | Display avatar lines | legacy | Authoring | |
| Remeshing (Quad Grid) | Regenerate aligned quad mesh per pattern (axis follows 2D rotation) | MD8 | Core-sim | |
| Auto-Grading | Automatic size-set grading | ABSENT in MD | — | CLO 2024-only |
| Body measurement comparison | Avatar tapes + size editor readouts | MD10 | Authoring | No dedicated compare report (CLO tech pack CLO-only) |

## Area 7: Trims & Hardware

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Create Button / Buttonhole | Place buttons/buttonholes on patterns | legacy | Authoring | |
| Buttons/Buttonholes along Pattern Outline | Auto-distribute along outline | 5.1 / MD11 | Authoring | |
| Fasten/Unfasten Button | Physically button through buttonhole in sim; move fastened pairs | legacy | Core-sim | Workflow-critical |
| Button styles | Shape presets, size, weight, material (metal presets updated 2026.0), thread material (v4.2.0), name; .btn | legacy | Authoring | |
| Buttonhole styles | Shape, size, material, name; Z-offset (2024.2); .bth | legacy | Authoring | |
| Custom OBJ button / custom buttonhole image | Register own geometry/images | legacy | Authoring | |
| Custom button texture | Create/apply button texture (size-based modeling set MD11) | legacy | Authoring | |
| Mirror/symmetric duplicate button & buttonhole | Mirror Paste, Duplicate to Symmetric Pattern | legacy | Authoring | |
| 2D Button/Buttonhole measurements | Dimension readouts | legacy | Authoring | |
| Button polygon optimization | Reduce button/zipper/accessory polycount numerically or by planes | 2024.0 | Authoring | |
| Zipper tool | Draw zipper along edges in 3D or 2D (MD10); auto-generated geometry | v6-era (legacy) | Authoring | |
| Zip/Unzip | Open/close zipper interactively; slider gizmo | legacy | Core-sim | |
| Two-way zippers | Head-to-head or bottom-to-bottom locking | 2025.0 | Authoring | |
| Zipper properties | Tape width/color/texture, teeth size/type, stopper (MD10), slider/puller selection | legacy | Authoring | |
| Custom OBJ zipper slider/puller | Register custom sliders/pullers | 12 | Authoring | |
| Custom/preset OBJ zipper teeth | OBJ teeth w/ presets (2025.0), fully custom (2026.0) | 2025.0/2026.0 | Authoring | |
| Separate Teeth & Tape | Independent teeth/tape control | 11 | Authoring | |
| Zipper materials per part | Different materials for slider/puller/stopper/teeth | 12 | Authoring | |
| Zipper Style window | Zipper style management (2025.1); Edit Zipper tool (2025.2) | 2025.1 | Authoring | |
| OBJ Trims | Import OBJ/FBX as trim; Glue to garment; transform; multi-select (11); weight; stiffness; sim status | v3.0.0 | Authoring | |
| Trim Style Window | Trim library mgmt (2024.1, reworked 2025.0); .trm files | 2024.1 | Authoring | |
| Tack on Trim | Tack trims at multiple points/patterns | 2025.0 | Core-sim | |
| Trim Mirror Paste | Mirror trims | 2025.1 | Authoring | |
| Convert Pattern → Trim | Pattern becomes rigid trim (armor workflow) | 2025.2 | Authoring | Rigid-body garments |
| Convert Pattern → Avatar Accessory | Pattern becomes .zacs accessory | 2025.2 | Authoring | |
| Graphics (2D/3D placement) | Place images on patterns from 2D or directly on 3D garment; over seamline (MD10); Z-offset (MD10) | legacy | Authoring | 3D placement projects across seams |
| Graphic types | Embroidery / Logo / Print / Wash (Jeanologia) style types | 2025.0 | Authoring | Jeanologia = obscure |
| Graphic control suite | Normal map, transform, tile (X/Y/pattern), copy/duplicate, measurements, placement-by-measurement (2025.2), on/off toggle (2026.0) | legacy→2026 | Authoring | |
| Patch Style | Patch trim type | 2024.1 | Authoring | |
| Grommets | Dedicated grommet tool | ABSENT (UNVERIFIED) | — | Via custom trims + holes only |
| Belts | Dedicated belt tool | ABSENT | — | Built from patterns/trims |
| Embroidery sim | True stitch-level embroidery | ABSENT | — | Graphics-based only |

## Area 8: Retopology / Mesh / UV

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Triangular/Quadrangular mesh toggle | Per-pattern mesh style: Triangle, Quad (Optimized), Quad (Grid) | legacy (reorganized 2024.0) | Core-sim | |
| Improved quad remeshing | Better auto quad retopo | 2024.2 | Authoring | Perf/quality again 2026.0 |
| Retopology tools | Draw topo points/lines/faces on patterns; division loops (Shift+Ctrl), extrude, cut faces, Ngon highlight; face selection + Show Topology Only (12.1); hotkeys Y/U (11) | v5.1.0, major 12 | Authoring | |
| Remeshing to Retopology (Selected) | Convert remesh to editable retopo | 5.1.4 | Authoring | Obscure |
| Lock Remesh Patterns | Lock remeshed patterns in 2D | 12 | Authoring | |
| Select Mesh (box) / Select Mesh Brush | Mesh-level selection incl. internal lines (v4.0.0); brush (MD9) | legacy | Authoring | |
| UV Editor mode | Edit garment UVs; tiles; wire display (7.5) | legacy | Authoring | |
| Edit UV location / Move UV with value | Precise UV transforms | MD10 / 11 | Authoring | |
| Automatic UV packing | One-click packing | 12.2 | Authoring | New nesting algorithm 2024.1 |
| UV Snapshot | Save UV layout image per tile | legacy | Authoring | |
| Bake Textures (UV mode) | Bake garment textures to unified UV | legacy | Authoring | |
| View UV maps (diffuse/normal/rough/metal/displacement) | Per-map UV preview; displacement view/bake (11); OBJ topstitch UVs (11) | MD9–11 | Authoring | |
| Change Normal Map Blending Method | Blend mode for normal maps | 12 | Authoring | Obscure |
| UV texture seam improvements / Fill Texture Seams | Seam padding incl. infinite non-invading padding | 2024.1 | Authoring | |
| Unified UV Coordinates (0–1) export | Combine all patterns into one UV space at export | v3.1.0 | Authoring | |
| Back/Side UV Expansion | Edit back+side-face UVs of thick meshes | 2025.0 | Authoring | Obscure, export-critical |
| Select All with Same Property (UV) | Property-based UV selection | 2025.2 | Authoring | |
| Anti-alias lines (UV) | Display option | 2025.0 | Authoring | |
| Thin/Thick mesh export | Export flat or extruded-with-thickness meshes | v2.3.0 | Authoring | |
| Weld/Unweld export | Merge sewn vertices or keep split at export | legacy | Authoring | |
| Merge Vertex by Proximity at Export | Distance-based weld at export | 12 | Authoring | |
| Density maps (mesh density painting) | Paint variable mesh density | ABSENT (UNVERIFIED) | — | Not found in MD manual |

## Area 9: Animation

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Animation mode + Timeline | Dedicated animation workspace/editor | legacy | Authoring | |
| Record Animation | Record drape sim as garment cache while motion plays | legacy | Core-sim | |
| Play / Deactivate timeline / Delete animation | Playback controls | legacy | Authoring | |
| Scene Time Warp | Retime scene animation | legacy | Authoring | Obscure |
| Wind Animation | Animate wind in timeline | legacy | Core-sim | |
| Animation Editor improvements | Keyframe editing of caches: delete keys, re-key with S, copy/paste keys, blend/loop caches | 12 | Authoring | Cache keyframe editing = obscure |
| Animation Layers | Multi-layer timeline (12.1); layer setup (2025.2); clamp range to selected layers (2025.1); cut/merge cache layers | 12.1–2025.2 | Authoring | |
| Keyframe Animation system | Key avatar joints, wind, sim/fabric props; hierarchical property layers; insert-key from Property Editor | 2025.0 | Authoring | |
| Keyable property expansion | Shrinkage weft/warp, Solidify, Pressure keyable | 2025.2 | Core-sim | |
| Animation Marker | Timeline markers | 2026.0 | Authoring | |
| Fit Horizontal / frame range fit | Fit timeline view (12.1; slider run 2024.0) | 12.1 | Authoring | |
| Save min-max animation range in .zprj | Timeline range persisted | 2024.0 | Authoring | |
| Video Capture | Render viewport animation to MP4/AVI/MOV with size presets, autoseq numbering | 2024.0 | Authoring | |
| Cache export: Maya .mc/.mcx, PC2, MDD (Standard & Maya/Max) | Point-cache animation export w/ weld/thin-thick options | legacy | Authoring | |
| OBJ Sequence export | One OBJ per frame | legacy | Authoring | |
| Alembic import/export | Vertex+animation interchange; file type settings (11); current-frame option (2024.0); no-FPS-change (12.2); morph animation (2026.0) | legacy | Authoring | |
| USD animation data | Animation via USD (2024.1); cached avatar animation export (2024.2) | 12.1+ | Authoring | |
| Export Joint Keyframe Animation with FBX | Joint keys out via FBX | 12 | Authoring | |
| Bake Keyframes for joint export | Key every frame at export | 2026.0 | Authoring | |
| Play Region export option | Export only playback range (FBX/glTF) | 2026.0 | Authoring | |
| Animation Import | Import motions (FBX/COLLADA) for avatars incl. camera animation data (FBX v2.2.0) | legacy | Authoring | |
| Camera keyframes in timeline | Keyframe MD camera | (UNVERIFIED) | Authoring | Camera anim import via FBX confirmed; native camera keying unconfirmed |
| Garment Animation Making | End-to-end drape-animation workflow docs | legacy | Authoring | |
| Animation with Trim Simulation | Trims simulated during recording | 2025.0 | Core-sim | |
| Auto Convert to Motion | Imported animation → .mtn | 2025.0 | Authoring | |

## Area 10: Rendering / Viewport

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Garment rendering styles | Textured, Thick Textured, Monochrome, Mesh/Wireframe variants, Wireframe-on-Monochrome (Thick) (2024.2); new 2D styles (2025.0) | legacy | Authoring | |
| Schematic Render | Stylized schematic/flat display of garment | 2025.0 | Authoring | |
| Toon Shader | See Area 4 | 2026.0 | Authoring | |
| Show 3D Shadow / Grid / Mesh | Viewport toggles | legacy | Authoring | |
| Show Light Controller / Show Light | Adjustable viewport lights | 2025.0 (controller) | Authoring | No offline renderer in MD |
| Offline render engine (V-Ray) | Ray-traced rendering | ABSENT in MD | — | CLO-only; MD10 "High-time Quality Render" is display-level (UNVERIFIED detail) |
| Colorways | Multiple color variants per garment | ABSENT in MD | — | CLO-only; MD has only garment-info Colorway metadata field |
| Print Layout mode | Physical print/plot layout | ABSENT in MD | — | CLO-only; MD states no paper-pattern printing |
| 3D Snapshot | PNG snapshot of 3D view; layout improved 12.2 | legacy | Authoring | |
| Multiview/Turnaround snapshot | Multi-angle & turntable stills | 2025.0 | Authoring | |
| Snapshot as HTML | Interactive HTML snapshot export | MD10 | Authoring | Obscure |
| Thumbnails in 4 views | Save 4-view thumbnails | MD10 | Authoring | |
| Turntable images + XML metadata on save | Meta-data .zprj/.zpac save emits XML + turntable images (folder or ZIP) | v3.1.38 | Ecosystem | Obscure pipeline hook |
| Show Statistics | FPS/mesh statistics overlay | 12.1 | Authoring | Replaced Show Simulation Speed |
| Isolate Selection | Isolate selected objects in viewport | 2026.0 | Authoring | |
| Camera Setting / Show Camera / Custom View | Camera FOV (.cmp), saved transforms (.cmt), custom saved views | legacy | Authoring | |
| Zoom Extents All (3D) | Frame scene | legacy | Authoring | |
| 3D Background options | Background color/image; aspect-ratio maintain toggle (2025.0) | legacy | Authoring | |
| 2D/3D sync editing | 2D pattern & 3D garment windows live-linked | core design | Core-sim | Foundational |
| Switch Viewport Layout by Hotkey | Cycle window layouts | 12.1 | Authoring | |
| Show 3D Garment Window Size | Display viewport pixel size | legacy | Authoring | Obscure |
| Show/Hide 3D Color | Hide overlay colors (layers/solidify etc.) | 2024.1 | Authoring | |
| Ghost/hide avatar | Avatar display toggles incl. X-ray | legacy | Authoring | |

## Area 11: Import / Export

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| OBJ import/export | Geometry+material; import as avatar/trim/prop/morph; export w/ weld/thin-thick/unified UV/zip-with-textures | legacy | Authoring | |
| FBX import/export | Mesh/material/joint/animation/camera; avatar skeleton export (11); multi-avatar (12.2); axis conversion (12.2); as trim (12); as scene&props (2024.2); material names (2025.0); Daz scale (2024.0) | v2.2.0+ | Authoring | |
| Alembic import/export | See Area 9 | legacy | Authoring | |
| COLLADA (.dae) import | Rigged avatar import (OpenCOLLADA) | legacy | Authoring | Import-only |
| USD import/export | Full USD (12.1); USD Layer window; single/multi object (2024.0); sim-data option for UE Chaos Cloth (2024.0.173); thin/thick via layer (2024.2); axis conversion (2024.2); unified UV (12.2) | 12.1 | Authoring | |
| USDZ export | AR package export | 2025.0 | Authoring | |
| glTF 2.0 / GLB import/export | Avatar/trim import w/ auto arrangement points; export w/ lights, animations, embedded, metadata XML extras | 2026.0 | Authoring | |
| VRM import/export | Metaverse avatar format (glTF-based; humanoid rig required) | 2026.0 | Authoring | |
| LXO export | Modo export | legacy | Authoring | Obscure |
| Maya/Max/point caches | .mc/.mcx/.pc2/.mdd | legacy | Authoring | |
| DXF (AAMA/ASTM) import/export | CAD pattern exchange | ABSENT in MD | — | Explicitly CLO-only per official FAQ — major clone-planning fact |
| Adobe Illustrator / AI curves import | Vector pattern import | ABSENT | — | Official FAQ: cannot be imported |
| SVG / PDF / PLY / STL | Other formats | ABSENT (UNVERIFIED) | — | Not in compatible-format table |
| Texture image formats | jpg/png/bmp/psd/tga/tif/gif/dds/hdr/exr/pict + ~30 more | legacy | Authoring | |
| Substance .sbsar import | Procedural materials | 11/12 | Ecosystem | |
| Native: .zprj / .zpac | Project / garment files; Add (merge) variants; meta-data variants | legacy | Authoring | |
| Native: .avt / .avte / .pos / .mtn / .avs | Avatar, encrypted avatar, pose, motion, avatar-size | legacy | Authoring | .avte cannot be exported |
| Native: .zfab / .psp / .sst / .ssp / .btn / .bth / .trm / .zacs / .prt | Fabric, physics, topstitch, sewing style, button, buttonhole, trim, accessory, print | legacy→2025 | Authoring | |
| Native: .arr / .pan / .mea / .cmp / .cmt / .smp | Arrangement points, bounding volumes, tapes, camera proj/transform, sim properties | legacy | Authoring | Deep asset-file granularity |
| Import/Export Presets | Save/recall import-export dialog settings | 2025.1 | Authoring | Obscure automation aid |
| Merge Vertex by Proximity at Export | See Area 8 | 12 | Authoring | |
| OBJ auto-scale/unit handling | Unit guess for OBJ; FBX units native | legacy | Authoring | |
| GoZ (ZBrush bridge) | Round-trip with ZBrush (plugin improvements 2024.2) | legacy | Ecosystem | |
| Omniverse Connector | NVIDIA Omniverse live connect (Beta) | 12.1 | Ecosystem | |
| MD LiveSync | One-click sync of mesh/materials/skeletal anim/geo caches to Unreal etc. | 2024.0 | Ecosystem | |
| Sansar export | Sansar platform export | legacy | Ecosystem | Obscure |
| EveryWear (Beta plug-in) | Auto-rig garments for games/VRChat; masking brush, thin/thick (2024.2), partial auto rigging (2025.1), target joint edit/mirror weights/bind pose (2025.2), FBX export (2025.2), Template-Based Rigging (2026.0) | 2024.0 | Ecosystem | Garment auto-rigging pipeline |
| Save Selected Garment | Export only selected garment | 2025.1 | Authoring | |
| Open Auto Save File | Recover autosaved .zprj/.zpac | legacy | Authoring | |
| Project thumbnail icons / custom ZSE thumbnails | File-icon thumbnails (2024.0/2024.2) | 2024.0 | Authoring | |
| 2GB file-size limit handling | Known save limit + fix guidance | legacy | Authoring | Engineering constraint |
| GarmentCode support | Programmatic garment format | ABSENT (UNVERIFIED) | — | No evidence in MD docs |

## Area 12: Modular / Library Workflows

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| Modular Mode | Combine garment "Blocks" via Block Editor into full garments | v3.0.0 | Authoring | |
| Module Structure / Block folders | Block taxonomy, folder management in Configurator | v3.0.0 | Authoring | |
| Create/Edit Custom Blocks | Author own modular blocks; open/save blocks | v3.0.0 | Authoring | |
| Sew Blocks / Notch Sewing tags | Auto-join blocks via saved sewing relations | v3.0.0 | Authoring | |
| Modular Configurator ordering/selection | Guided block choice flow | v3.0.0 | Authoring | |
| Modular Library | Group/Category/Style/Block library; auto modular labeling; save style/block | 2025.0 | Authoring | |
| Library Window | Asset browser (garments, avatars, fabrics, trims, poses); My Library | legacy | Authoring | |
| New Library Window | Reworked universal browser; docked widget (2025.2); preview in docked library (2026.0); list view, show extensions, select-all downloads (12.2) | 2025.0 | Authoring | |
| CONNECT Store | In-app 3D asset store (Store 11 → CONNECT); download folder path setting (2026.0) | 11 | Ecosystem | |
| CLO-SET file sharing | Cloud share/versioning via CLO-SET | MD10 | Ecosystem | |
| One CLO-SET account sign-in | Unified account | 2024.0 | Ecosystem | |
| Default asset library | 77 fabrics, default garments, avatars, motions, trims | legacy | Authoring | |
| Garment/Project Meta Data | Editable info: Code, Name, Description, Price, Fabric, Size, Colorway, Category, Memo | v4.1.0 | Ecosystem | |
| Welcome window / live update | Launch content feed; live patch updates | 11 | Ecosystem | |

## Area 13: AI / Auto Features (2024–2026)

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| AI Studio plug-in activation | Opt-in AI plugin framework (icon 2025.1; activation flow 2025.2) | 2024.1 | Ecosystem | |
| AI Texture Generator (Beta) | Generate fabric textures from prompts | 2024.1 | Ecosystem | |
| AI Graphic Generator (Beta) | Generate graphics (Embroidery/Patch/Pop Art styles) | 2024.1 | Ecosystem | |
| PBR Map Generator | One-click PBR map set from image (non-LLM ML) | 2024.1 | Authoring | |
| AI Image Generator | Prompt/image → garment images; Virtual Try-On; Flat-Lay generation | ~2025.2 | Ecosystem | Image-to-image, not image-to-3D-garment |
| AI Pose Generator (Beta) | Text/image → avatar pose (.pos) | 2025.1 | Ecosystem (AI) | |
| AI Pattern Drafter (Beta) | Sketch → drafted parametric pattern measurements | 2025.1 | Ecosystem | Closest thing to image-to-garment |
| Auto Sewing | See Area 2 (heuristic, arrangement-point based) | 2025.0 | Authoring | |
| Auto Fitting / Auto Convert to Avatar/Motion | See Areas 5/6 | MD10/2024/2025 | Authoring | |
| Text-to-garment 3D | Direct text → 3D garment | ABSENT (UNVERIFIED) | — | Not offered as of 2026.0 |

## Area 14: UX / System Features Affecting Parity

| Feature | What it does (1 line) | MD version | Class | Notes |
|---|---|---|---|---|
| History Window + 3D States | Undo history panel; save/restore named 3D states (preview improved 12.2) | legacy | Authoring | Checkpoint system beyond undo |
| Undo/Redo | Standard undo incl. tool-specific step-undo | legacy | Authoring | |
| Object Browser | All scene objects + style windows + scene window unified | 2025.0 (rework) | Authoring | |
| Scene Browser | Scene hierarchy (wind, lights, props) | 2025.0 | Authoring | |
| Property Editor | Context property panel w/ open/save of property files | legacy | Authoring | |
| Modes | SIMULATION / ANIMATION / UV EDITOR / MODULAR / SCULPT (+Pattern Drafter viewport) | legacy | Authoring | No RENDER mode (unlike CLO) |
| Custom UI Layout | Save/restore workspace layouts; open windows from title-bar right-click | 2025.0 | Authoring | |
| UI Color Customization | Theme colors | 2025.2 | Authoring | |
| Set Shortcuts + search | Full hotkey remapping; shortcut search (MD10); Active Tool Hotkey Reference window (12) | v4.2.0 | Authoring | |
| Preferences | Graphics, view controls, UI, 2D/3D, snap, camera, simulation defaults, default files, language | legacy | Authoring | |
| Unit systems | mm/cm/inch/feet-inch across dialogs | legacy | Authoring | |
| Gizmos | Unified/Divided gizmo, axis setting, scale modification | legacy | Authoring | Configurable gizmo = obscure |
| Move with Arrow Keys (3D) | Keyboard nudge in 3D | legacy | Authoring | |
| Save User Setting as Configuration File | Portable settings export | legacy | Authoring | Obscure |
| Autosave / crash recovery | Auto Save files openable; corruption-prevention guidance | legacy | Authoring | |
| Filename/path 255-char limit removal | Long path support | 2025.2 | Authoring | |
| Python Script API | PATTERN/FABRIC/IMPORT/EXPORT/UTILITY APIs; in-app editor; plug-in registration; menu renamed Plugins (2025.2) | ~2024.2 | Ecosystem | developer.marvelousdesigner.com |
| REST API | Remote/programmatic control (headless-style automation) | ~2024–2025 | Ecosystem | Closest to headless mode; no true CLI (UNVERIFIED) |
| Silent Install | No-GUI installer | 12.2 | Ecosystem | |
| Linux support | Linux setup (network license) | ~2024 | Ecosystem | Plus Win/Mac (M1 native 11) |
| Tablet pen pressure/integration | Pen pressure in brushes | MD10 | Authoring | |
| Multi-language | UI language switch | 7.1.1 | Authoring | |
| Merge Items (library) | Merge library items | 11 | Authoring | Obscure |

## Obscure-but-workflow-critical shortlist

1. **Directional notches + Reverse Sewing** — sewing direction semantics; wrong parity flips patterns in 3D.
2. **Set Sublayer on seams** — ordering for self-folding sewing (seam allowances/pleats); silent stability lever.
3. **Seam Tension (Ease/Stretch, strength, ratio)** — per-seam tension model, 2025.0; almost never demoed.
4. **Fold Arrangement (incl. symmetric folding)** — pre-sim folding of collars/plackets; without it layered garments explode.
5. **Layer Clone (Over/Under)** — instance-sewn duplicate layers; the canonical padded-jacket/lining workflow.
6. **Use Layer + Layer-Based Collision Detection + Show Layer Depth** — the whole multi-layer dressing system.
7. **Superimpose / Smart Arrangement** — sewn-pattern placement helpers everyone uses, no one documents.
8. **Press tool + Turned sewing type** — flattening sewn double plies; distinct from Fold Angle.
9. **Match Up / Match 2D Pattern Measurements** — numeric seam-length reconciliation between patterns.
10. **Unfold Symmetric Editing (with Sewing)** — persistent half-symmetry with linked sewing, not just mirror-copy.
11. **Linked Editing (instance patterns with interval array)** — live-linked pattern instances.
12. **Elastic Ratio/Strength/Entire-Length on lines** — shirring engine; segment-total vs per-segment semantics.
13. **Steam brush + Steam Eraser (Add/Remove modes)** — localized shrinkage painting.
14. **Bond / Skive** — area stiffen/soften with fabric-preset physics; leather & tailoring workflows.
15. **Seam Taping with Extend modes and Fusible/Reinforcement physical presets** — interfacing simulation.
16. **Pin on segment/internal line + Attach Pin to Avatar + Line Tack** — the full pin/tack taxonomy beyond simple pins.
17. **Fasten/Unfasten Button through Buttonhole** — physically simulated buttoning incl. moving fastened pairs.
18. **Roll Up (2D) and Roll Up Selected Area (3D)** — two different hem-rolling tools.
19. **Re-Target Draping / Re-Drape 3D Arrangement** — re-drape on new avatar WITHOUT resizing (vs Auto Fitting).
20. **Maintain Topology option in Auto Fitting** — mesh-stable refits for game pipelines (2025.1).
21. **Flatten / Flattening as Straight Line / merged multi-area flatten** — 3D-surface-to-2D patternmaking loop.
22. **Back/Side UV Expansion for thick meshes** — editable side/back UVs; export-quality blocker if missing.
23. **Weld/Unweld + Merge Vertex by Proximity + Unified UV coordinates at export** — export mesh topology contracts.
24. **Turntable images + XML metadata emitted on meta-data save** — automated asset-pipeline hook (v3.1.38).
25. **Simulation Properties file (.smp) with CG iteration/residual, substeps, per-collision-pair toggles** — full solver parameterization saved/restorable; plus the granular native asset formats (.arr/.pan/.mea/.cmp/.cmt/.ssp/.psp).

**Negative parity facts (verified):** MD has **no DXF-AAMA/ASTM**, **no grading/auto-grading**, **no seam allowance/notch/annotation tools**, **no colorways**, **no offline renderer**, **no print layout** — all CLO-only.

## Sources

- https://support.marvelousdesigner.com/hc/en-us/categories/51985515993625-Manual (full Zendesk help-center corpus: 709 articles across 52 sections retrieved via API)
- https://support.marvelousdesigner.com/api/v2/help_center/en-us/articles.json (article bodies)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358120307353 (MD 2025.0/2025.1/2025.2 New Feature List)
- https://support.marvelousdesigner.com/hc/en-us/articles/55837641308313 (MD 2026.0 New Feature List)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358170563481 (MD 2024.x New Feature Lists)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358241113625 (MD 12/12.1/12.2 New Feature List)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358258177817 (MD 11 New Feature List)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358356324505 (MD 10 New Feature List)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358199862553 (Compatible File Format)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358169252633 (Marvelous Designer File Format)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358306734361 (MD vs CLO differences — DXF/grading/tech-pack CLO-only)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358236635673 (Illustrator curves not importable)
- https://developer.marvelousdesigner.com/index.html (Python/REST API scope)
- https://www.cgchannel.com/2025/11/clo-virtual-fashion-releases-marvelous-designer-2025-2/
- https://www.cgchannel.com/2026/04/clo-virtual-fashion-releases-marvelous-designer-2026-0/
- https://www.cgchannel.com/2025/04/clo-virtual-fashion-releases-marvelous-designer-2025-0/
- https://digitalproduction.com/2025/08/21/marvelous-designer-2025-1-draw-wash-repeat/
- https://support.marvelousdesigner.com/hc/en-us/articles/47358145573401 (MD+Unreal Tips&Tricks — LiveSync, USD Chaos Cloth)
- https://www.marvelousdesigner.com/product/newfeature
