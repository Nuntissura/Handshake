# WPV round selection for candidate 1d1f583a60feccc975d6a8dca62564765b858122 (generated; named MT proof tests only, CX-EXEC-014).
# Reuse basis: native non-live proof tests that passed in validator unions 11f1b03404af6fe0d009e37abb04a0b9e6ee4fb2,9e4bee3426d23abab9c42decc7c11fd71d63d018,5da8b75a0211aa607ea430a6764770709915f80e,791a81dd23cee301006995a2b7bb550cfeba4471,7548bc72c8c819c139d6257674a84bd95a215b0d,d9ce6017e91430c69d410b555bd79fd5f6916926 (newest first) with unchanged inputs.
ROUND_SELECTION_SHA=1d1f583a60feccc975d6a8dca62564765b858122
CORE_TESTS=(loom_block_collection_views_tests)
CORE_FILTER='(binary_id(=handshake_core::loom_block_collection_views_tests) & (test(=table_sort_by_typed_column_is_correct_across_a_page_boundary) | test(=kanban_move_via_real_tag_edges_reflects_in_requery_and_embedded_store) | test(=free_kanban_places_shared_tag_cards_once_each) | test(=calendar_buckets_by_journal_date_with_date_filter) | test(=resort_persists_into_view_definition) | test(=saved_view_creation_is_idempotent_and_authority_backed)))'
NATIVE_TESTS=(test_block_collection_view)
NATIVE_LIB=0
NATIVE_FILTER='(binary_id(=handshake-native::test_block_collection_view) & (test(=block_collection_views_live_surrealdb_self_seed_full_round_trip) | test(=calendar_date_range_validates_then_emits) | test(=calendar_renders_two_day_buckets) | test(=client_card_move_top_level_tags) | test(=client_create_view_body) | test(=client_update_view_wraps_definition) | test(=empty_states_render_without_panic) | test(=kanban_card_drag_emits_card_move_then_requery_lands_card) | test(=kind_switch_emits_kind_change) | test(=new_view_creates_and_switches) | test(=table_renders_three_rows_with_titles) | test(=table_sort_click_emits_backend_sort_then_toggles)))'
EXTRACTED_CRATES=()
