use crate::models::{
    HealthStatus, ProjectHealthState, RuntimeStatus, WorkspaceHealthStatusCounts,
    WorkspaceObservabilitySummary, WorkspaceRuntimeStatus, WorkspaceRuntimeStatusCounts,
};

pub fn aggregate_runtime_status<I>(statuses: I) -> RuntimeStatus
where
    I: IntoIterator<Item = RuntimeStatus>,
{
    let statuses = statuses.into_iter().collect::<Vec<_>>();
    if statuses.is_empty() {
        return RuntimeStatus::Stopped;
    }

    if statuses
        .iter()
        .all(|status| *status == RuntimeStatus::Stopped)
    {
        return RuntimeStatus::Stopped;
    }
    if statuses.contains(&RuntimeStatus::Stopping) {
        return RuntimeStatus::Stopping;
    }
    if statuses.contains(&RuntimeStatus::Starting) {
        return RuntimeStatus::Starting;
    }
    if statuses.contains(&RuntimeStatus::Failed) {
        return RuntimeStatus::Failed;
    }
    if statuses.contains(&RuntimeStatus::Running) {
        return RuntimeStatus::Running;
    }

    RuntimeStatus::Stopped
}

pub fn aggregate_health_status<I>(statuses: I) -> HealthStatus
where
    I: IntoIterator<Item = HealthStatus>,
{
    let statuses = statuses.into_iter().collect::<Vec<_>>();
    if statuses.is_empty() {
        return HealthStatus::Unsupported;
    }
    if statuses.contains(&HealthStatus::Unhealthy) {
        return HealthStatus::Unhealthy;
    }
    if statuses.contains(&HealthStatus::Checking) {
        return HealthStatus::Checking;
    }
    if statuses.contains(&HealthStatus::Healthy) {
        return HealthStatus::Healthy;
    }
    if statuses.contains(&HealthStatus::Unknown) {
        return HealthStatus::Unknown;
    }

    HealthStatus::Unsupported
}

pub fn build_workspace_observability_summary(
    workspace_runtime: &WorkspaceRuntimeStatus,
    health_states: &[ProjectHealthState],
) -> WorkspaceObservabilitySummary {
    WorkspaceObservabilitySummary {
        workspace_id: workspace_runtime.workspace_id.clone(),
        total_projects: workspace_runtime.projects.len() as u32,
        runtime_status: aggregate_runtime_status(
            workspace_runtime
                .projects
                .iter()
                .map(|project| project.status),
        ),
        health_status: aggregate_health_status(health_states.iter().map(|state| state.status)),
        runtime_counts: WorkspaceRuntimeStatusCounts::from_statuses(
            workspace_runtime
                .projects
                .iter()
                .map(|project| project.status),
        ),
        health_counts: WorkspaceHealthStatusCounts::from_statuses(
            health_states.iter().map(|state| state.status),
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{
        HealthStatus, ProcessRuntimeState, ProjectHealthState, WorkspaceRuntimeStatus,
    };

    use super::{aggregate_health_status, build_workspace_observability_summary};

    #[test]
    fn aggregates_health_status_prioritizing_failures() {
        let aggregated = aggregate_health_status([
            HealthStatus::Healthy,
            HealthStatus::Unsupported,
            HealthStatus::Unhealthy,
        ]);

        assert_eq!(aggregated, HealthStatus::Unhealthy);
    }

    #[test]
    fn builds_workspace_summary_with_counts() {
        let workspace_runtime = WorkspaceRuntimeStatus {
            workspace_id: "workspace-main".into(),
            status: crate::models::RuntimeStatus::Running,
            projects: vec![
                ProcessRuntimeState {
                    project_id: "project-a".into(),
                    status: crate::models::RuntimeStatus::Running,
                    pid: Some(1),
                    started_at: None,
                    stopped_at: None,
                    exit_code: None,
                    last_error: None,
                    command_preview: "npm run dev".into(),
                },
                ProcessRuntimeState {
                    project_id: "project-b".into(),
                    status: crate::models::RuntimeStatus::Failed,
                    pid: None,
                    started_at: None,
                    stopped_at: Some("2026-04-17T09:10:00Z".into()),
                    exit_code: Some(1),
                    last_error: Some("boom".into()),
                    command_preview: "npm run dev".into(),
                },
            ],
        };
        let summary = build_workspace_observability_summary(
            &workspace_runtime,
            &[
                ProjectHealthState {
                    project_id: "project-a".into(),
                    status: HealthStatus::Healthy,
                    last_checked_at: None,
                    last_healthy_at: None,
                    last_error: None,
                    consecutive_successes: 1,
                    consecutive_failures: 0,
                },
                ProjectHealthState {
                    project_id: "project-b".into(),
                    status: HealthStatus::Unhealthy,
                    last_checked_at: None,
                    last_healthy_at: None,
                    last_error: Some("boom".into()),
                    consecutive_successes: 0,
                    consecutive_failures: 2,
                },
            ],
        );

        assert_eq!(summary.total_projects, 2);
        assert_eq!(summary.runtime_counts.running, 1);
        assert_eq!(summary.runtime_counts.failed, 1);
        assert_eq!(summary.health_counts.healthy, 1);
        assert_eq!(summary.health_counts.unhealthy, 1);
        assert_eq!(summary.health_status, HealthStatus::Unhealthy);
    }
}
