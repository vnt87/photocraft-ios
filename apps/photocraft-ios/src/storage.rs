//! Sandbox-local preferences, lossless document codecs, and durable crash recovery.
use photocraft_doc::Document;
use photocraft_format::{RecoveryEntry, RecoveryStore};
use photocraft_ui_egui::{ExportSettings, Recoverable, Services};
use std::{
    cell::RefCell,
    ffi::OsString,
    io::Read,
    path::{Component, Path, PathBuf},
    rc::Rc,
};

const MAX_PREFS: u64 = 4 * 1024 * 1024;
const MAX_METADATA: u64 = 64 * 1024;

fn container_prefix(prefix: &str) -> bool {
    let Some(prefix) = prefix.strip_prefix('/') else { return false };
    let parts: Vec<_> = prefix.split('/').collect();
    if parts.iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
        return false;
    }
    let uuid = |value: &str| {
        value.len() == 36 && value.bytes().enumerate().all(|(i, byte)| if matches!(i, 8 | 13 | 18 | 23) { byte == b'-' } else { byte.is_ascii_hexdigit() })
    };
    match parts.as_slice() {
        ["var", "mobile", "Containers", "Data", "Application", app] | ["private", "var", "mobile", "Containers", "Data", "Application", app] => uuid(app),
        ["Users", _, "Library", "Developer", "CoreSimulator", "Devices", device, "data", "Containers", "Data", "Application", app] => uuid(device) && uuid(app),
        _ => false,
    }
}

/// App updates may move the container. Only existing managed snapshots are relocated;
/// external paths and malformed or escaping paths retain their original meaning.
fn remap_managed_path(path: &str, root: &Path) -> Option<String> {
    const MARKER: &str = "/Library/Application Support/ai.storyteller.photocraft/documents/";
    if path.len() > 4096 || !Path::new(path).is_absolute() || path.contains(['\\', '\0']) {
        return None;
    }
    let (prefix, relative) = path.split_once(MARKER)?;
    if !container_prefix(prefix) {
        return None;
    }
    if Path::new(path).components().any(|c| !matches!(c, Component::RootDir | Component::Normal(_))) {
        return None;
    }
    // Check the raw segments too: Path::components normalizes embedded `.` and empty segments.
    let mut parts = relative.split('/');
    let (reservation, name) = (parts.next()?, parts.next()?);
    if parts.next().is_some() || [reservation, name].iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
        return None;
    }
    let documents = root.join("documents");
    let destination = documents.join(reservation).join(name);
    if !std::fs::symlink_metadata(&destination).ok()?.is_file() {
        return None;
    }
    let canonical_root = root.canonicalize().ok()?;
    let canonical_documents = documents.canonicalize().ok()?;
    let canonical_destination = destination.canonicalize().ok()?;
    if !canonical_documents.starts_with(&canonical_root) || !canonical_destination.starts_with(&canonical_documents) {
        return None;
    }
    let destination = destination.to_str()?;
    (destination != path).then(|| destination.to_owned())
}

fn remap_recent_files(text: String, root: &Path) -> String {
    if text.len() as u64 > MAX_PREFS {
        return text;
    }
    let Ok(mut prefs) = serde_json::from_str::<serde_json::Value>(&text) else { return text };
    let Some(recent) = prefs.get_mut("fileHandling").and_then(|p| p.get_mut("recentFiles")).and_then(serde_json::Value::as_array_mut) else { return text };
    let mut changed = false;
    for entry in recent {
        if let Some(path) = entry.as_str().and_then(|p| remap_managed_path(p, root)) {
            *entry = path.into();
            changed = true;
        }
    }
    if changed { serde_json::to_string(&prefs).unwrap_or(text) } else { text }
}

/// Resolve only the supplied sandbox HOME; never fall back to desktop configuration paths.
pub fn app_support_dir(home: Option<OsString>) -> Result<PathBuf, String> {
    let home = PathBuf::from(home.ok_or("iOS sandbox HOME is unavailable")?);
    if !home.is_absolute() || home.components().any(|c| matches!(c, Component::ParentDir)) || home == Path::new("/") {
        return Err("iOS sandbox HOME must be an absolute directory".into());
    }
    Ok(home.join("Library/Application Support/ai.storyteller.photocraft"))
}

fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > limit {
        return Err("file is not regular or exceeds its size limit".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path).map_err(|e| e.to_string())?.take(limit + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("file exceeds its size limit".into());
    }
    Ok(bytes)
}

fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 255 && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

fn entries(dir: &Path) -> Vec<RecoveryEntry> {
    let mut entries = Vec::new();
    let Ok(files) = std::fs::read_dir(dir) else { return entries };
    for file in files.flatten() {
        let path = file.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Ok(bytes) = bounded_read(&path, MAX_METADATA) else { continue };
        let Ok(info) = serde_json::from_slice::<photocraft_format::autosave::RecoveryInfo>(&bytes) else { continue };
        if !valid_key(&info.key) || path.file_stem().is_none_or(|s| s != info.key.as_str()) {
            continue;
        }
        let bundle = dir.join(format!("{}.pcraft", info.key));
        if std::fs::symlink_metadata(&bundle).is_ok_and(|m| m.is_dir()) && bundle.join("manifest.json").is_file() {
            entries.push(RecoveryEntry { info, bundle });
        }
    }
    entries.sort_by(|a, b| b.info.saved_at.cmp(&a.info.saved_at).then(a.info.key.cmp(&b.info.key)));
    entries
}

/// Export through the shared colour/depth-aware codecs, including full native ZIP documents.
pub fn export_document(doc: &Document, name: &str, settings: &ExportSettings) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut opts = photocraft_io::ExportOptions::default();
    if let Some(q) = settings.jpeg_quality {
        opts.encode.jpeg_quality = q;
    }
    opts.encode.webp_lossless = settings.webp_lossless;
    if let Some(q) = settings.webp_quality {
        opts.encode.webp_quality = q;
    }
    opts.tiff_layers = settings.tiff_layers;
    opts.xmp = if settings.xmp_all { photocraft_io::XmpEmbed::All } else { photocraft_io::XmpEmbed::None };
    opts.tga_bits = settings.tga_bits;
    opts.image_previews = settings.image_previews;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| photocraft_io::export(doc, name, &opts).map(|r| (r.bytes, r.warnings)).map_err(|e| e.to_string())))
        .map_err(|_| "Export failed unexpectedly; the document is unchanged".to_string())?
}

/// Import all shared document formats without flattening native documents.
pub fn import_document(name: &str, bytes: &[u8], max_svg_group_depth: usize) -> Result<(Document, Vec<String>), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        photocraft_io::import_with_svg_group_depth(name, bytes, max_svg_group_depth).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
    }))
    .map_err(|_| "Import failed unexpectedly".to_string())?
}

/// Install persistence only. Picker, document writes, and clipboard belong to their own bridges.
pub fn install(services: &mut Services, root: PathBuf) -> Result<(), String> {
    if !root.is_absolute() || root.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Application Support must be an absolute directory".into());
    }
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let prefs = root.join("preferences.json");
    let prefs_load = prefs.clone();
    let prefs_root = root.clone();
    services.import = Some(Box::new(import_document));
    services.export = Some(Box::new(export_document));
    services.load_prefs = Some(Box::new(move || {
        let text = String::from_utf8(bounded_read(&prefs_load, MAX_PREFS).ok()?).ok()?;
        Some(remap_recent_files(text, &prefs_root))
    }));
    services.save_prefs = Some(Box::new(move |text| {
        if text.len() as u64 > MAX_PREFS {
            return Err("preferences exceed their size limit".into());
        }
        photocraft_format::atomic_write(&prefs, text.as_bytes()).map_err(|e| e.to_string())
    }));
    let dir = root.join("Recovery");
    let store = Rc::new(RefCell::new(RecoveryStore::new(&dir)));
    let s = store.clone();
    services.autosave = Some(Box::new(move |doc, revision, path| {
        s.try_borrow_mut().map_err(|_| "recovery is busy".to_string())?.autosave_checked(doc, revision, path.map(str::to_owned))
    }));
    let s = store.clone();
    services.autosave_results = Some(Box::new(move || s.try_borrow_mut().map(|mut s| s.take_completed()).unwrap_or_default()));
    let s = store.clone();
    services.discard_autosave = Some(Box::new(move |id| {
        if let Ok(mut s) = s.try_borrow_mut()
            && let Err(e) = s.discard(id)
        {
            log::error!("Could not discard recovery: {e}");
        }
    }));
    let adopt_dir = dir.clone();
    services.adopt_autosave = Some(Box::new(move |id, key| {
        if valid_key(key) && entries(&adopt_dir).iter().any(|e| e.info.key == key) {
            if let Ok(mut s) = store.try_borrow_mut() {
                s.adopt(id, key);
            }
        } else {
            log::error!("Rejected invalid recovery key");
        }
    }));
    services.recover = Some(Box::new(move || {
        entries(&dir)
            .into_iter()
            .map(|e| Recoverable {
                key: e.info.key.clone(),
                name: e.info.document_name.clone(),
                path: e.info.original_path.as_deref().and_then(|p| remap_managed_path(p, &root)).or_else(|| e.info.original_path.clone()),
                load: Box::new(move || photocraft_format::recover(&e).map_err(|e| e.to_string())),
            })
            .collect()
    }));
    Ok(())
}

#[cfg(test)]
mod tests;
