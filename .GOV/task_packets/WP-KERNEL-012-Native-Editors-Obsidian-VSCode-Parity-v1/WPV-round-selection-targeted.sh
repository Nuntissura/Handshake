# WPV targeted evidence selection for the MT-170 current-state resolver diagnostic.
# This is procedural recovery evidence for AC-170-1 only; it is not acceptance or a verdict.
ROUND_SELECTION_SHA=3e5f4024806b3052efa1917cd57cd9a45b0462ae
ROUND_SELECTION_MODE=targeted
NATIVE_SKIP=1
CORE_TESTS=(knowledge_documents_api_tests)
CORE_FILTER='(binary_id(=handshake_core::knowledge_documents_api_tests) & test(=mt170_wikilink_to_standalone_loom_block_projects_one_mention_edge))'
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER=''
EXTRACTED_CRATES=()
