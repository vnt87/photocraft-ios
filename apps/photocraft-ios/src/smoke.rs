//! Debug-only native service probes, selected through simctl's child environment.
use photocraft_ui_egui::PhotocraftApp;

pub fn run(app: &mut PhotocraftApp, mode: &str) -> Result<(), String> {
    let pixels = [255, 0, 0, 255, 0, 255, 0, 127, 0, 0, 255, 0, 40, 80, 120, 255];
    let copy = app.services.clipboard_set_image.as_mut().ok_or("clipboard copy is unavailable")?;
    copy(2, 2, &pixels)?;
    let paste = app.services.clipboard_get_image.as_mut().ok_or("clipboard paste is unavailable")?;
    let actual = paste().ok_or("clipboard paste returned no image")?;
    if actual != (2, 2, pixels.to_vec()) {
        return Err("native clipboard pixels changed".into());
    }
    log::info!("iOS service smoke: native clipboard roundtrip passed");
    let previous = app.services.load_prefs.as_mut().and_then(|load| load());
    let text = serde_json::to_string(app.session.prefs()).map_err(|error| error.to_string())?;
    app.services.save_prefs.as_mut().ok_or("preferences writer unavailable")?(&text)?;
    if app.services.load_prefs.as_mut().and_then(|load| load()).as_deref() != Some(text.as_str()) {
        return Err("native preferences roundtrip failed".into());
    }
    if let Some(previous) = previous {
        app.services.save_prefs.as_mut().ok_or("preferences writer unavailable")?(&previous)?;
    }
    log::info!("iOS service smoke: sandbox preferences roundtrip passed");
    match mode {
        "clipboard" => {}
        "url" => {
            app.services.open_url.as_ref().ok_or("URL service unavailable")?("https://example.com")?;
            log::info!("iOS service smoke: system browser requested");
        }
        "open" => {
            app.open_dialog_file()?;
            log::info!("iOS service smoke: open picker requested");
        }
        "save" => {
            app.run("file.new", serde_json::json!({"width": 16, "height": 16}))?;
            app.save_as(None)?;
            log::info!("iOS service smoke: synthetic document export requested");
        }
        _ => return Err("unknown iOS service smoke mode".into()),
    }
    Ok(())
}
