use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SNAPSHOTS_DIR_NAME: &str = "snapshots";
pub const CONFIG_FILE_NAME: &str = "tabsnap-companion.conf";
const CONFIG_VERSION: &str = "1";
const LOCAL_APP_DIR_NAME: &str = "TabSnap";

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
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Executable has no parent directory.",
                )
            })?
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

    pub fn config_path(&self) -> PathBuf {
        self.root_dir.join(CONFIG_FILE_NAME)
    }

    pub fn is_initialized(&self) -> bool {
        self.snapshots_dir.is_dir()
    }

    pub fn ensure_directories(&self) -> io::Result<()> {
        fs::create_dir_all(&self.snapshots_dir)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageMode {
    Portable,
    Local,
    Custom(PathBuf),
}

impl StorageMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Local => "local",
            Self::Custom(_) => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveHint {
    Removable,
    Fixed,
    Remote,
    Optical,
    RamDisk,
    Unknown,
}

impl DriveHint {
    pub fn name(self) -> &'static str {
        match self {
            Self::Removable => "removable",
            Self::Fixed => "fixed",
            Self::Remote => "remote",
            Self::Optical => "optical",
            Self::RamDisk => "ram-disk",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStorage {
    pub mode: StorageMode,
    pub snapshots_dir: PathBuf,
    pub drive_hint: DriveHint,
}

pub fn load_storage_mode(layout: &PortableLayout) -> io::Result<StorageMode> {
    let config_path = layout.config_path();
    let contents = match fs::read_to_string(&config_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(StorageMode::Portable),
        Err(error) => return Err(error),
    };

    parse_storage_config(&contents)
}

pub fn save_storage_mode(layout: &PortableLayout, mode: &StorageMode) -> io::Result<()> {
    let contents = serialize_storage_config(mode)?;
    fs::write(layout.config_path(), contents)
}

pub fn resolve_storage(layout: &PortableLayout, mode: &StorageMode) -> io::Result<ResolvedStorage> {
    let local_base = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    resolve_storage_with_local_base(layout, mode, local_base.as_deref())
}

pub fn activate_storage(layout: &PortableLayout, mode: StorageMode) -> io::Result<ResolvedStorage> {
    let resolved = resolve_storage(layout, &mode)?;
    validate_storage_dir(&resolved.snapshots_dir)?;
    save_storage_mode(layout, &mode)?;
    Ok(resolved)
}

pub fn validate_storage_dir(path: &Path) -> io::Result<()> {
    if path.exists() && !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Storage path is not a directory: {}", path.display()),
        ));
    }

    fs::create_dir_all(path)?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let probe_path = path.join(format!(".tabsnap-write-test-{}-{nonce}", process::id()));

    let write_result = (|| -> io::Result<()> {
        let mut probe = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe_path)?;
        probe.write_all(b"tabsnap")?;
        probe.sync_all()
    })();

    let cleanup_result = if probe_path.exists() {
        fs::remove_file(&probe_path)
    } else {
        Ok(())
    };

    write_result?;
    cleanup_result?;
    Ok(())
}

fn resolve_storage_with_local_base(
    layout: &PortableLayout,
    mode: &StorageMode,
    local_base: Option<&Path>,
) -> io::Result<ResolvedStorage> {
    let snapshots_dir = match mode {
        StorageMode::Portable => layout.snapshots_dir().to_path_buf(),
        StorageMode::Local => {
            let base = local_base
                .filter(|path| !path.as_os_str().is_empty())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "LOCALAPPDATA is unavailable; local storage cannot be resolved.",
                    )
                })?;
            base.join(LOCAL_APP_DIR_NAME).join(SNAPSHOTS_DIR_NAME)
        }
        StorageMode::Custom(path) => {
            if !path.is_absolute() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Custom storage path must be absolute.",
                ));
            }
            path.clone()
        }
    };

    Ok(ResolvedStorage {
        mode: mode.clone(),
        drive_hint: drive_hint(&snapshots_dir),
        snapshots_dir,
    })
}

fn parse_storage_config(contents: &str) -> io::Result<StorageMode> {
    let mut version = None;
    let mut mode = None;
    let mut custom_path = None;

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line.split_once('=').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid storage configuration line.",
            )
        })?;

        match key {
            "version" => version = Some(value),
            "mode" => mode = Some(value),
            "custom_path" => custom_path = Some(value),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unknown storage configuration key: {key}."),
                ));
            }
        }
    }

    if version != Some(CONFIG_VERSION) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unsupported storage configuration version.",
        ));
    }

    match mode {
        Some("portable") => Ok(StorageMode::Portable),
        Some("local") => Ok(StorageMode::Local),
        Some("custom") => {
            let path = custom_path
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Custom storage configuration is missing custom_path.",
                    )
                })?;
            Ok(StorageMode::Custom(PathBuf::from(path)))
        }
        Some(other) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown storage mode: {other}."),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Storage configuration is missing mode.",
        )),
    }
}

fn serialize_storage_config(mode: &StorageMode) -> io::Result<String> {
    let mut output = format!("version={CONFIG_VERSION}\nmode={}\n", mode.name());

    if let StorageMode::Custom(path) = mode {
        let value = path.to_string_lossy();
        if value.contains('\n') || value.contains('\r') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Custom storage path contains an unsupported line break.",
            ));
        }
        output.push_str("custom_path=");
        output.push_str(&value);
        output.push('\n');
    }

    Ok(output)
}

#[cfg(windows)]
fn drive_hint(path: &Path) -> DriveHint {
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        #[link_name = "GetDriveTypeW"]
        fn get_drive_type_w(root_path_name: *const u16) -> u32;
    }

    let Some(root) = path.ancestors().last() else {
        return DriveHint::Unknown;
    };
    if !root.is_absolute() {
        return DriveHint::Unknown;
    }

    let mut wide: Vec<u16> = root.as_os_str().encode_wide().collect();
    wide.push(0);

    let kind = unsafe { get_drive_type_w(wide.as_ptr()) };
    match kind {
        2 => DriveHint::Removable,
        3 => DriveHint::Fixed,
        4 => DriveHint::Remote,
        5 => DriveHint::Optical,
        6 => DriveHint::RamDisk,
        _ => DriveHint::Unknown,
    }
}

#[cfg(not(windows))]
fn drive_hint(_path: &Path) -> DriveHint {
    DriveHint::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("tabsnap-{label}-{}-{nonce}", process::id()))
    }

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
    fn defaults_to_portable_when_config_is_absent() {
        let root = temp_root("default");
        let layout =
            PortableLayout::from_executable_path(root.join("tabsnap-companion.exe")).unwrap();

        assert_eq!(load_storage_mode(&layout).unwrap(), StorageMode::Portable);
    }

    #[test]
    fn storage_config_round_trips_all_modes() {
        let custom = temp_root("custom");
        let cases = [
            StorageMode::Portable,
            StorageMode::Local,
            StorageMode::Custom(custom),
        ];

        for mode in cases {
            let config = serialize_storage_config(&mode).unwrap();
            assert_eq!(parse_storage_config(&config).unwrap(), mode);
        }
    }

    #[test]
    fn resolves_local_storage_under_local_app_data() {
        let root = temp_root("local-root");
        let local = temp_root("local-app-data");
        let layout =
            PortableLayout::from_executable_path(root.join("tabsnap-companion.exe")).unwrap();
        let resolved =
            resolve_storage_with_local_base(&layout, &StorageMode::Local, Some(&local)).unwrap();

        assert_eq!(
            resolved.snapshots_dir,
            local.join(LOCAL_APP_DIR_NAME).join(SNAPSHOTS_DIR_NAME),
        );
    }

    #[test]
    fn rejects_relative_custom_storage() {
        let root = temp_root("relative");
        let layout =
            PortableLayout::from_executable_path(root.join("tabsnap-companion.exe")).unwrap();
        let error = resolve_storage_with_local_base(
            &layout,
            &StorageMode::Custom(PathBuf::from("relative-library")),
            None,
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn validates_writable_storage_and_removes_probe() {
        let storage = temp_root("write-probe");

        validate_storage_dir(&storage).unwrap();
        assert!(storage.is_dir());
        assert_eq!(fs::read_dir(&storage).unwrap().count(), 0);

        fs::remove_dir_all(storage).unwrap();
    }

    #[test]
    fn activate_custom_storage_persists_selection() {
        let root = temp_root("activate-root");
        fs::create_dir_all(&root).unwrap();
        let custom = temp_root("activate-custom");
        let layout =
            PortableLayout::from_executable_path(root.join("tabsnap-companion.exe")).unwrap();
        let mode = StorageMode::Custom(custom.clone());

        let resolved = activate_storage(&layout, mode.clone()).unwrap();

        assert_eq!(resolved.mode, mode);
        assert_eq!(resolved.snapshots_dir, custom);
        assert_eq!(load_storage_mode(&layout).unwrap(), resolved.mode);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(resolved.snapshots_dir).unwrap();
    }
}
