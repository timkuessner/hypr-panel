use async_channel;
use std::fs;
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

pub fn start_brightness_listener() -> async_channel::Receiver<BrightnessInfo> {
    let (sender, receiver) = async_channel::unbounded();
    let last_state: Arc<Mutex<Option<BrightnessInfo>>> = Arc::new(Mutex::new(None));
    let pending_since: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));

    {
        let pending_since = Arc::clone(&pending_since);
        let sender = sender.clone();
        let last_state = Arc::clone(&last_state);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(50));
            let trigger = {
                let guard = pending_since.lock().unwrap();
                guard.map_or(false, |t| t.elapsed() >= Duration::from_millis(200))
            };
            if trigger {
                {
                    let mut guard = pending_since.lock().unwrap();
                    *guard = None;
                }
                if let Some(info) = read_brightness_info() {
                    let mut guard = last_state.lock().unwrap();
                    if guard.as_ref() != Some(&info) {
                        let _ = sender.send_blocking(info.clone());
                        *guard = Some(info);
                    }
                }
            }
        });
    }

    {
        let pending_since = Arc::clone(&pending_since);
        std::thread::spawn(move || {
            let socket = match udev::MonitorBuilder::new()
                .and_then(|b| b.match_subsystem("backlight"))
                .and_then(|b| b.listen())
            {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[brightness] Failed to create udev monitor: {}", e);
                    return;
                }
            };
            let raw_fd = socket.as_raw_fd();
            loop {
                let mut pollfd = libc::pollfd {
                    fd: raw_fd,
                    events: libc::POLLIN,
                    revents: 0,
                };
                let ret = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, -1) };
                if ret < 0 {
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
                if pollfd.revents & libc::POLLIN != 0 {
                    for _ in socket.iter() {
                        let mut guard = pending_since.lock().unwrap();
                        if guard.is_none() {
                            *guard = Some(Instant::now());
                        }
                    }
                }
            }
        });
    }

    receiver
}