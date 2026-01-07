//! Qt Capture Rust Wrapper
//!
//! This is the outer "doll" of the Matryoshka architecture.
//! It wraps the Qt binary and provides:
//! - Single instance locking (prevent double runs)
//! - Display hotplug monitoring (kill on cable unplug)
//! - Shutter sound suppression (mute during capture)
//!
//! IPC Protocol (Qt → Rust via stdout):
//! - REQ_MUTE: Mute audio before capture
//! - CAPTURE_SUCCESS: Capture completed successfully
//! - CAPTURE_FAIL: Capture failed or cancelled

use std::env;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};

use anyhow::{Context, Result};
use sys_display_hotplug::DisplayWatcher;
use sys_shutter_suppressor::AudioGuard;
use sys_single_instance::InstanceLock;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("[qt-capture] Error: {:#}", e);
            AudioGuard::unmute(); // Safety unmute on error
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    // 1. Acquire single instance lock
    let _lock = InstanceLock::try_acquire("qt-capture")
        .context("Failed to acquire instance lock - is another capture running?")?;

    // 2. Mute audio BEFORE spawning Qt (Portal shutter plays during captureAll)
    AudioGuard::mute();

    // 3. Spawn Qt child process
    let args: Vec<String> = env::args().skip(1).collect();
    let mut child = spawn_qt_child(&args)?;
    let child_pid = child.id();

    // 4. Start display hotplug monitor (background thread)
    let watcher = DisplayWatcher::start(move || {
        eprintln!("[qt-capture] Display topology changed! Killing Qt...");
        kill_process(child_pid);
        // Note: This callback runs in background thread, so we can't return from main here.
        // The IPC loop will detect the child died and exit.
    });

    // 5. IPC loop - read Qt stdout
    let exit_code = if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut capture_success = false;
        let mut capture_path: Option<String> = None;

        for line in reader.lines() {
            match line {
                Ok(msg) => {
                    let trimmed = msg.trim();
                    match trimmed {
                        "REQ_MUTE" => {
                            // Already muted at startup - this is a no-op now
                            // Kept for backwards compatibility
                        }
                        "CAPTURE_SUCCESS" => {
                            capture_success = true;
                        }
                        "CAPTURE_FAIL" => {
                            capture_success = false;
                            break;
                        }
                        _ => {
                            // Check if it's a path (starts with /)
                            if trimmed.starts_with('/') && capture_success {
                                capture_path = Some(trimmed.to_string());
                                break;
                            } else {
                                // Passthrough Qt debug output
                                eprintln!("[Qt] {}", trimmed);
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }

        // Output the capture path if successful
        if let Some(path) = capture_path {
            println!("{}", path);
            ExitCode::from(0)
        } else {
            ExitCode::from(1)
        }
    } else {
        ExitCode::from(1)
    };

    // 5. Cleanup
    watcher.stop();
    let _ = child.wait();
    AudioGuard::unmute(); // Always unmute on exit

    Ok(exit_code)
}

/// Spawn the Qt binary as a child process
fn spawn_qt_child(args: &[String]) -> Result<Child> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().context("No parent dir for executable")?;

    // Find Qt binary based on platform
    let (qt_bin, qt_runtime_dir) = find_qt_paths(exe_dir)?;

    let mut cmd = Command::new(&qt_bin);
    cmd.args(args)
        .stdout(Stdio::piped()) // Capture for IPC
        .stderr(Stdio::inherit()); // Let errors flow

    // Set environment for portable Qt runtime
    #[cfg(target_os = "linux")]
    {
        let libs_path = qt_runtime_dir.join("libs");
        let plugins_path = qt_runtime_dir.join("plugins");
        
        // Append to existing LD_LIBRARY_PATH if set
        let mut ld_path = libs_path.to_string_lossy().to_string();
        if let Ok(existing) = env::var("LD_LIBRARY_PATH") {
            ld_path = format!("{}:{}", ld_path, existing);
        }
        
        cmd.env("LD_LIBRARY_PATH", ld_path)
            .env("QT_PLUGIN_PATH", &plugins_path)
            .env("QT_QPA_PLATFORM_PLUGIN_PATH", plugins_path.join("platforms"));
    }

    cmd.spawn().context("Failed to spawn Qt binary")
}

/// Find Qt binary and runtime directory
fn find_qt_paths(exe_dir: &std::path::Path) -> Result<(PathBuf, PathBuf)> {
    let qt_runtime = exe_dir.join("qt-runtime");

    #[cfg(target_os = "macos")]
    {
        let qt_bin = qt_runtime.join("capture.app/Contents/MacOS/capture");
        if qt_bin.exists() {
            return Ok((qt_bin, qt_runtime));
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try wrapper script first (handles env setup)
        let wrapper = qt_runtime.join("capture");
        if wrapper.exists() {
            return Ok((wrapper, qt_runtime));
        }
        // Fall back to direct binary
        let qt_bin = qt_runtime.join("bin/capture-bin");
        if qt_bin.exists() {
            return Ok((qt_bin, qt_runtime));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let qt_bin = qt_runtime.join("capture.exe");
        if qt_bin.exists() {
            return Ok((qt_bin, qt_runtime));
        }
    }

    anyhow::bail!(
        "Qt binary not found. Expected qt-runtime directory at {:?}",
        qt_runtime
    )
}

/// Kill a process by PID
fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        use std::process::Command;
        let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
    }
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
}