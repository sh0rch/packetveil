//! Minimal systemd `sd_notify` client (no external crate).

/// Send a state notification to the service manager. No-op when not running
/// under systemd (`NOTIFY_SOCKET` unset) or on non-Unix platforms.
pub fn sd_notify(msg: &str) {
    #[cfg(target_family = "unix")]
    {
        if let Ok(path) = std::env::var("NOTIFY_SOCKET") {
            use std::os::unix::net::UnixDatagram;
            let sock = match UnixDatagram::unbound() {
                Ok(s) => s,
                Err(_) => return,
            };
            let _ = sock.send_to(msg.as_bytes(), &path);
        }
    }
}

/// Spawn a background thread that pings the systemd watchdog for the lifetime
/// of the process.
///
/// Period is `min(500 ms, WATCHDOG_USEC / 2)` - the same cadence as the native
/// datapath's main loop, and never slower than half the configured
/// `WatchdogSec`. A `WATCHDOG_USEC` of 0 (watchdog disabled) falls back to the
/// 500 ms default; `sd_notify` is a no-op without `NOTIFY_SOCKET` anyway.
pub fn spawn_watchdog() {
    let period = std::env::var("WATCHDOG_USEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|us| *us > 0)
        .map(|us| std::time::Duration::from_micros(us / 2))
        .unwrap_or(std::time::Duration::from_millis(500))
        .min(std::time::Duration::from_millis(500));
    let r = std::thread::Builder::new()
        .name("gutd-watchdog".to_string())
        .spawn(move || loop {
            sd_notify("WATCHDOG=1");
            std::thread::sleep(period);
        });
    if let Err(e) = r {
        eprintln!("gutd: failed to spawn watchdog thread: {e}");
    }
}
