//! Two-finger canvas gestures for touch screens (iPad): pan, pinch-zoom and rotate, plus palm
//! rejection. Platform-agnostic: it consumes `egui::Event::Touch` (or hand-built [`TouchPoint`]s)
//! and produces per-event [`GestureDelta`]s the canvas applies to the view ([`apply_to_view`]).
//!
//! - The first two contacts are tracked; further ones are ignored until one lifts.
//! - Each move yields the change since the previous one in centroid (pan), finger distance
//!   (zoom factor) and finger angle (rotation, wrapped to (-pi, pi] so crossing +-180 degrees is
//!   seamless). [`GestureDelta::pivot`] is the *previous* centroid: zooming and rotating about it,
//!   then panning, keeps the document point that was under the fingers under them.
//! - Palm rejection: a lone contact never produces a delta, and [`GestureRecognizer::blocks_painting`]
//!   tells the canvas to ignore the emulated pointer of a lone finger while a stylus is active
//!   (touching or hovering), and of any two-finger gesture.
//! - Pen contacts: where the platform tells pen from finger touches ([`GestureRecognizer::feed_frame_with`],
//!   iPadOS through `StylusFeed::is_pen_touch`), a pen touch is never a finger: it is not a
//!   gesture contact nor a palm. egui-winit drives its emulated pointer from the first touch that
//!   lands while no touch owns it; when that touch is the pen, a palm resting on the canvas does
//!   not block the pen's stroke. When the palm landed first it owns the pointer and painting stays
//!   blocked (egui-winit gives the pointer to the pen only after the palm lifts).
//!
//! Never panics: non-finite positions are dropped, zero finger distance neither zooms nor rotates.

use egui::{Pos2, Rect, Vec2};

use crate::canvas::ViewXform;
use crate::state::View;

/// Finger distances (points) at or below this give no reliable scale or angle.
const MIN_DISTANCE: f32 = 1.0e-3;
/// One event never scales the view by more than this factor (either way): a glitchy jump.
const MAX_STEP: f32 = 4.0;

/// The phase of one touch contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

impl From<egui::TouchPhase> for TouchPhase {
    fn from(p: egui::TouchPhase) -> Self {
        match p {
            egui::TouchPhase::Start => Self::Start,
            egui::TouchPhase::Move => Self::Move,
            egui::TouchPhase::End => Self::End,
            egui::TouchPhase::Cancel => Self::Cancel,
        }
    }
}

/// One touch contact event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchPoint {
    pub id: u64,
    pub pos: Pos2,
    pub phase: TouchPhase,
}

/// What a two-finger move changed since the previous one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureDelta {
    /// How far the fingers' centroid moved (screen points).
    pub pan_delta: Vec2,
    /// Finger distance now over before (>1 zooms in).
    pub zoom_factor: f32,
    /// Change of the angle between the fingers, radians in (-pi, pi]; clockwise on screen is positive.
    pub rotation_delta_rad: f32,
    /// The centroid before this move: zoom and rotate about it, then pan.
    pub pivot: Pos2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Contact {
    id: u64,
    pos: Pos2,
}

/// The two-finger state machine.
#[derive(Clone, Debug, Default)]
pub struct GestureRecognizer {
    /// At most two tracked contacts, in the order they landed.
    active_touches: Vec<Contact>,
    last_distance: Option<f32>,
    last_angle: Option<f32>,
    last_centroid: Option<Pos2>,
    /// The egui frame [`Self::feed_frame`] last ran in.
    last_frame: Option<u64>,
    /// Finger contacts currently down (tracked or not), for [`Self::blocks_painting`].
    down: Vec<u64>,
    /// The touch driving egui-winit's emulated pointer (the first one to land while none did),
    /// and whether it is a pen.
    pointer: Option<(u64, bool)>,
}

fn finite(p: Pos2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// `a` wrapped to (-pi, pi].
fn wrap_rad(a: f32) -> f32 {
    if !a.is_finite() {
        return 0.0;
    }
    let tau = std::f32::consts::TAU;
    let mut x = a % tau;
    if x > std::f32::consts::PI {
        x -= tau;
    } else if x <= -std::f32::consts::PI {
        x += tau;
    }
    x
}

impl GestureRecognizer {
    /// Forget everything (a focus loss or a tool change in the middle of a gesture).
    pub fn reset(&mut self) {
        let frame = self.last_frame;
        *self = Self::default();
        self.last_frame = frame;
    }

    /// Contacts currently down.
    pub fn touch_count(&self) -> usize {
        self.down.len()
    }

    /// Two contacts are tracked: a pan/zoom/rotate gesture is in progress.
    pub fn is_gesturing(&self) -> bool {
        self.active_touches.len() >= 2
    }

    /// Should the canvas ignore the emulated pointer (no painting, no tool drag)? True during a
    /// two-finger gesture, and for a lone finger while a stylus is touching or hovering (palm).
    /// A pen that owns the emulated pointer is never blocked by a resting palm.
    pub fn blocks_painting(&self, stylus_active: bool) -> bool {
        let pen_drives_pointer = matches!(self.pointer, Some((_, true)));
        self.is_gesturing() || (stylus_active && !self.down.is_empty() && !pen_drives_pointer)
    }

    /// Mirror egui-winit's choice of the touch that drives the emulated pointer.
    fn track_pointer(&mut self, id: u64, phase: TouchPhase, pen: bool) {
        match phase {
            TouchPhase::Start if self.pointer.is_none() => self.pointer = Some((id, pen)),
            TouchPhase::End | TouchPhase::Cancel if self.pointer.is_some_and(|(p, _)| p == id) => self.pointer = None,
            _ => {}
        }
    }

    fn baseline(&mut self) {
        match (self.active_touches.first(), self.active_touches.get(1)) {
            (Some(a), Some(b)) => {
                self.last_centroid = Some(centroid(a.pos, b.pos));
                let (d, ang) = dist_angle(a.pos, b.pos);
                self.last_distance = (d > MIN_DISTANCE).then_some(d);
                self.last_angle = (d > MIN_DISTANCE).then_some(ang);
            }
            _ => {
                self.last_centroid = None;
                self.last_distance = None;
                self.last_angle = None;
            }
        }
    }

    /// Feed one touch event; returns the gesture change it caused, if any.
    ///
    /// `stylus_active`: a stylus is touching or hovering. A lone finger then never starts
    /// anything (it yields no delta in any case); two fingers still pan, zoom and rotate, so a
    /// hand holding the pencil can pinch with the other one.
    pub fn update(&mut self, touch: TouchPoint, stylus_active: bool) -> Option<GestureDelta> {
        let _ = stylus_active; // a lone contact never yields a delta; see `blocks_painting`
        if !finite(touch.pos) {
            return None;
        }
        match touch.phase {
            TouchPhase::Start => {
                if !self.down.contains(&touch.id) {
                    self.down.push(touch.id);
                }
                if let Some(c) = self.active_touches.iter_mut().find(|c| c.id == touch.id) {
                    c.pos = touch.pos;
                } else if self.active_touches.len() < 2 {
                    self.active_touches.push(Contact { id: touch.id, pos: touch.pos });
                }
                self.baseline();
                None
            }
            TouchPhase::End | TouchPhase::Cancel => {
                self.down.retain(|&id| id != touch.id);
                let before = self.active_touches.len();
                self.active_touches.retain(|c| c.id != touch.id);
                if self.active_touches.len() != before {
                    // A freed slot goes to a contact that was down but untracked.
                    self.baseline();
                }
                None
            }
            TouchPhase::Move => {
                let c = self.active_touches.iter_mut().find(|c| c.id == touch.id)?;
                c.pos = touch.pos;
                self.step()
            }
        }
    }

    fn step(&mut self) -> Option<GestureDelta> {
        let (a, b) = (self.active_touches.first()?.pos, self.active_touches.get(1)?.pos);
        let now_c = centroid(a, b);
        let (dist, angle) = dist_angle(a, b);
        let pivot = self.last_centroid.unwrap_or(now_c);
        let pan_delta = now_c - pivot;
        let usable = dist > MIN_DISTANCE && dist.is_finite();
        let zoom_factor = match (self.last_distance, usable) {
            (Some(d0), true) => (dist / d0).clamp(1.0 / MAX_STEP, MAX_STEP),
            _ => 1.0,
        };
        let rotation_delta_rad = match (self.last_angle, usable) {
            (Some(a0), true) => wrap_rad(angle - a0),
            _ => 0.0,
        };
        self.last_centroid = Some(now_c);
        self.last_distance = usable.then_some(dist);
        self.last_angle = usable.then_some(angle);
        let delta = GestureDelta { pan_delta, zoom_factor, rotation_delta_rad, pivot };
        let moved = pan_delta != Vec2::ZERO || zoom_factor != 1.0 || rotation_delta_rad != 0.0;
        (moved && pan_delta.x.is_finite() && pan_delta.y.is_finite()).then_some(delta)
    }

    /// Feed a frame's raw events once per egui frame (the canvas can run in several passes; later
    /// passes of the same frame return nothing). Contacts that start outside `area` are ignored.
    pub fn feed_frame(&mut self, frame: u64, events: &[egui::Event], area: Rect, stylus_active: bool) -> Vec<GestureDelta> {
        self.feed_frame_with(frame, events, area, stylus_active, |_| false)
    }

    /// [`Self::feed_frame`] where `is_pen(touch id)` tells pen contacts from fingers: pen touches
    /// only decide who drives the emulated pointer; they never gesture and are never a palm.
    pub fn feed_frame_with(&mut self, frame: u64, events: &[egui::Event], area: Rect, stylus_active: bool, is_pen: impl Fn(u64) -> bool) -> Vec<GestureDelta> {
        if self.last_frame.replace(frame) == Some(frame) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for e in events {
            let egui::Event::Touch { device_id: _, id, phase, pos, force: _ } = e else { continue };
            let phase = TouchPhase::from(*phase);
            // egui-winit hands the pointer to a touch wherever it lands, so track it first.
            let pen = is_pen(id.0);
            self.track_pointer(id.0, phase, pen);
            if pen {
                continue;
            }
            if phase == TouchPhase::Start && !area.contains(*pos) {
                continue;
            }
            out.extend(self.update(TouchPoint { id: id.0, pos: *pos, phase }, stylus_active));
        }
        out
    }
}

fn centroid(a: Pos2, b: Pos2) -> Pos2 {
    Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn dist_angle(a: Pos2, b: Pos2) -> (f32, f32) {
    let d = b - a;
    (d.length(), d.y.atan2(d.x))
}

/// Apply `d` to `view`: zoom (clamped to the zoom range) and rotate about the gesture pivot, then
/// pan, so the document point that was under the fingers stays under them. `xf` is the view's
/// mapping before the change and `ppp` the canvas's pixels per point. Returns false (and leaves
/// the view alone) for unusable input.
pub fn apply_to_view(view: &mut View, xf: &ViewXform, d: &GestureDelta, ppp: f32) -> bool {
    let ppp = if ppp.is_finite() && ppp > 0.0 { ppp } else { 1.0 };
    let ok = d.zoom_factor.is_finite()
        && d.zoom_factor > 0.0
        && d.rotation_delta_rad.is_finite()
        && d.pan_delta.x.is_finite()
        && d.pan_delta.y.is_finite()
        && finite(d.pivot)
        && view.zoom.is_finite()
        && view.zoom > 0.0
        && xf.zoom.is_finite()
        && xf.zoom > 0.0;
    if !ok {
        return false;
    }
    let anchor = xf.to_doc(d.pivot);
    let new_zoom = crate::zoom_levels::clamp(view.zoom * d.zoom_factor, view.doc_size);
    // A mirrored view turns the opposite way on screen for the same document rotation.
    let sign = if xf.flip { -1.0 } else { 1.0 };
    let new_rotation = crate::rotate_view::wrap_deg(view.rotation + sign * d.rotation_delta_rad.to_degrees());
    let next = ViewXform { zoom: new_zoom / ppp, rotation: new_rotation, ..*xf };
    let target = d.pivot + d.pan_delta;
    let u = next.unmap_vec((target - next.rect.center()) / next.zoom);
    let center = [anchor[0] as f32 - u.x, anchor[1] as f32 - u.y];
    if !(center[0].is_finite() && center[1].is_finite()) {
        return false;
    }
    view.zoom = new_zoom;
    view.rotation = new_rotation;
    view.center = center;
    true
}

#[cfg(test)]
mod tests {
    use egui::{pos2, vec2};

    use super::*;

    fn tp(id: u64, x: f32, y: f32, phase: TouchPhase) -> TouchPoint {
        TouchPoint { id, pos: pos2(x, y), phase }
    }

    /// Two fingers down at `a` and `b`.
    fn down(r: &mut GestureRecognizer, a: (f32, f32), b: (f32, f32)) {
        assert!(r.update(tp(1, a.0, a.1, TouchPhase::Start), false).is_none());
        assert!(r.update(tp(2, b.0, b.1, TouchPhase::Start), false).is_none());
    }

    fn xf(rotation: f32, flip: bool) -> ViewXform {
        ViewXform { rect: Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)), zoom: 1.0, center: [400.0, 300.0], flip, rotation, aspect: 1.0 }
    }

    fn xf_of(view: &View, base: &ViewXform, ppp: f32) -> ViewXform {
        ViewXform { zoom: view.zoom / ppp, center: view.center, rotation: view.rotation, ..*base }
    }

    #[test]
    fn single_touch_gives_no_delta() {
        let mut r = GestureRecognizer::default();
        assert!(r.update(tp(1, 10.0, 10.0, TouchPhase::Start), false).is_none());
        assert!(r.update(tp(1, 50.0, 50.0, TouchPhase::Move), false).is_none());
        assert!(!r.is_gesturing());
        assert!(r.update(tp(1, 50.0, 50.0, TouchPhase::End), false).is_none());
        assert_eq!(r.touch_count(), 0);
    }

    #[test]
    fn two_finger_drag_pans_by_the_centroid() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (200.0, 100.0));
        let d1 = r.update(tp(1, 110.0, 110.0, TouchPhase::Move), false);
        let d2 = r.update(tp(2, 210.0, 110.0, TouchPhase::Move), false);
        let total = [d1, d2].iter().flatten().fold(Vec2::ZERO, |s, d| s + d.pan_delta);
        assert!((total - vec2(10.0, 10.0)).length() < 1e-4, "{total:?}");
        assert!(r.is_gesturing());
    }

    #[test]
    fn pinch_out_zooms_in_and_pinch_in_zooms_out() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (200.0, 100.0));
        let d = r.update(tp(2, 300.0, 100.0, TouchPhase::Move), false).expect("delta");
        assert!((d.zoom_factor - 2.0).abs() < 1e-5);
        assert!(d.rotation_delta_rad.abs() < 1e-6);
        let d = r.update(tp(2, 200.0, 100.0, TouchPhase::Move), false).expect("delta");
        assert!((d.zoom_factor - 0.5).abs() < 1e-5);
    }

    #[test]
    fn rotation_is_clockwise_positive_and_pivot_is_previous_centroid() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (200.0, 100.0));
        // Second finger swings from east to south (y down): +90 degrees on screen.
        let d = r.update(tp(2, 100.0, 200.0, TouchPhase::Move), false).expect("delta");
        assert!((d.rotation_delta_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-4, "{}", d.rotation_delta_rad);
        assert!((d.pivot - pos2(150.0, 100.0)).length() < 1e-4);
    }

    #[test]
    fn rotation_wraps_across_the_branch_cut() {
        let mut r = GestureRecognizer::default();
        // Angle 170 degrees, then 190 (= -170): a +20 degree turn, not -340.
        let at = |deg: f32| {
            let a = deg.to_radians();
            (100.0 + 50.0 * a.cos(), 100.0 + 50.0 * a.sin())
        };
        down(&mut r, (100.0, 100.0), at(170.0));
        let (x, y) = at(190.0);
        let d = r.update(tp(2, x, y, TouchPhase::Move), false).expect("delta");
        assert!((d.rotation_delta_rad.to_degrees() - 20.0).abs() < 1e-2, "{}", d.rotation_delta_rad.to_degrees());
        // And the other way.
        let (x, y) = at(170.0);
        let d = r.update(tp(2, x, y, TouchPhase::Move), false).expect("delta");
        assert!((d.rotation_delta_rad.to_degrees() + 20.0).abs() < 1e-2);
    }

    #[test]
    fn a_full_turn_accumulates_without_jumps() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (0.0, 0.0), (50.0, 0.0));
        let mut total = 0.0f32;
        for k in 1..=72 {
            let a = (k as f32 * 5.0).to_radians();
            let d = r.update(tp(2, 50.0 * a.cos(), 50.0 * a.sin(), TouchPhase::Move), false).expect("delta");
            assert!((d.rotation_delta_rad.to_degrees() - 5.0).abs() < 1e-2);
            total += d.rotation_delta_rad;
        }
        assert!((total.to_degrees() - 360.0).abs() < 0.1);
    }

    #[test]
    fn zero_distance_is_guarded() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (100.0, 100.0));
        // Still coincident: no scale, no angle, no NaN; the move only pans.
        let d = r.update(tp(1, 110.0, 100.0, TouchPhase::Move), false);
        if let Some(d) = d {
            assert_eq!(d.zoom_factor, 1.0);
            assert_eq!(d.rotation_delta_rad, 0.0);
            assert!(d.pan_delta.x.is_finite());
        }
        // The first usable distance only sets the baseline (the move above separated the
        // fingers by 10); a huge jump after it is clamped, never infinite.
        let d = r.update(tp(2, 300.0, 100.0, TouchPhase::Move), false).expect("delta");
        assert_eq!(d.zoom_factor, MAX_STEP);
        // (Finger 2 passed from the west to the east of finger 1: a half turn.)
        assert!((d.rotation_delta_rad.abs() - std::f32::consts::PI).abs() < 1e-4);
        // Coincident again, then apart: no zoom or turn from a zero baseline.
        assert!(r.update(tp(2, 110.0, 100.0, TouchPhase::Move), false).is_some());
        let d = r.update(tp(2, 110.0, 100.0, TouchPhase::Move), false);
        assert!(d.is_none());
    }

    #[test]
    fn hostile_positions_are_ignored() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (200.0, 100.0));
        assert!(r.update(tp(2, f32::NAN, 0.0, TouchPhase::Move), false).is_none());
        assert!(r.update(tp(2, f32::INFINITY, 0.0, TouchPhase::Move), false).is_none());
        assert!(r.update(tp(2, 1.0e30, 0.0, TouchPhase::Move), false).is_some_and(|d| d.zoom_factor <= MAX_STEP && d.zoom_factor.is_finite()));
        assert!(r.update(tp(99, 5.0, 5.0, TouchPhase::Move), false).is_none(), "unknown id");
        assert!(r.update(tp(99, 5.0, 5.0, TouchPhase::End), false).is_none());
    }

    #[test]
    fn lifting_a_finger_ends_the_gesture_and_a_new_one_rebases() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (200.0, 100.0));
        r.update(tp(2, 200.0, 100.0, TouchPhase::End), false);
        assert!(!r.is_gesturing());
        assert!(r.update(tp(1, 300.0, 300.0, TouchPhase::Move), false).is_none());
        // A new second finger far away: the first move after it is relative to the new pair.
        r.update(tp(3, 400.0, 300.0, TouchPhase::Start), false);
        let d = r.update(tp(3, 400.0, 300.0, TouchPhase::Move), false);
        assert!(d.is_none(), "no change, no delta: {d:?}");
        let d = r.update(tp(3, 500.0, 300.0, TouchPhase::Move), false).expect("delta");
        assert!((d.zoom_factor - 2.0).abs() < 1e-5);
        r.update(tp(1, 0.0, 0.0, TouchPhase::Cancel), false);
        r.update(tp(3, 0.0, 0.0, TouchPhase::Cancel), false);
        assert_eq!(r.touch_count(), 0);
    }

    #[test]
    fn a_third_finger_is_ignored() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (200.0, 100.0));
        r.update(tp(3, 900.0, 900.0, TouchPhase::Start), false);
        assert!(r.update(tp(3, 0.0, 0.0, TouchPhase::Move), false).is_none());
        let d = r.update(tp(2, 300.0, 100.0, TouchPhase::Move), false).expect("delta");
        assert!((d.zoom_factor - 2.0).abs() < 1e-5);
        assert_eq!(r.touch_count(), 3);
    }

    #[test]
    fn palm_rejection_blocks_a_lone_finger_while_the_stylus_is_active() {
        let mut r = GestureRecognizer::default();
        assert!(!r.blocks_painting(true), "nothing down");
        r.update(tp(1, 10.0, 10.0, TouchPhase::Start), true);
        assert!(r.blocks_painting(true));
        assert!(!r.blocks_painting(false), "a lone finger paints when there is no stylus");
        assert!(r.update(tp(1, 90.0, 90.0, TouchPhase::Move), true).is_none());
        r.update(tp(1, 90.0, 90.0, TouchPhase::End), true);
        assert!(!r.blocks_painting(true));
    }

    #[test]
    fn two_fingers_gesture_with_the_stylus_hovering_and_block_painting() {
        let mut r = GestureRecognizer::default();
        down(&mut r, (100.0, 100.0), (200.0, 100.0));
        let d = r.update(tp(2, 300.0, 100.0, TouchPhase::Move), true);
        assert!(d.is_some());
        assert!(r.blocks_painting(false), "a gesture never paints");
    }

    #[test]
    fn feed_frame_reads_egui_events_once_per_frame_and_filters_the_area() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 400.0));
        let touch = |id: u64, phase, x: f32, y: f32| egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(id),
            phase,
            pos: pos2(x, y),
            force: None,
        };
        let mut r = GestureRecognizer::default();
        let evs = [
            touch(1, egui::TouchPhase::Start, 100.0, 100.0),
            touch(2, egui::TouchPhase::Start, 200.0, 100.0),
            touch(3, egui::TouchPhase::Start, 900.0, 900.0),
            touch(2, egui::TouchPhase::Move, 300.0, 100.0),
        ];
        let out = r.feed_frame(1, &evs, area, false);
        assert_eq!(out.len(), 1);
        assert!((out[0].zoom_factor - 2.0).abs() < 1e-5);
        assert_eq!(r.touch_count(), 2, "the outside contact is not tracked");
        assert!(r.feed_frame(1, &evs, area, false).is_empty(), "same frame, second pass");
    }

    fn ev(id: u64, phase: egui::TouchPhase, x: f32, y: f32) -> egui::Event {
        egui::Event::Touch { device_id: egui::TouchDeviceId(0), id: egui::TouchId(id), phase, pos: pos2(x, y), force: None }
    }

    #[test]
    fn a_pen_touch_is_neither_a_palm_nor_a_gesture_finger() {
        use egui::TouchPhase::{End, Move, Start};
        let area = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 400.0));
        let is_pen = |id: u64| id == 9;
        let mut r = GestureRecognizer::default();
        // The pen alone, stylus active: it paints (before, it counted as a lone finger).
        r.feed_frame_with(1, &[ev(9, Start, 10.0, 10.0)], area, true, is_pen);
        assert_eq!(r.touch_count(), 0);
        assert!(!r.blocks_painting(true));
        // A palm lands while the pen draws: the pen owns the pointer, the stroke goes on.
        r.feed_frame_with(2, &[ev(1, Start, 200.0, 300.0), ev(9, Move, 20.0, 10.0)], area, true, is_pen);
        assert_eq!(r.touch_count(), 1);
        assert!(!r.blocks_painting(true));
        // A second finger with the palm is still a gesture, and the pen never joins one.
        let d = r.feed_frame_with(3, &[ev(2, Start, 300.0, 300.0), ev(9, Move, 90.0, 90.0)], area, true, is_pen);
        assert!(d.is_empty() && r.is_gesturing() && r.blocks_painting(true));
        r.feed_frame_with(4, &[ev(2, End, 300.0, 300.0), ev(9, End, 90.0, 90.0)], area, true, is_pen);
        // The pen lifted: the resting palm is a palm again while the pen hovers.
        assert!(r.blocks_painting(true));
        assert!(!r.blocks_painting(false));
        r.feed_frame_with(5, &[ev(1, End, 200.0, 300.0)], area, true, is_pen);
        assert!(!r.blocks_painting(true));
    }

    #[test]
    fn a_palm_that_landed_first_keeps_blocking_the_pen() {
        use egui::TouchPhase::{End, Start};
        let area = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 400.0));
        let is_pen = |id: u64| id == 9;
        let mut r = GestureRecognizer::default();
        // egui-winit gives the pointer to the palm; the pen's touch never drives it.
        r.feed_frame_with(1, &[ev(1, Start, 200.0, 300.0), ev(9, Start, 10.0, 10.0)], area, true, is_pen);
        assert!(r.blocks_painting(true));
        // The pointer owner is tracked even when it lands outside the canvas.
        r.feed_frame_with(2, &[ev(1, End, 200.0, 300.0), ev(9, End, 10.0, 10.0)], area, true, is_pen);
        r.feed_frame_with(3, &[ev(5, Start, 900.0, 900.0), ev(9, Start, 10.0, 10.0), ev(1, Start, 20.0, 20.0)], area, true, is_pen);
        assert_eq!(r.pointer, Some((5, false)));
        assert!(r.blocks_painting(true), "a finger owns the pointer and the palm is down");
        // Without a classifier every touch is a finger, as before.
        let mut plain = GestureRecognizer::default();
        plain.feed_frame(1, &[ev(9, Start, 10.0, 10.0)], area, true);
        assert!(plain.blocks_painting(true));
    }

    #[test]
    fn apply_pan_moves_the_view_opposite_to_the_fingers() {
        let base = xf(0.0, false);
        let mut view = View { zoom: 1.0, center: [400.0, 300.0], ..Default::default() };
        let d = GestureDelta { pan_delta: vec2(30.0, -20.0), zoom_factor: 1.0, rotation_delta_rad: 0.0, pivot: pos2(400.0, 300.0) };
        assert!(apply_to_view(&mut view, &base, &d, 1.0));
        assert!((view.center[0] - 370.0).abs() < 1e-3 && (view.center[1] - 320.0).abs() < 1e-3, "{:?}", view.center);
    }

    #[test]
    fn apply_keeps_the_document_point_under_the_fingers() {
        for (rot, flip, ppp) in [(0.0, false, 1.0), (33.0, false, 1.0), (-120.0, true, 2.0), (170.0, true, 1.0)] {
            let base = ViewXform { zoom: 1.0 / ppp, rotation: rot, ..xf(rot, flip) };
            let mut view = View { zoom: 1.0, center: [400.0, 300.0], rotation: rot, ..Default::default() };
            let pivot = pos2(620.0, 180.0);
            let d = GestureDelta { pan_delta: vec2(15.0, 25.0), zoom_factor: 1.7, rotation_delta_rad: 0.4, pivot };
            let doc = base.to_doc(pivot);
            assert!(apply_to_view(&mut view, &base, &d, ppp));
            let after = xf_of(&view, &base, ppp);
            let s = after.to_screen(doc[0] as f32, doc[1] as f32);
            assert!((s - (pivot + d.pan_delta)).length() < 0.05, "rot {rot} flip {flip}: {s:?}");
            assert!((view.zoom - 1.7).abs() < 1e-5);
            assert!(view.rotation > -180.0 && view.rotation <= 180.0);
        }
    }

    #[test]
    fn apply_rotates_the_content_with_the_fingers() {
        let base = xf(0.0, false);
        let mut view = View { zoom: 1.0, center: [400.0, 300.0], ..Default::default() };
        let d = GestureDelta { pan_delta: Vec2::ZERO, zoom_factor: 1.0, rotation_delta_rad: 30f32.to_radians(), pivot: pos2(400.0, 300.0) };
        assert!(apply_to_view(&mut view, &base, &d, 1.0));
        assert!((view.rotation - 30.0).abs() < 1e-3);
        // Mirrored view: the same finger turn is the opposite document rotation.
        let mut view = View { zoom: 1.0, center: [400.0, 300.0], ..Default::default() };
        assert!(apply_to_view(&mut view, &xf(0.0, true), &d, 1.0));
        assert!((view.rotation + 30.0).abs() < 1e-3);
    }

    #[test]
    fn apply_wraps_rotation_into_the_half_open_range() {
        let base = xf(170.0, false);
        let mut view = View { zoom: 1.0, center: [400.0, 300.0], rotation: 170.0, ..Default::default() };
        let d = GestureDelta { pan_delta: Vec2::ZERO, zoom_factor: 1.0, rotation_delta_rad: 20f32.to_radians(), pivot: pos2(10.0, 10.0) };
        assert!(apply_to_view(&mut view, &base, &d, 1.0));
        assert!((view.rotation + 170.0).abs() < 1e-3, "{}", view.rotation);
    }

    #[test]
    fn apply_clamps_zoom_and_rejects_garbage() {
        let base = xf(0.0, false);
        let mut view = View { zoom: 100.0, center: [400.0, 300.0], ..Default::default() };
        let d = GestureDelta { pan_delta: Vec2::ZERO, zoom_factor: 4.0, rotation_delta_rad: 0.0, pivot: pos2(100.0, 100.0) };
        assert!(apply_to_view(&mut view, &base, &d, 1.0));
        assert!(view.zoom <= 128.0 + 1e-3, "{}", view.zoom);
        let before = view.clone();
        for bad in [
            GestureDelta { zoom_factor: f32::NAN, ..d },
            GestureDelta { zoom_factor: 0.0, ..d },
            GestureDelta { rotation_delta_rad: f32::INFINITY, ..d },
            GestureDelta { pan_delta: vec2(f32::NAN, 0.0), ..d },
            GestureDelta { pivot: pos2(f32::NAN, 0.0), ..d },
        ] {
            assert!(!apply_to_view(&mut view, &base, &bad, 1.0));
        }
        assert_eq!(view.center, before.center);
        assert_eq!(view.zoom, before.zoom);
        // A bad ppp falls back to 1 instead of dividing by zero.
        assert!(apply_to_view(&mut view, &base, &GestureDelta { zoom_factor: 1.0, ..d }, 0.0));
    }

    #[test]
    fn end_to_end_pinch_keeps_the_point_between_the_fingers() {
        let base = xf(0.0, false);
        let mut view = View { zoom: 1.0, center: [400.0, 300.0], ..Default::default() };
        let mut r = GestureRecognizer::default();
        down(&mut r, (300.0, 300.0), (500.0, 300.0));
        let anchor = base.to_doc(pos2(400.0, 300.0));
        // Spread symmetrically about (400, 300), in steps, re-reading the view each time.
        for k in 1..=10 {
            let half = 100.0 + 10.0 * k as f32;
            for (id, x) in [(1, 400.0 - half), (2, 400.0 + half)] {
                if let Some(d) = r.update(tp(id, x, 300.0, TouchPhase::Move), false) {
                    let cur = xf_of(&view, &base, 1.0);
                    assert!(apply_to_view(&mut view, &cur, &d, 1.0));
                }
            }
        }
        let after = xf_of(&view, &base, 1.0);
        let s = after.to_screen(anchor[0] as f32, anchor[1] as f32);
        assert!((s - pos2(400.0, 300.0)).length() < 0.1, "{s:?}");
        assert!((view.zoom - 2.0).abs() < 1e-3, "{}", view.zoom);
    }
}
