mod battery_listener;
mod bluetooth_listener;
mod hyprland_listener;
mod wifi_listener;

mod battery_widget;

use chrono::{Local, Timelike};
use gtk4::gdk::Display;
use gtk4::{Application, ApplicationWindow, CenterBox, CssProvider, Label};
use gtk4::{glib, prelude::*};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::fs;

fn main() {
    let app = Application::builder()
        .application_id("com.example.hypr-panel")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(1920)
        .default_height(30)
        .decorated(false)
        .build();

    let css = fs::read_to_string("style.css").expect("CSS file not found");
    let provider = CssProvider::new();
    provider.load_from_data(&css);

    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    window.init_layer_shell();
    window.set_namespace(Some("hypr-panel"));
    window.set_layer(Layer::Top);
    window.auto_exclusive_zone_enable();
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);

    let container = CenterBox::new();
    container.set_margin_start(10);
    container.set_margin_end(10);

    let left = Label::builder().label("Desktop").build();
    container.set_start_widget(Some(&left));

    let active_window_receiver = hyprland_listener::start_active_window_listener();
    let left_clone = left.clone();
    glib::spawn_future_local(async move {
        while let Ok(class) = active_window_receiver.recv().await {
            left_clone.set_label(&class);
        }
    });

    let center = Label::builder().label("1 2 3 4 5").use_markup(true).build();
    container.set_center_widget(Some(&center));

    let workspace_receiver = hyprland_listener::start_workspace_listener();
    let center_clone = center.clone();
    glib::spawn_future_local(async move {
        while let Ok((active_ws, max_ws)) = workspace_receiver.recv().await {
            let workspace_count = max_ws.max(5);
            let mut workspace_text = String::new();

            for i in 1..=workspace_count {
                if i > 1 {
                    workspace_text.push_str(" ");
                }

                let distance = (i - active_ws).abs();

                let size_pt = match distance {
                    0 => 11,
                    _ => 10,
                };

                let size_pango = size_pt * 1024;

                if distance == 0 {
                    workspace_text.push_str(&format!(
                        "<span size=\"{}\" weight=\"bold\">{}</span>",
                        size_pango, i
                    ));
                } else {
                    workspace_text.push_str(&format!("<span size=\"{}\">{}</span>", size_pango, i));
                }
            }

            center_clone.set_markup(&workspace_text);
        }
    });

    let right_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

    let wifi_label = Label::builder().label("...").build();
    let bt_label = Label::builder().label("...").build();
    let (battery_widget, battery_updater) = battery_widget::build_battery_widget();
    let datetime_label = Label::builder()
        .label(&format!("{}", Local::now().format("%a %b %d %H:%M")))
        .build();

    let wifi_receiver = wifi_listener::start_wifi_listener();
    let wifi_label_clone = wifi_label.clone();
    glib::spawn_future_local(async move {
        while let Ok(info) = wifi_receiver.recv().await {
            let text = if info.connected {
                match (info.ssid.as_deref(), info.signal) {
                    (Some(ssid), Some(sig)) => format!("  {} {}%", ssid, sig),
                    (Some(ssid), None) => format!("  {}", ssid),
                    _ => "Connected".to_string(),
                }
            } else {
                "Disconnected".to_string()
            };
            wifi_label_clone.set_label(&text);
        }
    });

    let bt_receiver = bluetooth_listener::start_bluetooth_listener();
    let bt_label_clone = bt_label.clone();
    glib::spawn_future_local(async move {
        while let Ok(info) = bt_receiver.recv().await {
            let text: String = if !info.enabled {
                "Off".to_string()
            } else if info.connected_devices.is_empty() {
                "On".to_string()
            } else {
                format!("{}", info.connected_devices.join(", "))
            };
            bt_label_clone.set_label(&text);
        }
    });

    right_box.append(&wifi_label);
    right_box.append(&bt_label);
    right_box.append(&battery_widget);
    right_box.append(&datetime_label);

    container.set_end_widget(Some(&right_box));

    let now: chrono::DateTime<_> = Local::now();
    let seconds_until_next_minute = 60 - now.second();

    let datetime_label_clone = datetime_label.clone();
    glib::timeout_add_seconds_local(seconds_until_next_minute, move || {
        datetime_label_clone.set_label(&format!("{}", Local::now().format("%a %b %d %H:%M")));

        let datetime_label_clone2 = datetime_label_clone.clone();
        glib::timeout_add_seconds_local(60, move || {
            datetime_label_clone2.set_label(&format!("{}", Local::now().format("%a %b %d %H:%M")));
            glib::ControlFlow::Continue
        });

        glib::ControlFlow::Break
    });

    let battery_receiver = battery_listener::start_battery_listener();
    glib::spawn_future_local(async move {
        while let Ok(info) = battery_receiver.recv().await {
            battery_updater(info);
        }
    });

    window.set_child(Some(&container));
    window.present();
}
