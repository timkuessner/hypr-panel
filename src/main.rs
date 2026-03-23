mod battery_listener;
mod bluetooth_listener;
mod brightness_listener;
mod hud_overlay;
mod hyprland_listener;
mod volume_listener;
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

    let cap_brightness_down =
        hud_overlay::build_key_cap(app, hud_overlay::X_BRIGHTNESS_DOWN);
    let cap_brightness_up =
        hud_overlay::build_key_cap(app, hud_overlay::X_BRIGHTNESS_UP);
    let cap_volume_mute =
        hud_overlay::build_key_cap(app, hud_overlay::X_VOLUME_MUTE);
    let cap_volume_down =
        hud_overlay::build_key_cap(app, hud_overlay::X_VOLUME_DOWN);
    let cap_volume_up =
        hud_overlay::build_key_cap(app, hud_overlay::X_VOLUME_UP);

    let bar_brightness = hud_overlay::build_level_bar(
        app,
        hud_overlay::X_BRIGHTNESS_BAR,
        hud_overlay::W_BRIGHTNESS_BAR,
    );
    let bar_volume = hud_overlay::build_level_bar(
        app,
        hud_overlay::X_VOLUME_BAR,
        hud_overlay::W_VOLUME_BAR,
    );

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(1920)
        .default_height(25)
        .decorated(false)
        .build();

    window.init_layer_shell();
    window.set_namespace(Some("hypr-panel"));
    window.set_layer(Layer::Top);
    window.auto_exclusive_zone_enable();
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);

    let container = CenterBox::new();
    container.set_margin_start(7);
    container.set_margin_end(7);

    let left_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let logo = gtk4::Image::from_file("logo.svg");
    logo.set_pixel_size(16);
 
    let active_window_label = Label::builder().label("Desktop").build();
 
    left_box.append(&logo);
    left_box.append(&active_window_label);

    let center = Label::builder().label("1 2 3 4 5").use_markup(true).build();

    let right_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let wifi_label = Label::builder().label("...").build();
    let bt_label = Label::builder().label("...").build();
    let (battery_widget, battery_updater) = battery_widget::build_battery_widget();
    let datetime_label = Label::builder()
        .label(&format!("{}", Local::now().format("%a %b %d %H:%M")))
        .build();

    right_box.append(&battery_widget);
    right_box.append(&wifi_label);
    right_box.append(&bt_label);
    right_box.append(&datetime_label);

    container.set_start_widget(Some(&left_box));
    container.set_center_widget(Some(&center));
    container.set_end_widget(Some(&right_box));

    window.set_child(Some(&container));
    window.present();

    let active_window_receiver = hyprland_listener::start_active_window_listener();
    let active_window_label_clone = active_window_label.clone();
    glib::spawn_future_local(async move {
        while let Ok(class) = active_window_receiver.recv().await {
            active_window_label_clone.set_label(&class);
        }
    });

    let workspace_receiver = hyprland_listener::start_workspace_listener();
    let center_clone = center.clone();
    glib::spawn_future_local(async move {
        while let Ok((active_ws, max_ws)) = workspace_receiver.recv().await {
            let workspace_count = max_ws.max(5);
            let mut workspace_text = String::new();

            for i in 1..=workspace_count {
                if i > 1 {
                    workspace_text.push(' ');
                }
                let distance = (i - active_ws).abs();
                let size_pango = match distance {
                    0 => 11,
                    _ => 10,
                } * 1024;

                if distance == 0 {
                    workspace_text.push_str(&format!(
                        "<span size=\"{}\" weight=\"bold\">{}</span>",
                        size_pango, i
                    ));
                } else {
                    workspace_text
                        .push_str(&format!("<span size=\"{}\">{}</span>", size_pango, i));
                }
            }

            center_clone.set_markup(&workspace_text);
        }
    });

    let wifi_receiver = wifi_listener::start_wifi_listener();
    let wifi_label_clone = wifi_label.clone();
    glib::spawn_future_local(async move {
        while let Ok(info) = wifi_receiver.recv().await {
            let text = if info.connected {
                match info.signal {
                    Some(s) if s >= 75 => "󰤨",
                    Some(s) if s >= 50 => "󰤥",
                    Some(s) if s >= 25 => "󰤢",
                    Some(_) => "󰤟",
                    None => "󰤨",
                }
            } else {
                "󰤭"
            };
            wifi_label_clone.set_label(text);
        }
    });

    let bt_receiver = bluetooth_listener::start_bluetooth_listener();
    let bt_label_clone = bt_label.clone();
    glib::spawn_future_local(async move {
        while let Ok(info) = bt_receiver.recv().await {
            if !info.enabled {
                bt_label_clone.set_visible(false);
            } else {
                let text = if info.connected_devices.is_empty() {
                    "󰂯".to_string()
                } else {
                    format!("󰂱 {}", info.connected_devices.join(", "))
                };
                bt_label_clone.set_label(&text);
                bt_label_clone.set_visible(true);
            }
        }
    });

    let now: chrono::DateTime<_> = Local::now();
    let seconds_until_next_minute = 60 - now.second();

    let datetime_label_clone = datetime_label.clone();
    glib::timeout_add_seconds_local(seconds_until_next_minute, move || {
        datetime_label_clone.set_label(&format!("{}", Local::now().format("%a %b %d %H:%M")));

        let datetime_label_clone2 = datetime_label_clone.clone();
        glib::timeout_add_seconds_local(60, move || {
            datetime_label_clone2
                .set_label(&format!("{}", Local::now().format("%a %b %d %H:%M")));
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

    let volume_receiver = volume_listener::start_volume_listener();
    glib::spawn_future_local(async move {
        use volume_listener::{KeyAction, VolumeKey};
        while let Ok(event) = volume_receiver.recv().await {
            match (&event.key, &event.action) {
                (VolumeKey::Up, KeyAction::Press | KeyAction::Repeat) => {
                    cap_volume_up();
                    if let Some(info) = &event.info {
                        bar_volume(info.volume.clamp(0.0, 1.0));
                    }
                }
                (VolumeKey::Down, KeyAction::Press | KeyAction::Repeat) => {
                    cap_volume_down();
                    if let Some(info) = &event.info {
                        bar_volume(info.volume.clamp(0.0, 1.0));
                    }
                }
                (VolumeKey::Mute, KeyAction::Press | KeyAction::Repeat) => {
                    cap_volume_mute();
                }
                _ => {}
            }
        }
    });

    let brightness_receiver = brightness_listener::start_brightness_listener();
    glib::spawn_future_local(async move {
        use brightness_listener::{BrightnessKey, KeyAction};
        while let Ok(event) = brightness_receiver.recv().await {
            match (&event.key, &event.action) {
                (BrightnessKey::Up, KeyAction::Press | KeyAction::Repeat) => {
                    cap_brightness_up();
                    if let Some(info) = &event.info { bar_brightness(info.fraction()); }
                }
                (BrightnessKey::Down, KeyAction::Press | KeyAction::Repeat) => {
                    cap_brightness_down();
                    if let Some(info) = &event.info { bar_brightness(info.fraction()); }
                }
                _ => {}
            }
        }
    });
}