use serde::{Deserialize, Serialize};

use crate::models::{
    CommandArgs, DetectedProjectType, DetectionEvidenceKind, ProjectPackageManager,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionEvidence {
    pub kind: DetectionEvidenceKind,
    pub source: String,
    pub detail: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionWarning {
    pub code: String,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandValidation {
    pub is_runnable: bool,
    pub command_preview: String,
    #[serde(default)]
    pub resolved_executable: Option<String>,
    #[serde(default)]
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub detected_type: DetectedProjectType,
    pub display_name: String,
    pub path: String,
    pub working_dir: Option<String>,
    pub package_manager: Option<ProjectPackageManager>,
    pub executable: Option<String>,
    pub command: Option<String>,
    #[serde(default)]
    pub args: CommandArgs,
    pub command_preview: String,
    pub command_validation: CommandValidation,
    pub confidence: f64,
    #[serde(default)]
    pub evidence: Vec<DetectionEvidence>,
    #[serde(default)]
    pub warnings: Vec<DetectionWarning>,
}

#[cfg(test)]
mod tests {
    use super::{CommandValidation, DetectionEvidence, DetectionResult, DetectionWarning};
    use crate::models::{DetectedProjectType, DetectionEvidenceKind, ProjectPackageManager};

    #[test]
    fn round_trips_detection_result_shape() {
        let result = DetectionResult {
            detected_type: DetectedProjectType::ReactVite,
            display_name: "centralita-ui".into(),
            path: r"C:\Projects\centralita-ui".into(),
            working_dir: Some(r"C:\Projects\centralita-ui".into()),
            package_manager: Some(ProjectPackageManager::Pnpm),
            executable: Some("pnpm".into()),
            command: Some("pnpm dev".into()),
            args: vec!["dev".into()],
            command_preview: "pnpm dev".into(),
            command_validation: CommandValidation {
                is_runnable: true,
                command_preview: "pnpm dev".into(),
                resolved_executable: Some(r"C:\Program Files\nodejs\pnpm.cmd".into()),
                issues: Vec::new(),
            },
            confidence: 0.9,
            evidence: vec![DetectionEvidence {
                kind: DetectionEvidenceKind::Dependency,
                source: "package.json".into(),
                detail: "react dependency found".into(),
                weight: 0.25,
            }],
            warnings: vec![DetectionWarning {
                code: "simple-monorepo".into(),
                message: "Multiple package.json files detected".into(),
                source: Some("packages/app/package.json".into()),
            }],
        };

        let json = serde_json::to_value(&result).expect("detection result should serialize");
        let decoded: DetectionResult =
            serde_json::from_value(json.clone()).expect("detection result should deserialize");

        assert_eq!(decoded, result);
        assert_eq!(json["detectedType"], "reactVite");
        assert_eq!(json["packageManager"], "pnpm");
        assert_eq!(json["commandValidation"]["isRunnable"], true);
    }
}
