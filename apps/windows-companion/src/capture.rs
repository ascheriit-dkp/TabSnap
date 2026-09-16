use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::coordination::{BrowserCapability, BrowserInstance, MAX_BROWSER_INSTANCES};

pub const CAPTURE_VERSION: u8 = 1;
pub const CAPTURE_CLAIM_LEASE: Duration = Duration::from_secs(15);
pub const CAPTURE_TARGET_TIMEOUT: Duration = Duration::from_secs(60);
pub const CAPTURE_JOB_RETENTION: Duration = Duration::from_secs(300);
pub const MAX_CAPTURE_JOBS: usize = 8;
pub const MAX_CAPTURE_RESULT_BYTES: usize = 65 * 1024 * 1024;
pub const MAX_CAPTURE_JOB_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFailure {
    PasswordRequired,
    CaptureFailed,
    EncryptionFailed,
    TimedOut,
}

impl CaptureFailure {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "password-required" => Some(Self::PasswordRequired),
            "capture-failed" => Some(Self::CaptureFailed),
            "encryption-failed" => Some(Self::EncryptionFailed),
            "timed-out" => Some(Self::TimedOut),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PasswordRequired => "password-required",
            Self::CaptureFailed => "capture-failed",
            Self::EncryptionFailed => "encryption-failed",
            Self::TimedOut => "timed-out",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureTargetStateView {
    Pending,
    Claimed,
    Complete { bytes: u64 },
    Failed { reason: CaptureFailure },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureTargetStatus {
    pub instance: BrowserInstance,
    pub state: CaptureTargetStateView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureJobStatus {
    pub job_id: String,
    pub targets: Vec<CaptureTargetStatus>,
}

impl CaptureJobStatus {
    pub fn is_terminal(&self) -> bool {
        self.targets.iter().all(|target| {
            matches!(
                target.state,
                CaptureTargetStateView::Complete { .. } | CaptureTargetStateView::Failed { .. }
            )
        })
    }

    pub fn completed_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| matches!(target.state, CaptureTargetStateView::Complete { .. }))
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| matches!(target.state, CaptureTargetStateView::Failed { .. }))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    InvalidJobId,
    DuplicateJob,
    NoTargets,
    CapacityExceeded,
    NotFound,
    TargetNotFound,
    AlreadyFinished,
    NotTerminal,
    EmptyResult,
    ResultTooLarge,
    JobBytesExceeded,
}

#[derive(Debug)]
enum CaptureTargetState {
    Pending,
    Claimed { claimed_at: Instant },
    Complete { encrypted: Vec<u8> },
    Failed { reason: CaptureFailure },
}

#[derive(Debug)]
struct CaptureTarget {
    instance: BrowserInstance,
    state: CaptureTargetState,
}

#[derive(Debug)]
struct CaptureJob {
    job_id: String,
    created_at: Instant,
    targets: Vec<CaptureTarget>,
}

#[derive(Debug)]
pub struct CaptureJobStore {
    jobs: HashMap<String, CaptureJob>,
    claim_lease: Duration,
    target_timeout: Duration,
    retention: Duration,
    max_result_bytes: usize,
    max_job_bytes: usize,
}

impl Default for CaptureJobStore {
    fn default() -> Self {
        Self::new(
            CAPTURE_CLAIM_LEASE,
            CAPTURE_TARGET_TIMEOUT,
            CAPTURE_JOB_RETENTION,
            MAX_CAPTURE_RESULT_BYTES,
            MAX_CAPTURE_JOB_BYTES,
        )
    }
}

impl CaptureJobStore {
    pub fn new(
        claim_lease: Duration,
        target_timeout: Duration,
        retention: Duration,
        max_result_bytes: usize,
        max_job_bytes: usize,
    ) -> Self {
        Self {
            jobs: HashMap::new(),
            claim_lease,
            target_timeout,
            retention,
            max_result_bytes,
            max_job_bytes,
        }
    }

    pub fn create(
        &mut self,
        job_id: String,
        instances: Vec<BrowserInstance>,
        now: Instant,
    ) -> Result<CaptureJobStatus, CaptureJobError> {
        self.refresh(now);
        if !valid_job_id(&job_id) {
            return Err(CaptureJobError::InvalidJobId);
        }
        if self.jobs.contains_key(&job_id) {
            return Err(CaptureJobError::DuplicateJob);
        }
        if self.jobs.len() >= MAX_CAPTURE_JOBS {
            return Err(CaptureJobError::CapacityExceeded);
        }

        let mut seen = HashSet::new();
        let mut targets = instances
            .into_iter()
            .filter(|instance| instance.capabilities.contains(&BrowserCapability::Capture))
            .filter(|instance| seen.insert(instance.instance_id.clone()))
            .map(|instance| CaptureTarget {
                instance,
                state: CaptureTargetState::Pending,
            })
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return Err(CaptureJobError::NoTargets);
        }
        if targets.len() > MAX_BROWSER_INSTANCES {
            return Err(CaptureJobError::CapacityExceeded);
        }
        targets.sort_by(|left, right| {
            (left.instance.browser, left.instance.instance_id.as_str())
                .cmp(&(right.instance.browser, right.instance.instance_id.as_str()))
        });

        self.jobs.insert(
            job_id.clone(),
            CaptureJob {
                job_id: job_id.clone(),
                created_at: now,
                targets,
            },
        );
        self.status(&job_id, now).ok_or(CaptureJobError::NotFound)
    }

    pub fn next_assignment(
        &mut self,
        instance_id: &str,
        now: Instant,
    ) -> Option<CaptureAssignment> {
        self.refresh(now);
        let mut candidates = self
            .jobs
            .values()
            .filter(|job| {
                job.targets.iter().any(|target| {
                    target.instance.instance_id == instance_id
                        && match target.state {
                            CaptureTargetState::Pending => true,
                            CaptureTargetState::Claimed { claimed_at } => {
                                now.saturating_duration_since(claimed_at) >= self.claim_lease
                            }
                            CaptureTargetState::Complete { .. }
                            | CaptureTargetState::Failed { .. } => false,
                        }
                })
            })
            .map(|job| (job.created_at, job.job_id.clone()))
            .collect::<Vec<_>>();
        candidates.sort();

        let (_, job_id) = candidates.into_iter().next()?;
        let job = self.jobs.get_mut(&job_id)?;
        let target = job
            .targets
            .iter_mut()
            .find(|target| target.instance.instance_id == instance_id)?;
        target.state = CaptureTargetState::Claimed { claimed_at: now };
        Some(CaptureAssignment { job_id })
    }

    pub fn submit_result(
        &mut self,
        job_id: &str,
        instance_id: &str,
        encrypted: Vec<u8>,
        now: Instant,
    ) -> Result<(), CaptureJobError> {
        self.refresh(now);
        if encrypted.is_empty() {
            return Err(CaptureJobError::EmptyResult);
        }
        if encrypted.len() > self.max_result_bytes {
            return Err(CaptureJobError::ResultTooLarge);
        }

        let job = self.jobs.get_mut(job_id).ok_or(CaptureJobError::NotFound)?;
        let target_index = job
            .targets
            .iter()
            .position(|target| target.instance.instance_id == instance_id)
            .ok_or(CaptureJobError::TargetNotFound)?;
        if matches!(
            job.targets[target_index].state,
            CaptureTargetState::Complete { .. } | CaptureTargetState::Failed { .. }
        ) {
            return Err(CaptureJobError::AlreadyFinished);
        }

        let current_bytes = job
            .targets
            .iter()
            .filter_map(|target| match &target.state {
                CaptureTargetState::Complete { encrypted } => Some(encrypted.len()),
                _ => None,
            })
            .sum::<usize>();
        if current_bytes.saturating_add(encrypted.len()) > self.max_job_bytes {
            return Err(CaptureJobError::JobBytesExceeded);
        }

        job.targets[target_index].state = CaptureTargetState::Complete { encrypted };
        Ok(())
    }

    pub fn submit_failure(
        &mut self,
        job_id: &str,
        instance_id: &str,
        reason: CaptureFailure,
        now: Instant,
    ) -> Result<(), CaptureJobError> {
        self.refresh(now);
        let job = self.jobs.get_mut(job_id).ok_or(CaptureJobError::NotFound)?;
        let target = job
            .targets
            .iter_mut()
            .find(|target| target.instance.instance_id == instance_id)
            .ok_or(CaptureJobError::TargetNotFound)?;
        match target.state {
            CaptureTargetState::Pending | CaptureTargetState::Claimed { .. } => {
                target.state = CaptureTargetState::Failed { reason };
                Ok(())
            }
            CaptureTargetState::Complete { .. } | CaptureTargetState::Failed { .. } => {
                Err(CaptureJobError::AlreadyFinished)
            }
        }
    }

    pub fn terminal_export(
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
        self.refresh(now);
        let job = self.jobs.get(job_id)?;
        Some(CaptureJobStatus {
            job_id: job.job_id.clone(),
            targets: job
                .targets
                .iter()
                .map(|target| CaptureTargetStatus {
                    instance: target.instance.clone(),
                    state: match &target.state {
                        CaptureTargetState::Pending => CaptureTargetStateView::Pending,
                        CaptureTargetState::Claimed { .. } => CaptureTargetStateView::Claimed,
                        CaptureTargetState::Complete { encrypted } => {
                            CaptureTargetStateView::Complete {
                                bytes: encrypted.len() as u64,
                            }
                        }
                        CaptureTargetState::Failed { reason } => {
                            CaptureTargetStateView::Failed { reason: *reason }
                        }
                    },
                })
                .collect(),
        })
    }

    fn refresh(&mut self, now: Instant) {
        for job in self.jobs.values_mut() {
            if now.saturating_duration_since(job.created_at) >= self.target_timeout {
                for target in &mut job.targets {
                    if matches!(
                        target.state,
                        CaptureTargetState::Pending | CaptureTargetState::Claimed { .. }
                    ) {
                        target.state = CaptureTargetState::Failed {
                            reason: CaptureFailure::TimedOut,
                        };
                    }
                }
            }
        }
        self.jobs
            .retain(|_, job| now.saturating_duration_since(job.created_at) < self.retention);
    }
}

pub fn valid_job_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::{BrowserCapability, BrowserKind};

    const JOB_ID: &str = "11111111111111111111111111111111";
    const CHROME_ID: &str = "0123456789abcdef0123456789abcdef";
    const FIREFOX_ID: &str = "fedcba9876543210fedcba9876543210";

    fn instance(
        instance_id: &str,
        browser: BrowserKind,
        capabilities: Vec<BrowserCapability>,
    ) -> BrowserInstance {
        BrowserInstance {
            instance_id: instance_id.to_owned(),
            browser,
            version: None,
            capabilities,
        }
    }

    fn store() -> CaptureJobStore {
        CaptureJobStore::new(
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(120),
            16,
            24,
        )
    }

    #[test]
    fn creates_jobs_only_for_capture_capable_browsers() {
        let start = Instant::now();
        let mut jobs = store();
        let status = jobs
            .create(
                JOB_ID.to_owned(),
                vec![
                    instance(
                        CHROME_ID,
                        BrowserKind::Chrome,
                        vec![BrowserCapability::Capture],
                    ),
                    instance(
                        FIREFOX_ID,
                        BrowserKind::Firefox,
                        vec![BrowserCapability::Restore],
                    ),
                ],
                start,
            )
            .unwrap();

        assert_eq!(status.targets.len(), 1);
        assert_eq!(status.targets[0].instance.instance_id, CHROME_ID);
        assert_eq!(status.targets[0].state, CaptureTargetStateView::Pending);
    }

    #[test]
    fn claim_lease_prevents_duplicate_work_and_allows_retry() {
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
            jobs.next_assignment(CHROME_ID, start).unwrap().job_id,
            JOB_ID
        );
        assert!(
            jobs.next_assignment(CHROME_ID, start + Duration::from_secs(9))
                .is_none()
        );
        assert_eq!(
            jobs.next_assignment(CHROME_ID, start + Duration::from_secs(10))
                .unwrap()
                .job_id,
            JOB_ID
        );
    }

    #[test]
    fn records_partial_success_and_failure_without_plaintext() {
        let start = Instant::now();
        let mut jobs = store();
        jobs.create(
            JOB_ID.to_owned(),
            vec![
                instance(
                    CHROME_ID,
                    BrowserKind::Chrome,
                    vec![BrowserCapability::Capture],
                ),
                instance(
                    FIREFOX_ID,
                    BrowserKind::Firefox,
                    vec![BrowserCapability::Capture],
                ),
            ],
            start,
        )
        .unwrap();

        jobs.submit_result(JOB_ID, CHROME_ID, vec![1, 2, 3, 4], start)
            .unwrap();
        jobs.submit_failure(JOB_ID, FIREFOX_ID, CaptureFailure::PasswordRequired, start)
            .unwrap();

        let status = jobs.status(JOB_ID, start).unwrap();
        assert!(status.is_terminal());
        assert_eq!(status.completed_count(), 1);
        assert_eq!(status.failed_count(), 1);
        assert!(matches!(
            status.targets[0].state,
            CaptureTargetStateView::Complete { bytes: 4 }
        ));
        assert!(matches!(
            status.targets[1].state,
            CaptureTargetStateView::Failed {
                reason: CaptureFailure::PasswordRequired
            }
        ));
    }

    #[test]
    fn duplicate_terminal_submissions_report_already_finished_before_byte_limits() {
        let start = Instant::now();
        let mut jobs = store();
        jobs.create(
            JOB_ID.to_owned(),
            vec![
                instance(
                    CHROME_ID,
                    BrowserKind::Chrome,
                    vec![BrowserCapability::Capture],
                ),
                instance(
                    FIREFOX_ID,
                    BrowserKind::Firefox,
                    vec![BrowserCapability::Capture],
                ),
            ],
            start,
        )
        .unwrap();

        jobs.submit_result(JOB_ID, CHROME_ID, vec![0; 16], start)
            .unwrap();
        assert_eq!(
            jobs.submit_result(JOB_ID, CHROME_ID, vec![0; 16], start),
            Err(CaptureJobError::AlreadyFinished)
        );
        jobs.submit_failure(JOB_ID, FIREFOX_ID, CaptureFailure::CaptureFailed, start)
            .unwrap();
        assert_eq!(
            jobs.submit_result(JOB_ID, FIREFOX_ID, vec![0; 16], start),
            Err(CaptureJobError::AlreadyFinished)
        );
    }

    #[test]
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

        let status = jobs
            .status(JOB_ID, start + Duration::from_secs(30))
            .unwrap();
        assert!(matches!(
            status.targets[0].state,
            CaptureTargetStateView::Failed {
                reason: CaptureFailure::TimedOut
            }
        ));
    }

    #[test]
    fn enforces_result_and_job_byte_limits() {
        let start = Instant::now();
        let mut jobs = store();
        jobs.create(
            JOB_ID.to_owned(),
            vec![
                instance(
                    CHROME_ID,
                    BrowserKind::Chrome,
                    vec![BrowserCapability::Capture],
                ),
                instance(
                    FIREFOX_ID,
                    BrowserKind::Firefox,
                    vec![BrowserCapability::Capture],
                ),
            ],
            start,
        )
        .unwrap();

        assert_eq!(
            jobs.submit_result(JOB_ID, CHROME_ID, vec![0; 17], start),
            Err(CaptureJobError::ResultTooLarge)
        );
        jobs.submit_result(JOB_ID, CHROME_ID, vec![0; 16], start)
            .unwrap();
        assert_eq!(
            jobs.submit_result(JOB_ID, FIREFOX_ID, vec![0; 9], start),
            Err(CaptureJobError::JobBytesExceeded)
        );
    }
}
