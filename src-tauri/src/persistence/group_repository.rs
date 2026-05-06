use rusqlite::{params, OptionalExtension};

use crate::{
    models::{EntityId, GroupNode},
    persistence::{AppDatabase, PersistenceResult},
};

#[derive(Debug, Clone)]
pub struct GroupRepository {
    database: AppDatabase,
}

impl GroupRepository {
    pub fn new(database: AppDatabase) -> Self {
        Self { database }
    }

    pub fn create(&self, group: &GroupNode) -> PersistenceResult<()> {
        let connection = self.database.connect()?;

        connection.execute(
            r#"
        INSERT INTO groups (
          id,
          workspace_id,
          parent_group_id,
          name,
          color,
          sort_order,
          created_at,
          updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
      "#,
            params![
                group.id,
                group.workspace_id,
                group.parent_group_id,
                group.name,
                group.color,
                group.sort_order,
                group.created_at,
                group.updated_at
            ],
        )?;

        Ok(())
    }

    pub fn list_by_workspace(&self, workspace_id: &EntityId) -> PersistenceResult<Vec<GroupNode>> {
        let connection = self.database.connect()?;
        let mut statement = connection.prepare(
            r#"
        SELECT
          id,
          workspace_id,
          parent_group_id,
          name,
          color,
          sort_order,
          created_at,
          updated_at
        FROM groups
        WHERE workspace_id = ?1
        ORDER BY
          COALESCE(parent_group_id, ''),
          sort_order ASC,
          name ASC,
          id ASC
      "#,
        )?;

        let rows = statement.query_map([workspace_id], |row| {
            Ok(GroupNode {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                parent_group_id: row.get(2)?,
                name: row.get(3)?,
                color: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        let groups = rows.collect::<Result<Vec<_>, _>>()?;

        Ok(groups)
    }

    pub fn find_by_id(&self, group_id: &EntityId) -> PersistenceResult<Option<GroupNode>> {
        let connection = self.database.connect()?;
        let group = connection
            .query_row(
                r#"
          SELECT
            id,
            workspace_id,
            parent_group_id,
            name,
            color,
            sort_order,
            created_at,
            updated_at
          FROM groups
          WHERE id = ?1
        "#,
                [group_id],
                |row| {
                    Ok(GroupNode {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        parent_group_id: row.get(2)?,
                        name: row.get(3)?,
                        color: row.get(4)?,
                        sort_order: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(group)
    }

    pub fn update(&self, group: &GroupNode) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let updated_rows = connection.execute(
            r#"
        UPDATE groups
        SET
          workspace_id = ?2,
          parent_group_id = ?3,
          name = ?4,
          color = ?5,
          sort_order = ?6,
          created_at = ?7,
          updated_at = ?8
        WHERE id = ?1
      "#,
            params![
                group.id,
                group.workspace_id,
                group.parent_group_id,
                group.name,
                group.color,
                group.sort_order,
                group.created_at,
                group.updated_at
            ],
        )?;

        Ok(updated_rows > 0)
    }

    pub fn delete(&self, group_id: &EntityId) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let deleted_rows = connection.execute("DELETE FROM groups WHERE id = ?1", [group_id])?;

        Ok(deleted_rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::GroupRepository;
    use crate::{
        models::{GroupNode, Workspace},
        persistence::{test_utils::TestDatabase, WorkspaceRepository},
    };

    #[test]
    fn supports_group_crud() {
        let test_database = TestDatabase::new("group-repository-crud");
        let workspace_repository = WorkspaceRepository::new(test_database.database());
        let group_repository = GroupRepository::new(test_database.database());
        let workspace = Workspace {
            id: "workspace-main".into(),
            name: "Centralita".into(),
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let original_group = GroupNode {
            id: "group-dev".into(),
            workspace_id: workspace.id.clone(),
            parent_group_id: None,
            name: "Development".into(),
            color: "#3b82f6".into(),
            sort_order: 10,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };

        workspace_repository
            .create(&workspace)
            .expect("workspace should exist before creating groups");
        group_repository
            .create(&original_group)
            .expect("group should be created");

        let groups = group_repository
            .list_by_workspace(&workspace.id)
            .expect("groups should be listed");
        assert_eq!(groups, vec![original_group.clone()]);

        let loaded_group = group_repository
            .find_by_id(&original_group.id)
            .expect("group should be readable");
        assert_eq!(loaded_group, Some(original_group.clone()));

        let updated_group = GroupNode {
            name: "Platform".into(),
            color: "#0f172a".into(),
            sort_order: 20,
            updated_at: "2026-04-14T10:00:00Z".into(),
            ..original_group.clone()
        };

        let was_updated = group_repository
            .update(&updated_group)
            .expect("group should be updated");
        assert!(was_updated);

        let reloaded_group = group_repository
            .find_by_id(&updated_group.id)
            .expect("group should still exist");
        assert_eq!(reloaded_group, Some(updated_group.clone()));

        let was_deleted = group_repository
            .delete(&updated_group.id)
            .expect("group should be deleted");
        assert!(was_deleted);

        assert!(group_repository
            .find_by_id(&updated_group.id)
            .expect("group lookup should succeed")
            .is_none());
    }
}
