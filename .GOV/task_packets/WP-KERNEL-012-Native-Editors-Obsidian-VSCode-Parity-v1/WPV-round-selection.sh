# MT-170 remaining acceptance, including original full MT-046 PC-046-01/03.
ROUND_SELECTION_SHA=1ad04e431fa1c8a6b71533122bdfd532111a0557
ROUND_SELECTION_MODE=full
CORE_LIB=0
CORE_TESTS=(knowledge_documents_api_tests)
CORE_FILTER='binary_id(=handshake_core::knowledge_documents_api_tests) & (test(=mt170_wikilink_to_standalone_loom_block_projects_one_mention_edge) | test(=mt032_rich_documents_are_addressable_and_target_backlinks_are_inbound) | test(=mt032_save_delete_and_backlink_rebuild_delete_races_do_not_resurrect))'
NATIVE_TESTS=(test_interconnect_ckc_to_note test_interconnect_loom_backlink_search)
NATIVE_LIB=0
NATIVE_FILTER='(binary_id(=handshake-native::test_interconnect_ckc_to_note) | binary_id(=handshake-native::test_interconnect_loom_backlink_search)) & test(/^interconnect_ic(0[1-5]|1[0-4])_/)'
EXTRACTED_CRATES=(handshake_document)