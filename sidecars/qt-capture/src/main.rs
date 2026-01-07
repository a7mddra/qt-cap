use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use sys_single_instance::InstanceLock;
use sys_shutter_suppressor::AudioGuard;
use sys_display_hotplug::DisplayMonitor;

fn main() -> anyhow::Result<()> {
    // 1. Acquire Lock
    let _lock = InstanceLock::try_acquire("spatialshot-capture")?;

    // 2. Setup Monitor Watcher
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();

    // 3. Spawn Qt Child
    let mut child = spawn_qt_child()?;
    let child_pid = child.id();

    // 4. Background Thread: Watch Monitors
    thread::spawn(move || {
        let mut monitor = DisplayMonitor::new();
        while r_clone.load(Ordering::Relaxed) {
            if monitor.check() {
                eprintln!("[RUST] Monitor topology changed! Killing Qt...");
                let _ = kill_child(child_pid);
                std::process::exit(1); // Hard exit
            }
            thread::sleep(Duration::from_millis(500));
        }
    });

    // 5. IPC Loop (Main Thread)
    // We capture stdout from Qt to handle commands
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        
        for line in reader.lines() {
            match line {
                Ok(msg) => {
                    match msg.trim() {
                        "REQ_MUTE" => {
                            AudioGuard::mute();
                            // Send ACK back to Qt's stdin (if we piped it)
                            // Note: We need to keep stdin open to write back
                            // Implementation detail: Write to child.stdin
                        },
                        "REQ_UNMUTE" => {
                            AudioGuard::unmute();
                        },
                        "CAPTURE_SUCCESS" => {
                            // Qt finished successfully
                            break;
                        },
                        _ => println!("[QT] {}", msg), // Passthrough logs
                    }
                }
                Err(_) => break,
            }
        }
    }

    // 6. Cleanup
    running.store(false, Ordering::Relaxed);
    let _ = child.wait();
    AudioGuard::unmute(); // Safety unmute
    Ok(())
}

fn spawn_qt_child() -> anyhow::Result<Child> {
    // Logic to find binary based on OS (Matryoshka doll logic)
    let exe_path = std::env::current_exe()?;
    let dir = exe_path.parent().unwrap();
    
    // Assume dist structure:
    // /bin/capture-wrapper (This rust binary)
    // /qt-runtime/bin/capture-core (The C++ binary)
    
    let qt_bin = if cfg!(target_os = "macos") {
        dir.join("../qt-runtime/capture.app/Contents/MacOS/capture")
    } else {
        dir.join("qt-runtime/bin/capture-core")
    };

    let mut cmd = Command::new(qt_bin);
    cmd.stdout(Stdio::piped()) // Capture Qt output for IPC
       .stderr(Stdio::inherit()); // Let errors flow

    // Environment injection (Linux specific)
    if cfg!(target_os = "linux") {
        let lib_path = dir.join("qt-runtime/libs");
        let plugins_path = dir.join("qt-runtime/plugins");
        cmd.env("LD_LIBRARY_PATH", lib_path)
           .env("QT_PLUGIN_PATH", plugins_path);
    }

    Ok(cmd.spawn()?)
}

fn kill_child(pid: u32) {
    #[cfg(unix)]
    let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
    #[cfg(windows)]
    let _ = Command::new("taskkill").args(&["/F", "/PID", &pid.to_string()]).output();
}