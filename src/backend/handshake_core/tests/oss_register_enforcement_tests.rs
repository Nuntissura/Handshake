//! OSS Register Enforcement Tests
//!
//! Per SPEC v02.113 11.10.4 (2) + 11.7.5.7.1:
//! - Every crate in `src/backend/handshake_core/Cargo.lock` MUST exist in
//!   `.GOV/roles_shared/OSS_REGISTER.md`
//! - Every npm package in `app/package.json` (dependencies + devDependencies) MUST exist in
//!   `.GOV/roles_shared/OSS_REGISTER.md`
//! - GPL/AGPL entries MUST have `integration_mode_default == "external_process"`
//!
//! Error codes:
//! - HSK-OSS-000: Path/file resolution error
//! - HSK-OSS-001: Missing required table header
//! - HSK-OSS-002: Invalid integration_mode_default value
//! - HSK-OSS-003: Cargo.lock package missing from register
//! - HSK-OSS-004: package.json dependency missing from register
//! - HSK-OSS-005: Copyleft isolation violation (GPL/AGPL not external_process)
//! - HSK-OSS-006: Row format error (wrong column count)

#[cfg(test)]
mod oss_register_enforcement {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    //=== PATH RESOLUTION (CARGO_MANIFEST_DIR-based) ===

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("HSK-OSS-000: Failed to resolve repo root from CARGO_MANIFEST_DIR")
    }

    fn oss_register_path() -> PathBuf {
        repo_root().join(".GOV/roles_shared/OSS_REGISTER.md")
    }

    fn cargo_lock_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")
    }

    fn package_json_path() -> PathBuf {
        repo_root().join("app/package.json")
    }

    //=== CONSTANTS ===

    /// Exact header pattern (column order matters)
    const HEADER_PATTERN: &str = "| component_id | name | upstream_ref | license | integration_mode_default | capabilities_required | pinning_policy | compliance_notes | test_fixture | used_by_modules |";

    const VALID_MODES: [&str; 3] = ["embedded_lib", "external_process", "external_service"];

    //=== TYPES ===

    #[derive(Debug, Clone)]
    struct RegisterEntry {
        component_id: String,
        name: String,
        license: String,
        integration_mode_default: String,
    }

    //=== PARSERS ===

    fn is_copyleft_license(license: &str) -> bool {
        let license_upper = license.to_uppercase();
        license_upper.contains("AGPL")
            || (license_upper.contains("GPL") && !license_upper.contains("LGPL"))
    }

    /// Parse OSS_REGISTER.md with strict format validation.
    ///
    /// Parsing rules:
    /// 1. Header detection: Line must EXACTLY match HEADER_PATTERN (after trim)
    /// 2. Separator rows: Lines matching `| --- | --- | ...` are skipped
    /// 3. Data rows: Lines starting and ending with `|` MUST have exactly 10 columns (including empty cells)
    /// 4. integration_mode_default: Must be one of VALID_MODES
    ///
    /// Errors:
    /// - HSK-OSS-001: No header found
    /// - HSK-OSS-002: Invalid integration_mode_default
    /// - HSK-OSS-006: Row has wrong column count
    fn parse_oss_register() -> Result<Vec<RegisterEntry>, String> {
        let path = oss_register_path();
        let content = fs::read_to_string(&path).map_err(|e| {
            format!(
                "HSK-OSS-000: Cannot read OSS_REGISTER.md at {}: {}",
                path.display(),
                e
            )
        })?;

        let mut entries = Vec::new();
        let mut in_table = false;
        let mut header_found = false;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Detect header (exact match required)
            if trimmed == HEADER_PATTERN {
                in_table = true;
                header_found = true;
                continue;
            }

            // Skip separator rows (e.g., "| --- | --- | --- | --- | --- |")
            if trimmed.starts_with('|') && trimmed.contains("---") {
                continue;
            }

            // End table on blank line or new section header
            if in_table && (trimmed.is_empty() || trimmed.starts_with('#')) {
                in_table = false;
                continue;
            }

            // Parse data rows
            if in_table && trimmed.starts_with('|') && trimmed.ends_with('|') {
                let cols: Vec<&str> = trimmed
                    .trim_matches('|')
                    .split('|')
                    .map(|s| s.trim())
                    .collect();

                // Fail fast: row must have exactly 10 columns
                if cols.len() != 10 {
                    return Err(format!(
                        "HSK-OSS-006: Row format error at line {}: expected 10 columns, found {}. \
                         Row: '{}'",
                        line_num + 1,
                        cols.len(),
                        trimmed
                    ));
                }

                let component_id = cols[0].to_string();
                let name = cols[1].to_string();
                let license = cols[3].to_string();
                let integration_mode_default = cols[4].to_string();

                // Validate integration_mode_default
                if !VALID_MODES.contains(&integration_mode_default.as_str()) {
                    return Err(format!(
                        "HSK-OSS-002: Invalid integration_mode_default '{}' for name '{}' at line {}. \
                         Must be one of: embedded_lib, external_process, external_service",
                        integration_mode_default,
                        name,
                        line_num + 1
                    ));
                }

                entries.push(RegisterEntry {
                    component_id,
                    name,
                    license,
                    integration_mode_default,
                });
            }
        }

        if !header_found {
            return Err(format!(
                "HSK-OSS-001: OSS_REGISTER.md missing required table header. \
                 Expected exact match: '{}'",
                HEADER_PATTERN
            ));
        }

        if entries.is_empty() {
            return Err(
                "HSK-OSS-001: OSS_REGISTER.md has header but no valid data rows".to_string(),
            );
        }

        Ok(entries)
    }

    /// Parse Cargo.lock [[package]] blocks, extract "name" field (ALL packages).
    fn parse_cargo_lock_packages() -> HashSet<String> {
        let path = cargo_lock_path();
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("HSK-OSS-000: Cannot read {:?}: {}", path, e));

        content
            .lines()
            .filter(|line| line.starts_with("name = "))
            .map(|line| {
                line.trim_start_matches("name = ")
                    .trim_matches('"')
                    .to_string()
            })
            .collect()
    }

    /// Parse package.json dependencies + devDependencies keys.
    fn parse_package_json_deps() -> HashSet<String> {
        let path = package_json_path();
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("HSK-OSS-000: Cannot read {:?}: {}", path, e));

        let json: serde_json::Value =
            serde_json::from_str(&content).expect("HSK-OSS-000: Invalid JSON in package.json");

        let mut deps = HashSet::new();

        for key in ["dependencies", "devDependencies"] {
            if let Some(obj) = json.get(key).and_then(|v| v.as_object()) {
                deps.extend(obj.keys().cloned());
            }
        }

        deps
    }

    //=== TESTS ===

    #[test]
    fn test_register_format_valid() {
        let entries = parse_oss_register().expect("Register format must be valid");
        assert!(
            !entries.is_empty(),
            "HSK-OSS-001: Register should not be empty"
        );

        // Verify we have a reasonable number of entries (sanity check)
        assert!(
            entries.len() >= 100,
            "Expected at least 100 entries (Cargo.lock has ~400 packages), found {}",
            entries.len()
        );
    }

    #[test]
    fn test_cargo_lock_coverage() {
        let register = parse_oss_register().expect("Register must be valid for coverage check");
        let registered: HashSet<String> = register.iter().map(|e| e.name.to_lowercase()).collect();
        let cargo_deps = parse_cargo_lock_packages();

        let missing: Vec<&String> = cargo_deps
            .iter()
            .filter(|dep| !registered.contains(&dep.to_lowercase()))
            .collect();

        assert!(
            missing.is_empty(),
            "HSK-OSS-003: Cargo.lock packages not in OSS_REGISTER.md: {:?}",
            missing
        );
    }

    #[test]
    fn test_package_json_coverage() {
        let register = parse_oss_register().expect("Register must be valid for coverage check");
        let registered: HashSet<String> = register.iter().map(|e| e.name.to_lowercase()).collect();
        let npm_deps = parse_package_json_deps();

        let missing: Vec<&String> = npm_deps
            .iter()
            .filter(|dep| !registered.contains(&dep.to_lowercase()))
            .collect();

        assert!(
            missing.is_empty(),
            "HSK-OSS-004: package.json deps not in OSS_REGISTER.md: {:?}",
            missing
        );
    }

    #[test]
    fn test_copyleft_isolation() {
        // Per 11.10.4 (2): GPL/AGPL MUST be external_process (not embedded_lib, not external_service)
        let register = parse_oss_register().expect("Register must be valid for copyleft check");

        let violations: Vec<String> = register
            .iter()
            .filter(|e| is_copyleft_license(&e.license))
            .filter(|e| e.integration_mode_default != "external_process")
            .map(|e| {
                format!(
                    "{} (name: {}, license: {}) has integration_mode_default '{}' - must be 'external_process' per 11.10.4 (2)",
                    e.component_id, e.name, e.license, e.integration_mode_default
                )
            })
            .collect();

        assert!(
            violations.is_empty(),
            "HSK-OSS-005: Copyleft isolation violations: {:?}",
            violations
        );
    }
}
