use serde::{Deserialize, Serialize};

use crate::models::{CommandArgs, EntityId, EnvironmentVariables, IsoDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeLogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub project_id: EntityId,
    pub executable: String,
    #[serde(default)]
    pub args: CommandArgs,
    pub working_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvironmentVariables>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRuntimeState {
    pub project_id: EntityId,
    pub status: RuntimeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<IsoDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<IsoDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub command_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRuntimeStatus {
    pub workspace_id: EntityId,
    pub status: RuntimeStatus,
    #[serde(default)]
    pub projects: Vec<ProcessRuntimeState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogLine {
    pub project_id: EntityId,
    pub stream: RuntimeLogStream,
    pub line: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub partial: bool,
    pub timestamp: IsoDateTime,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusEvent {
    pub project_id: EntityId,
    pub status: RuntimeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub timestamp: IsoDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub command_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProcessExitedEvent {
    pub project_id: EntityId,
    pub status: RuntimeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub timestamp: IsoDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub command_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProcessErrorEvent {
    pub project_id: EntityId,
    pub status: RuntimeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub timestamp: IsoDateTime,
    pub message: String,
    pub command_preview: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeOperationScope {
    Project,
    Group,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBulkFailure {
    pub project_id: EntityId,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBulkOperationResult {
    pub scope: RuntimeOperationScope,
    pub target_id: EntityId,
    pub status: RuntimeStatus,
    #[serde(default)]
    pub requested_project_ids: Vec<EntityId>,
    #[serde(default)]
    pub affected_project_ids: Vec<EntityId>,
    #[serde(default)]
    pub skipped_project_ids: Vec<EntityId>,
    #[serde(default)]
    pub failures: Vec<RuntimeBulkFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum RuntimeEvent {
    ProjectStarting(RuntimeStatusEvent),
    ProjectStarted(RuntimeStatusEvent),
    ProjectStopping(RuntimeStatusEvent),
    ProjectStopped(RuntimeProcessExitedEvent),
    ProjectFailed(RuntimeProcessErrorEvent),
    ProjectLogLine(RuntimeLogLine),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        ProcessRuntimeState, RunRequest, RuntimeBulkOperationResult, RuntimeEvent, RuntimeLogLine,
        RuntimeLogStream, RuntimeOperationScope, RuntimeProcessExitedEvent, RuntimeStatus,
        RuntimeStatusEvent, WorkspaceRuntimeStatus,
    };

    #[test]
    fn serializes_runtime_status_with_expected_wire_format() {
        let serialized = serde_json::to_string(&RuntimeStatus::Starting)
            .expect("runtime status should serialize");

        assert_eq!(serialized, "\"STARTING\"");
    }

    #[test]
    fn round_trips_run_request_with_env() {
        let mut env = BTreeMap::new();
        env.insert("PORT".into(), "1420".into());

        let request = RunRequest {
            project_id: "project-ui".into(),
            executable: "npm".into(),
            args: vec!["run".into(), "dev".into()],
            working_dir: r"C:\Projects\ui".into(),
            env: Some(env),
        };

        let json = serde_json::to_value(&request).expect("run request should serialize");
        let decoded: RunRequest =
            serde_json::from_value(json.clone()).expect("run request should deserialize");

        assert_eq!(decoded, request);
        assert_eq!(json["projectId"], "project-ui");
        assert_eq!(json["args"][1], "dev");
    }

    #[test]
    fn round_trips_runtime_event_variants() {
        let event = RuntimeEvent::ProjectStarted(RuntimeStatusEvent {
            project_id: "project-ui".into(),
            status: RuntimeStatus::Running,
            pid: Some(4812),
            timestamp: "2026-04-16T09:30:00Z".into(),
            message: Some("Process started".into()),
            command_preview: "npm run dev".into(),
        });

        let json = serde_json::to_value(&event).expect("runtime event should serialize");
        let decoded: RuntimeEvent =
            serde_json::from_value(json.clone()).expect("runtime event should deserialize");

        assert_eq!(decoded, event);
        assert_eq!(json["type"], "projectStarted");
        assert_eq!(json["payload"]["status"], "RUNNING");
        assert_eq!(json["payload"]["commandPreview"], "npm run dev");
    }

    #[test]
    fn serializes_log_lines_and_process_state_with_optional_fields() {
        let log_line = RuntimeLogLine {
            project_id: "project-ui".into(),
            stream: RuntimeLogStream::Stderr,
            line: "Port already in use".into(),
            partial: false,
            timestamp: "2026-04-16T09:31:00Z".into(),
        };
        let state = ProcessRuntimeState {
            project_id: "project-ui".into(),
            status: RuntimeStatus::Failed,
            pid: Some(4812),
            started_at: Some("2026-04-16T09:30:00Z".into()),
            stopped_at: Some("2026-04-16T09:31:00Z".into()),
            exit_code: Some(1),
            last_error: Some("Process exited unexpectedly".into()),
            command_preview: "npm run dev".into(),
        };
        let exited_event = RuntimeProcessExitedEvent {
            project_id: "project-ui".into(),
            status: RuntimeStatus::Stopped,
            pid: Some(4812),
            timestamp: "2026-04-16T09:31:00Z".into(),
            exit_code: Some(0),
            message: Some("Process stopped".into()),
            command_preview: "npm run dev".into(),
        };

        let log_json = serde_json::to_value(&log_line).expect("log line should serialize");
        let state_json = serde_json::to_value(&state).expect("runtime state should serialize");
        let exited_json = serde_json::to_value(&exited_event).expect("exit event should serialize");

        assert_eq!(log_json["stream"], "stderr");
        assert_eq!(state_json["lastError"], "Process exited unexpectedly");
        assert_eq!(exited_json["exitCode"], 0);
        assert_eq!(exited_json["commandPreview"], "npm run dev");
    }

    #[test]
    fn serializes_workspace_status_and_bulk_operation_results() {
        let workspace_status = WorkspaceRuntimeStatus {
            workspace_id: "workspace-main".into(),
            status: RuntimeStatus::Running,
            projects: vec![ProcessRuntimeState {
                project_id: "project-ui".into(),
                status: RuntimeStatus::Running,
                pid: Some(4812),
                started_at: Some("2026-04-16T09:30:00Z".into()),
                stopped_at: None,
                exit_code: None,
                last_error: None,
                command_preview: "pnpm dev".into(),
            }],
        };
        let bulk_result = RuntimeBulkOperationResult {
            scope: RuntimeOperationScope::Group,
            target_id: "group-frontend".into(),
            status: RuntimeStatus::Running,
            requested_project_ids: vec!["project-ui".into()],
            affected_project_ids: vec!["project-ui".into()],
            skipped_project_ids: vec![],
            failures: vec![],
        };

        let workspace_json =
            serde_json::to_value(&workspace_status).expect("workspace status should serialize");
        let bulk_json = serde_json::to_value(&bulk_result).expect("bulk result should serialize");

        assert_eq!(workspace_json["workspaceId"], "workspace-main");
        assert_eq!(bulk_json["scope"], "group");
        assert_eq!(bulk_json["affectedProjectIds"][0], "project-ui");
    }
}
