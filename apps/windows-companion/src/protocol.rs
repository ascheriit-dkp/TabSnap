use std::collections::HashMap;
#[cfg(unix)]
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::coordination::{
    BROWSER_LEASE_SECONDS, BrowserCapability, BrowserKind, BrowserRegistration, BrowserRegistry,
    COORDINATION_VERSION, HeartbeatResult, RegistryError, valid_instance_id,
};
use crate::library::{MAX_SNAPSHOT_FILE_BYTES, SnapshotLibrary};

pub const PROTOCOL_VERSION: u8 = 1;
const TOKEN_BYTES: usize = 32;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(10);
const CHROME_EXTENSION_PREFIX: &str = "chrome-extension://";
const FIREFOX_EXTENSION_PREFIX: &str = "moz-extension://";
const BROWSER_WIRE_PREFIX: &str = "tabsnap-browser:v1";
const MAX_COORDINATION_BODY_BYTES: u64 = 1024;

#[derive(Debug)]
pub struct ProtocolServer {
    listener: TcpListener,
    library: SnapshotLibrary,
    token: String,
    registry: Mutex<BrowserRegistry>,
}

impl ProtocolServer {
    pub fn bind(library: SnapshotLibrary) -> io::Result<Self> {
        Self::bind_with_token(library, generate_session_token()?)
    }

    fn bind_with_token(library: SnapshotLibrary, token: String) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        if address.ip() != Ipv4Addr::LOCALHOST {
            return Err(io::Error::other(
                "Companion protocol did not bind to IPv4 loopback.",
            ));
        }

        Ok(Self {
            listener,
            library,
            token,
            registry: Mutex::new(BrowserRegistry::default()),
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn endpoint(&self) -> io::Result<String> {
        Ok(format!("http://{}", self.local_addr()?))
    }

    pub fn session_token(&self) -> &str {
        &self.token
    }

    pub fn pairing_code(&self) -> io::Result<String> {
        Ok(format!(
            "tabsnap-companion:v{PROTOCOL_VERSION}:{}:{}",
            self.local_addr()?.port(),
            self.token
        ))
    }

    pub fn serve_forever(&self) -> io::Result<()> {
        loop {
            let (stream, peer) = self.listener.accept()?;
            if !peer.ip().is_loopback() {
                continue;
            }
            let _ = self.handle_connection(stream);
        }
    }

    fn handle_connection(&self, mut stream: TcpStream) -> io::Result<()> {
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
        stream.set_write_timeout(Some(IO_TIMEOUT))?;

        let response = match read_request(&mut stream) {
            Ok(request) => self.route(request),
            Err(error) => HttpResponse::json_error(error.status, error.message),
        };

        write_response(&mut stream, response)
    }

    fn route(&self, request: HttpRequest) -> HttpResponse {
        let cors_origin = match request.header("origin") {
            Some(origin) if valid_extension_origin(origin) => Some(origin.to_owned()),
            Some(_) => return HttpResponse::json_error(403, "Browser origin is not allowed."),
            None => None,
        };

        if request.method == "OPTIONS" {
            if cors_origin.is_none() {
                return HttpResponse::json_error(
                    403,
                    "CORS preflight requires an extension origin.",
                );
            }
            if !is_protocol_path(&request.path) {
                return HttpResponse::json_error(404, "Unknown protocol endpoint.")
                    .with_cors(cors_origin.as_deref());
            }
            return HttpResponse::empty(204).with_cors(cors_origin.as_deref());
        }

        if !self.authenticated(&request) {
            return HttpResponse::json_error(401, "Session authentication required.")
                .with_cors(cors_origin.as_deref());
        }

        let response = match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/v1/status") => HttpResponse::json(
                200,
                format!(
                    "{{\"protocolVersion\":{PROTOCOL_VERSION},\"transport\":\"loopback-http\",\"authentication\":\"session-bearer\",\"coordinationVersion\":{COORDINATION_VERSION},\"browserLeaseSeconds\":{BROWSER_LEASE_SECONDS}}}"
                ),
            ),
            ("GET", "/v1/snapshots") => self.list_snapshots(),
            ("POST", "/v1/snapshot") => self.store_snapshot(&request),
            ("GET", "/v1/snapshot") => self.load_snapshot(&request),
            ("POST", "/v1/browser/register") => {
                self.register_browser(&request, cors_origin.as_deref())
            }
            ("POST", "/v1/browser/heartbeat") => {
                self.heartbeat_browser(&request, cors_origin.as_deref())
            }
            ("GET", "/v1/browsers") => self.list_browsers(),
            _ => HttpResponse::json_error(404, "Unknown protocol endpoint."),
        };

        response.with_cors(cors_origin.as_deref())
    }

    fn register_browser(&self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        let Some(origin) = origin else {
            return HttpResponse::json_error(
                403,
                "Browser registration requires an extension origin.",
            );
        };
        if !is_text_plain(request) {
            return HttpResponse::json_error(415, "Browser registration must be text/plain.");
        }
        let Some(registration) = parse_browser_registration(&request.body) else {
            return HttpResponse::json_error(400, "Browser registration is invalid.");
        };

        let Ok(mut registry) = self.registry.lock() else {
            return HttpResponse::json_error(500, "Browser registry is unavailable.");
        };
        match registry.register(registration, origin, Instant::now()) {
            Ok(()) => HttpResponse::empty(204),
            Err(RegistryError::InvalidRegistration) => {
                HttpResponse::json_error(400, "Browser registration is invalid.")
            }
            Err(RegistryError::OriginMismatch) => HttpResponse::json_error(
                409,
                "Browser instance belongs to another extension origin.",
            ),
            Err(RegistryError::CapacityExceeded) => {
                HttpResponse::json_error(429, "Browser registry capacity is exhausted.")
            }
        }
    }

    fn heartbeat_browser(&self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        let Some(origin) = origin else {
            return HttpResponse::json_error(
                403,
                "Browser heartbeat requires an extension origin.",
            );
        };
        if !is_text_plain(request) {
            return HttpResponse::json_error(415, "Browser heartbeat must be text/plain.");
        }
        let Some(instance_id) = parse_browser_heartbeat(&request.body) else {
            return HttpResponse::json_error(400, "Browser heartbeat is invalid.");
        };

        let Ok(mut registry) = self.registry.lock() else {
            return HttpResponse::json_error(500, "Browser registry is unavailable.");
        };
        match registry.heartbeat(instance_id, origin, Instant::now()) {
            HeartbeatResult::Refreshed => HttpResponse::empty(204),
            HeartbeatResult::NotFound => {
                HttpResponse::json_error(404, "Browser instance is not registered.")
            }
            HeartbeatResult::OriginMismatch => HttpResponse::json_error(
                403,
                "Browser instance belongs to another extension origin.",
            ),
        }
    }

    fn list_browsers(&self) -> HttpResponse {
        let Ok(mut registry) = self.registry.lock() else {
            return HttpResponse::json_error(500, "Browser registry is unavailable.");
        };
        let browsers = registry
            .active(Instant::now())
            .into_iter()
            .map(|instance| {
                let version = instance
                    .version
                    .as_deref()
                    .map(json_string)
                    .unwrap_or_else(|| "null".to_owned());
                let capabilities = instance
                    .capabilities
                    .into_iter()
                    .map(|capability| json_string(capability.as_str()))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{{\"instanceId\":{},\"browser\":{},\"version\":{version},\"capabilities\":[{capabilities}]}}",
                    json_string(&instance.instance_id),
                    json_string(instance.browser.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        HttpResponse::json(
            200,
            format!(
                "{{\"protocolVersion\":{PROTOCOL_VERSION},\"coordinationVersion\":{COORDINATION_VERSION},\"browserLeaseSeconds\":{BROWSER_LEASE_SECONDS},\"browsers\":[{browsers}]}}"
            ),
        )
    }

    fn authenticated(&self, request: &HttpRequest) -> bool {
        let Some(value) = request.header("authorization") else {
            return false;
        };
        let expected = format!("Bearer {}", self.token);
        constant_time_eq(value.as_bytes(), expected.as_bytes())
    }

    fn list_snapshots(&self) -> HttpResponse {
        match self.library.list() {
            Ok(entries) => {
                let snapshots = entries
                    .into_iter()
                    .map(|entry| {
                        format!(
                            "{{\"name\":{},\"size\":{}}}",
                            json_string(&entry.file_name),
                            entry.size
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                HttpResponse::json(
                    200,
                    format!(
                        "{{\"protocolVersion\":{PROTOCOL_VERSION},\"snapshots\":[{snapshots}]}}"
                    ),
                )
            }
            Err(_) => HttpResponse::json_error(500, "Unable to read snapshot library."),
        }
    }

    fn store_snapshot(&self, request: &HttpRequest) -> HttpResponse {
        if request.body.is_empty() {
            return HttpResponse::json_error(400, "Snapshot body is empty.");
        }
        if request
            .header("content-type")
            .is_none_or(|value| !value.eq_ignore_ascii_case("application/octet-stream"))
        {
            return HttpResponse::json_error(
                415,
                "Snapshot body must be application/octet-stream.",
            );
        }
        let Some(name) = request.header("x-tabsnap-name") else {
            return HttpResponse::json_error(400, "X-TabSnap-Name is required.");
        };
        if name.trim().is_empty() {
            return HttpResponse::json_error(400, "X-TabSnap-Name must not be empty.");
        }

        match self.library.write_snapshot(name, &request.body) {
            Ok(entry) => HttpResponse::json(
                201,
                format!(
                    "{{\"name\":{},\"size\":{}}}",
                    json_string(&entry.file_name),
                    entry.size
                ),
            ),
            Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
                HttpResponse::json_error(400, "Snapshot was rejected by the library.")
            }
            Err(_) => HttpResponse::json_error(500, "Unable to store snapshot."),
        }
    }

    fn load_snapshot(&self, request: &HttpRequest) -> HttpResponse {
        let Some(name) = request.header("x-tabsnap-name") else {
            return HttpResponse::json_error(400, "X-TabSnap-Name is required.");
        };

        let entry = match self.library.list() {
            Ok(entries) => entries
                .into_iter()
                .find(|entry| entry.file_name.eq_ignore_ascii_case(name)),
            Err(_) => return HttpResponse::json_error(500, "Unable to read snapshot library."),
        };
        let Some(entry) = entry else {
            return HttpResponse::json_error(404, "Snapshot not found.");
        };

        match std::fs::read(&entry.path) {
            Ok(bytes) if bytes.len() as u64 <= MAX_SNAPSHOT_FILE_BYTES => {
                HttpResponse::binary(200, bytes).with_header("X-TabSnap-Name", entry.file_name)
            }
            Ok(_) => HttpResponse::json_error(413, "Snapshot exceeds the protocol file limit."),
            Err(_) => HttpResponse::json_error(500, "Unable to read snapshot."),
        }
    }

    #[cfg(test)]
    fn serve_n(&self, count: usize) -> io::Result<()> {
        for _ in 0..count {
            let (stream, peer) = self.listener.accept()?;
            if !peer.ip().is_loopback() {
                continue;
            }
            self.handle_connection(stream)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

#[derive(Debug)]
struct RequestError {
    status: u16,
    message: &'static str,
}

impl RequestError {
    fn new(status: u16, message: &'static str) -> Self {
        Self { status, message }
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
}

impl HttpResponse {
    fn empty(status: u16) -> Self {
        Self {
            status,
            content_type: "application/octet-stream",
            body: Vec::new(),
            headers: Vec::new(),
        }
    }

    fn json(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: body.into_bytes(),
            headers: Vec::new(),
        }
    }

    fn json_error(status: u16, message: &str) -> Self {
        Self::json(status, format!("{{\"error\":{}}}", json_string(message)))
    }

    fn binary(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type: "application/octet-stream",
            body,
            headers: Vec::new(),
        }
    }

    fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    fn with_cors(mut self, origin: Option<&str>) -> Self {
        if let Some(origin) = origin {
            self.headers
                .push(("Access-Control-Allow-Origin".to_owned(), origin.to_owned()));
            self.headers.push((
                "Access-Control-Allow-Methods".to_owned(),
                "GET, POST, OPTIONS".to_owned(),
            ));
            self.headers.push((
                "Access-Control-Allow-Headers".to_owned(),
                "Authorization, Content-Type, X-TabSnap-Name".to_owned(),
            ));
            self.headers.push((
                "Access-Control-Allow-Private-Network".to_owned(),
                "true".to_owned(),
            ));
            self.headers.push(("Vary".to_owned(), "Origin".to_owned()));
        }
        self
    }
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, RequestError> {
    let mut received = Vec::with_capacity(4096);
    let header_end = loop {
        if let Some(position) = find_bytes(&received, b"\r\n\r\n") {
            break position + 4;
        }
        if received.len() >= MAX_HEADER_BYTES {
            return Err(RequestError::new(431, "Request headers are too large."));
        }

        let mut chunk = [0_u8; 4096];
        let read = stream
            .read(&mut chunk)
            .map_err(|_| RequestError::new(400, "Unable to read request."))?;
        if read == 0 {
            return Err(RequestError::new(
                400,
                "Request ended before headers completed.",
            ));
        }
        received.extend_from_slice(&chunk[..read]);
        if received.len() > MAX_HEADER_BYTES + MAX_SNAPSHOT_FILE_BYTES as usize {
            return Err(RequestError::new(413, "Request is too large."));
        }
    };

    if header_end > MAX_HEADER_BYTES {
        return Err(RequestError::new(431, "Request headers are too large."));
    }

    let header_text = std::str::from_utf8(&received[..header_end - 4])
        .map_err(|_| RequestError::new(400, "Request headers are not valid UTF-8."))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| RequestError::new(400, "Request line is missing."))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| RequestError::new(400, "Request method is missing."))?;
    let path = request_parts
        .next()
        .ok_or_else(|| RequestError::new(400, "Request path is missing."))?;
    let version = request_parts
        .next()
        .ok_or_else(|| RequestError::new(400, "HTTP version is missing."))?;
    if request_parts.next().is_some() || version != "HTTP/1.1" {
        return Err(RequestError::new(
            400,
            "Only HTTP/1.1 requests are accepted.",
        ));
    }
    if !matches!(method, "GET" | "POST" | "OPTIONS") {
        return Err(RequestError::new(405, "HTTP method is not allowed."));
    }
    if !path.starts_with('/') || path.contains('#') {
        return Err(RequestError::new(400, "Invalid protocol path."));
    }

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| RequestError::new(400, "Malformed request header."))?;
        if name.is_empty() || !name.bytes().all(is_header_name_byte) {
            return Err(RequestError::new(400, "Malformed request header name."));
        }
        let name = name.to_ascii_lowercase();
        if headers.insert(name, value.trim().to_owned()).is_some() {
            return Err(RequestError::new(400, "Duplicate request header."));
        }
    }

    if headers.contains_key("transfer-encoding") {
        return Err(RequestError::new(
            400,
            "Transfer-Encoding is not supported.",
        ));
    }

    let content_length = match headers.get("content-length") {
        Some(value) => value
            .parse::<usize>()
            .map_err(|_| RequestError::new(400, "Invalid Content-Length."))?,
        None => 0,
    };
    let body_limit = if matches!(path, "/v1/browser/register" | "/v1/browser/heartbeat") {
        MAX_COORDINATION_BODY_BYTES
    } else {
        MAX_SNAPSHOT_FILE_BYTES
    };
    if content_length as u64 > body_limit {
        return Err(RequestError::new(
            413,
            "Request body exceeds the protocol limit.",
        ));
    }
    if method == "POST" && !headers.contains_key("content-length") {
        return Err(RequestError::new(
            411,
            "POST requests require Content-Length.",
        ));
    }

    let mut body = received[header_end..].to_vec();
    if body.len() > content_length {
        return Err(RequestError::new(
            400,
            "Request contains bytes beyond Content-Length.",
        ));
    }
    while body.len() < content_length {
        let remaining = content_length - body.len();
        let mut chunk = vec![0_u8; remaining.min(16 * 1024)];
        let read = stream
            .read(&mut chunk)
            .map_err(|_| RequestError::new(400, "Unable to read request body."))?;
        if read == 0 {
            return Err(RequestError::new(400, "Request body ended early."));
        }
        body.extend_from_slice(&chunk[..read]);
    }

    Ok(HttpRequest {
        method: method.to_owned(),
        path: path.to_owned(),
        headers,
        body,
    })
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> io::Result<()> {
    let reason = status_reason(response.status);
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n",
        response.status,
        reason,
        response.body.len(),
        response.content_type
    );
    for (name, value) in response.headers {
        if !header_value_is_safe(&value) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Response header contains a line break.",
            ));
        }
        head.push_str(&name);
        head.push_str(": ");
        head.push_str(&value);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    stream.write_all(head.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn is_protocol_path(path: &str) -> bool {
    matches!(
        path,
        "/v1/status"
            | "/v1/snapshots"
            | "/v1/snapshot"
            | "/v1/browser/register"
            | "/v1/browser/heartbeat"
            | "/v1/browsers"
    )
}

fn valid_extension_origin(origin: &str) -> bool {
    valid_chromium_extension_origin(origin) || valid_firefox_extension_origin(origin)
}

fn valid_chromium_extension_origin(origin: &str) -> bool {
    let Some(id) = origin.strip_prefix(CHROME_EXTENSION_PREFIX) else {
        return false;
    };
    id.len() == 32 && id.bytes().all(|byte| matches!(byte, b'a'..=b'p'))
}

fn valid_firefox_extension_origin(origin: &str) -> bool {
    let Some(id) = origin.strip_prefix(FIREFOX_EXTENSION_PREFIX) else {
        return false;
    };
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }

    bytes.iter().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            *byte == b'-'
        } else {
            matches!(*byte, b'0'..=b'9' | b'a'..=b'f')
        }
    })
}

fn is_text_plain(request: &HttpRequest) -> bool {
    request.header("content-type").is_some_and(|value| {
        value
            .split(';')
            .next()
            .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("text/plain"))
    })
}

fn parse_browser_registration(body: &[u8]) -> Option<BrowserRegistration> {
    let text = std::str::from_utf8(body).ok()?;
    if text.contains('\r') {
        return None;
    }
    let parts = text.split('\n').collect::<Vec<_>>();
    if parts.len() != 5 || parts[0] != BROWSER_WIRE_PREFIX {
        return None;
    }

    let instance_id = parts[1].to_owned();
    let browser = BrowserKind::parse(parts[2])?;
    let version = match parts[3] {
        "-" => None,
        value => Some(value.to_owned()),
    };
    let capabilities = parts[4]
        .split(',')
        .map(BrowserCapability::parse)
        .collect::<Option<Vec<_>>>()?;

    let registration = BrowserRegistration {
        instance_id,
        browser,
        version,
        capabilities,
    };
    registration.validate().then_some(registration)
}

fn parse_browser_heartbeat(body: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(body).ok()?;
    if text.contains('\r') {
        return None;
    }
    let parts = text.split('\n').collect::<Vec<_>>();
    if parts.len() != 2 || parts[0] != BROWSER_WIRE_PREFIX || !valid_instance_id(parts[1]) {
        return None;
    }
    Some(parts[1])
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn is_header_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn header_value_is_safe(value: &str) -> bool {
    !value.contains(['\r', '\n'])
}

fn status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        _ => "Internal Server Error",
    }
}

fn generate_session_token() -> io::Result<String> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    fill_random(&mut bytes)?;
    let mut token = String::with_capacity(TOKEN_BYTES * 2);
    for byte in bytes {
        token.push_str(&format!("{byte:02x}"));
    }
    Ok(token)
}

#[cfg(windows)]
fn fill_random(bytes: &mut [u8]) -> io::Result<()> {
    use std::ffi::c_void;
    use std::ptr;

    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;

    #[link(name = "bcrypt")]
    unsafe extern "system" {
        #[link_name = "BCryptGenRandom"]
        fn bcrypt_gen_random(
            algorithm: *mut c_void,
            buffer: *mut u8,
            buffer_length: u32,
            flags: u32,
        ) -> i32;
    }

    let length = u32::try_from(bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Random buffer is too large."))?;
    let status = unsafe {
        bcrypt_gen_random(
            ptr::null_mut(),
            bytes.as_mut_ptr(),
            length,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status != 0 {
        return Err(io::Error::other(format!(
            "BCryptGenRandom failed with NTSTATUS 0x{:08x}.",
            status as u32
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn fill_random(bytes: &mut [u8]) -> io::Result<()> {
    File::open("/dev/urandom")?.read_exact(bytes)
}

#[cfg(not(any(windows, unix)))]
fn fill_random(_bytes: &mut [u8]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Secure OS randomness is not implemented on this platform.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    const TEST_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const EXTENSION_ORIGIN: &str = "chrome-extension://abcdefghijklmnopabcdefghijklmnop";
    const FIREFOX_EXTENSION_ORIGIN: &str = "moz-extension://944cfddf-7a95-3c47-bd9a-663b3ce8d699";

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "tabsnap-protocol-{label}-{}-{nonce}",
            process::id()
        ))
    }

    fn request(address: SocketAddr, request: &[u8]) -> Vec<u8> {
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(request).unwrap();
        stream.flush().unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        response
    }

    fn response_body(response: &[u8]) -> &[u8] {
        let start = find_bytes(response, b"\r\n\r\n").unwrap() + 4;
        &response[start..]
    }

    fn test_server(label: &str) -> (ProtocolServer, std::path::PathBuf) {
        let root = temp_root(label);
        let library = SnapshotLibrary::new(&root);
        let server = ProtocolServer::bind_with_token(library, TEST_TOKEN.to_owned()).unwrap();
        (server, root)
    }

    #[test]
    fn binds_only_to_ipv4_loopback_and_generates_256_bit_token() {
        let root = temp_root("bind");
        let server = ProtocolServer::bind(SnapshotLibrary::new(&root)).unwrap();

        assert_eq!(server.local_addr().unwrap().ip(), Ipv4Addr::LOCALHOST);
        assert_eq!(server.session_token().len(), TOKEN_BYTES * 2);
        assert!(
            server
                .session_token()
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
        assert!(
            server
                .pairing_code()
                .unwrap()
                .starts_with("tabsnap-companion:v1:")
        );
    }

    #[test]
    fn rejects_unauthenticated_requests() {
        let (server, root) = test_server("unauthorized");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(1).unwrap());

        let response = request(
            address,
            b"GET /v1/status HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );

        assert!(response.starts_with(b"HTTP/1.1 401 Unauthorized\r\n"));
        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn rejects_non_extension_browser_origins_even_with_token() {
        let (server, root) = test_server("origin");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(1).unwrap());
        let request_text = format!(
            "GET /v1/status HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: https://example.com\r\nAuthorization: Bearer {TEST_TOKEN}\r\nConnection: close\r\n\r\n"
        );

        let response = request(address, request_text.as_bytes());

        assert!(response.starts_with(b"HTTP/1.1 403 Forbidden\r\n"));
        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn supports_extension_preflight_without_performing_an_operation() {
        let (server, root) = test_server("preflight");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(1).unwrap());
        let request_text = format!(
            "OPTIONS /v1/snapshot HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {EXTENSION_ORIGIN}\r\nAccess-Control-Request-Method: POST\r\nConnection: close\r\n\r\n"
        );

        let response = request(address, request_text.as_bytes());
        let response_text = String::from_utf8(response).unwrap();

        assert!(response_text.starts_with("HTTP/1.1 204 No Content\r\n"));
        assert!(response_text.contains(&format!(
            "Access-Control-Allow-Origin: {EXTENSION_ORIGIN}\r\n"
        )));
        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn supports_firefox_extension_preflight() {
        let (server, root) = test_server("firefox-preflight");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(1).unwrap());
        let request_text = format!(
            "OPTIONS /v1/status HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\r\nAccess-Control-Request-Method: GET\r\nConnection: close\r\n\r\n"
        );

        let response = request(address, request_text.as_bytes());
        let response_text = String::from_utf8(response).unwrap();

        assert!(response_text.starts_with("HTTP/1.1 204 No Content\r\n"));
        assert!(response_text.contains(&format!(
            "Access-Control-Allow-Origin: {FIREFOX_EXTENSION_ORIGIN}\r\n"
        )));
        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn registers_heartbeats_and_lists_browser_instances() {
        let (server, root) = test_server("browser-registry");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(3).unwrap());
        let body = format!(
            "{BROWSER_WIRE_PREFIX}\n0123456789abcdef0123456789abcdef\nfirefox\n143.0\ncapture,restore"
        );
        let register = format!(
            "POST /v1/browser/register HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\r\nAuthorization: Bearer {TEST_TOKEN}\r\nContent-Type: text/plain;charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let registered = request(address, register.as_bytes());
        assert!(registered.starts_with(b"HTTP/1.1 204 No Content\r\n"));

        let heartbeat_body = format!("{BROWSER_WIRE_PREFIX}\n0123456789abcdef0123456789abcdef");
        let heartbeat = format!(
            "POST /v1/browser/heartbeat HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\r\nAuthorization: Bearer {TEST_TOKEN}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{heartbeat_body}",
            heartbeat_body.len()
        );
        let refreshed = request(address, heartbeat.as_bytes());
        assert!(refreshed.starts_with(b"HTTP/1.1 204 No Content\r\n"));

        let list = format!(
            "GET /v1/browsers HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\r\nAuthorization: Bearer {TEST_TOKEN}\r\nConnection: close\r\n\r\n"
        );
        let listed = request(address, list.as_bytes());
        assert!(listed.starts_with(b"HTTP/1.1 200 OK\r\n"));
        let body = String::from_utf8_lossy(response_body(&listed));
        assert!(body.contains("\"coordinationVersion\":1"));
        assert!(body.contains("\"browser\":\"firefox\""));
        assert!(body.contains("\"capabilities\":[\"capture\",\"restore\"]"));

        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn rejects_browser_registration_without_extension_origin() {
        let (server, root) = test_server("browser-origin");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(1).unwrap());
        let body = format!(
            "{BROWSER_WIRE_PREFIX}\n0123456789abcdef0123456789abcdef\nchrome\n140.0\ncapture,restore"
        );
        let register = format!(
            "POST /v1/browser/register HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TEST_TOKEN}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );

        let response = request(address, register.as_bytes());
        assert!(response.starts_with(b"HTTP/1.1 403 Forbidden\r\n"));

        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn stores_lists_and_loads_opaque_snapshot_over_loopback() {
        let (server, root) = test_server("roundtrip");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(3).unwrap());
        let opaque = b"encrypted snapshot bytes\0\xff";

        let post_head = format!(
            "POST /v1/snapshot HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TEST_TOKEN}\r\nContent-Type: application/octet-stream\r\nX-TabSnap-Name: Work Session\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            opaque.len()
        );
        let mut post = post_head.into_bytes();
        post.extend_from_slice(opaque);
        let stored = request(address, &post);
        assert!(stored.starts_with(b"HTTP/1.1 201 Created\r\n"));
        assert!(String::from_utf8_lossy(response_body(&stored)).contains("Work Session.tabsnap"));

        let list_request = format!(
            "GET /v1/snapshots HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TEST_TOKEN}\r\nConnection: close\r\n\r\n"
        );
        let listed = request(address, list_request.as_bytes());
        assert!(listed.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(String::from_utf8_lossy(response_body(&listed)).contains("Work Session.tabsnap"));

        let get_request = format!(
            "GET /v1/snapshot HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TEST_TOKEN}\r\nX-TabSnap-Name: Work Session.tabsnap\r\nConnection: close\r\n\r\n"
        );
        let loaded = request(address, get_request.as_bytes());
        assert!(loaded.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert_eq!(response_body(&loaded), opaque);

        worker.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_request_body_over_protocol_limit_before_reading_it() {
        let (server, root) = test_server("too-large");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(1).unwrap());
        let request_text = format!(
            "POST /v1/snapshot HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TEST_TOKEN}\r\nContent-Type: application/octet-stream\r\nX-TabSnap-Name: huge\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            MAX_SNAPSHOT_FILE_BYTES + 1
        );

        let response = request(address, request_text.as_bytes());
        assert!(response.starts_with(b"HTTP/1.1 413 Content Too Large\r\n"));

        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }
}
