use async_channel;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq)]
pub struct BluetoothInfo {
    pub enabled: bool,
    pub connected_devices: Vec<String>,
}

fn get_bluetooth_info() -> BluetoothInfo {
    let show = Command::new("bluetoothctl")
        .arg("show")
        .output();

    let enabled = match show {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|l| l.trim() == "Powered: yes"),
        Err(_) => false,
    };

    if !enabled {
        return BluetoothInfo {
            enabled: false,
            connected_devices: vec![],
        };
    }

    let devices = Command::new("bluetoothctl")
        .args(&["devices", "Connected"])
        .output();

    let connected_devices = match devices {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut parts = l.splitn(3, ' ');
                parts.next(); // "Device"
                parts.next(); // MAC
                parts.next().map(|name| name.to_string())
            })
            .collect(),
        Err(_) => vec![],
    };

    BluetoothInfo {
        enabled,
        connected_devices,
    }
}

pub fn start_bluetooth_listener() -> async_channel::Receiver<BluetoothInfo> {
    let (sender, receiver) = async_channel::unbounded();

    let _ = sender.send_blocking(get_bluetooth_info());

    std::thread::spawn(move || {
        let mut child = match Command::new("bluetoothctl")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[bluetooth] Failed to spawn bluetoothctl: {}", e);
                return;
            }
        };

        let mut stdin = child.stdin.take().expect("bluetoothctl has no stdin");
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                if stdin.write_all(b"\n").is_err() {
                    break;
                }
            }
        });

        let stdout = child.stdout.take().expect("bluetoothctl has no stdout");
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("[CHG]")
                    && (line.contains("Connected") || line.contains("Powered"))
                {
                    let _ = sender.send_blocking(get_bluetooth_info());
                }
            }
        }

        let _ = child.wait();
    });

    receiver
}