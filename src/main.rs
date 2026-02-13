use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box, Label, Orientation};
use gtk_layer_shell::{Edge, Layer, LayerShell};

fn main() {
    let app = Application::new(
        Some("com.example.hypr-panel"),
        Default::default(),
    );

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::new(app);
    window.set_default_size(1920, 30);

    // Initialize layer shell
    window.init_layer_shell();
    
    // Configure as a panel
    window.set_layer(Layer::Top);
    window.auto_exclusive_zone_enable();
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);

    // Create panel content
    let container = Box::new(Orientation::Horizontal, 10);
    container.set_margin_start(10);
    container.set_margin_end(10);
    
    // Left side - workspaces (placeholder)
    let left = Label::new(Some("code-oss"));
    container.pack_start(&left, false, false, 0);
    
    // Center - window title (placeholder)
    let center = Label::new(Some("1 2 3 4 5"));
    container.set_center_widget(Some(&center));
    
    // Right side - system info (placeholder)
    let right = Label::new(Some("network | battery | Fri Feb 13 15:20"));
    container.pack_end(&right, false, false, 0);

    window.add(&container);
    window.show_all();
}