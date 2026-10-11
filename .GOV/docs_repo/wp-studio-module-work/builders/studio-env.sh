# Studio module team build environment (WP-KERNEL-STUDIO). Source AFTER the leading `cd` so the gameplan hook
# resolves the Studio pointer:  cd "<dir>" && source "<this file>" && cargo ...
# Operator 2026-10-11: no cargo builds/tests/checks on C:; all build output, TMP and TEMP under Handshake_Artifacts/<WP>/<owner>.
_studio_art="D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO"
_studio_owner="${STUDIO_OWNER:-studio-team-dev}"
export CARGO_TARGET_DIR="${_studio_art}/${_studio_owner}/target"
export TMP="${_studio_art}/${_studio_owner}/tmp"
export TEMP="${TMP}"
export TMPDIR="${TMP}"
export CARGO_PROFILE_DEV_DEBUG=line-tables-only
export CARGO_PROFILE_TEST_DEBUG=line-tables-only
mkdir -p "${CARGO_TARGET_DIR}" "${TMP}"
