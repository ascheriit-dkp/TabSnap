use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::capture::valid_job_id;
use crate::coordination::{BrowserCapability, BrowserInstance, BrowserKind, MAX_BROWSER_INSTANCES};
use crate::machine::{MachineSnapshotEntry, MachineTargetState};

pub const RESTORE_VERSION: u8 = 1;
pub const RESTORE_CLAIM_LEASE: Duration = Duration::from_secs(15);
pub const RESTORE_TARGET_TIMEOUT: Duration = Duration::from_secs(60);
pub const RESTORE_JOB_RETENTION: Duration = Duration::from_secs(300);
pub const MAX_RESTORE_JOBS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreFailure {
    PasswordRequired,
    DecryptFailed,
    RestoreFailed,
    TimedOut,
    PayloadUnavailable,
}

impl RestoreFailure {
    pub fn parse_wire(value: &str) -> Option<Self> {
        match value {
            "password-required" => Some(Self::PasswordRequired),
            "decrypt-failed" => Some(Self::DecryptFailed),
            "restore-failed" => Some(Self::RestoreFailed),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PasswordRequired => "password-required",
            Self::DecryptFailed => "decrypt-failed",
            Self::RestoreFailed => "restore-failed",
            Self::TimedOut => "timed-out",
            Self::PayloadUnavailable => "payload-unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreSkip {
    SourceUnavailable,
    BrowserUnavailable,
}

impl RestoreSkip {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceUnavailable => "source-unavailable",
            Self::BrowserUnavailable => "browser-unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreTargetStateView {
    Pending,
    Claimed,
    Complete,
    Failed { reason: RestoreFailure },
    Skipped { reason: RestoreSkip },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreTargetStatus {
    pub source_instance_id: String,
    pub source_browser: BrowserKind,
    pub source_version: Option<String>,
    pub destination: Option<BrowserInstance>,
    pub state: RestoreTargetStateView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreJobStatus {
    pub job_id: String,
    pub machine_file_name: String,
    pub targets: Vec<RestoreTargetStatus>,
}

impl RestoreJobStatus {
    pub fn is_terminal(&self) -> bool {
        self.targets.iter().all(|target| {
            matches!(
                target.state,
                RestoreTargetStateView::Complete
                    | RestoreTargetStateView::Failed { .. }
                    | RestoreTargetStateView::Skipped { .. }
            )
        })
    }

    pub fn completed_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| matches!(target.state, RestoreTargetStateView::Complete))
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| matches!(target.state, RestoreTargetStateView::Failed { .. }))
            .count()
    }

    pub fn skipped_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| matches!(target.state, RestoreTargetStateView::Skipped { .. }))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreAssignment {
    pub job_id: String,
    pub machine_file_name: String,
    pub source_instance_id: String,
    pub source_browser: BrowserKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreJobError {
    InvalidJobId,
    DuplicateJob,
    CapacityExceeded,
    NotFound,
    TargetNotFound,
    AlreadyFinished,
    NoRetryableTargets,
}

#[derive(Debug)]
enum RestoreTargetState {
    Pending,
    Claimed { claimed_at: Instant },
    Complete,
    Failed { reason: RestoreFailure },
    Skipped { reason: RestoreSkip },
}

#[derive(Debug)]
struct RestoreTarget {
    source_instance_id: String,
    source_browser: BrowserKind,
    source_version: Option<String>,
    destination: Option<BrowserInstance>,
    state: RestoreTargetState,
}

#[derive(Debug)]
struct RestoreJob {
    job_id: String,
    machine_file_name: String,
    attempt_started_at: Instant,
    targets: Vec<RestoreTarget>,
}

#[derive(Debug)]
pub struct RestoreJobStore {
    jobs: HashMap<String, RestoreJob>,
    claim_lease: Duration,
    target_timeout: Duration,
    retention: Duration,
}

impl Default for RestoreJobStore {
    fn default() -> Self {
        Self::new(
            RESTORE_CLAIM_LEASE,
            RESTORE_TARGET_TIMEOUT,
            RESTORE_JOB_RETENTION,
        )
    }
}

impl RestoreJobStore {
    pub fn new(claim_lease: Duration, target_timeout: Duration, retention: Duration) -> Self {
        Self {
            jobs: HashMap::new(),
            claim_lease,
            target_timeout,
            retention,
        }
    }

    pub fn create(
        &mut self,
        job_id: String,
        machine: &MachineSnapshotEntry,
        instances: Vec<BrowserInstance>,
        now: Instant,
    ) -> Result<RestoreJobStatus, RestoreJobError> {
        self.refresh(now);
        if !valid_job_id(&job_id) {
            return Err(RestoreJobError::InvalidJobId);
        }
        if self.jobs.contains_key(&job_id) {
            return Err(RestoreJobError::DuplicateJob);
        }
        if self.jobs.len() >= MAX_RESTORE_JOBS
            || machine.manifest.targets.len() > MAX_BROWSER_INSTANCES
        {
            return Err(RestoreJobError::CapacityExceeded);
        }

        let mut targets = machine
            .manifest
            .targets
            .iter()
            .map(|source| RestoreTarget {
                source_instance_id: source.instance_id.clone(),
                source_browser: source.browser,
                source_version: source.version.clone(),
                destination: None,
                state: match source.state {
                    MachineTargetState::Complete { .. } => RestoreTargetState::Skipped {
                        reason: RestoreSkip::BrowserUnavailable,
                    },
                    MachineTargetState::Failed { .. } => RestoreTargetState::Skipped {
                        reason: RestoreSkip::SourceUnavailable,
                    },
                },
            })
            .collect::<Vec<_>>();

        assign_available_destinations(&mut targets, instances, &HashSet::new());

        self.jobs.insert(
            job_id.clone(),
            RestoreJob {
                job_id: job_id.clone(),
                machine_file_name: machine.file_name.clone(),
                attempt_started_at: now,
                targets,
            },
        );
        self.status(&job_id, now).ok_or(RestoreJobError::NotFound)
    }

    pub fn next_assignment(
        &mut self,
        instance_id: &str,
        now: Instant,
    ) -> Option<RestoreAssignment> {
        self.refresh(now);
        let mut candidates = self
            .jobs
            .values()
            .filter(|job| {
                job.targets.iter().any(|target| {
                    target
                        .destination
                        .as_ref()
                        .is_some_and(|destination| destination.instance_id == instance_id)
                        && match target.state {
                            RestoreTargetState::Pending => true,
                            RestoreTargetState::Claimed { claimed_at } => {
                                now.saturating_duration_since(claimed_at) >= self.claim_lease
                            }
                            RestoreTargetState::Complete
                            | RestoreTargetState::Failed { .. }
                            | RestoreTargetState::Skipped { .. } => false,
                        }
                })
            })
            .map(|job| (job.attempt_started_at, job.job_id.clone()))
            .collect::<Vec<_>>();
        candidates.sort();

        let (_, job_id) = candidates.into_iter().next()?;
        let job = self.jobs.get_mut(&job_id)?;
        let target = job.targets.iter_mut().find(|target| {
            target
                .destination
                .as_ref()
                .is_some_and(|destination| destination.instance_id == instance_id)
                && matches!(
                    target.state,
                    RestoreTargetState::Pending | RestoreTargetState::Claimed { .. }
                )
        })?;
        target.state = RestoreTargetState::Claimed { claimed_at: now };
        Some(RestoreAssignment {
            job_id,
            machine_file_name: job.machine_file_name.clone(),
            source_instance_id: target.source_instance_id.clone(),
            source_browser: target.source_browser,
        })
    }

    pub fn submit_success(
        &mut self,
        job_id: &str,
        destination_instance_id: &str,
        now: Instant,
    ) -> Result<(), RestoreJobError> {
        self.finish_target(job_id, destination_instance_id, None, now)
    }

    pub fn submit_failure(
        &mut self,
        job_id: &str,
        destination_instance_id: &str,
        reason: RestoreFailure,
        now: Instant,
    ) -> Result<(), RestoreJobError> {
        self.finish_target(job_id, destination_instance_id, Some(reason), now)
    }

    fn finish_target(
        &mut self,
        job_id: &str,
        destination_instance_id: &str,
        failure: Option<RestoreFailure>,
        now: Instant,
    ) -> Result<(), RestoreJobError> {
        self.refresh(now);
        let job = self.jobs.get_mut(job_id).ok_or(RestoreJobError::NotFound)?;
        let target =
            job.targets
                .iter_mut()
                .find(|target| {
                    target.destination.as_ref().is_some_and(|destination| {
                        destination.instance_id == destination_instance_id
                    })
                })
                .ok_or(RestoreJobError::TargetNotFound)?;

        match target.state {
            RestoreTargetState::Pending | RestoreTargetState::Claimed { .. } => {
                target.state = match failure {
                    Some(reason) => RestoreTargetState::Failed { reason },
                    None => RestoreTargetState::Complete,
                };
                Ok(())
            }
            RestoreTargetState::Complete
            | RestoreTargetState::Failed { .. }
            | RestoreTargetState::Skipped { .. } => Err(RestoreJobError::AlreadyFinished),
        }
    }

    pub fn retry(
        &mut self,
        job_id: &str,
        instances: Vec<BrowserInstance>,
        now: Instant,
    ) -> Result<RestoreJobStatus, RestoreJobError> {
        self.refresh(now);
        let job = self.jobs.get_mut(job_id).ok_or(RestoreJobError::NotFound)?;

        let retryable = job.targets.iter().any(|target| {
            matches!(
                target.state,
                RestoreTargetState::Failed { .. }
                    | RestoreTargetState::Skipped {
                        reason: RestoreSkip::BrowserUnavailable
                    }
            )
        });
        if !retryable {
            return Err(RestoreJobError::NoRetryableTargets);
        }

        let used = job
            .targets
            .iter()
            .filter(|target| {
                !matches!(
                    target.state,
                    RestoreTargetState::Failed { .. }
                        | RestoreTargetState::Skipped {
                            reason: RestoreSkip::BrowserUnavailable
                        }
                )
            })
            .filter_map(|target| target.destination.as_ref())
            .map(|destination| destination.instance_id.clone())
            .collect::<HashSet<_>>();

        for target in &mut job.targets {
            if matches!(
                target.state,
                RestoreTargetState::Failed { .. }
                    | RestoreTargetState::Skipped {
                        reason: RestoreSkip::BrowserUnavailable
                    }
            ) {
                target.destination = None;
                target.state = RestoreTargetState::Skipped {
                    reason: RestoreSkip::BrowserUnavailable,
                };
            }
        }

        assign_available_destinations(&mut job.targets, instances, &used);
        job.attempt_started_at = now;
        self.status(job_id, now).ok_or(RestoreJobError::NotFound)
    }

    pub fn status(&mut self, job_id: &str, now: Instant) -> Option<RestoreJobStatus> {
        self.refresh(now);
        let job = self.jobs.get(job_id)?;
        Some(RestoreJobStatus {
            job_id: job.job_id.clone(),
            machine_file_name: job.machine_file_name.clone(),
            targets: job
                .targets
                .iter()
                .map(|target| RestoreTargetStatus {
                    source_instance_id: target.source_instance_id.clone(),
                    source_browser: target.source_browser,
                    source_version: target.source_version.clone(),
                    destination: target.destination.clone(),
                    state: match target.state {
                        RestoreTargetState::Pending => RestoreTargetStateView::Pending,
                        RestoreTargetState::Claimed { .. } => RestoreTargetStateView::Claimed,
                        RestoreTargetState::Complete => RestoreTargetStateView::Complete,
                        RestoreTargetState::Failed { reason } => {
                            RestoreTargetStateView::Failed { reason }
                        }
                        RestoreTargetState::Skipped { reason } => {
                            RestoreTargetStateView::Skipped { reason }
                        }
                    },
                })
                .collect(),
        })
    }

    fn refresh(&mut self, now: Instant) {
        for job in self.jobs.values_mut() {
            if now.saturating_duration_since(job.attempt_started_at) >= self.target_timeout {
                for target in &mut job.targets {
                    if matches!(
                        target.state,
                        RestoreTargetState::Pending | RestoreTargetState::Claimed { .. }
                    ) {
                        target.state = RestoreTargetState::Failed {
                            reason: RestoreFailure::TimedOut,
                        };
                    }
                }
            }
        }

        self.jobs.retain(|_, job| {
            now.saturating_duration_since(job.attempt_started_at) < self.retention
        });
    }
}

fn assign_available_destinations(
    targets: &mut [RestoreTarget],
    instances: Vec<BrowserInstance>,
    already_used: &HashSet<String>,
) {
    let mut seen = HashSet::new();
    let mut available = instances
        .into_iter()
        .filter(|instance| instance.capabilities.contains(&BrowserCapability::Restore))
        .filter(|instance| seen.insert(instance.instance_id.clone()))
        .filter(|instance| !already_used.contains(&instance.instance_id))
        .collect::<Vec<_>>();
    available.sort_by(|left, right| {
        (left.browser, left.instance_id.as_str()).cmp(&(right.browser, right.instance_id.as_str()))
    });

    let mut used = already_used.clone();

    for target in targets.iter_mut().filter(|target| is_unmapped(target)) {
        if let Some(destination) = available.iter().find(|instance| {
            instance.instance_id == target.source_instance_id
                && !used.contains(&instance.instance_id)
        }) {
            target.destination = Some(destination.clone());
            target.state = RestoreTargetState::Pending;
            used.insert(destination.instance_id.clone());
        }
    }

    for target in targets.iter_mut().filter(|target| is_unmapped(target)) {
        if let Some(destination) = available.iter().find(|instance| {
            instance.browser == target.source_browser && !used.contains(&instance.instance_id)
        }) {
            target.destination = Some(destination.clone());
            target.state = RestoreTargetState::Pending;
            used.insert(destination.instance_id.clone());
        }
    }

    for target in targets.iter_mut().filter(|target| is_unmapped(target)) {
        if let Some(destination) = available
            .iter()
            .find(|instance| !used.contains(&instance.instance_id))
        {
            target.destination = Some(destination.clone());
            target.state = RestoreTargetState::Pending;
            used.insert(destination.instance_id.clone());
        }
    }
}

fn is_unmapped(target: &RestoreTarget) -> bool {
    target.destination.is_none()
        && matches!(
            target.state,
            RestoreTargetState::Skipped {
                reason: RestoreSkip::BrowserUnavailable
            }
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureFailure;
    use crate::machine::{
        MACHINE_CONTAINER_VERSION, MachineManifest, MachineTargetManifest, MachineTargetState,
    };
    use std::path::PathBuf;
    use std::time::SystemTime;

    const JOB_ID: &str = "11111111111111111111111111111111";
    const CHROME_SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const EDGE_SOURCE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const CHROME_DEST: &str = "cccccccccccccccccccccccccccccccc";
    const EDGE_DEST: &str = "dddddddddddddddddddddddddddddddd";
    const FIREFOX_DEST: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn browser(instance_id: &str, browser: BrowserKind) -> BrowserInstance {
        BrowserInstance {
            instance_id: instance_id.to_owned(),
            browser,
            version: Some("140.0".to_owned()),
            capabilities: vec![BrowserCapability::Capture, BrowserCapability::Restore],
        }
    }

    fn machine(targets: Vec<MachineTargetManifest>) -> MachineSnapshotEntry {
        MachineSnapshotEntry {
            file_name: "whole-machine.tabsnap-machine".to_owned(),
            path: PathBuf::from("whole-machine.tabsnap-machine"),
            size: 42,
            modified: Some(SystemTime::UNIX_EPOCH),
            manifest: MachineManifest {
                version: MACHINE_CONTAINER_VERSION,
                job_id: "99999999999999999999999999999999".to_owned(),
                stored_unix_ms: 0,
                targets,
            },
        }
    }

    fn complete_source(instance_id: &str, browser: BrowserKind) -> MachineTargetManifest {
        MachineTargetManifest {
            instance_id: instance_id.to_owned(),
            browser,
            version: Some("140.0".to_owned()),
            state: MachineTargetState::Complete {
                payload_offset: 10,
                payload_len: 20,
            },
        }
    }

    fn store() -> RestoreJobStore {
        RestoreJobStore::new(
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(120),
        )
    }

    #[test]
    fn reserves_native_destinations_before_cross_browser_fallback() {
        let start = Instant::now();
        let snapshot = machine(vec![
            complete_source(CHROME_SOURCE, BrowserKind::Chrome),
            complete_source(EDGE_SOURCE, BrowserKind::Edge),
        ]);
        let mut jobs = store();
        let status = jobs
            .create(
                JOB_ID.to_owned(),
                &snapshot,
                vec![
                    browser(EDGE_DEST, BrowserKind::Edge),
                    browser(FIREFOX_DEST, BrowserKind::Firefox),
                ],
                start,
            )
            .unwrap();

        let chrome = status
            .targets
            .iter()
            .find(|target| target.source_instance_id == CHROME_SOURCE)
            .unwrap();
        let edge = status
            .targets
            .iter()
            .find(|target| target.source_instance_id == EDGE_SOURCE)
            .unwrap();
        assert_eq!(
            edge.destination.as_ref().unwrap().instance_id,
            EDGE_DEST,
            "native Edge destination must not be consumed by Chrome fallback"
        );
        assert_eq!(
            chrome.destination.as_ref().unwrap().instance_id,
            FIREFOX_DEST
        );
    }

    #[test]
    fn prefers_same_live_instance_then_same_browser() {
        let start = Instant::now();
        let snapshot = machine(vec![complete_source(CHROME_SOURCE, BrowserKind::Chrome)]);
        let mut jobs = store();
        let status = jobs
            .create(
                JOB_ID.to_owned(),
                &snapshot,
                vec![
                    browser(CHROME_DEST, BrowserKind::Chrome),
                    browser(CHROME_SOURCE, BrowserKind::Chrome),
                ],
                start,
            )
            .unwrap();

        assert_eq!(
            status.targets[0].destination.as_ref().unwrap().instance_id,
            CHROME_SOURCE
        );
    }

    #[test]
    fn records_missing_capture_payload_as_non_retryable_skip() {
        let start = Instant::now();
        let snapshot = machine(vec![MachineTargetManifest {
            instance_id: CHROME_SOURCE.to_owned(),
            browser: BrowserKind::Chrome,
            version: None,
            state: MachineTargetState::Failed {
                reason: CaptureFailure::CaptureFailed,
            },
        }]);
        let mut jobs = store();
        let status = jobs
            .create(
                JOB_ID.to_owned(),
                &snapshot,
                vec![browser(CHROME_DEST, BrowserKind::Chrome)],
                start,
            )
            .unwrap();

        assert!(matches!(
            status.targets[0].state,
            RestoreTargetStateView::Skipped {
                reason: RestoreSkip::SourceUnavailable
            }
        ));
        assert_eq!(
            jobs.retry(
                JOB_ID,
                vec![browser(CHROME_DEST, BrowserKind::Chrome)],
                start
            ),
            Err(RestoreJobError::NoRetryableTargets)
        );
    }

    #[test]
    fn retries_browser_unavailable_and_failed_targets_without_replaying_success() {
        let start = Instant::now();
        let snapshot = machine(vec![
            complete_source(CHROME_SOURCE, BrowserKind::Chrome),
            complete_source(EDGE_SOURCE, BrowserKind::Edge),
        ]);
        let mut jobs = store();
        let created = jobs
            .create(
                JOB_ID.to_owned(),
                &snapshot,
                vec![browser(CHROME_DEST, BrowserKind::Chrome)],
                start,
            )
            .unwrap();
        assert_eq!(created.skipped_count(), 1);

        let assignment = jobs.next_assignment(CHROME_DEST, start).unwrap();
        assert_eq!(assignment.source_instance_id, CHROME_SOURCE);
        jobs.submit_success(JOB_ID, CHROME_DEST, start).unwrap();

        let retried = jobs
            .retry(
                JOB_ID,
                vec![
                    browser(CHROME_DEST, BrowserKind::Chrome),
                    browser(EDGE_DEST, BrowserKind::Edge),
                ],
                start + Duration::from_secs(1),
            )
            .unwrap();
        assert!(matches!(
            retried
                .targets
                .iter()
                .find(|target| target.source_instance_id == CHROME_SOURCE)
                .unwrap()
                .state,
            RestoreTargetStateView::Complete
        ));
        assert_eq!(
            retried
                .targets
                .iter()
                .find(|target| target.source_instance_id == EDGE_SOURCE)
                .unwrap()
                .destination
                .as_ref()
                .unwrap()
                .instance_id,
            EDGE_DEST
        );
    }

    #[test]
    fn claim_lease_prevents_duplicate_restore_and_failure_is_retryable() {
        let start = Instant::now();
        let snapshot = machine(vec![complete_source(CHROME_SOURCE, BrowserKind::Chrome)]);
        let mut jobs = store();
        jobs.create(
            JOB_ID.to_owned(),
            &snapshot,
            vec![browser(CHROME_DEST, BrowserKind::Chrome)],
            start,
        )
        .unwrap();

        assert!(jobs.next_assignment(CHROME_DEST, start).is_some());
        assert!(
            jobs.next_assignment(CHROME_DEST, start + Duration::from_secs(9))
                .is_none()
        );
        assert!(
            jobs.next_assignment(CHROME_DEST, start + Duration::from_secs(10))
                .is_some()
        );
        jobs.submit_failure(
            JOB_ID,
            CHROME_DEST,
            RestoreFailure::DecryptFailed,
            start + Duration::from_secs(10),
        )
        .unwrap();
        let retried = jobs
            .retry(
                JOB_ID,
                vec![browser(CHROME_DEST, BrowserKind::Chrome)],
                start + Duration::from_secs(11),
            )
            .unwrap();
        assert!(matches!(
            retried.targets[0].state,
            RestoreTargetStateView::Pending
        ));
    }

    #[test]
    fn multiple_restore_jobs_for_one_destination_are_isolated_and_ordered() {
        let start = Instant::now();
        let second_job = "22222222222222222222222222222222";
        let snapshot = machine(vec![complete_source(CHROME_SOURCE, BrowserKind::Chrome)]);
        let mut jobs = store();
        let destination = browser(CHROME_DEST, BrowserKind::Chrome);

        jobs.create(
            JOB_ID.to_owned(),
            &snapshot,
            vec![destination.clone()],
            start,
        )
        .unwrap();
        jobs.create(
            second_job.to_owned(),
            &snapshot,
            vec![destination],
            start + Duration::from_millis(1),
        )
        .unwrap();

        let first = jobs
            .next_assignment(CHROME_DEST, start + Duration::from_millis(1))
            .unwrap();
        assert_eq!(first.job_id, JOB_ID);
        jobs.submit_success(JOB_ID, CHROME_DEST, start + Duration::from_millis(1))
            .unwrap();

        let second = jobs
            .next_assignment(CHROME_DEST, start + Duration::from_millis(1))
            .unwrap();
        assert_eq!(second.job_id, second_job);
        jobs.submit_failure(
            second_job,
            CHROME_DEST,
            RestoreFailure::RestoreFailed,
            start + Duration::from_millis(1),
        )
        .unwrap();

        let first_status = jobs
            .status(JOB_ID, start + Duration::from_millis(1))
            .unwrap();
        let second_status = jobs
            .status(second_job, start + Duration::from_millis(1))
            .unwrap();
        assert_eq!(first_status.completed_count(), 1);
        assert_eq!(first_status.failed_count(), 0);
        assert_eq!(second_status.completed_count(), 0);
        assert_eq!(second_status.failed_count(), 1);
    }

    #[test]
    fn unrelated_destination_cannot_finish_another_targets_restore() {
        let start = Instant::now();
        let snapshot = machine(vec![complete_source(CHROME_SOURCE, BrowserKind::Chrome)]);
        let mut jobs = store();
        jobs.create(
            JOB_ID.to_owned(),
            &snapshot,
            vec![browser(CHROME_DEST, BrowserKind::Chrome)],
            start,
        )
        .unwrap();

        assert_eq!(
            jobs.submit_success(JOB_ID, EDGE_DEST, start),
            Err(RestoreJobError::TargetNotFound)
        );
        let status = jobs.status(JOB_ID, start).unwrap();
        assert!(matches!(
            status.targets[0].state,
            RestoreTargetStateView::Pending
        ));
    }

    #[test]
    fn unfinished_targets_time_out() {
        let start = Instant::now();
        let snapshot = machine(vec![complete_source(CHROME_SOURCE, BrowserKind::Chrome)]);
        let mut jobs = store();
        jobs.create(
            JOB_ID.to_owned(),
            &snapshot,
            vec![browser(CHROME_DEST, BrowserKind::Chrome)],
            start,
        )
        .unwrap();

        let status = jobs
            .status(JOB_ID, start + Duration::from_secs(30))
            .unwrap();
        assert!(matches!(
            status.targets[0].state,
            RestoreTargetStateView::Failed {
                reason: RestoreFailure::TimedOut
            }
        ));
    }
}
