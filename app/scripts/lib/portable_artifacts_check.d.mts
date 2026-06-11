// Type declarations for portable_artifacts_check.mjs (WP-KERNEL-009 / MT-026).

export interface PortableArtifactViolation {
  check: string;
  problem: string;
}

export interface PortableArtifactFacts {
  cargo_target_dir?: string | null;
  vite_configs?: Record<string, string>;
  harness_out_dir?: string | null;
  tauri_frontend_dist?: string | null;
}

export declare function checkPortableArtifactBoundaries(args: {
  repoRoot: string;
}): { violations: PortableArtifactViolation[]; facts: PortableArtifactFacts };
