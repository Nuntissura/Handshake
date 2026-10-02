# WPV targeted selection for candidate dbdae422e64113b18975f90f87ed34738f853a04, PHASE A = CORE (hand-written; MT-167 own core
# proof items; CX-EXEC-014 remediation MT-167.json#remediation_20261002_targeted_dbdae422). Evidence only, no verdict.
# Orchestrator cadence 2026-10-02: core first; native downstream (phase B, CORE_SKIP=1) only if every MT-167 core item passes.
# Core: MT-167-VALIDATOR-TIMING + MT-167-EQUAL-COUNTS incl. the new pass-2 == pass-1 totals assertion
# (mt167_code_nav_index_cost_tests), the "existing code-nav / knowledge-code-index tests" (knowledge_code_index_tests,
# knowledge_code_nav_api_tests), and the account test at its real module path (GP-161).
# NATIVE_SKIP=1 also skips the native build, the backend build and the native run in this phase.
ROUND_SELECTION_SHA=dbdae422e64113b18975f90f87ed34738f853a04
ROUND_SELECTION_MODE=targeted
NATIVE_SKIP=1
CORE_TESTS=(mt167_code_nav_index_cost_tests knowledge_code_index_tests knowledge_code_nav_api_tests)
CORE_FILTER='(binary_id(=handshake_core) & test(=api::code_nav_index::account_tests::mounted_index_account_source_witness_revocation_and_workspace_delete)) | binary_id(=handshake_core::mt167_code_nav_index_cost_tests) | binary_id(=handshake_core::knowledge_code_index_tests) | binary_id(=handshake_core::knowledge_code_nav_api_tests)'
NATIVE_TESTS=()
NATIVE_LIB=0
EXTRACTED_CRATES=()
