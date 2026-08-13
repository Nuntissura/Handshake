DELETE FROM model_lane_resource_scope_registry
WHERE table_name = 'worktree_vm_bindings';

DROP TABLE IF EXISTS worktree_vm_bindings;
