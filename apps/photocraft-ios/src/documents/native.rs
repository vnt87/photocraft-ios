//! Audited UIKit boundary. Only the save worker waits; the main event loop never does.
use super::{ReservationBatch, bounded_read, bounded_read_limit, reserve_path, validate_managed_path};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{
    NSArray, NSData, NSDataWritingOptions, NSError, NSFileCoordinator, NSFileCoordinatorReadingOptions, NSFileCoordinatorWritingOptions, NSObject,
    NSObjectProtocol, NSString, NSURL, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions,
};
use objc2_ui_kit::{UIDocumentPickerDelegate, UIDocumentPickerViewController, UIViewController};
use objc2_uniform_type_identifiers::UTType;
use photocraft_ui_egui::{FileDialogAnswer, FileDialogReply, FileDialogRequest, Services};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

thread_local! {
    // UIKit's delegate is weak: hold both objects until its terminal callback.
    static ACTIVE: RefCell<Option<(Retained<PickerDelegate>, Retained<UIDocumentPickerViewController>)>> = const { RefCell::new(None) };
}

enum Completion {
    Open(FileDialogReply, PathBuf),
    Export(mpsc::Sender<Result<Retained<NSURL>, String>>),
}
struct PickerState {
    completion: RefCell<Option<Completion>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements. This class is confined to the
    // main thread and owns only Rust ivars; UIKit callbacks use the declared signatures.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = PickerState]
    struct PickerDelegate;
    // SAFETY: NSObjectProtocol has no additional invariants.
    unsafe impl NSObjectProtocol for PickerDelegate {}
    // SAFETY: All callbacks execute on UIKit's main thread; arguments are borrowed only
    // during the call. Selected NSURLs are retained before crossing to a worker.
    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn picked(&self, _picker: &UIDocumentPickerViewController, urls: &NSArray<NSURL>) {
            if let Some(completion) = self.ivars().completion.borrow_mut().take() {
                match completion {
                    Completion::Open(reply, root) => {
                        let urls: Vec<Retained<NSURL>> = urls.iter().collect();
                        let result = std::thread::Builder::new().name("photocraft-document-import".into()).spawn(move || {
                            let answer = read_selection(urls, &root).map(FileDialogAnswer::Paths).unwrap_or_else(FileDialogAnswer::Unavailable);
                            reply.send(Some(answer));
                        });
                        if let Err(error) = result {
                            log::error!("Could not start document import: {error}");
                        }
                    }
                    Completion::Export(reply) => {
                        let result = urls.iter().next().ok_or_else(|| "The document picker returned no destination".into());
                        let _ = reply.send(result);
                    }
                }
            }
            release_picker();
        }
        #[unsafe(method(documentPickerWasCancelled:))]
        fn cancelled(&self, _picker: &UIDocumentPickerViewController) {
            if let Some(completion) = self.ivars().completion.borrow_mut().take() {
                match completion {
                    Completion::Open(reply, _) => reply.send(None),
                    Completion::Export(reply) => {
                        let _ = reply.send(Err("cancelled".into()));
                    }
                }
            }
            release_picker();
        }
    }
);

fn release_picker() {
    // Retain the receiver until after UIKit has returned from the callback.
    dispatch2::DispatchQueue::main().exec_async(|| {
        ACTIVE.with(|active| {
            active.borrow_mut().take();
        })
    });
}

fn root_controller(mtm: objc2::MainThreadMarker) -> Result<Retained<UIViewController>, String> {
    let window = crate::scenes::window(mtm);
    let window = window.ok_or("No active UIKit window")?;
    let root = window.rootViewController().ok_or("The UIKit window has no root controller")?;
    if root.presentedViewController().is_some() {
        return Err("Another UIKit dialog is already open".into());
    }
    Ok(root)
}

fn fail_completion(completion: Completion, error: String) {
    match completion {
        Completion::Open(reply, _) => reply.send(Some(FileDialogAnswer::Unavailable(error))),
        Completion::Export(reply) => {
            let _ = reply.send(Err(error));
        }
    }
}
fn show_picker(
    picker: Retained<UIDocumentPickerViewController>,
    completion: Completion,
    mtm: objc2::MainThreadMarker,
    ctl: Option<&photocraft_ui_egui::jobs_ui::SaveCtl>,
) {
    let root = match root_controller(mtm) {
        Ok(root) => root,
        Err(error) => {
            fail_completion(completion, error);
            return;
        }
    };
    if ACTIVE.with(|active| active.borrow().is_some()) {
        fail_completion(completion, "A document picker is already open".into());
        return;
    }
    let allocated = PickerDelegate::alloc(mtm).set_ivars(PickerState { completion: RefCell::new(Some(completion)) });
    // SAFETY: NSObject init initializes this allocated subclass, on the main thread;
    // Rust ivars have been initialized and ownership transfers to Retained.
    let delegate: Retained<PickerDelegate> = unsafe { msg_send![super(allocated), init] };
    picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // The worker remains cancellable while this main-queue presentation is pending.
    // Once presented, UIKit can write on selection; only its Cancel button can cancel.
    if let Some(ctl) = ctl
        && let Err(error) = ctl.commit()
    {
        if let Some(completion) = delegate.ivars().completion.borrow_mut().take() {
            fail_completion(completion, error);
        }
        return;
    }
    ACTIVE.with(|active| *active.borrow_mut() = Some((delegate, picker.clone())));
    root.presentViewController_animated_completion(&picker, true, None);
}

/// Security access is tied to the retained immutable URL and balanced even on read errors.
struct SecurityAccess {
    url: Retained<NSURL>,
    started: bool,
}
impl SecurityAccess {
    fn new(url: Retained<NSURL>) -> Self {
        // SAFETY: NSURL is retained and immutable (the binding implements Send + Sync).
        // Foundation permits security scope on worker threads; Drop balances only success.
        let started = unsafe { url.startAccessingSecurityScopedResource() };
        Self { url, started }
    }
}
impl Drop for SecurityAccess {
    fn drop(&mut self) {
        if self.started {
            // SAFETY: The retained URL is live, and this guard owns one successful start.
            unsafe { self.url.stopAccessingSecurityScopedResource() };
        }
    }
}
fn read_selection(urls: Vec<Retained<NSURL>>, root: &Path) -> Result<Vec<String>, String> {
    if urls.len() > 64 {
        return Err("Select at most 64 documents at once".into());
    }
    let mut files = Vec::new();
    let mut batch = ReservationBatch::default();
    let mut total = 0u64;
    for url in urls {
        let access = SecurityAccess::new(url);
        let path = access.url.path().ok_or("The selected document has no file path")?.to_string();
        let name = super::file_name(&path)?;
        let bytes = coordinated_read(&access.url)?;
        total = total.checked_add(bytes.len() as u64).ok_or("Selected documents are too large")?;
        if total > super::MAX_DOCUMENT_BYTES {
            return Err("Selected documents exceed the 512 MiB import limit".into());
        }
        let managed = batch.reserve(root, &name)?;
        remember_destination(&managed, &access.url)?;
        photocraft_format::atomic_write(&managed, &bytes).map_err(|e| e.to_string())?;
        files.push(managed.to_str().ok_or("Non-Unicode managed document path")?.to_owned());
    }
    batch.commit();
    Ok(files)
}

fn coordinated_read(url: &NSURL) -> Result<Vec<u8>, String> {
    let outcome = RefCell::new(None);
    let reader = block2::RcBlock::new(|target: std::ptr::NonNull<NSURL>| {
        // SAFETY: Foundation guarantees a live immutable NSURL during the synchronous
        // coordination accessor; no pointer or borrowed Objective-C object escapes.
        let target = unsafe { target.as_ref() };
        let result = target.path().ok_or("The provider returned no local file path".into()).and_then(|path| bounded_read(Path::new(&path.to_string())));
        *outcome.borrow_mut() = Some(result);
    });
    let mut error: Option<Retained<NSError>> = None;
    NSFileCoordinator::new().coordinateReadingItemAtURL_options_error_byAccessor(url, NSFileCoordinatorReadingOptions::empty(), Some(&mut error), &reader);
    if let Some(error) = error {
        return Err(error.to_string());
    }
    drop(reader);
    outcome.into_inner().ok_or("The file provider did not allow reading")?
}

fn bookmark_path(path: &Path) -> Result<PathBuf, String> {
    Ok(path.parent().ok_or("Missing managed document directory")?.join("metadata").join("destination.bookmark"))
}
fn remember_destination(path: &Path, url: &Retained<NSURL>) -> Result<(), String> {
    let access = SecurityAccess::new(url.clone());
    let url = &access.url;
    // iOS bookmarks preserve implicit security scope; macOS-only WithSecurityScope is
    // deliberately omitted. The picker-provided URL remains retained for this operation.
    let data = url
        .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(NSURLBookmarkCreationOptions::MinimalBookmark, None, None)
        .map_err(|e| e.to_string())?;
    let bookmark = bookmark_path(path)?;
    std::fs::create_dir_all(bookmark.parent().ok_or("Missing bookmark metadata directory")?).map_err(|e| e.to_string())?;
    photocraft_format::atomic_write(&bookmark, &data.to_vec()).map_err(|e| e.to_string())
}
fn resolve_destination(path: &Path) -> Result<Option<Retained<NSURL>>, String> {
    let bookmark = bookmark_path(path)?;
    if !bookmark.exists() {
        return Ok(None);
    }
    let data = NSData::from_vec(bounded_read_limit(&bookmark, 1024 * 1024)?);
    // SAFETY: NSData and NSURL are retained, no borrowed objects escape, and the optional
    // stale output is null (we refresh the bookmark after each successful external write).
    let url = unsafe {
        NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
            &data,
            NSURLBookmarkResolutionOptions::WithoutUI,
            None,
            std::ptr::null_mut(),
        )
    }
    .map_err(|e| format!("The saved destination is unavailable: {e}. Use Save As to choose it again."))?;
    Ok(Some(url))
}

fn coordinated_write(url: Retained<NSURL>, bytes: &[u8], commit: impl Fn() -> Result<(), String>) -> Result<(), String> {
    let access = SecurityAccess::new(url);
    let data = NSData::with_bytes(bytes);
    let outcome = RefCell::new(None);
    let writer = block2::RcBlock::new(|target: std::ptr::NonNull<NSURL>| {
        // SAFETY: NSFileCoordinator supplies a valid NSURL for the duration of its
        // synchronous accessor. It is never retained or used after the accessor returns.
        let target = unsafe { target.as_ref() };
        let result = commit().and_then(|()| data.writeToURL_options_error(target, NSDataWritingOptions::Atomic).map_err(|e| e.to_string()));
        *outcome.borrow_mut() = Some(result);
    });
    let mut error: Option<Retained<NSError>> = None;
    NSFileCoordinator::new().coordinateWritingItemAtURL_options_error_byAccessor(
        &access.url,
        NSFileCoordinatorWritingOptions::ForReplacing,
        Some(&mut error),
        &writer,
    );
    if let Some(error) = error {
        return Err(error.to_string());
    }
    drop(writer);
    outcome.into_inner().ok_or("The file provider did not allow writing")?
}

pub fn install(services: &mut Services, root: PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(root.join("documents")).map_err(|e| e.to_string())?;
    services.save_requires_worker = true;
    let dialog_root = root.clone();
    services.file_dialog = Some(Box::new(move |request, _frame, reply| match request {
        FileDialogRequest::Save { suggested } => {
            let answer = reserve_path(&dialog_root, &suggested)
                .and_then(|p| p.to_str().map(str::to_owned).ok_or("Non-Unicode managed document path".into()))
                .map(FileDialogAnswer::SaveTo)
                .unwrap_or_else(FileDialogAnswer::Unavailable);
            reply.send(Some(answer));
        }
        FileDialogRequest::Open { multiple, extensions, .. } => {
            let root = dialog_root.clone();
            dispatch2::DispatchQueue::main().exec_async(move || {
                let Some(mtm) = objc2::MainThreadMarker::new() else {
                    reply.send(Some(FileDialogAnswer::Unavailable("UIKit main queue unavailable".into())));
                    return;
                };
                let types: Vec<Retained<UTType>> = match extensions {
                    Some(extensions) => extensions.iter().filter_map(|extension| UTType::typeWithFilenameExtension(&NSString::from_str(extension))).collect(),
                    None => ["com.adobe.photoshop-image", "public.image", "public.data"]
                        .iter()
                        .filter_map(|id| UTType::typeWithIdentifier(&NSString::from_str(id)))
                        .collect(),
                };
                let types = NSArray::from_retained_slice(&types);
                let picker = UIDocumentPickerViewController::initForOpeningContentTypes_asCopy(UIDocumentPickerViewController::alloc(mtm), &types, false);
                picker.setAllowsMultipleSelection(multiple);
                show_picker(picker, Completion::Open(reply, root), mtm, None);
            });
        }
    }));
    let write_root = root.clone();
    services.write = Some(Box::new(move |path, bytes| {
        let path = Path::new(path);
        validate_managed_path(&write_root, path)?;
        let destination = resolve_destination(path)?.ok_or("Use document Save or Save As to choose an external destination asynchronously")?;
        coordinated_write(destination.clone(), bytes, || Ok(()))?;
        photocraft_format::atomic_write(path, bytes).map_err(|e| e.to_string())?;
        remember_destination(path, &destination)
    }));
    services.save_file = Some(Arc::new(move |doc, path, settings, ctl| {
        let path = PathBuf::from(path);
        validate_managed_path(&root, &path)?;
        ctl.check()?;
        ctl.progress(0.1, "Encoding document");
        let (bytes, warnings) = crate::storage::export_document(doc, path.to_str().ok_or("Non-Unicode document path")?, settings)?;
        ctl.check()?;
        if let Some(destination) = resolve_destination(&path)? {
            coordinated_write(destination.clone(), &bytes, || ctl.commit())?;
            // External success is the authoritative save result. Keep a reopenable copy.
            photocraft_format::atomic_write(&path, &bytes).map_err(|e| format!("External save succeeded, but the managed copy failed: {e}"))?;
            remember_destination(&path, &destination).map_err(|e| format!("External save succeeded, but its destination could not be remembered: {e}"))?;
            return Ok(warnings);
        }
        // This is a private staging file, never the caller's existing external destination.
        // The picker is constructed only after the encoded bytes have reached disk.
        let stage = path.parent().ok_or("Missing managed directory")?.join("export");
        std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
        let stage = stage.join(super::file_name(path.to_str().ok_or("Non-Unicode document path")?)?);
        photocraft_format::atomic_write(&stage, &bytes).map_err(|e| e.to_string())?;
        let presentation_ctl = ctl.clone();
        let (tx, rx) = mpsc::channel();
        let picker_stage = stage.clone();
        dispatch2::DispatchQueue::main().exec_async(move || {
            let Some(mtm) = objc2::MainThreadMarker::new() else {
                let _ = tx.send(Err("UIKit main queue unavailable".into()));
                return;
            };
            let Some(stage) = picker_stage.to_str() else {
                let _ = tx.send(Err("Non-Unicode export path".into()));
                return;
            };
            let url = NSURL::fileURLWithPath(&NSString::from_str(stage));
            let urls = NSArray::from_retained_slice(&[url]);
            let picker = UIDocumentPickerViewController::initForExportingURLs_asCopy(UIDocumentPickerViewController::alloc(mtm), &urls, true);
            show_picker(picker, Completion::Export(tx), mtm, Some(&presentation_ctl));
        });
        let result = rx.recv().map_err(|_| "The document picker closed without saving".to_owned()).and_then(|result| result);
        let _ = std::fs::remove_file(stage);
        let destination = result?;
        // Never arm a managed path until UIKit confirms the external export completed.
        remember_destination(&path, &destination).map_err(|e| format!("External export succeeded, but its destination could not be remembered: {e}"))?;
        photocraft_format::atomic_write(&path, &bytes).map_err(|e| format!("External export succeeded, but the managed copy failed: {e}"))?;
        Ok(warnings)
    }));
    Ok(())
}
