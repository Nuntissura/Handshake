# WPV diagnostic-only MT170 resolver observation on candidate c0afe905114738812227f26ab45fc83705ebd841.
# One focused integration test; this is not MT170 acceptance or a union proof.
ROUND_SELECTION_SHA=c0afe905114738812227f26ab45fc83705ebd841
ROUND_SELECTION_MODE=targeted
NATIVE_SKIP=1
CORE_TESTS=(knowledge_documents_api_tests)
CORE_FILTER='(binary_id(=handshake_core::knowledge_documents_api_tests) & test(=mt170_wikilink_to_standalone_loom_block_projects_one_mention_edge))'
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER=''
EXTRACTED_CRATES=()
