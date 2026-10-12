use super::*;
use photocraft_color::{ColorMode, SampleType};
use photocraft_doc::Size;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

fn temp() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!("photocraft-ios-storage-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&root).unwrap();
    root
}
fn doc() -> Arc<Document> {
    Arc::new(Document::new("Recovery test", Size::new(2, 3), ColorMode::Rgb, SampleType::U16))
}
fn services(root: &Path) -> Services {
    let mut s = Services::default();
    install(&mut s, root.to_path_buf()).unwrap();
    s
}
fn completed(s: &mut Services) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some((_, _, result)) = s.autosave_results.as_mut().unwrap()().into_iter().next() {
            return result;
        }
        assert!(std::time::Instant::now() < deadline, "autosave timed out");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
#[test]
fn sandbox_home_never_defaults_to_ambient_configuration() {
    assert!(app_support_dir(None).is_err());
    assert!(app_support_dir(Some("relative".into())).is_err());
    assert!(app_support_dir(Some("/sandbox/../outside".into())).is_err());
    assert!(app_support_dir(Some("/".into())).is_err());
    assert_eq!(app_support_dir(Some("/sandbox".into())).unwrap(), Path::new("/sandbox/Library/Application Support/ai.storyteller.photocraft"));
}
#[test]
fn preferences_survive_relaunch_and_failed_write_preserves_old_file() {
    let root = temp();
    let mut s = services(&root);
    s.save_prefs.as_mut().unwrap()("{\"theme\":\"dark\"}").unwrap();
    let old = s.load_prefs.as_mut().unwrap()().unwrap();
    assert!(s.save_prefs.as_mut().unwrap()(&"x".repeat(MAX_PREFS as usize + 1)).is_err());
    drop(s);
    let mut s = services(&root);
    assert_eq!(s.load_prefs.as_mut().unwrap()(), Some(old));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn native_zip_and_png_use_shared_codecs_and_keep_native_depth() {
    let mut d = (*doc()).clone();
    d.layers.push(photocraft_doc::Layer::group("Editable group", vec![photocraft_doc::Layer::group("Nested", Vec::new())]));
    let (bytes, _) = export_document(&d, "roundtrip.pcraft", &ExportSettings::default()).unwrap();
    let (restored, _) = import_document("roundtrip.pcraft", &bytes, 8).unwrap();
    assert_eq!(restored.size, d.size);
    assert_eq!(restored.depth, SampleType::U16);
    assert_eq!(restored.layers.len(), 1);
    assert_eq!(restored.layers.first().unwrap().name, "Editable group");
    let (png, _) = export_document(&d, "roundtrip.png", &ExportSettings::default()).unwrap();
    let (restored, _) = import_document("roundtrip.png", &png, 8).unwrap();
    assert_eq!(restored.size, d.size);
    assert!(import_document("bad.pcraft", b"broken", 8).is_err());
}
#[test]
fn recovery_survives_multiple_launches_and_adoption_discards_original() {
    let root = temp();
    let d = doc();
    let mut s = services(&root);
    s.autosave.as_mut().unwrap()(&d, 7, None).unwrap();
    completed(&mut s).unwrap();
    drop(s);
    for _ in 0..2 {
        let mut s = services(&root);
        let mut recovered = s.recover.as_mut().unwrap()();
        assert_eq!(recovered.len(), 1);
        let entry = recovered.remove(0);
        assert_eq!((entry.load)().unwrap().depth, SampleType::U16);
        s.adopt_autosave.as_mut().unwrap()(d.id.0, &entry.key);
    }
    let mut s = services(&root);
    let entry = s.recover.as_mut().unwrap()().remove(0);
    s.adopt_autosave.as_mut().unwrap()(d.id.0, &entry.key);
    s.discard_autosave.as_mut().unwrap()(d.id.0);
    assert!(s.recover.as_mut().unwrap()().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn failed_autosave_reports_failure_and_same_revision_retries() {
    let root = temp();
    std::fs::write(root.join("Recovery"), b"blocked").unwrap();
    let mut s = services(&root);
    let d = doc();
    s.autosave.as_mut().unwrap()(&d, 3, None).unwrap();
    assert!(completed(&mut s).is_err());
    std::fs::remove_file(root.join("Recovery")).unwrap();
    s.autosave.as_mut().unwrap()(&d, 3, None).unwrap();
    completed(&mut s).unwrap();
    assert_eq!(s.recover.as_mut().unwrap()().len(), 1);
    drop(s);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn traversal_corrupt_and_oversized_sidecars_are_ignored() {
    let root = temp();
    let dir = root.join("Recovery");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("bad.json"), br#"{"key":"../outside","document_name":"bad","original_path":null,"saved_at":0,"revision":0}"#).unwrap();
    std::fs::write(dir.join("broken.json"), b"{").unwrap();
    std::fs::write(dir.join("big.json"), vec![b' '; MAX_METADATA as usize + 1]).unwrap();
    let mut s = services(&root);
    s.adopt_autosave.as_mut().unwrap()(42, "../outside");
    s.discard_autosave.as_mut().unwrap()(42);
    assert!(s.recover.as_mut().unwrap()().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

fn managed_snapshot(root: &Path) -> (String, String) {
    let current = root.join("documents/reservation/photo.pcraft");
    std::fs::create_dir_all(current.parent().unwrap()).unwrap();
    std::fs::write(&current, b"managed snapshot").unwrap();
    let old = "/var/mobile/Containers/Data/Application/11111111-2222-3333-4444-555555555555/Library/Application Support/ai.storyteller.photocraft/documents/reservation/photo.pcraft".to_string();
    (old, current.to_string_lossy().into_owned())
}

#[test]
fn moved_container_rebases_matching_recent_snapshot_and_preserves_unknown_preferences() {
    let root = temp();
    let (old, current) = managed_snapshot(&root);
    let prefs = serde_json::json!({"fileHandling":{"recentFiles":[old,"/external/photo.psd"],"unknown":42},"future":{"unchanged":[1,2]}});
    let mut s = services(&root);
    s.save_prefs.as_mut().unwrap()(&prefs.to_string()).unwrap();
    let loaded: serde_json::Value = serde_json::from_str(&s.load_prefs.as_mut().unwrap()().unwrap()).unwrap();
    let mut expected = prefs;
    expected["fileHandling"]["recentFiles"][0] = current.into();
    assert_eq!(loaded, expected);
    drop(s);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unrelated_missing_malformed_and_traversal_preferences_are_returned_verbatim() {
    let root = temp();
    managed_snapshot(&root);
    let mut s = services(&root);
    for text in [
        " { \"fileHandling\": { \"recentFiles\": [\"/external/photo.psd\", \"/var/mobile/Containers/Data/Application/11111111-2222-3333-4444-555555555555/Library/Application Support/ai.storyteller.photocraft/documents/missing/photo.pcraft\", \"/var/mobile/Containers/Data/Application/11111111-2222-3333-4444-555555555555/Library/Application Support/ai.storyteller.photocraft/documents/reservation/../photo.pcraft\", \"/var/mobile/Containers/Data/Application/11111111-2222-3333-4444-555555555555/Library/Application Support/ai.storyteller.photocraft/documents/reservation//photo.pcraft\", \"relative/Library/Application Support/ai.storyteller.photocraft/documents/reservation/photo.pcraft\", 7] } } ",
        "{\"fileHandling\":{\"recentFiles\":\"wrong type\"}}",
        "{broken JSON",
    ] {
        s.save_prefs.as_mut().unwrap()(text).unwrap();
        assert_eq!(s.load_prefs.as_mut().unwrap()().as_deref(), Some(text));
    }
    drop(s);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn moved_container_rebases_recovery_original_path_without_changing_sidecar() {
    let root = temp();
    let (old, current) = managed_snapshot(&root);
    let mut s = services(&root);
    let d = doc();
    s.autosave.as_mut().unwrap()(&d, 9, Some(&old)).unwrap();
    completed(&mut s).unwrap();
    drop(s);
    let mut s = services(&root);
    let recovered = s.recover.as_mut().unwrap()();
    assert_eq!(recovered.first().unwrap().path.as_deref(), Some(current.as_str()));
    let disk = entries(&root.join("Recovery"));
    assert_eq!(disk.first().unwrap().info.original_path.as_deref(), Some(old.as_str()));
    drop(s);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn managed_snapshot_symlink_outside_root_does_not_rebase() {
    let root = temp();
    let outside = temp();
    std::fs::write(outside.join("photo.pcraft"), b"outside").unwrap();
    std::fs::create_dir_all(root.join("documents")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("documents/reservation")).unwrap();
    let old = "/var/mobile/Containers/Data/Application/11111111-2222-3333-4444-555555555555/Library/Application Support/ai.storyteller.photocraft/documents/reservation/photo.pcraft";
    let text = serde_json::json!({"fileHandling":{"recentFiles":[old]}}).to_string();
    let mut s = services(&root);
    s.save_prefs.as_mut().unwrap()(&text).unwrap();
    assert_eq!(s.load_prefs.as_mut().unwrap()(), Some(text));
    drop(s);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}

#[test]
fn external_archive_marker_collision_never_rebases() {
    let root = temp();
    managed_snapshot(&root);
    let text = serde_json::json!({"fileHandling":{"recentFiles":["/Volumes/backup/Library/Application Support/ai.storyteller.photocraft/documents/reservation/photo.pcraft"]}}).to_string();
    let mut s = services(&root);
    s.save_prefs.as_mut().unwrap()(&text).unwrap();
    assert_eq!(s.load_prefs.as_mut().unwrap()(), Some(text));
    drop(s);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn physical_and_simulator_container_prefixes_require_valid_uuids() {
    let root = temp();
    let (_, current) = managed_snapshot(&root);
    let app = "11111111-2222-3333-4444-555555555555";
    let device = "AAAA1111-BBBB-2222-CCCC-333333333333";
    for prefix in [
        format!("/var/mobile/Containers/Data/Application/{app}"),
        format!("/private/var/mobile/Containers/Data/Application/{app}"),
        format!("/Users/owner/Library/Developer/CoreSimulator/Devices/{device}/data/Containers/Data/Application/{app}"),
    ] {
        let old = format!("{prefix}/Library/Application Support/ai.storyteller.photocraft/documents/reservation/photo.pcraft");
        assert_eq!(remap_managed_path(&old, &root), Some(current.clone()));
    }
    for prefix in [
        "/var/mobile/Containers/Data/Application/not-a-uuid",
        "/var/mobile/Containers/Data/Application/11111111-2222-3333-4444-55555555555Z",
        "/Users/owner/Library/Developer/CoreSimulator/Devices/not-a-uuid/data/Containers/Data/Application/11111111-2222-3333-4444-555555555555",
        "/Users/../Library/Developer/CoreSimulator/Devices/AAAA1111-BBBB-2222-CCCC-333333333333/data/Containers/Data/Application/11111111-2222-3333-4444-555555555555",
    ] {
        let old = format!("{prefix}/Library/Application Support/ai.storyteller.photocraft/documents/reservation/photo.pcraft");
        assert_eq!(remap_managed_path(&old, &root), None);
    }
    std::fs::remove_dir_all(root).unwrap();
}
