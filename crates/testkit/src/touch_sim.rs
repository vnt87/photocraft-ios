//! Synthetic multi-touch streams for headless gesture tests.
//!
//! `photocraft-testkit` sits below the UI layer and must not depend on egui (`cargo xtask layers`),
//! so the streams are made of neutral [`TouchSample`]s. A UI test converts each sample to an
//! `egui::Event::Touch { device_id: TouchDeviceId(device), id: TouchId(id), phase, pos, force }`.
//!
//! A stream is a list of frames; each frame is the samples egui would deliver in one pass. No
//! timing is involved: frame `n` is simply the `n`-th step. Streams are deterministic and every
//! position is finite (non-finite or zero-sized input is replaced by a safe default).

/// Device id every generated sample carries.
pub const DEVICE_ID: u64 = 42;

/// Phase of one contact, mirroring `egui::TouchPhase`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Start,
    Move,
    End,
    Cancel,
}

/// One contact event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchSample {
    pub device: u64,
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub phase: Phase,
    /// Normalised pressure (a stylus); `None` for a finger.
    pub force: Option<f32>,
}

/// Builds frame sequences for common gestures.
#[derive(Clone, Copy, Debug)]
pub struct TouchStreamBuilder {
    device: u64,
    first_id: u64,
}

impl Default for TouchStreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn fin(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

impl TouchStreamBuilder {
    pub fn new() -> Self {
        Self { device: DEVICE_ID, first_id: 1 }
    }

    /// Contact ids start here (so two builders can drive distinct fingers).
    pub fn with_first_id(mut self, id: u64) -> Self {
        self.first_id = id;
        self
    }

    fn s(&self, i: usize, p: (f32, f32), phase: Phase, force: Option<f32>) -> TouchSample {
        TouchSample { device: self.device, id: self.first_id.saturating_add(i as u64), x: fin(p.0), y: fin(p.1), phase, force }
    }

    /// Generic: `n` fingers whose position at progress `t` in `0..=1` is `at(i, t)`. One Start
    /// frame, `steps` Move frames (t = 1/steps .. 1), one End frame.
    fn sequence(&self, n: usize, steps: usize, at: impl Fn(usize, f32) -> (f32, f32)) -> Vec<Vec<TouchSample>> {
        let steps = steps.clamp(1, 10_000);
        let mut frames = Vec::with_capacity(steps + 2);
        frames.push((0..n).map(|i| self.s(i, at(i, 0.0), Phase::Start, None)).collect());
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            frames.push((0..n).map(|i| self.s(i, at(i, t), Phase::Move, None)).collect());
        }
        frames.push((0..n).map(|i| self.s(i, at(i, 1.0), Phase::End, None)).collect());
        frames
    }

    /// Two fingers on a horizontal line through `center`, their distance going from `2 * r_start`
    /// to `2 * r_end`.
    pub fn pinch(&self, center: (f32, f32), r_start: f32, r_end: f32, steps: usize) -> Vec<Vec<TouchSample>> {
        let (r0, r1) = (fin(r_start), fin(r_end));
        self.sequence(2, steps, |i, t| {
            let r = r0 + (r1 - r0) * t;
            let side = if i == 0 { -1.0 } else { 1.0 };
            (fin(center.0) + side * r, fin(center.1))
        })
    }

    /// Two fingers opposite each other at `radius` from `center`, turning from `angle_start_rad`
    /// to `angle_end_rad` (screen space, y down: positive turns clockwise on screen).
    pub fn rotate(&self, center: (f32, f32), radius: f32, angle_start_rad: f32, angle_end_rad: f32, steps: usize) -> Vec<Vec<TouchSample>> {
        let (a0, a1, r) = (fin(angle_start_rad), fin(angle_end_rad), fin(radius));
        self.sequence(2, steps, |i, t| {
            let a = a0 + (a1 - a0) * t;
            let side = if i == 0 { -1.0 } else { 1.0 };
            (fin(center.0) + side * r * a.cos(), fin(center.1) + side * r * a.sin())
        })
    }

    /// `finger_count` fingers side by side (30 points apart) dragged from `start` to `end`.
    pub fn pan(&self, start: (f32, f32), end: (f32, f32), finger_count: usize, steps: usize) -> Vec<Vec<TouchSample>> {
        let n = finger_count.clamp(1, 10);
        self.sequence(n, steps, |i, t| {
            let x = fin(start.0) + (fin(end.0) - fin(start.0)) * t + 30.0 * i as f32;
            let y = fin(start.1) + (fin(end.1) - fin(start.1)) * t;
            (x, y)
        })
    }

    /// One contact with `force` (a stylus) along `path`: Start at its first point, a Move per
    /// further point, End at the last. Empty path gives no frames.
    pub fn stylus_stroke(&self, path: &[(f32, f32)], force: f32) -> Vec<Vec<TouchSample>> {
        let (Some(first), Some(last)) = (path.first(), path.last()) else { return Vec::new() };
        let f = Some(if force.is_finite() { force.clamp(0.0, 1.0) } else { 0.5 });
        let mut frames = vec![vec![self.s(0, *first, Phase::Start, f)]];
        frames.extend(path.iter().skip(1).map(|p| vec![self.s(0, *p, Phase::Move, f)]));
        frames.push(vec![self.s(0, *last, Phase::End, f)]);
        frames
    }

    /// One resting finger (a palm) at `at` for `moves` frames, then lifted.
    pub fn palm(&self, at: (f32, f32), moves: usize) -> Vec<Vec<TouchSample>> {
        self.sequence(1, moves, |_, _| at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinch_frames_start_move_end() {
        let f = TouchStreamBuilder::new().pinch((100.0, 100.0), 10.0, 50.0, 4);
        assert_eq!(f.len(), 6);
        assert!(f.first().is_some_and(|fr| fr.iter().all(|s| s.phase == Phase::Start) && fr.len() == 2));
        let last_move = &f[4];
        assert_eq!((last_move[0].x, last_move[1].x), (50.0, 150.0));
        assert!(f.last().is_some_and(|fr| fr.iter().all(|s| s.phase == Phase::End)));
    }

    #[test]
    fn degenerate_input_stays_finite() {
        let b = TouchStreamBuilder::new();
        for frames in
            [b.pinch((f32::NAN, 0.0), f32::INFINITY, 1.0, 0), b.rotate((0.0, 0.0), f32::NAN, 0.0, f32::NAN, 0), b.pan((0.0, 0.0), (f32::NAN, 1.0), 0, 0)]
        {
            assert!(!frames.is_empty());
            assert!(frames.iter().flatten().all(|s| s.x.is_finite() && s.y.is_finite()));
        }
        assert!(b.stylus_stroke(&[], 1.0).is_empty());
    }

    #[test]
    fn rotate_keeps_the_radius() {
        let f = TouchStreamBuilder::new().rotate((0.0, 0.0), 10.0, 0.0, 1.0, 5);
        for fr in &f {
            let d = ((fr[1].x - fr[0].x).powi(2) + (fr[1].y - fr[0].y).powi(2)).sqrt();
            assert!((d - 20.0).abs() < 1e-3);
        }
    }
}
