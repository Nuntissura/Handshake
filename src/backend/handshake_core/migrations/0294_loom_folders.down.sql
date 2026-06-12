-- WP-KERNEL-009 MT-181 FolderTreeAndColorLabels (down).
DROP INDEX IF EXISTS idx_loom_folder_members_folder_order;
DROP INDEX IF EXISTS idx_loom_folder_members_block;
DROP TABLE IF EXISTS loom_folder_members;
DROP INDEX IF EXISTS idx_loom_folders_workspace_parent;
DROP TABLE IF EXISTS loom_folders;
