// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Capture Engine sidecar build automation.
//!
//! Handles building the Qt-based screen capture tool and its Rust wrapper
//! as a portable distribution for Tauri integration.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::utils::{project_root, run_cmd, copy_dir_all, target_triple};

/// Get the Qt capture sidecar directory path.
pub fn sidecar_dir() -> std::path::PathBuf {
    project_root().join("sidecars").join("qt-capture")
}

/// Get the Qt native source directory.
fn qt_native_dir() -> std::path::PathBuf {
    sidecar_dir().join("native")
}

/// Build the capture-engine sidecar (Qt + Rust wrapper).
pub fn build() -> Result<()> {
    println!("\n🔨 Building Capture Engine...");
    
    // Step 1: Build Qt native binary
    build_qt_native()?;
    
    // Step 2: Build Rust wrapper
    build_rust_wrapper()?;
    
    // Step 3: Package for Tauri
    package_for_tauri()?;
    
    println!("\n✅ Capture Engine build complete!");
    Ok(())
}

/// Build only the Qt native binary (no Rust wrapper).
pub fn build_qt_only() -> Result<()> {
    println!("\n🔨 Building Qt native binary...");
    build_qt_native()?;
    println!("\n✅ Qt build complete!");
    Ok(())
}

/// Build the Qt native binary using CMake.
fn build_qt_native() -> Result<()> {
    println!("\n📦 Building Qt native...");
    
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

/// Build the Rust wrapper binary.
fn build_rust_wrapper() -> Result<()> {
    println!("\n🦀 Building Rust wrapper...");
    
    let sidecar = sidecar_dir();
    
    run_cmd(
        "cargo",
        &["build", "--release", "-p", "capture-engine"],
        &project_root(),
    )?;
    
    Ok(())
}

/// Package the build artifacts for Tauri.
fn package_for_tauri() -> Result<()> {
    println!("\n📋 Packaging for Tauri...");
    
    let target_dir = project_root().join("target").join("release");
    let qt_dist = qt_native_dir().join("dist");
    let qt_runtime = target_dir.join("qt-runtime");
    
    // Copy qt-runtime folder
    if qt_runtime.exists() {
        fs::remove_dir_all(&qt_runtime)?;
    }
    copy_dir_all(&qt_dist, &qt_runtime)?;
    
    // Verify binary exists
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

/// Clean capture engine build artifacts.
pub fn clean() -> Result<()> {
    println!("\n🧹 Cleaning capture engine artifacts...");
    
    let native_dir = qt_native_dir();
    
    for dir in ["build", "dist"] {
        let path = native_dir.join(dir);
        if path.exists() {
            println!("  Removing {}", path.display());
            fs::remove_dir_all(&path)?;
        }
    }
    
    let qt_runtime = project_root().join("target").join("release").join("qt-runtime");
    if qt_runtime.exists() {
        println!("  Removing {}", qt_runtime.display());
        fs::remove_dir_all(&qt_runtime)?;
    }
    
    Ok(())
}
