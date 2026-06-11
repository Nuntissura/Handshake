//! WP-KERNEL-009 / MT-025 — Rust-side forbidden-dependency tripwires.
//!
//! Two layers over the PRODUCT cargo manifests declared in the runtime
//! dependency allowlist:
//!
//!  1. Direct-declaration scan (`forbidden_direct_cargo_dependencies`): no
//!     forbidden crate (sqlite, redis, testcontainers, ...) may be DECLARED
//!     as a direct dependency, including `alias = { package = "real" }`
//!     renames.
//!  2. Feature-aware ACTIVATION scan (`forbidden_resolved_cargo_packages`):
//!     `cargo metadata`-based closure over activated edges proving no
//!     forbidden crate is COMPILED, direct or transitive. This catches the
//!     feature-smuggling cases (`sqlx = { features = ["sqlite"] }`, or a
//!     renamed feature whose targets include `dep:rusqlite`) that the
//!     direct-name scan cannot see.
//!
//! Honest scope note (mirrors the node-side test
//! `app/src/lib/dependency_policy/no_sqlite_regression.test.ts`): Cargo.lock
//! and cargo metadata's resolve graph both hold feature-UNION entries — sqlx
//! ships an optional `sqlx-sqlite`, so `libsqlite3-sys` appears there while
//! remaining INERT. Union presence is therefore reported as an advisory, not
//! a violation; activation analysis (here) and the independent
//! `cargo tree --all-features -i <crate>` proof (node-side MT-025 test and
//! the MT-032 validator hook) are the authoritative checks.

use std::path::Path;

use super::{repo_root_from_manifest_dir, RuntimeDependencyAllowlist};

/// Extracts direct dependency crate names from `[dependencies]`-style
/// sections of a Cargo.toml (handles `alias = { package = "real" }`).
fn cargo_manifest_dependency_names(cargo_toml: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_dep_section = false;
    for raw_line in cargo_toml.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            let section = line.trim_matches(|c| c == '[' || c == ']');
            let last = section.rsplit('.').next().unwrap_or(section);
            in_dep_section = matches!(
                last,
                "dependencies" | "dev-dependencies" | "build-dependencies"
            ) || section.ends_with("dependencies");
            continue;
        }
        if !in_dep_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let mut name = line[..eq].trim().trim_matches('"').to_string();
        if let Some(pkg_pos) = line.find("package") {
            let rest = &line[pkg_pos..];
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].find('"') {
                    name = rest[start + 1..start + 1 + end].to_string();
                }
            }
        }
        if name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
            && !name.is_empty()
        {
            names.push(name);
        }
    }
    names
}

/// Feature-aware ACTIVATION scan over `cargo metadata`.
///
/// IMPORTANT semantics (probed against the real workspace, 2026-06): cargo
/// metadata's `resolve` graph is the feature-UNION — optional dependencies
/// that no feature ever enables still appear as nodes and edges (sqlx's
/// optional `sqlx-sqlite` → `libsqlite3-sys` chain shows up although the
/// product never enables sqlx's `sqlite` feature). Node presence is therefore
/// NOT activation evidence. What IS authoritative in the metadata:
///   - `resolve.nodes[].features`  — the finally-enabled feature set per node;
///   - `packages[].features`       — each package's feature map
///                                    (`feat -> [dep:name, name/feat, ...]`);
///   - `packages[].dependencies[].optional` — manifest-level edge optionality.
///
/// A dependency edge is ACTIVATED iff it is non-optional, or some ENABLED
/// feature of the dependent activates it (`dep:name`, non-`?` `name/feat`, or
/// the implicit feature named after the optional dep). The activated set is
/// the BFS closure over activated edges from the workspace root. This catches
/// the MT-025 smuggling cases — `sqlx = { features = ["sqlite"] }` and a
/// renamed feature whose targets include `dep:rusqlite` — while staying quiet
/// on dormant union entries (those remain advisories proven inert by the
/// `cargo tree -i` proof in the node-side MT-025 test and the MT-032 hook).
///
/// Returns (manifest, crate, class) triples for every forbidden crate in the
/// ACTIVATED graph. Errors carry the cargo invocation failure text.
pub fn forbidden_resolved_cargo_packages(
    repo_root: &Path,
    allowlist: &RuntimeDependencyAllowlist,
) -> Result<Vec<(String, String, String)>, String> {
    let mut violations = Vec::new();
    for manifest_rel in &allowlist.product_manifests.cargo {
        let manifest_path =
            repo_root.join(manifest_rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        let output = std::process::Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(&manifest_path)
            .output()
            .map_err(|e| format!("spawn cargo metadata for {manifest_rel}: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed for {manifest_rel}: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let doc: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("parse cargo metadata JSON for {manifest_rel}: {e}"))?;
        for (name, class_id) in
            forbidden_activated_packages(&doc, allowlist).map_err(|e| format!("{manifest_rel}: {e}"))?
        {
            violations.push((manifest_rel.clone(), name, class_id));
        }
    }
    Ok(violations)
}

/// Pure activation analysis over a parsed `cargo metadata` document (testable
/// against synthetic fixtures). Returns (crate, class) pairs for forbidden
/// crates inside the feature-activated closure.
fn forbidden_activated_packages(
    doc: &serde_json::Value,
    allowlist: &RuntimeDependencyAllowlist,
) -> Result<Vec<(String, String)>, String> {
    use std::collections::{HashMap, HashSet, VecDeque};

    let packages = doc["packages"]
        .as_array()
        .ok_or("cargo metadata has no packages array")?;
    let mut pkg_by_id: HashMap<&str, &serde_json::Value> = HashMap::new();
    for package in packages {
        if let Some(id) = package["id"].as_str() {
            pkg_by_id.insert(id, package);
        }
    }
    let nodes = doc["resolve"]["nodes"]
        .as_array()
        .ok_or("cargo metadata has no resolve graph")?;
    let mut node_by_id: HashMap<&str, &serde_json::Value> = HashMap::new();
    for node in nodes {
        if let Some(id) = node["id"].as_str() {
            node_by_id.insert(id, node);
        }
    }
    let root = doc["resolve"]["root"]
        .as_str()
        .ok_or("cargo metadata resolve has no root")?;

    // Optional-dependency names a node's ENABLED features activate.
    let activated_optionals = |id: &str| -> HashSet<String> {
        let mut out = HashSet::new();
        let (Some(node), Some(pkg)) = (node_by_id.get(id), pkg_by_id.get(id)) else {
            return out;
        };
        let feature_map = pkg["features"].as_object();
        for feature in node["features"].as_array().into_iter().flatten() {
            let Some(feature) = feature.as_str() else { continue };
            // Implicit optional-dep feature: a feature named after the dep.
            out.insert(feature.to_string());
            let Some(targets) = feature_map.and_then(|m| m.get(feature)).and_then(|t| t.as_array())
            else {
                continue;
            };
            for target in targets {
                let Some(target) = target.as_str() else { continue };
                if let Some(dep) = target.strip_prefix("dep:") {
                    out.insert(dep.to_string());
                } else if let Some((dep, _feat)) = target.split_once('/') {
                    // `name/feat` activates the optional dep; `name?/feat` does not.
                    if !dep.ends_with('?') {
                        out.insert(dep.to_string());
                    }
                }
            }
        }
        out
    };

    // BFS over ACTIVATED edges from the root.
    let mut activated: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    activated.insert(root.to_string());
    queue.push_back(root.to_string());
    while let Some(id) = queue.pop_front() {
        let Some(node) = node_by_id.get(id.as_str()) else { continue };
        let is_root = id == root;
        let active_optionals = activated_optionals(&id);
        let parent_pkg = pkg_by_id.get(id.as_str());
        for edge in node["deps"].as_array().into_iter().flatten() {
            let Some(target_id) = edge["pkg"].as_str() else { continue };
            let Some(target_pkg) = pkg_by_id.get(target_id) else { continue };
            let target_name = target_pkg["name"].as_str().unwrap_or_default();
            // Edge kinds: normal (null) + build always; dev only for the root
            // (mirrors `cargo tree -e normal,build` + root dev-deps compiled
            // for tests).
            let kind_ok = edge["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|k| match k["kind"].as_str() {
                    None | Some("build") => true,
                    Some("dev") => is_root,
                    Some(_) => false,
                });
            if !kind_ok {
                continue;
            }
            // Manifest-level optionality of this edge (match by package name,
            // honoring renames via the entry's `package`/`rename` fields).
            let optional = parent_pkg
                .and_then(|p| p["dependencies"].as_array())
                .into_iter()
                .flatten()
                .filter(|d| {
                    d["name"].as_str() == Some(target_name)
                        || d["rename"].as_str().is_some_and(|r| {
                            active_optionals.contains(r) || r == target_name
                        })
                })
                .any(|d| d["optional"].as_bool() == Some(true));
            let edge_active = !optional
                || active_optionals.contains(target_name)
                || edge["name"]
                    .as_str()
                    .is_some_and(|n| active_optionals.contains(n));
            if edge_active && activated.insert(target_id.to_string()) {
                queue.push_back(target_id.to_string());
            }
        }
    }

    let mut violations = Vec::new();
    for id in &activated {
        let Some(pkg) = pkg_by_id.get(id.as_str()) else { continue };
        let Some(name) = pkg["name"].as_str() else { continue };
        let lowered = name.to_ascii_lowercase();
        for class in &allowlist.forbidden_runtime_dependency_classes {
            if class
                .cargo_crate_name_substrings
                .iter()
                .any(|s| lowered.contains(s))
            {
                violations.push((name.to_string(), class.id.clone()));
            }
        }
    }
    violations.sort();
    Ok(violations)
}

/// Returns (manifest, crate, class) triples for every direct product
/// dependency matching a forbidden cargo substring.
pub fn forbidden_direct_cargo_dependencies(
    repo_root: &Path,
    allowlist: &RuntimeDependencyAllowlist,
) -> Vec<(String, String, String)> {
    let mut violations = Vec::new();
    for manifest_rel in &allowlist.product_manifests.cargo {
        let manifest_path = repo_root.join(manifest_rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        let Ok(raw) = std::fs::read_to_string(&manifest_path) else {
            violations.push((
                manifest_rel.clone(),
                "<unreadable manifest>".to_string(),
                "io".to_string(),
            ));
            continue;
        };
        let names = cargo_manifest_dependency_names(&raw);
        for class in &allowlist.forbidden_runtime_dependency_classes {
            for name in &names {
                let lowered = name.to_ascii_lowercase();
                if class
                    .cargo_crate_name_substrings
                    .iter()
                    .any(|s| lowered.contains(s))
                {
                    violations.push((manifest_rel.clone(), name.clone(), class.id.clone()));
                }
            }
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_manifests_declare_no_forbidden_crates() {
        let repo_root = repo_root_from_manifest_dir();
        let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(&repo_root)
            .expect("allowlist loads");
        let violations = forbidden_direct_cargo_dependencies(&repo_root, &allowlist);
        assert!(
            violations.is_empty(),
            "forbidden direct cargo dependencies found: {violations:?}"
        );
    }

    /// MT-025 authoritative Rust-side proof: the feature-activated closure of
    /// BOTH product manifests contains no sqlite/redis/testcontainers crate,
    /// direct or transitive. The dormant sqlx-sqlite union entries stay
    /// dormant because no enabled feature activates them. (duckdb is not
    /// sqlite and is intentionally not a forbidden substring.)
    #[test]
    fn resolved_dependency_graph_contains_no_forbidden_crates() {
        let repo_root = repo_root_from_manifest_dir();
        let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(&repo_root)
            .expect("allowlist loads");
        assert_eq!(
            allowlist.product_manifests.cargo.len(),
            2,
            "expected both product manifests (handshake_core + src-tauri)"
        );
        let violations = forbidden_resolved_cargo_packages(&repo_root, &allowlist)
            .expect("cargo metadata resolves both product manifests");
        assert!(
            violations.is_empty(),
            "forbidden crates ACTIVATED in the product graph: {violations:?}"
        );
    }

    /// Synthetic cargo-metadata fixture mirroring the real sqlx layout:
    /// root → sqlx(non-optional); sqlx has OPTIONAL sqlx-sqlite behind its
    /// `sqlite` feature; sqlx-sqlite → libsqlite3-sys (non-optional).
    fn sqlx_fixture(enabled_sqlx_features: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "packages": [
                {
                    "id": "root-id", "name": "product",
                    "features": {},
                    "dependencies": [
                        { "name": "sqlx", "optional": false, "kind": null }
                    ]
                },
                {
                    "id": "sqlx-id", "name": "sqlx",
                    "features": {
                        "postgres": ["sqlx-core/postgres"],
                        "sqlite": ["dep:sqlx-sqlite"]
                    },
                    "dependencies": [
                        { "name": "sqlx-sqlite", "optional": true, "kind": null }
                    ]
                },
                {
                    "id": "sqlx-sqlite-id", "name": "sqlx-sqlite",
                    "features": {},
                    "dependencies": [
                        { "name": "libsqlite3-sys", "optional": false, "kind": null }
                    ]
                },
                {
                    "id": "libsqlite3-sys-id", "name": "libsqlite3-sys",
                    "features": {},
                    "dependencies": []
                }
            ],
            "resolve": {
                "root": "root-id",
                "nodes": [
                    {
                        "id": "root-id",
                        "features": [],
                        "deps": [ { "name": "sqlx", "pkg": "sqlx-id",
                                    "dep_kinds": [ { "kind": null } ] } ]
                    },
                    {
                        "id": "sqlx-id",
                        "features": enabled_sqlx_features,
                        "deps": [ { "name": "sqlx_sqlite", "pkg": "sqlx-sqlite-id",
                                    "dep_kinds": [ { "kind": null } ] } ]
                    },
                    {
                        "id": "sqlx-sqlite-id",
                        "features": [],
                        "deps": [ { "name": "libsqlite3_sys", "pkg": "libsqlite3-sys-id",
                                    "dep_kinds": [ { "kind": null } ] } ]
                    },
                    { "id": "libsqlite3-sys-id", "features": [], "deps": [] }
                ]
            }
        })
    }

    #[test]
    fn dormant_optional_sqlite_union_is_not_flagged() {
        let repo_root = repo_root_from_manifest_dir();
        let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(&repo_root)
            .expect("allowlist loads");
        // Union nodes exist, but sqlx's `sqlite` feature is NOT enabled.
        let doc = sqlx_fixture(&["postgres"]);
        let violations = forbidden_activated_packages(&doc, &allowlist).expect("analysis runs");
        assert!(
            violations.is_empty(),
            "dormant union entries must not be flagged: {violations:?}"
        );
    }

    #[test]
    fn feature_smuggled_sqlite_is_flagged() {
        let repo_root = repo_root_from_manifest_dir();
        let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(&repo_root)
            .expect("allowlist loads");
        // `sqlx = { features = ["sqlite"] }` smuggling: the enabled feature
        // activates dep:sqlx-sqlite, pulling libsqlite3-sys transitively.
        let doc = sqlx_fixture(&["postgres", "sqlite"]);
        let violations = forbidden_activated_packages(&doc, &allowlist).expect("analysis runs");
        let names: Vec<&str> = violations.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"sqlx-sqlite"), "sqlx-sqlite must be flagged: {names:?}");
        assert!(
            names.contains(&"libsqlite3-sys"),
            "transitive libsqlite3-sys must be flagged: {names:?}"
        );
    }

    #[test]
    fn renamed_feature_activating_sqlite_dep_is_flagged() {
        let repo_root = repo_root_from_manifest_dir();
        let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(&repo_root)
            .expect("allowlist loads");
        // A feature whose NAME hides its payload: `embedded-db = ["dep:rusqlite"]`.
        let doc = serde_json::json!({
            "packages": [
                {
                    "id": "root-id", "name": "product",
                    "features": { "embedded-db": ["dep:rusqlite"] },
                    "dependencies": [ { "name": "rusqlite", "optional": true, "kind": null } ]
                },
                { "id": "rusqlite-id", "name": "rusqlite", "features": {}, "dependencies": [] }
            ],
            "resolve": {
                "root": "root-id",
                "nodes": [
                    {
                        "id": "root-id",
                        "features": ["embedded-db"],
                        "deps": [ { "name": "rusqlite", "pkg": "rusqlite-id",
                                    "dep_kinds": [ { "kind": null } ] } ]
                    },
                    { "id": "rusqlite-id", "features": [], "deps": [] }
                ]
            }
        });
        let violations = forbidden_activated_packages(&doc, &allowlist).expect("analysis runs");
        assert!(
            violations.iter().any(|(n, c)| n == "rusqlite" && c == "sqlite"),
            "renamed-feature activation must be flagged: {violations:?}"
        );
    }

    #[test]
    fn tripwire_catches_direct_sqlite_declaration() {
        let fixture = r#"
[package]
name = "evil"

[dependencies]
rusqlite = "0.32"
serde = "1"

[dev-dependencies]
sqlx = { version = "0.8", features = ["sqlite"] }
"#;
        let names = cargo_manifest_dependency_names(fixture);
        assert!(names.contains(&"rusqlite".to_string()));
        assert!(names.contains(&"sqlx".to_string()));
        let repo_root = repo_root_from_manifest_dir();
        let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(&repo_root)
            .expect("allowlist loads");
        let sqlite = allowlist.forbidden_class("sqlite").expect("sqlite class");
        assert!(
            names.iter().any(|n| sqlite
                .cargo_crate_name_substrings
                .iter()
                .any(|s| n.to_ascii_lowercase().contains(s))),
            "fixture must trip the sqlite class"
        );
    }

    #[test]
    fn tripwire_catches_renamed_package_declaration() {
        let fixture = r#"
[dependencies]
harmless_alias = { package = "libsqlite3-sys", version = "0.30" }
"#;
        let names = cargo_manifest_dependency_names(fixture);
        assert!(
            names.contains(&"libsqlite3-sys".to_string()),
            "package rename must not hide the real crate: {names:?}"
        );
    }

    #[test]
    fn product_manifests_have_dependencies_at_all() {
        // Guards against the scanner silently parsing nothing.
        let repo_root = repo_root_from_manifest_dir();
        let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(&repo_root)
            .expect("allowlist loads");
        for manifest_rel in &allowlist.product_manifests.cargo {
            let manifest_path =
                repo_root.join(manifest_rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            let raw = std::fs::read_to_string(&manifest_path)
                .unwrap_or_else(|e| panic!("read {manifest_rel}: {e}"));
            let names = cargo_manifest_dependency_names(&raw);
            assert!(
                names.len() >= 3,
                "{manifest_rel}: expected >=3 direct dependencies, got {names:?}"
            );
        }
    }
}
