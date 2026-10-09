# MT168 affected mounted account and downstream proof; original greens retain original bindings
ROUND_SELECTION_SHA=292d7a1c8bd0b24f9bf0be07746b08e7b8d7f0a4
ROUND_SELECTION_MODE=targeted
ROUND_SELECTION_OUTPUT_SUFFIX=-mt168-link-r14
CORE_SKIP=0
CORE_CHECK_LIB=0
CORE_LIB=1
CORE_TESTS=(mt167_code_nav_index_cost_tests)
CORE_FILTER='(binary_id(=handshake_core) & test(=api::code_nav_index::account_tests::mounted_index_account_source_witness_revocation_and_workspace_delete)) | binary_id(=handshake_core::mt167_code_nav_index_cost_tests)'
NATIVE_SKIP=1
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER='none()'
EXTRACTED_CRATES=()
