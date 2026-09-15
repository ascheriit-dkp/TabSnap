use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_SNAPSHOT_FILE_BYTES: u64 = 65 * 1024 * 1024;
const SNAPSHOT_EXTENSION: &str = "tabsnap";
const MAX_STEM_CHARS: usize = 80;
const MAX_COLLISION_ATTEMPTS: u32 = 10_000;
const MAX_TEMP_ATTEMPTS: u32 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotEntry {
    pub file_name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotLibrary {
    root: PathBuf,
}

impl SnapshotLibrary {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure(&self) -> io::Result<()> {
        if self.root.exists() && !self.root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Snapshot library is not a directory: {}", self.root.display()),
            ));
        }
        fs::create_dir_all(&self.root)
    }

    pub fn list(&self) -> io::Result<Vec<SnapshotEntry>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        if !self.root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Snapshot library is not a directory: {}", self.root.display()),
            ));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }

            let path = entry.path();
            if !has_snapshot_extension(&path) {
                continue;
            }

            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let metadata = entry.metadata()?;
            if metadata.len() > MAX_SNAPSHOT_FILE_BYTES {
                continue;
            }

            entries.push(SnapshotEntry {
                file_name,
                path,
                size: metadata.len(),
                modified: metadata.modified().ok(),
            });
        }

        entries.sort_by(|left, right| {
            left.file_name
                .to_ascii_lowercase()
                .cmp(&right.file_name.to_ascii_lowercase())
                .then_with(|| left.file_name.cmp(&right.file_name))
        });
        Ok(entries)
    }

    pub fn import_file(&self, source: &Path) -> io::Result<SnapshotEntry> {
        self.ensure()?;
        validate_snapshot_source(source)?;

        let source_name = source
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Snapshot file name is not valid Unicode."))?;
        let destination_name = available_snapshot_name(&self.root, source_name)?;
        let destination = self.root.join(destination_name);

        atomic_copy_snapshot(source, &destination)?;
        entry_from_path(destination)
    }

    pub fn write_snapshot(&self, suggested_name: &str, bytes: &[u8]) -> io::Result<SnapshotEntry> {
        if bytes.len() as u64 > MAX_SNAPSHOT_FILE_BYTES {
            return Err(snapshot_too_large_error());
        }

        self.ensure()?;
        let destination_name = available_snapshot_name(&self.root, suggested_name)?;
        let destination = self.root.join(destination_name);
        atomic_write_bytes(&destination, bytes)?;
        entry_from_path(destination)
    }

    pub fn export_file(&self, file_name: &str, destination_dir: &Path) -> io::Result<PathBuf> {
        validate_library_file_name(file_name)?;
        let source = self.root.join(file_name);
        validate_snapshot_source(&source)?;

        if destination_dir.exists() && !destination_dir.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Export destination is not a directory: {}", destination_dir.display()),
            ));
        }
        fs::create_dir_all(destination_dir)?;

        let destination_name = available_snapshot_name(destination_dir, file_name)?;
        let destination = destination_dir.join(destination_name);
        atomic_copy_snapshot(&source, &destination)?;
        Ok(destination)
    }
}

fn entry_from_path(path: PathBuf) -> io::Result<SnapshotEntry> {
    let metadata = fs::metadata(&path)?;
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Snapshot file name is not valid Unicode."))?
        .to_owned();

    Ok(SnapshotEntry {
        file_name,
        path,
        size: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn validate_snapshot_source(path: &Path) -> io::Result<()> {
    if !has_snapshot_extension(path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Snapshot file must use the .tabsnap extension.",
        ));
    }

    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Snapshot source must be a regular file, not a directory or symbolic link.",
        ));
    }
    if metadata.len() > MAX_SNAPSHOT_FILE_BYTES {
        return Err(snapshot_too_large_error());
    }
    Ok(())
}

fn validate_library_file_name(file_name: &str) -> io::Result<()> {
    let path = Path::new(file_name);
    let mut components = path.components();
    let only = components.next();
    if components.next().is_some()
        || !matches!(only, Some(Component::Normal(_)))
        || path.file_name().and_then(OsStr::to_str) != Some(file_name)
        || !has_snapshot_extension(path)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Snapshot library name must be one .tabsnap file name without a path.",
        ));
    }
    Ok(())
}

fn has_snapshot_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(SNAPSHOT_EXTENSION))
}

fn safe_snapshot_name(suggested_name: &str) -> String {
    let lower = suggested_name.to_ascii_lowercase();
    let raw_stem = if lower.ends_with(".tabsnap") {
        &suggested_name[..suggested_name.len() - ".tabsnap".len()]
    } else {
        suggested_name
    };

    let mut stem = String::new();
    for character in raw_stem.chars() {
        let invalid = character.is_control()
            || matches!(character, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*');
        stem.push(if invalid { '_' } else { character });
        if stem.chars().count() >= MAX_STEM_CHARS {
            break;
        }
    }

    let trimmed = stem.trim_matches(|character: char| character == ' ' || character == '.');
    let mut stem = if trimmed.is_empty() {
        "snapshot".to_owned()
    } else {
        trimmed.to_owned()
    };

    let reserved_candidate = stem
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.']);
    if is_windows_reserved_name(reserved_candidate) {
        stem.insert(0, '_');
    }

    format!("{stem}.tabsnap")
}

fn is_windows_reserved_name(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }

    for prefix in ["COM", "LPT"] {
        if let Some(number) = upper.strip_prefix(prefix) {
            if matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") {
                return true;
            }
        }
    }
    false
}

fn available_snapshot_name(directory: &Path, suggested_name: &str) -> io::Result<String> {
    let safe_name = safe_snapshot_name(suggested_name);
    if !directory.join(&safe_name).exists() {
        return Ok(safe_name);
    }

    let stem = safe_name
        .strip_suffix(".tabsnap")
        .expect("safe snapshot names always use .tabsnap");
    for index in 2..=MAX_COLLISION_ATTEMPTS {
        let candidate = format!("{stem} ({index}).tabsnap");
        if !directory.join(&candidate).exists() {
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not choose a collision-free snapshot file name.",
    ))
}

fn atomic_write_bytes(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Snapshot destination has no parent directory.")
    })?;
    let (temp_path, mut temp_file) = create_temp_file(parent)?;

    let result = (|| -> io::Result<()> {
        temp_file.write_all(bytes)?;
        temp_file.sync_all()?;
        drop(temp_file);
        fs::rename(&temp_path, destination)
    })();

    if result.is_err() && temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn atomic_copy_snapshot(source: &Path, destination: &Path) -> io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Snapshot destination has no parent directory.")
    })?;
    let (temp_path, mut temp_file) = create_temp_file(parent)?;

    let result = (|| -> io::Result<()> {
        let source_file = File::open(source)?;
        let mut limited = source_file.take(MAX_SNAPSHOT_FILE_BYTES + 1);
        let copied = io::copy(&mut limited, &mut temp_file)?;
        if copied > MAX_SNAPSHOT_FILE_BYTES {
            return Err(snapshot_too_large_error());
        }
        temp_file.sync_all()?;
        drop(temp_file);
        fs::rename(&temp_path, destination)
    })();

    if result.is_err() && temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn create_temp_file(directory: &Path) -> io::Result<(PathBuf, File)> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();

    for attempt in 0..MAX_TEMP_ATTEMPTS {
        let path = directory.join(format!(
            ".tabsnap-tmp-{}-{nonce}-{attempt}",
            process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not create a unique temporary snapshot file.",
    ))
}

fn snapshot_too_large_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Snapshot exceeds the {MAX_SNAPSHOT_FILE_BYTES}-byte file limit."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("tabsnap-library-{label}-{}-{nonce}", process::id()))
    }

    fn assert_no_temp_files(directory: &Path) {
        let leftovers = fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(".tabsnap-tmp-"))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn lists_only_regular_tabsnap_files_in_deterministic_order() {
        let root = temp_root("list");
        fs::create_dir_all(root.join("nested.tabsnap")).unwrap();
        fs::write(root.join("zeta.tabsnap"), b"z").unwrap();
        fs::write(root.join("Alpha.TABSNAP"), b"a").unwrap();
        fs::write(root.join("notes.txt"), b"ignore").unwrap();

        let entries = SnapshotLibrary::new(&root).list().unwrap();
        let names: Vec<_> = entries.into_iter().map(|entry| entry.file_name).collect();

        assert_eq!(names, vec!["Alpha.TABSNAP", "zeta.tabsnap"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn writes_opaque_bytes_atomically_and_uses_collision_safe_names() {
        let root = temp_root("write");
        let library = SnapshotLibrary::new(&root);
        let first = library.write_snapshot("Work Session", b"encrypted-one").unwrap();
        let second = library.write_snapshot("Work Session.tabsnap", b"encrypted-two").unwrap();

        assert_eq!(first.file_name, "Work Session.tabsnap");
        assert_eq!(second.file_name, "Work Session (2).tabsnap");
        assert_eq!(fs::read(first.path).unwrap(), b"encrypted-one");
        assert_eq!(fs::read(second.path).unwrap(), b"encrypted-two");
        assert_no_temp_files(&root);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sanitizes_windows_unsafe_and_reserved_names() {
        let root = temp_root("safe-name");
        let library = SnapshotLibrary::new(&root);
        let entry = library.write_snapshot("../CON:<bad>*", b"opaque").unwrap();

        assert_eq!(entry.path.parent(), Some(root.as_path()));
        assert!(entry.file_name.ends_with(".tabsnap"));
        assert!(!entry.file_name.contains(['/', '\\', ':', '<', '>', '*']));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imports_without_changing_bytes_and_never_overwrites() {
        let root = temp_root("import-library");
        let source_root = temp_root("import-source");
        fs::create_dir_all(&source_root).unwrap();
        let source = source_root.join("research.tabsnap");
        let opaque = b"TABSNAP opaque encrypted bytes";
        fs::write(&source, opaque).unwrap();
        let library = SnapshotLibrary::new(&root);

        let first = library.import_file(&source).unwrap();
        let second = library.import_file(&source).unwrap();

        assert_eq!(first.file_name, "research.tabsnap");
        assert_eq!(second.file_name, "research (2).tabsnap");
        assert_eq!(fs::read(first.path).unwrap(), opaque);
        assert_eq!(fs::read(second.path).unwrap(), opaque);
        assert_no_temp_files(&root);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(source_root).unwrap();
    }

    #[test]
    fn rejects_wrong_extension_and_oversized_import() {
        let root = temp_root("reject-library");
        let source_root = temp_root("reject-source");
        fs::create_dir_all(&source_root).unwrap();
        let wrong = source_root.join("snapshot.bin");
        fs::write(&wrong, b"opaque").unwrap();
        let oversized = source_root.join("huge.tabsnap");
        let huge = File::create(&oversized).unwrap();
        huge.set_len(MAX_SNAPSHOT_FILE_BYTES + 1).unwrap();
        let library = SnapshotLibrary::new(&root);

        assert_eq!(library.import_file(&wrong).unwrap_err().kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            library.import_file(&oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );

        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
        fs::remove_dir_all(source_root).unwrap();
    }

    #[test]
    fn exports_exact_bytes_with_collision_safe_name() {
        let root = temp_root("export-library");
        let destination = temp_root("export-destination");
        let library = SnapshotLibrary::new(&root);
        let entry = library.write_snapshot("home", b"ciphertext").unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("home.tabsnap"), b"existing").unwrap();

        let exported = library.export_file(&entry.file_name, &destination).unwrap();

        assert_eq!(exported.file_name().unwrap(), "home (2).tabsnap");
        assert_eq!(fs::read(exported).unwrap(), b"ciphertext");
        assert_eq!(fs::read(destination.join("home.tabsnap")).unwrap(), b"existing");
        assert_no_temp_files(&destination);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(destination).unwrap();
    }

    #[test]
    fn export_rejects_path_traversal() {
        let root = temp_root("traversal");
        let destination = temp_root("traversal-destination");
        let library = SnapshotLibrary::new(&root);
        library.write_snapshot("safe", b"opaque").unwrap();

        let error = library.export_file("../safe.tabsnap", &destination).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

        fs::remove_dir_all(root).unwrap();
    }
}
