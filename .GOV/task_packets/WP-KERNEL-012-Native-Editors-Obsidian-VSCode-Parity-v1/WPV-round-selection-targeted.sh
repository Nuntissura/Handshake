# WPV targeted selection for candidate 222b33cc97421388d7be78b8d0a3b0eb577f9928, ATTEMPT 2 (hand-written; MT-167 own core
# proof items only; CX-EXEC-014 remediation MT-167.json#remediation_20261002_targeted_222b33cc_a2). Evidence only, no verdict.
# Narrowed per orchestrator 2026-10-02: only the core items attempt 1 did not run. Native downstream (MT-008 PC-008-05,
# MT-064 PC-064-04, MT-065 PC-065-01) is NOT rerun: attempt-1 native results (junit-<sha>-native-targeted-a1.xml) stand as
# the AC-167-4 evidence. NATIVE_SKIP=1 therefore also skips the native build, the backend build and the native run.
# Core: MT-167-VALIDATOR-TIMING + MT-167-EQUAL-COUNTS (mt167_code_nav_index_cost_tests), MT-167-VALIDATOR-TIMING's
# "existing code-nav / knowledge-code-index tests" (knowledge_code_index_tests, knowledge_code_nav_api_tests), and the
# account test at its real module path (GP-161).
ROUND_SELECTION_SHA=222b33cc97421388d7be78b8d0a3b0eb577f9928
ROUND_SELECTION_MODE=targeted
NATIVE_SKIP=1
CORE_TESTS=(mt167_code_nav_index_cost_tests knowledge_code_index_tests knowledge_code_nav_api_tests)
CORE_FILTER='(binary_id(=handshake_core) & test(=api::code_nav_index::account_tests::mounted_index_account_source_witness_revocation_and_workspace_delete)) | binary_id(=handshake_core::mt167_code_nav_index_cost_tests) | binary_id(=handshake_core::knowledge_code_index_tests) | binary_id(=handshake_core::knowledge_code_nav_api_tests)'
NATIVE_TESTS=()
NATIVE_LIB=0
EXTRACTED_CRATES=()
