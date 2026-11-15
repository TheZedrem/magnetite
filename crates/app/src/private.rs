//! Private windows: a Magnetite of its own (`--private`), started by the one
//! in use. It runs on a new, empty data folder that is deleted when it quits,
//! so nothing of it stays and nothing of the user's profile is in it: no
//! logins, history, extensions, sync account or saved tabs. Under Linux the
//! folder is in memory (`XDG_RUNTIME_DIR`).

use std::path::{Path, PathBuf};

/// The data folder of a private instance, deleted when dropped.
pub struct PrivateDir(PathBuf);

impl PrivateDir {
    /// A new, empty folder for this process.
    pub fn create() -> std::io::Result<Self> {
        let dir = folder(std::process::id());
        // Left by an earlier process of the same id that did not end well.
        // `remove_dir_all` takes a link away without following it.
        if dir.symlink_metadata().is_ok() {
            std::fs::remove_dir_all(&dir)?;
        }
        // Made here and now, or not at all: in a shared temp folder, someone
        // else could have put a link to their folder in its place.
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(&dir)?;
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

/// The data folder of private instance `pid`: in `XDG_RUNTIME_DIR` (memory
/// under Linux) when there is one, else in the system's temp folder.
fn folder(pid: u32) -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(std::env::temp_dir)
        .join(format!("magnetite-private-{pid}"))
}

/// Open a private window: start this executable again with `--private`.
/// It is a program of its own from then on; this one waits for it to end
/// and then deletes its folder, in case it could not: a quit that hung ends
/// without cleaning up (`Engine::quit`), and its state may still have been
/// saving while it deleted the folder.
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
            std::thread::spawn(move || {
                let _ = child.wait();
                let dir = folder(child.id());
                if dir.symlink_metadata().is_ok()
                    && let Err(e) = std::fs::remove_dir_all(&dir)
                {
                    eprintln!("magnetite: cannot delete {}: {e}", dir.display());
                }
            });
        }
        Err(e) => eprintln!("magnetite: cannot open a private window: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::PrivateDir;

    /// The only test that makes one: the folder is named after the process,
    /// so two at once in the same test run would be the same folder. A link
    /// left in its place is taken away, not followed: where it points stays.
    #[test]
    fn a_private_folder_is_new_the_users_only_and_gone_with_everything_in_it() {
        #[cfg(unix)]
        let target = {
            let target = crate::files::test_dir("private-link");
            std::fs::create_dir(&target).unwrap();
            std::fs::write(target.join("theirs"), b"keep").unwrap();
            std::os::unix::fs::symlink(&target, super::folder(std::process::id())).unwrap();
            target
        };
        let dir = PrivateDir::create().unwrap();
        let path = dir.path().to_path_buf();
        assert!(
            path.symlink_metadata().unwrap().is_dir(),
            "a folder, not a link"
        );
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
        #[cfg(unix)]
        {
            assert_eq!(std::fs::read(target.join("theirs")).unwrap(), b"keep");
            std::fs::remove_dir_all(&target).unwrap();
        }
    }
}
