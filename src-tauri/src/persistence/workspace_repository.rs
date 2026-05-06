use rusqlite::{params, OptionalExtension};

use crate::{
    models::{EntityId, Workspace},
    persistence::{AppDatabase, PersistenceResult},
};

#[derive(Debug, Clone)]
pub struct WorkspaceRepository {
    database: AppDatabase,
}

impl WorkspaceRepository {
    pub fn new(database: AppDatabase) -> Self {
        Self { database }
    }

    pub fn create(&self, workspace: &Workspace) -> PersistenceResult<()> {
        let connection = self.database.connect()?;

        connection.execute(
            r#"
        INSERT INTO workspaces (id, name, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4)
      "#,
            params![
                workspace.id,
                workspace.name,
                workspace.created_at,
                workspace.updated_at
            ],
        )?;

        Ok(())
    }

    pub fn list(&self) -> PersistenceResult<Vec<Workspace>> {
        let connection = self.database.connect()?;
        let mut statement = connection.prepare(
            r#"
        SELECT id, name, created_at, updated_at
        FROM workspaces
        ORDER BY created_at ASC, name ASC, id ASC
      "#,
        )?;

        let rows = statement.query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;

        let workspaces = rows.collect::<Result<Vec<_>, _>>()?;

        Ok(workspaces)
    }

    pub fn find_by_id(&self, workspace_id: &EntityId) -> PersistenceResult<Option<Workspace>> {
        let connection = self.database.connect()?;
        let workspace = connection
            .query_row(
                r#"
          SELECT id, name, created_at, updated_at
          FROM workspaces
          WHERE id = ?1
        "#,
                [workspace_id],
                |row| {
                    Ok(Workspace {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                    })
                },
            )
            .optional()?;

        Ok(workspace)
    }

    pub fn update(&self, workspace: &Workspace) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let updated_rows = connection.execute(
            r#"
        UPDATE workspaces
        SET name = ?2, created_at = ?3, updated_at = ?4
        WHERE id = ?1
      "#,
            params![
                workspace.id,
                workspace.name,
                workspace.created_at,
                workspace.updated_at
            ],
        )?;

        Ok(updated_rows > 0)
    }

    pub fn delete(&self, workspace_id: &EntityId) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let deleted_rows =
            connection.execute("DELETE FROM workspaces WHERE id = ?1", [workspace_id])?;

        Ok(deleted_rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::WorkspaceRepository;
    use crate::{models::Workspace, persistence::test_utils::TestDatabase};

    #[test]
    fn supports_workspace_crud() {
        let test_database = TestDatabase::new("workspace-repository-crud");
        let repository = WorkspaceRepository::new(test_database.database());
        let original_workspace = Workspace {
            id: "workspace-main".into(),
            name: "Centralita".into(),
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };

        repository
            .create(&original_workspace)
            .expect("workspace should be created");

        let listed_workspaces = repository.list().expect("workspaces should be listed");
        assert_eq!(listed_workspaces, vec![original_workspace.clone()]);

        let loaded_workspace = repository
            .find_by_id(&original_workspace.id)
            .expect("workspace should be readable");
        assert_eq!(loaded_workspace, Some(original_workspace.clone()));

        let updated_workspace = Workspace {
            name: "Centralita Updated".into(),
            updated_at: "2026-04-14T10:00:00Z".into(),
            ..original_workspace.clone()
        };

        let was_updated = repository
            .update(&updated_workspace)
            .expect("workspace should be updated");
        assert!(was_updated);

        let reloaded_workspace = repository
            .find_by_id(&updated_workspace.id)
            .expect("workspace should still exist");
        assert_eq!(reloaded_workspace, Some(updated_workspace.clone()));

        let was_deleted = repository
            .delete(&updated_workspace.id)
            .expect("workspace should be deleted");
        assert!(was_deleted);

        assert!(repository
            .find_by_id(&updated_workspace.id)
            .expect("workspace lookup should succeed")
            .is_none());
    }
}
