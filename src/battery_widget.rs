use crate::battery_listener::BatteryInfo;
use gtk4::prelude::*;
use gtk4::{Box as GBox, Label, Orientation, Overlay, ProgressBar};

pub fn build_battery_widget() -> (GBox, impl Fn(BatteryInfo)) {
    let container = GBox::new(Orientation::Horizontal, 4);
    container.add_css_class("battery-box");

    let overlay = Overlay::new();

    let bar = ProgressBar::new();
    bar.add_css_class("battery");
    bar.set_valign(gtk4::Align::Center);
    bar.set_fraction(0.0);

    let pct_label = Label::new(Some("-"));
    pct_label.add_css_class("battery-percent");
    pct_label.set_halign(gtk4::Align::Center);
    pct_label.set_valign(gtk4::Align::Center);
    pct_label.set_can_target(false);

    overlay.set_child(Some(&bar));
    overlay.add_overlay(&pct_label);

    container.append(&overlay);

    let bar_c = bar.clone();
    let pct_label_c = pct_label.clone();

    let updater = move |info: BatteryInfo| {
        let fraction = (info.capacity as f64 / 100.0).clamp(0.0, 1.0);
        bar_c.set_fraction(fraction);
        pct_label_c.set_label(&format!("{}", info.capacity));

        bar_c.remove_css_class("charging");
        bar_c.remove_css_class("critical");
        bar_c.remove_css_class("low");

        if info.status == "Charging" || info.status == "Full" {
            bar_c.add_css_class("charging");
        } else if info.capacity <= 15 {
            bar_c.add_css_class("critical");
        } else if info.capacity <= 30 {
            bar_c.add_css_class("low");
        }
    };

    (container, updater)
}
