use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const KEY_W: i32 = 100;
const KEY_H: i32 = 10;

const LEVEL_H: i32 = 30;
const LEVEL_GAP: i32 = 8;
const BAR_PAD_X: f64 = 10.0;
const BAR_PAD_Y: f64 = 12.0;
const BAR_H: f64 = 6.0;

pub const X_BRIGHTNESS_DOWN: i32 = 292;
pub const X_BRIGHTNESS_UP: i32 = 388;
pub const X_VOLUME_MUTE: i32 = 483;
pub const X_VOLUME_DOWN: i32 = 578;
pub const X_VOLUME_UP: i32 = 674;

pub const X_BRIGHTNESS_BAR: i32 = X_BRIGHTNESS_DOWN;
pub const W_BRIGHTNESS_BAR: i32 = X_BRIGHTNESS_UP + KEY_W - X_BRIGHTNESS_DOWN;
pub const X_VOLUME_BAR: i32 = X_VOLUME_DOWN;
pub const W_VOLUME_BAR: i32 = X_VOLUME_UP + KEY_W - X_VOLUME_DOWN;

pub fn build_key_cap(app: &Application, margin_left: i32) -> impl Fn() + 'static {
    let window = ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .default_width(KEY_W)
        .default_height(KEY_H)
        .build();
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_margin(Edge::Left, margin_left);
    window.set_margin(Edge::Bottom, 0);

    let area = DrawingArea::new();
    area.set_content_width(KEY_W);
    area.set_content_height(KEY_H);
    window.set_child(Some(&area));
    window.set_visible(false);

    area.set_draw_func(|_area, cr, _w, _h| {
        let w = KEY_W as f64;
        let h = KEY_H as f64;
        arch_shape(cr, w, h);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        let _ = cr.fill();
    });

    let win_c = window.clone();
    let hide_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    move || {
        if let Some(id) = hide_timer.borrow_mut().take() {
            id.remove();
        }
        win_c.set_visible(true);

        let win2 = win_c.clone();
        let timer2 = hide_timer.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(300), move || {
            win2.set_visible(false);
            *timer2.borrow_mut() = None;
        });
        *hide_timer.borrow_mut() = Some(id);
    }
}

pub fn build_level_bar(
    app: &Application,
    margin_left: i32,
    bar_width: i32,
) -> impl Fn(f32) + 'static {
    let window = ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .default_width(bar_width)
        .default_height(LEVEL_H)
        .build();
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_margin(Edge::Left, margin_left);
    window.set_margin(Edge::Bottom, KEY_H + LEVEL_GAP);

    let area = DrawingArea::new();
    area.set_content_width(bar_width);
    area.set_content_height(LEVEL_H);
    window.set_child(Some(&area));
    window.set_visible(false);

    let fraction: Rc<Cell<f32>> = Rc::new(Cell::new(0.0));

    {
        let fraction = fraction.clone();
        let bw_f = bar_width as f64;
        area.set_draw_func(move |_area, cr, _w, _h| {
            let w = bw_f;
            let frac = fraction.get() as f64;

            let bx = BAR_PAD_X;
            let by = BAR_PAD_Y;
            let bw = w - 2.0 * BAR_PAD_X;
            let bh = BAR_H;
            pill(cr, bx, by, bw, bh, bh / 2.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.20);
            let _ = cr.fill();

            if frac > 0.0 {
                pill(cr, bx, by, (bw * frac).max(bh), bh, bh / 2.0);
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.92);
                let _ = cr.fill();
            }
        });
    }

    let win_c = window.clone();
    let frac_c = fraction.clone();
    let area_c = area.clone();
    let hide_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    move |new_frac: f32| {
        frac_c.set(new_frac.clamp(0.0, 1.0));
        area_c.queue_draw();

        if let Some(id) = hide_timer.borrow_mut().take() {
            id.remove();
        }
        win_c.set_visible(true);

        let win2 = win_c.clone();
        let timer2 = hide_timer.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(1500), move || {
            win2.set_visible(false);
            *timer2.borrow_mut() = None;
        });
        *hide_timer.borrow_mut() = Some(id);
    }
}

fn arch_shape(cr: &cairo::Context, w: f64, h: f64) {
    let cx = w / 2.0;

    cr.new_path();
    cr.move_to(cx, 0.0);

    cr.curve_to(
        cx + w * 0.21,
        0.0,
        w - w * 0.25,
        h,
        w,
        h,
    );

    cr.line_to(0.0, h);

    cr.curve_to(
        w * 0.25,
        h,
        cx - w * 0.21,
        0.0,
        cx,
        0.0,
    );

    cr.close_path();
}

fn pill(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::PI;
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.arc(x + w - r, y + r, r, 3.0 * PI / 2.0, 2.0 * PI);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.close_path();
}
