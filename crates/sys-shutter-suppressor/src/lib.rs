//! Shutter sound suppressor for screen capture
//! 
//! Mutes system audio during capture to suppress shutter sounds on:
//! - macOS: CoreGraphics always plays a sound
//! - Linux/Wayland: Portal may play a sound
//! 
//! Windows and X11 are silent by default, so no action needed.

use std::env;
use std::process::Command;
use std::sync::OnceLock;

/// Cached check for wpctl availability on Linux
static HAS_WPCTL: OnceLock<bool> = OnceLock::new();
static HAS_PACTL: OnceLock<bool> = OnceLock::new();

pub struct AudioGuard;

impl AudioGuard {
    /// Mute system audio (call before capture)
    #[inline]
    pub fn mute() {
        #[cfg(target_os = "macos")]
        Self::mute_macos();

        #[cfg(target_os = "linux")]
        if Self::is_wayland() {
            Self::mute_linux();
        }
        // Windows & X11: no-op
    }

    /// Unmute system audio (call after capture)
    #[inline]
    pub fn unmute() {
        #[cfg(target_os = "macos")]
        Self::unmute_macos();

        #[cfg(target_os = "linux")]
        if Self::is_wayland() {
            Self::unmute_linux();
        }
        // Windows & X11: no-op
    }

    // ========== macOS ==========
    
    #[cfg(target_os = "macos")]
    fn mute_macos() {
        // Using osascript for now - fast enough with flush
        // TODO: Replace with native CoreAudio FFI for zero-delay:
        // AudioObjectSetPropertyData(kAudioDevicePropertyMute)
        let _ = Command::new("osascript")
            .args(["-e", "set volume with output muted"])
            .output();
    }

    #[cfg(target_os = "macos")]
    fn unmute_macos() {
        let _ = Command::new("osascript")
            .args(["-e", "set volume without output muted"])
            .output();
    }

    // ========== Linux (Wayland only) ==========

    #[cfg(target_os = "linux")]
    fn mute_linux() {
        if *HAS_WPCTL.get_or_init(|| Self::has_cmd("wpctl")) {
            // PipeWire/WirePlumber (fastest, standard on modern Wayland)
            let _ = Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "1"])
                .output();
        } else if *HAS_PACTL.get_or_init(|| Self::has_cmd("pactl")) {
            // PulseAudio fallback
            let _ = Command::new("pactl")
                .args(["set-sink-mute", "@DEFAULT_SINK@", "1"])
                .output();
        } else {
            // ALSA fallback (rare)
            let _ = Command::new("amixer")
                .args(["-q", "sset", "Master", "mute"])
                .output();
        }
    }

    #[cfg(target_os = "linux")]
    fn unmute_linux() {
        if *HAS_WPCTL.get_or_init(|| Self::has_cmd("wpctl")) {
            let _ = Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "0"])
                .output();
        } else if *HAS_PACTL.get_or_init(|| Self::has_cmd("pactl")) {
            let _ = Command::new("pactl")
                .args(["set-sink-mute", "@DEFAULT_SINK@", "0"])
                .output();
        } else {
            let _ = Command::new("amixer")
                .args(["-q", "sset", "Master", "unmute"])
                .output();
        }
    }

    /// Check if running in Wayland session
    #[cfg(target_os = "linux")]
    fn is_wayland() -> bool {
        env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
    }

    /// Check if a command exists in PATH
    #[cfg(target_os = "linux")]
    fn has_cmd(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mute_unmute_doesnt_panic() {
        // Just verify no panic - actual audio state is system-dependent
        AudioGuard::mute();
        AudioGuard::unmute();
    }
}