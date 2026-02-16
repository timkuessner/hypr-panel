use async_channel;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::process::Command;

pub fn start_active_window_listener() -> async_channel::Receiver<String> {
    let (sender, receiver) = async_channel::unbounded();

    std::thread::spawn(move || {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR not set");
        let hypr_instance = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .expect("HYPRLAND_INSTANCE_SIGNATURE not set");
        let socket_path = format!("{}/hypr/{}/.socket2.sock", runtime_dir, hypr_instance);

        loop {
            match UnixStream::connect(&socket_path) {
                Ok(stream) => {
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if line.starts_with("activewindow>>") {
                                let parts: Vec<&str> = line
                                    .strip_prefix("activewindow>>")
                                    .unwrap()
                                    .split(',')
                                    .collect();
                                if parts.len() >= 2 {
                                    let class = parts[0];
                                    let class_str = if class.is_empty() {
                                        "Desktop".to_string()
                                    } else {
                                        class.to_string()
                                    };
                                    let _ = sender.send_blocking(class_str);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to Hyprland socket: {}", e);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    });

    receiver
}

fn get_workspace_info() -> (i32, i32) {
    let output = Command::new("hyprctl")
        .args(&["workspaces", "-j"])
        .output();
    
    let active_output = Command::new("hyprctl")
        .args(&["activeworkspace", "-j"])
        .output();

    let mut active_workspace = 1;
    let mut max_workspace = 5;

    if let Ok(output) = active_output {
        if let Ok(json_str) = String::from_utf8(output.stdout) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(id) = json["id"].as_i64() {
                    active_workspace = id as i32;
                }
            }
        }
    }

    if let Ok(output) = output {
        if let Ok(json_str) = String::from_utf8(output.stdout) {
            if let Ok(workspaces) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                max_workspace = workspaces
                    .iter()
                    .filter_map(|ws| ws["id"].as_i64())
                    .map(|id| id as i32)
                    .max()
                    .unwrap_or(5)
                    .max(5);
            }
        }
    }

    (active_workspace, max_workspace)
}

pub fn start_workspace_listener() -> async_channel::Receiver<(i32, i32)> {
    let (sender, receiver) = async_channel::unbounded();

    let initial_state = get_workspace_info();
    let _ = sender.send_blocking(initial_state);

    std::thread::spawn(move || {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR not set");
        let hypr_instance = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .expect("HYPRLAND_INSTANCE_SIGNATURE not set");
        let socket_path = format!("{}/hypr/{}/.socket2.sock", runtime_dir, hypr_instance);

        loop {
            match UnixStream::connect(&socket_path) {
                Ok(stream) => {
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if line.starts_with("workspace>>")
                                || line.starts_with("destroyworkspace>>")
                                || line.starts_with("createworkspace>>")
                            {
                                let workspace_info = get_workspace_info();
                                let _ = sender.send_blocking(workspace_info);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to Hyprland socket: {}", e);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    });

    receiver
}