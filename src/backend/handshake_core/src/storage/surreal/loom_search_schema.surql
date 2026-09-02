-- MT-016 dedicated embedding-model and LoomSearchV2 authority extension.
-- The shared schema owns workspaces, loom_blocks, loom_edges,
-- knowledge_retrieval_traces, and kernel_event_ledger. This file adds the
-- exact-scope fields and dedicated durable projections required by MT-016.

DEFINE FIELD OVERWRITE owner_account_id ON TABLE loom_blocks TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE loom_blocks TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE loom_blocks TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE loom_blocks TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE favorite ON TABLE loom_blocks TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE pin_order ON TABLE loom_blocks TYPE option<int>;
DEFINE INDEX OVERWRITE idx_loom_blocks_exact_scope ON TABLE loom_blocks FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id;

DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_block_search_index TYPE record<loom_blocks> ASSERT record::exists($value) AND $value = record::id($this.id) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_block_search_index TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE content_type ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE search_text ON TABLE loom_block_search_index TYPE string;
DEFINE FIELD OVERWRITE source_content_hash ON TABLE loom_block_search_index TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE embedding ON TABLE loom_block_search_index TYPE option<array<float>> ASSERT $value = NONE OR array::len($value) = 768;
DEFINE FIELD OVERWRITE embedding_model ON TABLE loom_block_search_index TYPE option<string> ASSERT ($value = NONE) = ($this.embedding = NONE);
DEFINE FIELD OVERWRITE embedding_dimension ON TABLE loom_block_search_index TYPE option<int> ASSERT ($value = NONE AND $this.embedding = NONE) OR ($value = 768 AND $this.embedding != NONE);
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE loom_block_search_index TYPE 'active' | 'stale' | 'revoked' DEFAULT 'active';
DEFINE FIELD OVERWRITE mutation_fingerprint ON TABLE loom_block_search_index TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE index_event_id ON TABLE loom_block_search_index TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE indexed_at ON TABLE loom_block_search_index TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_block_search_index ON TABLE loom_block_search_index FIELDS block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_block_search_exact_scope ON TABLE loom_block_search_index FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, content_type;
DEFINE INDEX OVERWRITE idx_loom_block_search_embedding_space ON TABLE loom_block_search_index FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, embedding_model;
DEFINE INDEX OVERWRITE idx_loom_block_search_exact_scope_state ON TABLE loom_block_search_index FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, lifecycle_state;
DEFINE INDEX OVERWRITE idx_loom_block_search_receipt ON TABLE loom_block_search_index FIELDS index_event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_block_search_embedding_hnsw ON TABLE loom_block_search_index FIELDS embedding HNSW DIMENSION 768 TYPE F32 DIST COSINE;

DEFINE FIELD OVERWRITE owner_account_id ON TABLE knowledge_retrieval_traces TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE knowledge_retrieval_traces TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE knowledge_retrieval_traces TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE knowledge_retrieval_traces TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE query_fingerprint ON TABLE knowledge_retrieval_traces TYPE option<string> ASSERT $value = NONE OR string::len($value) = 64;
DEFINE FIELD OVERWRITE result_set_fingerprint ON TABLE knowledge_retrieval_traces TYPE option<string> ASSERT $value = NONE OR string::len($value) = 64;
DEFINE INDEX OVERWRITE idx_knowledge_retrieval_traces_exact_scope ON TABLE knowledge_retrieval_traces FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, created_at;

DEFINE TABLE OVERWRITE loom_search_result_sets SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE result_set_id ON TABLE loom_search_result_sets TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE trace_id ON TABLE loom_search_result_sets TYPE record<knowledge_retrieval_traces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE loom_search_result_sets TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE loom_search_result_sets TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE loom_search_result_sets TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE loom_search_result_sets TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_search_result_sets TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE query_fingerprint ON TABLE loom_search_result_sets TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE result_set_fingerprint ON TABLE loom_search_result_sets TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE semantic_available ON TABLE loom_search_result_sets TYPE bool;
DEFINE FIELD OVERWRITE embedding_model ON TABLE loom_search_result_sets TYPE option<string>;
DEFINE FIELD OVERWRITE total ON TABLE loom_search_result_sets TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE receipt_event_id ON TABLE loom_search_result_sets TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_search_result_sets TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_search_result_sets ON TABLE loom_search_result_sets FIELDS result_set_id UNIQUE;
DEFINE INDEX OVERWRITE uq_loom_search_result_scope_fingerprint ON TABLE loom_search_result_sets FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, query_fingerprint, result_set_fingerprint UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_search_result_receipt ON TABLE loom_search_result_sets FIELDS receipt_event_id UNIQUE;

DEFINE TABLE OVERWRITE loom_search_results SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE result_id ON TABLE loom_search_results TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE result_set_id ON TABLE loom_search_results TYPE record<loom_search_result_sets> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_search_results TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE loom_search_results TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE loom_search_results TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE loom_search_results TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE loom_search_results TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_search_results TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE result_set_fingerprint ON TABLE loom_search_results TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE ordinal ON TABLE loom_search_results TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE score ON TABLE loom_search_results TYPE float;
DEFINE FIELD OVERWRITE fts_rank ON TABLE loom_search_results TYPE float;
DEFINE FIELD OVERWRITE trgm_sim ON TABLE loom_search_results TYPE float;
DEFINE FIELD OVERWRITE vector_sim ON TABLE loom_search_results TYPE float;
DEFINE FIELD OVERWRITE edge_degree ON TABLE loom_search_results TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE highlight ON TABLE loom_search_results TYPE string;
DEFINE FIELD OVERWRITE source_index_event_id ON TABLE loom_search_results TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_search_results TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_search_results ON TABLE loom_search_results FIELDS result_id UNIQUE;
DEFINE INDEX OVERWRITE uq_loom_search_results_ordinal ON TABLE loom_search_results FIELDS result_set_id, ordinal UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_search_results_scope_set ON TABLE loom_search_results FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, result_set_id;
