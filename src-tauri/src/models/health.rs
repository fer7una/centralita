use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::{EntityId, IsoDateTime, RuntimeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthStatus {
    Unknown,
    Checking,
    Healthy,
    Unhealthy,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckKind {
    Http,
    Tcp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpHealthCheckConfig {
    pub enabled: bool,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub grace_period_ms: u64,
    pub success_threshold: u32,
    pub failure_threshold: u32,
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub expected_status_codes: Vec<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpHealthCheckConfig {
    pub enabled: bool,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub grace_period_ms: u64,
    pub success_threshold: u32,
    pub failure_threshold: u32,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HealthCheckConfig {
    Http(HttpHealthCheckConfig),
    Tcp(TcpHealthCheckConfig),
}

impl HealthCheckConfig {
    pub fn kind(&self) -> HealthCheckKind {
        match self {
            Self::Http(_) => HealthCheckKind::Http,
            Self::Tcp(_) => HealthCheckKind::Tcp,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Http(config) => config.enabled,
            Self::Tcp(config) => config.enabled,
        }
    }

    pub fn interval_ms(&self) -> u64 {
        match self {
            Self::Http(config) => config.interval_ms,
            Self::Tcp(config) => config.interval_ms,
        }
    }

    pub fn timeout_ms(&self) -> u64 {
        match self {
            Self::Http(config) => config.timeout_ms,
            Self::Tcp(config) => config.timeout_ms,
        }
    }

    pub fn grace_period_ms(&self) -> u64 {
        match self {
            Self::Http(config) => config.grace_period_ms,
            Self::Tcp(config) => config.grace_period_ms,
        }
    }

    pub fn success_threshold(&self) -> u32 {
        match self {
            Self::Http(config) => config.success_threshold,
            Self::Tcp(config) => config.success_threshold,
        }
    }

    pub fn failure_threshold(&self) -> u32 {
        match self {
            Self::Http(config) => config.failure_threshold,
            Self::Tcp(config) => config.failure_threshold,
        }
    }

    pub fn normalized(&self) -> Self {
        const MIN_INTERVAL_MS: u64 = 1_000;
        const MIN_TIMEOUT_MS: u64 = 250;

        match self {
            Self::Http(config) => Self::Http(HttpHealthCheckConfig {
                enabled: config.enabled,
                interval_ms: config.interval_ms.max(MIN_INTERVAL_MS),
                timeout_ms: config.timeout_ms.max(MIN_TIMEOUT_MS),
                grace_period_ms: config.grace_period_ms,
                success_threshold: config.success_threshold.max(1),
                failure_threshold: config.failure_threshold.max(1),
                url: config.url.trim().to_owned(),
                method: if config.method.trim().is_empty() {
                    "GET".into()
                } else {
                    config.method.trim().to_ascii_uppercase()
                },
                expected_status_codes: if config.expected_status_codes.is_empty() {
                    vec![200]
                } else {
                    config.expected_status_codes.clone()
                },
                headers: config.headers.clone(),
                contains_text: config
                    .contains_text
                    .as_ref()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty()),
            }),
            Self::Tcp(config) => Self::Tcp(TcpHealthCheckConfig {
                enabled: config.enabled,
                interval_ms: config.interval_ms.max(MIN_INTERVAL_MS),
                timeout_ms: config.timeout_ms.max(MIN_TIMEOUT_MS),
                grace_period_ms: config.grace_period_ms,
                success_threshold: config.success_threshold.max(1),
                failure_threshold: config.failure_threshold.max(1),
                host: config.host.trim().to_owned(),
                port: config.port,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHealthState {
    pub project_id: EntityId,
    pub status: HealthStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<IsoDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_healthy_at: Option<IsoDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub consecutive_successes: u32,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRuntimeStatusCounts {
    pub stopped: u32,
    pub starting: u32,
    pub running: u32,
    pub stopping: u32,
    pub failed: u32,
}

impl WorkspaceRuntimeStatusCounts {
    pub fn from_statuses<I>(statuses: I) -> Self
    where
        I: IntoIterator<Item = RuntimeStatus>,
    {
        let mut counts = Self {
            stopped: 0,
            starting: 0,
            running: 0,
            stopping: 0,
            failed: 0,
        };

        for status in statuses {
            match status {
                RuntimeStatus::Stopped => counts.stopped += 1,
                RuntimeStatus::Starting => counts.starting += 1,
                RuntimeStatus::Running => counts.running += 1,
                RuntimeStatus::Stopping => counts.stopping += 1,
                RuntimeStatus::Failed => counts.failed += 1,
            }
        }

        counts
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceHealthStatusCounts {
    pub unknown: u32,
    pub checking: u32,
    pub healthy: u32,
    pub unhealthy: u32,
    pub unsupported: u32,
}

impl WorkspaceHealthStatusCounts {
    pub fn from_statuses<I>(statuses: I) -> Self
    where
        I: IntoIterator<Item = HealthStatus>,
    {
        let mut counts = Self {
            unknown: 0,
            checking: 0,
            healthy: 0,
            unhealthy: 0,
            unsupported: 0,
        };

        for status in statuses {
            match status {
                HealthStatus::Unknown => counts.unknown += 1,
                HealthStatus::Checking => counts.checking += 1,
                HealthStatus::Healthy => counts.healthy += 1,
                HealthStatus::Unhealthy => counts.unhealthy += 1,
                HealthStatus::Unsupported => counts.unsupported += 1,
            }
        }

        counts
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceObservabilitySummary {
    pub workspace_id: EntityId,
    pub total_projects: u32,
    pub runtime_status: RuntimeStatus,
    pub health_status: HealthStatus,
    pub runtime_counts: WorkspaceRuntimeStatusCounts,
    pub health_counts: WorkspaceHealthStatusCounts,
}

#[cfg(test)]
mod tests {
    use super::{
        HealthCheckConfig, HealthStatus, HttpHealthCheckConfig, TcpHealthCheckConfig,
        WorkspaceHealthStatusCounts, WorkspaceRuntimeStatusCounts,
    };
    use crate::models::RuntimeStatus;

    #[test]
    fn normalizes_http_health_check_defaults() {
        let config = HealthCheckConfig::Http(HttpHealthCheckConfig {
            enabled: true,
            interval_ms: 50,
            timeout_ms: 20,
            grace_period_ms: 0,
            success_threshold: 0,
            failure_threshold: 0,
            url: " http://127.0.0.1:1420/health ".into(),
            method: "".into(),
            expected_status_codes: vec![],
            headers: None,
            contains_text: Some(" ok ".into()),
        })
        .normalized();

        let HealthCheckConfig::Http(http) = config else {
            panic!("expected http config");
        };

        assert_eq!(http.interval_ms, 1_000);
        assert_eq!(http.timeout_ms, 250);
        assert_eq!(http.success_threshold, 1);
        assert_eq!(http.failure_threshold, 1);
        assert_eq!(http.method, "GET");
        assert_eq!(http.expected_status_codes, vec![200]);
        assert_eq!(http.contains_text.as_deref(), Some("ok"));
    }

    #[test]
    fn normalizes_tcp_health_check_defaults() {
        let config = HealthCheckConfig::Tcp(TcpHealthCheckConfig {
            enabled: true,
            interval_ms: 100,
            timeout_ms: 100,
            grace_period_ms: 0,
            success_threshold: 0,
            failure_threshold: 0,
            host: " localhost ".into(),
            port: 3000,
        })
        .normalized();

        let HealthCheckConfig::Tcp(tcp) = config else {
            panic!("expected tcp config");
        };

        assert_eq!(tcp.interval_ms, 1_000);
        assert_eq!(tcp.timeout_ms, 250);
        assert_eq!(tcp.host, "localhost");
        assert_eq!(tcp.success_threshold, 1);
        assert_eq!(tcp.failure_threshold, 1);
    }

    #[test]
    fn counts_runtime_and_health_statuses() {
        let runtime_counts = WorkspaceRuntimeStatusCounts::from_statuses([
            RuntimeStatus::Running,
            RuntimeStatus::Running,
            RuntimeStatus::Failed,
        ]);
        let health_counts = WorkspaceHealthStatusCounts::from_statuses([
            HealthStatus::Healthy,
            HealthStatus::Unsupported,
            HealthStatus::Checking,
        ]);

        assert_eq!(runtime_counts.running, 2);
        assert_eq!(runtime_counts.failed, 1);
        assert_eq!(health_counts.healthy, 1);
        assert_eq!(health_counts.checking, 1);
        assert_eq!(health_counts.unsupported, 1);
    }
}
