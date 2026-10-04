# WPV evidence-only sixth full-schema Atelier parity test on candidate c0afe905114738812227f26ab45fc83705ebd841.
# Core library only; this single-test proof does not complete semantic acceptance.
ROUND_SELECTION_SHA=c0afe905114738812227f26ab45fc83705ebd841
ROUND_SELECTION_MODE=targeted
NATIVE_SKIP=1
CORE_TESTS=()
CORE_FILTER='(binary_id(=handshake_core) & test(=storage::surreal::schema::tests::mt138_full_schema_atelier_noop_matches_bounded_projection))'
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER=''
EXTRACTED_CRATES=()
