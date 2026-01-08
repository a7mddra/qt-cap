// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Capture Engine sidecar build automation.
//!
//! Handles building the Qt-based screen capture tool and its Rust wrapper
//! as a portable distribution for Tauri integration.

use anyhow::Result;
use std::fs;

use crate::utils::{project_root, run_cmd};

pub fn sidecar_dir() -> std::path::PathBuf {
    project_root().join("sidecars").join("qt-capture")
}

fn qt_native_dir() -> std::path::PathBuf {
    sidecar_dir().join("native")
}

pub fn build() -> Result<()> {
    println!("\nBuilding Capture Engine...");

    // 1. Build Qt (CMake)
    build_qt_native()?;

    // 2. Deploy Qt (Bundle)
    println!("\nDeploying Qt runtime...");
    deploy_qt_native()?;

    // 3. Sign (macOS only)
    #[cfg(target_os = "macos")]
    {
        println!("\nSigning macOS bundle...");
        crate::qt::macos::sign(&qt_native_dir())?;
    }

    // 4. Build Rust Wrapper
    build_rust_wrapper()?;

    // 5. Package into app/binaries
    package_artifacts()?;

    println!("\nCapture Engine build complete!");
    Ok(())
}

pub fn build_qt_only() -> Result<()> {
    println!("\nBuilding Qt native binary (CMake only)...");
    build_qt_native()?;
    println!("\nQt build complete!");
    Ok(())
}

fn build_qt_native() -> Result<()> {
    println!("\nRunning Qt CMake build...");

    let native_dir = qt_native_dir();

    #[cfg(target_os = "linux")]
    crate::qt::linux::build(&native_dir)?;

    #[cfg(target_os = "macos")]
    crate::qt::macos::build(&native_dir)?;

    #[cfg(target_os = "windows")]
    crate::qt::windows::build(&native_dir)?;

    Ok(())
}

fn deploy_qt_native() -> Result<()> {
    let native_dir = qt_native_dir();

    #[cfg(target_os = "linux")]
    crate::qt::linux::deploy(&native_dir)?;

    #[cfg(target_os = "macos")]
    crate::qt::macos::deploy(&native_dir)?;

    #[cfg(target_os = "windows")]
    crate::qt::windows::deploy(&native_dir)?;

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

fn package_artifacts() -> Result<()> {
    println!("\nPackaging artifacts for Tauri...");

    let target_dir = project_root().join("target").join("release");
    let qt_runtime_src = qt_native_dir().join("qt-runtime");
    
    // Tauri app structure
    let app_binaries = project_root().join("app").join("binaries");
    fs::create_dir_all(&app_binaries)?;

    // 1. Move qt-runtime to app/binaries/qt-runtime
    let qt_runtime_dst = app_binaries.join("qt-runtime");
    if qt_runtime_dst.exists() {
        fs::remove_dir_all(&qt_runtime_dst)?;
    }
    
    if !qt_runtime_src.exists() {
        anyhow::bail!("Qt runtime not found at {}", qt_runtime_src.display());
    }

    println!("  Moving qt-runtime to {}", qt_runtime_dst.display());
    // Try rename first (atomic move), fall back to copy+delete
    if fs::rename(&qt_runtime_src, &qt_runtime_dst).is_err() {
        crate::utils::copy_dir_all(&qt_runtime_src, &qt_runtime_dst)?;
        fs::remove_dir_all(&qt_runtime_src)?;
    }

    // 2. Copy and rename Rust binary
    let src_binary_name = format!("capture-engine{}", if cfg!(windows) { ".exe" } else { "" });
    let src_binary_path = target_dir.join(&src_binary_name);

    if !src_binary_path.exists() {
        anyhow::bail!("Rust binary not found: {}", src_binary_path.display());
    }

    let target_triple = sys_info::os_type().unwrap_or_else(|_| "unknown".to_string()); 
    // Note: This is a rough guess. Ideally we use the actual target triple from cargo.
    // But since we are running xtask on the host, we can assume host target.
    // For now let's construct it properly via rustc or use a hardcoded guess since xtask is local.
    // A safer bet for now is to just use a fixed suffix or the strict one requested if we knew it.
    // User requested: ocr-engine-x86_64-unknown-linux-gnu.
    // We should probably shell out to `rustc -vV` to get host triple or just use a helper.
    let host_triple = get_host_target_triple()?;
    
    let dst_binary_name = format!("capture-engine-{}{}", host_triple, if cfg!(windows) { ".exe" } else { "" });
    let dst_binary_path = app_binaries.join(&dst_binary_name);

    println!("  Copying binary to {}", dst_binary_path.display());
    fs::copy(&src_binary_path, &dst_binary_path)?;

    Ok(())
}

fn get_host_target_triple() -> Result<String> {
    let output = std::process::Command::new("rustc")
        .arg("-vV")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("host: ") {
            return Ok(line.trim_start_matches("host: ").trim().to_string());
        }
    }
    Ok("unknown-target".to_string())
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
