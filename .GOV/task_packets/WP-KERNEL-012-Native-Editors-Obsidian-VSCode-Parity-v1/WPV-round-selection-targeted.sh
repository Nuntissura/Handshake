# MT168 CURRENT catalog measurement: five direct checks; no runtime acceptance
ROUND_SELECTION_SHA=92841e94ba2fad87f7c9980098c2dfab10106f74
ROUND_SELECTION_MODE=targeted
ROUND_SELECTION_OUTPUT_SUFFIX=-mt168-pinmeasure
CORE_SKIP=0
CORE_CHECK_LIB=0
CORE_LIB=1
CORE_TESTS=()
CORE_FILTER='binary_id(=handshake_core) & test(/^(storage::surreal::schema::tests::declarative_schema_catalog_is_complete_and_content_sensitive|storage::surreal::schema::tests::mt139_current_schema_info_pin_matches_fresh_mem_catalog|storage::surreal::schema::tests::mt109_loom_catalog_dependencies_are_complete_and_deterministic|storage::surreal::schema::tests::mt138_canonical_atelier_catalog_fingerprint_matches_compiled_pin|storage::surreal::schema::tests::mt109_authority_catalog_pins_are_deterministic)$/)'
NATIVE_SKIP=1
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER=''
EXTRACTED_CRATES=()
