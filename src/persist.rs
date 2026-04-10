use crate::error::{Result, StorageError};
use fs2::FileExt;
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;

/// Atomic write: write to a temp file in the same directory, then rename.
pub fn atomic_write(path: &Path, data: &impl Serialize) -> Result<()> {
    let json =
        serde_json::to_string_pretty(data).map_err(|e| StorageError::json(path, e))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &json).map_err(|e| StorageError::io(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| StorageError::io(path, e))?;
    Ok(())
}

/// Read and deserialize JSON from a file.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let data = fs::read_to_string(path).map_err(|e| StorageError::io(path, e))?;
    serde_json::from_str(&data).map_err(|e| StorageError::json(path, e))
}

/// Acquire an exclusive file lock on the given lock file path.
/// Creates the lock file if it doesn't exist.
/// Returns a guard that releases the lock when dropped.
pub fn lock_exclusive(lock_path: &Path) -> Result<LockGuard> {
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|e| StorageError::io(lock_path, e))?;
    file.lock_exclusive()
        .map_err(|e| StorageError::io(lock_path, e))?;
    Ok(LockGuard(file))
}

pub struct LockGuard(fs::File);

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
