-- Down-migration for 0242_knowledge_memory_conflict_jobs.sql (MT-122/MT-123).
DELETE FROM knowledge_schema_registry WHERE family_key IN (
    'memory_conflict_detection_jobs',
    'memory_conflict_detection_findings',
    'memory_conflict_resolution_jobs'
);
DROP TABLE IF EXISTS knowledge_memory_conflict_resolution_jobs;
DROP TABLE IF EXISTS knowledge_memory_conflict_detection_findings;
DROP TABLE IF EXISTS knowledge_memory_conflict_detection_jobs;
