use rusqlite::{params, OptionalExtension};

use crate::{
    models::{
        DetectedProjectType, EntityId, HealthCheckConfig, HealthCheckKind, HttpHealthCheckConfig,
        ProjectNode, ProjectPackageManager, TcpHealthCheckConfig,
    },
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
          healthcheck_type,
          healthcheck_enabled,
          healthcheck_interval_ms,
          healthcheck_timeout_ms,
          healthcheck_grace_period_ms,
          healthcheck_success_threshold,
          healthcheck_failure_threshold,
          healthcheck_payload_json,
          created_at,
          updated_at
        )
        VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
          ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26
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
                serialize_health_check_type(&project.health_check)?,
                healthcheck_enabled(&project.health_check),
                healthcheck_interval_ms(&project.health_check),
                healthcheck_timeout_ms(&project.health_check),
                healthcheck_grace_period_ms(&project.health_check),
                healthcheck_success_threshold(&project.health_check),
                healthcheck_failure_threshold(&project.health_check),
                serialize_json_option(&project.health_check)?,
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
          healthcheck_type,
          healthcheck_enabled,
          healthcheck_interval_ms,
          healthcheck_timeout_ms,
          healthcheck_grace_period_ms,
          healthcheck_success_threshold,
          healthcheck_failure_threshold,
          healthcheck_payload_json,
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
                healthcheck_type: row.get(16)?,
                healthcheck_enabled: row.get(17)?,
                healthcheck_interval_ms: row.get(18)?,
                healthcheck_timeout_ms: row.get(19)?,
                healthcheck_grace_period_ms: row.get(20)?,
                healthcheck_success_threshold: row.get(21)?,
                healthcheck_failure_threshold: row.get(22)?,
                healthcheck_payload_json: row.get(23)?,
                created_at: row.get(24)?,
                updated_at: row.get(25)?,
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
            healthcheck_type,
            healthcheck_enabled,
            healthcheck_interval_ms,
            healthcheck_timeout_ms,
            healthcheck_grace_period_ms,
            healthcheck_success_threshold,
            healthcheck_failure_threshold,
            healthcheck_payload_json,
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
                        healthcheck_type: row.get(16)?,
                        healthcheck_enabled: row.get(17)?,
                        healthcheck_interval_ms: row.get(18)?,
                        healthcheck_timeout_ms: row.get(19)?,
                        healthcheck_grace_period_ms: row.get(20)?,
                        healthcheck_success_threshold: row.get(21)?,
                        healthcheck_failure_threshold: row.get(22)?,
                        healthcheck_payload_json: row.get(23)?,
                        created_at: row.get(24)?,
                        updated_at: row.get(25)?,
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
          healthcheck_type = ?17,
          healthcheck_enabled = ?18,
          healthcheck_interval_ms = ?19,
          healthcheck_timeout_ms = ?20,
          healthcheck_grace_period_ms = ?21,
          healthcheck_success_threshold = ?22,
          healthcheck_failure_threshold = ?23,
          healthcheck_payload_json = ?24,
          created_at = ?25,
          updated_at = ?26
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
                serialize_health_check_type(&project.health_check)?,
                healthcheck_enabled(&project.health_check),
                healthcheck_interval_ms(&project.health_check),
                healthcheck_timeout_ms(&project.health_check),
                healthcheck_grace_period_ms(&project.health_check),
                healthcheck_success_threshold(&project.health_check),
                healthcheck_failure_threshold(&project.health_check),
                serialize_json_option(&project.health_check)?,
                project.created_at,
                project.updated_at
            ],
        )?;

        Ok(updated_rows > 0)
    }

    pub fn update_health_check(
        &self,
        project_id: &EntityId,
        health_check: &Option<HealthCheckConfig>,
        updated_at: &str,
    ) -> PersistenceResult<bool> {
        let connection = self.database.connect()?;
        let updated_rows = connection.execute(
            r#"
        UPDATE projects
        SET
          healthcheck_type = ?2,
          healthcheck_enabled = ?3,
          healthcheck_interval_ms = ?4,
          healthcheck_timeout_ms = ?5,
          healthcheck_grace_period_ms = ?6,
          healthcheck_success_threshold = ?7,
          healthcheck_failure_threshold = ?8,
          healthcheck_payload_json = ?9,
          updated_at = ?10
        WHERE id = ?1
      "#,
            params![
                project_id,
                serialize_health_check_type(health_check)?,
                healthcheck_enabled(health_check),
                healthcheck_interval_ms(health_check),
                healthcheck_timeout_ms(health_check),
                healthcheck_grace_period_ms(health_check),
                healthcheck_success_threshold(health_check),
                healthcheck_failure_threshold(health_check),
                serialize_json_option(health_check)?,
                updated_at
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
    healthcheck_type: Option<String>,
    healthcheck_enabled: Option<i64>,
    healthcheck_interval_ms: Option<i64>,
    healthcheck_timeout_ms: Option<i64>,
    healthcheck_grace_period_ms: Option<i64>,
    healthcheck_success_threshold: Option<i64>,
    healthcheck_failure_threshold: Option<i64>,
    healthcheck_payload_json: Option<String>,
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
            health_check: deserialize_health_check(
                record.healthcheck_type,
                record.healthcheck_enabled,
                record.healthcheck_interval_ms,
                record.healthcheck_timeout_ms,
                record.healthcheck_grace_period_ms,
                record.healthcheck_success_threshold,
                record.healthcheck_failure_threshold,
                record.healthcheck_payload_json,
            )?,
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

fn serialize_health_check_type(
    value: &Option<HealthCheckConfig>,
) -> PersistenceResult<Option<String>> {
    match value {
        Some(value) => match serde_json::to_value(value.kind())? {
            serde_json::Value::String(serialized) => Ok(Some(serialized)),
            _ => Err(std::io::Error::other("health check type must serialize as string").into()),
        },
        None => Ok(None),
    }
}

fn deserialize_health_check(
    healthcheck_type: Option<String>,
    healthcheck_enabled: Option<i64>,
    healthcheck_interval_ms: Option<i64>,
    healthcheck_timeout_ms: Option<i64>,
    healthcheck_grace_period_ms: Option<i64>,
    healthcheck_success_threshold: Option<i64>,
    healthcheck_failure_threshold: Option<i64>,
    healthcheck_payload_json: Option<String>,
) -> PersistenceResult<Option<HealthCheckConfig>> {
    let Some(healthcheck_type) = healthcheck_type else {
        return Ok(None);
    };
    let kind: HealthCheckKind =
        serde_json::from_value(serde_json::Value::String(healthcheck_type))?;
    let payload = healthcheck_payload_json.ok_or_else(|| {
        std::io::Error::other("health check payload is required when health check type is present")
    })?;

    match kind {
        HealthCheckKind::Http => {
            let mut config: HttpHealthCheckConfig = serde_json::from_str(&payload)?;
            if let Some(enabled) = healthcheck_enabled {
                config.enabled = enabled != 0;
            }
            if let Some(interval_ms) = healthcheck_interval_ms {
                config.interval_ms = interval_ms as u64;
            }
            if let Some(timeout_ms) = healthcheck_timeout_ms {
                config.timeout_ms = timeout_ms as u64;
            }
            if let Some(grace_period_ms) = healthcheck_grace_period_ms {
                config.grace_period_ms = grace_period_ms as u64;
            }
            if let Some(success_threshold) = healthcheck_success_threshold {
                config.success_threshold = success_threshold as u32;
            }
            if let Some(failure_threshold) = healthcheck_failure_threshold {
                config.failure_threshold = failure_threshold as u32;
            }

            Ok(Some(HealthCheckConfig::Http(config).normalized()))
        }
        HealthCheckKind::Tcp => {
            let mut config: TcpHealthCheckConfig = serde_json::from_str(&payload)?;
            if let Some(enabled) = healthcheck_enabled {
                config.enabled = enabled != 0;
            }
            if let Some(interval_ms) = healthcheck_interval_ms {
                config.interval_ms = interval_ms as u64;
            }
            if let Some(timeout_ms) = healthcheck_timeout_ms {
                config.timeout_ms = timeout_ms as u64;
            }
            if let Some(grace_period_ms) = healthcheck_grace_period_ms {
                config.grace_period_ms = grace_period_ms as u64;
            }
            if let Some(success_threshold) = healthcheck_success_threshold {
                config.success_threshold = success_threshold as u32;
            }
            if let Some(failure_threshold) = healthcheck_failure_threshold {
                config.failure_threshold = failure_threshold as u32;
            }

            Ok(Some(HealthCheckConfig::Tcp(config).normalized()))
        }
    }
}

fn healthcheck_enabled(value: &Option<HealthCheckConfig>) -> Option<i64> {
    value.as_ref().map(|config| i64::from(config.enabled()))
}

fn healthcheck_interval_ms(value: &Option<HealthCheckConfig>) -> Option<i64> {
    value.as_ref().map(|config| config.interval_ms() as i64)
}

fn healthcheck_timeout_ms(value: &Option<HealthCheckConfig>) -> Option<i64> {
    value.as_ref().map(|config| config.timeout_ms() as i64)
}

fn healthcheck_grace_period_ms(value: &Option<HealthCheckConfig>) -> Option<i64> {
    value.as_ref().map(|config| config.grace_period_ms() as i64)
}

fn healthcheck_success_threshold(value: &Option<HealthCheckConfig>) -> Option<i64> {
    value
        .as_ref()
        .map(|config| config.success_threshold() as i64)
}

fn healthcheck_failure_threshold(value: &Option<HealthCheckConfig>) -> Option<i64> {
    value
        .as_ref()
        .map(|config| config.failure_threshold() as i64)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::ProjectRepository;
    use crate::{
        models::{
            DetectedProjectType, DetectionEvidence, DetectionEvidenceKind, DetectionWarning,
            GroupNode, HealthCheckConfig, HttpHealthCheckConfig, ProjectNode,
            ProjectPackageManager, Workspace,
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
            health_check: Some(HealthCheckConfig::Http(HttpHealthCheckConfig {
                enabled: true,
                interval_ms: 5_000,
                timeout_ms: 2_000,
                grace_period_ms: 3_000,
                success_threshold: 1,
                failure_threshold: 2,
                url: "http://127.0.0.1:3000/health".into(),
                method: "GET".into(),
                expected_status_codes: vec![200],
                headers: None,
                contains_text: None,
            })),
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
            health_check: None,
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
