use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const SNAPSHOTS_DIR_NAME: &str = "snapshots";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableLayout {
    executable_path: PathBuf,
    root_dir: PathBuf,
    snapshots_dir: PathBuf,
}

impl PortableLayout {
    pub fn discover() -> io::Result<Self> {
        Self::from_executable_path(env::current_exe()?)
    }

    pub fn from_executable_path(executable_path: impl Into<PathBuf>) -> io::Result<Self> {
        let executable_path = executable_path.into();
        let root_dir = executable_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Executable has no parent directory."))?
            .to_path_buf();
        let snapshots_dir = root_dir.join(SNAPSHOTS_DIR_NAME);

        Ok(Self {
            executable_path,
            root_dir,
            snapshots_dir,
        })
    }

    pub fn executable_path(&self) -> &Path {
        &self.executable_path
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn snapshots_dir(&self) -> &Path {
        &self.snapshots_dir
    }

    pub fn is_initialized(&self) -> bool {
        self.snapshots_dir.is_dir()
    }

    pub fn ensure_directories(&self) -> io::Result<()> {
        fs::create_dir_all(&self.snapshots_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn derives_portable_storage_next_to_executable() {
        let executable = PathBuf::from("/media/TABSNAP/TabSnap/tabsnap-companion.exe");
        let layout = PortableLayout::from_executable_path(executable.clone()).unwrap();

        assert_eq!(layout.executable_path(), executable);
        assert_eq!(layout.root_dir(), Path::new("/media/TABSNAP/TabSnap"));
        assert_eq!(
            layout.snapshots_dir(),
            Path::new("/media/TABSNAP/TabSnap/snapshots"),
        );
    }

    #[test]
    fn rejects_executable_without_parent_directory() {
        let error = PortableLayout::from_executable_path("tabsnap-companion.exe").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn initializes_snapshot_directory_without_other_state() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("tabsnap-companion-{nonce}"));
        let executable = root.join("tabsnap-companion.exe");
        let layout = PortableLayout::from_executable_path(executable).unwrap();

        assert!(!layout.is_initialized());
        layout.ensure_directories().unwrap();
        assert!(layout.is_initialized());
        assert!(layout.snapshots_dir().is_dir());

        fs::remove_dir_all(root).unwrap();
    }
}
