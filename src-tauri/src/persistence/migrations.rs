use std::collections::{BTreeMap, HashSet};

use rusqlite::{params, Connection, Transaction};

use crate::persistence::PersistenceResult;

type Migration = fn(&Transaction<'_>) -> PersistenceResult<()>;

#[derive(Clone, Debug)]
struct MigrationGroupRow {
    id: String,
    workspace_id: String,
    parent_group_id: Option<String>,
    name: String,
    sort_order: i64,
    created_at: String,
}

const MIGRATIONS: &[(u32, Migration)] = &[
    (1, migration_v1),
    (2, migration_v2),
    (3, migration_v3),
    (4, migration_v4),
    (5, migration_v5),
];

pub const CURRENT_SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

pub fn apply_all(connection: &mut Connection) -> PersistenceResult<()> {
    let current_version = schema_version(connection)?;

    for (version, migration) in MIGRATIONS {
        if *version <= current_version {
            continue;
        }

        let transaction = connection.unchecked_transaction()?;
        migration(&transaction)?;
        transaction.pragma_update(None, "user_version", *version as i64)?;
        transaction.commit()?;
    }

    Ok(())
}

fn schema_version(connection: &Connection) -> PersistenceResult<u32> {
    let version =
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?;

    Ok(version as u32)
}

fn migration_v1(transaction: &Transaction<'_>) -> PersistenceResult<()> {
    transaction.execute_batch(
    r#"
      CREATE TABLE IF NOT EXISTS workspaces (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS groups (
        id TEXT PRIMARY KEY,
        workspace_id TEXT NOT NULL,
        parent_group_id TEXT NULL,
        name TEXT NOT NULL,
        color TEXT NOT NULL,
        sort_order INTEGER NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
        FOREIGN KEY (parent_group_id) REFERENCES groups(id) ON DELETE CASCADE
      );

      CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        workspace_id TEXT NOT NULL,
        group_id TEXT NOT NULL,
        name TEXT NOT NULL,
        path TEXT NOT NULL,
        detected_type TEXT NULL,
        color TEXT NULL,
        command TEXT NULL,
        args_json TEXT NULL,
        env_json TEXT NULL,
        working_dir TEXT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
        FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
      );

      CREATE INDEX IF NOT EXISTS idx_groups_workspace_id ON groups(workspace_id);
      CREATE INDEX IF NOT EXISTS idx_groups_parent_group_id ON groups(parent_group_id);
      CREATE INDEX IF NOT EXISTS idx_groups_sort_order ON groups(workspace_id, parent_group_id, sort_order);
      CREATE INDEX IF NOT EXISTS idx_projects_workspace_id ON projects(workspace_id);
      CREATE INDEX IF NOT EXISTS idx_projects_group_id ON projects(group_id);
    "#,
  )?;

    Ok(())
}

fn migration_v2(transaction: &Transaction<'_>) -> PersistenceResult<()> {
    transaction.execute_batch(
        r#"
      ALTER TABLE projects ADD COLUMN package_manager TEXT NULL;
      ALTER TABLE projects ADD COLUMN executable TEXT NULL;
      ALTER TABLE projects ADD COLUMN detection_confidence REAL NULL;
      ALTER TABLE projects ADD COLUMN detection_evidence_json TEXT NULL;
      ALTER TABLE projects ADD COLUMN warnings_json TEXT NULL;
    "#,
    )?;

    Ok(())
}

fn migration_v3(transaction: &Transaction<'_>) -> PersistenceResult<()> {
    transaction.execute_batch(
        r#"
      CREATE TABLE IF NOT EXISTS run_history (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        started_at TEXT NOT NULL,
        ended_at TEXT NULL,
        exit_code INTEGER NULL,
        final_runtime_status TEXT NOT NULL,
        stop_reason TEXT NULL,
        error_message TEXT NULL,
        command_preview TEXT NOT NULL,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
      );

      CREATE INDEX IF NOT EXISTS idx_run_history_project_id_started_at
      ON run_history(project_id, started_at DESC);
    "#,
    )?;

    Ok(())
}

fn migration_v4(transaction: &Transaction<'_>) -> PersistenceResult<()> {
    let mut statement = transaction.prepare(
        r#"
      SELECT
        id,
        workspace_id,
        parent_group_id,
        name,
        sort_order,
        created_at
      FROM groups
      ORDER BY
        workspace_id ASC,
        sort_order ASC,
        created_at ASC,
        id ASC
    "#,
    )?;

    let rows = statement.query_map([], |row| {
        Ok(MigrationGroupRow {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            parent_group_id: row.get(2)?,
            name: row.get(3)?,
            sort_order: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    let groups = rows.collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    let mut groups_by_key: BTreeMap<(String, Option<String>, String), Vec<MigrationGroupRow>> =
        BTreeMap::new();
    for group in groups {
        groups_by_key
            .entry((
                group.workspace_id.clone(),
                group.parent_group_id.clone(),
                normalize_group_name(&group.name),
            ))
            .or_default()
            .push(group);
    }

    for duplicate_groups in groups_by_key.values_mut() {
        if duplicate_groups.len() < 2 {
            continue;
        }

        duplicate_groups.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then(left.created_at.cmp(&right.created_at))
                .then(left.id.cmp(&right.id))
        });

        let duplicate_ids = duplicate_groups
            .iter()
            .map(|group| group.id.clone())
            .collect::<HashSet<_>>();
        let canonical_index = duplicate_groups
            .iter()
            .position(|group| {
                group
                    .parent_group_id
                    .as_ref()
                    .map(|parent_id| !duplicate_ids.contains(parent_id))
                    .unwrap_or(true)
            })
            .unwrap_or(0);
        let canonical_group = duplicate_groups.remove(canonical_index);

        for duplicate_group in duplicate_groups.iter() {
            if canonical_group.parent_group_id.as_deref() == Some(duplicate_group.id.as_str()) {
                transaction.execute(
                    "UPDATE groups SET parent_group_id = ?2 WHERE id = ?1",
                    params![canonical_group.id, duplicate_group.parent_group_id],
                )?;
            }

            transaction.execute(
                "UPDATE groups SET parent_group_id = ?1 WHERE parent_group_id = ?2",
                params![canonical_group.id, duplicate_group.id],
            )?;
            transaction.execute(
                "UPDATE projects SET group_id = ?1 WHERE group_id = ?2",
                params![canonical_group.id, duplicate_group.id],
            )?;
            transaction.execute(
                "DELETE FROM groups WHERE id = ?1",
                [duplicate_group.id.as_str()],
            )?;
        }
    }

    transaction.execute_batch(
        r#"
      CREATE UNIQUE INDEX IF NOT EXISTS idx_groups_workspace_name_unique
      ON groups(workspace_id, COALESCE(parent_group_id, ''), lower(trim(name)));
    "#,
    )?;

    Ok(())
}

fn migration_v5(transaction: &Transaction<'_>) -> PersistenceResult<()> {
    transaction.execute_batch(
        r#"
      DROP INDEX IF EXISTS idx_groups_workspace_name_unique;

      CREATE UNIQUE INDEX IF NOT EXISTS idx_groups_workspace_parent_name_unique
      ON groups(workspace_id, COALESCE(parent_group_id, ''), lower(trim(name)));
    "#,
    )?;

    Ok(())
}

fn normalize_group_name(name: &str) -> String {
    name.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::apply_all;
    use crate::persistence::PersistenceResult;

    #[test]
    fn migrates_v2_schema_without_losing_existing_projects() {
        let mut connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        create_v2_schema(&mut connection).expect("v2 schema should be created");

        apply_all(&mut connection).expect("all migrations should apply");

        let run_history_exists = table_exists(&connection, "run_history");
        let project_name: String = connection
            .query_row(
                "SELECT name FROM projects WHERE id = 'project-1'",
                [],
                |row| row.get(0),
            )
            .expect("existing project should remain");

        assert!(run_history_exists);
        assert_eq!(project_name, "API");
    }

    #[test]
    fn migrates_v3_schema_by_removing_duplicate_groups() {
        let mut connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        create_v3_schema_with_duplicate_groups(&mut connection)
            .expect("v3 schema with duplicated groups should be created");

        apply_all(&mut connection).expect("all migrations should apply");

        let frontend_group_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM groups WHERE workspace_id = 'workspace-1' AND lower(trim(name)) = 'frontend'",
                [],
                |row| row.get(0),
            )
            .expect("frontend groups should be countable");
        let ui_parent_group_id: String = connection
            .query_row(
                "SELECT parent_group_id FROM groups WHERE id = 'group-ui'",
                [],
                |row| row.get(0),
            )
            .expect("ui group should remain");
        let frontend_project_group_id: String = connection
            .query_row(
                "SELECT group_id FROM projects WHERE id = 'project-frontend'",
                [],
                |row| row.get(0),
            )
            .expect("frontend project should remain");
        let duplicate_insert_error = connection.execute(
            INSERT_GROUP_SQL,
            params![
                "group-frontend-again",
                "workspace-1",
                Option::<String>::None,
                " FRONTEND ",
                "#2563eb",
                30,
                "2026-04-17T11:00:00Z",
                "2026-04-17T11:00:00Z",
            ],
        );
        let different_parent_insert = connection.execute(
            INSERT_GROUP_SQL,
            params![
                "group-frontend-under-ui",
                "workspace-1",
                Some("group-ui"),
                " FRONTEND ",
                "#2563eb",
                30,
                "2026-04-17T11:00:00Z",
                "2026-04-17T11:00:00Z",
            ],
        );

        assert_eq!(frontend_group_count, 1);
        assert_eq!(ui_parent_group_id, "group-frontend-primary");
        assert_eq!(frontend_project_group_id, "group-frontend-primary");
        assert!(duplicate_insert_error.is_err());
        assert!(different_parent_insert.is_ok());
    }

    #[test]
    fn migrates_v4_schema_to_allow_matching_names_under_different_parents() {
        let mut connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        create_v4_schema_with_global_group_name_index(&mut connection)
            .expect("v4 schema should be created");

        apply_all(&mut connection).expect("v5 migration should apply");

        let different_parent_insert = connection.execute(
            INSERT_GROUP_SQL,
            params![
                "group-frontend-under-ui",
                "workspace-1",
                Some("group-ui"),
                " FRONTEND ",
                "#2563eb",
                30,
                "2026-04-17T11:00:00Z",
                "2026-04-17T11:00:00Z",
            ],
        );
        let same_parent_insert = connection.execute(
            INSERT_GROUP_SQL,
            params![
                "group-frontend-root",
                "workspace-1",
                Option::<String>::None,
                " FRONTEND ",
                "#2563eb",
                40,
                "2026-04-17T11:30:00Z",
                "2026-04-17T11:30:00Z",
            ],
        );

        assert!(different_parent_insert.is_ok());
        assert!(same_parent_insert.is_err());
    }

    const INSERT_GROUP_SQL: &str = r#"
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
    "#;

    fn create_v2_schema(connection: &mut Connection) -> PersistenceResult<()> {
        connection.execute_batch(
            r#"
          CREATE TABLE workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
          );
          CREATE TABLE groups (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            parent_group_id TEXT NULL,
            name TEXT NOT NULL,
            color TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
          );
          CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            group_id TEXT NOT NULL,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            detected_type TEXT NULL,
            color TEXT NULL,
            command TEXT NULL,
            args_json TEXT NULL,
            env_json TEXT NULL,
            working_dir TEXT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            package_manager TEXT NULL,
            executable TEXT NULL,
            detection_confidence REAL NULL,
            detection_evidence_json TEXT NULL,
            warnings_json TEXT NULL
          );
          INSERT INTO workspaces (id, name, created_at, updated_at)
          VALUES ('workspace-1', 'Main', '2026-04-17T09:00:00Z', '2026-04-17T09:00:00Z');
          INSERT INTO groups (id, workspace_id, parent_group_id, name, color, sort_order, created_at, updated_at)
          VALUES ('group-1', 'workspace-1', NULL, 'Backend', '#2563eb', 10, '2026-04-17T09:00:00Z', '2026-04-17T09:00:00Z');
          INSERT INTO projects (
            id,
            workspace_id,
            group_id,
            name,
            path,
            detected_type,
            color,
            command,
            args_json,
            env_json,
            working_dir,
            created_at,
            updated_at,
            package_manager,
            executable,
            detection_confidence,
            detection_evidence_json,
            warnings_json
          )
          VALUES (
            'project-1',
            'workspace-1',
            'group-1',
            'API',
            'C:\Projects\api',
            NULL,
            NULL,
            'npm run dev',
            '[]',
            NULL,
            'C:\Projects\api',
            '2026-04-17T09:00:00Z',
            '2026-04-17T09:00:00Z',
            'npm',
            'npm',
            NULL,
            NULL,
            NULL
          );
          PRAGMA user_version = 2;
        "#,
        )?;

        Ok(())
    }

    fn create_v3_schema_with_duplicate_groups(
        connection: &mut Connection,
    ) -> PersistenceResult<()> {
        connection.execute_batch(
            r#"
          CREATE TABLE workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
          );
          CREATE TABLE groups (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            parent_group_id TEXT NULL,
            name TEXT NOT NULL,
            color TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
          );
          CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            group_id TEXT NOT NULL,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            detected_type TEXT NULL,
            color TEXT NULL,
            command TEXT NULL,
            args_json TEXT NULL,
            env_json TEXT NULL,
            working_dir TEXT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            package_manager TEXT NULL,
            executable TEXT NULL,
            detection_confidence REAL NULL,
            detection_evidence_json TEXT NULL,
            warnings_json TEXT NULL
          );
          CREATE TABLE run_history (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT NULL,
            exit_code INTEGER NULL,
            final_runtime_status TEXT NOT NULL,
            stop_reason TEXT NULL,
            error_message TEXT NULL,
            command_preview TEXT NOT NULL
          );

          INSERT INTO workspaces (id, name, created_at, updated_at)
          VALUES ('workspace-1', 'Main', '2026-04-17T09:00:00Z', '2026-04-17T09:00:00Z');

          INSERT INTO groups (id, workspace_id, parent_group_id, name, color, sort_order, created_at, updated_at)
          VALUES
            ('group-frontend-primary', 'workspace-1', NULL, 'Frontend', '#2563eb', 10, '2026-04-17T09:00:00Z', '2026-04-17T09:00:00Z'),
            ('group-frontend-duplicate', 'workspace-1', NULL, ' frontend ', '#2563eb', 20, '2026-04-17T10:00:00Z', '2026-04-17T10:00:00Z'),
            ('group-ui', 'workspace-1', 'group-frontend-duplicate', 'UI', '#0f172a', 10, '2026-04-17T10:30:00Z', '2026-04-17T10:30:00Z');

          INSERT INTO projects (
            id,
            workspace_id,
            group_id,
            name,
            path,
            detected_type,
            color,
            command,
            args_json,
            env_json,
            working_dir,
            created_at,
            updated_at,
            package_manager,
            executable,
            detection_confidence,
            detection_evidence_json,
            warnings_json
          )
          VALUES (
            'project-frontend',
            'workspace-1',
            'group-frontend-duplicate',
            'App',
            'C:\Projects\frontend',
            NULL,
            NULL,
            'npm run dev',
            '[]',
            NULL,
            'C:\Projects\frontend',
            '2026-04-17T10:00:00Z',
            '2026-04-17T10:00:00Z',
            'npm',
            'npm',
            NULL,
            NULL,
            NULL
          );

          PRAGMA user_version = 3;
        "#,
        )?;

        Ok(())
    }

    fn create_v4_schema_with_global_group_name_index(
        connection: &mut Connection,
    ) -> PersistenceResult<()> {
        connection.execute_batch(
            r#"
          CREATE TABLE workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
          );
          CREATE TABLE groups (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            parent_group_id TEXT NULL,
            name TEXT NOT NULL,
            color TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
          );
          CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            group_id TEXT NOT NULL,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            detected_type TEXT NULL,
            color TEXT NULL,
            command TEXT NULL,
            args_json TEXT NULL,
            env_json TEXT NULL,
            working_dir TEXT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            package_manager TEXT NULL,
            executable TEXT NULL,
            detection_confidence REAL NULL,
            detection_evidence_json TEXT NULL,
            warnings_json TEXT NULL
          );
          CREATE TABLE run_history (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT NULL,
            exit_code INTEGER NULL,
            final_runtime_status TEXT NOT NULL,
            stop_reason TEXT NULL,
            error_message TEXT NULL,
            command_preview TEXT NOT NULL
          );

          INSERT INTO workspaces (id, name, created_at, updated_at)
          VALUES ('workspace-1', 'Main', '2026-04-17T09:00:00Z', '2026-04-17T09:00:00Z');

          INSERT INTO groups (id, workspace_id, parent_group_id, name, color, sort_order, created_at, updated_at)
          VALUES
            ('group-frontend', 'workspace-1', NULL, 'Frontend', '#2563eb', 10, '2026-04-17T09:00:00Z', '2026-04-17T09:00:00Z'),
            ('group-ui', 'workspace-1', 'group-frontend', 'UI', '#0f172a', 10, '2026-04-17T10:30:00Z', '2026-04-17T10:30:00Z');

          CREATE UNIQUE INDEX idx_groups_workspace_name_unique
          ON groups(workspace_id, lower(trim(name)));

          PRAGMA user_version = 4;
        "#,
        )?;

        Ok(())
    }

    fn table_exists(connection: &Connection, table_name: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table_name],
                |row| row.get::<_, i64>(0),
            )
            .expect("sqlite_master should be readable")
            == 1
    }
}
