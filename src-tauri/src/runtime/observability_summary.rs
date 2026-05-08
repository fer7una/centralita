use crate::models::{
    RuntimeStatus, WorkspaceObservabilitySummary, WorkspaceRuntimeStatus,
    WorkspaceRuntimeStatusCounts,
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

pub fn build_workspace_observability_summary(
    workspace_runtime: &WorkspaceRuntimeStatus,
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
        runtime_counts: WorkspaceRuntimeStatusCounts::from_statuses(
            workspace_runtime
                .projects
                .iter()
                .map(|project| project.status),
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{ProcessRuntimeState, WorkspaceRuntimeStatus};

    use super::build_workspace_observability_summary;

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
        let summary = build_workspace_observability_summary(&workspace_runtime);

        assert_eq!(summary.total_projects, 2);
        assert_eq!(summary.runtime_counts.running, 1);
        assert_eq!(summary.runtime_counts.failed, 1);
    }
}
