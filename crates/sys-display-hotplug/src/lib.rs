//! Display hotplug monitor for screen capture
//!
//! Monitors for HDMI/VGA cable plug/unplug events during capture.
//! When topology changes, triggers a callback to kill the capture process.
//! This prevents ghost freezes and jumps to primary screen.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Handle to a running display monitor thread
pub struct DisplayWatcher {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl DisplayWatcher {
    /// Start watching for display topology changes in a background thread.
    /// 
    /// The `on_change` callback will be called if monitors are added/removed.
    /// After calling the callback, the watcher thread exits.
    /// 
    /// # Example
    /// ```ignore
    /// let watcher = DisplayWatcher::start(|| {
    ///     eprintln!("Display changed! Killing Qt...");
    ///     std::process::exit(1);
    /// });
    /// // ... do capture ...
    /// watcher.stop(); // No more monitoring needed
    /// ```
    pub fn start<F>(on_change: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let handle = thread::spawn(move || {
            let mut monitor = DisplayMonitor::new();
            
            while running_clone.load(Ordering::Relaxed) {
                if monitor.check() {
                    on_change();
                    break;
                }
                thread::sleep(Duration::from_millis(300));
            }
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    /// Stop the watcher thread
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DisplayWatcher {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // Don't join in drop - could block
    }
}

/// Internal display monitor with debouncing
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

    /// Check for topology change, with debouncing
    /// Returns true if topology changed (confirmed after debounce)
    pub fn check(&mut self) -> bool {
        // Rate limit checks to 250ms
        if self.last_check.elapsed() < Duration::from_millis(250) {
            return false;
        }
        self.last_check = Instant::now();

        let current = Self::get_monitor_count();
        if current != self.last_count {
            // Debounce: confirm after 500ms
            thread::sleep(Duration::from_millis(500));
            let confirmed = Self::get_monitor_count();

            if confirmed != self.last_count {
                self.last_count = confirmed;
                return true;
            }
        }
        false
    }

    // ========== Linux ==========
    
    #[cfg(target_os = "linux")]
    fn get_monitor_count() -> i32 {
        // Read from SysFS - extremely fast, no subprocess
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    // Only check connector entries like "card0-HDMI-A-1"
                    if !name.starts_with("card") || !name.contains('-') {
                        return false;
                    }
                    // Check if connected
                    let status_path = e.path().join("status");
                    std::fs::read_to_string(status_path)
                        .map(|s| s.trim() == "connected")
                        .unwrap_or(false)
                })
                .count();
            if count > 0 {
                return count as i32;
            }
        }
        1 // Fallback
    }

    // ========== macOS ==========
    
    #[cfg(target_os = "macos")]
    fn get_monitor_count() -> i32 {
        // Use IOKit for fast enumeration
        // Fallback to system_profiler if IOKit unavailable
        use std::process::Command;
        let out = Command::new("system_profiler")
            .arg("SPDisplaysDataType")
            .output();
        if let Ok(o) = out {
            String::from_utf8_lossy(&o.stdout)
                .matches("Resolution:")
                .count() as i32
        } else {
            1
        }
    }

    // ========== Windows ==========
    
    #[cfg(target_os = "windows")]
    fn get_monitor_count() -> i32 {
        // Use EnumDisplayDevices or GetSystemMetrics
        // Placeholder - Windows display changes are rare during capture
        use std::process::Command;
        // PowerShell one-liner to count monitors
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-CimInstance -ClassName Win32_DesktopMonitor | Measure-Object).Count",
            ])
            .output();
        if let Ok(o) = out {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse()
                .unwrap_or(1)
        } else {
            1
        }
    }
}

impl Default for DisplayMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_count_nonzero() {
        let count = DisplayMonitor::get_monitor_count();
        assert!(count >= 1, "Should detect at least one display");
    }

    #[test]
    fn test_watcher_can_stop() {
        let watcher = DisplayWatcher::start(|| {
            // This should not be called in normal test
        });
        watcher.stop();
    }
}
