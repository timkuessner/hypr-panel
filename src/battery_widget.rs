use crate::battery_listener::BatteryInfo;
use gtk4::prelude::*;
use gtk4::{Box as GBox, DrawingArea, Orientation};
use std::cell::Cell;
use std::rc::Rc;

const BODY_W: f64 = 22.0;
const BODY_H: f64 = 12.0;
const NOB_W: f64 = 2.0;
const NOB_H: f64 = 6.0;
const NOB_GAP: f64 = 1.0;
const TOTAL_W: f64 = BODY_W + NOB_GAP + NOB_W;
const RADIUS: f64 = 8.0;

pub fn build_battery_widget() -> (GBox, impl Fn(BatteryInfo)) {
    let container = GBox::new(Orientation::Horizontal, 0);
    container.add_css_class("battery-box");
    container.set_valign(gtk4::Align::Center);

    let capacity: Rc<Cell<u8>> = Rc::new(Cell::new(100));
    let is_charging: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let is_critical: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let is_low: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    let area = DrawingArea::new();
    area.set_content_width(TOTAL_W as i32);
    area.set_content_height(BODY_H as i32);

    {
        let capacity = capacity.clone();
        let is_charging = is_charging.clone();
        let is_critical = is_critical.clone();
        let is_low = is_low.clone();

        area.set_draw_func(move |_area, cr, _width, _height| {
            draw_battery(
                cr,
                capacity.get(),
                is_charging.get(),
                is_critical.get(),
                is_low.get(),
            );
        });
    }

    container.append(&area);

    let area_c = area.clone();
    let capacity_c = capacity.clone();
    let is_charging_c = is_charging.clone();
    let is_critical_c = is_critical.clone();
    let is_low_c = is_low.clone();

    let updater = move |info: BatteryInfo| {
        let charging = info.status == "Charging" || info.status == "Full";
        capacity_c.set(info.capacity);
        is_charging_c.set(charging);
        is_critical_c.set(!charging && info.capacity <= 15);
        is_low_c.set(!charging && info.capacity > 15 && info.capacity <= 30);
        area_c.queue_draw();
    };

    (container, updater)
}

fn squircle(cr: &cairo::Context, w: f64, h: f64, r: f64) {
    let c = r * 0.9091;

    cr.new_sub_path();
    cr.move_to(r, 0.0);
    cr.line_to(w - r, 0.0);
    cr.curve_to(w - r + c, 0.0, w, r - c, w, r);
    cr.line_to(w, h - r);
    cr.curve_to(w, h - r + c, w - r + c, h, w - r, h);
    cr.line_to(r, h);
    cr.curve_to(r - c, h, 0.0, h - r + c, 0.0, h - r);
    cr.line_to(0.0, r);
    cr.curve_to(0.0, r - c, r - c, 0.0, r, 0.0);
    cr.close_path();
}

fn draw_battery(
    cr: &cairo::Context,
    capacity: u8,
    is_charging: bool,
    is_critical: bool,
    is_low: bool,
) {
    let fraction = (capacity as f64 / 100.0).clamp(0.0, 1.0);

    let (fr, fg, fb) = if is_charging {
        (0.392, 0.863, 0.510)
    } else if is_critical {
        (0.941, 0.314, 0.275)
    } else if is_low {
        (0.941, 0.667, 0.196)
    } else {
        (1.0, 1.0, 1.0)
    };
    cr.push_group();

    // Squircle
    squircle(cr, BODY_W, BODY_H, RADIUS);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.5);
    let _ = cr.fill_preserve();
    cr.set_line_width(0.0);
    let _ = cr.stroke();

    // Fill
    if fraction > 0.0 {
        let _ = cr.save();

        squircle(cr, BODY_W, BODY_H, RADIUS);
        let _ = cr.clip();

        let fill_w = BODY_W * fraction;
        cr.rectangle(0.0, 0.0, fill_w, BODY_H);
        cr.set_source_rgb(fr, fg, fb);
        let _ = cr.fill();

        let _ = cr.restore();
    }

    // Percentage text, cut out
    cr.set_operator(cairo::Operator::DestOut);

    let layout = pangocairo::functions::create_layout(cr);
    let mut font_desc = pango::FontDescription::new();
    font_desc.set_family("Inter");
    font_desc.set_weight(pango::Weight::Ultrabold);
    font_desc.set_absolute_size(11.0 * pango::SCALE as f64);
    layout.set_font_description(Some(&font_desc));
    layout.set_text(&format!("{}", capacity));

    let (text_w, text_h) = layout.pixel_size();
    let tx = (BODY_W - text_w as f64) / 2.0;
    let ty = (BODY_H - text_h as f64) / 2.0;

    cr.move_to(tx, ty);
    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    pangocairo::functions::show_layout(cr, &layout);

    cr.set_operator(cairo::Operator::Over);

    let _ = cr.pop_group_to_source();
    let _ = cr.paint();

    // Nob
    let nob_x = BODY_W + NOB_GAP;
    let nob_y = (BODY_H - NOB_H) / 2.0;
    let nob_r = 1.5_f64;

    cr.new_sub_path();
    cr.move_to(nob_x, nob_y);
    cr.line_to(nob_x + NOB_W - nob_r, nob_y);
    cr.arc(
        nob_x + NOB_W - nob_r,
        nob_y + nob_r,
        nob_r,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    cr.line_to(nob_x + NOB_W, nob_y + NOB_H - nob_r);
    cr.arc(
        nob_x + NOB_W - nob_r,
        nob_y + NOB_H - nob_r,
        nob_r,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    cr.line_to(nob_x, nob_y + NOB_H);
    cr.close_path();

    if capacity == 100 {
        cr.set_source_rgb(fr, fg, fb);
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.5);
    }
    let _ = cr.fill();
}
