use async_channel;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq)]
pub struct WifiInfo {
    pub connected: bool,
    pub ssid: Option<String>,
    pub signal: Option<u8>,
}

fn get_wifi_info() -> WifiInfo {
    let output = Command::new("nmcli")
        .args(&["-t", "-f", "ACTIVE,SSID,SIGNAL", "dev", "wifi"])
        .output();

    if let Ok(output) = output {
        if let Ok(text) = String::from_utf8(output.stdout) {
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() >= 2 && parts[0] == "yes" {
                    let ssid = if parts[1].is_empty() {
                        None
                    } else {
                        Some(parts[1].to_string())
                    };
                    let signal = parts.get(2).and_then(|s| s.parse::<u8>().ok());
                    return WifiInfo {
                        connected: true,
                        ssid,
                        signal,
                    };
                }
            }
        }
    }

    WifiInfo {
        connected: false,
        ssid: None,
        signal: None,
    }
}

pub fn start_wifi_listener() -> async_channel::Receiver<WifiInfo> {
    let (sender, receiver) = async_channel::unbounded();

    let _ = sender.send_blocking(get_wifi_info());

    std::thread::spawn(move || {
        let mut child = match Command::new("nmcli")
            .arg("monitor")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[wifi] Failed to spawn nmcli monitor: {}", e);
                return;
            }
        };

        let stdout = child.stdout.take().expect("nmcli monitor has no stdout");
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("connected") || line.contains("disconnected") {
                    let _ = sender.send_blocking(get_wifi_info());
                }
            }
        }

        let _ = child.wait();
    });

    receiver
}