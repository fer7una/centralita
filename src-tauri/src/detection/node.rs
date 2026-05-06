use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::models::{
    DetectedProjectType, DetectionEvidence, DetectionEvidenceKind, DetectionResult,
    DetectionWarning, ProjectPackageManager,
};

use super::{analyze::DetectionAttempt, validate_command, ProjectScanSnapshot};

pub(super) fn analyze(snapshot: &ProjectScanSnapshot) -> DetectionAttempt {
    let has_package_json = snapshot.has_file("package.json");
    let has_vite_config =
        snapshot.has_file("vite.config.ts") || snapshot.has_file("vite.config.js");
    let has_next_config =
        snapshot.has_file("next.config.js") || snapshot.has_file("next.config.mjs");

    if !has_package_json && !has_vite_config && !has_next_config {
        return DetectionAttempt {
            candidate: None,
            warnings: Vec::new(),
        };
    }

    let mut warnings = Vec::new();
    let manifest = match parse_package_manifest(snapshot) {
        Ok(manifest) => manifest,
        Err(warning) => {
            warnings.push(warning);
            None
        }
    };

    let package_manager = infer_package_manager(snapshot);
    let display_name = manifest
        .as_ref()
        .and_then(|manifest| manifest.name.clone())
        .unwrap_or_else(|| snapshot.root_directory_name());
    let package_json_count = snapshot.files_named("package.json").len();
    if package_json_count > 1 {
        warnings.push(DetectionWarning {
            code: "simple-monorepo".into(),
            message:
                "Multiple package.json files were found. Review the selected folder before saving."
                    .into(),
            source: None,
        });
    }

    let has_next_dependency = manifest
        .as_ref()
        .is_some_and(|manifest| manifest.has_dependency("next"));
    let has_vite_dependency = manifest
        .as_ref()
        .is_some_and(|manifest| manifest.has_dependency("vite"));
    let has_react_dependency = manifest
        .as_ref()
        .is_some_and(|manifest| manifest.has_dependency("react"));
    let has_react_plugin = manifest.as_ref().is_some_and(|manifest| {
        manifest.has_dependency("@vitejs/plugin-react")
            || manifest.has_dependency("@vitejs/plugin-react-swc")
    }) || snapshot
        .file_content("vite.config.ts")
        .or_else(|| snapshot.file_content("vite.config.js"))
        .is_some_and(|content| content.contains("plugin-react") || content.contains("react("));
    let has_express_dependency = manifest
        .as_ref()
        .is_some_and(|manifest| manifest.has_dependency("express"));
    let has_entrypoint = has_entrypoint(snapshot);

    let detected_type = if has_next_config || has_next_dependency {
        Some(DetectedProjectType::NextJs)
    } else if (has_vite_config || has_vite_dependency) && has_react_dependency {
        Some(DetectedProjectType::ReactVite)
    } else if has_express_dependency && has_entrypoint {
        Some(DetectedProjectType::Express)
    } else if has_vite_config || has_vite_dependency {
        Some(DetectedProjectType::Vite)
    } else if manifest.is_some() {
        Some(DetectedProjectType::NodeGeneric)
    } else {
        None
    };

    let Some(detected_type) = detected_type else {
        return DetectionAttempt {
            candidate: None,
            warnings,
        };
    };

    let mut evidence = Vec::new();
    if manifest.is_some() {
        evidence.push(evidence_item(
            DetectionEvidenceKind::Manifest,
            "package.json",
            "package.json manifest found",
            0.35,
        ));
    }

    match detected_type {
        DetectedProjectType::NextJs => {
            if has_next_config {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::Config,
                    if snapshot.has_file("next.config.js") {
                        "next.config.js"
                    } else {
                        "next.config.mjs"
                    },
                    "Next.js config file found",
                    0.35,
                ));
            }
            if has_next_dependency {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::Dependency,
                    "package.json",
                    "next dependency found",
                    0.25,
                ));
            }
        }
        DetectedProjectType::ReactVite => {
            if has_vite_config {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::Config,
                    if snapshot.has_file("vite.config.ts") {
                        "vite.config.ts"
                    } else {
                        "vite.config.js"
                    },
                    "Vite config file found",
                    0.35,
                ));
            } else if has_vite_dependency {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::Dependency,
                    "package.json",
                    "vite dependency found",
                    0.25,
                ));
            }

            evidence.push(evidence_item(
                DetectionEvidenceKind::Dependency,
                "package.json",
                "react dependency found",
                0.25,
            ));

            if has_react_plugin {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::Plugin,
                    "package.json",
                    "React plugin for Vite found",
                    0.25,
                ));
            }
        }
        DetectedProjectType::Express => {
            evidence.push(evidence_item(
                DetectionEvidenceKind::Dependency,
                "package.json",
                "express dependency found",
                0.25,
            ));
            if let Some(entrypoint) = first_entrypoint(snapshot) {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::EntryPoint,
                    &entrypoint,
                    "Node entrypoint file found",
                    0.10,
                ));
            }
        }
        DetectedProjectType::Vite => {
            if has_vite_config {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::Config,
                    if snapshot.has_file("vite.config.ts") {
                        "vite.config.ts"
                    } else {
                        "vite.config.js"
                    },
                    "Vite config file found",
                    0.35,
                ));
            } else if has_vite_dependency {
                evidence.push(evidence_item(
                    DetectionEvidenceKind::Dependency,
                    "package.json",
                    "vite dependency found",
                    0.25,
                ));
            }
        }
        DetectedProjectType::NodeGeneric => {}
        _ => {}
    }

    if let Some(package_manager) = package_manager.as_ref() {
        let (source, detail) = match package_manager {
            ProjectPackageManager::Pnpm => ("pnpm-lock.yaml", "pnpm lockfile found"),
            ProjectPackageManager::Yarn => ("yarn.lock", "yarn lockfile found"),
            ProjectPackageManager::Npm => {
                if snapshot.has_file("package-lock.json") {
                    ("package-lock.json", "npm lockfile found")
                } else {
                    ("package.json", "npm selected as default package manager")
                }
            }
            ProjectPackageManager::Maven | ProjectPackageManager::Gradle => {
                ("package.json", "node package manager inferred")
            }
        };

        if source != "package.json" || snapshot.has_file("package-lock.json") {
            evidence.push(evidence_item(
                DetectionEvidenceKind::Lockfile,
                source,
                detail,
                0.10,
            ));
        }
    }

    let selected_script = manifest
        .as_ref()
        .and_then(|manifest| select_script_name(manifest, &detected_type));
    if let Some(script_name) = selected_script.as_ref() {
        evidence.push(evidence_item(
            DetectionEvidenceKind::Script,
            "package.json",
            &format!("{} script available", script_name),
            0.15,
        ));
    } else {
        warnings.push(DetectionWarning {
            code: "manual-command-required".into(),
            message: "No suitable package script was found. Review the command before saving."
                .into(),
            source: Some("package.json".into()),
        });
    }

    if !has_package_json && (has_vite_config || has_next_config) {
        warnings.push(DetectionWarning {
            code: "missing-package-json".into(),
            message: "Config files were found without package.json. Detection may be incomplete."
                .into(),
            source: None,
        });
    }

    let (executable, args, command) = selected_script
        .as_deref()
        .and_then(|script_name| build_package_command(package_manager.as_ref(), script_name))
        .unwrap_or((None, Vec::new(), None));
    let confidence = evidence
        .iter()
        .map(|item| item.weight)
        .sum::<f64>()
        .clamp(0.0, 1.0);
    let command_preview = command
        .clone()
        .unwrap_or_else(|| "Manual review required".into());
    let command_validation = validate_command(
        &snapshot.root_path,
        Some(&snapshot.root_path),
        executable.as_deref(),
        &args,
    );

    DetectionAttempt {
        candidate: Some(DetectionResult {
            detected_type,
            display_name,
            path: snapshot.root_path.clone(),
            working_dir: Some(snapshot.root_path.clone()),
            package_manager,
            executable,
            command,
            args,
            command_preview,
            command_validation,
            confidence,
            evidence,
            warnings: Vec::new(),
        }),
        warnings,
    }
}

#[derive(Debug, Clone)]
struct PackageManifest {
    name: Option<String>,
    scripts: HashMap<String, String>,
    dependencies: HashSet<String>,
}

impl PackageManifest {
    fn has_dependency(&self, dependency: &str) -> bool {
        self.dependencies.contains(dependency)
    }

    fn has_script(&self, script_name: &str) -> bool {
        self.scripts.contains_key(script_name)
    }
}

fn parse_package_manifest(
    snapshot: &ProjectScanSnapshot,
) -> Result<Option<PackageManifest>, DetectionWarning> {
    let Some(package_json) = snapshot.file_content("package.json") else {
        return Ok(None);
    };

    let value: Value = serde_json::from_str(package_json).map_err(|error| DetectionWarning {
        code: "invalid-package-json".into(),
        message: format!("package.json could not be parsed: {}", error),
        source: Some("package.json".into()),
    })?;

    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let scripts = read_string_map(value.get("scripts"));
    let mut dependencies = HashSet::new();
    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        dependencies.extend(read_string_map(value.get(section)).into_keys());
    }

    Ok(Some(PackageManifest {
        name,
        scripts,
        dependencies,
    }))
}

fn read_string_map(value: Option<&Value>) -> HashMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn infer_package_manager(snapshot: &ProjectScanSnapshot) -> Option<ProjectPackageManager> {
    if snapshot.has_file("pnpm-lock.yaml") {
        Some(ProjectPackageManager::Pnpm)
    } else if snapshot.has_file("yarn.lock") {
        Some(ProjectPackageManager::Yarn)
    } else if snapshot.has_file("package-lock.json") || snapshot.has_file("package.json") {
        Some(ProjectPackageManager::Npm)
    } else {
        None
    }
}

fn select_script_name(
    manifest: &PackageManifest,
    detected_type: &DetectedProjectType,
) -> Option<String> {
    let priorities: &[&str] = match detected_type {
        DetectedProjectType::Vite
        | DetectedProjectType::ReactVite
        | DetectedProjectType::NextJs => &["dev"],
        DetectedProjectType::Express | DetectedProjectType::NodeGeneric => {
            &["dev", "start", "serve"]
        }
        _ => &[],
    };

    priorities
        .iter()
        .find(|script_name| manifest.has_script(script_name))
        .map(|script_name| (*script_name).to_string())
}

fn build_package_command(
    package_manager: Option<&ProjectPackageManager>,
    script_name: &str,
) -> Option<(Option<String>, Vec<String>, Option<String>)> {
    let package_manager = package_manager?;

    let (executable, args, command) = match package_manager {
        ProjectPackageManager::Npm => (
            Some("npm".into()),
            vec!["run".into(), script_name.into()],
            Some(format!("npm run {}", script_name)),
        ),
        ProjectPackageManager::Pnpm => (
            Some("pnpm".into()),
            vec![script_name.into()],
            Some(format!("pnpm {}", script_name)),
        ),
        ProjectPackageManager::Yarn => (
            Some("yarn".into()),
            vec![script_name.into()],
            Some(format!("yarn {}", script_name)),
        ),
        ProjectPackageManager::Maven | ProjectPackageManager::Gradle => return None,
    };

    Some((executable, args, command))
}

fn has_entrypoint(snapshot: &ProjectScanSnapshot) -> bool {
    first_entrypoint(snapshot).is_some()
}

fn first_entrypoint(snapshot: &ProjectScanSnapshot) -> Option<String> {
    [
        "server.js",
        "server.ts",
        "app.js",
        "app.ts",
        "index.js",
        "index.ts",
        "main.js",
        "main.ts",
    ]
    .into_iter()
    .find(|candidate| snapshot.has_file(candidate))
    .map(ToOwned::to_owned)
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

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use crate::{
        detection::scan_project_folder,
        models::{DetectedProjectType, ProjectPackageManager},
    };

    use super::analyze;

    #[test]
    fn detects_next_js_projects() {
        let snapshot =
            scan_project_folder(fixture_path("next-app")).expect("next fixture should scan");
        let result = analyze(&snapshot)
            .candidate
            .expect("next candidate should exist");

        assert_eq!(result.detected_type, DetectedProjectType::NextJs);
        assert_eq!(result.package_manager, Some(ProjectPackageManager::Yarn));
        assert_eq!(result.command.as_deref(), Some("yarn dev"));
    }

    #[test]
    fn detects_express_projects() {
        let snapshot =
            scan_project_folder(fixture_path("express-app")).expect("express fixture should scan");
        let result = analyze(&snapshot)
            .candidate
            .expect("express candidate should exist");

        assert_eq!(result.detected_type, DetectedProjectType::Express);
        assert_eq!(result.command.as_deref(), Some("npm run dev"));
    }

    #[test]
    fn warns_on_invalid_package_json() {
        let snapshot =
            scan_project_folder(fixture_path("invalid-package-json")).expect("fixture should scan");
        let result = analyze(&snapshot);

        assert!(result.candidate.is_none());
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.code == "invalid-package-json"));
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-fixtures")
            .join("analyzer")
            .join(name)
    }
}
