# MT168 focused exact163 upgrade reproduction; original51c passing proof retains original bindings
ROUND_SELECTION_SHA=34aa1a4ddf9830a88ab75c358d51b864ebdee6d7
ROUND_SELECTION_MODE=targeted
ROUND_SELECTION_OUTPUT_SUFFIX=-mt168-exact163-r7
CORE_SKIP=0
CORE_CHECK_LIB=0
CORE_LIB=1
CORE_TESTS=()
CORE_FILTER='(binary_id(=handshake_core) & test(=storage::surreal::schema::tests::mt168_revision_163_upgrade_preserves_data_and_restarts_current))'
NATIVE_SKIP=1
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER='none()'
EXTRACTED_CRATES=()
