from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{path}: expected one anchor, found {count}: {old[:100]!r}')
    file.write_text(text.replace(old, new, 1))


# coordination.rs: machine parser reuses the canonical version validator.
replace_once(
    'apps/windows-companion/src/coordination.rs',
    'fn valid_browser_version(value: &str) -> bool {\n',
    'pub fn valid_browser_version(value: &str) -> bool {\n',
)

# capture.rs: expose a borrowed, terminal-only view so M31 can stream opaque payloads
# without duplicating up to 256 MiB in memory.
path = 'apps/windows-companion/src/capture.rs'
replace_once(
    path,
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureAssignment {
    pub job_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureJobError {
''',
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureAssignment {
    pub job_id: String,
}

#[derive(Debug)]
pub enum CaptureTargetExport<'a> {
    Complete {
        instance: &'a BrowserInstance,
        encrypted: &'a [u8],
    },
    Failed {
        instance: &'a BrowserInstance,
        reason: CaptureFailure,
    },
}

impl CaptureTargetExport<'_> {
    pub fn instance(&self) -> &BrowserInstance {
        match self {
            Self::Complete { instance, .. } | Self::Failed { instance, .. } => instance,
        }
    }
}

#[derive(Debug)]
pub struct CaptureJobExport<'a> {
    pub job_id: &'a str,
    pub targets: Vec<CaptureTargetExport<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureJobError {
''',
)
replace_once(
    path,
    '''    AlreadyFinished,
    EmptyResult,
''',
    '''    AlreadyFinished,
    NotTerminal,
    EmptyResult,
''',
)
replace_once(
    path,
    '''    pub fn status(&mut self, job_id: &str, now: Instant) -> Option<CaptureJobStatus> {
''',
    '''    pub fn terminal_export(
        &mut self,
        job_id: &str,
        now: Instant,
    ) -> Result<CaptureJobExport<'_>, CaptureJobError> {
        self.refresh(now);
        let job = self.jobs.get(job_id).ok_or(CaptureJobError::NotFound)?;
        if job.targets.iter().any(|target| {
            matches!(
                target.state,
                CaptureTargetState::Pending | CaptureTargetState::Claimed { .. }
            )
        }) {
            return Err(CaptureJobError::NotTerminal);
        }

        let targets = job
            .targets
            .iter()
            .map(|target| match &target.state {
                CaptureTargetState::Complete { encrypted } => CaptureTargetExport::Complete {
                    instance: &target.instance,
                    encrypted,
                },
                CaptureTargetState::Failed { reason } => CaptureTargetExport::Failed {
                    instance: &target.instance,
                    reason: *reason,
                },
                CaptureTargetState::Pending | CaptureTargetState::Claimed { .. } => {
                    unreachable!("terminal capture export checked unfinished targets")
                }
            })
            .collect();
        Ok(CaptureJobExport {
            job_id: &job.job_id,
            targets,
        })
    }

    pub fn status(&mut self, job_id: &str, now: Instant) -> Option<CaptureJobStatus> {
''',
)
replace_once(
    path,
    '''    #[test]
    fn times_out_unfinished_targets() {
''',
    '''    #[test]
    fn terminal_export_rejects_live_jobs_and_borrows_opaque_results() {
        let start = Instant::now();
        let mut jobs = store();
        jobs.create(
            JOB_ID.to_owned(),
            vec![instance(
                CHROME_ID,
                BrowserKind::Chrome,
                vec![BrowserCapability::Capture],
            )],
            start,
        )
        .unwrap();

        assert_eq!(
            jobs.terminal_export(JOB_ID, start).unwrap_err(),
            CaptureJobError::NotTerminal
        );
        jobs.submit_result(JOB_ID, CHROME_ID, vec![4, 5, 6], start)
            .unwrap();
        let export = jobs.terminal_export(JOB_ID, start).unwrap();
        assert_eq!(export.job_id, JOB_ID);
        assert!(matches!(
            &export.targets[0],
            CaptureTargetExport::Complete { encrypted, .. } if *encrypted == [4, 5, 6]
        ));
    }

    #[test]
    fn times_out_unfinished_targets() {
''',
)

# machine.rs: refuse manifest-only bundles with zero successful encrypted payloads.
path = 'apps/windows-companion/src/machine.rs'
replace_once(
    path,
    '''    let mut seen = HashSet::new();
    let mut payload_bytes = 0_usize;
''',
    '''    let mut seen = HashSet::new();
    let mut payload_bytes = 0_usize;
    let mut complete_targets = 0_usize;
''',
)
replace_once(
    path,
    '''                payload_bytes = payload_bytes
                    .checked_add(encrypted.len())
''',
    '''                complete_targets += 1;
                payload_bytes = payload_bytes
                    .checked_add(encrypted.len())
''',
)
replace_once(
    path,
    '''    if output.is_empty() || output.len() > MAX_MACHINE_MANIFEST_BYTES {
''',
    '''    if complete_targets == 0 {
        return Err(invalid_data(
            "Machine snapshot requires at least one encrypted browser payload.",
        ));
    }
    if output.is_empty() || output.len() > MAX_MACHINE_MANIFEST_BYTES {
''',
)

# protocol.rs: companion-side persistence from the borrowed terminal capture view.
path = 'apps/windows-companion/src/protocol.rs'
replace_once(
    path,
    '''use crate::library::{MAX_SNAPSHOT_FILE_BYTES, SnapshotLibrary};
''',
    '''use crate::library::{MAX_SNAPSHOT_FILE_BYTES, SnapshotLibrary};
use crate::machine::{MachineSnapshotEntry, MachineSnapshotLibrary};
''',
)
replace_once(
    path,
    '''    pub fn capture_job_status(&self, job_id: &str) -> io::Result<Option<CaptureJobStatus>> {
        let mut jobs = self
            .capture_jobs
            .lock()
            .map_err(|_| io::Error::other("Capture job store is unavailable."))?;
        Ok(jobs.status(job_id, Instant::now()))
    }
}
''',
    '''    pub fn capture_job_status(&self, job_id: &str) -> io::Result<Option<CaptureJobStatus>> {
        let mut jobs = self
            .capture_jobs
            .lock()
            .map_err(|_| io::Error::other("Capture job store is unavailable."))?;
        Ok(jobs.status(job_id, Instant::now()))
    }

    pub fn persist_capture_job(
        &self,
        library: &MachineSnapshotLibrary,
        job_id: &str,
        suggested_name: &str,
    ) -> io::Result<MachineSnapshotEntry> {
        let mut jobs = self
            .capture_jobs
            .lock()
            .map_err(|_| io::Error::other("Capture job store is unavailable."))?;
        let export = jobs
            .terminal_export(job_id, Instant::now())
            .map_err(capture_control_error)?;
        library.write_capture_job(suggested_name, &export)
    }
}
''',
)
replace_once(
    path,
    '''        CaptureJobError::CapacityExceeded => io::Error::other("Capture job capacity is exhausted."),
        CaptureJobError::InvalidJobId | CaptureJobError::DuplicateJob => {
''',
    '''        CaptureJobError::CapacityExceeded => io::Error::other("Capture job capacity is exhausted."),
        CaptureJobError::NotFound => io::Error::new(io::ErrorKind::NotFound, "Capture job was not found."),
        CaptureJobError::NotTerminal => io::Error::new(
            io::ErrorKind::WouldBlock,
            "Capture job is not complete yet.",
        ),
        CaptureJobError::InvalidJobId | CaptureJobError::DuplicateJob => {
''',
)

# main.rs: M31 persists the terminal job atomically beside the existing snapshot library.
path = 'apps/windows-companion/src/main.rs'
replace_once(path, 'pub mod library;\n', 'pub mod library;\npub mod machine;\n')
replace_once(
    path,
    '''use library::SnapshotLibrary;
use protocol::ProtocolServer;
''',
    '''use library::SnapshotLibrary;
use machine::MachineSnapshotLibrary;
use protocol::ProtocolServer;
''',
)
replace_once(
    path,
    '''    let server = ProtocolServer::bind(library)?;
    let control = server.capture_control();
''',
    '''    let machine_library = MachineSnapshotLibrary::new(library.root().to_path_buf());
    let server = ProtocolServer::bind(library)?;
    let control = server.capture_control();
''',
)
replace_once(
    path,
    '''            println!(
                "capture complete: {} succeeded, {} failed",
                current.completed_count(),
                current.failed_count()
            );
            println!(
                "M30 results are held in memory only; M31 adds the machine snapshot container."
            );
            return Ok(());
''',
    '''            println!(
                "capture complete: {} succeeded, {} failed",
                current.completed_count(),
                current.failed_count()
            );
            if current.completed_count() == 0 {
                return Err("Capture completed without any encrypted browser payloads.".into());
            }
            let suggested_name = format!("tabsnap-machine-{}", current.job_id);
            let stored = control.persist_capture_job(
                &machine_library,
                &current.job_id,
                &suggested_name,
            )?;
            println!("machine snapshot: {}", stored.path.display());
            println!("machine snapshot bytes: {}", stored.size);
            println!(
                "machine snapshot contains {} browser target(s); encrypted payloads remain opaque to the companion",
                stored.manifest.targets.len()
            );
            return Ok(());
''',
)
