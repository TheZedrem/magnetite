//! Private windows: a Magnetite of its own (`--private`), started by the one
//! in use. It runs on a new, empty data folder that is deleted when it quits,
//! so nothing of it stays and nothing of the user's profile is in it: no
//! logins, history, extensions, sync account or saved tabs. Under Linux the
//! folder is in memory (`XDG_RUNTIME_DIR`).

use std::path::{Path, PathBuf};

/// The data folder of a private instance, deleted when dropped.
pub struct PrivateDir(PathBuf);

impl PrivateDir {
    /// A new, empty folder for this process: in `XDG_RUNTIME_DIR` (memory
    /// under Linux) when there is one, else in the system's temp folder.
    pub fn create() -> std::io::Result<Self> {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|dir| dir.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join(format!("magnetite-private-{}", std::process::id()));
        // Left by an earlier process of the same id that did not end well.
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        crate::files::private_dir(&dir)?;
        Ok(Self(dir))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for PrivateDir {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.0) {
            eprintln!("magnetite: cannot delete {}: {e}", self.0.display());
        }
    }
}

/// Open a private window: start this executable again with `--private`.
/// It is a program of its own from then on; this one only reaps it.
pub fn open_window() {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("magnetite: cannot open a private window: {e}");
            return;
        }
    };
    let child = std::process::Command::new(exe)
        .arg("--private")
        // The test port is this instance's; a second one could not use it.
        .env_remove("MAGNETITE_DEVTOOLS_PORT")
        .stdin(std::process::Stdio::null())
        .spawn();
    match child {
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(e) => eprintln!("magnetite: cannot open a private window: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::PrivateDir;

    /// The only test that makes one: the folder is named after the process,
    /// so two at once in the same test run would be the same folder.
    #[test]
    fn a_private_folder_is_new_the_users_only_and_gone_with_everything_in_it() {
        let dir = PrivateDir::create().unwrap();
        let path = dir.path().to_path_buf();
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .filter(|d| d.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        assert_eq!(path.parent(), Some(base.as_path()));
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 0, "empty");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        std::fs::create_dir_all(path.join("Default/Cache")).unwrap();
        std::fs::write(path.join("Default/Cache/page"), b"seen").unwrap();
        std::fs::write(path.join("state.json"), b"{}").unwrap();
        drop(dir);
        assert!(!path.exists());
    }
}
