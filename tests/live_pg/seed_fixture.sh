#!/usr/bin/env bash
# =============================================================================
# WP-KERNEL-012 — live-PG seed fixture for the `requires_pg` batch
# -----------------------------------------------------------------------------
# Seeds a live handshake_core + PostgreSQL backend with the entities the
# `#[ignore = "requires_pg"]` proofs need, then prints an `export HSK_TEST_*=...`
# block the KERNEL_BUILDER can `eval`/source before running the batch.
#
# VERIFIED against a live backend on 2026-07-09 (handshake_core 0.1.0, migration
# 2026): every create call below was exercised and its response id field
# confirmed. The two gotchas found live and handled here:
#   * POST /workspaces REJECTS the x-hsk-* identity headers (400 bad_request):
#     workspace create is the human write-path. So identity headers are sent
#     ONLY to /knowledge/documents (which requires them), never to /workspaces
#     or the loom routes (which ignore them — WriteContext::human).
#   * POST /loom/blocks with a `document_id` returns 500 (HSK-500-LOOM): a loom
#     block cannot be born already bound to a rich document via this route. So
#     block A is created WITHOUT document_id. Consequence: the E2-16 transclusion
#     read-through has no source doc to resolve (SEED-NOTE at the end).
#
# Batch consumers of these env vars (verified against the worktree tests/):
#   MT-044  test_parity_rich_editor.rs   (E2-11..E2-22)
#   MT-044  test_parity_knowledge.rs     (E3-23..E3-36)
#   MT-044  test_parity_search.rs        (E4-37..E4-43)
#   MT-045  test_perf_large_rich.rs      (LR-01..LR-07)
#   MT-045  test_perf_large_knowledge.rs (LK-01..LK-05)
#   MT-045  test_perf_large_code.rs
#   MT-046  test_interconnect_*.rs       (IC-*)
# The backend suite tests/wp_kernel_012_native_editor_routes_pg_tests.rs is
# SELF-SEEDING (its own knowledge_pg() fixture + pg.create_workspace()); it reads
# NO HSK_TEST_* vars and needs nothing from this script — only a live PG + the env
# its knowledge_pg fixture resolves (DATABASE_URL / TEST_DATABASE_URL).
#
# Route/body evidence (file:line, worktree src/):
#   POST  /workspaces                                api/workspaces.rs:569  {name}            -> {"id":..}
#   POST  /knowledge/documents                       api/knowledge_documents.rs:635 {workspace_id,title,content_json} (IDENT req'd) -> {"document":{"rich_document_id":..}}
#   POST  /workspaces/:ws/loom/blocks                api/loom.rs:389  {content_type,title}    -> {"block_id":..}
#   POST  /workspaces/:ws/loom/edges                 api/loom.rs:1754 {source_block_id,target_block_id,edge_type,created_by} -> {"edge_id":..}
#   PATCH /workspaces/:ws/loom/blocks/:id            api/loom.rs:1505 {add_tags:[<tag_hub block id>]}
#   POST  /workspaces/:ws/loom/folders               api/loom.rs:1281 {name,color?}           -> {"folder_id":..}
#   PUT   /workspaces/:ws/loom/folders/:f/blocks/:b  api/loom.rs:1368
#   POST  /workspaces/:ws/loom/canvas-boards         api/loom.rs:3629 {title}                 -> {"block_id":..}
#   POST  /workspaces/:ws/loom/views/definitions     api/loom.rs:3964 {title,definition}      -> {"block":{"block_id":..}}
#   PUT   /workspaces/:ws/loom/journals/:date        api/loom.rs:507
#   POST  /workspaces/:ws/loom/wiki                  api/loom.rs:855  {title,block_ids:[..]}  -> {"projection_id":..,"rendered_content":..}
#   POST  /workspaces/:ws/loom/import                api/loom.rs:1892 {bytes_b64,mime,original_filename} -> {"asset_id":..}
#   view-definition JSON shape                       frontend backend_client.rs:4853 definition_to_json
#
# Idempotency: re-runnable. Each run mints FRESH ids; the printed export block
# reflects the latest run. Pass an existing HSK_TEST_WORKSPACE_ID to seed into it.
# =============================================================================
set -u

BASE="${HSK_TEST_BASE:-http://127.0.0.1:37501}"

# Distinctive, long, fuzzy-tolerant token used as the searchable block title so the
# FTS (E4-37), fuzzy (E4-38, one-char typo), and unlinked-mention (E3-30) proofs
# all key off one seeded string. Also the default HSK_TEST_QUERY.
TOKEN="zebraparityknowledge"

# Identity headers (operator = write-capable). Sent ONLY to /knowledge/documents.
IDENT=(-H "x-hsk-actor-id: kb-seed"
       -H "x-hsk-kernel-task-run-id: kb-seed-run"
       -H "x-hsk-session-run-id: kb-seed-sess"
       -H "x-hsk-actor-kind: operator")

# 1x1 transparent PNG (valid image bytes) for the asset-embed proof (E2-14).
PNG_B64="iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="

# ---- helpers ---------------------------------------------------------------

# json_str KEY (JSON on stdin) -> first  "KEY":"value"  value, or empty.
json_str() {
  grep -oE "\"$1\":\"[^\"]*\"" | head -n1 | sed -E 's/^"[^"]*":"//; s/"$//'
}

# _do METHOD PATH BODY EXTRA_HEADER_ARRAY_NAME
_do() {
  local method="$1" path="$2" body="$3" url resp code out
  url="${BASE}${path}"
  # shellcheck disable=SC2178
  if [ -n "$body" ]; then
    resp=$(curl -sS -X "$method" "$url" "${_HDRS[@]}" \
      -H "Content-Type: application/json" -d "$body" -w $'\n%{http_code}' 2>/dev/null)
  else
    resp=$(curl -sS -X "$method" "$url" "${_HDRS[@]}" -w $'\n%{http_code}' 2>/dev/null)
  fi
  code=$(printf '%s' "$resp" | tail -n1)
  out=$(printf '%s' "$resp" | sed '$d')
  if [ "${code:0:1}" = "2" ]; then
    echo "  ok  $method $path -> HTTP $code" >&2
  else
    echo "  !!  $method $path -> HTTP ${code:-000}: $out" >&2
  fi
  printf '%s' "$out"
}

# req METHOD PATH [BODY]         — no identity headers (workspace + loom routes).
req() { _HDRS=(); _do "$1" "$2" "${3:-}"; }
# reqi METHOD PATH [BODY]        — WITH identity headers (knowledge-document routes).
reqi() { _HDRS=("${IDENT[@]}"); _do "$1" "$2" "${3:-}"; }

echo "=== WP-KERNEL-012 live-PG seed fixture ===" >&2
echo "BASE=$BASE" >&2

# ---- 0. reachability gate --------------------------------------------------
HEALTH=$(curl -sS -o /dev/null -w '%{http_code}' "${BASE}/health" 2>/dev/null || echo 000)
if [ "${HEALTH:0:1}" != "2" ]; then
  echo "FATAL: handshake_core is not reachable at ${BASE}/health (HTTP ${HEALTH})." >&2
  echo "       Start the managed backend + PostgreSQL, then re-run. Nothing seeded." >&2
  exit 1
fi
echo "  ok  GET /health -> HTTP $HEALTH" >&2

# ---- 1. workspace (NO identity headers — human write-path) -----------------
WS="${HSK_TEST_WORKSPACE_ID:-}"
if [ -n "$WS" ]; then
  echo "  reuse workspace $WS (from HSK_TEST_WORKSPACE_ID)" >&2
else
  WS=$(req POST "/workspaces" '{"name":"mt044-live-pg-seed"}' | json_str id)
fi
if [ -z "$WS" ]; then
  echo "FATAL: could not resolve a workspace id. Nothing usable seeded." >&2
  exit 1
fi
echo "  workspace = $WS" >&2

# ---- 2. a rich document (IDENT required) — a real KRD in the workspace ------
# Not attached to block A (a loom block cannot be born with a document_id via the
# create route — HSK-500-LOOM), but seeded so the workspace holds a real rich
# document and to exercise the identity-header write path.
DOC_BODY=$(cat <<JSON
{"workspace_id":"$WS","title":"$TOKEN source doc",
 "content_json":{"type":"doc","content":[
   {"type":"paragraph","content":[{"type":"text","text":"The $TOKEN source document."}]}]}}
JSON
)
DOC_ID=$(reqi POST "/knowledge/documents" "$DOC_BODY" | json_str rich_document_id)
echo "  document  = ${DOC_ID:-<none>}" >&2

# ---- 3. main content block A (HSK_TEST_BLOCK_ID / LOOM_A / QS_BLOCK_ID) -----
# Plain note whose title is $TOKEN: searchable (E4-37/38), a tag/backlink/mention
# anchor, and the graph focus. NO document_id (create rejects it, HSK-500-LOOM).
BLOCK_A=$(req POST "/workspaces/$WS/loom/blocks" \
  "{\"content_type\":\"note\",\"title\":\"$TOKEN\"}" | json_str block_id)
echo "  block A   = ${BLOCK_A:-<none>}" >&2

# ---- 4. block B + edge B->A (backlink + local-graph edge) -------------------
# Satisfies E3-23 local graph (>=1 node + >=1 edge), E3-29 backlinks (>=1), and
# IC-12 (loom_A linked to loom_B; graph/local returns >=2 nodes + >=1 edge).
BLOCK_B=$(req POST "/workspaces/$WS/loom/blocks" \
  '{"content_type":"note","title":"seed-backlink-source"}' | json_str block_id)
if [ -n "$BLOCK_A" ] && [ -n "$BLOCK_B" ]; then
  req POST "/workspaces/$WS/loom/edges" \
    "{\"source_block_id\":\"$BLOCK_B\",\"target_block_id\":\"$BLOCK_A\",\"edge_type\":\"mention\",\"created_by\":\"user\"}" >/dev/null
fi
echo "  block B   = ${BLOCK_B:-<none>} (edge B->A mention)" >&2

# ---- 5. block C: unlinked mention of $TOKEN (no edge to A) ------------------
# Satisfies E3-30 unlinked-mentions (>=1 mentioning block with no formal edge).
BLOCK_C=$(req POST "/workspaces/$WS/loom/blocks" \
  "{\"content_type\":\"note\",\"title\":\"a reference to $TOKEN in passing\"}" | json_str block_id)
echo "  block C   = ${BLOCK_C:-<none>} (unlinked mention of $TOKEN)" >&2

# ---- 6. tag hub T + tag A (the add_tags recipe) ----------------------------
# TagHub is a first-class block (content_type=tag_hub). Tagging = PATCH add_tags
# carrying the TagHub block id (api/loom.rs:1552 requires the target be tag_hub).
TAG_HUB=$(req POST "/workspaces/$WS/loom/blocks" \
  '{"content_type":"tag_hub","title":"seed-tag"}' | json_str block_id)
if [ -n "$BLOCK_A" ] && [ -n "$TAG_HUB" ]; then
  req PATCH "/workspaces/$WS/loom/blocks/$BLOCK_A" "{\"add_tags\":[\"$TAG_HUB\"]}" >/dev/null
fi
echo "  tag hub   = ${TAG_HUB:-<none>} (A tagged via add_tags; query GET /loom/tags/$TAG_HUB/blocks)" >&2

# ---- 7. folder F (HSK_TEST_FOLDER_ID) + add A (breadcrumb spine) ------------
FOLDER=$(req POST "/workspaces/$WS/loom/folders" \
  '{"name":"seed-folder","color":"#3366cc"}' | json_str folder_id)
if [ -n "$FOLDER" ] && [ -n "$BLOCK_A" ]; then
  req PUT "/workspaces/$WS/loom/folders/$FOLDER/blocks/$BLOCK_A" '{}' >/dev/null
fi
echo "  folder    = ${FOLDER:-<none>} (A added -> breadcrumbs)" >&2

# ---- 8. canvas board (HSK_TEST_BOARD_ID) -----------------------------------
BOARD=$(req POST "/workspaces/$WS/loom/canvas-boards" '{"title":"seed-canvas"}' | json_str block_id)
echo "  board     = ${BOARD:-<none>}" >&2

# ---- 9. views: kanban (HSK_TEST_VIEW_ID) + calendar (HSK_TEST_VIEW_ID_CALENDAR)
# E3-35 (kanban card-move) and E3-36 (calendar) BOTH read HSK_TEST_VIEW_ID but
# need different view kinds — an inherent single-env conflict in the suite. Both
# are seeded; HSK_TEST_VIEW_ID -> kanban, HSK_TEST_VIEW_ID_CALENDAR -> calendar.
VIEW_KANBAN=$(req POST "/workspaces/$WS/loom/views/definitions" \
  '{"title":"seed-kanban","definition":{"kind":"kanban","group_by":{"kind":"tag"}}}' | json_str block_id)
VIEW_CAL=$(req POST "/workspaces/$WS/loom/views/definitions" \
  '{"title":"seed-calendar","definition":{"kind":"calendar"}}' | json_str block_id)
echo "  view kbn  = ${VIEW_KANBAN:-<none>}" >&2
echo "  view cal  = ${VIEW_CAL:-<none>}" >&2

# ---- 10. daily journal 2026-06-26 (E3-36 calendar surfaces it) --------------
req PUT "/workspaces/$WS/loom/journals/2026-06-26" '{}' >/dev/null
echo "  journal   = 2026-06-26" >&2

# ---- 11. wiki projection (HSK_TEST_WIKI_PROJECTION_ID), compiled from A ------
WIKI_RESP=$(req POST "/workspaces/$WS/loom/wiki" \
  "{\"title\":\"seed-wiki\",\"block_ids\":[\"${BLOCK_A}\"]}")
WIKI_ID=$(printf '%s' "$WIKI_RESP" | json_str projection_id)
echo "  wiki proj = ${WIKI_ID:-<none>}" >&2
if printf '%s' "$WIKI_RESP" | grep -qE '"rendered_content":"[^"]'; then
  echo "  (wiki rendered_content is non-empty -> E3-32 satisfiable)" >&2
else
  echo "  SEED-WARN: wiki rendered_content looks empty; E3-32 may still gate." >&2
fi

# ---- 12. asset (HSK_TEST_ASSET_ID) via base64 import ------------------------
ASSET=$(req POST "/workspaces/$WS/loom/import" \
  "{\"bytes_b64\":\"$PNG_B64\",\"mime\":\"image/png\",\"original_filename\":\"seed.png\"}" | json_str asset_id)
echo "  asset     = ${ASSET:-<none>}" >&2

# ---- 13. locus work-packet (HSK_TEST_LOCUS_WP_ID) via psql ------------------
# The Locus reverse-lookup proof (MT-074 OP-03: other_pillar_op03_locus_resolve_
# reverse_live) resolves a REAL row in product `public.work_packets`. That table
# is kernel/gov state with NO create-route, so it is seeded directly via psql on
# the managed cluster (default 127.0.0.1:5544, superuser postgres, db handshake).
# The Locus route itself is also proven by route1_locus_work_packet_resolve, which
# self-seeds its own work_packet; this step makes the OP-03 interop scenario
# reproducible instead of a 404 SEED-GAP. Override conn via HSK_PGHOST/PGPORT/
# PGUSER/PGDB/HSK_PSQL_BIN. Idempotent (ON CONFLICT DO NOTHING).
LOCUS_WP="${HSK_TEST_LOCUS_WP_ID:-WP-KERNEL-012}"
PSQL_BIN="${HSK_PSQL_BIN:-}"
if [ -z "$PSQL_BIN" ]; then
  for c in "/c/Program Files/PostgreSQL/16/bin/psql" "psql"; do
    if command -v "$c" >/dev/null 2>&1 || [ -x "$c" ]; then PSQL_BIN="$c"; break; fi
  done
fi
WP_SEEDED=""
if [ -n "$PSQL_BIN" ]; then
  if "$PSQL_BIN" -h "${HSK_PGHOST:-127.0.0.1}" -p "${HSK_PGPORT:-5544}" \
       -U "${HSK_PGUSER:-postgres}" -d "${HSK_PGDB:-handshake}" -v ON_ERROR_STOP=1 -tAc \
       "INSERT INTO public.work_packets (wp_id,version,title,status,priority,task_board_status,reporter,created_at,updated_at,vector_clock,metadata) VALUES ('$LOCUS_WP',1,'Handshake Native Editor Parity (Obsidian + VS Code)','in_progress',1,'in_progress','operator','2026-07-08T00:00:00Z','2026-07-08T00:00:00Z','{\"operator\":1}','{}') ON CONFLICT (wp_id) DO NOTHING;" >/dev/null 2>&1; then
    WP_SEEDED="$LOCUS_WP"
    echo "  locus wp  = $LOCUS_WP (seeded into public.work_packets)" >&2
  else
    echo "  SEED-WARN: work_packet insert failed (psql/PG auth?); OP-03 stays gated." >&2
  fi
else
  echo "  SEED-WARN: no psql found; HSK_TEST_LOCUS_WP_ID left for external seed." >&2
fi

# =============================================================================
# EXPORT BLOCK  (source/eval this before the cargo batch)
# =============================================================================
QS="${BLOCK_A}"   # HSK_TEST_QS_BLOCK_ID falls back to HSK_TEST_BLOCK_ID anyway.

echo
echo "# ---- 8< ---- copy/eval from here (WP-KERNEL-012 live-PG seed) ---- 8< ----"
echo "export HSK_TEST_BASE=$BASE"
echo "export HSK_TEST_WORKSPACE_ID=$WS"
echo "export HSK_TEST_QUERY=$TOKEN"
echo "export HSK_TEST_CONTENT_TYPE=note"
[ -n "$BLOCK_A" ]     && echo "export HSK_TEST_BLOCK_ID=$BLOCK_A"           || echo "# SEED-GAP HSK_TEST_BLOCK_ID (block A create failed)"
[ -n "$BLOCK_A" ]     && echo "export HSK_TEST_LOOM_A=$BLOCK_A"             || echo "# SEED-GAP HSK_TEST_LOOM_A"
[ -n "$QS" ]          && echo "export HSK_TEST_QS_BLOCK_ID=$QS"             || echo "# SEED-GAP HSK_TEST_QS_BLOCK_ID"
[ -n "$FOLDER" ]      && echo "export HSK_TEST_FOLDER_ID=$FOLDER"           || echo "# SEED-GAP HSK_TEST_FOLDER_ID"
[ -n "$BOARD" ]       && echo "export HSK_TEST_BOARD_ID=$BOARD"             || echo "# SEED-GAP HSK_TEST_BOARD_ID"
[ -n "$VIEW_KANBAN" ] && echo "export HSK_TEST_VIEW_ID=$VIEW_KANBAN"        || echo "# SEED-GAP HSK_TEST_VIEW_ID"
[ -n "$VIEW_CAL" ]    && echo "export HSK_TEST_VIEW_ID_CALENDAR=$VIEW_CAL"
[ -n "$WIKI_ID" ]     && echo "export HSK_TEST_WIKI_PROJECTION_ID=$WIKI_ID" || echo "# SEED-GAP HSK_TEST_WIKI_PROJECTION_ID"
[ -n "$ASSET" ]       && echo "export HSK_TEST_ASSET_ID=$ASSET"             || echo "# SEED-GAP HSK_TEST_ASSET_ID"
[ -n "$WP_SEEDED" ]   && echo "export HSK_TEST_LOCUS_WP_ID=$WP_SEEDED"      || echo "# SEED-GAP HSK_TEST_LOCUS_WP_ID (psql work_packet seed unavailable)"
echo "# ---- 8< ---- end export block ---- 8< ----"

# =============================================================================
# HONEST SEED-GAPS / NOTES  (these sub-proofs stay requires_pg-gated by design)
# =============================================================================
echo
echo "# SEED-GAP HSK_TEST_TRANSCLUSION_HEAD (MT-045 LR-05): a 50-hop transclusion chain whose"
echo "#   each block resolves source_document_id -> the next hop. Not a simple create; needs a"
echo "#   dedicated chain-builder. Left UNSET -> perf_lr05_transclusion_chain_live stays gated."
echo "# SEED-GAP HSK_TEST_FIND_STRING (MT-044 E4-42): asserts >=3 DISTINCT file paths from a"
echo "#   file-faceted search. Needs a code-nav-indexed corpus / file-type blocks with distinct"
echo "#   paths (POST /workspaces/:ws/code-nav/index), not plain notes. Left at default -> gated."
echo "# SEED-GAP semantic search (MT-044 E4-39): needs pgvector + a configured embedding model +"
echo "#   the mt250 fixture. No env can seed the model -> parity_semantic_search stays gated."
echo "# SEED-NOTE E2-16 transclusion read-through (parity_transclusion_read_through): the seeded"
echo "#   HSK_TEST_BLOCK_ID has NO source rich document (loom-block create rejects document_id with"
echo "#   HSK-500-LOOM), so /loom/blocks/{id}/transclusion returns resolved=false. That ONE sub-proof"
echo "#   stays gated; all other HSK_TEST_BLOCK_ID proofs (search/backlinks/graph/tags/wikilink) pass."
echo "# SEED-NOTE E4-40 faceted filter: HSK_TEST_CONTENT_TYPE=note is correct, but the proof uses an"
echo "#   EMPTY query + facet, which returns 0 hits on this build (empty-query FTS matches nothing)."
echo "# SEED-NOTE HSK_TEST_TAG_BLOCK_ID / HSK_TEST_SEARCH_PRESEEDED / HSK_TEST_FOLDERS_PRESEEDED"
echo "#   (MT-045 LK-03/04/05): deliberately UNSET. Those perf proofs SELF-SEED exact corpora"
echo "#   (5000 tagged / 5000 search / 200 folders) and assert exact counts; a partial pre-seed"
echo "#   would FAIL the count assertions. Leave unset -> self-seed."
echo "# SEED-NOTE HSK_TEST_VIEW_ID: E3-35 (kanban) + E3-36 (calendar) share one var. It points at"
echo "#   the kanban view; to run E3-36:  export HSK_TEST_VIEW_ID=\$HSK_TEST_VIEW_ID_CALENDAR"

echo >&2
echo "=== seed complete ===" >&2
