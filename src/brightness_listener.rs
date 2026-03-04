use async_channel;
use evdev::{Device, EventSummary, KeyCode};
use std::fs;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct BrightnessInfo {
    pub value: u32,
    pub max: u32,
}

impl BrightnessInfo {
    pub fn fraction(&self) -> f32 {
        if self.max == 0 {
            return 0.0;
        }
        (self.value as f32 / self.max as f32).clamp(0.0, 1.0)
    }
}

fn find_backlight_path() -> Option<String> {
    let entries = fs::read_dir("/sys/class/backlight").ok()?;
    for entry in entries.flatten() {
        return Some(entry.path().to_string_lossy().to_string());
    }
    None
}

fn read_brightness_info() -> Option<BrightnessInfo> {
    let path = find_backlight_path()?;
    let value = fs::read_to_string(format!("{}/brightness", path))
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    let max = fs::read_to_string(format!("{}/max_brightness", path))
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    Some(BrightnessInfo { value, max })
}

const BRIGHTNESS_KEYS: [KeyCode; 2] = [
    KeyCode::KEY_BRIGHTNESSUP,
    KeyCode::KEY_BRIGHTNESSDOWN,
];

fn find_brightness_devices() -> Vec<Device> {
    evdev::enumerate()
        .filter_map(|(_path, device)| {
            let keys = device.supported_keys()?;
            if BRIGHTNESS_KEYS.iter().any(|k| keys.contains(*k)) {
                Some(device)
            } else {
                None
            }
        })
        .collect()
}

pub fn start_brightness_listener() -> async_channel::Receiver<BrightnessInfo> {
    let (sender, receiver) = async_channel::unbounded();

    let devices = find_brightness_devices();

    if devices.is_empty() {
        eprintln!(
            "[brightness] No input devices found with brightness keys. \
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
                            // Fire on key-down (1) AND repeat (2) so the HUD
                            // stays visible while the user holds the key
                            EventSummary::Key(_, key, value)
                                if BRIGHTNESS_KEYS.contains(&key) && value != 0 =>
                            {
                                // Give the kernel/userspace handler a moment
                                // to write the new brightness value
                                std::thread::sleep(Duration::from_millis(30));
                                if let Some(info) = read_brightness_info() {
                                    let _ = sender.send_blocking(info);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[brightness] evdev read error: {e}");
                    break;
                }
            }
        });
    }

    receiver
}