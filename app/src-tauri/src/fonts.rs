use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use walkdir::WalkDir;

const DESIGN_PACK_ID: &str = "design-pack-40";
const DESIGN_PACK_VERSION: &str = "design-pack-40@1";
const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub generated_at: String,
    pub pack_version: String,
    pub fonts: Vec<FontRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontRecord {
    pub id: String,
    pub family: String,
    pub style: String,
    pub weight: FontWeight,
    pub source: String,
    pub format: String,
    pub path: String,
    pub license: FontLicense,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FontWeight {
    Variable { min: u16, max: u16 },
    Fixed { value: u16 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontLicense {
    pub spdx: String,
    pub license_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
    pub manifest: Manifest,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackVersionFile {
    schema_version: u32,
    pack_id: String,
    pack_version: String,
}

fn fonts_root(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir failed: {e}"))?;
    Ok(app_data.join("fonts"))
}

fn bundled_dir(root: &Path) -> PathBuf {
    root.join("bundled")
}

fn user_dir(root: &Path) -> PathBuf {
    root.join("user")
}

fn cache_dir(root: &Path) -> PathBuf {
    root.join("cache")
}

fn manifest_path(root: &Path) -> PathBuf {
    cache_dir(root).join("manifest.json")
}

fn pack_version_path(root: &Path) -> PathBuf {
    root.join("fonts_pack_version.json")
}

fn ensure_runtime_dirs(root: &Path) -> Result<(), String> {
    fs::create_dir_all(bundled_dir(root)).map_err(|e| format!("create bundled dir failed: {e}"))?;
    fs::create_dir_all(user_dir(root)).map_err(|e| format!("create user dir failed: {e}"))?;
    fs::create_dir_all(cache_dir(root)).map_err(|e| format!("create cache dir failed: {e}"))?;
    Ok(())
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("no parent directory for {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("create_dir_all failed: {e}"))?;

    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");

    {
        let mut file = fs::File::create(&tmp).map_err(|e| format!("create tmp failed: {e}"))?;
        file.write_all(bytes)
            .map_err(|e| format!("write tmp failed: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("sync tmp failed: {e}"))?;
    }

    fs::rename(&tmp, path).map_err(|e| format!("rename tmp failed: {e}"))?;
    Ok(())
}

fn is_allowed_font_ext(ext: &str) -> bool {
    matches!(ext, "ttf" | "otf" | "woff2" | "woff")
}

fn detect_format(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn sanitize_family(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let ok = ch.is_ascii_alphanumeric() || ch == ' ' || ch == '_' || ch == '-';
        if ok {
            out.push(ch);
        }
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex::encode(digest)
}

fn try_parse_font_meta(
    bytes: &[u8],
    format: &str,
    fallback_stem: &str,
) -> (String, String, FontWeight) {
    if matches!(format, "ttf" | "otf") {
        if let Ok(face) = ttf_parser::Face::parse(bytes, 0) {
            let family = best_effort_family(&face).unwrap_or_else(|| fallback_stem.to_string());
            let family = sanitize_family(&family);

            let style = if face.is_italic() { "italic" } else { "normal" }.to_string();
            let weight = best_effort_weight(&face);

            return (family, style, weight);
        }
    }

    (
        sanitize_family(fallback_stem),
        "normal".to_string(),
        FontWeight::Fixed { value: 400 },
    )
}

fn best_effort_family(face: &ttf_parser::Face<'_>) -> Option<String> {
    use ttf_parser::name_id;

    let mut best: Option<String> = None;
    for name in face.names() {
        let id = name.name_id;
        if id == name_id::TYPOGRAPHIC_FAMILY || id == name_id::FAMILY {
            if let Some(s) = name.to_string() {
                best = Some(s);
                if id == name_id::TYPOGRAPHIC_FAMILY {
                    return best;
                }
            }
        }
    }
    best
}

fn best_effort_weight(face: &ttf_parser::Face<'_>) -> FontWeight {
    let wght = ttf_parser::Tag::from_bytes(b"wght");
    for axis in face.variation_axes() {
        if axis.tag == wght {
            let min = axis.min_value.round() as i32;
            let max = axis.max_value.round() as i32;
            if (1..=1000).contains(&min) && (1..=1000).contains(&max) && min <= max {
                return FontWeight::Variable {
                    min: min as u16,
                    max: max as u16,
                };
            }
        }
    }

    FontWeight::Fixed {
        value: face.weight().to_number(),
    }
}

fn resolve_pack_source_dir(app: &AppHandle, pack_id: &str) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource_dir failed: {e}"))?
        .join("resources")
        .join("fonts")
        .join(pack_id);
    if bundled.exists() {
        return Ok(bundled);
    }

    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("fonts")
        .join(pack_id);
    if dev.exists() {
        return Ok(dev);
    }

    Err(format!(
        "font pack resources not found for {pack_id} (checked {} and {})",
        bundled.display(),
        dev.display()
    ))
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
    for entry in WalkDir::new(from)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(from)
            .map_err(|e| format!("strip_prefix failed: {e}"))?;
        let dest = to.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create_dir_all failed: {e}"))?;
        }
        fs::copy(entry.path(), &dest).map_err(|e| format!("copy failed: {e}"))?;
    }
    Ok(())
}

fn load_pack_version(root: &Path) -> Option<PackVersionFile> {
    let path = pack_version_path(root);
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_pack_version(root: &Path, pack_id: &str, pack_version: &str) -> Result<(), String> {
    let pv = PackVersionFile {
        schema_version: 1,
        pack_id: pack_id.to_string(),
        pack_version: pack_version.to_string(),
    };
    let bytes = serde_json::to_vec_pretty(&pv)
        .map_err(|e| format!("serialize pack version failed: {e}"))?;
    atomic_write(&pack_version_path(root), &bytes)
}

fn collect_fonts(root: &Path) -> Result<Vec<FontRecord>, String> {
    let root_canon = root
        .canonicalize()
        .map_err(|e| format!("canonicalize fonts root failed: {e}"))?;
    let bundled = bundled_dir(root);
    let bundled_canon = bundled.canonicalize().ok();
    let user = user_dir(root);

    let mut out = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    for (dir, source) in [(bundled, "bundled"), (user, "user")] {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let format = detect_format(path);
            if !is_allowed_font_ext(&format) {
                continue;
            }

            let abs = path
                .canonicalize()
                .map_err(|e| format!("canonicalize font file failed: {e}"))?;
            if !abs.starts_with(&root_canon) {
                continue;
            }

            let mut bytes = Vec::new();
            fs::File::open(&abs)
                .and_then(|mut f| f.read_to_end(&mut bytes))
                .map_err(|e| format!("read font file failed: {e}"))?;

            let sha = sha256_hex(&bytes);
            let id = format!("sha256:{sha}");
            if !seen_ids.insert(id.clone()) {
                continue;
            }

            let stem = abs
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown");
            let (family, style, weight) = try_parse_font_meta(&bytes, &format, stem);

            let (spdx, license_file) = if source == "bundled" {
                let candidates = ["OFL.txt", "LICENSE.txt", "UFL.txt"];
                let mut chosen: Option<PathBuf> = None;
                let mut cur = abs.parent().map(Path::to_path_buf);
                while let Some(dir) = cur {
                    for cand in candidates {
                        let p = dir.join(cand);
                        if p.exists() {
                            chosen = Some(p);
                            break;
                        }
                    }
                    if chosen.is_some() {
                        break;
                    }

                    if let Some(bundled_root) = bundled_canon.as_ref() {
                        if !dir.starts_with(bundled_root) || dir == *bundled_root {
                            break;
                        }
                    }

                    cur = dir.parent().map(Path::to_path_buf);
                }
                let spdx = match chosen
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                {
                    Some("LICENSE.txt") => "Apache-2.0",
                    Some("UFL.txt") => "UFL-1.0",
                    Some("OFL.txt") => "OFL-1.1",
                    _ => "UNKNOWN",
                };
                (
                    spdx.to_string(),
                    chosen.map(|p| p.to_string_lossy().to_string()),
                )
            } else {
                ("UNKNOWN".to_string(), None)
            };

            out.push(FontRecord {
                id,
                family,
                style,
                weight,
                source: source.to_string(),
                format,
                path: abs.to_string_lossy().to_string(),
                license: FontLicense { spdx, license_file },
            });
        }
    }

    out.sort_by(|a, b| a.family.cmp(&b.family).then(a.id.cmp(&b.id)));
    Ok(out)
}

fn rebuild_manifest_inner(root: &Path) -> Result<Manifest, String> {
    ensure_runtime_dirs(root)?;
    let fonts = collect_fonts(root)?;
    let manifest = Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        generated_at: now_rfc3339(),
        pack_version: DESIGN_PACK_VERSION.to_string(),
        fonts,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("serialize manifest failed: {e}"))?;
    atomic_write(&manifest_path(root), &bytes)?;
    Ok(manifest)
}

fn load_manifest(root: &Path) -> Result<Manifest, String> {
    let bytes = fs::read(manifest_path(root)).map_err(|e| format!("read manifest failed: {e}"))?;
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse manifest failed: {e}"))?;
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err("manifest schemaVersion mismatch".to_string());
    }
    Ok(manifest)
}

fn parse_hash_id(input: &str) -> Option<&str> {
    input.strip_prefix("sha256:").or(Some(input))
}

#[tauri::command]
pub fn fonts_bootstrap_pack(app: AppHandle, pack_id: Option<String>) -> Result<(), String> {
    let pack_id = pack_id.unwrap_or_else(|| DESIGN_PACK_ID.to_string());
    if pack_id != DESIGN_PACK_ID {
        return Err(format!(
            "unknown pack_id '{pack_id}' (only '{DESIGN_PACK_ID}' is supported)"
        ));
    }

    let root = fonts_root(&app)?;
    ensure_runtime_dirs(&root)?;

    let prev = load_pack_version(&root);
    let needs_copy = prev
        .as_ref()
        .map(|p| p.pack_version != DESIGN_PACK_VERSION || p.pack_id != DESIGN_PACK_ID)
        .unwrap_or(true)
        || !bundled_dir(&root).exists()
        || !WalkDir::new(bundled_dir(&root))
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| e.file_type().is_file());

    if needs_copy {
        let from = resolve_pack_source_dir(&app, &pack_id)?;
        copy_dir_recursive(&from, &bundled_dir(&root))?;
        write_pack_version(&root, &pack_id, DESIGN_PACK_VERSION)?;
    }

    Ok(())
}

#[tauri::command]
pub fn fonts_rebuild_manifest(app: AppHandle) -> Result<Manifest, String> {
    let root = fonts_root(&app)?;
    rebuild_manifest_inner(&root)
}

#[tauri::command]
pub fn fonts_list(app: AppHandle) -> Result<Manifest, String> {
    let root = fonts_root(&app)?;
    ensure_runtime_dirs(&root)?;

    match load_manifest(&root) {
        Ok(m) => Ok(m),
        Err(_) => rebuild_manifest_inner(&root),
    }
}

#[tauri::command]
pub fn fonts_import(app: AppHandle, paths: Vec<String>) -> Result<ImportResult, String> {
    let root = fonts_root(&app)?;
    ensure_runtime_dirs(&root)?;

    let mut imported = Vec::new();
    let mut skipped = Vec::new();

    for raw in paths {
        let src = PathBuf::from(&raw);
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !is_allowed_font_ext(&ext) {
            skipped.push(raw);
            continue;
        }

        let meta = fs::metadata(&src).map_err(|e| format!("stat failed for {raw}: {e}"))?;
        if meta.len() > MAX_IMPORT_BYTES {
            skipped.push(raw);
            continue;
        }

        let bytes = fs::read(&src).map_err(|e| format!("read failed for {raw}: {e}"))?;
        let sha = sha256_hex(&bytes);
        let id = format!("sha256:{sha}");

        let dest = user_dir(&root).join(format!("{sha}.{ext}"));
        if dest.exists() {
            skipped.push(raw);
            continue;
        }
        atomic_write(&dest, &bytes)?;
        imported.push(id);
    }

    let manifest = rebuild_manifest_inner(&root)?;

    Ok(ImportResult {
        imported,
        skipped,
        manifest,
    })
}

#[tauri::command]
pub fn fonts_remove(app: AppHandle, font_id: String) -> Result<(), String> {
    let root = fonts_root(&app)?;
    ensure_runtime_dirs(&root)?;

    let hash = parse_hash_id(&font_id).ok_or_else(|| "invalid font_id".to_string())?;
    let hash = hash.trim();
    if hash.is_empty() {
        return Err("invalid font_id".to_string());
    }

    let mut removed_any = false;
    for ext in ["ttf", "otf", "woff2", "woff"] {
        let p = user_dir(&root).join(format!("{hash}.{ext}"));
        if p.exists() {
            fs::remove_file(&p).map_err(|e| format!("remove failed: {e}"))?;
            removed_any = true;
        }
    }

    if removed_any {
        let _ = rebuild_manifest_inner(&root)?;
    }
    Ok(())
}
