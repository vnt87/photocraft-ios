//! Safe-area diagnostics for the iPad runner. The wrapper forwards every frame to the app and
//! logs the egui rectangles next to UIKit's window metrics, but only when they change.
//!
//! winit sizes the Metal surface to the safe area while egui lays out the full view; logging
//! both sides lets a device run show whether they agree.

use eframe::egui;
use objc2_foundation::MainThreadMarker;

pub struct SafeAreaProbe<A> {
    inner: A,
    last: Option<String>,
}

impl<A> SafeAreaProbe<A> {
    pub fn new(inner: A) -> Self {
        Self { inner, last: None }
    }

    /// Logs what egui lays out with: the persisted insets, not the per-frame raw input, which only
    /// carries them on resize, scale, focus and occlusion events.
    fn report(&mut self, ctx: &egui::Context) {
        let (screen, insets) = ctx.input(|i| (i.viewport_rect(), i.safe_area_insets().0));
        let uikit = MainThreadMarker::new().and_then(crate::scenes::metrics);
        let ctx_content = ctx.input(|i| i.content_rect());
        let line = match uikit {
            Some([w, h, top, left, bottom, right]) => format!(
                "egui screen {:.0}x{:.0} insets t{:.0} l{:.0} b{:.0} r{:.0} content {:.0}x{:.0}; UIWindow bounds {w:.0}x{h:.0} insets t{top:.0} l{left:.0} b{bottom:.0} r{right:.0}; surface {:.0}x{:.0}",
                screen.width(),
                screen.height(),
                insets.top,
                insets.left,
                insets.bottom,
                insets.right,
                ctx_content.width(),
                ctx_content.height(),
                w - left - right,
                h - top - bottom,
            ),
            None => format!("egui screen {:.0}x{:.0}; UIKit metrics unavailable", screen.width(), screen.height()),
        };
        if self.last.as_deref() != Some(line.as_str()) {
            log::info!("safe area: {line}");
            self.last = Some(line);
        }
    }
}

impl<A: eframe::App> eframe::App for SafeAreaProbe<A> {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.report(ctx);
        self.inner.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.inner.ui(ui, frame);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.inner.raw_input_hook(ctx, raw_input);
        // egui-winit reads the insets from a foreground scene's key window and reports zero until
        // that window exists (launch, scene activation). UIKit's window is the source of truth.
        if let Some([_, _, top, left, bottom, right]) = MainThreadMarker::new().and_then(crate::scenes::metrics) {
            raw_input.safe_area_insets =
                Some(egui::SafeAreaInsets(egui::epaint::MarginF32 { top: top as f32, left: left as f32, bottom: bottom as f32, right: right as f32 }));
        }
    }
}
