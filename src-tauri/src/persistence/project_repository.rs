use rusqlite::{params, OptionalExtension};

use crate::{
    models::{DetectedProjectType, EntityId, ProjectNode, ProjectPackageManager},
    persistence::{AppDatabase, PersistenceResult},
};

#[derive(Debug, Clone)]
pub struct ProjectRepository {
    database: AppDatabase,
}

impl ProjectRepository {
    pub fn new(database: AppDatabase) -> Self {
        Self { database }
    }

    pub fn create(&self, project: &ProjectNode) -> PersistenceResult<()> {
        let connection = self.database.connect()?;

        connection.execute(
            r#"
        INSERT INTO projects (
          id,
          workspace_id,
          group_id,
          name,
          path,
          detected_type,
          color,
          package_manager,
          executable,
          command,
          args_json,
          env_json,
          working_dir,
          detection_confidence,
          detection_evidence_json,
          warnings_json,
          created_at,
          updated_at
        )
        VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
          ?17, ?18
        )
      "#,
            params![
                project.id,
                project.workspace_id,
                project.group_id,
                project.name,
                project.path,
                serialize_detected_type(&project.detected_type)?,
                project.color,
                serialize_package_manager(&project.package_manager)?,
                project.executable,
                project.command,
                serialize_json_option(&project.args)?,
                serialize_json_option(&project.env)?,
                project.working_dir,
                project.detection_confidence,
                serialize_json_option(&project.detection_evidence)?,
                serialize_json_option(&project.warnings)?,
                project.created_at,
                project.updated_at
            ],
        )?;

        Ok(())
    }

    pub fn list_by_workspace(
        &self,
        workspace_id: &EntityId,
    ) -> PersistenceResult<Vec<ProjectNode>> {
        let connection = self.database.connect()?;
        let mut statement = connection.prepare(
            r#"
        SELECT
          id,
          workspace_id,
          group_id,
          name,
          path,
          detected_type,
          color,
          package_manager,
          executable,
          command,
          args_json,
          env_json,
          working_dir,
          detection_confidence,
          detection_evidence_json,
          warnings_json,
          created_at,
          updated_at
        FROM projects
        WHERE workspace_id = ?1
        ORDER BY group_id ASC, name ASC, id ASC
      "#,
        )?;

        let rows = statement.query_map([workspace_id], |row| {
            Ok(ProjectRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                group_id: row.get(2)?,
                name: row.get(3)?,
                path: row.get(4)?,
                detected_type: row.get(5)?,
                color: row.get(6)?,
                package_manager: row.get(7)?,
                executable: row.get(8)?,
                command: row.get(9)?,
                args_json: row.get(10)?,
                env_json: row.get(11)?,
                working_dir: row.get(12)?,
                detection_confidence: row.get(13)?,
                detection_evidence_json: row.get(14)?,
                warnings_json: row.get(15)?,
                created_at: row.get(16)?,
                updated_at: row.get(17)?,
            })
        })?;

        rows.map(|row| row.map_err(Into::into).and_then(ProjectNode::try_from))
            .collect()
    }

    pub fn find_by_id(&self, project_id: &EntityId) -> PersistenceResult<Option<ProjectNode>> {
        let connection = self.database.connect()?;
        let record = connection
            .query_row(
                r#"
          SELECT
            id,
            workspace_id,
            group_id,
            name,
            path,
            detected_type,
            color,
            package_manager,
            executable,
            command,
            args_json,
            env_json,
            working_dir,
            detection_confidence,
            detection_evidence_json,
            warnings_json,
            created_at,
            updated_at
          FROM projects
          WHERE id = ?1
        "#,
                [project_id],
                |row| {
                    Ok(ProjectRecord {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        group_id: row.get(2)?,
                        name: row.get(3)?,
                        path: row.get(4)?,
                        detected_type: row.get(5)?,
                        color: row.get(6)?,
                        package_manager: row.get(7)?,
                        executable: row.get(8)?,
                        command: row.get(9)?,
                        args_json: row.get(10)?,
                        env_json: row.get(11)?,
                        working_dir: row.get(12)?,
                        detection_confidence: row.get(13)?,
                        detection_evidence_json: row.get(14)?,
                        warnings_json: row.get(15)?,
                        created_at: row.get(16)?,
                        updated_at: row.get(17)?,
                    })
                },
            )
            .optional()?;

        record.map(ProjectNode::try_from).transpose()
    }

    pub fn update(&self, project: &ProjectNode) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let updated_rows = connection.execute(
            r#"
        UPDATE projects
        SET
          workspace_id = ?2,
          group_id = ?3,
          name = ?4,
          path = ?5,
          detected_type = ?6,
          color = ?7,
          package_manager = ?8,
          executable = ?9,
          command = ?10,
          args_json = ?11,
          env_json = ?12,
          working_dir = ?13,
          detection_confidence = ?14,
          detection_evidence_json = ?15,
          warnings_json = ?16,
          created_at = ?17,
          updated_at = ?18
        WHERE id = ?1
      "#,
            params![
                project.id,
                project.workspace_id,
                project.group_id,
                project.name,
                project.path,
                serialize_detected_type(&project.detected_type)?,
                project.color,
                serialize_package_manager(&project.package_manager)?,
                project.executable,
                project.command,
                serialize_json_option(&project.args)?,
                serialize_json_option(&project.env)?,
                project.working_dir,
                project.detection_confidence,
                serialize_json_option(&project.detection_evidence)?,
                serialize_json_option(&project.warnings)?,
                project.created_at,
                project.updated_at
            ],
        )?;

        Ok(updated_rows > 0)
    }

    pub fn delete(&self, project_id: &EntityId) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let deleted_rows =
            connection.execute("DELETE FROM projects WHERE id = ?1", [project_id])?;

        Ok(deleted_rows > 0)
    }
}

#[derive(Debug)]
struct ProjectRecord {
    id: EntityId,
    workspace_id: EntityId,
    group_id: EntityId,
    name: String,
    path: String,
    detected_type: Option<String>,
    color: Option<String>,
    package_manager: Option<String>,
    executable: Option<String>,
    command: Option<String>,
    args_json: Option<String>,
    env_json: Option<String>,
    working_dir: Option<String>,
    detection_confidence: Option<f64>,
    detection_evidence_json: Option<String>,
    warnings_json: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<ProjectRecord> for ProjectNode {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(record: ProjectRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            id: record.id,
            workspace_id: record.workspace_id,
            group_id: record.group_id,
            name: record.name,
            path: record.path,
            detected_type: deserialize_detected_type(record.detected_type)?,
            color: record.color,
            package_manager: deserialize_package_manager(record.package_manager)?,
            executable: record.executable,
            command: record.command,
            args: deserialize_json_option(record.args_json)?,
            env: deserialize_json_option(record.env_json)?,
            working_dir: record.working_dir,
            detection_confidence: record.detection_confidence,
            detection_evidence: deserialize_json_option(record.detection_evidence_json)?,
            warnings: deserialize_json_option(record.warnings_json)?,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }
}

fn serialize_detected_type(
    value: &Option<DetectedProjectType>,
) -> PersistenceResult<Option<String>> {
    match value {
        Some(value) => match serde_json::to_value(value)? {
            serde_json::Value::String(serialized) => Ok(Some(serialized)),
            _ => {
                Err(std::io::Error::other("detected project type must serialize as string").into())
            }
        },
        None => Ok(None),
    }
}

fn deserialize_detected_type(
    value: Option<String>,
) -> PersistenceResult<Option<DetectedProjectType>> {
    match value {
        Some(value) => Ok(Some(serde_json::from_value(serde_json::Value::String(
            value,
        ))?)),
        None => Ok(None),
    }
}

fn serialize_package_manager(
    value: &Option<ProjectPackageManager>,
) -> PersistenceResult<Option<String>> {
    match value {
        Some(value) => match serde_json::to_value(value)? {
            serde_json::Value::String(serialized) => Ok(Some(serialized)),
            _ => Err(std::io::Error::other("package manager must serialize as string").into()),
        },
        None => Ok(None),
    }
}

fn deserialize_package_manager(
    value: Option<String>,
) -> PersistenceResult<Option<ProjectPackageManager>> {
    match value {
        Some(value) => Ok(Some(serde_json::from_value(serde_json::Value::String(
            value,
        ))?)),
        None => Ok(None),
    }
}

fn serialize_json_option<T>(value: &Option<T>) -> PersistenceResult<Option<String>>
where
    T: serde::Serialize,
{
    match value {
        Some(value) => Ok(Some(serde_json::to_string(value)?)),
        None => Ok(None),
    }
}

fn deserialize_json_option<T>(value: Option<String>) -> PersistenceResult<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match value {
        Some(value) => Ok(Some(serde_json::from_str(&value)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::ProjectRepository;
    use crate::{
        models::{
            DetectedProjectType, DetectionEvidence, DetectionEvidenceKind, DetectionWarning,
            GroupNode, ProjectNode, ProjectPackageManager, Workspace,
        },
        persistence::{test_utils::TestDatabase, GroupRepository, WorkspaceRepository},
    };

    #[test]
    fn supports_project_crud() {
        let test_database = TestDatabase::new("project-repository-crud");
        let workspace_repository = WorkspaceRepository::new(test_database.database());
        let group_repository = GroupRepository::new(test_database.database());
        let project_repository = ProjectRepository::new(test_database.database());
        let workspace = Workspace {
            id: "workspace-main".into(),
            name: "Centralita".into(),
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let group = GroupNode {
            id: "group-dev".into(),
            workspace_id: workspace.id.clone(),
            parent_group_id: None,
            name: "Development".into(),
            color: "#3b82f6".into(),
            sort_order: 10,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let mut env = BTreeMap::new();
        env.insert("PORT".into(), "3000".into());

        let original_project = ProjectNode {
            id: "project-ui".into(),
            workspace_id: workspace.id.clone(),
            group_id: group.id.clone(),
            name: "UI".into(),
            path: r"C:\Projects\ui".into(),
            detected_type: Some(DetectedProjectType::Vite),
            color: Some("#f59e0b".into()),
            package_manager: Some(ProjectPackageManager::Npm),
            executable: Some("npm".into()),
            command: Some("npm run dev".into()),
            args: Some(vec!["--host".into(), "0.0.0.0".into()]),
            env: Some(env),
            working_dir: Some(r"C:\Projects\ui".into()),
            detection_confidence: Some(0.8),
            detection_evidence: Some(vec![DetectionEvidence {
                kind: DetectionEvidenceKind::StructuralFile,
                source: "vite.config.ts".into(),
                detail: "vite config found".into(),
                weight: 0.35,
            }]),
            warnings: Some(vec![DetectionWarning {
                code: "review-command".into(),
                message: "Command inferred from project scripts".into(),
                source: Some("package.json".into()),
            }]),
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };

        workspace_repository
            .create(&workspace)
            .expect("workspace should exist before creating projects");
        group_repository
            .create(&group)
            .expect("group should exist before creating projects");
        project_repository
            .create(&original_project)
            .expect("project should be created");

        let projects = project_repository
            .list_by_workspace(&workspace.id)
            .expect("projects should be listed");
        assert_eq!(projects, vec![original_project.clone()]);

        let loaded_project = project_repository
            .find_by_id(&original_project.id)
            .expect("project should be readable");
        assert_eq!(loaded_project, Some(original_project.clone()));

        let updated_project = ProjectNode {
            name: "UI Updated".into(),
            detected_type: Some(DetectedProjectType::ReactVite),
            color: None,
            package_manager: Some(ProjectPackageManager::Pnpm),
            executable: Some("pnpm".into()),
            command: Some("pnpm dev".into()),
            args: None,
            env: None,
            detection_confidence: Some(0.92),
            detection_evidence: Some(vec![DetectionEvidence {
                kind: DetectionEvidenceKind::Dependency,
                source: "package.json".into(),
                detail: "react dependency found".into(),
                weight: 0.25,
            }]),
            warnings: None,
            updated_at: "2026-04-14T10:00:00Z".into(),
            ..original_project.clone()
        };

        let was_updated = project_repository
            .update(&updated_project)
            .expect("project should be updated");
        assert!(was_updated);

        let reloaded_project = project_repository
            .find_by_id(&updated_project.id)
            .expect("project should still exist");
        assert_eq!(reloaded_project, Some(updated_project.clone()));

        let was_deleted = project_repository
            .delete(&updated_project.id)
            .expect("project should be deleted");
        assert!(was_deleted);

        assert!(project_repository
            .find_by_id(&updated_project.id)
            .expect("project lookup should succeed")
            .is_none());
    }
}
