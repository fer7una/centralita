use serde::{Deserialize, Serialize};

use crate::models::{EntityId, IsoDateTime, RuntimeStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunHistoryEntry {
    pub id: EntityId,
    pub project_id: EntityId,
    pub started_at: IsoDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<IsoDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub final_runtime_status: RuntimeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub command_preview: String,
}

#[cfg(test)]
mod tests {
    use super::RunHistoryEntry;
    use crate::models::RuntimeStatus;

    #[test]
    fn serializes_run_history_with_optional_metadata() {
        let entry = RunHistoryEntry {
            id: "run-1".into(),
            project_id: "project-api".into(),
            started_at: "2026-04-17T09:00:00Z".into(),
            ended_at: Some("2026-04-17T09:05:00Z".into()),
            exit_code: Some(1),
            final_runtime_status: RuntimeStatus::Failed,
            stop_reason: Some("unexpected-exit".into()),
            error_message: Some("Port already in use".into()),
            command_preview: "npm run dev".into(),
        };

        let json = serde_json::to_value(&entry).expect("run history should serialize");

        assert_eq!(json["finalRuntimeStatus"], "FAILED");
        assert_eq!(json["stopReason"], "unexpected-exit");
    }
}
