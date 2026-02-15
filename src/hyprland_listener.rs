use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use async_channel;

pub fn start_listener() -> async_channel::Receiver<String> {
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
                                let parts: Vec<&str> = line.strip_prefix("activewindow>>")
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