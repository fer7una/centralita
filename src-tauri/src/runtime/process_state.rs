use crate::models::{EntityId, ProcessRuntimeState, RuntimeStatus};

pub fn initial_process_state(
    project_id: impl Into<EntityId>,
    command_preview: impl Into<String>,
) -> ProcessRuntimeState {
    ProcessRuntimeState {
        project_id: project_id.into(),
        status: RuntimeStatus::Stopped,
        pid: None,
        started_at: None,
        stopped_at: None,
        exit_code: None,
        last_error: None,
        command_preview: command_preview.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::initial_process_state;
    use crate::models::RuntimeStatus;

    #[test]
    fn creates_stopped_state_without_runtime_metadata() {
        let state = initial_process_state("project-api", "npm run dev");

        assert_eq!(state.project_id, "project-api");
        assert_eq!(state.status, RuntimeStatus::Stopped);
        assert_eq!(state.command_preview, "npm run dev");
        assert!(state.pid.is_none());
        assert!(state.last_error.is_none());
    }
}
