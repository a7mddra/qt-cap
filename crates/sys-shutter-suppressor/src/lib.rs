use std::process::Command;
use std::env;

pub struct AudioGuard;

impl AudioGuard {
    pub fn mute() {
        // macOS: Force mute because CoreGraphics always plays a sound
        if cfg!(target_os = "macos") {
            Self::exec("osascript", &["-e", "set volume with output muted"]);
        } 
        // Linux: Only mute if on Wayland (X11 is silent)
        else if cfg!(target_os = "linux") && Self::is_wayland() {
            if Self::has_cmd("wpctl") {
                // PipeWire/WirePlumber (Standard on modern Wayland)
                let _ = Self::exec("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "1"]);
            } else if Self::has_cmd("pactl") {
                // PulseAudio
                let _ = Self::exec("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "1"]);
            } else {
                // ALSA Fallback
                let _ = Self::exec("amixer", &["-q", "sset", "Master", "mute"]);
            }
        }
        // Windows & X11: No operation needed
    }

    pub fn unmute() {
        if cfg!(target_os = "macos") {
            Self::exec("osascript", &["-e", "set volume without output muted"]);
        } 
        else if cfg!(target_os = "linux") && Self::is_wayland() {
            if Self::has_cmd("wpctl") {
                let _ = Self::exec("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "0"]);
            } else if Self::has_cmd("pactl") {
                let _ = Self::exec("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "0"]);
            } else {
                let _ = Self::exec("amixer", &["-q", "sset", "Master", "unmute"]);
            }
        }
    }

    /// Robustly checks if we are running in a Wayland session
    fn is_wayland() -> bool {
        env::var("XDG_SESSION_TYPE")
            .map(|v| v.to_lowercase().contains("wayland"))
            .unwrap_or(false)
    }

    fn has_cmd(cmd: &str) -> bool {
        // "which" is generally available on *nix systems
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn exec(cmd: &str, args: &[&str]) {
        let _ = Command::new(cmd).args(args).output();
    }
}