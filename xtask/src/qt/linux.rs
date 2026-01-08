// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Linux Qt deployment.
//!
//! Builds Qt project with CMake and creates portable distribution
//! with all required libraries and plugins.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::copy_dir_all;

pub fn build(native_dir: &Path) -> Result<()> {
    let build_dir = native_dir.join("build");
    let dist_dir = native_dir.join("dist");

    let qmake = find_qmake()?;
    let qt_prefix = get_qt_prefix(&qmake)?;
    let qt_plugins = get_qt_plugins(&qmake)?;

    println!("  Qt Prefix: {}", qt_prefix);
    println!("  Qt Plugins: {}", qt_plugins);
    
    let qt_qml = get_qt_qml(&qmake)?;
    println!("  Qt QML: {}", qt_qml);

    println!("  Configuring CMake...");
    fs::create_dir_all(&build_dir)?;

    let status = Command::new("cmake")
        .args([
            "-S",
            native_dir.to_str().unwrap(),
            "-B",
            build_dir.to_str().unwrap(),
            "-DCMAKE_BUILD_TYPE=Release",
            &format!("-DCMAKE_PREFIX_PATH={}", qt_prefix),
        ])
        .status()
        .context("Failed to run cmake configure")?;

    if !status.success() {
        anyhow::bail!("CMake configure failed");
    }

    println!("  Building...");
    let status = Command::new("cmake")
        .args([
            "--build",
            build_dir.to_str().unwrap(),
            "--config",
            "Release",
            "--parallel",
        ])
        .status()
        .context("Failed to run cmake build")?;

    if !status.success() {
        anyhow::bail!("CMake build failed");
    }

    println!("  Creating distribution...");
    create_distribution(native_dir, &build_dir, &dist_dir, &qt_plugins, &qt_qml, &qt_prefix)?;

    Ok(())
}

fn find_qmake() -> Result<String> {
    for cmd in ["qmake6", "qmake"] {
        if let Ok(output) = Command::new("which").arg(cmd).output() {
            if output.status.success() {
                return Ok(cmd.to_string());
            }
        }
    }
    anyhow::bail!("qmake not found. Please install Qt6 development packages.")
}

fn get_qt_prefix(qmake: &str) -> Result<String> {
    let output = Command::new(qmake)
        .args(["-query", "QT_INSTALL_PREFIX"])
        .output()
        .context("Failed to query Qt prefix")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_qt_plugins(qmake: &str) -> Result<String> {
    let output = Command::new(qmake)
        .args(["-query", "QT_INSTALL_PLUGINS"])
        .output()
        .context("Failed to query Qt plugins")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_qt_qml(qmake: &str) -> Result<String> {
    let output = Command::new(qmake)
        .args(["-query", "QT_INSTALL_QML"])
        .output()
        .context("Failed to query Qt QML path")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn create_distribution(
    _native_dir: &Path,
    build_dir: &Path,
    dist_dir: &Path,
    qt_plugins: &str,
    qt_qml: &str,
    qt_prefix: &str,
) -> Result<()> {
    if dist_dir.exists() {
        fs::remove_dir_all(dist_dir)?;
    }

    let bin_dir = dist_dir.join("bin");
    let libs_dir = dist_dir.join("libs");
    let plugins_dir = dist_dir.join("plugins");
    let qml_dir = dist_dir.join("qml");
    let fonts_dir = dist_dir.join("fonts");

    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&libs_dir)?;
    fs::create_dir_all(&plugins_dir)?;
    fs::create_dir_all(&qml_dir)?;
    fs::create_dir_all(&fonts_dir)?;

    let bin_src = build_dir.join("capture-bin");
    let bin_dst = bin_dir.join("capture-bin");

    let bin_src = if !bin_src.exists() {
        build_dir.join("capture")
    } else {
        bin_src
    };

    if !bin_src.exists() {
        anyhow::bail!("Compiled binary not found at {}", bin_src.display());
    }

    fs::copy(&bin_src, &bin_dst)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin_dst, fs::Permissions::from_mode(0o755))?;
    }

    let plugin_dirs = [
        "platforms",
        "imageformats",
        "xcbglintegrations",
        "platformthemes",
        "wayland-decoration-client",
        "wayland-graphics-integration-client",
        "wayland-shell-integration",
    ];
    let qt_plugins_path = Path::new(qt_plugins);

    for plugin_dir in plugin_dirs {
        let src = qt_plugins_path.join(plugin_dir);
        let dst = plugins_dir.join(plugin_dir);
        if src.exists() {
            copy_dir_all(&src, &dst)?;
        } else {
            println!("  Warning: Plugin category '{}' not found.", plugin_dir);
        }
    }

    let qml_modules = ["QtQuick", "Qt5Compat", "QtQml"];
    let qt_qml_path = Path::new(qt_qml);

    for module in qml_modules {
        let src = qt_qml_path.join(module);
        let dst = qml_dir.join(module);
        if src.exists() {
            copy_dir_all(&src, &dst)?;
        } else {
            println!("  Warning: QML module '{}' not found at {}", module, src.display());
        }
    }
    
    // Copy root QML files (builtins.qmltypes, etc.)
    if let Ok(entries) = fs::read_dir(qt_qml_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name() {
                    fs::copy(&path, qml_dir.join(name))?;
                }
            }
        }
    }

    let qt_lib_path = Path::new(qt_prefix).join("lib");
    let qt_lib_str = qt_lib_path.to_string_lossy().to_string();

    let mut visited = HashSet::new();
    println!("  Resolving dependencies (search path: {})...", qt_lib_str);
    resolve_libraries_recursive(&bin_dst, &libs_dir, &mut visited, &qt_lib_str)?;

    let all_plugins = find_all_files(&plugins_dir)?;
    for plugin in all_plugins {
        resolve_libraries_recursive(&plugin, &libs_dir, &mut visited, &qt_lib_str)?;
    }
    
    let all_qml_plugins = find_all_files(&qml_dir)?;
    for plugin in all_qml_plugins {
        resolve_libraries_recursive(&plugin, &libs_dir, &mut visited, &qt_lib_str)?;
    }

    bundle_misc_libraries(&libs_dir, &qt_lib_path)?;

    // FIX 1: Disabled bundling of system-level HAL libraries (libxcb, libwayland, etc.)
    // These must come from the user's OS to match the kernel/drivers.
    // bundle_xcb_libraries(&libs_dir)?;

    if check_command_exists("patchelf") {
        println!("  Setting RPATH with patchelf...");
        // It is safe to patch the binary and the shared libs
        patch_rpath_recursive(&bin_dir, "lib", &libs_dir)?;
        patch_rpath_recursive(&libs_dir, "lib", &libs_dir)?;
        
        // FIX 2: Disabled patchelf on plugins.
        // Patchelf corrupts the .qtmetadata section in plugins, causing "metadata not found".
        // The wrapper script sets LD_LIBRARY_PATH, which is sufficient.
        
        // patch_rpath_recursive(&plugins_dir, "plugin", &libs_dir)?;
        // patch_rpath_recursive(&qml_dir, "qml", &libs_dir)?;
    } else {
        println!("  Warning: patchelf not found. RPATH not set.");
    }

    let sys_fonts_conf = Path::new("/etc/fonts/fonts.conf");
    if sys_fonts_conf.exists() {
        fs::copy(sys_fonts_conf, fonts_dir.join("fonts.conf"))?;
    }

    create_runner_script(dist_dir)?;

    fs::write(
        dist_dir.join("qt.conf"),
        "[Paths]\nPrefix = .\nPlugins = plugins\nQml2Imports = qml\n",
    )?;

    Ok(())
}

fn find_all_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_all_files(&path)?);
            } else {
                if let Some(ext) = path.extension() {
                    if ext == "so" {
                        files.push(path);
                    }
                }
            }
        }
    }
    Ok(files)
}

fn resolve_libraries_recursive(
    binary: &Path,
    libs_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    qt_lib_path: &str,
) -> Result<()> {
    let output = Command::new("ldd")
        .arg(binary)
        .env("LD_LIBRARY_PATH", qt_lib_path)
        .output()
        .context("Failed to run ldd")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // FIX 3: Expanded blacklist to ignore system libs that crash if bundled.
    let skip_libs = [
        "linux-vdso",
        "libgcc_s",
        "libc.so",
        "libm.so",
        "ld-linux",
        "libpthread",
        "librt",
        "libdl",
        "libGL",
        "libEGL",
        "libGLX",
        "libOpenGL",
        "libdrm",
        "libglapi",
        "libstdc++",
        "libgcc_s",
        "libglib",
        "libpcre",
        "libz",
        "libxcb", 
        "libX11",
        "libXext",
        "libXau",
        "libXdmcp",
        "libxkbcommon",
        "libwayland",
        "libffi",
        "libexpat",
        "libdbus",
    ];

    for line in stdout.lines() {
        if let Some(arrow_pos) = line.find("=>") {
            let after_arrow = &line[arrow_pos + 2..].trim();
            if let Some(path_end) = after_arrow.find(" (") {
                let lib_path_str = &after_arrow[..path_end].trim();
                let lib_path = Path::new(lib_path_str);

                if !lib_path.exists() {
                    continue;
                }

                let lib_name = lib_path.file_name().unwrap().to_str().unwrap();

                let mut skip = false;
                for s in skip_libs {
                    if lib_name.contains(s) {
                        skip = true;
                        break;
                    }
                }
                if skip {
                    continue;
                }

                if !visited.contains(lib_path) {
                    visited.insert(lib_path.to_path_buf());

                    let dst = libs_dir.join(lib_name);
                    if !dst.exists() {
                        fs::copy(lib_path, &dst)?;
                    }

                    resolve_libraries_recursive(lib_path, libs_dir, visited, qt_lib_path)?;
                }
            }
        }
    }

    Ok(())
}

fn bundle_misc_libraries(libs_dir: &Path, qt_lib_path: &Path) -> Result<()> {
    // FIX 4: Explicitly force these libraries to be bundled.
    // This fixes the crash where "Squiggle" visuals fail due to missing ShaderTools.
    let critical_libs = [
        "libQt6Qml.so.6",
        "libQt6QmlWorkerScript.so.6",
        "libQt6Core5Compat.so.6",
        "libQt6ShaderTools.so.6", // <--- THE CRITICAL MISSING LIBRARY
        "libQt6Svg.so.6",
        "libQt6OpenGL.so.6",
        "libQt6WaylandClient.so.6",
        "libQt6WaylandCompositor.so.6",
        "libQt6WlShellIntegration.so.6",
        "libQt6XcbQpa.so.6",
    ];

    println!("  Bundling critical Qt libraries...");

    for lib_name in critical_libs {
        let src_path = qt_lib_path.join(lib_name);
        let dst_path = libs_dir.join(lib_name);

        if src_path.exists() {
            if !dst_path.exists() {
                fs::copy(&src_path, &dst_path)?;
                println!("    + {}", lib_name);
            }
        } else {
            // FIX 5: Fixed Borrow Checker Error here.
            // Using &name to borrow instead of move.
            let mut found = false;
            if let Ok(entries) = fs::read_dir(qt_lib_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    
                    if name_str.starts_with(lib_name) {
                        let real_src = entry.path();
                        let real_dst = libs_dir.join(&name); // <--- Fixed line
                        if !real_dst.exists() {
                            fs::copy(&real_src, &real_dst)?;
                            println!("    + {} (found as {})", lib_name, name_str);
                        }
                        found = true;
                    }
                }
            }
            if !found {
                println!("    ! Warning: Could not find critical lib: {}", lib_name);
            }
        }
    }
    Ok(())
}

// Helper Functions that were missing in previous block

fn create_runner_script(dist_dir: &Path) -> Result<()> {
    let script = r#"#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
export LD_LIBRARY_PATH="$DIR/libs:$DIR/lib:$LD_LIBRARY_PATH"
export QT_PLUGIN_PATH="$DIR/plugins"
export QT_QPA_PLATFORM_PLUGIN_PATH="$DIR/plugins/platforms"
export QML2_IMPORT_PATH="$DIR/qml"

exec "$DIR/bin/capture-bin" "$@"
"#;

    let script_path = dist_dir.join("capture");
    fs::write(&script_path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

fn patch_rpath_recursive(root: &Path, _type_hint: &str, libs_dir: &Path) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    // Iterate over all files
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_file() {
            continue;
        }

        // Check if it's an ELF file or .so
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let is_so = name.ends_with(".so") || name.contains(".so.");
        let is_bin = path.parent().map(|p| p.ends_with("bin")).unwrap_or(false);

        if is_so || is_bin {
            // Calculate relative path from this file's directory to libs_dir
            let file_dir = path.parent().unwrap();
            
            // We need to find the path from file_dir to libs_dir
            let relative_to_libs = pathdiff::diff_paths(libs_dir, file_dir);

            if let Some(rel) = relative_to_libs {
                let origin_path = format!("$ORIGIN/{}", rel.display());
                
                // Run patchelf
                let _ = Command::new("patchelf")
                    .arg("--set-rpath")
                    .arg(&origin_path)
                    .arg(path)
                    .output();
            }
        }
    }
    Ok(())
}

fn check_command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}