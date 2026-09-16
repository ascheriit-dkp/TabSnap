from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, got {count}")
    return text.replace(old, new, 1)


main_path = Path("apps/windows-companion/src/main.rs")
main = main_path.read_text()
if "pub mod coordination;" not in main:
    main = replace_once(main, "pub mod library;\n", "pub mod coordination;\npub mod library;\n", "main module")
    main_path.write_text(main)

path = Path("apps/windows-companion/src/protocol.rs")
text = path.read_text()
if "fn register_browser(&self" in text:
    raise SystemExit(0)

text = replace_once(
    text,
    "use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};\nuse std::time::Duration;\n\nuse crate::library::{MAX_SNAPSHOT_FILE_BYTES, SnapshotLibrary};\n",
    "use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};\nuse std::sync::Mutex;\nuse std::time::{Duration, Instant};\n\nuse crate::coordination::{\n    BROWSER_LEASE_SECONDS, BrowserCapability, BrowserKind, BrowserRegistration, BrowserRegistry,\n    COORDINATION_VERSION, HeartbeatResult, RegistryError, valid_instance_id,\n};\nuse crate::library::{MAX_SNAPSHOT_FILE_BYTES, SnapshotLibrary};\n",
    "imports",
)
text = replace_once(
    text,
    'const FIREFOX_EXTENSION_PREFIX: &str = "moz-extension://";\n',
    'const FIREFOX_EXTENSION_PREFIX: &str = "moz-extension://";\nconst BROWSER_WIRE_PREFIX: &str = "tabsnap-browser:v1";\nconst MAX_COORDINATION_BODY_BYTES: u64 = 1024;\n',
    "constants",
)
text = replace_once(
    text,
    "    library: SnapshotLibrary,\n    token: String,\n",
    "    library: SnapshotLibrary,\n    token: String,\n    registry: Mutex<BrowserRegistry>,\n",
    "server fields",
)
text = replace_once(
    text,
    "            listener,\n            library,\n            token,\n",
    "            listener,\n            library,\n            token,\n            registry: Mutex::new(BrowserRegistry::default()),\n",
    "server init",
)
text = replace_once(
    text,
    '                    "{{\\\"protocolVersion\\\":{PROTOCOL_VERSION},\\\"transport\\\":\\\"loopback-http\\\",\\\"authentication\\\":\\\"session-bearer\\\"}}"\n',
    '                    "{{\\\"protocolVersion\\\":{PROTOCOL_VERSION},\\\"transport\\\":\\\"loopback-http\\\",\\\"authentication\\\":\\\"session-bearer\\\",\\\"coordinationVersion\\\":{COORDINATION_VERSION},\\\"browserLeaseSeconds\\\":{BROWSER_LEASE_SECONDS}}}"\n',
    "status payload",
)
text = replace_once(
    text,
    '            ("GET", "/v1/snapshots") => self.list_snapshots(),\n            ("POST", "/v1/snapshot") => self.store_snapshot(&request),\n            ("GET", "/v1/snapshot") => self.load_snapshot(&request),\n',
    '            ("GET", "/v1/snapshots") => self.list_snapshots(),\n            ("POST", "/v1/snapshot") => self.store_snapshot(&request),\n            ("GET", "/v1/snapshot") => self.load_snapshot(&request),\n            ("POST", "/v1/browser/register") => {\n                self.register_browser(&request, cors_origin.as_deref())\n            }\n            ("POST", "/v1/browser/heartbeat") => {\n                self.heartbeat_browser(&request, cors_origin.as_deref())\n            }\n            ("GET", "/v1/browsers") => self.list_browsers(),\n',
    "routes",
)

methods_anchor = "    fn authenticated(&self, request: &HttpRequest) -> bool {\n"
methods = '''    fn register_browser(&self, request: &HttpRequest, origin: Option<&str>) -> HttpResponse {
        let Some(origin) = origin else {
            return HttpResponse::json_error(403, "Browser registration requires an extension origin.");
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
            return HttpResponse::json_error(403, "Browser heartbeat requires an extension origin.");
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
                    "{{\\\"instanceId\\\":{},\\\"browser\\\":{},\\\"version\\\":{version},\\\"capabilities\\\":[{capabilities}]}}",
                    json_string(&instance.instance_id),
                    json_string(instance.browser.as_str()),
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        HttpResponse::json(
            200,
            format!(
                "{{\\\"protocolVersion\\\":{PROTOCOL_VERSION},\\\"coordinationVersion\\\":{COORDINATION_VERSION},\\\"browserLeaseSeconds\\\":{BROWSER_LEASE_SECONDS},\\\"browsers\\\":[{browsers}]}}"
            ),
        )
    }

'''
text = replace_once(text, methods_anchor, methods + methods_anchor, "server methods")

text = replace_once(
    text,
    '''    if content_length as u64 > MAX_SNAPSHOT_FILE_BYTES {
        return Err(RequestError::new(
            413,
            "Request body exceeds the protocol limit.",
        ));
    }
''',
    '''    let body_limit = if matches!(path, "/v1/browser/register" | "/v1/browser/heartbeat") {
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
''',
    "body limit",
)
text = replace_once(
    text,
    'fn is_protocol_path(path: &str) -> bool {\n    matches!(path, "/v1/status" | "/v1/snapshots" | "/v1/snapshot")\n}\n',
    '''fn is_protocol_path(path: &str) -> bool {
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
''',
    "protocol paths",
)

helper_anchor = "fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {\n"
helpers = '''fn is_text_plain(request: &HttpRequest) -> bool {
    request.header("content-type").is_some_and(|value| {
        value
            .split(';')
            .next()
            .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("text/plain"))
    })
}

fn parse_browser_registration(body: &[u8]) -> Option<BrowserRegistration> {
    let text = std::str::from_utf8(body).ok()?;
    if text.contains('\\r') {
        return None;
    }
    let parts = text.split('\\n').collect::<Vec<_>>();
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
    if text.contains('\\r') {
        return None;
    }
    let parts = text.split('\\n').collect::<Vec<_>>();
    if parts.len() != 2 || parts[0] != BROWSER_WIRE_PREFIX || !valid_instance_id(parts[1]) {
        return None;
    }
    Some(parts[1])
}

'''
text = replace_once(text, helper_anchor, helpers + helper_anchor, "helpers")
text = replace_once(
    text,
    '        405 => "Method Not Allowed",\n        411 => "Length Required",\n',
    '        405 => "Method Not Allowed",\n        409 => "Conflict",\n        411 => "Length Required",\n',
    "409 reason",
)
text = replace_once(
    text,
    '        431 => "Request Header Fields Too Large",\n',
    '        429 => "Too Many Requests",\n        431 => "Request Header Fields Too Large",\n',
    "429 reason",
)

test_anchor = "    #[test]\n    fn stores_lists_and_loads_opaque_snapshot_over_loopback() {\n"
tests = '''    #[test]
    fn registers_heartbeats_and_lists_browser_instances() {
        let (server, root) = test_server("browser-registry");
        let address = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.serve_n(3).unwrap());
        let body = format!(
            "{BROWSER_WIRE_PREFIX}\\n0123456789abcdef0123456789abcdef\\nfirefox\\n143.0\\ncapture,restore"
        );
        let register = format!(
            "POST /v1/browser/register HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: text/plain;charset=UTF-8\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{body}",
            body.len()
        );
        let registered = request(address, register.as_bytes());
        assert!(registered.starts_with(b"HTTP/1.1 204 No Content\\r\\n"));

        let heartbeat_body =
            format!("{BROWSER_WIRE_PREFIX}\\n0123456789abcdef0123456789abcdef");
        let heartbeat = format!(
            "POST /v1/browser/heartbeat HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{heartbeat_body}",
            heartbeat_body.len()
        );
        let refreshed = request(address, heartbeat.as_bytes());
        assert!(refreshed.starts_with(b"HTTP/1.1 204 No Content\\r\\n"));

        let list = format!(
            "GET /v1/browsers HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nOrigin: {FIREFOX_EXTENSION_ORIGIN}\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nConnection: close\\r\\n\\r\\n"
        );
        let listed = request(address, list.as_bytes());
        assert!(listed.starts_with(b"HTTP/1.1 200 OK\\r\\n"));
        let body = String::from_utf8_lossy(response_body(&listed));
        assert!(body.contains("\\\"coordinationVersion\\\":1"));
        assert!(body.contains("\\\"browser\\\":\\\"firefox\\\""));
        assert!(body.contains("\\\"capabilities\\\":[\\\"capture\\\",\\\"restore\\\"]"));

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
            "{BROWSER_WIRE_PREFIX}\\n0123456789abcdef0123456789abcdef\\nchrome\\n140.0\\ncapture,restore"
        );
        let register = format!(
            "POST /v1/browser/register HTTP/1.1\\r\\nHost: 127.0.0.1\\r\\nAuthorization: Bearer {TEST_TOKEN}\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{body}",
            body.len()
        );

        let response = request(address, register.as_bytes());
        assert!(response.starts_with(b"HTTP/1.1 403 Forbidden\\r\\n"));

        worker.join().unwrap();
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

'''
text = replace_once(text, test_anchor, tests + test_anchor, "tests")

path.write_text(text)
