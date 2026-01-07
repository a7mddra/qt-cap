// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Build automation for ocr-engine and capture-engine
//!
//! Usage:
//!   cargo xtask build              Build everything (OCR + Capture)
//!   cargo xtask build-ocr          Build PaddleOCR sidecar executable
//!   cargo xtask build-capture      Build Capture Engine (Qt + Rust)
//!   cargo xtask build-capture-qt   Build Qt native only (no Rust)
//!   cargo xtask clean              Clean all build artifacts
//!   cargo xtask run <cmd>          Run Tauri commands (dev, build, etc.)

mod ocr_sidecar;
mod capture_sidecar;
mod tauri;
mod utils;
mod qt;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for sidecars")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build everything (OCR sidecar + Capture Engine)
    Build,
    
    /// Build PaddleOCR sidecar executable
    BuildOcr,
    
    /// Build Capture Engine (Qt + Rust wrapper)
    BuildCapture,
    
    /// Build Qt native only (no Rust wrapper)
    BuildCaptureQt,
    
    /// Build Tauri application for release
    BuildApp,
    
    /// Clean all build artifacts
    Clean,
    
    /// Run Tauri commands (dev, build, etc.)
    Run {
        /// Tauri command to run (e.g., dev, build)
        #[arg(default_value = "dev")]
        cmd: String,
    },
    
    /// Start development mode
    Dev,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Build => {
            ocr_sidecar::build()?;
            capture_sidecar::build()?;
            tauri::build()?;
        }
        Commands::BuildOcr => {
            ocr_sidecar::build()?;
        }
        Commands::BuildCapture => {
            capture_sidecar::build()?;
        }
        Commands::BuildCaptureQt => {
            capture_sidecar::build_qt_only()?;
        }
        Commands::BuildApp => {
            tauri::build()?;
        }
        Commands::Clean => {
            ocr_sidecar::clean()?;
            capture_sidecar::clean()?;
            tauri::clean()?;
        }
        Commands::Run { cmd } => {
            tauri::run(&cmd)?;
        }
        Commands::Dev => {
            tauri::run("dev")?;
        }
    }
    
    Ok(())
}
