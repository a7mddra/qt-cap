//! Single instance lock for preventing multiple capture overlays
//!
//! Creates a .lock file to ensure only one capture session runs at a time.
//! This prevents double freezes and multiple overlays.
//!
//! IMPORTANT: The lock is automatically released on drop, but if the process
//! crashes, the lock file may remain. The OS file lock (via fs2) handles this
//! gracefully - a stale lock file without an active lock can be re-acquired.

use anyhow::{anyhow, Context, Result};
use fs2::FileExt;
use std::fs::{self, File};
use std::path::PathBuf;

/// A held instance lock - automatically releases on drop
pub struct InstanceLock {
    file: File,
    path: PathBuf,
}

impl InstanceLock {
    /// Try to acquire the instance lock.
    /// 
    /// Returns Ok(lock) if this is the only running instance.
    /// Returns Err if another instance is already running.
    /// 
    /// # Example
    /// ```ignore
    /// let _lock = InstanceLock::try_acquire("my-capture-app")?;
    /// // ... do capture ...
    /// // Lock automatically released when _lock goes out of scope
    /// ```
    pub fn try_acquire(app_name: &str) -> Result<Self> {
        let dir = Self::lock_dir()?;
        fs::create_dir_all(&dir)?;
        
        let path = dir.join(format!("{}.lock", app_name));
        
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("Failed to open lock file: {:?}", path))?;

        // Try exclusive lock - fails immediately if locked by another process
        file.try_lock_exclusive()
            .map_err(|_| anyhow!("Another instance is already running (lock: {:?})", path))?;

        Ok(Self { file, path })
    }

    /// Force release a potentially stale lock (emergency cleanup)
    /// 
    /// This removes the lock file entirely. Use with caution - only when
    /// you're certain no other instance is running.
    pub fn force_release(app_name: &str) -> Result<()> {
        let dir = Self::lock_dir()?;
        let path = dir.join(format!("{}.lock", app_name));
        
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove stale lock: {:?}", path))?;
        }
        Ok(())
    }

    /// Get the lock directory (XDG_RUNTIME_DIR or fallback to cache)
    fn lock_dir() -> Result<PathBuf> {
        dirs::runtime_dir()
            .or_else(dirs::cache_dir)
            .context("Failed to resolve lock directory (no XDG_RUNTIME_DIR or cache dir)")
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        // Unlock the file
        let _ = self.file.unlock();
        // Remove the lock file (harmless if it fails)
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_and_release() {
        let app_name = "test-single-instance-123";
        
        // First acquire should succeed
        let lock = InstanceLock::try_acquire(app_name);
        assert!(lock.is_ok(), "First lock should succeed");
        
        // Second acquire should fail
        let lock2 = InstanceLock::try_acquire(app_name);
        assert!(lock2.is_err(), "Second lock should fail");
        
        // Release first lock
        drop(lock);
        
        // Now acquire should succeed again
        let lock3 = InstanceLock::try_acquire(app_name);
        assert!(lock3.is_ok(), "Lock after release should succeed");
    }

    #[test]
    fn test_force_release() {
        let app_name = "test-force-release-456";
        let _lock = InstanceLock::try_acquire(app_name).unwrap();
        
        // Force release while locked - this removes the file but lock remains
        // until _lock is dropped (OS behavior)
        let result = InstanceLock::force_release(app_name);
        assert!(result.is_ok());
    }
}
