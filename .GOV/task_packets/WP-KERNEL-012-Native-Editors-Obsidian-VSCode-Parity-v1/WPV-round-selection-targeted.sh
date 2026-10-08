# WPV full own MT-136 acceptance after the approved durable SUPPRESS manifest repair.
# Eleven existing runtime cases plus the literal PT-136-1 warm library check.
ROUND_SELECTION_SHA=0c63e656c96c75fefbea00eb776c9cf1cc2d0f76
ROUND_SELECTION_MODE=targeted
ROUND_SELECTION_OUTPUT_SUFFIX=-mt136
CORE_SKIP=0
CORE_CHECK_LIB=1
CORE_LIB=1
CORE_TESTS=(memory_capsule_recorder_surreal_tests)
CORE_FILTER='(binary_id(=handshake_core) & test(/^(storage::surreal::mt136_database_surface_proof_a::mt136_database_surface_proof_a|storage::surreal::mt136_database_surface_proof_b::mt136_database_surface_proof_b|storage::surreal::mt136_database_surface_proof_c::tests::uncovered_database_methods_use_real_durable_surreal_state|storage::surreal::database::not_implemented_surface::not_implemented_surface_is_declared|storage::surreal::mt136_kernel_action_submitter_proof::tests::submitter_contract)$/)) | binary_id(=handshake_core::memory_capsule_recorder_surreal_tests)'
NATIVE_SKIP=1
NATIVE_TESTS=()
NATIVE_LIB=0
NATIVE_FILTER=''
EXTRACTED_CRATES=()
