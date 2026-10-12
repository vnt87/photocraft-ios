//! LINUX-2: synthetic multi-touch input (`photocraft_testkit::touch_sim`) driven through the real
//! app headlessly (no GPU, no windowing, no timing: one egui frame per stream frame). Proves
//! pinch zooms the view, two-finger rotation turns the view without touching layer pixels, and a
//! resting palm leaves nothing on the layer while the stylus stroke is committed.

use egui::{Modifiers, PointerButton, Pos2};
use photocraft_doc::LayerId;
use photocraft_geom::Rect;
use photocraft_testkit::touch_sim::{Phase, TouchSample, TouchStreamBuilder};
use photocraft_ui_egui::PhotocraftApp;
use photocraft_ui_egui::canvas::ViewXform;
use photocraft_ui_egui::stylus::PenSample;
use serde_json::json;

type Harness = egui_kittest::Harness<'static, PhotocraftApp>;

const W: usize = 400;
const H: usize = 300;

fn harness() -> Harness {
    egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 720.0)).with_pixels_per_point(1.0).with_max_steps(64).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.session.prefs.edit(|p| p.interface.language = "en".into());
        app
    })
}

/// A white 400x300 document, black 12 px hard brush, 100 % zoom centred, `tool` active.
fn setup(h: &mut Harness, tool: &str) -> LayerId {
    let layer = {
        let app = h.state_mut();
        while app.session.active().is_some() {
            app.run("file.close", json!({"discard": true})).expect("close");
        }
        app.run("file.new", json!({"width": W, "height": H, "background": "white"})).expect("new");
        let layer = app.session.active().expect("document").active_layer.expect("layer");
        app.run("tools.setColors", json!({"foreground": [0.0, 0.0, 0.0, 1.0]})).expect("colours");
        app.run("tools.setBrush", json!({"brush": {"size": 12, "hardness": 1.0}})).expect("brush");
        app.sync_views();
        app.ui.extras.rulers = false;
        layer
    };
    let ctx = h.ctx.clone();
    let (req, _rx) = photocraft_ui_egui::control::ControlRequest::new("ui.set", json!({"tool": tool, "zoom": 1.0, "center": [200, 150]}));
    photocraft_ui_egui::control::handle(h.state_mut(), &ctx, &req);
    h.run_steps(6);
    layer
}

fn pixels(h: &Harness, layer: LayerId) -> Vec<[f32; 4]> {
    let st = h.state().session.active().expect("document");
    let surface = st.doc.layer(layer).expect("layer").surface().expect("pixel layer");
    let mut px = vec![[0.0; 4]; W * H];
    surface.read_rgba_into(Rect::new(0, 0, W as i32, H as i32), &mut px);
    px
}

fn canvas_center(h: &Harness) -> Pos2 {
    h.state().last_canvas_rect.center()
}

fn xform(h: &Harness) -> ViewXform {
    let app = h.state();
    let v = &app.ui.views[0];
    ViewXform {
        rect: photocraft_ui_egui::rulers::content_rect(app, app.last_canvas_rect),
        zoom: v.zoom,
        center: v.center,
        flip: app.ui.view.flip_horizontal,
        rotation: v.rotation,
        aspect: app.ui.view.display_aspect(),
    }
}

fn to_event(s: &TouchSample) -> egui::Event {
    egui::Event::Touch {
        device_id: egui::TouchDeviceId(s.device),
        id: egui::TouchId(s.id),
        phase: match s.phase {
            Phase::Start => egui::TouchPhase::Start,
            Phase::Move => egui::TouchPhase::Move,
            Phase::End => egui::TouchPhase::End,
            Phase::Cancel => egui::TouchPhase::Cancel,
        },
        pos: egui::pos2(s.x, s.y),
        force: s.force,
    }
}

/// One egui frame per stream frame.
fn play(h: &mut Harness, frames: &[Vec<TouchSample>]) {
    for frame in frames {
        for s in frame {
            h.event(to_event(s));
        }
        h.step();
    }
    h.run_steps(3);
}

fn pt(p: Pos2) -> (f32, f32) {
    (p.x, p.y)
}

#[test]
fn test_pinch_to_zoom_scales_viewport() {
    let mut h = harness();
    setup(&mut h, "hand");
    let (z0, c0) = (h.state().ui.views[0].zoom, h.state().ui.views[0].center);
    let c = canvas_center(&h);
    // Fingers 100 -> 200 points apart: twice as far, twice the zoom.
    play(&mut h, &TouchStreamBuilder::new().pinch(pt(c), 50.0, 100.0, 10));
    let v = &h.state().ui.views[0];
    assert!((v.zoom / z0 - 2.0).abs() < 0.02, "zoom {z0} -> {}", v.zoom);
    assert!(v.rotation.abs() < 0.01, "a pure pinch does not rotate: {}", v.rotation);
    // Fingers symmetric about the canvas centre: the document point under it stays there.
    let moved = ((v.center[0] - c0[0]).powi(2) + (v.center[1] - c0[1]).powi(2)).sqrt();
    assert!(moved < 1.0, "centre drifted {moved}");

    // And back in.
    let z1 = v.zoom;
    play(&mut h, &TouchStreamBuilder::new().pinch(pt(c), 100.0, 50.0, 10));
    let z2 = h.state().ui.views[0].zoom;
    assert!((z2 / z1 - 0.5).abs() < 0.01, "zoom {z1} -> {z2}");
}

#[test]
fn test_two_finger_rotate_canvas() {
    let mut h = harness();
    let layer = setup(&mut h, "brush");
    // Give the layer some content so distortion would show.
    h.state_mut().run("paint.stroke", json!({"points": [[50, 50, 1], [350, 250, 1]], "size": 20, "hardness": 1.0, "color": "#336699"})).expect("stroke");
    h.run_steps(2);
    let before = pixels(&h, layer);
    let (z0, c0) = (h.state().ui.views[0].zoom, h.state().ui.views[0].center);
    let c = canvas_center(&h);
    play(&mut h, &TouchStreamBuilder::new().rotate(pt(c), 80.0, 0.0, std::f32::consts::FRAC_PI_4, 12));
    let v = &h.state().ui.views[0];
    assert!((v.rotation.abs() - 45.0).abs() < 0.5, "view angle {}", v.rotation);
    assert!((v.zoom - z0).abs() < 1e-3 * z0, "rotation must not zoom");
    let moved = ((v.center[0] - c0[0]).powi(2) + (v.center[1] - c0[1]).powi(2)).sqrt();
    assert!(moved < 1.0, "centre drifted {moved}");
    assert_eq!(pixels(&h, layer), before, "rotating the view must not touch layer pixels");
}

#[test]
fn test_two_finger_pan_moves_the_view_not_the_pixels() {
    let mut h = harness();
    let layer = setup(&mut h, "brush");
    let before = pixels(&h, layer);
    let (z0, c0) = (h.state().ui.views[0].zoom, h.state().ui.views[0].center);
    let c = canvas_center(&h);
    play(&mut h, &TouchStreamBuilder::new().pan((c.x - 15.0, c.y), (c.x - 15.0 + 60.0, c.y + 20.0), 2, 8));
    let v = &h.state().ui.views[0];
    // Dragging the image right/down shows document points further left/up.
    assert!((v.center[0] - (c0[0] - 60.0)).abs() < 1.0 && (v.center[1] - (c0[1] - 20.0)).abs() < 1.0, "centre {c0:?} -> {:?}", v.center);
    assert!((v.zoom - z0).abs() < 1e-3 * z0 && v.rotation.abs() < 0.01);
    assert_eq!(pixels(&h, layer), before, "two fingers never paint");
}

/// A pointer press, drag along `path` and release (the stylus as the emulated pointer).
fn pointer_stroke(h: &mut Harness, path: &[Pos2]) {
    let Some(first) = path.first() else { return };
    h.event(egui::Event::PointerMoved(*first));
    h.step();
    h.event(egui::Event::PointerButton { pos: *first, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    for p in path.iter().skip(1) {
        h.event(egui::Event::PointerMoved(*p));
        h.step();
    }
    let last = path.last().unwrap_or(first);
    h.event(egui::Event::PointerButton { pos: *last, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.step();
    h.run_steps(4);
}

/// A finger that lands and scrubs along `path`: touch events plus the emulated pointer, as
/// egui-winit delivers them.
fn finger_stroke(h: &mut Harness, path: &[Pos2]) {
    let (Some(first), Some(last)) = (path.first(), path.last()) else { return };
    let touch = |p: &Pos2, phase| to_event(&TouchSample { device: 42, id: 7, x: p.x, y: p.y, phase, force: None });
    h.event(egui::Event::PointerMoved(*first));
    h.event(touch(first, Phase::Start));
    h.event(egui::Event::PointerButton { pos: *first, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    for p in path.iter().skip(1) {
        h.event(touch(p, Phase::Move));
        h.event(egui::Event::PointerMoved(*p));
        h.step();
    }
    h.event(touch(last, Phase::End));
    h.event(egui::Event::PointerButton { pos: *last, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.step();
    h.run_steps(4);
}

fn dark_pixels(px: &[[f32; 4]], rows: std::ops::Range<usize>) -> usize {
    rows.map(|y| (0..W).filter(|x| px.get(y * W + x).is_some_and(|p| p[0] < 0.5)).count()).sum()
}

fn row(h: &Harness, doc_y: f32, x0: f32, x1: f32, n: usize) -> Vec<Pos2> {
    let xf = xform(h);
    (0..=n).map(|i| xf.to_screen(x0 + (x1 - x0) * i as f32 / n as f32, doc_y)).collect()
}

#[test]
fn test_palm_rejection_with_stylus() {
    // Control: with no stylus around, a finger paints like a mouse, so the palm assertion below
    // is not vacuous.
    let mut h = harness();
    let layer = setup(&mut h, "brush");
    let path = row(&h, 220.0, 60.0, 340.0, 14);
    finger_stroke(&mut h, &path);
    assert!(dark_pixels(&pixels(&h, layer), 210..231) > 500, "a lone finger without a stylus paints");

    // The stylus strokes the upper half.
    let layer = setup(&mut h, "brush");
    h.state().stylus.feed.set(Some(PenSample { pressure: 0.8, ..Default::default() }));
    let pen = row(&h, 80.0, 60.0, 340.0, 14);
    let pen_stream = TouchStreamBuilder::new().stylus_stroke(&pen.iter().map(|p| pt(*p)).collect::<Vec<_>>(), 0.8);
    assert_eq!(pen_stream.len(), pen.len() + 1);
    pointer_stroke(&mut h, &pen);
    let after_pen = pixels(&h, layer);
    assert!(dark_pixels(&after_pen, 70..91) > 500, "the stylus stroke is committed");
    assert_eq!(dark_pixels(&after_pen, 0..60) + dark_pixels(&after_pen, 110..H), 0, "and nothing else");

    // The pen hovers (pressure 0) while a palm rests and drags across the lower half.
    h.state().stylus.feed.set(Some(PenSample { pressure: 0.0, ..Default::default() }));
    h.run_steps(2);
    let palm = row(&h, 220.0, 60.0, 340.0, 14);
    finger_stroke(&mut h, &palm);
    assert_eq!(pixels(&h, layer), after_pen, "the palm leaves nothing; only the stylus stroke is on the layer");
    h.state().stylus.feed.set(None);
}
