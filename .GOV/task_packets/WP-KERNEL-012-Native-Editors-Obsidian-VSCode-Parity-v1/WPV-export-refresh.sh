#!/bin/bash
# WP-KERNEL-012 stable round source path ([VPX-011] text, Operator-approved "WP-012 scripts now", 2026-10-01).
# Sourced by WPV-union-round.sh and MT-032-phase-probe.sh (CX-VAL-007 entrypoint config).
#
# One fixed per-owner export directory: <owner-target>/export-current, refreshed IN PLACE to the frozen
# candidate at round start. Cargo hashes path crates outside a manifest's workspace root by absolute
# source path, so a new path per candidate is a cold rebuild; a fixed path keeps unchanged crates warm.
#
#   wpv_refresh_export_current <candidate-sha> <worktree> <owner-target-dir> <log-tag> [dry-run]
#
# Steps: (1) read export-current.identity (previous sha, tree, file count, manifest sha256) and remove it
# (a crash mid-refresh leaves no identity -> full replace next time); (2) incremental: apply
# `git diff --name-status -z --no-renames <prev> <candidate>`: delete removed files, extract added and
# modified files from `git archive <candidate> -- <paths>` with `tar -m` (current mtime, so Cargo never
# reuses stale output); unchanged files untouched; full replace when the identity is missing, partial or
# mismatched (own disposable output, CX-984-006); (3) clear the export's own round outputs
# (*/target/nextest) so no stale JUnit survives; (4) verify the file set equals `git ls-tree -r <candidate>`
# and every file's bytes equal that blob's `git archive <candidate>` bytes (WPV-export-verify.py; blob ids
# cannot be re-hashed from files after irreversible autocrlf conversion) and no other file exists (only the
# MT-165 harness `.git` pointer is excluded); (5) write export-current.identity last.
# Dry-run mode changes nothing: it reports the planned mode and diff and, when the export already holds the
# candidate, verifies it. Functional proof of the full and incremental paths uses a scratch owner root.
# Sets: WPV_EXPORT (path), WPV_EXPORT_IDENTITY (file), WPV_EXPORT_IDENTITY_SHA256, WPV_EXPORT_MODE.

WPV_EXPORT_HELPER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

wpv_export_verify() {
  # $1 sha  $2 worktree  $3 export dir  -> 0 when the export equals the candidate exactly
  # (file set == git ls-tree -r; every file's bytes == the candidate's git archive bytes; see WPV-export-verify.py)
  local out rc=0
  out="$(python "$WPV_EXPORT_HELPER_DIR/WPV-export-verify.py" "$2" "$1" "$3" 2>&1)" || rc=$?
  echo "$out" | sed 's/^/[export] /'
  [ "$rc" -eq 0 ] || return 1
  WPV_VERIFIED_COUNT="$(echo "$out" | sed -n 's/.*VERIFY_OK files=\([0-9]*\).*/\1/p')"
  WPV_VERIFIED_MANIFEST="$(echo "$out" | sed -n 's/.*manifest_sha256=\([0-9a-f]*\).*/\1/p')"
  [ -n "$WPV_VERIFIED_COUNT" ] && [ -n "$WPV_VERIFIED_MANIFEST" ]
}

wpv_refresh_export_current() {
  local sha="$1" wt="$2" root="$3" tag="${4:-[export]}" dry="${5:-}"
  local export="$root/export-current" ident="$root/export-current.identity"
  local prev="" mode="" t0 t1 tree_id refresh_seconds refreshed_utc identity_hash_line identity_hash
  WPV_EXPORT="$export"; WPV_EXPORT_IDENTITY="$ident"
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { echo "$tag FATAL: candidate must be a full sha"; return 2; }
  t0=$(date +%s)
  if [ -f "$ident" ]; then
    prev="$(sed -n 's/^candidate_sha=//p' "$ident")"
    [[ "$prev" =~ ^[0-9a-f]{40}$ ]] && grep -q '^state=complete$' "$ident" || prev=""
  fi
  if [ -n "$prev" ] && [ -d "$export" ] && git -C "$wt" cat-file -e "$prev^{commit}" 2>/dev/null; then
    mode="incremental"
  else
    mode="full"
  fi
  if [ -n "$dry" ]; then
    echo "$tag DRY-RUN previous=${prev:-none} candidate=$sha planned_mode=$mode"
    if [ "$mode" = incremental ]; then
      echo "$tag DRY-RUN diff: $(git -C "$wt" diff --name-status --no-renames "$prev" "$sha" | awk '{c[$1]++} END {for (k in c) printf "%s=%d ", k, c[k]}')"
      if [ "$prev" = "$sha" ]; then
        wpv_export_verify "$sha" "$wt" "$export" && echo "$tag DRY-RUN verify OK files=$WPV_VERIFIED_COUNT manifest=$WPV_VERIFIED_MANIFEST" || return 1
      fi
    fi
    return 0
  fi
  rm -f "$ident"
  if [ "$mode" = incremental ]; then
    local status path n_del=0 n_put=0 lst
    lst="$(mktemp)"
    while IFS= read -r -d '' status && IFS= read -r -d '' path; do
      case "$status" in
        D) rm -f -- "$export/$path"; n_del=$((n_del+1)) ;;
        A|M|T) printf '%s\0' "$path" >> "$lst"; n_put=$((n_put+1)) ;;
        *) echo "$tag unexpected diff status $status for $path; falling back to full replace"; mode="full"; break ;;
      esac
    done < <(git -C "$wt" diff --name-status -z --no-renames "$prev" "$sha")
    if [ "$mode" = incremental ] && [ "$n_put" -gt 0 ]; then
      # batches of 200 paths: archive exactly those blobs from the candidate, extract with current mtime
      xargs -0 -n 200 bash -c 'set -o pipefail; wt="$1"; sha="$2"; dst="$3"; shift 3; git -C "$wt" archive "$sha" -- "$@" | tar -x -m -C "$dst"' _ "$wt" "$sha" "$export" < "$lst" \
        || { echo "$tag incremental extraction failed; falling back to full replace"; mode="full"; }
    fi
    rm -f "$lst"
    # remove directories left empty by deletions (never the export root)
    find "$export" -mindepth 1 -type d -empty ! -path '*/target*' -delete 2>/dev/null || true
    echo "$tag incremental refresh $prev -> $sha: deleted=$n_del extracted=$n_put"
  fi
  if [ "$mode" = full ]; then
    case "$export" in */export-current) ;; *) echo "$tag FATAL: refusing to replace unexpected path $export"; return 2 ;; esac
    if [ -e "$export/.git" ]; then echo "$tag FATAL: export holds .git (session repo preserved for review); not replacing"; return 2; fi
    rm -rf -- "$export"
    mkdir -p "$export"
    git -C "$wt" archive "$sha" | tar -x -m -C "$export" || { echo "$tag FATAL: full archive extraction failed"; return 2; }
    echo "$tag full replace -> $sha"
  fi
  # (3) clear the export's own round outputs so no stale JUnit/nextest state survives
  find "$export" -type d -path '*/target/nextest' -prune -exec rm -rf {} + 2>/dev/null || true
  # (4) verify
  wpv_export_verify "$sha" "$wt" "$export" || { echo "$tag FATAL: export-current does not equal candidate $sha (identity not written)"; return 2; }
  t1=$(date +%s)
  # (5) identity last
  tree_id="$(git -C "$wt" rev-parse "$sha^{tree}")" || { echo "$tag FATAL: cannot resolve candidate tree for export identity"; return 2; }
  [[ "$tree_id" =~ ^[0-9a-f]{40}$ ]] || { echo "$tag FATAL: candidate tree id is malformed"; return 2; }
  [[ "$WPV_VERIFIED_COUNT" =~ ^[0-9]+$ ]] || { echo "$tag FATAL: verified export file count is malformed"; return 2; }
  [[ "$WPV_VERIFIED_MANIFEST" =~ ^[0-9a-f]{64}$ ]] || { echo "$tag FATAL: verified export manifest is malformed"; return 2; }
  refresh_seconds=$((t1 - t0))
  refreshed_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)" || { echo "$tag FATAL: cannot timestamp export identity"; return 2; }
  [[ "$refreshed_utc" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] || { echo "$tag FATAL: export identity timestamp is malformed"; return 2; }
  {
    echo "schema=handshake.wpv.export-identity@1"
    echo "candidate_sha=$sha"
    echo "tree_id=$tree_id"
    echo "file_count=$WPV_VERIFIED_COUNT"
    echo "manifest_sha256=$WPV_VERIFIED_MANIFEST"
    echo "previous_sha=${prev:-none}"
    echo "refresh_mode=$mode"
    echo "refresh_seconds=$refresh_seconds"
    echo "refreshed_utc=$refreshed_utc"
    echo "state=complete"
  } > "$ident.tmp" || { echo "$tag FATAL: cannot write export identity temporary file"; rm -f "$ident.tmp"; return 2; }
  mv -f "$ident.tmp" "$ident" || { echo "$tag FATAL: cannot publish export identity"; rm -f "$ident.tmp"; return 2; }
  [ -f "$ident" ] || { echo "$tag FATAL: export identity is missing after publication"; return 2; }
  grep -Fqx -- "schema=handshake.wpv.export-identity@1" "$ident" || { echo "$tag FATAL: export identity readback schema mismatch"; return 2; }
  grep -Fqx -- "candidate_sha=$sha" "$ident" || { echo "$tag FATAL: export identity readback candidate mismatch"; return 2; }
  grep -Fqx -- "tree_id=$tree_id" "$ident" || { echo "$tag FATAL: export identity readback tree mismatch"; return 2; }
  grep -Fqx -- "file_count=$WPV_VERIFIED_COUNT" "$ident" || { echo "$tag FATAL: export identity readback file count mismatch"; return 2; }
  grep -Fqx -- "manifest_sha256=$WPV_VERIFIED_MANIFEST" "$ident" || { echo "$tag FATAL: export identity readback manifest mismatch"; return 2; }
  grep -Fqx -- "previous_sha=${prev:-none}" "$ident" || { echo "$tag FATAL: export identity readback previous SHA mismatch"; return 2; }
  grep -Fqx -- "refresh_mode=$mode" "$ident" || { echo "$tag FATAL: export identity readback mode mismatch"; return 2; }
  grep -Fqx -- "refresh_seconds=$refresh_seconds" "$ident" || { echo "$tag FATAL: export identity readback duration mismatch"; return 2; }
  grep -Fqx -- "refreshed_utc=$refreshed_utc" "$ident" || { echo "$tag FATAL: export identity readback timestamp mismatch"; return 2; }
  grep -Fqx -- 'state=complete' "$ident" || { echo "$tag FATAL: export identity readback state mismatch"; return 2; }
  identity_hash_line="$(sha256sum "$ident")" || { echo "$tag FATAL: cannot hash export identity"; return 2; }
  identity_hash="${identity_hash_line%% *}"
  [[ "$identity_hash" =~ ^[0-9a-f]{64}$ ]] || { echo "$tag FATAL: export identity digest is malformed"; return 2; }
  WPV_EXPORT_MODE="$mode"
  WPV_EXPORT_IDENTITY_SHA256="$identity_hash"
  echo "$tag export identity ($ident sha256=$WPV_EXPORT_IDENTITY_SHA256):"
  sed "s/^/$tag   /" "$ident"
  return 0
}
