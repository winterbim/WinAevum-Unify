//! Atomic durable writes for mission-critical JSON/JSONL files (P0-6).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Write `data` to `path` via tempfile + fsync + rename (never truncating in place).
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    let tmp = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("aevum"),
        std::process::id()
    ));
    {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|e| format!("open tmp {}: {e}", tmp.display()))?;
        f.write_all(data)
            .map_err(|e| format!("write tmp {}: {e}", tmp.display()))?;
        f.sync_all()
            .map_err(|e| format!("fsync tmp {}: {e}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("rename {} → {}: {e}", tmp.display(), path.display())
    })?;
    // Best-effort directory fsync so the rename itself is durable.
    #[cfg(unix)]
    {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Restrict a file to owner read/write only (0600) on Unix.
pub fn set_mode_600(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("chmod 600 {}: {e}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Restrict a file to owner/group/other-readable (0644) on Unix.
pub fn set_mode_644(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o644))
            .map_err(|e| format!("chmod 644 {}: {e}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
