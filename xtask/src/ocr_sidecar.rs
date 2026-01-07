// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

use crate::utils::{copy_dir_all, project_root, run_cmd, target_triple};

pub fn sidecar_dir() -> std::path::PathBuf {
    project_root().join("sidecars").join("paddle-ocr")
}

fn tauri_dir() -> std::path::PathBuf {
    project_root().join("app")
}

fn venv_python() -> std::path::PathBuf {
    let sidecar = sidecar_dir();
    if cfg!(windows) {
        sidecar.join("venv").join("Scripts").join("python.exe")
    } else {
        sidecar.join("venv").join("bin").join("python")
    }
}

pub fn build() -> Result<()> {
    println!("\nBuilding PaddleOCR sidecar...");

    let sidecar = sidecar_dir();
    let venv = sidecar.join("venv");

    if !venv.exists() {
        println!("\nCreating virtual environment...");
        run_cmd("python3", &["-m", "venv", "venv"], &sidecar)?;
    }

    println!("\nInstalling dependencies...");
    let pip = if cfg!(windows) {
        venv.join("Scripts").join("pip.exe")
    } else {
        venv.join("bin").join("pip")
    };
    run_cmd(
        pip.to_str().unwrap(),
        &["install", "-r", "requirements.txt"],
        &sidecar,
    )?;

    println!("\nApplying patches...");
    let python = venv_python();
    let py = python.to_str().unwrap();

    run_cmd(py, &["patches/paddleocr.py"], &sidecar)?;
    run_cmd(py, &["patches/paddle_core.py"], &sidecar)?;
    run_cmd(py, &["patches/cpp_extension.py"], &sidecar)?;
    run_cmd(py, &["patches/iaa_augment.py"], &sidecar)?;

    println!("\nDownloading models...");
    run_cmd(py, &["download_models.py"], &sidecar)?;

    let home = dirs::home_dir().context("Could not find home directory")?;
    let cache = home.join(".paddleocr").join("whl");
    let models_dir = sidecar.join("models");

    fs::create_dir_all(&models_dir)?;

    let model_mappings = [
        ("det/en/en_PP-OCRv3_det_infer", "en_PP-OCRv3_det"),
        ("rec/en/en_PP-OCRv4_rec_infer", "en_PP-OCRv4_rec"),
        (
            "cls/ch_ppocr_mobile_v2.0_cls_infer",
            "ch_ppocr_mobile_v2.0_cls",
        ),
    ];

    for (src_rel, dst_name) in model_mappings {
        let src = cache.join(src_rel);
        let dst = models_dir.join(dst_name);
        if src.exists() {
            if dst.exists() {
                fs::remove_dir_all(&dst)?;
            }
            copy_dir_all(&src, &dst)?;
            println!("  Copied {} -> {}", src_rel, dst_name);
        }
    }

    println!("\nBuilding executable...");
    let pyinstaller = if cfg!(windows) {
        venv.join("Scripts").join("pyinstaller.exe")
    } else {
        venv.join("bin").join("pyinstaller")
    };
    run_cmd(
        pyinstaller.to_str().unwrap(),
        &["--clean", "ocr-engine.spec"],
        &sidecar,
    )?;

    println!("\nCopying to Tauri binaries...");
    let binary_name = if cfg!(windows) {
        format!("ocr-engine-{}.exe", target_triple())
    } else {
        format!("ocr-engine-{}", target_triple())
    };
    let dist_dir = sidecar.join("dist");
    let src_exe = if cfg!(windows) {
        dist_dir.join("ocr-engine.exe")
    } else {
        dist_dir.join("ocr-engine")
    };
    let tauri_binaries = tauri_dir().join("binaries");

    fs::create_dir_all(&tauri_binaries)?;

    let dst_exe = tauri_binaries.join(&binary_name);
    fs::copy(&src_exe, &dst_exe)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dst_exe, fs::Permissions::from_mode(0o755))?;
    }

    let size_mb = fs::metadata(&dst_exe)?.len() as f64 / (1024.0 * 1024.0);
    println!("  ✓ Built: {} ({:.1} MB)", dst_exe.display(), size_mb);

    println!("\nTesting executable...");
    let test_image = project_root().join("test_sample.png");
    if test_image.exists() {
        let output = Command::new(&dst_exe).arg(&test_image).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Hello OCR World!") {
            println!("  ✓ Test passed!");
        } else {
            println!("  ⚠ Test output: {}", stdout);
        }
    }

    println!("\nSidecar build complete!");
    Ok(())
}

pub fn clean() -> Result<()> {
    println!("\nCleaning sidecar artifacts...");

    let sidecar = sidecar_dir();

    for dir in ["venv", "build", "dist", "models"] {
        let path = sidecar.join(dir);
        if path.exists() {
            println!("  Removing {}", path.display());
            fs::remove_dir_all(&path)?;
        }
    }

    Ok(())
}
