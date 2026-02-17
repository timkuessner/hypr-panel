use async_channel;
use std::fs;
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct BatteryInfo {
    pub capacity: u8,
    pub status: String,
}

fn read_battery_info() -> Option<BatteryInfo> {
    let bat_path = "/sys/class/power_supply/BAT0";
    let capacity = fs::read_to_string(format!("{}/capacity", bat_path))
        .ok()?
        .trim()
        .parse::<u8>()
        .ok()?;
    let status = fs::read_to_string(format!("{}/status", bat_path))
        .ok()?
        .trim()
        .to_string();
    Some(BatteryInfo { capacity, status })
}

fn send_if_changed(
    sender: &async_channel::Sender<BatteryInfo>,
    last: &Arc<Mutex<Option<BatteryInfo>>>,
) -> bool {
    if let Some(info) = read_battery_info() {
        let mut guard = last.lock().unwrap();
        if guard.as_ref() != Some(&info) {
            let _ = sender.send_blocking(info.clone());
            *guard = Some(info);
            return true;
        }
    }
    false
}

pub fn start_battery_listener() -> async_channel::Receiver<BatteryInfo> {
    let (sender, receiver) = async_channel::unbounded();
    let last_state: Arc<Mutex<Option<BatteryInfo>>> = Arc::new(Mutex::new(None));

    send_if_changed(&sender, &last_state);

    let pending_since: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));

    {
        let pending_since = Arc::clone(&pending_since);
        let sender = sender.clone();
        let last_state = Arc::clone(&last_state);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(100));
                let trigger = {
                    let guard = pending_since.lock().unwrap();
                    guard.map_or(false, |t| t.elapsed() >= Duration::from_secs(1))
                };
                if trigger {
                    {
                        let mut guard = pending_since.lock().unwrap();
                        *guard = None;
                    }
                    send_if_changed(&sender, &last_state);
                }
            }
        });
    }

    {
        let pending_since = Arc::clone(&pending_since);
        std::thread::spawn(move || {
            let socket = match udev::MonitorBuilder::new()
                .and_then(|b| b.match_subsystem("power_supply"))
                .and_then(|b| b.listen())
            {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[battery] Failed to create udev monitor: {}", e);
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
                    eprintln!("[battery] poll() error");
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
                if pollfd.revents & libc::POLLIN != 0 {
                    for event in socket.iter() {
                        if event.event_type() == udev::EventType::Change
                            && event.sysname() == "BAT0"
                        {
                            let mut guard = pending_since.lock().unwrap();
                            if guard.is_none() {
                                *guard = Some(Instant::now());
                            }
                        }
                    }
                }
            }
        });
    }

    {
        let sender = sender.clone();
        let last_state = Arc::clone(&last_state);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(30));
            send_if_changed(&sender, &last_state);
        });
    }

    receiver
}