// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Capture Engine sidecar build automation.
//!
//! Handles building the Qt-based screen capture tool and its Rust wrapper
//! as a portable distribution for Tauri integration.

use anyhow::Result;
use std::fs;

use crate::utils::{copy_dir_all, project_root, run_cmd};

pub fn sidecar_dir() -> std::path::PathBuf {
    project_root().join("sidecars").join("qt-capture")
}

fn qt_native_dir() -> std::path::PathBuf {
    sidecar_dir().join("native")
}

pub fn build() -> Result<()> {
    println!("\nBuilding Capture Engine...");

    build_qt_native()?;

    build_rust_wrapper()?;

    package_for_tauri()?;

    println!("\nCapture Engine build complete!");
    Ok(())
}

pub fn build_qt_only() -> Result<()> {
    println!("\nBuilding Qt native binary...");
    build_qt_native()?;
    println!("\nQt build complete!");
    Ok(())
}

fn build_qt_native() -> Result<()> {
    println!("\nBuilding Qt native...");

    let native_dir = qt_native_dir();

    #[cfg(target_os = "linux")]
    {
        crate::qt::linux::build(&native_dir)?;
    }

    #[cfg(target_os = "macos")]
    {
        crate::qt::macos::build(&native_dir)?;
    }

    #[cfg(target_os = "windows")]
    {
        crate::qt::windows::build(&native_dir)?;
    }

    Ok(())
}

fn build_rust_wrapper() -> Result<()> {
    println!("\nBuilding Rust wrapper...");

    let _sidecar = sidecar_dir();

    run_cmd(
        "cargo",
        &["build", "--release", "-p", "capture-engine"],
        &project_root(),
    )?;

    Ok(())
}

fn package_for_tauri() -> Result<()> {
    println!("\nPackaging for Tauri...");

    let target_dir = project_root().join("target").join("release");
    let qt_dist = qt_native_dir().join("dist");
    let qt_runtime = target_dir.join("qt-runtime");

    if qt_runtime.exists() {
        fs::remove_dir_all(&qt_runtime)?;
    }
    copy_dir_all(&qt_dist, &qt_runtime)?;

    let binary_name = format!("capture-engine{}", if cfg!(windows) { ".exe" } else { "" });
    let binary_path = target_dir.join(&binary_name);

    if !binary_path.exists() {
        anyhow::bail!("Binary not found: {}", binary_path.display());
    }

    let size_mb = fs::metadata(&binary_path)?.len() as f64 / (1024.0 * 1024.0);
    println!("  ✓ Binary: {} ({:.1} MB)", binary_path.display(), size_mb);
    println!("  ✓ Qt Runtime: {}", qt_runtime.display());

    Ok(())
}

pub fn clean() -> Result<()> {
    println!("\nCleaning capture engine artifacts...");

    let native_dir = qt_native_dir();

    for dir in ["build", "dist"] {
        let path = native_dir.join(dir);
        if path.exists() {
            println!("  Removing {}", path.display());
            fs::remove_dir_all(&path)?;
        }
    }

    let qt_runtime = project_root()
        .join("target")
        .join("release")
        .join("qt-runtime");
    if qt_runtime.exists() {
        println!("  Removing {}", qt_runtime.display());
        fs::remove_dir_all(&qt_runtime)?;
    }

    Ok(())
}
