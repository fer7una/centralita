use crate::{
    events::{emit_history_appended, RuntimeEventEmitter},
    models::{ProjectNode, RunHistoryEntry, RuntimeStatus},
    persistence::{AppDatabase, FinalizeRunHistoryInput, RunHistoryRepository},
    runtime::{RuntimeError, RuntimeResult},
    utils::ids,
};

#[derive(Clone)]
pub struct HistoryRecorder {
    database: Option<AppDatabase>,
    event_emitter: RuntimeEventEmitter,
}

impl HistoryRecorder {
    pub fn new(database: Option<AppDatabase>, event_emitter: RuntimeEventEmitter) -> Self {
        Self {
            database,
            event_emitter,
        }
    }

    pub fn record_run_started(
        &self,
        project: &ProjectNode,
        command_preview: &str,
        started_at: &str,
    ) -> RuntimeResult<Option<String>> {
        let Some(database) = self.database.clone() else {
            return Ok(None);
        };
        let entry = RunHistoryEntry {
            id: ids::run_history_id(),
            project_id: project.id.clone(),
            started_at: started_at.to_owned(),
            ended_at: None,
            exit_code: None,
            final_runtime_status: RuntimeStatus::Running,
            final_health_status: None,
            stop_reason: None,
            error_message: None,
            command_preview: command_preview.to_owned(),
        };

        RunHistoryRepository::new(database)
            .create(&entry)
            .map_err(|error| {
                RuntimeError::new(format!("Failed to persist run history: {error}"))
            })?;
        emit_history_appended(&self.event_emitter, &entry);

        Ok(Some(entry.id))
    }

    pub fn record_start_failure(
        &self,
        project: &ProjectNode,
        command_preview: &str,
        started_at: &str,
        error_message: &str,
    ) -> RuntimeResult<Option<String>> {
        let Some(database) = self.database.clone() else {
            return Ok(None);
        };
        let entry = RunHistoryEntry {
            id: ids::run_history_id(),
            project_id: project.id.clone(),
            started_at: started_at.to_owned(),
            ended_at: Some(started_at.to_owned()),
            exit_code: None,
            final_runtime_status: RuntimeStatus::Failed,
            final_health_status: None,
            stop_reason: Some("start-failed".into()),
            error_message: Some(error_message.to_owned()),
            command_preview: command_preview.to_owned(),
        };

        RunHistoryRepository::new(database)
            .create(&entry)
            .map_err(|error| {
                RuntimeError::new(format!("Failed to persist run history: {error}"))
            })?;
        emit_history_appended(&self.event_emitter, &entry);

        Ok(Some(entry.id))
    }

    pub fn finalize_run(
        &self,
        run_history_id: &str,
        input: FinalizeRunHistoryInput,
    ) -> RuntimeResult<()> {
        let Some(database) = self.database.clone() else {
            return Ok(());
        };

        RunHistoryRepository::new(database)
            .finalize(&run_history_id.to_owned(), &input)
            .map_err(|error| {
                RuntimeError::new(format!("Failed to finalize run history: {error}"))
            })?;

        Ok(())
    }
}
