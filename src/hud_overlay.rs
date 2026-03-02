use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GBox, DrawingArea, Label, Orientation};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const HUD_MARGIN_BOTTOM: i32 = 0;

const HUD_VOLUME_X: i32 = 560;

const HUD_BRIGHTNESS_X: i32 = 280;

const BAR_W: i32 = 160;
const BAR_H: i32 = 6;

pub fn hud_volume_x()      -> i32 { HUD_VOLUME_X }
pub fn hud_brightness_x()  -> i32 { HUD_BRIGHTNESS_X }

pub fn build_hud_window(
    app: &Application,
    icon_normal: &'static str,
    icon_special: Option<&'static str>,
    margin_left: i32,
) -> impl Fn(f32, bool) + 'static {
    let window = ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, false);
    window.set_margin(Edge::Bottom, HUD_MARGIN_BOTTOM);
    window.set_margin(Edge::Left, margin_left);

    let pill = GBox::new(Orientation::Horizontal, 14);
    pill.add_css_class("hud-pill");
    pill.set_valign(gtk4::Align::Center);
    pill.set_halign(gtk4::Align::Center);

    let icon_label = Label::new(Some(icon_normal));
    icon_label.add_css_class("hud-icon");

    let bar_area = DrawingArea::new();
    bar_area.set_content_width(BAR_W);
    bar_area.set_content_height(BAR_H);
    bar_area.set_valign(gtk4::Align::Center);

    pill.append(&icon_label);
    pill.append(&bar_area);
    window.set_child(Some(&pill));
    window.set_visible(false);

    let fraction: Rc<Cell<f32>> = Rc::new(Cell::new(0.0));
    let special: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let hide_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    {
        let fraction = fraction.clone();
        let special = special.clone();
        bar_area.set_draw_func(move |_area, cr, width, height| {
            let w = width as f64;
            let h = height as f64;
            let r = h / 2.0;
            let frac = fraction.get() as f64;
            let is_special = special.get();

            pill_rect(cr, 0.0, 0.0, w, h, r);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
            let _ = cr.fill();

            if frac > 0.0 && !is_special {
                let fill_w = (w * frac).max(h);
                pill_rect(cr, 0.0, 0.0, fill_w, h, r);
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.88);
                let _ = cr.fill();
            }
        });
    }

    let window_c = window.clone();
    let fraction_c = fraction.clone();
    let special_c = special.clone();
    let hide_timer_c = hide_timer.clone();
    let icon_label_c = icon_label.clone();
    let bar_area_c = bar_area.clone();

    move |new_fraction: f32, new_special: bool| {
        fraction_c.set(new_fraction.clamp(0.0, 1.0));
        special_c.set(new_special);

        if let Some(alt) = icon_special {
            icon_label_c.set_label(if new_special { alt } else { icon_normal });
        }

        bar_area_c.queue_draw();

        if let Some(id) = hide_timer_c.borrow_mut().take() {
            id.remove();
        }

        window_c.set_visible(true);

        let window2 = window_c.clone();
        let timer2 = hide_timer_c.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(1500), move || {
            window2.set_visible(false);
            *timer2.borrow_mut() = None;
        });
        *hide_timer_c.borrow_mut() = Some(id);
    }
}

fn pill_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.arc(
        x + w - r,
        y + r,
        r,
        3.0 * std::f64::consts::FRAC_PI_2,
        0.0,
    );
    cr.arc(
        x + w - r,
        y + h - r,
        r,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.close_path();
}