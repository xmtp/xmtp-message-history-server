use crate::UPLOAD_DIR;
use anyhow::Result;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime},
};

pub fn spawn_cleanup_worker() {
    thread::spawn(|| loop {
        if let Err(err) = cleanup() {
            tracing::error!("{err:?}");
        };

        // Sleep four hours
        thread::sleep(Duration::from_secs(60 * 60 * 4));
    });
}

fn cleanup() -> Result<()> {
    let now = SystemTime::now();
    let upload_dir: PathBuf = UPLOAD_DIR.into();

    for entry in fs::read_dir(upload_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        let metadata = fs::metadata(&path)?;
        let modified = metadata.modified()?;
        let age = now.duration_since(modified)?;

        // Is the file less than a week old?
        if age < Duration::from_secs(60 * 60 * 24 * 7) {
            continue;
        }

        // Delete the old file
        fs::remove_file(path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        str::FromStr,
        time::{Duration, SystemTime},
    };

    use crate::UPLOAD_DIR;

    use super::cleanup;

    #[test]
    fn test_cleanup() {
        let path_oldfile = PathBuf::from_str(UPLOAD_DIR).unwrap().join("oldfile.bin");
        let path_newfile = path_oldfile.parent().unwrap().join("newfile.bin");

        let _ = std::fs::create_dir_all(path_oldfile.parent().unwrap());
        let _ = std::fs::remove_file(&path_oldfile);

        std::fs::write(&path_oldfile, b"hello there").unwrap();
        std::fs::write(&path_newfile, b"hello again").unwrap();

        let now = SystemTime::now();
        let one_day_ago = now
            .checked_sub(Duration::from_secs(24 * 60 * 60 * 2))
            .expect("Time calculation error");

        // Convert SystemTime to std::fs::FileTime
        let filetime = filetime::FileTime::from_system_time(one_day_ago);

        // Set the modification time
        filetime::set_file_mtime(&path_oldfile, filetime).unwrap();

        cleanup().unwrap();

        assert!(!path_oldfile.exists());
        assert!(path_newfile.exists());
    }
}
