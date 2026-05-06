use std::path::Path;

use crate::models::{
    DetectedProjectType, DetectionEvidence, DetectionEvidenceKind, DetectionResult,
    DetectionWarning,
};

use super::{
    java, node, scan_project_folder, validate_command, ProjectScanError, ProjectScanSnapshot,
};

#[derive(Debug)]
pub enum ProjectAnalysisError {
    Scan(ProjectScanError),
}

impl std::fmt::Display for ProjectAnalysisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scan(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ProjectAnalysisError {}

impl From<ProjectScanError> for ProjectAnalysisError {
    fn from(value: ProjectScanError) -> Self {
        Self::Scan(value)
    }
}

#[derive(Debug)]
pub(super) struct DetectionAttempt {
    pub candidate: Option<DetectionResult>,
    pub warnings: Vec<DetectionWarning>,
}

pub fn analyze_project_folder(
    path: impl AsRef<Path>,
) -> Result<DetectionResult, ProjectAnalysisError> {
    let snapshot = scan_project_folder(path)?;

    Ok(analyze_snapshot(&snapshot))
}

fn analyze_snapshot(snapshot: &ProjectScanSnapshot) -> DetectionResult {
    let attempts = [node::analyze(snapshot), java::analyze(snapshot)];
    let mut candidates = Vec::new();
    let mut accumulated_warnings = snapshot.warnings.clone();

    for attempt in attempts {
        accumulated_warnings.extend(attempt.warnings);
        if let Some(candidate) = attempt.candidate {
            candidates.push(candidate);
        }
    }

    if candidates.is_empty() {
        return fallback_unknown_result(snapshot, accumulated_warnings);
    }

    candidates.sort_by(compare_detection_results);
    let mut selected = candidates.remove(0);

    if let Some(runner_up) = candidates.first() {
        let selected_rank = specificity_rank(&selected.detected_type);
        let runner_up_rank = specificity_rank(&runner_up.detected_type);
        let confidence_delta = (selected.confidence - runner_up.confidence).abs();

        if selected_rank == runner_up_rank && confidence_delta < 0.10 {
            accumulated_warnings.push(DetectionWarning {
                code: "ambiguous-detection".into(),
                message: format!(
          "Detection is close between '{}' and '{}'; review the proposed command before saving",
          detected_type_label(&selected.detected_type),
          detected_type_label(&runner_up.detected_type)
        ),
                source: None,
            });
        }
    }

    selected.warnings.extend(accumulated_warnings);
    selected.confidence = selected.confidence.clamp(0.0, 1.0);
    selected.command_validation = validate_command(
        &selected.path,
        selected.working_dir.as_deref(),
        selected.executable.as_deref(),
        &selected.args,
    );
    selected.command_validation.command_preview = selected.command_preview.clone();
    if selected.command.is_none() {
        selected.command_validation.is_runnable = false;
        if !selected
            .command_validation
            .issues
            .iter()
            .any(|issue| issue == "No runnable start command was detected.")
        {
            selected
                .command_validation
                .issues
                .push("No runnable start command was detected.".into());
        }
    }
    if !selected.command_validation.is_runnable {
        selected.warnings.push(DetectionWarning {
            code: "command-validation-failed".into(),
            message:
                "The proposed command could not be validated locally. Review it before saving."
                    .into(),
            source: None,
        });
    }
    selected
        .evidence
        .sort_by(|left, right| right.weight.total_cmp(&left.weight));

    selected
}

fn fallback_unknown_result(
    snapshot: &ProjectScanSnapshot,
    mut warnings: Vec<DetectionWarning>,
) -> DetectionResult {
    if snapshot.files.is_empty() {
        warnings.push(DetectionWarning {
            code: "empty-directory".into(),
            message: "Selected folder is empty and does not contain detectable project markers"
                .into(),
            source: None,
        });
    } else {
        warnings.push(DetectionWarning {
            code: "no-detection-match".into(),
            message: "No detection rule matched this folder. Manual review is required.".into(),
            source: None,
        });
    }

    DetectionResult {
        detected_type: DetectedProjectType::Unknown,
        display_name: snapshot.root_directory_name(),
        path: snapshot.root_path.clone(),
        working_dir: Some(snapshot.root_path.clone()),
        package_manager: None,
        executable: None,
        command: None,
        args: Vec::new(),
        command_preview: "Manual review required".into(),
        command_validation: validate_command(
            &snapshot.root_path,
            Some(&snapshot.root_path),
            None,
            &[],
        ),
        confidence: 0.20,
        evidence: vec![DetectionEvidence {
            kind: DetectionEvidenceKind::Fallback,
            source: "scanner".into(),
            detail: "No specific project rule could be selected".into(),
            weight: 0.10,
        }],
        warnings,
    }
}

fn compare_detection_results(
    left: &DetectionResult,
    right: &DetectionResult,
) -> std::cmp::Ordering {
    specificity_rank(&right.detected_type)
        .cmp(&specificity_rank(&left.detected_type))
        .then_with(|| right.confidence.total_cmp(&left.confidence))
        .then_with(|| left.display_name.cmp(&right.display_name))
}

fn specificity_rank(detected_type: &DetectedProjectType) -> usize {
    match detected_type {
        DetectedProjectType::SpringBootMaven | DetectedProjectType::SpringBootGradle => 6,
        DetectedProjectType::ReactVite
        | DetectedProjectType::NextJs
        | DetectedProjectType::Express => 5,
        DetectedProjectType::Vite | DetectedProjectType::JavaJar => 4,
        DetectedProjectType::JavaMaven | DetectedProjectType::JavaGradle => 3,
        DetectedProjectType::NodeGeneric => 2,
        DetectedProjectType::Custom | DetectedProjectType::Unknown => 1,
    }
}

fn detected_type_label(detected_type: &DetectedProjectType) -> &'static str {
    match detected_type {
        DetectedProjectType::JavaMaven => "Java Maven",
        DetectedProjectType::JavaGradle => "Java Gradle",
        DetectedProjectType::SpringBootMaven => "Spring Boot Maven",
        DetectedProjectType::SpringBootGradle => "Spring Boot Gradle",
        DetectedProjectType::JavaJar => "Java JAR",
        DetectedProjectType::NodeGeneric => "Node",
        DetectedProjectType::Vite => "Vite",
        DetectedProjectType::ReactVite => "React/Vite",
        DetectedProjectType::NextJs => "Next.js",
        DetectedProjectType::Express => "Express",
        DetectedProjectType::Custom => "Custom",
        DetectedProjectType::Unknown => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::analyze_project_folder;
    use crate::models::DetectedProjectType;

    #[test]
    fn falls_back_to_unknown_for_empty_folder() {
        let directory = TempProjectDir::new("analysis-empty");

        let result = analyze_project_folder(directory.path()).expect("analysis should succeed");

        assert_eq!(result.detected_type, DetectedProjectType::Unknown);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.code == "empty-directory"));
    }

    #[test]
    fn returns_react_vite_for_node_fixture() {
        let result = analyze_project_folder(fixture_path("scanner/node-app"))
            .expect("node fixture analysis should succeed");

        assert_eq!(result.detected_type, DetectedProjectType::ReactVite);
        assert_eq!(
            result
                .package_manager
                .as_ref()
                .map(|value| format!("{value:?}")),
            Some("Pnpm".into())
        );
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-fixtures")
            .join(relative)
    }

    struct TempProjectDir {
        path: PathBuf,
    }

    impl TempProjectDir {
        fn new(test_name: &str) -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos();
            let path = env::temp_dir()
                .join("centralita-analysis-tests")
                .join(format!("{test_name}-{suffix}"));
            fs::create_dir_all(&path).expect("temp project directory should exist");

            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TempProjectDir {
        fn drop(&mut self) {
            if self.path.exists() {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
    }
}
