//! Platform-free Apple Pencil state machine. The UIKit callbacks in `native.rs` only read raw
//! values and call into a [`PencilBridge`]; everything that decides what reaches PhotoCraft
//! (the [`StylusFeed`], the eraser latch, the hover pointer) lives here and is unit-tested on
//! any host.
//!
//! How the Pencil reaches PhotoCraft on iPadOS (winit 0.30.13, egui-winit 0.36.2):
//!
//! - winit's `WinitView` forwards every `UITouch`, Pencil included, as `WindowEvent::Touch`
//!   with `Force::Calibrated { force, max_possible_force, altitude_angle }` and one device id for
//!   all touches; egui-winit turns it into `egui::Event::Touch { force: force / max }` plus an
//!   emulated mouse pointer that follows the first touch. Tilt (azimuth), roll, coalesced
//!   high-rate samples and the touch type are lost, and hover is not delivered at all.
//! - So the bridge adds only what winit drops: pen samples with tilt and roll into the
//!   [`StylusFeed`], the pen's touch id (so the stylus and gesture code can tell pen from finger),
//!   the hover position as an egui pointer move ([`HoverInjector`]), and the Pencil double-tap.
//!   Strokes themselves still come from winit's touch -> emulated pointer path.

use super::mapping::{PencilReading, contact_sample, hover_sample};
use photocraft_ui_egui::stylus::StylusFeed;
use std::sync::{Arc, Mutex, PoisonError};

/// The user's double-tap preference (`UIPencilInteraction.preferredTapAction`), without UIKit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapAction {
    Ignore,
    SwitchEraser,
    SwitchPrevious,
    /// Colour palette, ink attributes, system shortcut...: nothing PhotoCraft maps yet.
    Other,
}

impl TapAction {
    /// From `UIPencilPreferredAction`'s raw value (0 ignore, 1 eraser, 2 previous, ...).
    pub fn from_raw(raw: isize) -> Self {
        match raw {
            0 => Self::Ignore,
            1 => Self::SwitchEraser,
            2 => Self::SwitchPrevious,
            _ => Self::Other,
        }
    }
}

/// Where the hovering pen is, for the egui pointer (UIKit window points).
#[derive(Debug, Default)]
struct HoverShared {
    /// Latest hover position not yet handed to egui.
    pending: Option<(f32, f32)>,
    /// The hover ended (or turned into a touch): egui's pointer should go.
    gone: bool,
}

/// Owns the pen state between UIKit callbacks. Main-thread only in practice (UIKit), but holds
/// nothing thread-bound.
pub struct PencilBridge {
    feed: StylusFeed,
    /// Double-tap toggled to the eraser: every sample carries `eraser`, which the stylus code
    /// turns into the Photoshop pen-flip behaviour (Eraser, then back to the previous tool).
    eraser: bool,
    /// A pencil touch is down (hover callbacks are ignored meanwhile).
    touching: bool,
    /// Bumped by every feed write, so a delayed clear only clears its own sample.
    generation: u64,
    hover: Arc<Mutex<HoverShared>>,
    repaint: Option<egui::Context>,
}

/// How long a double-tap's sample stays in the feed when the pen is neither touching nor
/// hovering, so the next frame sees the eraser flip (milliseconds).
pub const TAP_SAMPLE_MS: u64 = 250;

impl PencilBridge {
    pub fn new(feed: StylusFeed, repaint: Option<egui::Context>) -> Self {
        Self { feed, eraser: false, touching: false, generation: 0, hover: Arc::default(), repaint }
    }

    /// The egui plugin that turns hover positions into pointer moves.
    pub fn hover_injector(&self) -> HoverInjector {
        HoverInjector { shared: Arc::clone(&self.hover) }
    }

    pub fn eraser(&self) -> bool {
        self.eraser
    }

    fn write(&mut self, sample: Option<photocraft_ui_egui::stylus::PenSample>) {
        self.feed.set(sample);
        self.generation = self.generation.wrapping_add(1);
    }

    fn wake(&self) {
        if let Some(ctx) = &self.repaint {
            ctx.request_repaint();
        }
    }

    fn hover_gone(&self) {
        let mut h = self.hover.lock().unwrap_or_else(PoisonError::into_inner);
        h.pending = None;
        h.gone = true;
    }

    /// A Pencil touch began (`id`: the id egui will see, i.e. winit's `UITouch` address), with
    /// its coalesced readings, oldest first.
    pub fn pen_began(&mut self, id: u64, readings: &[PencilReading]) {
        self.feed.mark_pen_touch(id);
        self.touching = true;
        self.hover_gone();
        self.pen_moved(readings);
    }

    /// Coalesced readings of a Pencil touch that moved (or changed force), oldest first.
    pub fn pen_moved(&mut self, readings: &[PencilReading]) {
        for r in readings {
            self.write(Some(contact_sample(r, self.eraser)));
        }
    }

    /// The Pencil touch ended or was cancelled. Its id stays marked: egui handles the end a
    /// frame later and must still see a pen.
    pub fn pen_ended(&mut self) {
        self.touching = false;
        self.write(None);
    }

    /// A finger touch began: UIKit may have reused a pen touch's address for it.
    pub fn finger_began(&mut self, id: u64) {
        self.feed.mark_finger_touch(id);
    }

    /// The Pencil hovers at `pos` (window points) with `reading`'s pose.
    pub fn hover(&mut self, pos: Option<(f64, f64)>, reading: &PencilReading) {
        if self.touching {
            return;
        }
        self.write(Some(hover_sample(reading, self.eraser)));
        if let Some((x, y)) = pos.filter(|(x, y)| x.is_finite() && y.is_finite()) {
            let mut h = self.hover.lock().unwrap_or_else(PoisonError::into_inner);
            h.pending = Some((x as f32, y as f32));
            h.gone = false;
        }
        self.wake();
    }

    /// The Pencil stopped hovering (left the range, or touched down).
    pub fn hover_ended(&mut self) {
        if self.touching {
            return;
        }
        self.write(None);
        self.hover_gone();
        self.wake();
    }

    /// The Pencil was double-tapped. Returns the generation to pass to
    /// [`Self::clear_tap_sample`] after [`TAP_SAMPLE_MS`] when a transient sample was posted.
    ///
    /// PhotoCraft has no "switch to previous tool" command; the stylus code's pen-flip already
    /// swaps between the Eraser and the tool used before it, so both Settings choices toggle the
    /// eraser latch.
    pub fn double_tap(&mut self, action: TapAction) -> Option<u64> {
        match action {
            TapAction::SwitchEraser | TapAction::SwitchPrevious => {}
            TapAction::Ignore | TapAction::Other => return None,
        }
        self.eraser = !self.eraser;
        let current = self.feed.get();
        let transient = current.is_none();
        // Re-post the current sample (or a hover one) with the new end so the next frame flips.
        let base = current.unwrap_or(photocraft_ui_egui::stylus::PenSample { pressure: 0.0, ..Default::default() });
        self.write(Some(photocraft_ui_egui::stylus::PenSample { eraser: self.eraser, ..base }));
        self.wake();
        transient.then_some(self.generation)
    }

    /// Clear a double-tap's transient sample unless anything wrote to the feed since.
    pub fn clear_tap_sample(&mut self, generation: u64) {
        if self.generation == generation && !self.touching {
            self.write(None);
            self.wake();
        }
    }
}

/// egui plugin: turns the hovering pen's position into `PointerMoved` / `PointerGone`, so
/// PhotoCraft's brush outline follows the floating tip (winit delivers no hover on iOS).
pub struct HoverInjector {
    shared: Arc<Mutex<HoverShared>>,
}

impl HoverInjector {
    /// Append the pending hover events to `events`. `zoom` is egui's zoom factor: UIKit window
    /// points are egui points at zoom 1. A frame where a touch is down or moving keeps the touch's
    /// pointer: the hover is dropped (it would fight the stroke).
    pub fn inject(&self, events: &mut Vec<egui::Event>, zoom: f32) {
        let mut h = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        let touching = events.iter().any(|e| matches!(e, egui::Event::Touch { phase: egui::TouchPhase::Start | egui::TouchPhase::Move, .. }));
        let pending = h.pending.take();
        if std::mem::take(&mut h.gone) && !touching {
            events.push(egui::Event::PointerGone);
        }
        if touching {
            return;
        }
        let zoom = if zoom.is_finite() && zoom > 0.0 { zoom } else { 1.0 };
        if let Some((x, y)) = pending {
            let p = egui::pos2(x / zoom, y / zoom);
            if p.x.is_finite() && p.y.is_finite() {
                events.push(egui::Event::PointerMoved(p));
            }
        }
    }
}

impl egui::Plugin for HoverInjector {
    fn debug_name(&self) -> &'static str {
        "photocraft-ios pencil hover"
    }

    fn input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        self.inject(&mut input.events, ctx.zoom_factor());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use photocraft_ui_egui::stylus::PenSample;
    use std::f64::consts::FRAC_PI_4;

    fn reading(force: f64) -> PencilReading {
        PencilReading { force, max_force: 4.0, altitude: FRAC_PI_4, azimuth: 0.0, roll: None }
    }

    fn bridge() -> (PencilBridge, StylusFeed) {
        let feed = StylusFeed::default();
        (PencilBridge::new(feed.clone(), None), feed)
    }

    #[test]
    fn a_touch_feeds_every_coalesced_sample_and_marks_the_pen() {
        let (mut b, feed) = bridge();
        b.pen_began(42, &[reading(1.0), reading(2.0)]);
        b.pen_moved(&[reading(3.0)]);
        let got: Vec<f32> = feed.drain().iter().map(|s| s.pressure).collect();
        assert_eq!(got, vec![0.25, 0.5, 0.75]);
        assert!(feed.is_pen_touch(42));
        assert!((feed.get().map_or(0.0, |s| s.tilt_x) - 45.0).abs() < 1e-3);
        b.pen_ended();
        assert_eq!(feed.get(), None);
        assert!(feed.is_pen_touch(42), "egui still has to see the end as a pen");
        b.finger_began(42);
        assert!(!feed.is_pen_touch(42), "a finger reusing the address is a finger");
    }

    #[test]
    fn hover_feeds_zero_pressure_and_is_ignored_while_touching() {
        let (mut b, feed) = bridge();
        let inj = b.hover_injector();
        b.hover(Some((100.0, 50.0)), &reading(3.0));
        assert_eq!(feed.get().map(|s| s.pressure), Some(0.0));
        let mut ev = Vec::new();
        inj.inject(&mut ev, 2.0);
        assert_eq!(ev, vec![egui::Event::PointerMoved(egui::pos2(50.0, 25.0))]);
        b.pen_began(1, &[reading(2.0)]);
        b.hover(Some((1.0, 1.0)), &reading(0.0));
        assert_eq!(feed.get().map(|s| s.pressure), Some(0.5), "touching wins over a late hover");
        b.hover_ended();
        assert!(feed.get().is_some(), "a hover end during the touch changes nothing");
        b.pen_ended();
        b.hover_ended();
        assert_eq!(feed.get(), None);
    }

    #[test]
    fn the_injector_skips_touch_frames_and_hostile_values() {
        let (mut b, _) = bridge();
        let inj = b.hover_injector();
        b.hover(Some((f64::NAN, 3.0)), &reading(0.0));
        let mut ev = Vec::new();
        inj.inject(&mut ev, 1.0);
        assert!(ev.is_empty(), "no position, no move");
        b.hover(Some((10.0, 20.0)), &reading(0.0));
        let start = egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(1),
            phase: egui::TouchPhase::Start,
            pos: egui::pos2(0.0, 0.0),
            force: None,
        };
        let mut ev = vec![start.clone()];
        inj.inject(&mut ev, 1.0);
        assert_eq!(ev, vec![start], "a touch keeps the pointer");
        b.hover(Some((10.0, 20.0)), &reading(0.0));
        let mut ev = Vec::new();
        inj.inject(&mut ev, f32::NAN);
        assert_eq!(ev, vec![egui::Event::PointerMoved(egui::pos2(10.0, 20.0))], "bad zoom falls back to 1");
        b.hover_ended();
        let mut ev = Vec::new();
        inj.inject(&mut ev, 1.0);
        assert_eq!(ev, vec![egui::Event::PointerGone]);
        let mut ev = Vec::new();
        inj.inject(&mut ev, 1.0);
        assert!(ev.is_empty(), "reported once");
    }

    #[test]
    fn double_tap_toggles_the_eraser_end_on_every_sample() {
        let (mut b, feed) = bridge();
        assert_eq!(b.double_tap(TapAction::Ignore), None);
        assert_eq!(b.double_tap(TapAction::Other), None);
        assert!(!b.eraser());
        // Hovering: the current sample is re-posted with the eraser end, nothing to clear later.
        b.hover(None, &reading(0.0));
        assert_eq!(b.double_tap(TapAction::SwitchEraser), None);
        assert_eq!(feed.get().map(|s| (s.pressure, s.eraser)), Some((0.0, true)));
        b.pen_began(1, &[reading(2.0)]);
        assert_eq!(feed.get().map(|s| s.eraser), Some(true), "later samples keep the end");
        b.pen_ended();
        assert_eq!(b.double_tap(TapAction::SwitchPrevious), Some(b.generation), "away from the glass: transient");
        assert_eq!(feed.get(), Some(PenSample { pressure: 0.0, eraser: false, ..Default::default() }));
    }

    #[test]
    fn a_transient_tap_sample_is_cleared_only_if_nothing_newer_arrived() {
        let (mut b, feed) = bridge();
        let g = b.double_tap(TapAction::SwitchEraser).unwrap_or(u64::MAX);
        b.clear_tap_sample(g);
        assert_eq!(feed.get(), None);
        let g = b.double_tap(TapAction::SwitchEraser).unwrap_or(u64::MAX);
        b.hover(None, &reading(0.0));
        b.clear_tap_sample(g);
        assert!(feed.get().is_some(), "the hover sample stays");
    }

    #[test]
    fn the_double_tap_reaches_the_tool_through_the_pen_flip() {
        use photocraft_ui_egui::state::Tool;
        let feed = StylusFeed::default();
        let mut app = photocraft_ui_egui::PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.stylus.feed = feed.clone();
        app.ui.tool = Tool::Brush;
        let mut b = PencilBridge::new(feed, None);
        let g = b.double_tap(TapAction::SwitchEraser);
        assert!(photocraft_ui_egui::stylus::Stylus::sync_eraser_tool(&mut app));
        assert_eq!(app.ui.tool, Tool::Eraser);
        if let Some(g) = g {
            b.clear_tap_sample(g);
        }
        b.double_tap(TapAction::SwitchEraser);
        assert!(photocraft_ui_egui::stylus::Stylus::sync_eraser_tool(&mut app));
        assert_eq!(app.ui.tool, Tool::Brush, "the second tap restores the tool before the eraser");
    }

    #[test]
    fn tap_actions_from_raw_values() {
        assert_eq!(TapAction::from_raw(0), TapAction::Ignore);
        assert_eq!(TapAction::from_raw(1), TapAction::SwitchEraser);
        assert_eq!(TapAction::from_raw(2), TapAction::SwitchPrevious);
        for raw in [3, 6, -1, isize::MAX] {
            assert_eq!(TapAction::from_raw(raw), TapAction::Other);
        }
    }
}
