use async_channel;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq)]
pub struct VolumeInfo {
    pub volume: f32,
    pub muted: bool,
}

fn get_volume_info() -> Option<VolumeInfo> {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let muted = text.contains("[MUTED]");
    let volume: f32 = text.split_whitespace().nth(1)?.parse().ok()?;
    Some(VolumeInfo {
        volume: volume.clamp(0.0, 1.5),
        muted,
    })
}

pub fn start_volume_listener() -> async_channel::Receiver<VolumeInfo> {
    let (sender, receiver) = async_channel::unbounded();

    std::thread::spawn(move || {
        let mut child = match Command::new("pactl")
            .arg("subscribe")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[volume] Failed to spawn pactl subscribe: {}", e);
                return;
            }
        };

        let stdout = child.stdout.take().expect("pactl subscribe has no stdout");
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("'change' on sink") || line.contains("'change' on source") {
                    if line.contains("on sink") {
                        if let Some(info) = get_volume_info() {
                            let _ = sender.send_blocking(info);
                        }
                    }
                }
            }
        }

        let _ = child.wait();
    });

    receiver
}