use std::path::Path;

use crate::models::{
    DetectedProjectType, DetectionEvidence, DetectionEvidenceKind, DetectionResult,
    DetectionWarning, ProjectPackageManager,
};

use super::{analyze::DetectionAttempt, validate_command, ProjectScanSnapshot};

pub(super) fn analyze(snapshot: &ProjectScanSnapshot) -> DetectionAttempt {
    let pom_content = snapshot.file_content("pom.xml");
    let gradle_content = snapshot
        .file_content("build.gradle")
        .or_else(|| snapshot.file_content("build.gradle.kts"));
    let root_jar = root_jar(snapshot);

    if pom_content.is_none() && gradle_content.is_none() && root_jar.is_none() {
        return DetectionAttempt {
            candidate: None,
            warnings: Vec::new(),
        };
    }

    let mut candidates = Vec::new();
    let mut warnings = Vec::new();

    if let Some(pom_content) = pom_content {
        candidates.push(build_maven_result(snapshot, pom_content));
    }

    if let Some(gradle_content) = gradle_content {
        candidates.push(build_gradle_result(snapshot, gradle_content));
    }

    if let Some(root_jar) = root_jar {
        candidates.push(build_jar_result(snapshot, &root_jar));
    }

    if candidates.len() > 1 {
        warnings.push(DetectionWarning {
            code: "multiple-java-candidates".into(),
            message:
                "Multiple Java execution strategies were detected. Review the selected command."
                    .into(),
            source: None,
        });
    }

    candidates.sort_by(|left, right| {
        java_specificity_rank(&right.detected_type)
            .cmp(&java_specificity_rank(&left.detected_type))
            .then_with(|| right.confidence.total_cmp(&left.confidence))
    });

    DetectionAttempt {
        candidate: candidates.into_iter().next(),
        warnings,
    }
}

fn build_maven_result(snapshot: &ProjectScanSnapshot, pom_content: &str) -> DetectionResult {
    let is_spring_boot = is_spring_boot_maven(pom_content);
    let executable = if snapshot.has_file("mvnw.cmd") {
        Some("mvnw.cmd".into())
    } else if snapshot.has_file("mvnw") {
        Some("mvnw".into())
    } else {
        Some("mvn".into())
    };
    let mut evidence = vec![evidence_item(
        DetectionEvidenceKind::StructuralFile,
        "pom.xml",
        "Maven pom.xml file found",
        0.35,
    )];

    if snapshot.has_file("mvnw") || snapshot.has_file("mvnw.cmd") {
        evidence.push(evidence_item(
            DetectionEvidenceKind::Wrapper,
            if snapshot.has_file("mvnw.cmd") {
                "mvnw.cmd"
            } else {
                "mvnw"
            },
            "Local Maven wrapper found",
            0.15,
        ));
    }

    if is_spring_boot {
        evidence.push(evidence_item(
            DetectionEvidenceKind::Plugin,
            "pom.xml",
            "Spring Boot Maven markers found",
            0.25,
        ));
    }

    if snapshot.has_file("src/main/resources/application.properties")
        || snapshot.has_file("src/main/resources/application.yml")
    {
        evidence.push(evidence_item(
            DetectionEvidenceKind::Config,
            if snapshot.has_file("src/main/resources/application.properties") {
                "src/main/resources/application.properties"
            } else {
                "src/main/resources/application.yml"
            },
            "Application configuration file found",
            0.10,
        ));
    }

    let display_name = extract_xml_tag(pom_content, "artifactId")
        .unwrap_or_else(|| snapshot.root_directory_name());

    if is_spring_boot {
        let args = vec!["spring-boot:run".into()];
        let command = executable
            .as_ref()
            .map(|executable| format!("{} {}", executable, args[0]));
        let command_validation = validate_command(
            &snapshot.root_path,
            Some(&snapshot.root_path),
            executable.as_deref(),
            &args,
        );

        DetectionResult {
            detected_type: DetectedProjectType::SpringBootMaven,
            display_name,
            path: snapshot.root_path.clone(),
            working_dir: Some(snapshot.root_path.clone()),
            package_manager: Some(ProjectPackageManager::Maven),
            executable,
            command: command.clone(),
            args,
            command_preview: command.unwrap_or_else(|| "Manual review required".into()),
            command_validation,
            confidence: evidence
                .iter()
                .map(|item| item.weight)
                .sum::<f64>()
                .clamp(0.0, 1.0),
            evidence,
            warnings: Vec::new(),
        }
    } else {
        let command_validation = validate_command(
            &snapshot.root_path,
            Some(&snapshot.root_path),
            executable.as_deref(),
            &[],
        );

        DetectionResult {
            detected_type: DetectedProjectType::JavaMaven,
            display_name,
            path: snapshot.root_path.clone(),
            working_dir: Some(snapshot.root_path.clone()),
            package_manager: Some(ProjectPackageManager::Maven),
            executable,
            command: None,
            args: Vec::new(),
            command_preview: "Manual review required".into(),
            command_validation,
            confidence: evidence
                .iter()
                .map(|item| item.weight)
                .sum::<f64>()
                .clamp(0.0, 1.0),
            evidence,
            warnings: vec![DetectionWarning {
                code: "manual-command-required".into(),
                message: "Generic Maven project detected without a runnable development task."
                    .into(),
                source: Some("pom.xml".into()),
            }],
        }
    }
}

fn build_gradle_result(snapshot: &ProjectScanSnapshot, gradle_content: &str) -> DetectionResult {
    let is_spring_boot = is_spring_boot_gradle(gradle_content);
    let executable = if snapshot.has_file("gradlew.bat") {
        Some("gradlew.bat".into())
    } else if snapshot.has_file("gradlew") {
        Some("gradlew".into())
    } else {
        Some("gradle".into())
    };
    let mut evidence = vec![evidence_item(
        DetectionEvidenceKind::StructuralFile,
        if snapshot.has_file("build.gradle.kts") {
            "build.gradle.kts"
        } else {
            "build.gradle"
        },
        "Gradle build file found",
        0.35,
    )];

    if snapshot.has_file("gradlew") || snapshot.has_file("gradlew.bat") {
        evidence.push(evidence_item(
            DetectionEvidenceKind::Wrapper,
            if snapshot.has_file("gradlew.bat") {
                "gradlew.bat"
            } else {
                "gradlew"
            },
            "Local Gradle wrapper found",
            0.15,
        ));
    }

    if is_spring_boot {
        evidence.push(evidence_item(
            DetectionEvidenceKind::Plugin,
            if snapshot.has_file("build.gradle.kts") {
                "build.gradle.kts"
            } else {
                "build.gradle"
            },
            "Spring Boot Gradle markers found",
            0.25,
        ));
    }

    let display_name =
        extract_gradle_project_name(snapshot).unwrap_or_else(|| snapshot.root_directory_name());

    if is_spring_boot {
        let args = vec!["bootRun".into()];
        let command = executable
            .as_ref()
            .map(|executable| format!("{} {}", executable, args[0]));
        let command_validation = validate_command(
            &snapshot.root_path,
            Some(&snapshot.root_path),
            executable.as_deref(),
            &args,
        );

        DetectionResult {
            detected_type: DetectedProjectType::SpringBootGradle,
            display_name,
            path: snapshot.root_path.clone(),
            working_dir: Some(snapshot.root_path.clone()),
            package_manager: Some(ProjectPackageManager::Gradle),
            executable,
            command: command.clone(),
            args,
            command_preview: command.unwrap_or_else(|| "Manual review required".into()),
            command_validation,
            confidence: evidence
                .iter()
                .map(|item| item.weight)
                .sum::<f64>()
                .clamp(0.0, 1.0),
            evidence,
            warnings: Vec::new(),
        }
    } else {
        let command_validation = validate_command(
            &snapshot.root_path,
            Some(&snapshot.root_path),
            executable.as_deref(),
            &[],
        );

        DetectionResult {
            detected_type: DetectedProjectType::JavaGradle,
            display_name,
            path: snapshot.root_path.clone(),
            working_dir: Some(snapshot.root_path.clone()),
            package_manager: Some(ProjectPackageManager::Gradle),
            executable,
            command: None,
            args: Vec::new(),
            command_preview: "Manual review required".into(),
            command_validation,
            confidence: evidence
                .iter()
                .map(|item| item.weight)
                .sum::<f64>()
                .clamp(0.0, 1.0),
            evidence,
            warnings: vec![DetectionWarning {
                code: "manual-command-required".into(),
                message: "Generic Gradle project detected without a runnable development task."
                    .into(),
                source: Some(
                    if snapshot.has_file("build.gradle.kts") {
                        "build.gradle.kts"
                    } else {
                        "build.gradle"
                    }
                    .into(),
                ),
            }],
        }
    }
}

fn build_jar_result(snapshot: &ProjectScanSnapshot, jar_relative_path: &str) -> DetectionResult {
    let jar_path = Path::new(&snapshot.root_path).join(jar_relative_path);
    let command = format!("java -jar {}", jar_path.display());
    let args = vec!["-jar".into(), jar_path.display().to_string()];
    let command_validation = validate_command(
        &snapshot.root_path,
        Some(&snapshot.root_path),
        Some("java"),
        &args,
    );

    DetectionResult {
        detected_type: DetectedProjectType::JavaJar,
        display_name: jar_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_else(|| jar_relative_path.trim_end_matches(".jar"))
            .to_string(),
        path: snapshot.root_path.clone(),
        working_dir: Some(snapshot.root_path.clone()),
        package_manager: None,
        executable: Some("java".into()),
        command: Some(command.clone()),
        args,
        command_preview: command,
        command_validation,
        confidence: 0.35,
        evidence: vec![evidence_item(
            DetectionEvidenceKind::Artifact,
            jar_relative_path,
            "Runnable JAR artifact found",
            0.35,
        )],
        warnings: Vec::new(),
    }
}

fn root_jar(snapshot: &ProjectScanSnapshot) -> Option<String> {
    snapshot
        .files
        .iter()
        .find(|file| file.relative_path.ends_with(".jar") && !file.relative_path.contains('/'))
        .map(|file| file.relative_path.clone())
}

fn is_spring_boot_maven(content: &str) -> bool {
    content.contains("spring-boot-starter")
        || content.contains("spring-boot-maven-plugin")
        || content.contains("spring-boot-starter-parent")
}

fn is_spring_boot_gradle(content: &str) -> bool {
    content.contains("org.springframework.boot") || content.contains("spring-boot-starter")
}

fn extract_xml_tag(content: &str, tag_name: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag_name);
    let close_tag = format!("</{}>", tag_name);
    let start = content.find(&open_tag)? + open_tag.len();
    let end = content[start..].find(&close_tag)? + start;

    Some(content[start..end].trim().to_string())
}

fn extract_gradle_project_name(snapshot: &ProjectScanSnapshot) -> Option<String> {
    let settings = snapshot
        .file_content("settings.gradle")
        .or_else(|| snapshot.file_content("settings.gradle.kts"))?;
    let needle = if settings.contains("rootProject.name") {
        "rootProject.name"
    } else {
        return None;
    };
    let suffix = settings.split_once(needle)?.1;
    let quoted = suffix
        .split(['"', '\''])
        .filter(|segment| !segment.trim().is_empty())
        .nth(1)?;

    Some(quoted.trim().to_string())
}

fn evidence_item(
    kind: DetectionEvidenceKind,
    source: &str,
    detail: &str,
    weight: f64,
) -> DetectionEvidence {
    DetectionEvidence {
        kind,
        source: source.into(),
        detail: detail.into(),
        weight,
    }
}

fn java_specificity_rank(detected_type: &DetectedProjectType) -> usize {
    match detected_type {
        DetectedProjectType::SpringBootMaven | DetectedProjectType::SpringBootGradle => 4,
        DetectedProjectType::JavaMaven | DetectedProjectType::JavaGradle => 3,
        DetectedProjectType::JavaJar => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use crate::{detection::scan_project_folder, models::DetectedProjectType};

    use super::analyze;

    #[test]
    fn detects_spring_boot_maven_projects() {
        let snapshot = scan_project_folder(fixture_path("spring-maven"))
            .expect("spring maven fixture should scan");
        let result = analyze(&snapshot)
            .candidate
            .expect("spring maven candidate should exist");

        assert_eq!(result.detected_type, DetectedProjectType::SpringBootMaven);
        assert_eq!(result.command.as_deref(), Some("mvnw.cmd spring-boot:run"));
    }

    #[test]
    fn detects_spring_boot_gradle_projects() {
        let snapshot = scan_project_folder(fixture_path("spring-gradle"))
            .expect("spring gradle fixture should scan");
        let result = analyze(&snapshot)
            .candidate
            .expect("spring gradle candidate should exist");

        assert_eq!(result.detected_type, DetectedProjectType::SpringBootGradle);
        assert_eq!(result.command.as_deref(), Some("gradlew.bat bootRun"));
    }

    #[test]
    fn falls_back_to_java_jar_when_no_better_java_runner_exists() {
        let snapshot =
            scan_project_folder(fixture_path("jar-only")).expect("jar fixture should scan");
        let result = analyze(&snapshot)
            .candidate
            .expect("jar candidate should exist");

        assert_eq!(result.detected_type, DetectedProjectType::JavaJar);
        assert_eq!(result.executable.as_deref(), Some("java"));
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-fixtures")
            .join("analyzer")
            .join(name)
    }
}
