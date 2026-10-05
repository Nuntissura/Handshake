# WPV targeted evidence selection for the MT-170 current-state resolver diagnostic.
# This is procedural recovery evidence for AC-170-1 only; it is not acceptance or a verdict.
ROUND_SELECTION_SHA=4b8d5546e9970e99888ccb9b0a9f29dac5fb937b
ROUND_SELECTION_MODE=targeted
NATIVE_SKIP=1
CORE_TESTS=(knowledge_documents_api_tests)
CORE_FILTER='(binary_id(=handshake_core::knowledge_documents_api_tests) & test(=mt170_wikilink_to_standalone_loom_block_projects_one_mention_edge))'
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER=''
EXTRACTED_CRATES=()
