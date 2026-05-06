use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

use crate::{
    events::{emit_health_changed, noop_runtime_event_emitter, RuntimeEventEmitter},
    models::{HealthCheckConfig, HealthStatus, ProjectHealthState},
    runtime::{health_check::execute_health_check, RuntimeResult},
    utils::timestamps,
};

#[derive(Clone)]
pub struct HealthManager {
    states: Arc<Mutex<HashMap<String, ProjectHealthState>>>,
    monitors: Arc<Mutex<HashMap<String, HealthMonitorHandle>>>,
    event_emitter: RuntimeEventEmitter,
}

#[derive(Clone)]
struct HealthMonitorHandle {
    stop_flag: Arc<AtomicBool>,
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthManager {
    pub fn new() -> Self {
        Self::with_event_emitter(noop_runtime_event_emitter())
    }

    pub fn with_event_emitter(event_emitter: RuntimeEventEmitter) -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
            monitors: Arc::new(Mutex::new(HashMap::new())),
            event_emitter,
        }
    }

    pub fn state_for_project(
        &self,
        project_id: &str,
        health_check: Option<&HealthCheckConfig>,
    ) -> ProjectHealthState {
        lock_mutex(&self.states)
            .get(project_id)
            .cloned()
            .unwrap_or_else(|| default_state(project_id, health_check))
    }

    pub fn states_for_projects(
        &self,
        project_ids: &[String],
        configs: &HashMap<String, Option<HealthCheckConfig>>,
    ) -> Vec<ProjectHealthState> {
        project_ids
            .iter()
            .map(|project_id| {
                self.state_for_project(
                    project_id,
                    configs.get(project_id).and_then(|config| config.as_ref()),
                )
            })
            .collect()
    }

    pub fn start_monitoring(&self, project_id: String, config: HealthCheckConfig) {
        let config = config.normalized();
        self.stop_monitoring(&project_id);
        self.store_state(default_state(&project_id, Some(&config)));

        let stop_flag = Arc::new(AtomicBool::new(false));
        lock_mutex(&self.monitors).insert(
            project_id.clone(),
            HealthMonitorHandle {
                stop_flag: stop_flag.clone(),
            },
        );

        let states = self.states.clone();
        let event_emitter = self.event_emitter.clone();
        let monitors = self.monitors.clone();
        thread::spawn(move || {
            if config.grace_period_ms() > 0 {
                thread::sleep(Duration::from_millis(config.grace_period_ms()));
            }
            if stop_flag.load(Ordering::SeqCst) {
                return;
            }

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                let current = {
                    let mut states = lock_mutex(&states);
                    let next_state = states
                        .entry(project_id.clone())
                        .or_insert_with(|| default_state(&project_id, Some(&config)));
                    next_state.status = HealthStatus::Checking;
                    next_state.clone()
                };
                emit_health_changed(&event_emitter, &current);

                let probe_result = execute_health_check(&config);
                let updated = {
                    let mut states = lock_mutex(&states);
                    let next_state = states
                        .entry(project_id.clone())
                        .or_insert_with(|| default_state(&project_id, Some(&config)));
                    apply_probe_result(next_state, &config, probe_result);
                    next_state.clone()
                };
                emit_health_changed(&event_emitter, &updated);

                let mut slept = 0_u64;
                while slept < config.interval_ms() {
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    let remaining = config.interval_ms() - slept;
                    let next_sleep = remaining.min(200);
                    thread::sleep(Duration::from_millis(next_sleep));
                    slept += next_sleep;
                }
            }

            lock_mutex(&monitors).remove(&project_id);
        });
    }

    pub fn refresh_now(
        &self,
        project_id: &str,
        health_check: Option<&HealthCheckConfig>,
        is_running: bool,
    ) -> RuntimeResult<ProjectHealthState> {
        let Some(config) = health_check.cloned() else {
            let state = default_state(project_id, None);
            self.store_state(state.clone());
            return Ok(state);
        };
        let config = config.normalized();
        if !config.enabled() {
            let state = default_state(project_id, Some(&config));
            self.store_state(state.clone());
            return Ok(state);
        }
        if !is_running {
            let state = default_state(project_id, Some(&config));
            self.store_state(state.clone());
            return Ok(state);
        }

        let updated = {
            let mut states = lock_mutex(&self.states);
            let current = states
                .entry(project_id.to_owned())
                .or_insert_with(|| default_state(project_id, Some(&config)));
            current.status = HealthStatus::Checking;
            apply_probe_result(current, &config, execute_health_check(&config));
            current.clone()
        };
        emit_health_changed(&self.event_emitter, &updated);

        Ok(updated)
    }

    pub fn last_known_status(&self, project_id: &str) -> Option<HealthStatus> {
        lock_mutex(&self.states)
            .get(project_id)
            .map(|state| state.status)
    }

    pub fn mark_process_stopped(&self, project_id: &str, health_check: Option<&HealthCheckConfig>) {
        self.stop_monitoring(project_id);
        let next_status = if supports_health_check(health_check) {
            HealthStatus::Unknown
        } else {
            HealthStatus::Unsupported
        };

        let updated = {
            let mut states = lock_mutex(&self.states);
            let state = states
                .entry(project_id.to_owned())
                .or_insert_with(|| default_state(project_id, health_check));
            state.status = next_status;
            state.consecutive_successes = 0;
            state.consecutive_failures = 0;
            state.clone()
        };
        emit_health_changed(&self.event_emitter, &updated);
    }

    pub fn stop_monitoring(&self, project_id: &str) {
        if let Some(handle) = lock_mutex(&self.monitors).remove(project_id) {
            handle.stop_flag.store(true, Ordering::SeqCst);
        }
    }

    pub fn clear_project(&self, project_id: &str) {
        self.stop_monitoring(project_id);
        lock_mutex(&self.states).remove(project_id);
    }

    pub fn shutdown(&self) {
        let handles = {
            let mut monitors = lock_mutex(&self.monitors);
            monitors
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };

        for handle in handles {
            handle.stop_flag.store(true, Ordering::SeqCst);
        }
    }

    fn store_state(&self, state: ProjectHealthState) {
        lock_mutex(&self.states).insert(state.project_id.clone(), state.clone());
        emit_health_changed(&self.event_emitter, &state);
    }
}

fn apply_probe_result(
    state: &mut ProjectHealthState,
    config: &HealthCheckConfig,
    probe_result: RuntimeResult<()>,
) {
    let checked_at = now_iso_or_fallback();
    state.last_checked_at = Some(checked_at.clone());

    match probe_result {
        Ok(()) => {
            state.last_error = None;
            state.consecutive_failures = 0;
            state.consecutive_successes += 1;
            state.last_healthy_at = Some(checked_at);
            state.status = if state.consecutive_successes >= config.success_threshold() {
                HealthStatus::Healthy
            } else {
                HealthStatus::Checking
            };
        }
        Err(error) => {
            state.last_error = Some(error.to_string());
            state.consecutive_successes = 0;
            state.consecutive_failures += 1;
            state.status = if state.consecutive_failures >= config.failure_threshold() {
                HealthStatus::Unhealthy
            } else {
                HealthStatus::Checking
            };
        }
    }
}

fn default_state(project_id: &str, health_check: Option<&HealthCheckConfig>) -> ProjectHealthState {
    ProjectHealthState {
        project_id: project_id.to_owned(),
        status: if supports_health_check(health_check) {
            HealthStatus::Unknown
        } else {
            HealthStatus::Unsupported
        },
        last_checked_at: None,
        last_healthy_at: None,
        last_error: None,
        consecutive_successes: 0,
        consecutive_failures: 0,
    }
}

fn supports_health_check(health_check: Option<&HealthCheckConfig>) -> bool {
    health_check.is_some_and(|config| config.enabled())
}

fn now_iso_or_fallback() -> String {
    timestamps::now_iso().unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{HealthCheckConfig, HealthStatus, HttpHealthCheckConfig};
    use crate::runtime::RuntimeError;

    use super::{apply_probe_result, default_state};

    #[test]
    fn upgrades_to_healthy_after_success_threshold() {
        let config = HealthCheckConfig::Http(HttpHealthCheckConfig {
            enabled: true,
            interval_ms: 1_000,
            timeout_ms: 1_000,
            grace_period_ms: 0,
            success_threshold: 2,
            failure_threshold: 2,
            url: "http://127.0.0.1:1420/health".into(),
            method: "GET".into(),
            expected_status_codes: vec![200],
            headers: None,
            contains_text: None,
        });
        let mut state = default_state("project-a", Some(&config));

        apply_probe_result(&mut state, &config, Ok(()));
        assert_eq!(state.status, HealthStatus::Checking);
        apply_probe_result(&mut state, &config, Ok(()));
        assert_eq!(state.status, HealthStatus::Healthy);
    }

    #[test]
    fn downgrades_to_unhealthy_after_failure_threshold() {
        let config = HealthCheckConfig::Http(HttpHealthCheckConfig {
            enabled: true,
            interval_ms: 1_000,
            timeout_ms: 1_000,
            grace_period_ms: 0,
            success_threshold: 1,
            failure_threshold: 2,
            url: "http://127.0.0.1:1420/health".into(),
            method: "GET".into(),
            expected_status_codes: vec![200],
            headers: None,
            contains_text: None,
        });
        let mut state = default_state("project-a", Some(&config));

        apply_probe_result(&mut state, &config, Err(RuntimeError::new("boom")));
        assert_eq!(state.status, HealthStatus::Checking);
        apply_probe_result(&mut state, &config, Err(RuntimeError::new("boom")));
        assert_eq!(state.status, HealthStatus::Unhealthy);
    }
}
