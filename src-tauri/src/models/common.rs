use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type EntityId = String;
pub type IsoDateTime = String;
pub type CommandArgs = Vec<String>;
pub type EnvironmentVariables = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DetectedProjectType {
    JavaMaven,
    JavaGradle,
    SpringBootMaven,
    SpringBootGradle,
    JavaJar,
    NodeGeneric,
    Vite,
    ReactVite,
    NextJs,
    Express,
    Custom,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectPackageManager {
    Npm,
    Pnpm,
    Yarn,
    Maven,
    Gradle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DetectionEvidenceKind {
    StructuralFile,
    Manifest,
    Config,
    Dependency,
    Plugin,
    Script,
    Lockfile,
    Wrapper,
    Artifact,
    EntryPoint,
    Workspace,
    Fallback,
}
