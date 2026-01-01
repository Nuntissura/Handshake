//! OSS Register Enforcement Tests
//!
//! Per SPEC v02.100 §11.10.4.2:
//! - Every crate in Cargo.lock MUST exist in docs/OSS_REGISTER.md
//! - Every npm package in package.json (deps + devDeps) MUST exist in docs/OSS_REGISTER.md
//! - GPL/AGPL entries MUST have IntegrationMode == "external_process"
//!
//! Error codes:
//! - HSK-OSS-000: Path/file resolution error
//! - HSK-OSS-001: Missing required table header
//! - HSK-OSS-002: Invalid IntegrationMode value
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
        repo_root().join("docs/OSS_REGISTER.md")
    }

    fn cargo_lock_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")
    }

    fn package_json_path() -> PathBuf {
        repo_root().join("app/package.json")
    }

    //=== CONSTANTS ===

    /// Exact header pattern (column order matters)
    const HEADER_PATTERN: &str = "| Component | License | IntegrationMode | Scope | Purpose |";

    const VALID_MODES: [&str; 3] = ["embedded_lib", "external_process", "external_service"];

    //=== TYPES ===

    #[derive(Debug, Clone)]
    struct RegisterEntry {
        component: String,
        license: String,
        integration_mode: String,
    }

    //=== PARSERS ===

    /// Parse OSS_REGISTER.md with strict format validation.
    ///
    /// Parsing rules:
    /// 1. Header detection: Line must EXACTLY match HEADER_PATTERN (after trim)
    /// 2. Separator rows: Lines matching `| --- | --- | ...` are skipped
    /// 3. Data rows: Lines starting and ending with `|` MUST have exactly 5 columns
    /// 4. IntegrationMode: Must be one of VALID_MODES
    ///
    /// Errors:
    /// - HSK-OSS-001: No header found
    /// - HSK-OSS-002: Invalid IntegrationMode
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
                // Split and filter empty parts from leading/trailing pipes
                let cols: Vec<&str> = trimmed
                    .split('|')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                // Fail fast: row must have exactly 5 columns
                if cols.len() != 5 {
                    return Err(format!(
                        "HSK-OSS-006: Row format error at line {}: expected 5 columns, found {}. \
                         Row: '{}'",
                        line_num + 1,
                        cols.len(),
                        trimmed
                    ));
                }

                let component = cols[0].to_string();
                let license = cols[1].to_string();
                let integration_mode = cols[2].to_string();

                // Validate IntegrationMode
                if !VALID_MODES.contains(&integration_mode.as_str()) {
                    return Err(format!(
                        "HSK-OSS-002: Invalid IntegrationMode '{}' for component '{}' at line {}. \
                         Must be one of: embedded_lib, external_process, external_service",
                        integration_mode,
                        component,
                        line_num + 1
                    ));
                }

                entries.push(RegisterEntry {
                    component,
                    license,
                    integration_mode,
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
        let content =
            fs::read_to_string(&path).expect(&format!("HSK-OSS-000: Cannot read {:?}", path));

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
        let content =
            fs::read_to_string(&path).expect(&format!("HSK-OSS-000: Cannot read {:?}", path));

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
        let registered: HashSet<String> = register
            .iter()
            .map(|e| e.component.to_lowercase())
            .collect();
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
        let registered: HashSet<String> = register
            .iter()
            .map(|e| e.component.to_lowercase())
            .collect();
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
        // Per §11.10.4.2: GPL/AGPL MUST be external_process (not embedded_lib, not external_service)
        let register = parse_oss_register().expect("Register must be valid for copyleft check");

        let violations: Vec<String> = register
            .iter()
            .filter(|e| {
                let license_upper = e.license.to_uppercase();
                license_upper.starts_with("GPL") || license_upper.starts_with("AGPL")
            })
            .filter(|e| e.integration_mode != "external_process")
            .map(|e| {
                format!(
                    "{} (license: {}) has IntegrationMode '{}' - must be 'external_process' per §11.10.4.2",
                    e.component, e.license, e.integration_mode
                )
            })
            .collect();

        assert!(
            violations.is_empty(),
            "HSK-OSS-005: Copyleft isolation violations: {:?}",
            violations
        );
    }

    #[test]
    fn test_no_gpl_agpl_present() {
        // Informational: verify current state has no GPL/AGPL components
        let register = parse_oss_register().expect("Register must be valid");

        let copyleft: Vec<&RegisterEntry> = register
            .iter()
            .filter(|e| {
                let license_upper = e.license.to_uppercase();
                license_upper.starts_with("GPL") || license_upper.starts_with("AGPL")
            })
            .collect();

        // This test documents current state - if we add GPL/AGPL in future,
        // test_copyleft_isolation will enforce the external_process rule
        assert!(
            copyleft.is_empty(),
            "Found GPL/AGPL components (must be external_process): {:?}",
            copyleft
        );
    }
}
