from pathlib import Path

path = Path('apps/windows-companion/src/capture.rs')
text = path.read_text()
old = '''        let job = self.jobs.get_mut(job_id).ok_or(CaptureJobError::NotFound)?;
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

        let target = job
            .targets
            .iter_mut()
            .find(|target| target.instance.instance_id == instance_id)
            .ok_or(CaptureJobError::TargetNotFound)?;
        match target.state {
            CaptureTargetState::Pending | CaptureTargetState::Claimed { .. } => {
                target.state = CaptureTargetState::Complete { encrypted };
                Ok(())
            }
            CaptureTargetState::Complete { .. } | CaptureTargetState::Failed { .. } => {
                Err(CaptureJobError::AlreadyFinished)
            }
        }
'''
new = '''        let job = self.jobs.get_mut(job_id).ok_or(CaptureJobError::NotFound)?;
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
'''
if text.count(old) != 1:
    raise SystemExit('submit_result anchor mismatch')
text = text.replace(old, new)
anchor = '''    #[test]
    fn times_out_unfinished_targets() {
'''
test = '''    #[test]
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
    fn times_out_unfinished_targets() {
'''
if text.count(anchor) != 1:
    raise SystemExit('test anchor mismatch')
path.write_text(text.replace(anchor, test))
