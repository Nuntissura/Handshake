DROP INDEX IF EXISTS uq_loom_folders_child_name;
DROP INDEX IF EXISTS uq_loom_folders_root_name;

ALTER TABLE loom_folders
    ADD CONSTRAINT uq_loom_folders_sibling_name
    UNIQUE (workspace_id, parent_folder_id, name);
