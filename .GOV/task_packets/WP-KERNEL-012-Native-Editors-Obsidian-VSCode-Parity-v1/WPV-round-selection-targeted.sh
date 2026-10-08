# WPV fresh full own MT-111 acceptance selection after the approved manual/version repair.
# Six named cases satisfy the own runtime floor; independent verdict follows the contract.
ROUND_SELECTION_SHA=6fc38af7ceb822ae87e425d510d44c1d509b40e5
ROUND_SELECTION_MODE=targeted
ROUND_SELECTION_OUTPUT_SUFFIX=-mt111-r3
CORE_SKIP=1
CORE_LIB=0
CORE_TESTS=()
CORE_FILTER=''
NATIVE_SKIP=0
NATIVE_TESTS=(test_flight_recorder_authz test_event_emitter test_manual_content)
NATIVE_LIB=1
NATIVE_FILTER='(binary_id(=handshake-native) & test(=event_emitter::tests::ingest_url_is_workspace_scoped_from_the_event)) | (binary_id(=handshake-native::test_flight_recorder_authz) & test(=mt111_flight_recorder_authorization_boundary_real_surrealdb)) | (binary_id(=handshake-native::test_event_emitter) & test(/^(post_body_matches_verified_native_editor_schema|ingest_url_is_workspace_scoped_and_credential_comes_from_the_on_disk_binding|event_emitter_native_editor_round_trip)$/)) | (binary_id(=handshake-native::test_manual_content) & test(=mt104_topics_exist_and_include_no_context_runtime_facts))'
EXTRACTED_CRATES=()
