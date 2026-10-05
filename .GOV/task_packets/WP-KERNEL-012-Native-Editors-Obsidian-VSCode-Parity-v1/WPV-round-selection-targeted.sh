# WPV post-163 direct/static current-pin measurement on candidate 9858d461bf33a341871327ebf4c4a7badad3300a.
# Five bounded pin inputs; full-schema parity remains a separate sixth measurement.
ROUND_SELECTION_SHA=9858d461bf33a341871327ebf4c4a7badad3300a
ROUND_SELECTION_MODE=targeted
NATIVE_SKIP=1
CORE_TESTS=()
CORE_FILTER='(binary_id(=handshake_core) & (test(=storage::surreal::schema::tests::declarative_schema_catalog_is_complete_and_content_sensitive) | test(=storage::surreal::schema::tests::mt139_current_schema_info_pin_matches_fresh_mem_catalog) | test(=storage::surreal::schema::tests::mt109_loom_catalog_dependencies_are_complete_and_deterministic) | test(=storage::surreal::schema::tests::mt138_canonical_atelier_catalog_fingerprint_matches_compiled_pin) | test(=storage::surreal::schema::tests::mt109_authority_catalog_pins_are_deterministic)))'
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER=''
EXTRACTED_CRATES=()
