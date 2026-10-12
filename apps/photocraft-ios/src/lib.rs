//! Native PhotoCraft iPadOS bootstrap. The platform services are a later ticket.
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
mod apple_entry;
#[cfg(any(target_os = "ios", test))]
mod clipboard;
#[cfg(any(target_os = "ios", test))]
mod documents;
#[cfg(any(target_os = "ios", test))]
mod logging;
/// Apple Pencil input; the mapping and state machine are platform-free and host-tested.
pub mod pencil;
#[cfg(target_os = "ios")]
mod services;
#[cfg(all(target_os = "ios", debug_assertions))]
mod smoke;
#[cfg(any(target_os = "ios", test))]
mod storage;

/// Configure Metal explicitly; UIKit/winit supplies the current window dimensions.
pub fn native_options() -> eframe::NativeOptions {
    let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = eframe::wgpu::Backends::METAL;
    setup.device_descriptor = std::sync::Arc::new(|adapter| {
        let available = adapter.limits();
        let requested = photocraft_ui_egui::gpu_canvas::device_limits(adapter);
        eframe::wgpu::DeviceDescriptor {
            label: Some("photocraft iOS Metal device"),
            required_limits: compatible_limits(requested, &available),
            ..Default::default()
        }
    });
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_app_id("ai.storyteller.photocraft").with_title("PhotoCraft"),
        renderer: eframe::Renderer::Wgpu,
        run_and_return: false,
        wgpu_options: eframe::WgpuConfiguration { wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(setup), ..Default::default() },
        ..Default::default()
    }
}

/// egui uses two inter-stage locations; Metal simulators may expose 15 instead of
/// wgpu's default 16. Keep every other editor limit and the default feature set.
fn compatible_limits(mut requested: eframe::wgpu::Limits, available: &eframe::wgpu::Limits) -> eframe::wgpu::Limits {
    // Below two, device creation must fail rather than pretend the egui shader is supported.
    requested.max_inter_stage_shader_variables = requested.max_inter_stage_shader_variables.min(available.max_inter_stage_shader_variables).max(2);
    requested
}

/// Assemble native iPadOS services; hosts keep the unsupported runner stub.
pub fn ios_services() -> photocraft_ui_egui::Services {
    #[cfg(target_os = "ios")]
    {
        match services::create() {
            Ok(services) => services,
            Err(error) => {
                log::error!("iOS platform services unavailable: {error}");
                photocraft_ui_egui::Services::default()
            }
        }
    }
    #[cfg(not(target_os = "ios"))]
    {
        photocraft_ui_egui::Services::default()
    }
}

/// Launch on iOS's main thread; other platforms return an error without a window.
pub fn run() -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        if let Err(error) = logging::install() {
            eprintln!("PhotoCraft logging unavailable: {error}");
        }
        log::info!("Starting PhotoCraft iPadOS with Metal");
        eframe::run_native(
            "PhotoCraft",
            native_options(),
            Box::new(|cc| {
                // The bootstrap uses winit's app-delegate lifecycle without UIKit scenes.
                // Hide the status bar through the safe platform API so it cannot overlay
                // egui's menu while scene-based safe-area handling is still unavailable.
                use winit::platform::ios::WindowExtIOS;
                if let Some(window) = cc.winit_window() {
                    window.set_prefers_status_bar_hidden(true);
                }
                let mut app = photocraft_ui_egui::PhotocraftApp::new(photocraft_engine::Session::new(), ios_services());
                if let Some(render_state) = cc.wgpu_render_state.clone() {
                    app.set_wgpu(render_state);
                }
                // Apple Pencil tilt, roll, hover and double-tap (winit forwards only touch force).
                if let Err(error) = pencil::install(app.stylus.feed.clone(), &cc.egui_ctx) {
                    log::error!("Apple Pencil input unavailable: {error}");
                }
                if app.services.save_file.is_none() {
                    photocraft_ui_egui::notices::post(
                        &mut app,
                        "Platform services unavailable",
                        vec!["File access and persistence could not be initialized. Documents remain in this session only; check the runner log.".into()],
                        true,
                        None,
                    );
                }
                #[cfg(debug_assertions)]
                if let Ok(mode) = std::env::var("PHOTOCRAFT_IOS_SERVICE_SMOKE") {
                    app.on_started(move |app| {
                        if let Err(error) = smoke::run(app, &mode) {
                            log::error!("iOS service smoke failed: {error}");
                        }
                    });
                }
                Ok(Box::new(app))
            }),
        )
        .map_err(|error| format!("PhotoCraft iOS startup failed: {error}"))
    }
    #[cfg(not(target_os = "ios"))]
    {
        Err("PhotoCraft iOS runner requires an iOS device or simulator".into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn simulator_varying_limit_does_not_reduce_other_editor_limits() {
        let requested = eframe::wgpu::Limits { max_texture_dimension_2d: 16384, max_buffer_size: 512 * 1024 * 1024, ..Default::default() };
        let available = eframe::wgpu::Limits { max_inter_stage_shader_variables: 15, ..requested.clone() };
        let actual = super::compatible_limits(requested.clone(), &available);
        assert_eq!(actual, eframe::wgpu::Limits { max_inter_stage_shader_variables: 15, ..requested });
    }

    #[test]
    fn adapters_below_egui_minimum_cannot_silently_lower_shader_requirements() {
        let available = eframe::wgpu::Limits { max_inter_stage_shader_variables: 1, ..Default::default() };
        assert_eq!(super::compatible_limits(eframe::wgpu::Limits::default(), &available).max_inter_stage_shader_variables, 2);
    }

    #[test]
    fn capable_adapters_keep_the_egui_varying_requirement() {
        let requested = eframe::wgpu::Limits::default();
        let available = eframe::wgpu::Limits { max_inter_stage_shader_variables: 32, ..requested.clone() };
        assert_eq!(super::compatible_limits(requested.clone(), &available), requested);
    }

    #[test]
    fn runner_rejects_non_ios_hosts() {
        if !cfg!(target_os = "ios") {
            assert!(super::run().unwrap_err().contains("iOS"));
        }
    }

    #[test]
    fn viewport_uses_os_size_and_only_metal() {
        let options = super::native_options();
        assert_eq!(options.renderer, eframe::Renderer::Wgpu);
        assert_eq!(options.viewport.app_id.as_deref(), Some("ai.storyteller.photocraft"));
        assert!(options.viewport.inner_size.is_none());
        assert!(!options.run_and_return);
        match options.wgpu_options.wgpu_setup {
            eframe::egui_wgpu::WgpuSetup::CreateNew(setup) => {
                assert_eq!(setup.instance_descriptor.backends, eframe::wgpu::Backends::METAL);
            }
            _ => panic!("runner must create its own Metal device"),
        }
    }

    #[test]
    fn ios_services_do_not_claim_unimplemented_file_or_clipboard_support() {
        let services = super::ios_services();
        assert!(services.file_dialog.is_none());
        assert!(services.write.is_none());
        assert!(services.save_file.is_none());
        assert!(services.clipboard_set_image.is_none());
        assert!(services.clipboard_get_image.is_none());
    }
}
