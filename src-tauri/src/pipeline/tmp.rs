use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Returns the directory used for per-session temporary WAVs.
/// Uses `%LOCALAPPDATA%\com.typr.app\tmp\` (different from `%APPDATA%`,
/// which holds `config.json` + `typr.db`). Resolved via `dirs::data_local_dir()`.
pub fn tmp_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir);
    base.join("com.typr.app").join("tmp")
}

/// Generate a fresh `<uuid>.wav` path. Creates the parent directory if
/// missing. Returns `None` when the directory cannot be created.
pub fn session_wav_path() -> Option<PathBuf> {
    let dir = tmp_dir();
    fs::create_dir_all(&dir).ok()?;
    let id = uuid::Uuid::new_v4();
    Some(dir.join(format!("{id}.wav")))
}

/// Best-effort sweep: delete `*.wav` and `*.txt` whose mtime is older than
/// `now - older_than`. Returns the count deleted.
pub fn sweep_stale_wavs(older_than: Duration) -> usize {
    sweep_dir(&tmp_dir(), older_than)
}

fn sweep_dir(dir: &Path, older_than: Duration) -> usize {
    let now = SystemTime::now();
    let mut deleted = 0usize;
    let Ok(entries) = fs::read_dir(dir) else { return 0; };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue; };
        if ext != "wav" && ext != "txt" { continue; }
        let Ok(meta) = entry.metadata() else { continue; };
        let Ok(mtime) = meta.modified() else { continue; };
        if let Ok(age) = now.duration_since(mtime) {
            if age >= older_than {
                if fs::remove_file(&path).is_ok() {
                    deleted += 1;
                }
            }
        }
    }
    deleted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn session_path_unique() {
        let a = session_wav_path().unwrap();
        let b = session_wav_path().unwrap();
        assert_ne!(a, b);
        assert_eq!(a.extension().unwrap(), "wav");
    }

    #[test]
    fn sweep_dir_deletes_old_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("old.wav");
        let new = tmp.path().join("new.wav");
        File::create(&old).unwrap().write_all(b"x").unwrap();
        File::create(&new).unwrap().write_all(b"x").unwrap();

        let backdate = SystemTime::now() - Duration::from_secs(60 * 30);
        filetime::set_file_mtime(&old, filetime::FileTime::from_system_time(backdate)).unwrap();

        let n = sweep_dir(tmp.path(), Duration::from_secs(60 * 10));
        assert_eq!(n, 1);
        assert!(!old.exists());
        assert!(new.exists());
    }

    #[test]
    fn sweep_dir_ignores_non_wav() {
        let tmp = tempfile::tempdir().unwrap();
        let other = tmp.path().join("other.bin");
        File::create(&other).unwrap().write_all(b"x").unwrap();
        let backdate = SystemTime::now() - Duration::from_secs(60 * 30);
        filetime::set_file_mtime(&other, filetime::FileTime::from_system_time(backdate)).unwrap();
        let n = sweep_dir(tmp.path(), Duration::from_secs(60 * 10));
        assert_eq!(n, 0);
        assert!(other.exists());
    }
}
