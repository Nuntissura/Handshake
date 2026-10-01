# WPV targeted selection for candidate 222b33cc97421388d7be78b8d0a3b0eb577f9928 (hand-written; MT-167 own proof items only,
# CX-EXEC-014 remediation MT-167.json#remediation_20261001_targeted_222b33cc). Evidence only; verdicts come from the union round.
# Core: MT-167-VALIDATOR-TIMING + MT-167-EQUAL-COUNTS (mt167_code_nav_index_cost_tests), existing code-index / code-nav tests
# (knowledge_code_index_tests, knowledge_code_nav_api_tests), and the account test at its real module path (GP-161).
# Native: MT-167-DOWNSTREAM = MT-008 PC-008-05 (ac005/ac006 exact), MT-064 PC-064-04 + MT-065 PC-065-01 (test_fems_interop_proofs whole).
ROUND_SELECTION_SHA=222b33cc97421388d7be78b8d0a3b0eb577f9928
ROUND_SELECTION_MODE=targeted
CORE_TESTS=(mt167_code_nav_index_cost_tests knowledge_code_index_tests knowledge_code_nav_api_tests)
CORE_FILTER='(binary_id(=handshake_core) & test(=api::code_nav_index::account_tests::mounted_index_account_source_witness_revocation_and_workspace_delete)) | binary_id(=handshake_core::mt167_code_nav_index_cost_tests) | binary_id(=handshake_core::knowledge_code_index_tests) | binary_id(=handshake_core::knowledge_code_nav_api_tests)'
NATIVE_TESTS=(test_completion_hover_accesskit test_fems_interop_proofs)
NATIVE_LIB=0
NATIVE_FILTER='(binary_id(=handshake-native::test_completion_hover_accesskit) & (test(=ac005_live_backend_completion_reaches_accesskit_and_stale_gutter) | test(=ac006_live_backend_hover_reaches_accesskit_with_definition_and_doc))) | binary_id(=handshake-native::test_fems_interop_proofs)'
EXTRACTED_CRATES=()
