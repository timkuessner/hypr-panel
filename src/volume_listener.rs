use async_channel;
use evdev::{Device, EventSummary, KeyCode};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct VolumeInfo {
    pub volume: f32,
    pub muted:  bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VolumeKey { Up, Down, Mute }

#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction { Press, Repeat, Release }

#[derive(Debug, Clone)]
pub struct VolumeEvent {
    pub key:    VolumeKey,
    pub action: KeyAction,
    pub info:   Option<VolumeInfo>,
}

fn get_volume_info() -> Option<VolumeInfo> {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let muted  = text.contains("[MUTED]");
    let volume: f32 = text.split_whitespace().nth(1)?.parse().ok()?;
    Some(VolumeInfo { volume: volume.clamp(0.0, 1.5), muted })
}

const VOLUME_KEYS: [KeyCode; 3] = [
    KeyCode::KEY_VOLUMEUP,
    KeyCode::KEY_VOLUMEDOWN,
    KeyCode::KEY_MUTE,
];

fn find_volume_devices() -> Vec<Device> {
    evdev::enumerate()
        .filter_map(|(_path, device)| {
            let keys = device.supported_keys()?;
            if VOLUME_KEYS.iter().any(|k| keys.contains(*k)) { Some(device) } else { None }
        })
        .collect()
}

pub fn start_volume_listener() -> async_channel::Receiver<VolumeEvent> {
    let (sender, receiver) = async_channel::unbounded();

    let devices = find_volume_devices();
    if devices.is_empty() {
        eprintln!(
            "[volume] No input devices found with volume/mute keys. \
             Make sure your user is in the 'input' group: sudo usermod -aG input $USER"
        );
    }

    for mut device in devices {
        let sender = sender.clone();
        std::thread::spawn(move || loop {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        match event.destructure() {
                            EventSummary::Key(_, key, value) if VOLUME_KEYS.contains(&key) => {
                                let volume_key = match key {
                                    KeyCode::KEY_VOLUMEUP   => VolumeKey::Up,
                                    KeyCode::KEY_VOLUMEDOWN => VolumeKey::Down,
                                    _                       => VolumeKey::Mute,
                                };

                                match value {
                                    0 => {
                                        let _ = sender.send_blocking(VolumeEvent {
                                            key:    volume_key,
                                            action: KeyAction::Release,
                                            info:   None,
                                        });
                                    }
                                    2 if volume_key == VolumeKey::Mute => {}
                                    v => {
                                        std::thread::sleep(Duration::from_millis(30));
                                        if let Some(info) = get_volume_info() {
                                            let _ = sender.send_blocking(VolumeEvent {
                                                key:    volume_key,
                                                action: if v == 1 { KeyAction::Press } else { KeyAction::Repeat },
                                                info:   Some(info),
                                            });
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[volume] evdev read error: {e}");
                    break;
                }
            }
        });
    }

    receiver
}