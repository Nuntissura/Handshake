---
file_id: tailor-bodykit-research-05-daz-bodykit-requirements
topic_id: T-BK-DAZ-REQS
title: "Daz Studio Exhaustive Inventory + BodyKit Extreme-Body Requirements (REQ-001..044) + Skeleton/Export Contract + License Findings"
status: non_normative_research
normative_status: non_normative_context_only
research_lane: "web-research sub-agent, session 2026-07-08. Sources: docs.daz3d.com (DSON spec incl. formula/graft pages), Daz forums, Diffeomorphic/Xin ecosystem, Renderotica-adjacent product ecosystem (Golden Palace/Dicktator/Breastacular etc.), UE/Khronos docs, license pages fetched directly."
key_verified_facts: "SMPL/SMPL-X/STAR license PROHIBITS pornographic use (verified at smpl-x.is.tue.mpg.de/modellicense.html) — DISQUALIFIED for Handshake. MakeHuman official-app exported meshes = CC0 (verified). MPFB2 assets = CC0 (verified). MB-Lab = AGPL propagates to outputs, unusable. Genesis mesh/topology/UV reuse prohibited by Daz EULA. DSON reading of user-owned files is low-risk (public spec)."
updated_at: "2026-07-08"
---

# Daz Studio Inventory + BodyKit Requirements (independent research lane)

## Part 1 inventory

### 1A. Figure platform

| Feature | What it does | Daz mechanism | BodyKit-relevant notes |
|---|---|---|---|
| Genesis unimesh | One shared base mesh per generation; all characters are morphs of it | "Unimesh" — single androgynous quad mesh, morphs never change vertex count/order | Core pattern to copy: one canonical topology, everything is deltas |
| Genesis 1 (2011) | Single androgynous base, male+female from morphs | One figure, gender = FBM dials | Proved single-base works; weaker gendered silhouettes |
| Genesis 2 (2013) | Split into separate F/M base figures | Two meshes, shared tech | Split done for better gendered defaults; costs double content |
| Genesis 3 (2015) | F/M figures, facial bones replace face morph-only rig | TriAx→general weights transition, ~16k quads (UNVERIFIED exact count) | Introduced twist bones, facial bone rig |
| Genesis 8 / 8.1 (2017/2021) | F/M figures, A-pose default; 8.1 added FACS + PBRSkin | Same topology as G3 (compatible morphs), 8.1: 63 FACS morphs, new eyelash + tear meshes, reworked eye UVs | 8.x is the ecosystem's workhorse and the target of most adult content |
| Genesis 9 (2022) | Return to single androgynous base for both sexes | One unimesh, ~2x G8 poly count (UNVERIFIED exact), evenly spaced quads for sculpting | Validates BodyKit single-base + masc/fem morphs plan |
| G9 gender handling | Fem/masc are just full-body morphs | Base Feminine / Base Masculine FBM dials; can be negative-dialed, mixed | Gender as continuous axis, not binary asset split |
| G9 modularity | Eyes, eyelashes, tear, mouth are separate fitted figures | Conforming sub-figures following head | Keeps base mesh clean; export must merge or handle multi-mesh |
| Base resolution vs SubD | Figure stores low-res cage; renders subdivided | Catmull-Clark subdivision, per-figure SubD level (view vs render) | BodyKit needs cage+SubD split to support HD detail |
| Backward compatibility clones | Old-generation clothing fits new figure | Built-in autofit clone shapes (G9 ships clones for G1/G2/G3/G8) | Clone-shape concept = cross-version garment compatibility |
| Character Essentials/Starter Essentials | Free base package incl. anatomical elements (G9) | DIM/Daz Central installed content bundle | BodyKit should ship complete base incl. genitalia — no paywalled anatomy |

### 1B. Morph system

| Feature | What it does | Daz mechanism | BodyKit-relevant notes |
|---|---|---|---|
| Morph targets | Shape change as vertex deltas | Delta arrays in DSF `modifier_library`, same vertex order required | Foundation; store as sparse deltas |
| Shaping sliders | User-facing dials in Shaping pane | Properties with min/max/default, grouped by region | Need region-grouped parameter UI |
| Slider limits + Limits Off | Clamp dial range; user can disable limits | Per-property limits flag; community routinely dials past 100% | Extreme bodies REQUIRE designed >100% ranges, not accidental extrapolation |
| Morph stacking | Multiple morphs sum on one mesh | Linear delta addition | Stacking without volume correction = Daz's biggest extreme-shape failure |
| FBM (Full Body Morph) | One dial reshapes whole body | Single morph asset spanning all regions | The coupling problem: FBMs entangle breast/hip/frame — BodyKit must regionalize |
| PBM (Partial Body Morph) | Region-scoped morph (e.g., forearm) | Delta set limited to region vertices | Basis for decoupled region morphs |
| Character preset | A named character = dialed morph mix + materials | .duf preset referencing installed morph products | "Character = recipe over morph library" model |
| HD morphs | Detail at SubD levels 2-4 (wrinkles, veins, muscle striation) | Proprietary .dhdm binary (subdivision-level deltas); SubD 3-4 typical | BodyKit: open multi-res delta format |
| HD morph gatekeeping | Only Daz PAs can author HD morphs | HD authoring tool never publicly released | Community pain point; BodyKit differentiator = open HD authoring |
| HD workaround | Bake detail to normal/displacement maps | ZBrush/Blender sculpt → map bake | Map-based detail lane should be first-class |
| Morph Loader Pro | Import OBJ as morph | Matches vertex order; scale/rotation presets; morph saved as deltas | Import pipeline for sculpted morphs (Blender round-trip) |
| Reverse deformations | Subtract already-dialed shapes when importing a sculpt | "Reverse Deformations = Yes" filter computes delta vs current stack | Essential for authoring correctives on top of stacked shapes |
| Deltas-only overwrite | Replace an existing morph's deltas | "Overwrite Existing: Deltas Only" | Versioned morph editing |
| ERC / property linking | Any property drives any other (morph→morph, morph→bone, bone→morph) | DSON `formula` objects: RPN operation stacks, output channel, `stage` sum/multiply; output = (Σ sums)x(Π multiplies) | THE coupling engine. BodyKit needs an equivalent dependency graph — but with user-visible/deletable links so couplings can be DECOUPLED |
| ERC Freeze | Records current property deltas as permanent links to a master dial | Auto-links every changed property to controller; user must prune wrong links | Error-prone; BodyKit should make link authoring explicit + auditable |
| JCM (joint corrective morph) | Morph auto-driven by joint rotation to fix bend artifacts | ERC link: bone rotation channel → morph value, scalar = 1/limit; interpolated | Must replicate; and must regenerate JCMs for extreme shapes (Daz doesn't) |
| MCM (morph corrective morph) | Morph auto-driven by another morph (fix morph x morph combos) | ERC morph→morph link, usually hidden dials | Needed for combinatorial fixes (huge breasts x skinny frame) |
| Multi-stage controllers | Corrective fires only when 2+ inputs active | Second-stage multiply controllers in ERC | Multiplicative gating in dependency graph |
| Pose controls | High-level dials ("Arms Up") drive many bones | ERC morph→bones aggregation dials | Same graph, opposite direction |
| Auto-follow morph projection | Fitted clothing auto-receives body morphs | On-the-fly delta projection from figure to conformer | Garments must inherit new/extreme morphs automatically |
| Auto-generated JCM projection | Fitted items also receive JCM activations | Projection of driven correctives to conformers | Keeps clothing bending with body |
| Rigidity maps/groups | Protect buttons/buckles from projected morph distortion | Painted rigidity weight map + rigidity groups w/ reference vertices | Needed so garment hardware survives extreme-body projection |
| Rigid follow nodes | Parent rigid prop to a face of deforming mesh | Node follows facet under morph/pose | Cheap alternative for accessories |
| Hidden/vendor morphs | Correctives hidden from users | Hidden property flag + hidden category | Separate authored-API vs user-facing dial namespaces |
| Head/body split presets | Apply head and body shapes independently | Separate head/body FBM products + presets | Users expect head/body independence — generalize to all regions |
| Shape Shift / proportion products | Fine proportion editing (Zev0 Shape Shift etc.) | 3rd-party morph packs + bone-scale dials | Market proof of demand for proportion axes Daz base lacks |
| Growing Up-style age/proportion | Proportion dials (production use here: petite adult builds) | Morphs + bone scaling ERC combos | Proportion = morph+skeleton-scale hybrid; BodyKit must couple both per axis |
| Fit Control | Per-garment adjustment morphs (loosen, expand regions) | Auto-generated adjustment morph set injected into clothing | Garment-side correction dials for extreme bodies |
| Breast Control / Breast Utilities / Breastacular | Deep breast parameterization | 30-43 breast morphs, 60-70 nipple morphs, extra breast/areola/nipple bones, gravity + collision morphs (L/R split), geoshell texture fix maps (Breast Utilities 2) | Direct feature checklist for BodyKit breast module |
| Glute Control / glute utilities | Same pattern for ass | Glute morph sets, squish/collision morphs | Round-ass-narrow-thigh axis must be native |
| Fat Control / SomeBody | Targeted fat placement | 41 regional fat morphs (belly, rolls, chin, arms...) | Fat distribution as independent regional axes |
| Muscularity / Musculature HD / Easy Flex | Muscle definition + flex states | HD muscle morphs + "flex" dials ERC-driven by pose | Muscle amount vs flexion are separate axes; flexion pose-driven |
| Morph projection to geografts | Grafts inherit body morphs automatically | Graft mesh receives projected deltas | Genital grafts must track body shape changes |

### 1C. Rigging

| Feature | What it does | Daz mechanism | BodyKit-relevant notes |
|---|---|---|---|
| Bone hierarchy | Standard skeleton per generation | Named bone tree (G9 renamed: spine1-4, new arm/leg chain, helper bones) | Stable canonical skeleton = content compatibility contract |
| Weight-map skinning | Mesh binding | LBS and DQS supported + blended LBS/DQS modes | Choose LBS for engine parity + optional DQS preview |
| TriAx (legacy) | Per-rotation-axis weight maps | Proprietary tri-axial maps (G1/G2 era) | Skip; industry moved to general maps |
| Twist bones / helper bones | Distribute forearm/thigh twist | Dedicated twist bones w/ weight maps (G9) | Include twist bones; they survive FBX |
| Facial bone rig | Bone-based face posing (G3+) | Facial bones + FACS morphs hybrid (G9) | Hybrid bones+morphs face architecture |
| Adjust Rigging to Shape | Re-fits bone centers/endpoints to a morphed shape | Recomputes joint centers from mesh regions; critical when morph scales/lengthens | MUST be automatic per-morph in BodyKit (joint auto-derivation from mesh markers) |
| ERC Freeze of rigging | Ties adjusted joint centers to the morph dial | Links joint-center offsets to morph value (0%→base, 100%→adjusted) | Skeleton must interpolate continuously with dials |
| Memorize rigging | Baseline for freeze operations | Stored rest rig state | Explicit rest-state versioning |
| Joint Editor | Manual bone center/orientation editing | Tool pane; center/end points, orientation | Advanced-user rig editing surface |
| Per-bone scaling | Scale a limb segment; propagates to children | Bone scale channels (propagating); used by proportion products | Limb length/thickness as first-class per-bone scale params |
| IK chains + pins | Drag hand/foot, body follows; pin parts | IK chains (DS 4.12+), Active Pose tool pins, pin translation/rotation w/ reach % | Ship modern full-body IK; Daz IK is weak (community complaint) |
| Weight-map brush | Paint skin weights | Node Weight Map brush tool | Needed only for authoring lane |
| Selection/face groups | Polys assigned to bones for selection | Face groups matching bone names | Region metadata doubles for masks/autofit |

### 1D. Fitting & clothing

| Feature | What it does | Daz mechanism | BodyKit-relevant notes |
|---|---|---|---|
| Fit-to (conforming) | Garment locked to figure, follows pose+shape | Conformer runtime: copies pose, projects morphs | Cloth engine input: garment must conform before/instead of sim |
| Transfer Utility | Rigs arbitrary mesh from figure's rig | Transfers weights, face groups, UVs optionally, morphs, smoothing modifier | One-click "make this mesh a garment" |
| Projection templates | Better transfers for dress/hair/skirt classes | Template items w/ tuned weight maps | Garment-class-aware projection profiles |
| Autofit (cross-figure) | Converts old-figure clothing to current figure | Clone shape of source figure + re-rig via templates | Needs donor clone shapes; breaks on skirts/heels (known) |
| Auto-follow | Clothing follows body morph dials incl. new ones | Live delta projection | Must survive extreme dials — Daz's projected result collapses on huge breasts (known failure) |
| Smoothing modifier | Relax garment + push out of body | Base Shape Matching (Pyramid Coordinates) or Basic/Laplacian/Generic; iterations, weight | Cheap post-projection relaxation; keep |
| Collision iterations | Push cloth outside body mesh | Collision detection passes (default 3), collision smoothing interval | Body-vs-garment collision resolve independent of full sim |
| Push modifier | Offset mesh along normals | Push modifier w/ weight map (4.23 weight-mapped push on shells) | Fix poke-through, fake thickness |
| Fit Control product | End-user garment fix morphs | Injected expand/loosen morphs per region | Ship as native "garment fix" dials |
| Known failure: big breasts | Autofit garments crease/spike between/under breasts | Projection has no volume awareness; community fixes: Fit Control, smoothing 20+, morph re-import w/ reverse deformation, dForce drape over timeline | BodyKit: sim-based drape + designed garment morph ranges instead |
| Known failure: petite figures | Clothing distortion on thin bodies | Same projection limits | Validates decoupled-region approach |
| dForce-as-fitter | Dial morph over timeline while simming | Start from undistorted shape, grow body into cloth | "Grow into garment" = robust extreme-body fitting strategy; adopt natively |

### 1E. Geografts & shells

| Feature | What it does | Daz mechanism | BodyKit-relevant notes |
|---|---|---|---|
| Geograft | Replaces a base-mesh region with new geometry | DSON `graft`: `vertex_pairs` (weld map source↔target), `hidden_polys` (base faces hidden), requires target vertex/poly count match | Precise, documented mechanism — replicate as native "region graft" |
| Graft morph inheritance | Grafts receive body morph projections | Auto-projection onto graft | Genitals follow body shape automatically |
| Graft rigging | Grafts carry own bones (shaft, clit, anus chains) | Conforming figure w/ extra bones grafted on | Genital rig merges into body rig |
| Geometry shell | Zero-geometry clone floating above mesh w/ own materials/UVs | Shell node: offset clone, per-surface visibility, own UV set | Used for graft texture blending, makeup, wet layers, tattoos |
| Shell UV blending | Hide body/graft seam | Shell w/ independent UV + blend textures over both body and graft; "blends with any white skins" (GP) — dark skins problematic (known complaint) | BodyKit: native material blend zone instead of shell hack |
| Official anatomical elements | Daz's own genitalia grafts (G8/G9, F+M) | Included in Character Essentials (G9); textures must come from character product | Baseline anatomy; texture-compat friction is a known pain |
| Golden Palace (G8F/G9) | 3rd-party female genitalia graft | Fully rigged (all parts posable), 90+ (G8F) / 500+ (G9) morphs+presets, huge-gape support, shell-based texture blending, anus+vagina, extreme dials | Feature bar for BodyKit vulva module |
| Dicktator (G8M/G9) | 3rd-party penis graft | Fully rigged glans→colon posable, flaccid↔erect, 360+ morphs/presets, foreskin, scrotum, 13 veins, displacement presets, cumshot props, shell textures | Feature bar for BodyKit penis module |
| Futalicious / expansions | Futa graft + morph/texture expansions | Same graft+shell pattern; MHX/Diffeo adds FK/IK shaft bones in Blender | Mixed-anatomy support = market requirement |
| Graft pose breakdown | Grafts distort at extreme poses | Bend correctives missing at range (forum-documented) | Genital JCMs must cover full pose+size range |
| Shells for effects | Makeup, wet sheen, tattoos as shells | Shell + LIE layers | Keep as material feature, not geometry |

### 1F. Physics

| Feature | What it does | Daz mechanism | BodyKit notes |
|---|---|---|---|
| dForce cloth | Physics drape/settle of garments | GPU (OpenCL) spring-mass surface sim | Handshake cloth engine replaces this lane |
| dForce surface params | Fabric behavior | Per-surface: friction, density/weight, stretch/bend stiffness, contraction, etc. | Map to Handshake cloth material params |
| dForce weight maps | Vary sim influence per vertex | Painted influence map over dynamic surface | Per-vertex sim mask required |
| Sim settings | Control run | Start bones from memorized pose, current-frame vs animated, collision mode/iterations, collision offset, self-collide | "Grow pose from rest" initialization is key for extreme bodies |
| dForce explosions | Sim blow-ups | Short edges vs collision offset; intersecting start states | Robust CCD + degenerate-edge handling needed |
| dForce strand hair | Simulated guide hairs | dForce modifier on guide strands; interpolated followers | Hair out of BodyKit scope; note pattern |
| No native soft body | Daz can't do volumetric jiggle | dForce = cloth only, no pressure (forum-confirmed) | Breast/ass/belly soft-body is an unmet need BodyKit can win |
| Jiggle products | Fake soft body | Jiggle Deformer plugin, eR Auto Breast Animator, dForce Soft Breast, HS dForce Hip and Breast V2, Ghost Dynamics | "Bouncidelic": not found (UNVERIFIED/likely nonexistent). Demand proven by product count |
| Blender/Diffeo softbody route | Users export to Blender for real softbody | Diffeomorphic softbody presets | Competing pipeline; BodyKit should keep physics in-suite or export sim-ready |

### 1G. Posing & animation

| Feature | What it does | Daz mechanism | BodyKit notes |
|---|---|---|---|
| Pose presets | Saved bone transforms (+ optional morph values) | .duf pose preset; hierarchical variants | Pose library format needed |
| Pose symmetry | Mirror pose L↔R | Symmetry command on figure | Trivial must-have |
| Puppeteer | 2D pad blending between saved poses; real-time record | Pose markers, proximity blend, record mode | Nice-to-have blend-board |
| Timeline | Keyframe animation | Keyframes/TCB, baking, layers (basic) | Minimal timeline for pose sequencing; render anim in Blender/UE |
| aniMate2 / aniblocks | Clip-based non-linear animation | aniBlock clips, sequences, mocap presets | Clip concept useful for loopable sex-scene cycles (export lane) |
| Active Pose / pins | Drag-posing with IK + pinning | Alt-drag IK, pin icons | Modern equivalent: full-body IK w/ pins |
| Expression system | Face posing | G8.1/G9 FACS dials (G9 200+ units) + HD detail layer per unit + facial bones | FACS-named face morphs = mocap-compatible (ARKit mapping) |
| Face Control G9 | In-viewport face controller gizmo | Add-on rig of handles driving facial bones/FACS | Viewport face gizmo board |
| Expression→export gap | FACS driven by ERC boards hard to export | Hundreds of morphs behind controller dials (forum pain point) | Export baked FACS/ARKit morphs directly |

### 1H. Materials/render (render lane only — BodyKit exports)

| Feature | What it does | Daz mechanism | BodyKit notes |
|---|---|---|---|
| Iray render | Path-traced PBR | NVIDIA Iray, MDL shaders | Not replicated; Blender/UE render |
| Iray Uber shader | General PBR w/ deep SSS control | Metallicity/spec/gloss, translucency, SSS, transmitted color/distance, dual-lobe specular | Export as principled-BSDF-compatible maps |
| PBRSkin shader | Skin-dedicated shader (8.1+) | SSS+transmission toggles, dual-lobe spec, micro-detail | Map to Blender Principled/UE subsurface profile |
| UV sets per generation | Textures keyed to figure UV layout | G9: head/body/arms/legs (+UDIM option), 4K base, 8K optional detail, 4K genital maps, 1K nails; Legacy-UV products remap G3/G8 textures onto G9 | UV layout = ecosystem compatibility decision (UV-layout IP comfort UNVERIFIED) |
| Geoshell material layers | Makeup/wet/tattoo overlays | Shell surfaces + opacity masks | Implement as layered material slots |
| LIE (Layered Image Editor) | Runtime texture layer compositing | Layers+masks+blend modes composited into temp textures | Equivalent: non-destructive texture layer stack, bake on export |
| Skin detail maps | Pore/wrinkle micro detail | Detail normal/spec maps, optional 8K | Ship detail-map slots; HD morph alternative |
| Genital material blending | Match graft to body skin | Shell blend textures per skin or "white skin" generic | Auto-blend by sampling body albedo at boundary = differentiator |

### 1I. Content/asset system

| Feature | What it does | Daz mechanism | BodyKit notes |
|---|---|---|---|
| DSON format | Open JSON scene/asset format | Spec on docs.daz3d.com; asset types: geometry, images, materials, modifiers, nodes, UV sets | Read-DSON importer feasible (spec public, dufman Python lib exists) for user-owned content migration |
| DSF vs DUF | Data files vs user-facing files | .dsf in /data referenced by assets; .duf top-level scene/preset; delayed loading | Two-tier asset store pattern is sound |
| Formulas in DSON | ERC persisted | `formula` objects w/ RPN ops, sum/multiply stages | Parseable: coupling graph of any Daz asset can be analyzed mechanically |
| Content library | Folder-based browser | Mapped content directories | Simple: keep |
| Smart Content + CMS | Metadata-driven browsing | PostgreSQL CMS; products, categories, compatibility tags, license tags (4.23) | Metadata DB w/ figure-compatibility + license tags |
| Preset taxonomy | Granular apply-able assets | Character, shaping, pose, wearables, materials, hierarchical presets | Preset kinds map 1:1 to BodyKit needs |
| Install pipeline | DIM / Daz Central / Connect | Package manager + metadata import | Handshake asset manager equivalent |
| Merchant resources | Morph packs licensed for derived characters | "Merchant resource" license class | BodyKit should define clear derivative-morph licensing |

### 1J. Interop & bridges

| Feature | What it does | Daz mechanism | BodyKit notes |
|---|---|---|---|
| FBX export | Mesh+rig+morphs+anim out | FBX exporter; Morph Export Rules dialog; anim bake to keyframes; cm units | Baseline; BodyKit must beat its reliability |
| OBJ export | Static mesh | OBJ w/ scale presets | Trivial |
| Alembic | Vertex-cache export | 3rd-party (Sagan) — not native | Handshake natively better for sim caches |
| USD | Scene interchange | Not native (forum requests only) (UNVERIFIED post-2025) | Consider USD as modern lane |
| glTF | — | Not supported by Daz | BodyKit: support glTF w/ morph targets |
| Daz to Blender bridge (official) | One-click transfer | FBX+JSON intermediary; shader conversion widely criticized | Low bar |
| Diffeomorphic (community standard) | Full-fidelity Daz→Blender | Reads DSON directly: Iray→Cycles shaders incl. SSS, JCMs as Blender drivers ("Auto JCM"), geograft merge (weld to body mesh), shells→material layers, HD via Xin addon, MHX rig w/ genital FK/IK | The real parity target for Blender export: morphs + drivers + merged grafts + materials |
| Xin daz_hd_morphs | HD morphs into Blender | Decodes .dhdm; outputs HD shape keys, rigged HD meshes, vector/normal displacement | HD detail can travel as vector displacement — adopt as export strategy |
| Daz to Unreal bridge | Character→UE | Plugin: materials, skeleton, morphs, anim; morph selection unstable; geograft morphs NOT supported; UE 5.1 rotation bugs; A-pose reference mismatch breaks morphs on Epic skeleton; "Use T0 As Ref Pose" needed | Pain list = BodyKit UE export acceptance tests |
| Premier rig conversion | One-click rig convert | UE / MetaHuman / Mixamo skeleton conversion (Premier 2024+) | Direct competitor feature; BodyKit must ship UE-ready rig natively |
| What survives FBX | Morphs=blendshapes yes (if listed), rig yes, JCM ERC links NO (must re-drive in engine), HD detail NO (base cage only), dForce no, shaders partially | FBX limits | Export contract must carry: baked JCM set + driver metadata sidecar + displacement for HD |
| Scale/orientation pain | Daz cm Y-up; Blender m Z-up; UE cm Z-up | FBX exports cm; Blender import scale 0.01 issues; bone orientation fix popups | Define exact unit/axis contract per target (Part 3) |
| Daz AI Studio | Text-to-image side service | Separate AI product (Premier bundling UNVERIFIED) | Not parity-relevant |

### 1K. Daz Studio Premier (2024-2026 subscription)

| Feature | What it does | Notes |
|---|---|---|
| Render Queue | Batch render multiple scenes/cameras | Premier-only |
| Simulation Manager | Per-object dForce sim runs | Premier-only |
| Geometry Sculptor | In-viewport sculpting (poke-through fixes, tweaks) | Premier-only; first native sculpt tool |
| Shape Transfer | G3/G8/G8.1 shapes → G9 | Premier-only |
| Pose Converter | G8/G8.1 poses → G9 | Premier-only |
| Rig conversion | UE / MetaHuman / Mixamo one-click | Premier |
| Environment export to Blender; Send-to Blender/Maya | Scene transfer | 4.23+/Premier |
| G8↔G9 autofit both directions | Cross-gen clothing | 4.23 |
| UI: Pages/nesting, Blender/Maya/UE preset layouts | Workflow | 4.23 |
| License tags + advanced Smart Content search | Filter owned content by license | 4.23 |
| Membership perks | Monthly subscription: exclusive content, bundles, discounts | Commercial model note |
| Cloud render / Genesis 10 | Not found — no cloud render located; no G10 announced | (UNVERIFIED negative) |

## Part 2 extreme-body requirements

Operator targets: petite female + unrealistically huge tits; narrow shoulders/hips/small hands with huge breasts; skinny; long legs; round ass + narrow thighs; oversized penis; muscular; slender; fat/obese M/F. (See also 03-operator-body-requirements.md OBR-001..004: full breast-shape space incl. fake/plastic implant look; shoulder/hip/thigh/midriff decoupling.)

**Region & decoupling architecture**
1. REQ-001 — Region-scoped morph system: every shape axis authored against an explicit vertex-region mask (breasts, glutes, thighs, shoulders, hands, belly, genitals...). Fixes: Daz FBMs entangle breast volume with frame/hip scaling.
2. REQ-002 — Zero implicit cross-region coupling: no morph may write outside its declared mask; cross-region effects only via explicit, user-visible links. Fixes: hidden ERC chains that drag hips/shoulders when dialing breasts.
3. REQ-003 — Inspectable/deletable coupling graph (ERC equivalent): dependency graph with named edges, per-edge disable, and "isolate region" override. Fixes: ERC-freeze links being invisible and error-prone.
4. REQ-004 — Orthogonal frame axes: shoulder width, hip width, hand/foot scale, limb length, torso length each a standalone axis (bone-scale + morph hybrid). Fixes: needing 3rd-party packs for basic proportions.
5. REQ-005 — Petite build = designed axis set (height, frame, limb ratios) usable simultaneously with any breast/ass volume at max. Acceptance test: petite + max breasts + narrow shoulders + small hands renders artifact-free.
6. REQ-006 — Compensation morphs authored per region-pair (breast x ribcage seam, glute x thigh crease) instead of full-body correctives. Fixes: MCM sprawl.
7. REQ-007 — Additive-with-volume-preservation blending option (delta-mush/volume correction post-pass) for stacked morphs. Fixes: linear delta stacking collapse at extremes.

**Extreme ranges by design**
8. REQ-008 — Every production axis ships with calibrated over-range (e.g., breast volume to 300% design max) with correctives authored across the FULL range, not extrapolated past 100%. Fixes: "Limits Off" artifacts.
9. REQ-009 — Multi-key morph tracks: axis = sequence of sculpted keyframes (0/50/100/200/300%) interpolated, not a single delta scaled. Fixes: linear morph degeneration at extremes (breast spikes, crease collapse).
10. REQ-010 — Breast module parity+: volume, shape family (round/teardrop/torpedo), sag/gravity, cleavage/separation, placement height/width, projection, firmness, L/R asymmetry, nipple/areola sub-module (size, puff, direction), collision/squish morphs L/R (bar: Breastacular/Breast Control 40+ breast, 60+ nipple morphs, dedicated bones).
11. REQ-011 — Dedicated breast/glute/belly bones (with soft-DOF) so animation jiggle and pose offsets don't rely on morphs alone; bones survive FBX export.
12. REQ-012 — Glute module: volume, roundness, lift, width, hip-dip fill, crease sharpness, squish/sit-deform morphs, independent of thigh thickness axis.
13. REQ-013 — Thigh/leg axes: thigh girth (inner/outer split), gap, calf, leg length via bone scale with automatic JCM re-derivation.
14. REQ-014 — Belly/fat module: belly size, apron/overhang, rolls (count/position), love handles, back fat, double chin, arm/thigh fat, fat-distribution presets M/F (bar: Fat Control 41 morphs, SomeBody). Obese M+F to production quality.
15. REQ-015 — Muscle module: mass axis (geometry) separate from definition axis (normal/displacement detail) separate from flexion state (pose-driven); per-group dials.
16. REQ-016 — Skinny/slender axes: subcutaneous fat removal, rib/hip-bone visibility, without collapsing breast/glute volumes (explicit region exclusion).

**Rigging stability at extremes**
17. REQ-017 — Automatic joint re-derivation from mesh landmarks on every shape change (continuous Adjust-Rigging-to-Shape), interpolated with dial value.
18. REQ-018 — Shape-aware corrective generation: JCMs re-solved (or RBF-interpolated) for the CURRENT body, not just base shape. Fixes: JCM breakdown when bending giant-breast/obese bodies.
19. REQ-019 — Skinning weight re-targeting at extremes: weight maps defined in canonical space and re-projected as regions grow.
20. REQ-020 — Twist bones + optional DQS/blended skinning to kill candy-wrapper artifacts on long slender limbs.
21. REQ-021 — Per-bone scale channels with child-propagation control exposed as safe UI axes (limb length != limb thickness).
22. REQ-022 — Pose-range guarantees for genital rigs: erection sweep, insertion poses, extreme spreads covered by correctives.

**Cloth/collision at extremes**
23. REQ-023 — Auto-generated convex/SDF collision proxies per region, updated with morph state, for cloth sim against huge breasts/ass/belly.
24. REQ-024 — "Grow-into-garment" fitting: sim while dialing body from neutral→target shape (community dForce timeline trick, made native).
25. REQ-025 — Garment morph inheritance (auto-follow equivalent) with volume-aware projection, not closest-point projection. Fixes: autofit spikes between/under breasts.
26. REQ-026 — Rigidity masks for garment hardware honored by both projection and sim.
27. REQ-027 — Garment-side fix dials auto-generated per region (Fit Control equivalent).
28. REQ-028 — Soft-body (pressure/volume) sim for breasts/glutes/belly — native, since Daz has none; with collision vs hands/props (squish).

**Genitalia architecture**
29. REQ-029 — Genitals as native region modules, not aftermarket grafts: vulva + penis + anus always available, toggleable resolution. Keep graft-style region-replace mechanism (DSON graft = vertex_pairs + hidden_polys) for third-party extensions.
30. REQ-030 — Penis module parity+ (bar: Dicktator): full pose rig (base→glans), flaccid/erect continuum dial, length/girth/curve/taper, foreskin state, scrotum size/tightness, vein intensity (displacement), oversized range per REQ-008/009.
31. REQ-031 — Vulva module parity+ (bar: Golden Palace): rigged labia/clitoris/vaginal canal/anus, open/close/gape dials incl. extreme gape, wetness material layer, 100+ shape presets.
32. REQ-032 — Erection state as pose+morph composite exportable as (a) bone pose and (b) baked blendshape, so it survives FBX to UE/Blender.
33. REQ-033 — Seamless genital skin: shared UV space + automatic albedo/normal boundary blending sampled from body skin (incl. dark skins).
34. REQ-034 — Genital collision proxies + soft-body response for insertion scenes.
35. REQ-035 — Mixed-anatomy support (futa: female body + penis module) with no cross-module conflicts.

**Skin/texture at extremes**
36. REQ-036 — UV strain compensation: detect texel stretch under extreme morphs and auto-generate corrected detail maps or per-morph UV adjustment.
37. REQ-037 — Multi-res detail lane: HD-morph equivalent stored as open vector-displacement/multi-res deltas, bakeable to normal/displacement on export.
38. REQ-038 — Layered texture stack (LIE equivalent): decals/tattoos/makeup/wet layers with masks, baked at export.

**Authoring & ecosystem**
39. REQ-039 — Open morph authoring round-trip: OBJ/glTF sculpt import with reverse-deformation filtering and deltas-only overwrite (Morph Loader Pro parity).
40. REQ-040 — HD authoring open to all users (no PA gatekeeping) — explicit differentiator.
41. REQ-041 — Character presets = recipes (dial values + material refs) separable from morph data; head/body/region preset granularity.
42. REQ-042 — Compatibility/metadata DB with region, axis, license tags (machine-readable).
43. REQ-043 — Deterministic, scriptable parameter API (all dials addressable by stable IDs) for LLM/parallel-agent workflows and batch body generation.
44. REQ-044 — Acceptance gates: automated render tests of the operator's canonical bodies at pose extremes, garment-draped, exported to Blender+UE and re-verified there.

## Part 3 skeleton/export contract

**Options considered**
| Option | Pros | Cons |
|---|---|---|
| UE5 Manny/Quinn skeleton native | Direct UE drop-in; documented IK bone set + twist bones; huge retarget ecosystem | Fixed-proportion assumptions poor for extremes; no facial bones; no genital bones |
| MetaHuman skeleton | Face rig depth; usable outside UE since 2025 | Heavy; DNA toolchain proprietary; building a rival char-gen ON MetaHuman assets is EULA-risky (UNVERIFIED nuance) |
| Custom BodyKit skeleton + IK-Rig retarget | Freedom for extreme proportions; breast/glute/genital bones first-class; UE5 IK Retargeter + Blender Rigify make retarget routine | Ship + maintain retarget assets |
| SMPL-X skeleton | Research-standard | License bars porn outright — dead on arrival |

**Recommendation**
- Custom canonical BodyKit skeleton, UE5-compatible by construction: match UE5 bone naming/orientation conventions where anatomy overlaps (pelvis/spine_01..., clavicle_l, twist bones), add ik_* bone set on the UE export profile only; ship an IK Rig + IK Retargeter asset for UE and a Rigify/Diffeo-style mapping for Blender.
- Extend with: breast_l/r chains, glute_l/r, belly, full genital chains (penis_01..05, scrotum, labia set, anus) — exported as plain FBX bones so UE physics can drive them.
- Units/axes contract: export FBX at 1 unit = 1 cm, Z-up/X-forward for UE; Blender lane via glTF (meters, format-handled) or FBX with explicit 0.01 scale handling. Per-target export profiles instead of one generic FBX.
- Morph travel: FBX blendshapes for UE (Interchange FBX reliable; UE glTF morph corruption documented 5.4); glTF morph targets for Blender/web. Export selected dials only (Morph-Export-Rules-style) with baked corrective set.
- Correctives/ERC: FBX carries no drivers — export (a) JCMs baked as blendshapes with pose-space naming (`jcm_thigh_fwd_90_l`) plus (b) machine-readable driver sidecar (JSON: bone channel → morph curve) + generators: Blender addon builds shape-key drivers (Diffeomorphic "Auto JCM" precedent), UE Pose Driver (RBF) setup.
- Face: FACS-style dials mapped to ARKit-52 blendshape names as interchange standard; full FACS optional.
- HD/detail: bake to vector displacement + normal maps per export; optional full-HD mesh for hero stills.
- Genitals/extreme morphs into engines: pre-welded single-mesh option (graft merged, one material set, blended textures baked) so UE never sees shells/grafts; genital bones ride the main skeleton; erection & gape ship as both pose assets and blendshapes.
- Anim interchange: FBX anim takes (baked keys); Alembic/USD cache lane for cloth-sim results.

## License findings

| Asset | License status | Commercial adult use | Source |
|---|---|---|---|
| Daz Genesis meshes/morphs | Proprietary EULA; Interactive License per-product for 3D redistribution; no content redistribution | Renders fine; **mesh/topology/UV reuse in BodyKit prohibited** | daz3d.com/eula |
| DSON format | Spec published openly, JSON | Reading user-owned files: low risk. Shipping Daz-derived data: prohibited | docs.daz3d.com DSON spec |
| SMPL / SMPL-X / STAR (MPI) | Academic non-commercial; commercial via Meshcapade; **"may not be used for pornographic purposes or to generate pornographic material whether commercial or not"** | **DISQUALIFIED for Handshake** (verified at smpl-x.is.tue.mpg.de/modellicense.html) | MPI license pages |
| GHUM (Google) | Research-oriented (terms not fully inspected) (UNVERIFIED) | Assume unusable pending check | — |
| MakeHuman | Code AGPL; **exported meshes CC0** when exported from official unmodified app via normal GUI export; CC0 exception void if used as library/scripted mass-export | CC0 output usable incl. adult; one-time manual export of a base mesh then independent development is the safe route | static.makehumancommunity.org/about/license.html (verified) |
| MPFB2 | Code GPL; **assets CC0** — no attribution, no content restrictions | Cleanest open base-mesh source; CC0 covers derivative topologies | static.makehumancommunity.org (verified) |
| MB-Lab | AGPL incl. base characters — propagates to output models | Unusable for closed commercial product | MB-Lab GitHub issues #215/#292 |
| CharMorph | Bases CC-BY / CC0 ("Vitruvian" = CC0) | Usable; check per-base | CG Channel |
| MetaHuman | UE EULA since 5.6; usable outside UE as content | Building a rival character generator on MetaHuman assets likely restricted (UNVERIFIED); porn not barred by UE EULA (UNVERIFIED) | unrealengine.com/eula |
| ARKit 52 blendshape spec | Naming convention, freely implementable | Usable | Apple docs ecosystem |
| Renderotica products (Golden Palace/Dicktator) | Proprietary per-product | Feature-parity reference only — no asset reuse | product pages |

**Base-mesh recommendation:** Author an **original base mesh** (commissioned or in-house; optionally bootstrapped from a one-time official MakeHuman CC0 GUI export / MPFB2 CC0 assets as anatomical reference), original UVs, original skeleton. Avoid the SMPL family entirely. Do not copy Genesis topology/UVs/morph data.

## Sources

Daz official: docs.daz3d.com DSON spec (start/formula/graft), Transfer Utility, JCM/MCM tutorials, Adjust Rigging tutorial, rigidity maps, DzMeshSmoothModifier, DzSkinBinding, Iray Uber reference, daz3d.com/eula + interactive-license-info + daz-studio-premier + blog 4.23 updates, bugs.daz3d.com FAQs, store product pages (Breast/Glute/Fat Control, Breast Utilities G9, Musculature HD, Easy Flex, dForce Soft Breast, HS dForce Hip and Breast V2, Face Control G9, Legacy UVs G9, G9 Starter Essentials).
Daz forums: ERC freeze (284776, 70147, 439127), HD morphs (31158, 180391, 725071), breast-autofit failures (178911, 547231, 674966, 553511), geografts (69084, 24555, 520106, 668201, 243236, 638521), dForce (602121, 249696, 358816, 405461), IK (357776, 355116), TriAx/DQS (252676, 260566), FBX morph rules (568681, 226036), USD/glTF requests (717931, 212646), G9 (598426, 626631, 600406, 598791), G10 (759641, 754716), Premier (703971).
Community: versluis.com (JCMs, ERC freeze, HD preview, Morph Loader, G8→G9), SickleYield tutorials, diffeomorphic.blogspot.com (ERC/FACS, HD + Xin, geoshells, softbody), xin888.gumroad.com/l/daz_hd_morphs + gitlab.com/x190/daz-hd-morphs, xanathon.com, renderguide.com (dForce, autofit, DazToUnreal), renderhub.com forums, 3dshards.com, 3xo.io, zonegfx.com product mirrors (Golden Palace, Dicktator, Breastacular), dartanbeck.com, melindaozel.com ARKit-FACS, pooyadeperson.com ARKit 52.
Engines/formats: dev.epicgames.com (FBX morph pipeline, Interchange, MetaHuman retargeting), forums.unrealengine.com (mannequin IK bones, glTF morph corruption 5.4, MetaHuman EULA), cgchannel.com, KhronosGroup glTF issue #1646.
Licenses (fetched directly): smpl-x.is.tue.mpg.de/modellicense.html, smpl.is.tue.mpg.de, github.com/Meshcapade/wiki, static.makehumancommunity.org/about/license.html, makehumancommunity.org license_explanation, lists.opensource.org MakeHuman thread, github.com/animate1978/MB-Lab issues, unrealengine.com/eula/mhc, github.com/MidnightArrowStudios/dufman.
