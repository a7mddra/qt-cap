use anyhow::{anyhow, Context, Result};
use fs2::FileExt;
use std::fs::{self, File};
use std::path::PathBuf;

pub struct InstanceLock {
    file: File,
    path: PathBuf,
}

impl InstanceLock {
    pub fn try_acquire(app_name: &str) -> Result<Self> {
        let dir = dirs::runtime_dir()
            .or_else(dirs::cache_dir)
            .context("Failed to resolve lock directory")?;
        
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.lock", app_name));

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        // Try to lock exclusively. If it fails, another instance exists.
        file.try_lock_exclusive()
            .map_err(|_| anyhow!("App is already running (Locked at {:?})", path))?;

        Ok(Self { file, path })
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        // Optional: Remove file, though keeping it is harmless
        let _ = fs::remove_file(&self.path); 
    }
}
