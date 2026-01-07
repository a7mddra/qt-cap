// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::fs;

use crate::utils::{project_root, run_cmd, run_cmd_with_node_bin};

pub fn ui_dir() -> std::path::PathBuf {
    project_root().join("ui")
}

pub fn tauri_dir() -> std::path::PathBuf {
    project_root().join("app")
}

pub fn run(cmd: &str) -> Result<()> {
    let ui = ui_dir();
    let app = tauri_dir();
    let node_bin = ui.join("node_modules").join(".bin");

    if !ui.join("node_modules").exists() {
        println!("\nInstalling npm dependencies...");
        run_cmd("npm", &["install"], &ui)?;
    }

    println!("\nRunning: tauri {}", cmd);
    run_cmd_with_node_bin("tauri", &[cmd], &app, &node_bin)?;

    Ok(())
}

pub fn build() -> Result<()> {
    println!("\nBuilding Tauri app...");
    let ui = ui_dir();
    let app = tauri_dir();
    let node_bin = ui.join("node_modules").join(".bin");

    if !ui.join("node_modules").exists() {
        println!("\nInstalling npm dependencies...");
        run_cmd("npm", &["install"], &ui)?;
    }

    run_cmd_with_node_bin("tauri", &["build"], &app, &node_bin)?;

    println!("\nApp build complete!");
    Ok(())
}

pub fn clean() -> Result<()> {
    println!("\nCleaning Tauri artifacts...");

    let tauri_target = tauri_dir().join("target");
    if tauri_target.exists() {
        println!("  Removing {}", tauri_target.display());
        fs::remove_dir_all(&tauri_target)?;
    }

    let binaries = tauri_dir().join("binaries");
    if binaries.exists() {
        for entry in fs::read_dir(&binaries)? {
            let entry = entry?;
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with("ocr-engine-")
            {
                println!("  Removing {}", entry.path().display());
                fs::remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}
