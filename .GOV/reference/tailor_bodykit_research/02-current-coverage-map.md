---
file_id: tailor-bodykit-research-02-current-coverage-map
topic_id: T-BK-COVERAGE
title: "WP-KERNEL-010 Current-Coverage Map: 448 MTs + Master Spec Section 13 + Prior Parity Review (pre-second-pass baseline)"
status: non_normative_research
normative_status: non_normative_context_only
research_lane: local repo-analysis sub-agent (read-only), session 2026-07-08
sources_read: "_MT_INDEX.json (448/448), _PARITY_REVIEW.json (complete), spec-modules/13-tailor-cloth-garment-engine.md v02.197 (lines 1-9925 complete), cloth_engine_research/02-md-feature-map.md (complete)"
purpose: "Definitive baseline of what WP-KERNEL-010 Cloth currently covers, for (a) the second-pass external MD parity gap analysis and (b) BodyKit fold-in attach-point design."
updated_at: "2026-07-08"
---

# WP-KERNEL-010 Current-Coverage Map

## MT group census

| Group | Count | Core ids (Section-13 block, MT-001..332) | Parity ids (MT-333..448) | What the parity block added |
|---|---|---|---|---|
| SolverCore | 44 | MT-001..MT-040 | MT-333..MT-336 | SeamKind::Fold dihedral behavior; `particle_distance_mm` in SimRunParams; SoftBodySpec (soft-body props from OBJ/FBX); Pin/Freeze constraints |
| KernelIntegration | 40 | MT-041..MT-080 | — (none) | — |
| Collision | 35 | MT-081..MT-114 | MT-337 | Precise self-collision mode (doubled narrow-phase iterations) |
| GarmentAuthoring | 51 | MT-115..MT-140 | MT-338..MT-362 | Primitive shapes; edge split/curvature; reverse seam; 3D-sketch-to-pattern; panel transforms; auto-seal; InternalLine + fold-line constraints; panel snapshots; vertex-level edits; Free/1:N/partial sewing; topstitch 3D geometry; 3D viewport sewing; auto-sew; sculpt; garment library/blocks/browsers; per-panel draft snapshots; GarmentParticleView |
| ValidationHBR | 26 | MT-141..MT-166 | — (none) | — |
| TrimRigid | 34 | MT-167..MT-190 | MT-363..MT-372 | GPU-resident rigid trim sim; Glue/Tack placement handlers; trim OBJ/FBX import; tack.wgsl + trim_contact.wgsl; pattern-to-rigid conversion + Solidify; trim library seed + register tool; ElasticInsert/BindingTape; topstitch-as-mesh |
| Fabric | 26 | MT-191..MT-212 | MT-373..MT-376 | Satin/Linen/Spandex presets; washed-denim variants; tailor_material_assignments per-panel override; air_drag as first-class field |
| UvTexture | 34 | MT-213..MT-234 | MT-377..MT-388 | UV editor panel; per-garment atlas isolation; UV coverage check; UV-space texture bake; 2D graphic layer editor; 5 blend modes; recolor colorway; fur stub (deferred); all-quad retopo; topstitch polygon optimization + smooth corners; generate_fabric_texture (AI) |
| RenderViewportExport | 28 | MT-235..MT-256 | MT-389..MT-394 | Isolate-selection toggle; Toon pipeline (feature-gated); Schematic render; turntable capture; MatCap; morph/blend-shape animation export |
| ModelFirstApi | 34 | MT-257..MT-278 | MT-395..MT-406 | edit_seam/reverse_seam/delete_seam tools; FabricMaterialHint; pose-generation tools; GarmentCodeRcDecoder (76-param, 8 categories); NGL planner; image-input authoring; DiffXPBD estimate_fabric_params; partial-pattern completion; garment foundation-model adapter registry |
| Animation | 35 | MT-279..MT-298 | MT-407..MT-421 | Keyframeable Trim Weight + Tack Strength; positioned WindActor; FBX motion auto-conversion; 6 MD-parity keyframeable fabric slots; timeline markers; recording mode; Alembic morph channels; trim-in-timeline wiring; reset_to_rest; frame-range export; FBX auto-key bake (x2: MT-419+MT-421); pin/freeze animation tools |
| AutoFit | 31 | MT-299..MT-316 | MT-422..MT-434 | Avatar friction override; parameterized built-in avatar library; IK pose editing; auto joint-mapping (Daz/Mixamo/MetaHuman/CC); custom avatar import; blend-shape import + warm-start refit; morph-delta import; MetaHuman DNA import; glTF/VRM avatar import; Preserve Topology refit; fit-property controls + Stress/Strain/Pressure heatmaps; fit-map projection to 2D; fabric-aware strain maps |
| ProductionBridge | 30 | MT-317..MT-332 | MT-435..MT-448 | ExportPreset records; .hgz round-trip archive; OBJ+mtl / FBX / glTF+VRM static importers; Alembic import shim; UE Chaos Cloth USD attributes; MDD/PC2 export; DXF R12 export; MetaHuman DNA import + export package; LiveSync/EveryWear deferred stub + watched-folder push; .hsk-garment project archive; export-config presets |

Total: 332 core + 116 parity = 448.

## Feature coverage inventory

**Pattern authoring (2D)** — 30 MTs
- Canonical GarmentSpec type (cm, gather_ratio, typed EdgeShape) [MT-115]; EdgeShape enum [MT-116]; Panel type (closed outline + grain + fabric ref) [MT-117]
- 2D-pattern → 3D initial arrangement [MT-119]; avatar-relative placement anchors (front/back/sleeve) [MT-120]
- Dart handling [MT-122]; pleat handling (parallel folds) [MT-123]; panel symmetry/mirror [MT-125]
- Parametric LLM-emittable garment definition [MT-126]; GarmentSpec → triangle tessellation [MT-127]; per-panel mesh resolution (coarse/fine) [MT-128]
- Notches/alignment marks [MT-131]; internal guide lines (topstitch/foldlines → UV) [MT-132]
- Collaborative panel editing via kernel CRDT [MT-135]; GarmentSpec versioning + forward migration [MT-139]; authoring integration test [MT-140]
- Primitive panel constructors Rectangle/Circle/Polygon [MT-338]; edge split at parametric t [MT-339]; curvature editing / Bezier handle drag [MT-340]
- 3D-sketch-to-pattern (3D Pen: draw on avatar, SDF projection, ARAP flatten) [MT-342]; affine panel transforms scale/rotate/translate [MT-343]; auto_seal_panel [MT-344]
- InternalLine authoring (FoldLine|PleatLine|DartLine|StressSeam|Baseline) [MT-345]; fold-seam-line dihedral constraint generation [MT-346]
- Per-panel snapshot/archive API [MT-347]; per-panel draft snapshots PSN- [MT-361]
- Bidirectional 2D↔3D arc-length feedback (simulated edge lengths written back to panels) [MT-348]; fine-grained move_vertex/move_edge_segment CRDT edits [MT-349]
- Post-simulation garment sculpt (push/pull/smooth/relax + re-unfurl + delta layer) [MT-356]; GarmentParticleView 3D-first editing projection [MT-362]

**Sewing / seams** — 17 MTs
- Seam/gather XPBD constraint with gather_ratio [MT-018]; GpuSeamConstraint struct [MT-026]; seam WGSL solve pass [MT-031]
- Seam type (edges of two panels + gather_ratio + allowance) [MT-118]; sewing resolution (pull edges together over substeps) [MT-121]
- PANEL_CLOSED validation [MT-133]; SEAM_LENGTH_MATCH validation [MT-134]
- SeamKind::Fold solver behavior (fold_angle_deg dihedral) [MT-333]; reverse_seam_direction [MT-341]
- Free Sewing (partial-edge [0,1] sub-range + numeric length override) [MT-350]; 1:N segment sewing SeamGroup with proportional gather [MT-351]; 1:N + partial-range combination [MT-352]
- 3D viewport seam authoring mode (Segment/Free/1:N/M:N) [MT-354]; auto_sew proposal batch [MT-355]
- edit_seam MCP (gather_ratio + range mutation) [MT-395]; reverse_seam MCP [MT-396]; delete_seam MCP (CRDT tombstone) [MT-397]

**Simulation engine (XPBD solver core)** — 42 MTs
- tailor-solver crate scaffold, deps behind cloth-solver feature [MT-001, MT-002]; ClothParticle SoA buffers [MT-003]; GarmentMesh ingest [MT-004]; ClothConstraint enum [MT-005]
- SimRunParams / SolverResult / SolverMode(Fitting|Animation|ChebyshevGs) / MaterialFrameParams [MT-006, MT-007, MT-008, MT-009]
- XPBD predict, substep loop, lambda reset, compliance→alpha with rest-geometry normalization [MT-010..MT-013]
- Anisotropic stretch (warp/weft), shear, dihedral bending, buckling handling [MT-014..MT-017]
- Tack point constraint [MT-019]; volume/pressure constraint (inflatables) [MT-020]; velocity update + damping [MT-021]
- wgpu device init (headless-capable) [MT-022]; GPU structs GpuParticle/Stretch/Bend/SimParams [MT-023, MT-024, MT-025, MT-027]
- WGSL shaders: integrate, stretch, bend, velocity/finalize [MT-028, MT-029, MT-030, MT-032]; compile-time WGSL↔Rust binding validation [MT-033]
- Deterministic 8-pass dispatch sequence [MT-034]; greedy graph coloring + color-partition buffers + Jacobi delta accumulation (no f32 atomics) [MT-035, MT-036, MT-037]
- Determinism envelope contract (fixed budget, stable order, seeded noise, race-free) [MT-038]; MeshComparator eps 0.1 mm + envelope [MT-039]; ClothSolver trait + cross-substep determinism test [MT-040]
- particle_distance_mm mesh-resolution param (MD Particle Distance) [MT-334]; SoftBodySpec soft-body props in the same solver pass [MT-335]; PinConstraint + FreezeRegion commands [MT-336]
- reset_to_rest() re-sim without reload [MT-417]; particle pin + simulation freeze as MCP/viewport tools [MT-420]

**Fabric / material physics** — 27 MTs
- Per-panel grain line → solver material frame [MT-124]
- ClothMaterialCompliance 6-scalar orthotropic [MT-191]; normalized [0,1] FabricProperties + FabricPreset enum [MT-192]; logarithmic normalized→raw mapping [MT-193]; rest-geometry normalization (mesh-resolution-portable presets) [MT-194]
- ClothMaterialPhysics (density, friction, damping, air-drag) [MT-195]; MaterialParamsGpu UBO [MT-196]; grain-direction tagging [MT-197]; preset resolution [MT-198]
- 9 seeded system presets (cotton…rubber) [MT-199]; calibration: stretch, shear, bending, gsm→mass, friction, damping [MT-200..MT-205]
- Aerodynamic air-drag (chiffon flutter) [MT-206]; wetness/weight modifier [MT-207]; sheerness/roughness render hints [MT-208]
- MaterialKeyframe (animatable material props) [MT-209]; preset CRUD API [MT-210]; FABRIC_PROPERTIES_IN_RANGE validation [MT-211]; silk-vs-denim drape integration test [MT-212]
- Satin/Linen/Spandex preset seeds [MT-373]; washed denim variants (stone/acid/bleach) [MT-374]; tailor_material_assignments per-panel override + PanelSpec.material_preset_id [MT-375]; air_drag first-class in FabricProperties/GPU [MT-376]

**Collision** — 30 MTs
- ClothBodyProxy / CollisionCapsule / CollisionSphere types [MT-081, MT-082, MT-083]; GpuCapsule/GpuSphere [MT-084, MT-085]
- Cloth-vs-capsule, cloth-vs-sphere, cloth-vs-SDF projection [MT-086, MT-087, MT-088]
- parry3d CPU broadphase only [MT-091]
- Self-collision: spatial hash broadphase, curvature/adjacency culling, repulsion response [MT-093, MT-094, MT-095]
- Multi-layer: inner→outer ordering, interlayer minimum spacing [MT-096, MT-097]
- Coulomb cloth-body friction, self-contact friction, no-energy-injection velocity correction [MT-098, MT-099, MT-100]
- Exaggerated proportions: 3-sphere-per-side bust decomposition, sternum capsule, max-magnitude overlap resolution, under-bust double pass, crease collision ordering, SDF fallback for extreme bust/hip [MT-101..MT-106]
- Collision WGSL pass [MT-107]; per-frame collider GPU upload [MT-108]
- NO_INTERPENETRATION / SELF_INTERSECTION / INTERLAYER_SPACING validators [MT-111, MT-112, MT-113]; exaggerated-avatar drape determinism test [MT-114]
- Precise self-collision mode (SimRunParams flag) [MT-337]

**Avatar / body** — 19 MTs
- tailor_avatars migration (AVT-, measurements JSONB) [MT-047]; tailor_body_proxies migration (BPX-, capsule/sphere/SDF) [MT-048]
- SDF generation/bake from avatar body mesh [MT-089]; automatic SDF fallback trigger (extreme proportions) [MT-090]; canonical capsule/sphere decomposition from parry [MT-092]
- Bind ClothBodyProxy from authority into sim run [MT-109]; avatar/proxy lifecycle events (Created/Updated/MeasurementsExtracted) [MT-110]
- Measurement estimation tool (description → AVT- measurements) [MT-270]
- Model-steerable pose generation (NL/reference image → PoseEditRequest → IK) [MT-399]; generate_avatar_pose MCP tool (PoseSpec, TailorAvatarPoseGenerated) [MT-404]
- Per-avatar body_friction_override [MT-422]; parameterized built-in avatar library (archetypes + anthropometric sliders → AVT-+BPX- rows) [MT-423]
- IK-mode avatar pose editing (FABRIK/2-bone over tailor_avatars skeleton) [MT-424]; auto joint-mapping from Daz/Mixamo/MetaHuman/CC rigs [MT-425]
- Custom avatar import pipeline OBJ/FBX (AVT- row + 25 measurements + capsule chain) [MT-426]; blend-shape avatar import + BlendShapeRefitRequest warm-start [MT-427]; OBJ morph-delta import → secondary BPX- [MT-428]; MetaHuman .dna import [MT-429]; glTF 2.0 + VRM 1.0 avatar import [MT-430]

**Fitting / grading / refit** — 23 MTs
- Parametric sizing from avatar measurements (auto-grade) [MT-130]
- Morph correspondence source→target [MT-299]; body-shape transfer (garment cage prior) [MT-300]; re-grade panels [MT-301]; re-drape solver run [MT-302]
- UV/texture preservation across refit [MT-303]; trim re-placement across refit [MT-304]
- Fit tension analysis heatmap [MT-305]; auto-ease adjustment [MT-306]; standard size-run S/M/L/XL [MT-307]; morph-target interpolation (blend between avatars) [MT-308]
- Fit quality score [MT-309]; refit caching [MT-310]; tailor_garment_fits migration [MT-311]; refit pipeline orchestration [MT-312]; batch refit as parallel jobs [MT-313]
- Refit determinism proof [MT-314]; FIT_VALID validator [MT-315]; 3-body refit integration test [MT-316]
- Preserve Topology refit option (RefitMode topology_lock) [MT-431]; per-garment fit properties + real-time Stress/Strain/Pressure viewport heatmaps [MT-432]; fit-map projection onto 2D panels [MT-433]; fabric-aware warp/weft strain channels [MT-434]

**Trims / hardware** — 34 MTs
- RigidProxy wrapper [MT-167]; cloth-rigid tack coupling [MT-168]; parry3d rigid step interleave (CPU) [MT-169]; two-way coupling [MT-170]
- Button + thread anchor [MT-171]; buttonhole slot constraint [MT-172]; zipper (tapes + slider t) [MT-173]; zipper teeth progressive coupling [MT-174]; two-way zipper [MT-175]
- Eyelet [MT-176]; lacing (1D strand through eyelets) [MT-177]; trim tack persistence [MT-178]; snap/hook fasteners [MT-179]; belt/strap chained rigid links [MT-180]; buckle [MT-181]
- Trim placement (anchor + orientation) [MT-182]; mass/inertia [MT-183]; trim-vs-cloth collision [MT-184]; trim-vs-body collision [MT-185]; closure open/close keyframe hook [MT-186]; trim render material refs [MT-187]
- tailor_trims migration [MT-188]; TRIM_ATTACHED validator [MT-189]; zip+button determinism test [MT-190]
- place_trim MCP tool [MT-263]
- GPU-resident trim sim (GpuTrimBody + rigid_integrate.wgsl) [MT-363]; Glue/Tack placement handler (TackDefinitionV1) [MT-364]; multi-point tack arrays + UI/MCP surface [MT-365]
- Trim OBJ/FBX import (TrimMeshV1, inertia, category) [MT-366]; tack.wgsl + trim_contact.wgsl passes [MT-367]; pattern-to-rigid conversion (TR-8, Solidify, MCP tool) [MT-368]
- Default trim library seed (10 categories) [MT-369]; register_trim_as_library_item [MT-370]; ElasticInsert + BindingTape edge trims [MT-371]

**UV / texture** — 30 MTs
- Panels-are-UV islands [MT-213]; [0,1] normalization preserving scale [MT-214]; atlas packing with padding [MT-215]; seam-aware island boundaries [MT-216]; grain-aligned V-axis [MT-217]; consistent texel density [MT-218]
- PBR slot set [MT-219]; texture authority migrations [MT-220]
- Albedo from preset color+weave [MT-221]; normal map from weave [MT-222]; roughness from sheerness hints [MT-223]; tileable base texture [MT-224]; positioned prints/graphics [MT-225]; topstitch/foldlines into texture [MT-226]; trim texture slots [MT-227]; procedural weave generator [MT-228]
- glTF/USD material export refs [MT-229]; CRDT UV/texture edits [MT-230]; model-first texture API [MT-231]; preview render hook [MT-232]; UV_VALID validator [MT-233]; UV integration test [MT-234]
- UV editor viewport panel [MT-377]; per-garment atlas isolation [MT-378]; UV coverage completeness (back/side/sleeve/lining) blocking check [MT-379]; UV-space texture bake (albedo/normal/AO) [MT-380]
- 2D graphic layer placement editor [MT-381]; 5 GraphicBlendMode variants [MT-382]; recolor/colorway export pass [MT-383]; fur strand material deferred stub [MT-384]

**Retopo / mesh & topstitch geometry** — 5 MTs
- Topstitch as extruded 3D ribbon in export [MT-353]; topstitch-as-mesh stitch loops + polygon merge [MT-372]
- Post-sim all-quad re-topology preserving UV + grain [MT-385]; topstitch polygon-count optimization [MT-386]; smooth-corner arc bevels for topstitch [MT-387]

**Animation** — 33 MTs
- Timeline type [MT-279]; Keyframe + Track<T> [MT-280]; keyframeable-property registry [MT-281]; step/linear/bezier interpolation [MT-282]
- Avatar pose animation driving body proxy [MT-283]; BVH/glTF skeletal pose import [MT-284]; keyframeable wind field [MT-285]; camera track (turntable/beauty) [MT-286]
- Material keyframes in timeline [MT-287]; closure (zipper t) animation [MT-288]; sim-over-timeline continuity [MT-289]; per-frame mesh caching [MT-290]; adaptive sub-frame substepping [MT-291]; motion-read realism gate [MT-292]
- tailor_animations migration [MT-293]; animation authoring API [MT-294]; scrub/preview (play/pause/seek) [MT-295]; export wiring OBJ-seq/Alembic [MT-296]; ANIMATION_STABLE validator [MT-297]; walk-cycle determinism test [MT-298]
- Keyframeable Trim Weight [MT-407]; keyframeable Tack Strength + MCP tool [MT-408]; positioned WindActor (cone, turbulence, falloff) [MT-409]; FBX motion auto-conversion → AnimationDraft [MT-410]
- Six MD-parity keyframeable fabric slots (Shrink weft/warp, Solidify, Pressure, Tack Strength, Trim Weight) [MT-411]; per-tack TackStrength Track<f32> [MT-412]; named timeline markers [MT-413]; recording session mode (live keyframe capture) [MT-414]
- Alembic morph-weight channels in/out [MT-415]; trim rigid step wired into timeline loop [MT-416]; frame-range export params + marker anchors [MT-418]; FBX joint auto-key bake [MT-419, MT-421]

**Rendering / viewport** — 17 MTs
- Throwaway Bevy testbed (dev-only, out of product scope) [MT-235]; Handshake-native wgpu viewport [MT-236]; PBR cloth render [MT-237]; real-time streaming update [MT-238]; orbit/pan/zoom [MT-239]; embedded GUI viewport panel [MT-240]
- Model-readable capture API [MT-241]; headless offscreen capture [MT-242]; console/error scan per capture [MT-243]; capture matrix incl. edge states [MT-244]; visual-debug overlays (constraints/contacts/pins) [MT-245]; wireframe/UV-check/normals debug modes [MT-246]
- Isolate-selection toggle [MT-389]; Toon pipeline (bands/outline/rim/MatCap, feature-gated) [MT-390]; Schematic technical-flat render mode [MT-391]; 360° turntable capture sequence [MT-392]; MatCap mode [MT-393]

**Import / export & production bridge** — 38 MTs
- GarmentCode-style parametric import/export [MT-129]
- glTF export (+skeleton) [MT-249]; USD export (UsdPreviewSurface) [MT-250]; OBJ-sequence export [MT-251]; Alembic point-cache export [MT-252]; Blender bridge (watched folder + manifest) [MT-253]; UE bridge (USD/FBX) [MT-254]; EXPORT_VALID [MT-255]; render/export integration test [MT-256]
- glTF morph-target + Alembic blend-shape animation weights export [MT-394]
- Production export package definition [MT-317]; export as Handshake JOB [MT-318]; Flight Recorder export lifecycle [MT-319]; typed manifest + hashes [MT-320]; headless-parallel export [MT-321]; process ownership/reclaim [MT-322]
- Blender import .py generator [MT-323]; UE import (USD/Datasmith + material map) [MT-324]; render-pipeline handoff plate [MT-325]; asset versioning stamps [MT-326]; retry/resume [MT-327]; operator progress reporting [MT-328]; batch wardrobe export [MT-329]; EXPORT_PACKAGE_COMPLETE [MT-330]; export ModelManual entries [MT-331]; production-bridge integration test [MT-332]
- ExportPreset records EPR- [MT-435]; OBJ+.mtl importer (avatar/prop/trim) [MT-437]; FBX static mesh import (feature-gated) [MT-438]; glTF/VRM static avatar import [MT-439]; Alembic import shim (Blender-headless conversion) [MT-440]
- USD Chaos Cloth sim-property embedding for UE5 [MT-441]; MDD + PC2 cache export [MT-442]; DXF R12 pattern export (allowances + grain lines) [MT-443]; MetaHuman DNA import via C library [MT-444]; MetaHuman export package (auto-resize binding) [MT-445]; LiveSync/EveryWear deferred stub + watched-folder UE auto-push (MOAT-7 note) [MT-446]; export-config preset system [MT-448]

**Library / modular / asset management** — 7 MTs
- Wardrobe membership [MT-078]; starter template library (tee/skirt/dress/trousers/bodice) [MT-137]
- GarmentLibraryEntry + Category/Style/Group hierarchy [MT-357]; GarmentBlock + BlockBox interchange contract [MT-358]; unified Library browser panel (garments/fabrics/trims/templates) [MT-359]
- .hgz round-trip garment archive [MT-436]; .hsk-garment project archive (session restore/team sharing) [MT-447]

**AI / model-first API** — 29 MTs
- ClothModelAdapter typed surface [MT-257]; author_garment [MT-258]; edit_panel [MT-259]; run_simulation [MT-260]; fetch_simulation_receipt [MT-261]; assign_material [MT-262]; capture_view [MT-264]; promote_garment [MT-265]; refit_garment [MT-266]
- SimulationReceipt schema [MT-267]; self-correction loop helper [MT-268]; fabric estimation (NL→preset) [MT-269]
- Typed routing fields on tool messages [MT-271]; parallel swarm authoring proof [MT-272]; per-agent attribution [MT-273]; cancellation/restart [MT-274]; typed tool error taxonomy [MT-275]; Tailor ContextBundle assembly [MT-276]; ModelManual entries [MT-277]; model-first integration test [MT-278]
- Model-first garment emit API (JSON GarmentSpec) [MT-136]; generate_fabric_texture (diffusion) [MT-388]
- FabricMaterialHint from natural_description [MT-398]; GarmentCodeRcDecoder (76 params, 8 category factories) [MT-400]; NGL structured planning tool [MT-401]; image-input garment authoring (VLM) [MT-402]; DiffXPBD estimate_fabric_params [MT-403]; partial-pattern diffusion completion [MT-405]; GarmentModelAdapter registry (AIpparel-style foundation models) [MT-406]

**Kernel integration** — 38 MTs
- Module scaffold [MT-041]; solver_binding sole import point [MT-042]; guard_authority_write tripwire [MT-043]
- Migrations: garments/panels/seams [MT-044, MT-045, MT-046]; sim runs [MT-049]; material presets [MT-050]; remaining 16-table set [MT-051]; prefixed UUID-v7 id minting [MT-052]
- Tailor* KernelEventType variants + wire strings/families [MT-053, MT-054]; sim-run event emission [MT-055]; EventLedger replay proof [MT-056]
- CRDT binding + persistence/replay/promote proof [MT-057, MT-058]; PromotionGate integration [MT-059]; fail-closed negative guard [MT-060]
- Sandbox dispatch [MT-061]; ContextBundle→ModelAdapter→ToolGate→ArtifactStore wire [MT-062]; lane normalization to SessionRun [MT-063]; AI Job Model execution [MT-064]; scheduler leases/backpressure/cancel/recovery [MT-065]
- Flight Recorder lifecycle [MT-066]; headless-parallel safety proof [MT-067]; REST API surface [MT-068]; MCP tool registration [MT-069]; ModelManual update [MT-070]
- TraceProjection [MT-071]; FEMS records [MT-072]; GarmentSpec persistence [MT-073]; SimulationReceipt feedback [MT-074]; ValidationDescriptor dispatch [MT-075]; fabric/measurement estimation API [MT-076]; garment version chain [MT-077]; HBR acceptance-matrix hook [MT-079]; end-to-end lifecycle test [MT-080]; draft→sandbox handoff [MT-138]

**Validation / HBR** — 26 MTs
- ValidationDescriptor type [MT-141]; ~35-check catalog registry [MT-142]; validation runner [MT-143]; result persistence [MT-144]; Blocking/Advisory gating [MT-145]
- DRAPE_CONVERGED [MT-146]; NO_INTERPENETRATION+SELF_INTERSECTION registration [MT-147]; SEAM_CLOSED [MT-148]; STRETCH_WITHIN_LIMITS [MT-149]; FABRIC_PRESET_VALID [MT-150]; MESH_MANIFOLD [MT-151]; GRAIN_CONSISTENT [MT-152]; UV_VALID hook [MT-153]; DETERMINISM_ENVELOPE [MT-154]
- MeshComparator as sole promotion equivalence [MT-155]; promotion decision [MT-156]; typed validation receipt [MT-157]
- HBR matrix hydration [MT-158]; HBR-INT/SWARM/VIS/QUIET/MAN evidence binding [MT-159..MT-163]; hbr-matrix-check in gov-check [MT-164]; HBR_VIOLATION runtime receipts [MT-165]; validation+HBR integration test [MT-166]

**UX / GUI & parallel-model navigation** — 3 dedicated MTs (+ GUI surfaces in other domains)
- Backend typed-JSON navigation routes by id [MT-247]; stable element ids / test hooks on Tailor controls [MT-248]; hierarchical Scene Object Browser (garments/avatars/trims/layers, visibility toggles) [MT-360]
- (GUI surfaces also live in other domains: viewport panel MT-240, UV editor MT-377, graphic layer editor MT-381, library browser MT-359, heatmap layers MT-432.)

## Spec Section 13 heading tree + normative scope

- **13. Tailor — Cloth/Garment Engine [TAI-SECTION-001]** — product LAW; 13.14 wins all contract conflicts; research package non-normative.
- **13.1 Overview, Scope, Model-First Differentiator**
  - §1 What Tailor Is — two compile units (`handshake_core::tailor` + standalone `tailor-solver`); Cloth*/Tailor* dual terminology.
  - §2.1 In Scope — 15 capability groups (pattern, seams M:N, anisotropic fabric, XPBD GPU solver, collision, avatar binding, keyframeable props, trims, UV-from-pattern, authority tables, interop, LLM lane, sandbox/validation ~35 checks, WGSL, validation catalog). **Feature ceiling = Marvelous Designer 2026.0 per 02-md-feature-map; all 8 D4 moats MUST be addressed; MOAT-7 MAY defer post-v1 but MUST be noted in T-PIPELINE-INTEROP.**
  - §2.2 Out of Scope — SQLite; runtime dependency on MD/CLO3D; Bevy/Avian in production; provider-specific LLM deps; Tauri commands last; numbered migrations.
  - §3 Model-First Differentiator — TAI-OVR-001..007: TailorModelAdapter; GarmentSpec single shared type (cm, [0,1] fabric, gather_ratio (0,20]); sandbox→validate→promote non-bypassable; SimulationReceipt with suggested_fix; MeshComparator not hashes; full EventLedger replayability.
  - §4 Kernel framing — TAI-OVR-008..014: atelier pattern; no wgpu in handshake_core; workspace crate; kernel-baseline prerequisite gate; canonical events; TEXT PKs; event_ledger_event_id on every row.
  - §5 Build order — solver prototype → kernel gate → kernel module → Tauri last.
  - §6 Research provenance (non-normative).
- **13.2 Architecture** (ARCH-001..042) — crate split + one-way dependency; ClothSolver trait exact signature (load_garment/simulate/update_params/unload/last_content_hash); SolverResult fields; crate layout + 8 WGSL shader files; pinned deps (wgpu 29, parry3d 0.17, wgsl_to_wgpu, schemars); kernel module layout (event_family.rs, schemas.rs, garment.rs, solver_binding.rs, simulation, material, validation, crdt_bridge, avatar.rs, wardrobe, model_adapter, storage_glue, api); canonical event/schema constants; GarmentSpec rules (cm suffixes, gather_ratio, EdgeShape enum); TailorSandboxAdapter (process tier, fs-only caps, block_on bridge); determinism (per-backend content_hash test; MeshComparator ε=0.1, envelope ε=1.0 for animated); ~35-check descriptor; dated migrations; PK prefix table (16 tables); extension points in existing files; cloth-solver Cargo feature; required tests; prohibition summary table.
- **13.3 XPBD Solver Core (WGSL/wgpu)** — §.1 scope; §.2 crate isolation + deps (`multigrid`, `cuda` features); §.3 normative XPBD substep loop (lambda reset, compliance 1e-9..1e-3, substeps-over-iterations; Fitting 10×5, Animation 4×3 defaults); §.4 constraints: anisotropic stretch/shear (Green strain, grain_cos), dihedral bending with buckling ramp, seam constraints (gather_ratio rest-length scaling, M:N via resampling), tack point constraints with per-tack compliance, volume/pressure in GpuSimParams; §.5 SimRunParams/SolverMode/MaterialFrameParams (solidify, pressure, shrink u/v, tack_compliance)/SolverResult + GpuSimParams field list (incl. wind, noise seed); §.6 Gauss-Seidel standard + ChebyshevGs (MGPBD) upgrade behind `multigrid`; §.7 per-backend determinism (4 requirements: fixed budget, stable coloring, seeded noise, race-free), cross-backend NOT asserted, MeshComparator contract; §.8 wgpu backends (Vulkan/DX12/Metal, CPU fallback), workgroup 64, std430 GPU structs, 8-pass sequence table, 32 capsule/16 sphere caps; §.9 graph coloring; §.10 ClothSolver trait; §.11 event emission boundary (solver never emits); §.12 owned checks (MESH_NOT_EMPTY, NO_DEGENERATE_TRIS, SEAMS_CLOSED, NO_INTERPENETRATION, SELF_INTERSECTION, DRAPE_CONVERGED, PRESET_*); §.13 superseded names.
- **13.4 Collision** — Body-proxy authority (COL-BODY-001..005: ClothBodyProxy mm, GpuCapsule/Sphere caps 32/16, tailor_body_proxies DDL with mode/breast_proxy_mode/sdf_artifact_ref/joint_hierarchy_json, avatars-before-proxies FK); capsule+sphere primary GPU mode (pre-constraint pass, clamped closest-point pushout); SDF secondary mode (64³ texture3d, re-bake on pose change, gradient pushout, auto-select when >6 breast spheres & >10 capsules, jump-flooding bake); parry CPU pre-processing (build_avatar_proxy, V-HACD breast decomposition, BVH validation culling); self-collision curvature culling (Laplace-Beltrami, 40–70% reduction) + spatial hash (cell 2r, 27-neighborhood, fixed-point i32 atomics Jacobi, friction damping); multi-layer (layer_index inner-first, asymmetric outer-stays-outside, rest distance t_inner+t_outer, INTERLAYER_SPACING blocking); exaggerated proportions (cup G–K+: max-magnitude correction, min inter-sphere spacing, doubled pass under breast_proxy_mode, breast-bone name tagging, 3-sphere+sternum canonical decomposition); events; schema ids; validation (AVATAR_BINDING, NO_INTERPENETRATION final-frame only, INTERLAYER_SPACING, SELF_INTERSECTION); sandbox binding; model lane `suggest_collision_proxy` + `garment_proxy_suggestion` column; migrations.
- **13.5 Fabric & Material Models** — orthotropic 6-scalar ClothMaterialCompliance mandatory (no scalar-multiply shortcut); ClothMaterialPhysics (kg/m², thickness m, friction, self_friction, damping, air_drag, pressure, solidify, shrinkage); MaterialParamsGpu 16-float UBO; grain tagging + isotropic fallback rule; MaterialKeyframe; two-layer design (normalized LLM surface vs raw solver) with single decode point; FabricProperties canonical type + 12-variant FabricPreset enum; **logarithmic mapping mandatory**; rest-geometry normalization mandatory; tailor_material_presets DDL + **9 seeded presets with full calibration JSON (cotton, silk, denim, leather, jersey, wool, chiffon, canvas, rubber)**; per-panel preset override; preset events; sandbox drape test spec (0.5 m² panel, 1 s, 30 substeps) + PRESET_* checks + self-correction receipt; MCP tools (fabric_preset_create/fork/list, panel_assign_material); ContextBundle fabric hint; CRDT LWW per property; invariants (density unit boundary, **buckling nonlinear model post-MVP**, no trim stiffness on FabricMaterial); risks.
- **13.6 Garment Authoring** — GarmentSpec canonical (GAR-001..006, 13-variant GarmentType); units contract (cm everywhere, mm in proxies, no normalized coords in authority); PanelSpec (CCW vertices, EdgeSpec, placement Transform3D, grain_angle_deg, material_preset_id, min area 1 cm²); EdgeShape contract (Straight/Quadratic/Cubic/Arc); SeamSpec (SeamKind Join|Fold|Tack; SeamEndpoint with partial range; gather_ratio (0,20]; M:N by resampling); DartSpec/PleatSpec (knife|box|accordion); FabricProperties; AvatarBinding + BodyMeasurements (5-field cm); pattern-to-mesh pipeline (arc-length sampling, spade CDT, placement transform, UV-from-2D); **three-tier LLM authoring** (Tier 1 GarmentCodeRC 76-param, Tier 2 direct panel JSON, Tier 3 program synthesis; TailorAuthoringOutput enum); author_garment tool contract; GarmentCode interop (round-trip, RC decoder 8 categories, curvature mapping); CRDT (LWW per vertex, seam set-union + tombstones, ai_edit_proposal for models; **GAR-CRDT-004: bidirectional 2D→3D loop deferred, v1 unidirectional**); lifecycle draft→…→promoted; tables; events; validation tables (fast/mesh/post); module layouts.
- **13.7 Auto-Fit & Retargeting** — §7.1 avatar+proxy authority (avatar1_2d_derived bridge; mm; 25-field `extract_measurements` with manifold check; measurement events); §7.2 exactly three RefitModes (RedrapeOnly, ScaleAndRedrape with EaseOverrideMap, OptimizePatterns differentiable DressAnyone loss ≥1800 s); §7.3 measurement-ratio panel grading (compute_panel_scales; seam-ratio invariance; TailorRefitPatternScaled); §7.4 three-pass progressive drape (gravity-free stitch 50 substeps → stiffened ×10 → full physics, 2400-frame cap, 300 s timeout); §7.5 ARAP re-unfurl with boundary pins + graphic-anchor pinning (30% area-change advisory); §7.6 refit validation gate (REFIT_INTERSECTION_FREE, REFIT_SEAM_CLOSURE <1%, UV_VALIDITY, MESH_TOPOLOGY, REFIT_CONVERGED); §7.7 blend-shape incremental refit (blend_t, warm-start); §7.8 sandbox adapter + tailor_refit_runs DDL; §7.9 refit events; §7.10 competing refit proposals as CRDT drafts, operator selects; §7.11 refit_garment MCP (RefitRequest schema, measurement maps, refit_intent, ChatGarment 4-descriptor material re-estimation); §7.12 risks (non-humanoid convergence, multi-layer sequential inner→outer, ARAP divergence lock_graphics_layer, no auto proxy for non-humanoids); §7.13 reuse moat; §7.14 provenance.
- **13.8 Trims & Cloth-Rigid Coupling** — TR-1 single mixed cloth-rigid XPBD substep loop; trim/tack/strength/kinematic/stiffness definitions; TR-2 GpuTrimBody (128 B)/GpuTackConstraint (32 B)/GpuCordParticle/Segment + inertia tensor init; TR-3 substep pass order incl. tack + trim_contact; quaternion predict/velocity; TR-4 ball-joint tack pass (Müller-Macklin SCA 2020, strength early-out, dual-node coloring); TR-5 one-sided trim-cloth contact (world-space triangle rebuild, brute force <200 tris else BVH; **two-way contact deferred post-MVP**); TR-6 authority: tailor_trims (12-category CHECK, mesh/inertia/tack_anchor JSON, is_library_item, converted_from_panel_id), tailor_trim_placements (tacks_json TackDefinitionV1; Glue = 1 tack, Tack = ≥2), tailor_zippers (two-way sliders, active_mask per tooth), tailor_lacings (eyelet sequence, straight|criss-cross); TR-7 stiffness [0,1000] (1000 = kinematic), mass, keyframeable tack strength/trim weight via MaterialFrameParams extension; TR-8 pattern-to-rigid 9-step algorithm, pre-sim only, Solidify soft alternative; TR-9 TrimPlacementRef in GarmentSpec, category-default library resolution; TR-10 7 trim events; TR-11 6 trim checks; TR-12 4 MCP tools (place_trim, define_zipper, convert_panel_to_trim, keyframe_tack_strength); TR-13 CRDT placement proposals; TR-14 prohibitions; TR-15 notes.
- **13.9 UV-from-Pattern & Texturing** — §1 UV = 2D pattern (no 3D unwrap; assign_panel_uvs; grain single authority with shader rotate_uv); §2 ARAP-only flatten (LSCM/ABF++ prohibited; single arap_unfurl_panel; Cholesky, <10 ms @5k tris); §3 packing via `rectangle-pack` (deterministic, overlap-free; atlas transform in vertex shader; fill-ratio warning <0.4); §4 TailorGraphicLayer (panel-local cm bbox, z_order, 5 blend modes, opacity, boundary_pinned default true, ARAP pin integration); §5 TailorPbrMaterial (full slot set + grain + tile size), TailorMaterialAssignment per panel, PBR Map Generator Tauri command (Sobel | WeaveMatrix modes, async); §6 four DDLs (tailor_uv_islands, tailor_pbr_materials, tailor_graphic_layers, tailor_material_assignments); §7 schema ids; §8 10 UV/texture events; §9 CRDT (LWW fields, z_order sequence merge, tombstone deletes); §10 sandbox scope (packing/map-gen direct; model-authored PBR + graphic layers sandboxed); §11 UV_COVERAGE ≥95% + UV_VALIDITY blocking; §12 5 MCP tools + model texturing sequence; **§13 post-MVP deferred: UDIM, UV-space bake, 90° rotation packing, FabricDiffusion, all-quad conversion, fur, toon**; §14 risks (ARAP gather>3, fill ratio, grain drift >2°, unpinned graphics, weave normal artifacts).
- **13.10 Animation & Keyframe Timeline** (closes MOAT-4 + Group 6) — §10.1 11 canonical track types; interpolation Linear/Step/CubicSpline (glTF Hermite; quat slerp); marker track; clamp-no-extrapolate; §10.2 GarmentAnimationDraftV1 in `tailor_garments.animation_json` (not a table); fps [1,120], **total_frames cap 1800 MVP**; material/wind/pose/blend-shape tracks + markers + export_range; §10.3 6 animation events (snapshot-cadence, not per-keyframe); §10.4 CRDT `/animation/` sub-tree, conflict matrix, snapshot triggers; §10.5 per-frame loop (update_params → update_wind → update_body_proxies → simulate_frame), pre-substep capsule projection, turbulence seed = frame_idx, WGSL hash noise, per-frame UBO upload, AvatarPoseSample via FK; shape-envelope promotion equivalence with turbulence; §10.6 import: glTF primary (gltf 1.4.1), FBX secondary behind `tailor-fbx-import`, **MTN prohibited** (re-export path), AvatarPoseSequenceV1; §10.7 export-range filter + **R-ANIM-039 FBX auto-key bake post-MVP**; §10.8 4 MCP tools (draft_create, add_keyframe, simulate, export) + model authoring sequence; §10.9 binding summary; §10.10 **DEFER-ANIM-001 per-tack compliance keyframing post-MVP (stored but ignored)**, DEFER-ANIM-002 FBX bake, turbulence determinism constraint, no real-time full-fidelity preview.
- **13.11 Kernel Integration** — module identity (11-K-1); Postgres-only authority + event-before-row CTE + KernelActor table (11-K-2); canonical 16-table set (11-K-3, incl. normative `tailor_avatars` DDL with source_kind CHECK); dated migrations (11-K-4); canonical GarmentSpec rules (11-K-5); schema-id constants (11-K-6); full ~60-variant event list + superseded table + 13 event families (11-K-7); sandbox adapter (process tier, fs-only, CPU-fallback flag, artifact class) (11-K-8); full 35-check catalog + ValidationFinding contract (11-K-9); PromotionGate binding (CPROM idempotency, no self-approval) (11-K-10); determinism/MeshComparator (11-K-11); CRDT (reuse kernel infra, yjs_bridge, ai_edit_proposal, per-panel LWW) (11-K-12); model lanes (LlmClient only, json_schema constrained, temp ≤0.2, swatch-image material estimation) (11-K-13); wardrobe (11-K-14); 12 Axum routes (11-K-15); normative full lifecycle sequence (11-K-16); portability (no hardcoded paths) (11-K-17); no-SQLite tripwire (11-K-18).
- **13.12 Viewport, Visual Debug & Render/Export Handoff** — 12.1 scope (**Tailor is NOT a photoreal renderer**; DCC does final render); 12.2 throwaway Bevy testbed (isolated crate, headless capture, egui debug overlay); 12.3 native viewport (rendered-to-texture, ≤15 fps live, ClothViewport contract, Solid/Wireframe/Debug pipelines, residual color gradient, CPU arrow normals, staging-buffer readback); 12.4 TailorVisualCapture + SimFrameMetadata (quantitative pre-filter), **settlement gate (kinetic_energy + max residual thresholds as policy params)**, Tauri commands, Axum routes, tailor_captures DDL (CAP-), model annotation verdicts; 12.5 MCP tools (CaptureFrame/AnnotateCapture/ExportGarment) + mandatory model inspection loop; 12.6 export formats (OBJ sequence MUST; glTF morph-target GLB custom encoder MUST, FLOAT deltas; USD time-sample SHOULD behind `usd-export`; **Alembic native write MUST NOT (Blender workaround); FBX write MUST NOT**), export event + tailor_exports DDL (EXP-); 12.7 invariants; 12.8 risks.
- **13.13 Model-First API & LLM Steering** — §.1 crate public API IS the model API; §.2 canonical GarmentSpec (full type reproduced); §.3 SimulationReceipt (SimStatus, MeshStats, ValidationFinding, SuggestedFix RFC-6901, RecommendedAction); §.4 **exactly six MCP tools: author_garment, simulate_garment, edit_garment (RFC-7396 merge patch), promote_garment (consent-gated), get_garment, estimate_fabric_params (diff-xpbd flag)**; §.5 bounded self-correction loop (≤5 same-config iterations; drape_quality_score ≥0.7); §.6 model-facing check subset with corrective actions; §.7 TailorModelAdapter binding; §.8 MeshComparator over content_hash; §.9 ContextBundle content (avatar_summary, presets, solver budget, ngl_description); §.10 DiffXPBD inverse fabric estimation preconditions; §.11 CRDT collaboration (leases, conflict surfacing); §.12 built-in Model Manual (7 mandated topics); §.13 storage binding; §.14 primitive binding table.
- **13.14 Canonical Tailor Authority Contracts** — the binding contract that wins all conflicts: naming table; ONE GarmentSpec (full canonical Rust); ONE avatar/body-proxy schema (tailor_avatars + tailor_body_proxies DDL, mm, capsules+spheres, no garment FK); ONE KernelEventType list (~60 variants) + superseded names; schema-ID constants; migration convention + required migration set; **canonical 16-table set** (tailor_garments, garment_crdt_docs, material_presets, avatars, body_proxies, simulation_runs, refit_runs, trims, trim_placements, zippers, lacings, uv_islands, pbr_materials, graphic_layers, material_assignments, wardrobe); full ValidationDescriptor catalog; determinism vs promotion equivalence; SimulationReceipt canonical feedback; contract-deferral rules for WPs.
- **13.15 Validation, Promotion Equivalence & HBR** — TailorValidationDescriptor stage/feature-flag selection; two severities; stable codes; **full 35-check catalog (19 Blocking / 16 Advisory over 5 stages: fast, mesh, post, preset, refit)**; ValidationFinding/SimulationReceipt contract; TailorGarmentValidationRecorded event; hash-equality gap, content_hash idempotency-only, MeshComparator primary+secondary verdict, animated shape-envelope exception; **HBR matrix INT/SWARM/VIS/QUIET/MAN/STOP obligations** (cancellable solver at substep boundaries, per-garment lane isolation + CPROM idempotency, full event visibility, no focus stealing/bounded logs, non-fixture operator approval, unconditional STOP semantics); idempotency key formats; migration note.

## Prior parity review verdicts

Reviewed against `02-md-feature-map.md` (11 groups, 180 MD features) via `/workflows tailor-md-parity-review` (11 reviewers). 332 core MTs → +116 parity MTs (MT-333..448) = 448.

| Area | MD feats | Covered | Partial | Gap | Proposed MTs |
|---|---|---|---|---|---|
| 2D Pattern Authoring | 20 | 5 | 7 | 8 | 12 |
| Sewing & Seam System | 19 | 7 | 5 | 7 | 10 |
| Fabric & Material Properties | 19 | 11 | 7 | 1 | 7 |
| Simulation Engine | 13 | 5 | 5 | 3 | 7 |
| Avatar System & Body Fitting | 20 | 6 | 7 | 7 | 15 |
| Animation & Dynamics | 15 | 5 | 6 | 4 | 10 |
| Trims, Accessories & Hardware | 13 | 3 | 2 | 8 | 10 |
| UV, Texturing & Rendering | 20 | 11 | 5 | 4 | 15 |
| Garment Library & Asset Mgmt | 8 | 1 | 4 | 3 | 7 |
| Import/Export & Pipeline Interop | 20 | 4 | 6 | 10 | 14 |
| AI / Model-Steerability | 13 | 6 | 4 | 5 | 9 |
| **Totals** | **180** | **64** | **58** | **60** | **116** |

The 60 "gap" + 58 "partial" verdicts were nominally closed by the 116 parity MTs; the review does not re-verify closure — that re-verification is exactly what the second-pass gap analysis must do.

Reshuffle-note substance (still unapplied — see reconciliation note):
- **Ordering fixes**: MT-116/117 before MT-115; InternalLine MT (now MT-345) before MT-122/123/132; transform MT (MT-343) before MT-125/130; primitives (MT-338) before MT-127; MT-127 before MT-128 (currently reversed); MT-334 after MT-128; self-collision precise MT between MT-095/096; MT-307/MT-313 after MT-315/316; avatar migrations MT-047/048 pulled to front of AutoFit; MT-109/110 moved/shared to AutoFit; MT-283 pose grouped before MT-285/287; MT-288 after tack-animation MTs; MT-290 caching after trim-in-timeline; MT-291 adjacent to MT-289; MT-296 after range-export + FBX bake MTs; MT-137 after MT-357; MT-435 before MT-318; MT-446 between MT-324 and MT-325; MT-443 after MT-131; MetaHuman MTs after MT-266/MT-307; MT-252 after MT-440; MT-284 consecutive with glTF avatar import.
- **Cross-group coupling**: MT-018↔MT-118 (SeamSpec shape determines GPU layout); MT-019↔MT-168 shared tack WGSL; MT-132↔MT-226↔topstitch-mesh chain; MT-118→MT-026→MT-121→MT-031 strict chain; MT-134 as day-one sewing proof target; MT-169 CPU parry is fallback, MT-363 primary; MT-364 before MT-168; MT-371 category migration after MT-188 before MT-366; MT-286→MT-392 dependency; MT-153/MT-233 re-examined with UV_COVERAGE (MT-379); MT-262 after MT-375; MT-206 after MT-376/MT-196; MT-207 after MT-209; MT-209 prerequisite of MT-287/407/408; MT-362 after MT-213; MT-388 between MT-231 and MT-232; MT-404 after MT-270; MT-126+MT-129 before MT-400.
- **Naming/scope corrections still open in MT text**: MT-259 `edit_panel` should be `edit_garment` (spec §13.13.4 RFC-7396 full-spec merge patch); MT-260 `run_simulation` → canonical `simulate_garment`; MT-261 `fetch_simulation_receipt` is not a spec tool — retitle to `get_garment` (currently missing from the MT set entirely); MT-269 is NL-path only (DiffXPBD path is MT-403); MT-220 references `tailor_uv_sets`, canonical is `tailor_uv_islands` (+ pbr_materials/graphic_layers/material_assignments); MT-249/250 group label should be ProductionBridge.
- **Reconciliation note**: parity MTs were authored as gap-closers with a nominal linear depends_on chain; the refinement MUST fold them into the clause-closure matrix and APPLY the reshuffle notes via create-task-packet.mjs before execution.

## Known self-declared gaps

**Explicitly deferred / post-MVP in Section 13:**
- MOAT-7 EveryWear-equivalent automated game rigging — deferrable post-v1, must be recorded as deferred requirement in T-PIPELINE-INTEROP (13.1 §2.1); MT-446 implements only a stub + watched-folder push.
- Bidirectional 2D→3D CRDT feedback loop deferred; v1 unidirectional [GAR-CRDT-004] — tension: MT-348 (arc-length write-back) partially implements it anyway.
- Nonlinear buckling bending model post-MVP; MVP may use linear fallback (13.5 invariant 4; risk: wrong wrinkle frequency).
- 13.9 §13 deferred list: UDIM tiles; UV-space texture bake; 90° island rotation in packing (v1 always rotation=0); FabricDiffusion; all-quad mesh conversion; fur strand material; toon shader. **Tension**: parity MTs MT-380 (UV bake), MT-385 (all-quad), MT-390 (toon, feature-gated) implement items 13.9 defers — spec edit needed to reconcile.
- DEFER-ANIM-001: per-tack animated compliance post-MVP (stored, silently ignored with INFO log) — while MT-412 implements per-tack TackStrength tracks; needs reconciliation.
- DEFER-ANIM-002 / R-ANIM-039: FBX auto-key bake post-MVP — MT-419 and MT-421 both implement it (duplicate MTs).
- TR-5.5: two-way rigid-cloth contact response (cloth pushing trims in contact pass) deferred to a future spec revision.
- 13.12: native Alembic write MUST NOT be implemented (OBJ + Blender-Python workaround; MT-440 is the shim); **FBX write MUST NOT be implemented (13.12) yet 13.10/MT-419/MT-421 specify FBX animated export with baked keys — direct internal contradiction**.
- MTN (Marvelous Designer motion format) import prohibited; re-export as FBX/glTF is the supported path (R-ANIM-035).
- `ChebyshevGs`/multigrid solver mode: SHOULD, feature-gated, falls back to Fitting; CubeCL CUDA optional.
- `estimate_fabric_params` behind `diff-xpbd` feature flag (~10× slower); `usd-export` feature-gated with OBJ fallback; FBX import behind `tailor-fbx-import`.
- Full-fidelity animated simulation is not real-time (CONSTRAINT-ANIM-003); animation cap 1800 frames MVP; cross-vendor turbulence determinism only via shape-envelope.
- MT-235 Bevy testbed explicitly OUT of product scope; MT-384 fur is a deferred stub by design.
- Non-humanoid avatars: automatic capsule-chain generation prohibited; operator MUST supply proxy JSON (REFIT-RISK-6) — a real authoring gap BodyKit could close.
- Out of scope entirely (13.1 §2.2): SQLite, runtime MD/CLO dependency, Bevy/Avian in product, provider-specific LLM deps.

**Self-flags in the feature map (02-md-feature-map.md):**
- 8 moats (MOAT-1..8); MOAT-1..6 required for MD parity, MOAT-7 post-v1 stretch, MOAT-8 (LLM steerability) the Handshake differentiator; ~129 features, ~18 at D4.
- MD-side caveats it records: Pattern Drafter is T-shirt-only Beta; AI Pose Generator is cloud-gated; CLO-SET marketplace network moat has "no OSS equivalent" (Handshake analog is only local library MT-357..359); Maya cache export listed in MD matrix (no corresponding MT — MDD/PC2 MT-442 is the nearest); fur experimental.

**Governance-level open items:**
- Reshuffle notes unapplied; parity MT dependency chain is nominal-linear, not true build order.
- Canonical event list (13.14) does not include `TailorAvatarPoseGenerated` (MT-404), `TailorAnimationImported` (MT-410), `TailorTopstitchOptimized` (MT-386), or the capture-annotation event used by 12.4.6 — parity MTs cite event names outside the canonical list; spec amendment required.
- MT group labels reported as `[undefined]` on the 332 core MTs in tooling (parity review note).

## Body/avatar status quo

This is the exact surface BodyKit attaches to. What exists today:

**Authority schema**
- `tailor_avatars` (AVT-{uuid_v7}): workspace_id, name, `source_kind` CHECK ∈ {smpl, smplx, metahuman, custom_obj, vrm, gltf, parametric, avatar1_2d_derived, non_humanoid}, `measurements_mm_json` (25-field anthropometric map, GarmentMeasurements naming: bust_circ_mm, waist_circ_mm, hip_circ_mm, shoulder_width_mm, arm_length_mm, inseam_mm, …), `source_mesh_artifact_ref`, `morph_params_json`, plus `garment_proxy_suggestion JSONB` (pending model proxy proposal, COL-MODEL-002). Defined normatively in 13.11/13.14 — MT-047.
- `tailor_body_proxies` (BPX-{uuid_v7}): FK avatar_id (one avatar → many proxies), `proxy_json` = serialized ClothBodyProxy, `mode` ∈ {capsule, capsule_sphere, capsule_sdf, sdf}, `breast_proxy_mode` ∈ {standard, multi_sphere, sdf_fallback}, `sdf_artifact_ref`, `lores_mesh_artifact_ref`, `joint_hierarchy_json`, `collision_thickness_mm` (default 2.5). No garment FK; garment → proxy via `tailor_garments.body_proxy_id` — MT-048.
- Schema ids `hsk.tailor.avatar@1`, `hsk.tailor.body_proxy@1`; events TailorAvatarCreated, TailorAvatarMeasurementsExtracted, TailorBodyProxyCreated/Updated (families tailor.avatar / tailor.body_proxy) — MT-110, MT-053.

**Geometry representation (all mm)**
- `ClothBodyProxy` = Vec<CollisionCapsule (joint_name, p0_mm, p1_mm, radius_mm)> + Vec<CollisionSphere (bone, center_mm, radius_mm)> + thickness_mm. GPU caps: **max 32 capsules + 16 spheres** per proxy (exceeding → `BodyProxyCapacityExceeded`). SDF secondary mode: 64³ texture3d, pre-sim bake (jump-flooding), re-bake on pose-change threshold, auto-selected when breast spheres >6 and capsules >10 — MT-081..085, MT-088..090.
- Proxy construction: `build_avatar_proxy()` (parry V-HACD, one capsule per limb bone, breast bones by name convention "breast/bust/BreastL…"), canonical large-bust decomposition = 3 spheres per side + sternum capsule; exaggerated-proportion target cup G–K+; min inter-sphere spacing gate — MT-092, MT-101, MT-102, [COL-BUST-*].
- Non-humanoid: operator-supplied proxy_json only; auto-generation prohibited (convex-hull only as approval-required hint).

**Measurements**
- LLM surface: `AvatarBinding { avatar_id, measurements_cm: Option<BodyMeasurements{height,bust,waist,hip,inseam}_cm> }` inside GarmentSpec; cm→mm at API boundary. Authority: 25 measurements in mm.
- `extract_measurements(mesh)` in tailor-solver (GarmentCodeData fit-aware ±20 mm plane search; requires manifold single-component mesh) — §7.1.3; measurement estimation tool (description → AVT- measurements) MT-270; `AVATAR_BINDING` blocking check.

**Avatar sources / import (current MTs)**
- Custom OBJ/FBX avatar import pipeline: AVT- row + auto 25 measurements + capsule chain [MT-426]; static importers OBJ+mtl [MT-437], FBX (feature-gated) [MT-438], glTF/VRM [MT-430, MT-439]; MetaHuman .dna (skeleton + bind pose → capsule chain; export sizing flag) [MT-429, MT-444, MT-445]; OBJ morph-delta → secondary BPX- [MT-428]; blend-shape import with multi-state proxies + warm-start BlendShapeRefitRequest (blend_t) [MT-427, §7.7]; Avatar1/ComfyUI 2D-derived bridge (`source_kind='avatar1_2d_derived'`, §7.1.1); **parameterized built-in avatar library**: seeded archetypes (female/male × petite/standard/tall × straight/hourglass/athletic) driven by anthropometric sliders writing AVT-+BPX- rows [MT-423].

**Skeleton & posing**
- Skeleton exists only as `joint_hierarchy_json` + capsule joint_names; no skinning/skin-weight system for the body. Joint mapping from Daz/Mixamo/MetaHuman/CC naming → canonical Tailor joint set [MT-425]. Pose = AvatarPoseTrack (per-bone translation/rotation/scale keyframes, FK-evaluated to AvatarPoseSample capsule positions per frame, `update_body_proxies` pre-substep) [MT-283, R-ANIM-019..025]. BVH/glTF pose import [MT-284]; FBX motion auto-conversion + JointMapper [MT-410]. IK editing (FABRIK/analytical 2-bone, drives capsule chain) [MT-424]. AI pose generation (NL/reference image → PoseSpec joint degrees → IK updater; TailorAvatarPoseGenerated) [MT-399, MT-404]. Per-avatar friction override [MT-422].

**Morphs**
- `morph_params_json` on tailor_avatars; AvatarBlendShapeTrack/BlendShapeTrack (weight 0–1) in the animation draft; morph-target interpolation for continuous body shapes in refit [MT-308]; morph animation export via glTF morph targets + Alembic channels [MT-394, MT-415]. Refit across morphs is a first-class subsystem (13.7, three modes + blend-shape warm-start).

**Model lane**
- `suggest_collision_proxy` tool (bone hierarchy + breast morph magnitude + measurements → ClothBodyProxy proposal, sandbox-gated) [COL-MODEL-001]; measurement/pose estimation tools [MT-270, MT-399/404]; ContextBundle carries `avatar_summary`.

**What does NOT exist today (the BodyKit-shaped hole):**
- No parametric body **mesh** generation engine: `parametric`/`smpl`/`smplx` are source_kind labels and slider-seeded proxy archetypes (MT-423), but nothing generates or deforms an actual body surface mesh from parameters — avatars are imported meshes or capsule/sphere/SDF abstractions.
- No avatar mesh sculpting, no extreme-morph authoring (extreme proportions are handled only as *collision* decompositions, cup G–K+ colliders — the body itself is static geometry).
- No body soft-tissue/deformation simulation (breasts etc. are rigid colliders; SoftBodySpec MT-335 covers props, not the avatar); no skinning/skin weights; no avatar surface materials/textures/rendering beyond proxy debug; no face/hands/feet detail; no avatar-mesh export path.
- Avatar generation from images/scans is limited to the 2D-derived bridge stub and measurement estimation; body proxy auto-generation is explicitly prohibited for non-humanoids.
