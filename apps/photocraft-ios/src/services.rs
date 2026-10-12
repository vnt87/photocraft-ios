//! Assemble native services without exposing platform objects to the shared UI.
use std::path::PathBuf;

fn support_directory() -> Result<PathBuf, String> {
    use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask};
    let directories =
        NSFileManager::defaultManager().URLsForDirectory_inDomains(NSSearchPathDirectory::ApplicationSupportDirectory, NSSearchPathDomainMask::UserDomainMask);
    let path = directories.firstObject().and_then(|url| url.path()).ok_or("iOS Application Support directory is unavailable")?;
    let root = PathBuf::from(path.to_string()).join("ai.storyteller.photocraft");
    let expected = crate::storage::app_support_dir(std::env::var_os("HOME"))?;
    // The OS can spell the same container through /var or /private/var.
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    if root.canonicalize().map_err(|error| error.to_string())? != expected.canonicalize().map_err(|error| error.to_string())? {
        return Err("Application Support is outside the expected app container".into());
    }
    Ok(root)
}

pub fn create() -> Result<photocraft_ui_egui::Services, String> {
    let root = support_directory()?;
    let mut services = photocraft_ui_egui::Services::default();
    crate::storage::install(&mut services, root.clone())?;
    crate::documents::install(&mut services, root)?;
    crate::clipboard::install(&mut services);
    services.save_requires_worker = true;
    Ok(services)
}
