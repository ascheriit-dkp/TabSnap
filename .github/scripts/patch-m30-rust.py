from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{path}: expected one anchor, found {count}: {old[:80]!r}')
    file.write_text(text.replace(old, new, 1))


# coordination.rs: origin-bound authorization for capture operations.
path = 'apps/windows-companion/src/coordination.rs'
replace_once(
    path,
    '''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatResult {
    Refreshed,
    NotFound,
    OriginMismatch,
}
''',
    '''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatResult {
    Refreshed,
    NotFound,
    OriginMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationResult {
    Allowed,
    NotFound,
    OriginMismatch,
    MissingCapability,
}
''',
)
replace_once(
    path,
    '''    pub fn active(&mut self, now: Instant) -> Vec<BrowserInstance> {
''',
    '''    pub fn authorize(
        &mut self,
        instance_id: &str,
        origin: &str,
        capability: BrowserCapability,
        now: Instant,
    ) -> AuthorizationResult {
        self.prune(now);
        let Some(entry) = self.entries.get(instance_id) else {
            return AuthorizationResult::NotFound;
        };
        if entry.origin != origin {
            return AuthorizationResult::OriginMismatch;
        }
        if !entry.registration.capabilities.contains(&capability) {
            return AuthorizationResult::MissingCapability;
        }
        AuthorizationResult::Allowed
    }

    pub fn active(&mut self, now: Instant) -> Vec<BrowserInstance> {
''',
)
replace_once(
    path,
    '''    #[test]
    fn validates_instance_version_and_capabilities() {
''',
    '''    #[test]
    fn authorizes_only_the_registered_origin_and_capability() {
        let start = Instant::now();
        let mut registry = BrowserRegistry::default();
        let mut capture_only = registration(INSTANCE_ID);
        capture_only.capabilities = vec![BrowserCapability::Capture];
        registry.register(capture_only, ORIGIN, start).unwrap();

        assert_eq!(
            registry.authorize(INSTANCE_ID, ORIGIN, BrowserCapability::Capture, start),
            AuthorizationResult::Allowed
        );
        assert_eq!(
            registry.authorize(INSTANCE_ID, ORIGIN, BrowserCapability::Restore, start),
            AuthorizationResult::MissingCapability
        );
        assert_eq!(
            registry.authorize(
                INSTANCE_ID,
                "moz-extension://944cfddf-7a95-3c47-bd9a-663b3ce8d699",
                BrowserCapability::Capture,
                start,
            ),
            AuthorizationResult::OriginMismatch
        );
    }

    #[test]
    fn validates_instance_version_and_capabilities() {
''',
)

# protocol.rs: shared capture control and browser-facing capture endpoints.
path = 'apps/windows-companion/src/protocol.rs'
replace_once(path, 'use std::sync::Mutex;\n', 'use std::sync::{Arc, Mutex};\n')
replace_once(
    path,
    '''use crate::coordination::{
    BROWSER_LEASE_SECONDS, BrowserCapability, BrowserKind, BrowserRegistration, BrowserRegistry,
    COORDINATION_VERSION, HeartbeatResult, RegistryError, valid_instance_id,
};
''',
    '''use crate::capture::{
    CAPTURE_VERSION, CaptureFailure, CaptureJobError, CaptureJobStatus, CaptureJobStore,
    MAX_CAPTURE_RESULT_BYTES, valid_job_id,
};
use crate::coordination::{
    AuthorizationResult, BROWSER_LEASE_SECONDS, BrowserCapability, BrowserKind,
    BrowserRegistration, BrowserRegistry, COORDINATION_VERSION, HeartbeatResult, RegistryError,
    valid_instance_id,
};
''',
)
replace_once(
    path,
    '''const BROWSER_WIRE_PREFIX: &str = "tabsnap-browser:v1";
const MAX_COORDINATION_BODY_BYTES: u64 = 1024;
''',
    '''const BROWSER_WIRE_PREFIX: &str = "tabsnap-browser:v1";
const CAPTURE_WIRE_PREFIX: &str = "tabsnap-capture:v1";
const MAX_COORDINATION_BODY_BYTES: u64 = 1024;
''',
)
replace_once(
    path,
    '''    token: String,
    registry: Mutex<BrowserRegistry>,
}
''',
    '''    token: String,
    registry: Arc<Mutex<BrowserRegistry>>,
    capture_jobs: Arc<Mutex<CaptureJobStore>>,
}

#[derive(Debug, Clone)]
pub struct CaptureControl {
    registry: Arc<Mutex<BrowserRegistry>>,
    capture_jobs: Arc<Mutex<CaptureJobStore>>,
}

impl CaptureControl {
    pub fn create_capture_job(&self) -> io::Result<CaptureJobStatus> {
        let now = Instant::now();
        let instances = self
            .registry
            .lock()
            .map_err(|_| io::Error::other("Browser registry is unavailable."))?
            .active(now);
        let mut jobs = self
            .capture_jobs
            .lock()
            .map_err(|_| io::Error::other("Capture job store is unavailable."))?;

        for _ in 0..4 {
            let job_id = generate_capture_job_id()?;
            match jobs.create(job_id, instances.clone(), now) {
                Ok(status) => return Ok(status),
                Err(CaptureJobError::DuplicateJob) => continue,
                Err(error) => return Err(capture_control_error(error)),
            }
        }
        Err(io::Error::other("Unable to allocate a unique capture job id."))
    }

    pub fn capture_job_status(&self, job_id: &str) -> io::Result<Option<CaptureJobStatus>> {
        let mut jobs = self
            .capture_jobs
            .lock()
            .map_err(|_| io::Error::other("Capture job store is unavailable."))?;
        Ok(jobs.status(job_id, Instant::now()))
    }
}
''',
)
replace_once(
    path,
    '''            token,
            registry: Mutex::new(BrowserRegistry::default()),
        })
''',
    '''            token,
            registry: Arc::new(Mutex::new(BrowserRegistry::default())),
            capture_jobs: Arc::new(Mutex::new(CaptureJobStore::default())),
        })
''',
)
replace_once(
    path,
    '''    pub fn pairing_code(&self) -> io::Result<String> {
''',
    '''    pub fn capture_control(&self) -> CaptureControl {
        CaptureControl {
            registry: Arc::clone(&self.registry),
            capture_jobs: Arc::clone(&self.capture_jobs),
        }
    }

    pub fn pairing_code(&self) -> io::Result<String> {
''',
)
replace_once(
    path,
    '''                    "{{\\"protocolVersion\\":{PROTOCOL_VERSION},\\"transport\\":\\"loopback-http\\",\\"authentication\\":\\"session-bearer\\",\\"coordinationVersion\\":{COORDINATION_VERSION},\\"browserLeaseSeconds\\":{BROWSER_LEASE_SECONDS}}}"
''',
    '''                    "{{\\"protocolVersion\\":{PROTOCOL_VERSION},\\"transport\\":\\"loopback-http\\",\\"authentication\\":\\"session-bearer\\",\\"coordinationVersion\\":{COORDINATION_VERSION},\\"browserLeaseSeconds\\":{BROWSER_LEASE_SECONDS},\\"captureVersion\\":{CAPTURE_VERSION}}}"
''',
)
replace_once(
    path,
    '''            ("GET", "/v1/browsers") => self.list_browsers(),
            _ => HttpResponse::json_error(404, "Unknown protocol endpoint."),
''',
    '''            ("GET", "/v1/browsers") => self.list_browsers(),
            ("POST", "/v1/browser/capture/poll") => {
                self.poll_capture(&request, cors_origin.as_deref())
            }
            ("POST", "/v1/browser/capture/result") => {
                self.submit_capture_result(&request, cors_origin.as_deref())
            }
            ("POST", "/v1/browser/capture/failure") => {
                self.submit_capture_failure(&request, cors_origin.as_deref())
            }
            _ => HttpResponse::json_error(404, "Unknown protocol endpoint."),
''',
)
replace_once(
    path,
    '''    fn authenticated(&self, request: &HttpRequest) -> bool {
''',
    '''    fn poll_capture(&self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        let Some(origin) = origin else {
            return HttpResponse::json_error(403, "Capture polling requires an extension origin.");
        };
        if !is_text_plain(request) {
            return HttpResponse::json_error(415, "Capture polling must be text/plain.");
        }
        let Some(instance_id) = parse_capture_poll(&request.body) else {
            return HttpResponse::json_error(400, "Capture poll is invalid.");
        };
        if let Err(response) = self.authorize_capture(instance_id, origin) {
            return response;
        }

        let Ok(mut jobs) = self.capture_jobs.lock() else {
            return HttpResponse::json_error(500, "Capture job store is unavailable.");
        };
        match jobs.next_assignment(instance_id, Instant::now()) {
            Some(assignment) => HttpResponse::json(
                200,
                format!(
                    "{{\\"protocolVersion\\":{PROTOCOL_VERSION},\\"coordinationVersion\\":{COORDINATION_VERSION},\\"captureVersion\\":{CAPTURE_VERSION},\\"jobId\\":{}}}",
                    json_string(&assignment.job_id)
                ),
            ),
            None => HttpResponse::empty(204),
        }
    }

    fn submit_capture_result(&self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        let Some(origin) = origin else {
            return HttpResponse::json_error(403, "Capture result requires an extension origin.");
        };
        if request.body.is_empty() {
            return HttpResponse::json_error(400, "Capture result is empty.");
        }
        if request
            .header("content-type")
            .is_none_or(|value| !value.eq_ignore_ascii_case("application/octet-stream"))
        {
            return HttpResponse::json_error(
                415,
                "Capture result must be application/octet-stream.",
            );
        }
        let Some(instance_id) = request.header("x-tabsnap-instance") else {
            return HttpResponse::json_error(400, "X-TabSnap-Instance is required.");
        };
        let Some(job_id) = request.header("x-tabsnap-job") else {
            return HttpResponse::json_error(400, "X-TabSnap-Job is required.");
        };
        if !valid_instance_id(instance_id) || !valid_job_id(job_id) {
            return HttpResponse::json_error(400, "Capture result identifiers are invalid.");
        }
        if let Err(response) = self.authorize_capture(instance_id, origin) {
            return response;
        }

        let Ok(mut jobs) = self.capture_jobs.lock() else {
            return HttpResponse::json_error(500, "Capture job store is unavailable.");
        };
        match jobs.submit_result(job_id, instance_id, request.body.clone(), Instant::now()) {
            Ok(()) => HttpResponse::empty(204),
            Err(CaptureJobError::NotFound | CaptureJobError::TargetNotFound) => {
                HttpResponse::json_error(404, "Capture job or target was not found.")
            }
            Err(CaptureJobError::AlreadyFinished) => {
                HttpResponse::json_error(409, "Capture target is already finished.")
            }
            Err(CaptureJobError::EmptyResult) => {
                HttpResponse::json_error(400, "Capture result is empty.")
            }
            Err(CaptureJobError::ResultTooLarge | CaptureJobError::JobBytesExceeded) => {
                HttpResponse::json_error(413, "Capture result exceeds the job limit.")
            }
            Err(_) => HttpResponse::json_error(400, "Capture result was rejected."),
        }
    }

    fn submit_capture_failure(&self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        let Some(origin) = origin else {
            return HttpResponse::json_error(403, "Capture failure requires an extension origin.");
        };
        if !is_text_plain(request) {
            return HttpResponse::json_error(415, "Capture failure must be text/plain.");
        }
        let Some((instance_id, job_id, reason)) = parse_capture_failure(&request.body) else {
            return HttpResponse::json_error(400, "Capture failure is invalid.");
        };
        if let Err(response) = self.authorize_capture(instance_id, origin) {
            return response;
        }

        let Ok(mut jobs) = self.capture_jobs.lock() else {
            return HttpResponse::json_error(500, "Capture job store is unavailable.");
        };
        match jobs.submit_failure(job_id, instance_id, reason, Instant::now()) {
            Ok(()) => HttpResponse::empty(204),
            Err(CaptureJobError::NotFound | CaptureJobError::TargetNotFound) => {
                HttpResponse::json_error(404, "Capture job or target was not found.")
            }
            Err(CaptureJobError::AlreadyFinished) => {
                HttpResponse::json_error(409, "Capture target is already finished.")
            }
            Err(_) => HttpResponse::json_error(400, "Capture failure was rejected."),
        }
    }

    fn authorize_capture(&self, instance_id: &str, origin: &str) -> Result<(), HttpResponse> {
        let Ok(mut registry) = self.registry.lock() else {
            return Err(HttpResponse::json_error(500, "Browser registry is unavailable."));
        };
        match registry.authorize(
            instance_id,
            origin,
            BrowserCapability::Capture,
            Instant::now(),
        ) {
            AuthorizationResult::Allowed => Ok(()),
            AuthorizationResult::NotFound => Err(HttpResponse::json_error(
                404,
                "Browser instance is not registered.",
            )),
            AuthorizationResult::OriginMismatch => Err(HttpResponse::json_error(
                403,
                "Browser instance belongs to another extension origin.",
            )),
            AuthorizationResult::MissingCapability => Err(HttpResponse::json_error(
                409,
                "Browser instance does not advertise capture capability.",
            )),
        }
    }

    fn authenticated(&self, request: &HttpRequest) -> bool {
''',
)
replace_once(
    path,
    '''                "Authorization, Content-Type, X-TabSnap-Name".to_owned(),
''',
    '''                "Authorization, Content-Type, X-TabSnap-Name, X-TabSnap-Instance, X-TabSnap-Job"
                    .to_owned(),
''',
)
replace_once(
    path,
    '''    let body_limit = if matches!(path, "/v1/browser/register" | "/v1/browser/heartbeat") {
        MAX_COORDINATION_BODY_BYTES
    } else {
        MAX_SNAPSHOT_FILE_BYTES
    };
''',
    '''    let body_limit = if matches!(
        path,
        "/v1/browser/register"
            | "/v1/browser/heartbeat"
            | "/v1/browser/capture/poll"
            | "/v1/browser/capture/failure"
    ) {
        MAX_COORDINATION_BODY_BYTES
    } else if path == "/v1/browser/capture/result" {
        MAX_CAPTURE_RESULT_BYTES as u64
    } else {
        MAX_SNAPSHOT_FILE_BYTES
    };
''',
)
replace_once(
    path,
    '''            | "/v1/browsers"
    )
}
''',
    '''            | "/v1/browsers"
            | "/v1/browser/capture/poll"
            | "/v1/browser/capture/result"
            | "/v1/browser/capture/failure"
    )
}
''',
)
replace_once(
    path,
    '''fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
''',
    '''fn parse_capture_poll(body: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(body).ok()?;
    if text.contains('\\r') {
        return None;
    }
    let parts = text.split('\\n').collect::<Vec<_>>();
    if parts.len() != 2 || parts[0] != CAPTURE_WIRE_PREFIX || !valid_instance_id(parts[1]) {
        return None;
    }
    Some(parts[1])
}

fn parse_capture_failure(body: &[u8]) -> Option<(&str, &str, CaptureFailure)> {
    let text = std::str::from_utf8(body).ok()?;
    if text.contains('\\r') {
        return None;
    }
    let parts = text.split('\\n').collect::<Vec<_>>();
    if parts.len() != 4
        || parts[0] != CAPTURE_WIRE_PREFIX
        || !valid_instance_id(parts[1])
        || !valid_job_id(parts[2])
    {
        return None;
    }
    Some((parts[1], parts[2], CaptureFailure::parse(parts[3])?))
}

fn capture_control_error(error: CaptureJobError) -> io::Error {
    match error {
        CaptureJobError::NoTargets => io::Error::new(
            io::ErrorKind::NotFound,
            "No connected browser advertises capture capability.",
        ),
        CaptureJobError::CapacityExceeded => io::Error::other("Capture job capacity is exhausted."),
        CaptureJobError::InvalidJobId | CaptureJobError::DuplicateJob => {
            io::Error::other("Unable to allocate capture job.")
        }
        _ => io::Error::other("Capture job store rejected the operation."),
    }
}

fn generate_capture_job_id() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    fill_random(&mut bytes)?;
    let mut job_id = String::with_capacity(32);
    for byte in bytes {
        job_id.push_str(&format!("{byte:02x}"));
    }
    Ok(job_id)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
''',
)

# Protocol integration coverage for the complete M30 browser flow.
replace_once(
    path,
    '''    #[test]
    fn rejects_browser_registration_without_extension_origin() {
''',
    '''    #[test]
    fn coordinates_capture_results_and_partial_failures() {
        let (server, root) = test_server("capture-job");
        let control = server.capture_control();
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(5).unwrap());

        let chrome_body = format!(
            "{BROWSER_WIRE_PREFIX}\\n0123456789abcdef0123456789abcdef\\nchrome\\n140.0\\ncapture,restore"
        );
        let chrome_register = format!(
            "POST /v1/browser/register HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{chrome_body}",
            chrome_body.len()
        );
        assert!(request(address, chrome_register.as_bytes())
            .starts_with(b"HTTP/1.1 204 No Content\\r\\n"));

        let firefox_id = "fedcba9876543210fedcba9876543210";
        let firefox_body = format!(
            "{BROWSER_WIRE_PREFIX}\\n{firefox_id}\\nfirefox\\n143.0\\ncapture,restore"
        );
        let firefox_register = format!(
            "POST /v1/browser/register HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{firefox_body}",
            firefox_body.len()
        );
        assert!(request(address, firefox_register.as_bytes())
            .starts_with(b"HTTP/1.1 204 No Content\\r\\n"));

        let job = control.create_capture_job().unwrap();
        assert_eq!(job.targets.len(), 2);

        let chrome_poll_body = format!(
            "{CAPTURE_WIRE_PREFIX}\\n0123456789abcdef0123456789abcdef"
        );
        let chrome_poll = format!(
            "POST /v1/browser/capture/poll HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{chrome_poll_body}",
            chrome_poll_body.len()
        );
        let assignment = request(address, chrome_poll.as_bytes());
        assert!(assignment.starts_with(b"HTTP/1.1 200 OK\\r\\n"));
        assert!(String::from_utf8_lossy(response_body(&assignment)).contains(&job.job_id));

        let opaque = b"opaque-encrypted-browser-snapshot";
        let result_head = format!(
            "POST /v1/browser/capture/result HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: application/octet-stream\\r\\nX-TabSnap-Instance: 0123456789abcdef0123456789abcdef\\r\\nX-TabSnap-Job: {}\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n",
            job.job_id,
            opaque.len()
        );
        let mut result_request = result_head.into_bytes();
        result_request.extend_from_slice(opaque);
        assert!(request(address, &result_request)
            .starts_with(b"HTTP/1.1 204 No Content\\r\\n"));

        let failure_body = format!(
            "{CAPTURE_WIRE_PREFIX}\\n{firefox_id}\\n{}\\npassword-required",
            job.job_id
        );
        let failure = format!(
            "POST /v1/browser/capture/failure HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{failure_body}",
            failure_body.len()
        );
        assert!(request(address, failure.as_bytes())
            .starts_with(b"HTTP/1.1 204 No Content\\r\\n"));

        let status = control.capture_job_status(&job.job_id).unwrap().unwrap();
        assert!(status.is_terminal());
        assert_eq!(status.completed_count(), 1);
        assert_eq!(status.failed_count(), 1);

        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn rejects_browser_registration_without_extension_origin() {
''',
)

# main.rs: add a minimal companion-side M30 capture command. M33 owns the GUI later.
path = 'apps/windows-companion/src/main.rs'
replace_once(path, 'pub mod coordination;\n', 'pub mod capture;\npub mod coordination;\n')
replace_once(
    path,
    '''use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
''',
    '''use std::env;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
''',
)
replace_once(
    path,
    '''use library::SnapshotLibrary;
use protocol::ProtocolServer;
''',
    '''use capture::{CaptureJobStatus, CaptureTargetStateView};
use library::SnapshotLibrary;
use protocol::ProtocolServer;
''',
)
replace_once(
    path,
    '''    println!("  tabsnap-companion serve");
''',
    '''    println!("  tabsnap-companion serve");
    println!("  tabsnap-companion capture");
''',
)
replace_once(
    path,
    '''fn run() -> Result<(), Box<dyn Error>> {
''',
    '''fn print_capture_status(status: &CaptureJobStatus) {
    for target in &status.targets {
        let state = match target.state {
            CaptureTargetStateView::Pending => "pending".to_owned(),
            CaptureTargetStateView::Claimed => "capturing".to_owned(),
            CaptureTargetStateView::Complete { bytes } => format!("complete ({bytes} bytes)"),
            CaptureTargetStateView::Failed { reason } => {
                format!("failed ({})", reason.as_str())
            }
        };
        println!(
            "{}\\t{}\\t{}",
            target.instance.browser.as_str(),
            target.instance.instance_id,
            state
        );
    }
}

fn run_capture_command(layout: &PortableLayout) -> Result<(), Box<dyn Error>> {
    let library = snapshot_library(layout)?;
    validate_storage_dir(library.root())?;
    let server = ProtocolServer::bind(library)?;
    let control = server.capture_control();

    println!("TabSnap coordinated capture v1");
    println!("endpoint: {}", server.endpoint()?);
    println!("pairing-code: {}", server.pairing_code()?);
    println!("Connect the browser pages you want to capture, then press Enter.");

    thread::spawn(move || {
        if let Err(error) = server.serve_forever() {
            eprintln!("TabSnap Companion protocol error: {error}");
        }
    });

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let created = control.create_capture_job()?;
    println!("capture-job: {}", created.job_id);
    print_capture_status(&created);

    let mut previous = created;
    loop {
        thread::sleep(Duration::from_millis(250));
        let Some(current) = control.capture_job_status(&previous.job_id)? else {
            return Err("Capture job expired before completion.".into());
        };
        if current != previous {
            println!();
            print_capture_status(&current);
        }
        if current.is_terminal() {
            println!();
            println!(
                "capture complete: {} succeeded, {} failed",
                current.completed_count(),
                current.failed_count()
            );
            println!("M30 results are held in memory only; M31 adds the machine snapshot container.");
            return Ok(());
        }
        previous = current;
    }
}

fn run() -> Result<(), Box<dyn Error>> {
''',
)
replace_once(
    path,
    '''        "serve" => serve(&layout)?,
''',
    '''        "serve" => serve(&layout)?,
        "capture" => run_capture_command(&layout)?,
''',
)
