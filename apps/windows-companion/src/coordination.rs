use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const COORDINATION_VERSION: u8 = 1;
pub const BROWSER_LEASE_SECONDS: u64 = 30;
pub const BROWSER_LEASE: Duration = Duration::from_secs(BROWSER_LEASE_SECONDS);
pub const MAX_BROWSER_INSTANCES: usize = 32;
pub const MAX_BROWSER_VERSION_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserKind {
    Chrome,
    Edge,
    Firefox,
}

impl BrowserKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "chrome" => Some(Self::Chrome),
            "edge" => Some(Self::Edge),
            "firefox" => Some(Self::Firefox),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Edge => "edge",
            Self::Firefox => "firefox",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserCapability {
    Capture,
    Restore,
}

impl BrowserCapability {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "capture" => Some(Self::Capture),
            "restore" => Some(Self::Restore),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::Restore => "restore",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserRegistration {
    pub instance_id: String,
    pub browser: BrowserKind,
    pub version: Option<String>,
    pub capabilities: Vec<BrowserCapability>,
}

impl BrowserRegistration {
    pub fn validate(&self) -> bool {
        if !valid_instance_id(&self.instance_id) {
            return false;
        }
        if self
            .version
            .as_deref()
            .is_some_and(|version| !valid_browser_version(version))
        {
            return false;
        }
        if self.capabilities.is_empty() || self.capabilities.len() > 2 {
            return false;
        }

        let mut capture = false;
        let mut restore = false;
        for capability in &self.capabilities {
            let seen = match capability {
                BrowserCapability::Capture => &mut capture,
                BrowserCapability::Restore => &mut restore,
            };
            if *seen {
                return false;
            }
            *seen = true;
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserInstance {
    pub instance_id: String,
    pub browser: BrowserKind,
    pub version: Option<String>,
    pub capabilities: Vec<BrowserCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    InvalidRegistration,
    OriginMismatch,
    CapacityExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatResult {
    Refreshed,
    NotFound,
    OriginMismatch,
}

#[derive(Debug)]
struct RegistryEntry {
    registration: BrowserRegistration,
    origin: String,
    last_seen: Instant,
}

#[derive(Debug)]
pub struct BrowserRegistry {
    entries: HashMap<String, RegistryEntry>,
    lease: Duration,
}

impl Default for BrowserRegistry {
    fn default() -> Self {
        Self::new(BROWSER_LEASE)
    }
}

impl BrowserRegistry {
    pub fn new(lease: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            lease,
        }
    }

    pub fn register(
        &mut self,
        registration: BrowserRegistration,
        origin: &str,
        now: Instant,
    ) -> Result<(), RegistryError> {
        self.prune(now);

        if !registration.validate() || origin.is_empty() {
            return Err(RegistryError::InvalidRegistration);
        }

        if let Some(existing) = self.entries.get(&registration.instance_id)
            && existing.origin != origin
        {
            return Err(RegistryError::OriginMismatch);
        }

        if !self.entries.contains_key(&registration.instance_id)
            && self.entries.len() >= MAX_BROWSER_INSTANCES
        {
            return Err(RegistryError::CapacityExceeded);
        }

        self.entries.insert(
            registration.instance_id.clone(),
            RegistryEntry {
                registration,
                origin: origin.to_owned(),
                last_seen: now,
            },
        );
        Ok(())
    }

    pub fn heartbeat(&mut self, instance_id: &str, origin: &str, now: Instant) -> HeartbeatResult {
        self.prune(now);

        let Some(entry) = self.entries.get_mut(instance_id) else {
            return HeartbeatResult::NotFound;
        };
        if entry.origin != origin {
            return HeartbeatResult::OriginMismatch;
        }

        entry.last_seen = now;
        HeartbeatResult::Refreshed
    }

    pub fn active(&mut self, now: Instant) -> Vec<BrowserInstance> {
        self.prune(now);
        let mut instances = self
            .entries
            .values()
            .map(|entry| BrowserInstance {
                instance_id: entry.registration.instance_id.clone(),
                browser: entry.registration.browser,
                version: entry.registration.version.clone(),
                capabilities: entry.registration.capabilities.clone(),
            })
            .collect::<Vec<_>>();
        instances.sort_by(|left, right| {
            (left.browser, left.instance_id.as_str())
                .cmp(&(right.browser, right.instance_id.as_str()))
        });
        instances
    }

    fn prune(&mut self, now: Instant) {
        self.entries
            .retain(|_, entry| now.saturating_duration_since(entry.last_seen) < self.lease);
    }
}

pub fn valid_instance_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn valid_browser_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_BROWSER_VERSION_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTANCE_ID: &str = "0123456789abcdef0123456789abcdef";
    const OTHER_INSTANCE_ID: &str = "fedcba9876543210fedcba9876543210";
    const ORIGIN: &str = "chrome-extension://abcdefghijklmnopabcdefghijklmnop";

    fn registration(instance_id: &str) -> BrowserRegistration {
        BrowserRegistration {
            instance_id: instance_id.to_owned(),
            browser: BrowserKind::Chrome,
            version: Some("140.0.0.0".to_owned()),
            capabilities: vec![BrowserCapability::Capture, BrowserCapability::Restore],
        }
    }

    #[test]
    fn registers_and_lists_only_active_instances() {
        let start = Instant::now();
        let mut registry = BrowserRegistry::new(Duration::from_secs(30));

        registry
            .register(registration(INSTANCE_ID), ORIGIN, start)
            .unwrap();

        assert_eq!(registry.active(start).len(), 1);
        assert!(registry.active(start + Duration::from_secs(30)).is_empty());
    }

    #[test]
    fn heartbeat_extends_the_lease() {
        let start = Instant::now();
        let mut registry = BrowserRegistry::new(Duration::from_secs(30));
        registry
            .register(registration(INSTANCE_ID), ORIGIN, start)
            .unwrap();

        assert_eq!(
            registry.heartbeat(INSTANCE_ID, ORIGIN, start + Duration::from_secs(20)),
            HeartbeatResult::Refreshed
        );
        assert_eq!(registry.active(start + Duration::from_secs(40)).len(), 1);
    }

    #[test]
    fn instance_id_cannot_be_taken_over_by_another_origin() {
        let start = Instant::now();
        let mut registry = BrowserRegistry::default();
        registry
            .register(registration(INSTANCE_ID), ORIGIN, start)
            .unwrap();

        assert_eq!(
            registry.register(
                registration(INSTANCE_ID),
                "moz-extension://944cfddf-7a95-3c47-bd9a-663b3ce8d699",
                start,
            ),
            Err(RegistryError::OriginMismatch)
        );
        assert_eq!(
            registry.heartbeat(
                INSTANCE_ID,
                "moz-extension://944cfddf-7a95-3c47-bd9a-663b3ce8d699",
                start,
            ),
            HeartbeatResult::OriginMismatch
        );
    }

    #[test]
    fn validates_instance_version_and_capabilities() {
        let mut invalid_id = registration("not-an-id");
        assert!(!invalid_id.validate());

        invalid_id = registration(INSTANCE_ID);
        invalid_id.version = Some("version with spaces".to_owned());
        assert!(!invalid_id.validate());

        invalid_id = registration(INSTANCE_ID);
        invalid_id.capabilities = vec![BrowserCapability::Capture, BrowserCapability::Capture];
        assert!(!invalid_id.validate());

        assert!(registration(OTHER_INSTANCE_ID).validate());
    }

    #[test]
    fn enforces_registry_capacity_after_pruning() {
        let start = Instant::now();
        let mut registry = BrowserRegistry::default();

        for index in 0..MAX_BROWSER_INSTANCES {
            let instance_id = format!("{index:032x}");
            registry
                .register(registration(&instance_id), ORIGIN, start)
                .unwrap();
        }

        assert_eq!(
            registry.register(
                registration("ffffffffffffffffffffffffffffffff"),
                ORIGIN,
                start,
            ),
            Err(RegistryError::CapacityExceeded)
        );
    }
}
