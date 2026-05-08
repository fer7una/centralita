use rusqlite::{params, OptionalExtension};

use crate::{
    models::{EntityId, RunHistoryEntry, RuntimeStatus},
    persistence::{AppDatabase, PersistenceResult},
};

#[derive(Debug, Clone)]
pub struct RunHistoryRepository {
    database: AppDatabase,
}

#[derive(Debug, Clone)]
pub struct FinalizeRunHistoryInput {
    pub ended_at: Option<String>,
    pub exit_code: Option<i32>,
    pub final_runtime_status: RuntimeStatus,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
}

impl RunHistoryRepository {
    pub fn new(database: AppDatabase) -> Self {
        Self { database }
    }

    pub fn create(&self, entry: &RunHistoryEntry) -> PersistenceResult<()> {
        let connection = self.database.connect()?;
        connection.execute(
            r#"
          INSERT INTO run_history (
            id,
            project_id,
            started_at,
            ended_at,
            exit_code,
            final_runtime_status,
            stop_reason,
            error_message,
            command_preview
          )
          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
            params![
                entry.id,
                entry.project_id,
                entry.started_at,
                entry.ended_at,
                entry.exit_code,
                serialize_runtime_status(&entry.final_runtime_status)?,
                entry.stop_reason,
                entry.error_message,
                entry.command_preview
            ],
        )?;

        Ok(())
    }

    pub fn finalize(
        &self,
        run_history_id: &EntityId,
        input: &FinalizeRunHistoryInput,
    ) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let updated_rows = connection.execute(
            r#"
          UPDATE run_history
          SET
            ended_at = ?2,
            exit_code = ?3,
            final_runtime_status = ?4,
            stop_reason = ?5,
            error_message = ?6
          WHERE id = ?1
        "#,
            params![
                run_history_id,
                input.ended_at,
                input.exit_code,
                serialize_runtime_status(&input.final_runtime_status)?,
                input.stop_reason,
                input.error_message
            ],
        )?;

        Ok(updated_rows > 0)
    }

    pub fn find_by_id(
        &self,
        run_history_id: &EntityId,
    ) -> PersistenceResult<Option<RunHistoryEntry>> {
        let connection = self.database.connect()?;
        connection
            .query_row(
                r#"
              SELECT
                id,
                project_id,
                started_at,
                ended_at,
                exit_code,
                final_runtime_status,
                stop_reason,
                error_message,
                command_preview
              FROM run_history
              WHERE id = ?1
            "#,
                [run_history_id],
                map_run_history_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_by_project(
        &self,
        project_id: &EntityId,
        limit: usize,
    ) -> PersistenceResult<Vec<RunHistoryEntry>> {
        let connection = self.database.connect()?;
        let mut statement = connection.prepare(
            r#"
          SELECT
            id,
            project_id,
            started_at,
            ended_at,
            exit_code,
            final_runtime_status,
            stop_reason,
            error_message,
            command_preview
          FROM run_history
          WHERE project_id = ?1
          ORDER BY started_at DESC, id DESC
          LIMIT ?2
        "#,
        )?;

        let rows = statement.query_map(params![project_id, limit as i64], map_run_history_row)?;

        rows.map(|row| row.map_err(Into::into)).collect()
    }

    pub fn list_by_workspace(
        &self,
        workspace_id: &EntityId,
        limit: usize,
    ) -> PersistenceResult<Vec<RunHistoryEntry>> {
        let connection = self.database.connect()?;
        let mut statement = connection.prepare(
            r#"
          SELECT
            rh.id,
            rh.project_id,
            rh.started_at,
            rh.ended_at,
            rh.exit_code,
            rh.final_runtime_status,
            rh.stop_reason,
            rh.error_message,
            rh.command_preview
          FROM run_history rh
          INNER JOIN projects p ON p.id = rh.project_id
          WHERE p.workspace_id = ?1
          ORDER BY rh.started_at DESC, rh.id DESC
          LIMIT ?2
        "#,
        )?;

        let rows = statement.query_map(params![workspace_id, limit as i64], map_run_history_row)?;

        rows.map(|row| row.map_err(Into::into)).collect()
    }
}

fn map_run_history_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunHistoryEntry> {
    Ok(RunHistoryEntry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        exit_code: row.get(4)?,
        final_runtime_status: deserialize_runtime_status(row.get(5)?).map_err(to_sql_error)?,
        stop_reason: row.get(6)?,
        error_message: row.get(7)?,
        command_preview: row.get(8)?,
    })
}

fn serialize_runtime_status(value: &RuntimeStatus) -> PersistenceResult<String> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(serialized) => Ok(serialized),
        _ => Err(std::io::Error::other("runtime status must serialize as string").into()),
    }
}

fn deserialize_runtime_status(value: String) -> PersistenceResult<RuntimeStatus> {
    Ok(serde_json::from_value(serde_json::Value::String(value))?)
}

fn to_sql_error(error: Box<dyn std::error::Error + Send + Sync>) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, error)
}

#[cfg(test)]
mod tests {
    use super::{FinalizeRunHistoryInput, RunHistoryRepository};
    use crate::{
        models::{GroupNode, ProjectNode, RunHistoryEntry, RuntimeStatus, Workspace},
        persistence::{
            test_utils::TestDatabase, GroupRepository, ProjectRepository, WorkspaceRepository,
        },
    };

    #[test]
    fn creates_finalizes_and_lists_run_history() {
        let test_database = TestDatabase::new("run-history-repository");
        let repository = RunHistoryRepository::new(test_database.database());
        seed_project(&test_database, "workspace-1", "project-1");
        let created_entry = RunHistoryEntry {
            id: "run-1".into(),
            project_id: "project-1".into(),
            started_at: "2026-04-17T09:00:00Z".into(),
            ended_at: None,
            exit_code: None,
            final_runtime_status: RuntimeStatus::Running,
            stop_reason: None,
            error_message: None,
            command_preview: "npm run dev".into(),
        };

        repository
            .create(&created_entry)
            .expect("run history should be created");
        repository
            .finalize(
                &created_entry.id,
                &FinalizeRunHistoryInput {
                    ended_at: Some("2026-04-17T09:05:00Z".into()),
                    exit_code: Some(0),
                    final_runtime_status: RuntimeStatus::Stopped,
                    stop_reason: Some("manual-stop".into()),
                    error_message: None,
                },
            )
            .expect("run history should finalize");

        let stored_entry = repository
            .find_by_id(&created_entry.id)
            .expect("run history lookup should work")
            .expect("run history should exist");
        let project_history = repository
            .list_by_project(&"project-1".into(), 10)
            .expect("project run history should load");
        let workspace_history = repository
            .list_by_workspace(&"workspace-1".into(), 10)
            .expect("workspace run history should load");

        assert_eq!(stored_entry.final_runtime_status, RuntimeStatus::Stopped);
        assert_eq!(project_history.len(), 1);
        assert_eq!(workspace_history.len(), 1);
    }

    fn seed_project(test_database: &TestDatabase, workspace_id: &str, project_id: &str) {
        let workspace_repository = WorkspaceRepository::new(test_database.database());
        let group_repository = GroupRepository::new(test_database.database());
        let project_repository = ProjectRepository::new(test_database.database());
        let workspace = Workspace {
            id: workspace_id.into(),
            name: "Main".into(),
            created_at: "2026-04-17T09:00:00Z".into(),
            updated_at: "2026-04-17T09:00:00Z".into(),
        };
        let group = GroupNode {
            id: "group-1".into(),
            workspace_id: workspace.id.clone(),
            parent_group_id: None,
            name: "Backend".into(),
            color: "#2563eb".into(),
            sort_order: 10,
            created_at: "2026-04-17T09:00:00Z".into(),
            updated_at: "2026-04-17T09:00:00Z".into(),
        };
        let project = ProjectNode {
            id: project_id.into(),
            workspace_id: workspace.id.clone(),
            group_id: group.id.clone(),
            name: "API".into(),
            path: r"C:\Projects\api".into(),
            detected_type: None,
            color: None,
            package_manager: None,
            executable: Some("cmd.exe".into()),
            command: Some("cmd.exe /C ping 127.0.0.1 -n 2".into()),
            args: None,
            env: None,
            working_dir: Some(r"C:\Projects\api".into()),
            detection_confidence: None,
            detection_evidence: None,
            warnings: None,
            created_at: "2026-04-17T09:00:00Z".into(),
            updated_at: "2026-04-17T09:00:00Z".into(),
        };

        workspace_repository
            .create(&workspace)
            .expect("workspace should be created");
        group_repository
            .create(&group)
            .expect("group should be created");
        project_repository
            .create(&project)
            .expect("project should be created");
    }
}
