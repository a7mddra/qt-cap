use std::thread;
use std::time::{Duration, Instant};
use std::process::Command;
use anyhow::Result;

pub struct DisplayMonitor {
    last_count: i32,
    last_check: Instant,
}

impl DisplayMonitor {
    pub fn new() -> Self {
        Self {
            last_count: Self::get_monitor_count(),
            last_check: Instant::now(),
        }
    }

    /// Returns true if topology changed
    pub fn check(&mut self) -> bool {
        // Rate limit checks to 500ms
        if self.last_check.elapsed() < Duration::from_millis(500) {
            return false;
        }
        self.last_check = Instant::now();

        let current = Self::get_monitor_count();
        if current != self.last_count {
            // Debounce: Wait 800ms to confirm it wasn't a glitch
            thread::sleep(Duration::from_millis(800));
            let confirmed = Self::get_monitor_count();
            
            if confirmed != self.last_count {
                self.last_count = confirmed;
                return true;
            }
        }
        false
    }

    #[cfg(target_os = "macos")]
    fn get_monitor_count() -> i32 {
        // Shelling out to system_profiler is slow, checking a folder is safer?
        // Actually, on macOS, checking connected displays via localized script is brittle.
        // A simple fallback:
        use std::process::Command;
        let out = Command::new("system_profiler").arg("SPDisplaysDataType").output();
        if let Ok(o) = out {
             String::from_utf8_lossy(&o.stdout).matches("Resolution:").count() as i32
        } else { 1 }
    }

    #[cfg(target_os = "linux")]
    fn get_monitor_count() -> i32 {
        // Universal Linux (SysFS) - Extremely fast, no deps
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            let count = entries.filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if !name.starts_with("card") || !name.contains("-") { return false; }
                    
                    let status_path = e.path().join("status");
                    std::fs::read_to_string(status_path)
                        .map(|s| s.trim() == "connected")
                        .unwrap_or(false)
                })
                .count();
            if count > 0 { return count as i32; }
        }
        1 // Fallback
    }

    #[cfg(target_os = "windows")]
    fn get_monitor_count() -> i32 {
        // PowerShell fallback if winapi crate is heavy
        1 // Placeholder for WinAPI impl
    }
}
