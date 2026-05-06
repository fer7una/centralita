use std::{ffi::OsStr, fs, io, path::Path};

use crate::models::DetectionWarning;

const DEFAULT_IGNORED_DIRECTORIES: &[&str] = &[
    "node_modules",
    "target",
    "build",
    ".git",
    ".idea",
    "dist",
    ".next",
];
const MAX_SCAN_DEPTH: usize = 6;
const MAX_SCANNED_ENTRIES: usize = 2048;
const MAX_TEXT_FILE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectScanSnapshot {
    pub root_path: String,
    pub files: Vec<ScannedFile>,
    pub warnings: Vec<DetectionWarning>,
    pub skipped_directories: Vec<String>,
}

impl ProjectScanSnapshot {
    pub fn has_file(&self, relative_path: &str) -> bool {
        self.files
            .iter()
            .any(|file| file.relative_path == relative_path)
    }

    pub fn file(&self, relative_path: &str) -> Option<&ScannedFile> {
        self.files
            .iter()
            .find(|file| file.relative_path == relative_path)
    }

    pub fn file_content(&self, relative_path: &str) -> Option<&str> {
        self.file(relative_path)
            .and_then(|file| file.content.as_deref())
    }

    pub fn files_named(&self, file_name: &str) -> Vec<&ScannedFile> {
        self.files
            .iter()
            .filter(|file| file.file_name == file_name)
            .collect()
    }

    pub fn root_directory_name(&self) -> String {
        Path::new(&self.root_path)
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("project")
            .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    pub relative_path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub content: Option<String>,
}

#[derive(Debug)]
pub enum ProjectScanError {
    Io(io::Error),
    InvalidRoot(String),
    NotADirectory(String),
}

impl std::fmt::Display for ProjectScanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::InvalidRoot(path) => {
                write!(formatter, "Project folder '{}' does not exist", path)
            }
            Self::NotADirectory(path) => {
                write!(formatter, "Project folder '{}' is not a directory", path)
            }
        }
    }
}

impl std::error::Error for ProjectScanError {}

impl From<io::Error> for ProjectScanError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn scan_project_folder(
    root: impl AsRef<Path>,
) -> Result<ProjectScanSnapshot, ProjectScanError> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(ProjectScanError::InvalidRoot(root.display().to_string()));
    }

    let metadata = fs::metadata(root)?;
    if !metadata.is_dir() {
        return Err(ProjectScanError::NotADirectory(root.display().to_string()));
    }

    let canonical_root = root.canonicalize()?;
    let mut snapshot = ProjectScanSnapshot {
        root_path: canonical_root.display().to_string(),
        files: Vec::new(),
        warnings: Vec::new(),
        skipped_directories: Vec::new(),
    };
    let mut scanned_entries = 0usize;

    visit_directory(
        &canonical_root,
        &canonical_root,
        0,
        &mut scanned_entries,
        &mut snapshot,
    )?;

    snapshot
        .files
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    snapshot.skipped_directories.sort();

    Ok(snapshot)
}

fn visit_directory(
    root: &Path,
    current_directory: &Path,
    depth: usize,
    scanned_entries: &mut usize,
    snapshot: &mut ProjectScanSnapshot,
) -> Result<(), ProjectScanError> {
    let mut entries = fs::read_dir(current_directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if *scanned_entries >= MAX_SCANNED_ENTRIES {
            snapshot.warnings.push(scan_warning(
                "scan-entry-limit",
                format!(
                    "Scanner stopped after reaching {} filesystem entries",
                    MAX_SCANNED_ENTRIES
                ),
                Some(relative_path(root, current_directory)),
            ));
            return Ok(());
        }

        *scanned_entries += 1;
        let path = entry.path();
        let relative = relative_path(root, &path);
        let file_type = entry.file_type()?;

        if file_type.is_symlink() {
            snapshot.warnings.push(scan_warning(
                "symlink-skipped",
                "Symlink skipped during project scan".into(),
                Some(relative),
            ));
            continue;
        }

        if file_type.is_dir() {
            let directory_name = entry.file_name().to_string_lossy().to_string();
            if is_ignored_directory(&directory_name) {
                snapshot.skipped_directories.push(relative);
                continue;
            }

            if depth >= MAX_SCAN_DEPTH {
                snapshot.warnings.push(scan_warning(
                    "scan-depth-limit",
                    format!("Scanner stopped descending after depth {}", MAX_SCAN_DEPTH),
                    Some(relative),
                ));
                continue;
            }

            visit_directory(root, &path, depth + 1, scanned_entries, snapshot)?;
            continue;
        }

        if !should_capture_file(&path, &relative) {
            continue;
        }

        let scanned_file = scan_file(&path, relative, snapshot)?;
        snapshot.files.push(scanned_file);
    }

    Ok(())
}

fn scan_file(
    path: &Path,
    relative_path: String,
    snapshot: &mut ProjectScanSnapshot,
) -> Result<ScannedFile, ProjectScanError> {
    let metadata = fs::metadata(path)?;
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_string();
    let size_bytes = metadata.len();
    let mut content = None;

    if should_read_as_text(path) {
        if size_bytes > MAX_TEXT_FILE_BYTES {
            snapshot.warnings.push(scan_warning(
                "large-file-skipped",
                format!(
                    "Skipped reading '{}' because it exceeds {} bytes",
                    relative_path, MAX_TEXT_FILE_BYTES
                ),
                Some(relative_path.clone()),
            ));
        } else {
            match fs::read(path) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(text) => {
                        content = Some(text);
                    }
                    Err(_) => {
                        snapshot.warnings.push(scan_warning(
                            "non-utf8-file",
                            format!(
                                "Skipped reading '{}' because it is not valid UTF-8",
                                relative_path
                            ),
                            Some(relative_path.clone()),
                        ));
                    }
                },
                Err(error) => {
                    snapshot.warnings.push(scan_warning(
                        "file-read-failed",
                        format!("Could not read '{}': {}", relative_path, error),
                        Some(relative_path.clone()),
                    ));
                }
            }
        }
    }

    Ok(ScannedFile {
        relative_path,
        file_name,
        size_bytes,
        content,
    })
}

fn should_capture_file(path: &Path, relative_path: &str) -> bool {
    let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    matches!(
        file_name,
        "package.json"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "vite.config.ts"
            | "vite.config.js"
            | "next.config.js"
            | "next.config.mjs"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "settings.gradle"
            | "settings.gradle.kts"
            | "mvnw"
            | "mvnw.cmd"
            | "gradlew"
            | "gradlew.bat"
            | "server.js"
            | "server.ts"
            | "app.js"
            | "app.ts"
            | "index.js"
            | "index.ts"
            | "main.js"
            | "main.ts"
    ) || matches!(
        relative_path,
        "src/main/resources/application.properties" | "src/main/resources/application.yml"
    ) || path.extension().and_then(OsStr::to_str) == Some("jar")
}

fn should_read_as_text(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str) != Some("jar")
}

fn is_ignored_directory(directory_name: &str) -> bool {
    DEFAULT_IGNORED_DIRECTORIES.contains(&directory_name)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn scan_warning(code: &str, message: String, source: Option<String>) -> DetectionWarning {
    DetectionWarning {
        code: code.into(),
        message,
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{scan_project_folder, ProjectScanError, MAX_TEXT_FILE_BYTES};

    #[test]
    fn scans_node_fixture_and_ignores_heavy_directories() {
        let fixture = fixture_path("node-app");

        let snapshot = scan_project_folder(&fixture).expect("node fixture should scan");

        let scanned_paths = snapshot
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert!(scanned_paths.contains(&"package.json"));
        assert!(scanned_paths.contains(&"pnpm-lock.yaml"));
        assert!(scanned_paths.contains(&"vite.config.ts"));
        assert!(scanned_paths.contains(&"src/main.ts"));
        assert!(!scanned_paths.contains(&"node_modules/left-pad/package.json"));
        assert!(!scanned_paths.contains(&"dist/assets/index.js"));
        assert!(snapshot
            .skipped_directories
            .contains(&"node_modules".into()));
        assert!(snapshot.skipped_directories.contains(&"dist".into()));
    }

    #[test]
    fn scans_java_fixture_with_wrapper_and_resources() {
        let fixture = fixture_path("java-app");

        let snapshot = scan_project_folder(&fixture).expect("java fixture should scan");

        let scanned_paths = snapshot
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert!(scanned_paths.contains(&"pom.xml"));
        assert!(scanned_paths.contains(&"mvnw"));
        assert!(scanned_paths.contains(&"src/main/resources/application.properties"));
        assert!(scanned_paths.contains(&"app.jar"));
        assert!(!scanned_paths.contains(&"target/app.jar"));
        assert!(snapshot.skipped_directories.contains(&"target".into()));
    }

    #[test]
    fn returns_empty_snapshot_for_empty_directory() {
        let directory = TempProjectDir::new("empty-scan");

        let snapshot = scan_project_folder(directory.path()).expect("empty directory should scan");

        assert!(snapshot.files.is_empty());
        assert!(snapshot.warnings.is_empty());
    }

    #[test]
    fn warns_when_large_file_exceeds_read_limit() {
        let directory = TempProjectDir::new("large-file-scan");
        let path = directory.path().join("package.json");
        let large_payload = "a".repeat(MAX_TEXT_FILE_BYTES as usize + 1);
        fs::write(&path, large_payload).expect("large file should be created");

        let snapshot = scan_project_folder(directory.path()).expect("large directory should scan");
        let package_json = snapshot
            .files
            .iter()
            .find(|file| file.relative_path == "package.json")
            .expect("package.json should be included");

        assert!(package_json.content.is_none());
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.code == "large-file-skipped"));
    }

    #[test]
    fn rejects_missing_directory() {
        let error = scan_project_folder(PathBuf::from(r"C:\centralita\missing-project"))
            .expect_err("missing directory should fail");

        assert!(matches!(error, ProjectScanError::InvalidRoot(_)));
    }

    #[test]
    fn rejects_file_root() {
        let directory = TempProjectDir::new("file-root-scan");
        let file = directory.path().join("package.json");
        fs::write(&file, "{}").expect("fixture file should be created");

        let error = scan_project_folder(&file).expect_err("file root should fail");

        assert!(matches!(error, ProjectScanError::NotADirectory(_)));
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-fixtures")
            .join("scanner")
            .join(name)
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
                .join("centralita-scan-tests")
                .join(format!("{test_name}-{suffix}"));
            fs::create_dir_all(&path).expect("temp project directory should exist");

            Self { path }
        }

        fn path(&self) -> &Path {
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
