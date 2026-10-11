-- Exact-scope extension for the existing ProjectKnowledgeIndex entity table.
-- The shared schema owns the base table; this provider owns only the fields
-- required by the UserManual bundle bridge's scoped writes and receipts.
DEFINE FIELD OVERWRITE owner_account_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_entities TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE mutation_fingerprint ON TABLE knowledge_entities TYPE string ASSERT string::len($value) = 64;

DEFINE INDEX OVERWRITE uq_knowledge_entities_identity ON TABLE knowledge_entities FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, entity_kind, entity_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_entities_exact_scope_kind ON TABLE knowledge_entities FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, entity_kind;
DEFINE INDEX OVERWRITE idx_knowledge_entities_recorded_event ON TABLE knowledge_entities FIELDS recorded_event_id UNIQUE;
