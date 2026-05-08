use serde::{Deserialize, Serialize};

use crate::models::{
    CommandArgs, DetectedProjectType, DetectionEvidence, DetectionWarning, EntityId,
    EnvironmentVariables, IsoDateTime, ProjectPackageManager,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectNode {
    pub id: EntityId,
    pub workspace_id: EntityId,
    pub group_id: EntityId,
    pub name: String,
    pub path: String,
    pub detected_type: Option<DetectedProjectType>,
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<ProjectPackageManager>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<CommandArgs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvironmentVariables>,
    pub working_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_evidence: Option<Vec<DetectionEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<DetectionWarning>>,
    pub created_at: IsoDateTime,
    pub updated_at: IsoDateTime,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::ProjectNode;
    use crate::models::{
        DetectedProjectType, DetectionEvidence, DetectionEvidenceKind, DetectionWarning,
        ProjectPackageManager,
    };

    #[test]
    fn round_trips_s1_project_shape() {
        let mut env = BTreeMap::new();
        env.insert("PORT".into(), "1420".into());

        let project = ProjectNode {
            id: "project-api".into(),
            workspace_id: "workspace-main".into(),
            group_id: "group-dev".into(),
            name: "API".into(),
            path: r"C:\Projects\api".into(),
            detected_type: Some(DetectedProjectType::Vite),
            color: None,
            package_manager: Some(ProjectPackageManager::Npm),
            executable: Some("npm".into()),
            command: Some("npm run dev".into()),
            args: Some(vec!["--host".into(), "0.0.0.0".into()]),
            env: Some(env),
            working_dir: Some(r"C:\Projects\api".into()),
            detection_confidence: Some(0.85),
            detection_evidence: Some(vec![DetectionEvidence {
                kind: DetectionEvidenceKind::Script,
                source: "package.json".into(),
                detail: "dev script uses vite".into(),
                weight: 0.15,
            }]),
            warnings: Some(vec![DetectionWarning {
                code: "manual-review".into(),
                message: "Command should be reviewed before saving".into(),
                source: None,
            }]),
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };

        let json = serde_json::to_value(&project).expect("project should serialize");
        let decoded: ProjectNode =
            serde_json::from_value(json.clone()).expect("project should deserialize");

        assert_eq!(decoded, project);
        assert_eq!(json["detectedType"], "vite");
        assert_eq!(json["packageManager"], "npm");
        assert_eq!(json["args"][0], "--host");
    }

    #[test]
    fn deserializes_missing_optional_args_and_env() {
        let payload = r#"{
      "id": "project-ui",
      "workspaceId": "workspace-main",
      "groupId": "group-dev",
      "name": "UI",
      "path": "C:\\Projects\\ui",
      "detectedType": null,
      "color": null,
      "packageManager": null,
      "executable": null,
      "command": null,
      "workingDir": null,
      "detectionConfidence": null,
      "detectionEvidence": null,
      "warnings": null,
      "createdAt": "2026-04-14T09:00:00Z",
      "updatedAt": "2026-04-14T09:00:00Z"
    }"#;

        let project: ProjectNode =
            serde_json::from_str(payload).expect("project should deserialize");

        assert!(project.args.is_none());
        assert!(project.env.is_none());
        assert!(project.package_manager.is_none());
        assert!(project.detection_evidence.is_none());
    }
}
