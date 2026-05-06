use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::models::CommandValidation;

pub fn validate_command(
    path: &str,
    working_dir: Option<&str>,
    executable: Option<&str>,
    args: &[String],
) -> CommandValidation {
    let command_preview = format_command_preview(executable, args);
    let mut issues = Vec::new();
    let resolved_working_dir = working_dir.unwrap_or(path).trim();

    if resolved_working_dir.is_empty() {
        issues.push("El directorio de trabajo está vacío.".into());
    }

    let working_dir_path = Path::new(resolved_working_dir);
    if !resolved_working_dir.is_empty() && !working_dir_path.is_dir() {
        issues.push(format!(
            "El directorio de trabajo no existe o no es una carpeta: {}",
            resolved_working_dir
        ));
    }

    let normalized_executable = executable.map(str::trim).filter(|value| !value.is_empty());
    let resolved_executable = match (normalized_executable, issues.is_empty()) {
        (Some(executable), true) => resolve_executable(executable, working_dir_path),
        _ => None,
    };

    if normalized_executable.is_none() {
        issues.push("No hay un ejecutable configurado.".into());
    } else if resolved_executable.is_none() {
        issues.push(format!(
            "No se encontro el ejecutable '{}' ni en la carpeta del proyecto ni en PATH.",
            normalized_executable.unwrap_or_default()
        ));
    }

    CommandValidation {
        is_runnable: issues.is_empty(),
        command_preview,
        resolved_executable: resolved_executable.map(|path| path.display().to_string()),
        issues,
    }
}

pub fn format_command_preview(executable: Option<&str>, args: &[String]) -> String {
    let Some(executable) = executable.map(str::trim).filter(|value| !value.is_empty()) else {
        return "Manual review required".into();
    };

    let serialized_args = args.iter().map(|value| {
        if value.contains(' ') {
            format!("\"{}\"", value)
        } else {
            value.clone()
        }
    });

    std::iter::once(executable.to_string())
        .chain(serialized_args)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn resolve_executable(executable: &str, working_dir: &Path) -> Option<PathBuf> {
    let candidate_path = Path::new(executable);
    if candidate_path.is_absolute() || executable.contains('\\') || executable.contains('/') {
        let anchored_path = if candidate_path.is_absolute() {
            candidate_path.to_path_buf()
        } else {
            working_dir.join(candidate_path)
        };

        return resolve_path_with_extensions(&anchored_path);
    }

    let local_candidate = working_dir.join(executable);
    if let Some(resolved) = resolve_path_with_extensions(&local_candidate) {
        return Some(resolved);
    }

    let path_entries = env::var_os("PATH")?;
    env::split_paths(&path_entries)
        .find_map(|directory| resolve_path_with_extensions(&directory.join(executable)))
}

fn resolve_path_with_extensions(candidate: &Path) -> Option<PathBuf> {
    if candidate.extension().is_some() {
        return has_file(candidate).then(|| canonical_or_original(candidate));
    }

    if let Some(resolved) = executable_extensions()
        .iter()
        .map(|extension| append_extension(candidate, extension))
        .find(|path| has_file(path))
        .map(|path| canonical_or_original(&path))
    {
        return Some(resolved);
    }

    has_file(candidate).then(|| canonical_or_original(candidate))
}

fn append_extension(candidate: &Path, extension: &str) -> PathBuf {
    let mut path = OsString::from(candidate.as_os_str());
    path.push(extension);
    PathBuf::from(path)
}

fn has_file(candidate: &Path) -> bool {
    candidate.is_file()
}

fn canonical_or_original(candidate: &Path) -> PathBuf {
    candidate
        .canonicalize()
        .unwrap_or_else(|_| candidate.to_path_buf())
}

fn executable_extensions() -> Vec<String> {
    if cfg!(windows) {
        env::var("PATHEXT")
            .ok()
            .map(|value| {
                value
                    .split(';')
                    .filter_map(|extension| {
                        let extension = extension.trim();
                        if extension.is_empty() {
                            None
                        } else if extension.starts_with('.') {
                            Some(extension.to_string())
                        } else {
                            Some(format!(".{extension}"))
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|extensions| !extensions.is_empty())
            .unwrap_or_else(|| vec![".com".into(), ".exe".into(), ".bat".into(), ".cmd".into()])
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{format_command_preview, validate_command};

    #[test]
    fn validates_local_wrapper_commands() {
        let directory = TempProjectDir::new("command-validation-wrapper");
        let wrapper_path = directory.path().join("mvnw.cmd");
        fs::write(&wrapper_path, "@echo off\r\n").expect("wrapper should exist");

        let validation = validate_command(
            &directory.path().display().to_string(),
            Some(&directory.path().display().to_string()),
            Some("mvnw.cmd"),
            &["spring-boot:run".into()],
        );

        assert!(validation.is_runnable);
        assert_eq!(validation.command_preview, "mvnw.cmd spring-boot:run");
        assert!(validation
            .resolved_executable
            .as_deref()
            .is_some_and(|path| path.ends_with("mvnw.cmd")));
    }

    #[test]
    fn reports_missing_executables() {
        let directory = TempProjectDir::new("command-validation-missing");
        let validation = validate_command(
            &directory.path().display().to_string(),
            Some(&directory.path().display().to_string()),
            Some("missing-command"),
            &["dev".into()],
        );

        assert!(!validation.is_runnable);
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("missing-command")));
    }

    #[cfg(windows)]
    #[test]
    fn prefers_windows_wrapper_extensions_over_extensionless_scripts() {
        let directory = TempProjectDir::new("command-validation-prefers-cmd");
        fs::write(directory.path().join("npm"), "#!/bin/sh\nexit 0\n")
            .expect("extensionless script should exist");
        fs::write(
            directory.path().join("npm.cmd"),
            "@echo off\r\nexit /b 0\r\n",
        )
        .expect("cmd wrapper should exist");

        let validation = validate_command(
            &directory.path().display().to_string(),
            Some(&directory.path().display().to_string()),
            Some("npm"),
            &["start".into()],
        );

        assert!(validation.is_runnable);
        assert!(validation
            .resolved_executable
            .as_deref()
            .is_some_and(|path| path.ends_with("npm.cmd")));
    }

    #[test]
    fn formats_preview_with_quoted_space_arguments() {
        let preview = format_command_preview(
            Some("npm"),
            &["run".into(), "dev".into(), "--host 0.0.0.0".into()],
        );

        assert_eq!(preview, "npm run dev \"--host 0.0.0.0\"");
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
                .join("centralita-command-validation")
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
