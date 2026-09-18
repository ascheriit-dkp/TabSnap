use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capture::{
    CaptureFailure, CaptureJobExport, CaptureTargetExport, MAX_CAPTURE_JOB_BYTES,
    MAX_CAPTURE_RESULT_BYTES, valid_job_id,
};
use crate::coordination::{
    BrowserKind, MAX_BROWSER_INSTANCES, valid_browser_version, valid_instance_id,
};

pub const MACHINE_CONTAINER_VERSION: u8 = 1;
pub const MACHINE_SNAPSHOT_EXTENSION: &str = "tabsnap-machine";
pub const MAX_MACHINE_MANIFEST_BYTES: usize = 16 * 1024;
pub const MAX_MACHINE_FILE_BYTES: u64 =
    MAX_CAPTURE_JOB_BYTES as u64 + MAX_MACHINE_MANIFEST_BYTES as u64 + PREFIX_BYTES as u64;

const MAGIC: &[u8; 9] = b"TABSNAPM\0";
const PREFIX_BYTES: usize = MAGIC.len() + 1 + 4;
const MAX_STEM_CHARS: usize = 80;
const MAX_COLLISION_ATTEMPTS: u32 = 10_000;
const MAX_TEMP_ATTEMPTS: u32 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineTargetState {
    Complete {
        payload_offset: u64,
        payload_len: u64,
    },
    Failed {
        reason: CaptureFailure,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineTargetManifest {
    pub instance_id: String,
    pub browser: BrowserKind,
    pub version: Option<String>,
    pub state: MachineTargetState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineManifest {
    pub version: u8,
    pub job_id: String,
    pub stored_unix_ms: u64,
    pub targets: Vec<MachineTargetManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSnapshotEntry {
    pub file_name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub manifest: MachineManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSnapshotLibrary {
    root: PathBuf,
}

impl MachineSnapshotLibrary {
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
                format!(
                    "Machine snapshot library is not a directory: {}",
                    self.root.display()
                ),
            ));
        }
        fs::create_dir_all(&self.root)
    }

    pub fn write_capture_job(
        &self,
        suggested_name: &str,
        capture: &CaptureJobExport<'_>,
    ) -> io::Result<MachineSnapshotEntry> {
        self.ensure()?;
        let stored_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_millis()
            .try_into()
            .map_err(|_| io::Error::other("System time exceeds the machine manifest range."))?;
        let encoded = encode_manifest(capture, stored_unix_ms)?;
        let payload_bytes = capture
            .targets
            .iter()
            .filter_map(|target| match target {
                CaptureTargetExport::Complete { encrypted, .. } => Some(encrypted.len() as u64),
                CaptureTargetExport::Failed { .. } => None,
            })
            .try_fold(0_u64, |total, bytes| total.checked_add(bytes))
            .ok_or_else(|| invalid_data("Machine snapshot payload size overflow."))?;
        let total_bytes = PREFIX_BYTES as u64 + encoded.len() as u64 + payload_bytes;
        if total_bytes > MAX_MACHINE_FILE_BYTES {
            return Err(machine_too_large_error());
        }

        let destination_name = available_machine_name(&self.root, suggested_name)?;
        let destination = self.root.join(destination_name);
        atomic_write_capture(&destination, &encoded, capture)?;
        inspect_machine_path(destination)
    }

    pub fn inspect(&self, file_name: &str) -> io::Result<MachineSnapshotEntry> {
        validate_library_file_name(file_name)?;
        inspect_machine_path(self.root.join(file_name))
    }

    pub fn read_encrypted_payload(
        &self,
        file_name: &str,
        instance_id: &str,
    ) -> io::Result<Vec<u8>> {
        if !valid_instance_id(instance_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Browser instance id is invalid.",
            ));
        }
        let entry = self.inspect(file_name)?;
        let target = entry
            .manifest
            .targets
            .iter()
            .find(|target| target.instance_id == instance_id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Browser instance is not in the machine snapshot.",
                )
            })?;
        let (payload_offset, payload_len) = match target.state {
            MachineTargetState::Complete {
                payload_offset,
                payload_len,
            } => (payload_offset, payload_len),
            MachineTargetState::Failed { .. } => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Browser instance has no encrypted payload in this machine snapshot.",
                ));
            }
        };

        let payload_len: usize = payload_len
            .try_into()
            .map_err(|_| invalid_data("Encrypted payload length is unsupported."))?;
        if payload_len > MAX_CAPTURE_RESULT_BYTES {
            return Err(machine_too_large_error());
        }
        let mut file = File::open(&entry.path)?;
        file.seek(SeekFrom::Start(payload_offset))?;
        let mut payload = vec![0_u8; payload_len];
        file.read_exact(&mut payload)?;
        Ok(payload)
    }
}

fn encode_manifest(capture: &CaptureJobExport<'_>, stored_unix_ms: u64) -> io::Result<Vec<u8>> {
    if !valid_job_id(capture.job_id) {
        return Err(invalid_data("Capture job id is invalid."));
    }
    if capture.targets.is_empty() || capture.targets.len() > MAX_BROWSER_INSTANCES {
        return Err(invalid_data("Machine snapshot target count is invalid."));
    }

    let mut seen = HashSet::new();
    let mut payload_bytes = 0_usize;
    let mut complete_targets = 0_usize;
    let mut output = Vec::with_capacity(1024);
    output.extend_from_slice(&stored_unix_ms.to_be_bytes());
    output.extend_from_slice(&decode_hex_128(capture.job_id)?);
    let target_count: u16 = capture
        .targets
        .len()
        .try_into()
        .map_err(|_| invalid_data("Machine snapshot target count is invalid."))?;
    output.extend_from_slice(&target_count.to_be_bytes());

    for target in &capture.targets {
        let instance = target.instance();
        if !seen.insert(instance.instance_id.as_str()) {
            return Err(invalid_data(
                "Machine snapshot has duplicate browser instance ids.",
            ));
        }
        output.extend_from_slice(&decode_hex_128(&instance.instance_id)?);
        output.push(browser_code(instance.browser));

        match instance.version.as_deref() {
            Some(version) => {
                if !valid_browser_version(version) {
                    return Err(invalid_data("Browser version metadata is invalid."));
                }
                let length: u8 = version
                    .len()
                    .try_into()
                    .map_err(|_| invalid_data("Browser version metadata is too large."))?;
                output.push(length);
                output.extend_from_slice(version.as_bytes());
            }
            None => output.push(0),
        }

        match target {
            CaptureTargetExport::Complete { encrypted, .. } => {
                if encrypted.is_empty() || encrypted.len() > MAX_CAPTURE_RESULT_BYTES {
                    return Err(invalid_data("Encrypted browser payload size is invalid."));
                }
                complete_targets += 1;
                payload_bytes = payload_bytes
                    .checked_add(encrypted.len())
                    .ok_or_else(|| invalid_data("Machine snapshot payload size overflow."))?;
                if payload_bytes > MAX_CAPTURE_JOB_BYTES {
                    return Err(machine_too_large_error());
                }
                output.push(1);
                let length: u32 = encrypted
                    .len()
                    .try_into()
                    .map_err(|_| machine_too_large_error())?;
                output.extend_from_slice(&length.to_be_bytes());
            }
            CaptureTargetExport::Failed { reason, .. } => {
                output.push(2);
                output.push(failure_code(*reason));
            }
        }
    }

    if complete_targets == 0 {
        return Err(invalid_data(
            "Machine snapshot requires at least one encrypted browser payload.",
        ));
    }
    if output.is_empty() || output.len() > MAX_MACHINE_MANIFEST_BYTES {
        return Err(invalid_data("Machine snapshot manifest is too large."));
    }
    Ok(output)
}

fn inspect_machine_path(path: PathBuf) -> io::Result<MachineSnapshotEntry> {
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Machine snapshot source must be a regular file, not a directory or symbolic link.",
        ));
    }
    if !has_machine_extension(&path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Machine snapshot file must use the .tabsnap-machine extension.",
        ));
    }
    if metadata.len() > MAX_MACHINE_FILE_BYTES {
        return Err(machine_too_large_error());
    }
    if metadata.len() < PREFIX_BYTES as u64 {
        return Err(invalid_data("Machine snapshot is truncated."));
    }

    let mut file = File::open(&path)?;
    let mut prefix = [0_u8; PREFIX_BYTES];
    file.read_exact(&mut prefix)?;
    if &prefix[..MAGIC.len()] != MAGIC {
        return Err(invalid_data("Machine snapshot magic is invalid."));
    }
    if prefix[MAGIC.len()] != MACHINE_CONTAINER_VERSION {
        return Err(invalid_data("Unsupported machine snapshot version."));
    }
    let manifest_len = u32::from_be_bytes(
        prefix[MAGIC.len() + 1..PREFIX_BYTES]
            .try_into()
            .expect("machine prefix stores one manifest length"),
    ) as usize;
    if manifest_len == 0 || manifest_len > MAX_MACHINE_MANIFEST_BYTES {
        return Err(invalid_data("Machine snapshot manifest length is invalid."));
    }
    if PREFIX_BYTES as u64 + manifest_len as u64 > metadata.len() {
        return Err(invalid_data("Machine snapshot manifest is truncated."));
    }

    let mut manifest_bytes = vec![0_u8; manifest_len];
    file.read_exact(&mut manifest_bytes)?;
    let payload_start = PREFIX_BYTES as u64 + manifest_len as u64;
    let manifest = parse_manifest(&manifest_bytes, payload_start)?;
    let payload_bytes = manifest
        .targets
        .iter()
        .filter_map(|target| match target.state {
            MachineTargetState::Complete { payload_len, .. } => Some(payload_len),
            MachineTargetState::Failed { .. } => None,
        })
        .try_fold(0_u64, |total, bytes| total.checked_add(bytes))
        .ok_or_else(|| invalid_data("Machine snapshot payload size overflow."))?;
    if payload_bytes > MAX_CAPTURE_JOB_BYTES as u64 {
        return Err(machine_too_large_error());
    }
    let expected_size = payload_start
        .checked_add(payload_bytes)
        .ok_or_else(|| invalid_data("Machine snapshot size overflow."))?;
    if expected_size != metadata.len() {
        return Err(invalid_data(
            "Machine snapshot payload lengths do not match the file size.",
        ));
    }

    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| invalid_data("Machine snapshot file name is not valid Unicode."))?
        .to_owned();
    Ok(MachineSnapshotEntry {
        file_name,
        path,
        size: metadata.len(),
        modified: metadata.modified().ok(),
        manifest,
    })
}

fn parse_manifest(bytes: &[u8], payload_start: u64) -> io::Result<MachineManifest> {
    let mut cursor = SliceCursor::new(bytes);
    let stored_unix_ms = cursor.u64()?;
    let job_id = encode_hex_128(cursor.array_16()?);
    let target_count = cursor.u16()? as usize;
    if target_count == 0 || target_count > MAX_BROWSER_INSTANCES {
        return Err(invalid_data("Machine snapshot target count is invalid."));
    }

    let mut seen = HashSet::new();
    let mut payload_offset = payload_start;
    let mut payload_total = 0_u64;
    let mut targets = Vec::with_capacity(target_count);
    for _ in 0..target_count {
        let instance_id = encode_hex_128(cursor.array_16()?);
        if !seen.insert(instance_id.clone()) {
            return Err(invalid_data(
                "Machine snapshot has duplicate browser instance ids.",
            ));
        }
        let browser = parse_browser_code(cursor.u8()?)?;
        let version_len = cursor.u8()? as usize;
        let version = if version_len == 0 {
            None
        } else {
            let version_bytes = cursor.take(version_len)?;
            let value = std::str::from_utf8(version_bytes)
                .map_err(|_| invalid_data("Browser version metadata is not UTF-8."))?;
            if !valid_browser_version(value) {
                return Err(invalid_data("Browser version metadata is invalid."));
            }
            Some(value.to_owned())
        };

        let state = match cursor.u8()? {
            1 => {
                let payload_len = cursor.u32()? as u64;
                if payload_len == 0 || payload_len > MAX_CAPTURE_RESULT_BYTES as u64 {
                    return Err(invalid_data("Encrypted browser payload size is invalid."));
                }
                payload_total = payload_total
                    .checked_add(payload_len)
                    .ok_or_else(|| invalid_data("Machine snapshot payload size overflow."))?;
                if payload_total > MAX_CAPTURE_JOB_BYTES as u64 {
                    return Err(machine_too_large_error());
                }
                let current_offset = payload_offset;
                payload_offset = payload_offset
                    .checked_add(payload_len)
                    .ok_or_else(|| invalid_data("Machine snapshot payload offset overflow."))?;
                MachineTargetState::Complete {
                    payload_offset: current_offset,
                    payload_len,
                }
            }
            2 => MachineTargetState::Failed {
                reason: parse_failure_code(cursor.u8()?)?,
            },
            _ => return Err(invalid_data("Machine snapshot target state is invalid.")),
        };

        targets.push(MachineTargetManifest {
            instance_id,
            browser,
            version,
            state,
        });
    }
    if !cursor.finished() {
        return Err(invalid_data("Machine snapshot manifest has trailing data."));
    }

    Ok(MachineManifest {
        version: MACHINE_CONTAINER_VERSION,
        job_id,
        stored_unix_ms,
        targets,
    })
}

fn atomic_write_capture(
    destination: &Path,
    manifest: &[u8],
    capture: &CaptureJobExport<'_>,
) -> io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Machine snapshot destination has no parent directory.",
        )
    })?;
    let (temp_path, mut temp_file) = create_temp_file(parent)?;

    let result = (|| -> io::Result<()> {
        temp_file.write_all(MAGIC)?;
        temp_file.write_all(&[MACHINE_CONTAINER_VERSION])?;
        let manifest_len: u32 = manifest
            .len()
            .try_into()
            .map_err(|_| invalid_data("Machine snapshot manifest is too large."))?;
        temp_file.write_all(&manifest_len.to_be_bytes())?;
        temp_file.write_all(manifest)?;
        for target in &capture.targets {
            if let CaptureTargetExport::Complete { encrypted, .. } = target {
                temp_file.write_all(encrypted)?;
            }
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
            ".tabsnap-machine-tmp-{}-{nonce}-{attempt}",
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
        "Could not create a unique machine snapshot temporary file.",
    ))
}

fn available_machine_name(directory: &Path, suggested_name: &str) -> io::Result<String> {
    let safe_name = safe_machine_name(suggested_name);
    if !directory.join(&safe_name).exists() {
        return Ok(safe_name);
    }
    let suffix = format!(".{MACHINE_SNAPSHOT_EXTENSION}");
    let stem = safe_name
        .strip_suffix(&suffix)
        .expect("safe machine snapshot names always use the machine extension");
    for index in 2..=MAX_COLLISION_ATTEMPTS {
        let candidate = format!("{stem} ({index}){suffix}");
        if !directory.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not choose a collision-free machine snapshot file name.",
    ))
}

fn safe_machine_name(suggested_name: &str) -> String {
    let suffix = format!(".{MACHINE_SNAPSHOT_EXTENSION}");
    let lower = suggested_name.to_ascii_lowercase();
    let raw_stem = if lower.ends_with(&suffix) {
        &suggested_name[..suggested_name.len() - suffix.len()]
    } else {
        suggested_name
    };
    let mut stem = String::new();
    for character in raw_stem.chars() {
        let invalid = character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            );
        stem.push(if invalid { '_' } else { character });
        if stem.chars().count() >= MAX_STEM_CHARS {
            break;
        }
    }
    let trimmed = stem.trim_matches(|character: char| character == ' ' || character == '.');
    let mut stem = if trimmed.is_empty() {
        "machine".to_owned()
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
    format!("{stem}{suffix}")
}

fn is_windows_reserved_name(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }

    for prefix in ["COM", "LPT"] {
        if upper.strip_prefix(prefix).is_some_and(|number| {
            matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        }) {
            return true;
        }
    }
    false
}

fn validate_library_file_name(file_name: &str) -> io::Result<()> {
    let path = Path::new(file_name);
    let mut components = path.components();
    let only = components.next();
    if components.next().is_some()
        || !matches!(only, Some(Component::Normal(_)))
        || path.file_name().and_then(OsStr::to_str) != Some(file_name)
        || !has_machine_extension(path)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Machine snapshot library name must be one .tabsnap-machine file name without a path.",
        ));
    }
    Ok(())
}

fn has_machine_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(MACHINE_SNAPSHOT_EXTENSION))
}

fn browser_code(browser: BrowserKind) -> u8 {
    match browser {
        BrowserKind::Chrome => 1,
        BrowserKind::Edge => 2,
        BrowserKind::Firefox => 3,
    }
}

fn parse_browser_code(value: u8) -> io::Result<BrowserKind> {
    match value {
        1 => Ok(BrowserKind::Chrome),
        2 => Ok(BrowserKind::Edge),
        3 => Ok(BrowserKind::Firefox),
        _ => Err(invalid_data("Machine snapshot browser kind is invalid.")),
    }
}

fn failure_code(reason: CaptureFailure) -> u8 {
    match reason {
        CaptureFailure::PasswordRequired => 1,
        CaptureFailure::CaptureFailed => 2,
        CaptureFailure::EncryptionFailed => 3,
        CaptureFailure::TimedOut => 4,
    }
}

fn parse_failure_code(value: u8) -> io::Result<CaptureFailure> {
    match value {
        1 => Ok(CaptureFailure::PasswordRequired),
        2 => Ok(CaptureFailure::CaptureFailed),
        3 => Ok(CaptureFailure::EncryptionFailed),
        4 => Ok(CaptureFailure::TimedOut),
        _ => Err(invalid_data("Machine snapshot failure reason is invalid.")),
    }
}

fn decode_hex_128(value: &str) -> io::Result<[u8; 16]> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(invalid_data("128-bit identifier is invalid."));
    }
    let mut output = [0_u8; 16];
    for (index, byte) in output.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| invalid_data("128-bit identifier is invalid."))?;
    }
    Ok(output)
}

fn encode_hex_128(bytes: [u8; 16]) -> String {
    let mut output = String::with_capacity(32);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn machine_too_large_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("Machine snapshot exceeds the {MAX_MACHINE_FILE_BYTES}-byte file limit."),
    )
}

struct SliceCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SliceCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> io::Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| invalid_data("Machine snapshot manifest offset overflow."))?;
        if end > self.bytes.len() {
            return Err(invalid_data("Machine snapshot manifest is truncated."));
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().expect("cursor returned two bytes"),
        ))
    }

    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .expect("cursor returned four bytes"),
        ))
    }

    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_be_bytes(
            self.take(8)?
                .try_into()
                .expect("cursor returned eight bytes"),
        ))
    }

    fn array_16(&mut self) -> io::Result<[u8; 16]> {
        Ok(self
            .take(16)?
            .try_into()
            .expect("cursor returned sixteen bytes"))
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureJobStore;
    use crate::coordination::{BrowserCapability, BrowserInstance};
    use std::time::{Duration, Instant};

    const JOB_ID: &str = "11111111111111111111111111111111";
    const CHROME_ID: &str = "0123456789abcdef0123456789abcdef";
    const FIREFOX_ID: &str = "fedcba9876543210fedcba9876543210";

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("tabsnap-machine-{label}-{}-{nonce}", process::id()))
    }

    fn capture_store() -> CaptureJobStore {
        CaptureJobStore::new(
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(120),
            64,
            128,
        )
    }

    fn browser(instance_id: &str, kind: BrowserKind, version: Option<&str>) -> BrowserInstance {
        BrowserInstance {
            instance_id: instance_id.to_owned(),
            browser: kind,
            version: version.map(str::to_owned),
            capabilities: vec![BrowserCapability::Capture, BrowserCapability::Restore],
        }
    }

    fn assert_no_temp_files(directory: &Path) {
        let leftovers = fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".tabsnap-machine-tmp-")
            })
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn writes_and_strictly_reads_versioned_machine_container() {
        let start = Instant::now();
        let mut jobs = capture_store();
        jobs.create(
            JOB_ID.to_owned(),
            vec![
                browser(CHROME_ID, BrowserKind::Chrome, Some("140.0.0.0")),
                browser(FIREFOX_ID, BrowserKind::Firefox, Some("143.0")),
            ],
            start,
        )
        .unwrap();
        jobs.submit_result(JOB_ID, CHROME_ID, vec![1, 2, 3, 4, 255], start)
            .unwrap();
        jobs.submit_failure(JOB_ID, FIREFOX_ID, CaptureFailure::PasswordRequired, start)
            .unwrap();
        let export = jobs.terminal_export(JOB_ID, start).unwrap();

        let root = temp_root("roundtrip");
        let library = MachineSnapshotLibrary::new(&root);
        let entry = library.write_capture_job("Work machine", &export).unwrap();

        assert_eq!(entry.file_name, "Work machine.tabsnap-machine");
        assert_eq!(entry.manifest.version, 1);
        assert_eq!(entry.manifest.job_id, JOB_ID);
        assert_eq!(entry.manifest.targets.len(), 2);
        assert_eq!(entry.manifest.targets[0].instance_id, CHROME_ID);
        assert_eq!(entry.manifest.targets[0].browser, BrowserKind::Chrome);
        assert_eq!(
            entry.manifest.targets[0].version.as_deref(),
            Some("140.0.0.0")
        );
        assert!(matches!(
            entry.manifest.targets[0].state,
            MachineTargetState::Complete { payload_len: 5, .. }
        ));
        assert!(matches!(
            entry.manifest.targets[1].state,
            MachineTargetState::Failed {
                reason: CaptureFailure::PasswordRequired
            }
        ));
        assert_eq!(
            library
                .read_encrypted_payload(&entry.file_name, CHROME_ID)
                .unwrap(),
            vec![1, 2, 3, 4, 255]
        );
        assert_no_temp_files(&root);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn uses_collision_safe_names_without_overwriting() {
        let start = Instant::now();
        let mut jobs = capture_store();
        jobs.create(
            JOB_ID.to_owned(),
            vec![browser(CHROME_ID, BrowserKind::Chrome, None)],
            start,
        )
        .unwrap();
        jobs.submit_result(JOB_ID, CHROME_ID, vec![7, 8], start)
            .unwrap();
        let export = jobs.terminal_export(JOB_ID, start).unwrap();
        let root = temp_root("collision");
        let library = MachineSnapshotLibrary::new(&root);

        let first = library.write_capture_job("machine", &export).unwrap();
        let second = library
            .write_capture_job("machine.tabsnap-machine", &export)
            .unwrap();
        assert_eq!(first.file_name, "machine.tabsnap-machine");
        assert_eq!(second.file_name, "machine (2).tabsnap-machine");
        assert_ne!(first.path, second.path);
        assert_no_temp_files(&root);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sanitizes_windows_reserved_machine_names() {
        for reserved in [
            "CON", "prn", "AUX.txt", "nul", "COM1", "com9.backup", "LPT1", "lpt9.log",
        ] {
            let safe = safe_machine_name(reserved);
            assert!(safe.starts_with('_'), "{reserved} -> {safe}");
            assert!(safe.ends_with(".tabsnap-machine"));
        }

        assert_eq!(
            safe_machine_name("COM10"),
            "COM10.tabsnap-machine",
            "COM10 is not a reserved Windows device name"
        );
        assert_eq!(
            safe_machine_name("LPT0"),
            "LPT0.tabsnap-machine",
            "LPT0 is not a reserved Windows device name"
        );
    }

    #[test]
    fn rejects_corrupt_magic_version_truncation_and_trailing_payload() {
        let start = Instant::now();
        let mut jobs = capture_store();
        jobs.create(
            JOB_ID.to_owned(),
            vec![browser(CHROME_ID, BrowserKind::Chrome, None)],
            start,
        )
        .unwrap();
        jobs.submit_result(JOB_ID, CHROME_ID, vec![1, 2, 3], start)
            .unwrap();
        let export = jobs.terminal_export(JOB_ID, start).unwrap();
        let root = temp_root("corrupt");
        let library = MachineSnapshotLibrary::new(&root);
        let valid = library.write_capture_job("valid", &export).unwrap();
        let original = fs::read(&valid.path).unwrap();

        let mut bad_magic = original.clone();
        bad_magic[0] ^= 0xff;
        let bad_magic_path = root.join("bad-magic.tabsnap-machine");
        fs::write(&bad_magic_path, bad_magic).unwrap();
        assert_eq!(
            inspect_machine_path(bad_magic_path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let mut bad_version = original.clone();
        bad_version[MAGIC.len()] = 99;
        let bad_version_path = root.join("bad-version.tabsnap-machine");
        fs::write(&bad_version_path, bad_version).unwrap();
        assert_eq!(
            inspect_machine_path(bad_version_path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let truncated_path = root.join("truncated.tabsnap-machine");
        fs::write(&truncated_path, &original[..original.len() - 1]).unwrap();
        assert_eq!(
            inspect_machine_path(truncated_path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let trailing_path = root.join("trailing.tabsnap-machine");
        let mut trailing = original;
        trailing.push(0);
        fs::write(&trailing_path, trailing).unwrap();
        assert_eq!(
            inspect_machine_path(trailing_path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_oversize_sparse_machine_file_before_allocation() {
        let root = temp_root("oversize");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("oversize.tabsnap-machine");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_MACHINE_FILE_BYTES + 1).unwrap();
        drop(file);

        assert!(inspect_machine_path(path).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
