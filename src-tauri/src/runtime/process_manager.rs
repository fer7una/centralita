use std::{
    collections::HashMap,
    env,
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

use crate::{
    detection::resolve_executable,
    events::{
        emit_log_line, emit_process_error, emit_process_exited, emit_status_changed,
        noop_runtime_event_emitter, RuntimeEventEmitter,
    },
    models::{
        EntityId, ProcessRuntimeState, ProjectNode, RunRequest, RuntimeLogLine, RuntimeLogStream,
        RuntimeProcessErrorEvent, RuntimeProcessExitedEvent, RuntimeStatus, RuntimeStatusEvent,
    },
    persistence::{AppDatabase, FinalizeRunHistoryInput, ProjectRepository},
    runtime::{
        initial_process_state, stop_process_tree, HistoryRecorder, LogBuffer, RuntimeError,
        RuntimeResult,
    },
    utils::timestamps,
};

const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const DEFAULT_LOG_BUFFER_LIMIT: usize = 500;
const LOG_READ_BUFFER_SIZE: usize = 4096;

#[derive(Clone)]
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<EntityId, ManagedProcess>>>,
    event_emitter: RuntimeEventEmitter,
    history_recorder: HistoryRecorder,
    log_buffer_limit: usize,
}

#[derive(Debug)]
struct ManagedProcess {
    state: ProcessRuntimeState,
    child: Option<Arc<Mutex<Child>>>,
    history_entry_id: Option<EntityId>,
    logs: LogBuffer,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self::with_database_and_event_emitter(None, noop_runtime_event_emitter())
    }

    pub fn with_event_emitter(event_emitter: RuntimeEventEmitter) -> Self {
        Self::with_database_and_event_emitter(None, event_emitter)
    }

    pub fn with_database_and_event_emitter(
        database: Option<AppDatabase>,
        event_emitter: RuntimeEventEmitter,
    ) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            event_emitter: event_emitter.clone(),
            history_recorder: HistoryRecorder::new(database, event_emitter),
            log_buffer_limit: DEFAULT_LOG_BUFFER_LIMIT,
        }
    }

    #[cfg(test)]
    fn with_event_emitter_and_capacity(
        event_emitter: RuntimeEventEmitter,
        log_buffer_limit: usize,
    ) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            event_emitter: event_emitter.clone(),
            history_recorder: HistoryRecorder::new(None, event_emitter),
            log_buffer_limit,
        }
    }

    pub fn start_persisted_project(
        &self,
        database: AppDatabase,
        project_id: &EntityId,
    ) -> RuntimeResult<ProcessRuntimeState> {
        let repository = ProjectRepository::new(database);
        let project = repository
            .find_by_id(project_id)
            .map_err(|error| {
                RuntimeError::new(format!("Failed to load project '{project_id}': {error}"))
            })?
            .ok_or_else(|| RuntimeError::new(format!("Project '{project_id}' not found")))?;

        self.start_project(&project)
    }

    pub fn start_project(&self, project: &ProjectNode) -> RuntimeResult<ProcessRuntimeState> {
        let started_at = now_iso_or_fallback();
        let run_request = match self.build_run_request(project) {
            Ok(run_request) => run_request,
            Err(error) => {
                let failed_state = self.store_failed_state(
                    project,
                    &command_preview_from_project(project),
                    &error.to_string(),
                    &started_at,
                );
                return Err(RuntimeError::new(
                    failed_state.last_error.unwrap_or_else(|| error.to_string()),
                ));
            }
        };
        let command_preview = command_preview_from_project(project);

        {
            let mut processes = lock_mutex(&self.processes);
            if let Some(existing) = processes.get(&project.id) {
                if matches!(
                    existing.state.status,
                    RuntimeStatus::Starting | RuntimeStatus::Running | RuntimeStatus::Stopping
                ) {
                    return Err(RuntimeError::new(format!(
                        "Project '{}' is already running",
                        project.id
                    )));
                }
            }

            let mut starting_state =
                initial_process_state(project.id.clone(), command_preview.clone());
            starting_state.status = RuntimeStatus::Starting;
            emit_status_changed(
                &self.event_emitter,
                &status_event_from_state(&starting_state, None),
            );
            processes.insert(
                project.id.clone(),
                ManagedProcess {
                    state: starting_state,
                    child: None,
                    history_entry_id: None,
                    logs: LogBuffer::new(self.log_buffer_limit),
                },
            );
        }

        let mut child = match spawn_child(&run_request) {
            Ok(child) => child,
            Err(error) => {
                let failed_message = format!("Failed to start project '{}': {error}", project.id);
                let failed_state = self.store_failed_state(
                    project,
                    &command_preview,
                    &failed_message,
                    &started_at,
                );
                return Err(RuntimeError::new(
                    failed_state
                        .last_error
                        .unwrap_or_else(|| failed_message.clone()),
                ));
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let pid = child.id();
        let child_handle = Arc::new(Mutex::new(child));
        let history_entry_id = self
            .history_recorder
            .record_run_started(project, &command_preview, &started_at)
            .unwrap_or_else(|error| {
                log::error!(
                    "Failed to persist run history for '{}': {error}",
                    project.id
                );
                None
            });
        let running_state = ProcessRuntimeState {
            project_id: project.id.clone(),
            status: RuntimeStatus::Running,
            pid: Some(pid),
            started_at: Some(started_at),
            stopped_at: None,
            exit_code: None,
            last_error: None,
            command_preview: command_preview.clone(),
        };

        {
            let mut processes = lock_mutex(&self.processes);
            processes.insert(
                project.id.clone(),
                ManagedProcess {
                    state: running_state.clone(),
                    child: Some(child_handle.clone()),
                    history_entry_id,
                    logs: LogBuffer::new(self.log_buffer_limit),
                },
            );
        }

        emit_status_changed(
            &self.event_emitter,
            &status_event_from_state(&running_state, Some("Process started".into())),
        );

        if let Some(stdout) = stdout {
            spawn_log_pump(
                self.processes.clone(),
                self.event_emitter.clone(),
                project.id.clone(),
                RuntimeLogStream::Stdout,
                stdout,
            );
        }

        if let Some(stderr) = stderr {
            spawn_log_pump(
                self.processes.clone(),
                self.event_emitter.clone(),
                project.id.clone(),
                RuntimeLogStream::Stderr,
                stderr,
            );
        }

        spawn_exit_monitor(
            self.processes.clone(),
            self.event_emitter.clone(),
            self.history_recorder.clone(),
            project.id.clone(),
            child_handle,
        );

        Ok(running_state)
    }

    pub fn get_state(&self, project_id: &EntityId) -> Option<ProcessRuntimeState> {
        lock_mutex(&self.processes)
            .get(project_id)
            .map(|process| process.state.clone())
    }

    pub fn get_logs(&self, project_id: &EntityId) -> Vec<RuntimeLogLine> {
        lock_mutex(&self.processes)
            .get(project_id)
            .map(|process| process.logs.recent())
            .unwrap_or_default()
    }

    pub fn project_state(&self, project: &ProjectNode) -> ProcessRuntimeState {
        lock_mutex(&self.processes)
            .get(&project.id)
            .map(|process| process.state.clone())
            .unwrap_or_else(|| stopped_state_for(project))
    }

    pub fn stop_project(&self, project: &ProjectNode) -> RuntimeResult<ProcessRuntimeState> {
        let command_preview = command_preview_from_project(project);
        let (child_handle, history_entry_id, previous_state) = {
            let mut processes = lock_mutex(&self.processes);
            let Some(process) = processes.get_mut(&project.id) else {
                let stopped_state = stopped_state_for(project);
                processes.insert(
                    project.id.clone(),
                    ManagedProcess {
                        state: stopped_state.clone(),
                        child: None,
                        history_entry_id: None,
                        logs: LogBuffer::new(self.log_buffer_limit),
                    },
                );
                return Ok(stopped_state);
            };

            if process.child.is_none() {
                return Ok(process.state.clone());
            }

            let previous_state = process.state.clone();
            if process.state.status != RuntimeStatus::Stopping {
                process.state.status = RuntimeStatus::Stopping;
                process.state.command_preview = command_preview.clone();
                emit_status_changed(
                    &self.event_emitter,
                    &status_event_from_state(&process.state, Some("Stopping process".into())),
                );
            }

            (
                process.child.clone(),
                process.history_entry_id.take(),
                previous_state,
            )
        };

        let Some(child_handle) = child_handle else {
            return Ok(stopped_state_for(project));
        };

        let exit_status = match stop_process_tree(&child_handle) {
            Ok(exit_status) => exit_status,
            Err(error) => {
                let message = format!("Failed to stop project '{}': {error}", project.id);
                {
                    let mut processes = lock_mutex(&self.processes);
                    if let Some(process) = processes.get_mut(&project.id) {
                        if process
                            .child
                            .as_ref()
                            .is_some_and(|current| Arc::ptr_eq(current, &child_handle))
                        {
                            process.state = previous_state;
                            process.history_entry_id = history_entry_id.clone();
                            process.state.command_preview = command_preview.clone();
                            process.state.last_error = Some(message.clone());
                            emit_status_changed(
                                &self.event_emitter,
                                &status_event_from_state(&process.state, Some(message.clone())),
                            );
                            emit_process_error(
                                &self.event_emitter,
                                &error_event_from_state(&process.state, message.clone()),
                            );
                        }
                    }
                }

                return Err(RuntimeError::new(message));
            }
        };

        let stopped_at = now_iso_or_fallback();
        let exit_code = exit_status.as_ref().and_then(|status| status.code());
        let final_state = {
            let mut processes = lock_mutex(&self.processes);
            let process = processes
                .entry(project.id.clone())
                .or_insert_with(|| ManagedProcess {
                    state: stopped_state_for(project),
                    child: None,
                    history_entry_id: None,
                    logs: LogBuffer::new(self.log_buffer_limit),
                });
            if process
                .child
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &child_handle))
            {
                process.child = None;
                process.history_entry_id = None;
                process.state = ProcessRuntimeState {
                    stopped_at: Some(stopped_at.clone()),
                    exit_code,
                    last_error: None,
                    ..stopped_state_for(project)
                };

                emit_status_changed(
                    &self.event_emitter,
                    &status_event_from_state(&process.state, Some("Process stopped".into())),
                );
                emit_process_exited(
                    &self.event_emitter,
                    &exited_event_from_state(&process.state, Some("Process stopped".into())),
                );
            }

            process.state.clone()
        };

        if let Some(run_history_id) = history_entry_id {
            if let Err(error) = self.history_recorder.finalize_run(
                &run_history_id,
                FinalizeRunHistoryInput {
                    ended_at: Some(stopped_at.clone()),
                    exit_code,
                    final_runtime_status: final_state.status,
                    stop_reason: Some("manual-stop".into()),
                    error_message: None,
                },
            ) {
                log::error!(
                    "Failed to finalize run history for '{}': {error}",
                    project.id
                );
            }
        }

        Ok(final_state)
    }

    pub fn remove_project_runtime(&self, project: &ProjectNode) -> RuntimeResult<()> {
        self.stop_project(project)?;
        lock_mutex(&self.processes).remove(&project.id);

        Ok(())
    }

    pub fn shutdown_all(&self) {
        let active_processes = {
            let processes = lock_mutex(&self.processes);
            processes
                .iter()
                .filter_map(|(project_id, process)| {
                    process.child.as_ref().map(|child| {
                        (
                            project_id.clone(),
                            child.clone(),
                            process.history_entry_id.clone(),
                        )
                    })
                })
                .collect::<Vec<_>>()
        };

        for (project_id, child_handle, history_entry_id) in active_processes {
            let exit_status = stop_process_tree(&child_handle).ok().flatten();
            let stopped_at = now_iso_or_fallback();
            let exit_code = exit_status.as_ref().and_then(|status| status.code());

            let final_state = {
                let mut processes = lock_mutex(&self.processes);
                match processes.get_mut(&project_id) {
                    Some(process)
                        if process
                            .child
                            .as_ref()
                            .is_some_and(|current| Arc::ptr_eq(current, &child_handle)) =>
                    {
                        process.child = None;
                        process.history_entry_id = None;
                        process.state.status = RuntimeStatus::Stopped;
                        process.state.stopped_at = Some(stopped_at.clone());
                        process.state.exit_code = exit_code;
                        process.state.last_error = None;

                        emit_status_changed(
                            &self.event_emitter,
                            &status_event_from_state(
                                &process.state,
                                Some("Process stopped during shutdown".into()),
                            ),
                        );
                        emit_process_exited(
                            &self.event_emitter,
                            &exited_event_from_state(
                                &process.state,
                                Some("Process stopped during shutdown".into()),
                            ),
                        );

                        Some(process.state.clone())
                    }
                    _ => None,
                }
            };

            if let Some(final_state) = final_state {
                if let Some(run_history_id) = history_entry_id {
                    if let Err(error) = self.history_recorder.finalize_run(
                        &run_history_id,
                        FinalizeRunHistoryInput {
                            ended_at: Some(stopped_at.clone()),
                            exit_code,
                            final_runtime_status: final_state.status,
                            stop_reason: Some("app-shutdown".into()),
                            error_message: None,
                        },
                    ) {
                        log::error!(
                            "Failed to finalize run history during shutdown for '{}': {error}",
                            project_id
                        );
                    }
                }
            }
        }
    }

    fn build_run_request(&self, project: &ProjectNode) -> RuntimeResult<RunRequest> {
        let executable = project
            .executable
            .clone()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                RuntimeError::new(format!(
                    "Project '{}' does not have an executable configured",
                    project.id
                ))
            })?;

        let working_dir = project
            .working_dir
            .clone()
            .unwrap_or_else(|| project.path.clone());
        if working_dir.trim().is_empty() {
            return Err(RuntimeError::new(format!(
                "Project '{}' does not have a valid working directory",
                project.id
            )));
        }

        if !Path::new(&working_dir).is_dir() {
            return Err(RuntimeError::new(format!(
                "Project '{}' has an invalid working directory: {working_dir}",
                project.id
            )));
        }

        let resolved_executable = resolve_executable(&executable, Path::new(&working_dir))
            .ok_or_else(|| {
                RuntimeError::new(format!(
                    "Project '{}' executable could not be resolved from the working directory or PATH: {executable}",
                    project.id
                ))
            })?;

        Ok(RunRequest {
            project_id: project.id.clone(),
            executable: resolved_executable.display().to_string(),
            args: project.args.clone().unwrap_or_default(),
            working_dir,
            env: project.env.clone(),
        })
    }

    fn store_failed_state(
        &self,
        project: &ProjectNode,
        command_preview: &str,
        last_error: &str,
        started_at: &str,
    ) -> ProcessRuntimeState {
        let mut failed_state =
            initial_process_state(project.id.clone(), command_preview.to_owned());
        failed_state.status = RuntimeStatus::Failed;
        failed_state.last_error = Some(last_error.to_owned());
        failed_state.stopped_at = Some(now_iso_or_fallback());

        let mut processes = lock_mutex(&self.processes);
        processes.remove(&project.id);
        processes.insert(
            project.id.clone(),
            ManagedProcess {
                state: failed_state.clone(),
                child: None,
                history_entry_id: None,
                logs: LogBuffer::new(self.log_buffer_limit),
            },
        );
        drop(processes);

        emit_status_changed(
            &self.event_emitter,
            &status_event_from_state(&failed_state, failed_state.last_error.clone()),
        );
        emit_process_error(
            &self.event_emitter,
            &error_event_from_state(&failed_state, last_error.to_owned()),
        );
        if let Err(error) = self.history_recorder.record_start_failure(
            project,
            command_preview,
            started_at,
            last_error,
        ) {
            log::error!(
                "Failed to persist start failure history for '{}': {error}",
                project.id
            );
        }

        failed_state
    }
}

fn spawn_child(run_request: &RunRequest) -> std::io::Result<Child> {
    let mut command = if should_spawn_via_cmd(&run_request.executable) {
        let mut command = Command::new(windows_command_shell());
        command
            .arg("/C")
            .arg(windows_shell_path(&run_request.executable));
        command.args(&run_request.args);
        command
    } else {
        let mut command = Command::new(&run_request.executable);
        command.args(&run_request.args);
        command
    };

    command
        .current_dir(&run_request.working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(env) = run_request.env.as_ref() {
        command.envs(env);
    }

    command.spawn()
}

#[cfg(windows)]
fn should_spawn_via_cmd(executable: &str) -> bool {
    Path::new(executable)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "cmd" | "bat"))
}

#[cfg(not(windows))]
fn should_spawn_via_cmd(_executable: &str) -> bool {
    false
}

#[cfg(windows)]
fn windows_command_shell() -> String {
    env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".into())
}

#[cfg(windows)]
fn windows_shell_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{stripped}");
    }

    path.strip_prefix(r"\\?\")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_owned())
}

#[cfg(not(windows))]
fn windows_shell_path(path: &str) -> String {
    path.to_owned()
}

fn spawn_log_pump<R>(
    processes: Arc<Mutex<HashMap<EntityId, ManagedProcess>>>,
    event_emitter: RuntimeEventEmitter,
    project_id: EntityId,
    stream: RuntimeLogStream,
    reader: R,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0_u8; LOG_READ_BUFFER_SIZE];
        let mut pending_bytes = Vec::new();

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    flush_pending_log_bytes(
                        &processes,
                        &event_emitter,
                        &project_id,
                        stream,
                        &mut pending_bytes,
                    );
                    break;
                }
                Ok(bytes_read) => {
                    pending_bytes.extend_from_slice(&buffer[..bytes_read]);
                    emit_complete_log_text(
                        &processes,
                        &event_emitter,
                        &project_id,
                        stream,
                        &mut pending_bytes,
                    );
                }
                Err(error) => {
                    let command_preview = {
                        let processes = lock_mutex(&processes);
                        processes
                            .get(&project_id)
                            .map(|process| process.state.command_preview.clone())
                            .unwrap_or_else(|| "Manual review required".into())
                    };

                    emit_process_error(
                        &event_emitter,
                        &RuntimeProcessErrorEvent {
                            project_id: project_id.clone(),
                            status: RuntimeStatus::Failed,
                            pid: None,
                            timestamp: now_iso_or_fallback(),
                            message: format!("Failed to read process output: {error}"),
                            command_preview,
                        },
                    );
                    break;
                }
            }
        }
    });
}

fn emit_complete_log_text(
    processes: &Arc<Mutex<HashMap<EntityId, ManagedProcess>>>,
    event_emitter: &RuntimeEventEmitter,
    project_id: &EntityId,
    stream: RuntimeLogStream,
    pending_bytes: &mut Vec<u8>,
) {
    loop {
        match std::str::from_utf8(pending_bytes) {
            Ok(text) => {
                if !text.is_empty() {
                    emit_log_chunk(
                        processes,
                        event_emitter,
                        project_id,
                        stream,
                        text.to_owned(),
                    );
                    pending_bytes.clear();
                }
                break;
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                if valid_up_to > 0 {
                    let text = String::from_utf8_lossy(&pending_bytes[..valid_up_to]).into_owned();
                    pending_bytes.drain(..valid_up_to);
                    emit_log_chunk(processes, event_emitter, project_id, stream, text);
                    continue;
                }

                if let Some(error_len) = error.error_len() {
                    let text = String::from_utf8_lossy(&pending_bytes[..error_len]).into_owned();
                    pending_bytes.drain(..error_len);
                    emit_log_chunk(processes, event_emitter, project_id, stream, text);
                    continue;
                }

                break;
            }
        }
    }
}

fn flush_pending_log_bytes(
    processes: &Arc<Mutex<HashMap<EntityId, ManagedProcess>>>,
    event_emitter: &RuntimeEventEmitter,
    project_id: &EntityId,
    stream: RuntimeLogStream,
    pending_bytes: &mut Vec<u8>,
) {
    if pending_bytes.is_empty() {
        return;
    }

    let text = String::from_utf8_lossy(pending_bytes).into_owned();
    pending_bytes.clear();
    emit_log_chunk(processes, event_emitter, project_id, stream, text);
}

fn emit_log_chunk(
    processes: &Arc<Mutex<HashMap<EntityId, ManagedProcess>>>,
    event_emitter: &RuntimeEventEmitter,
    project_id: &EntityId,
    stream: RuntimeLogStream,
    line: String,
) {
    let payload = RuntimeLogLine {
        project_id: project_id.clone(),
        stream,
        line,
        partial: true,
        timestamp: now_iso_or_fallback(),
    };

    {
        let mut processes = lock_mutex(processes);
        if let Some(process) = processes.get_mut(project_id) {
            process.logs.append(payload.clone());
        }
    }

    emit_log_line(event_emitter, &payload);
}

fn spawn_exit_monitor(
    processes: Arc<Mutex<HashMap<EntityId, ManagedProcess>>>,
    event_emitter: RuntimeEventEmitter,
    history_recorder: HistoryRecorder,
    project_id: EntityId,
    child_handle: Arc<Mutex<Child>>,
) {
    thread::spawn(move || loop {
        let exit_status = {
            let mut child = lock_mutex(&child_handle);
            child.try_wait()
        };

        match exit_status {
            Ok(Some(status)) => {
                let finalized = {
                    let mut processes = lock_mutex(&processes);
                    match processes.get_mut(&project_id) {
                        Some(process)
                            if process
                                .child
                                .as_ref()
                                .is_some_and(|current| Arc::ptr_eq(current, &child_handle)) =>
                        {
                            process.child = None;
                            let history_entry_id = process.history_entry_id.clone();
                            let was_stopping = process.state.status == RuntimeStatus::Stopping;
                            process.history_entry_id = None;
                            process.state.stopped_at = Some(now_iso_or_fallback());
                            process.state.exit_code = status.code();
                            process.state.status = if was_stopping || status.success() {
                                RuntimeStatus::Stopped
                            } else {
                                RuntimeStatus::Failed
                            };
                            process.state.last_error =
                                if process.state.status == RuntimeStatus::Stopped {
                                    None
                                } else {
                                    Some(unexpected_exit_message(status.code()))
                                };

                            emit_status_changed(
                                &event_emitter,
                                &status_event_from_state(
                                    &process.state,
                                    process.state.last_error.clone(),
                                ),
                            );
                            emit_process_exited(
                                &event_emitter,
                                &exited_event_from_state(
                                    &process.state,
                                    process.state.last_error.clone(),
                                ),
                            );
                            if let Some(message) = process.state.last_error.clone() {
                                emit_process_error(
                                    &event_emitter,
                                    &error_event_from_state(&process.state, message),
                                );
                            }

                            Some((
                                process.state.clone(),
                                history_entry_id,
                                process.state.last_error.clone(),
                            ))
                        }
                        _ => None,
                    }
                };

                if let Some((final_state, history_entry_id, error_message)) = finalized {
                    if let Some(run_history_id) = history_entry_id {
                        if let Err(error) = history_recorder.finalize_run(
                            &run_history_id,
                            FinalizeRunHistoryInput {
                                ended_at: Some(
                                    final_state
                                        .stopped_at
                                        .clone()
                                        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into()),
                                ),
                                exit_code: final_state.exit_code,
                                final_runtime_status: final_state.status,
                                stop_reason: Some(
                                    if final_state.status == RuntimeStatus::Stopped {
                                        if status.success() {
                                            "process-exited".into()
                                        } else {
                                            "manual-stop".into()
                                        }
                                    } else {
                                        "unexpected-exit".into()
                                    },
                                ),
                                error_message,
                            },
                        ) {
                            log::error!(
                                "Failed to finalize run history for '{}': {error}",
                                project_id
                            );
                        }
                    }
                }

                break;
            }
            Ok(None) => thread::sleep(EXIT_POLL_INTERVAL),
            Err(error) => {
                let finalized =
                    {
                        let mut processes = lock_mutex(&processes);
                        match processes.get_mut(&project_id) {
                            Some(process)
                                if process
                                    .child
                                    .as_ref()
                                    .is_some_and(|current| Arc::ptr_eq(current, &child_handle)) =>
                            {
                                process.child = None;
                                let history_entry_id = process.history_entry_id.clone();
                                process.history_entry_id = None;
                                process.state.status = RuntimeStatus::Failed;
                                process.state.stopped_at = Some(now_iso_or_fallback());
                                process.state.last_error =
                                    Some(format!("Failed to monitor process exit: {error}"));

                                emit_status_changed(
                                    &event_emitter,
                                    &status_event_from_state(
                                        &process.state,
                                        process.state.last_error.clone(),
                                    ),
                                );
                                emit_process_error(
                                    &event_emitter,
                                    &error_event_from_state(
                                        &process.state,
                                        process.state.last_error.clone().unwrap_or_else(|| {
                                            "Failed to monitor process exit".into()
                                        }),
                                    ),
                                );

                                Some((
                                    process.state.clone(),
                                    history_entry_id,
                                    process.state.last_error.clone(),
                                ))
                            }
                            _ => None,
                        }
                    };

                if let Some((final_state, history_entry_id, error_message)) = finalized {
                    if let Some(run_history_id) = history_entry_id {
                        if let Err(history_error) = history_recorder.finalize_run(
                            &run_history_id,
                            FinalizeRunHistoryInput {
                                ended_at: Some(
                                    final_state
                                        .stopped_at
                                        .clone()
                                        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into()),
                                ),
                                exit_code: final_state.exit_code,
                                final_runtime_status: RuntimeStatus::Failed,
                                stop_reason: Some("monitor-error".into()),
                                error_message,
                            },
                        ) {
                            log::error!(
                                "Failed to finalize run history after monitor error for '{}': {history_error}",
                                project_id
                            );
                        }
                    }
                }

                break;
            }
        }
    });
}

fn command_preview_from_project(project: &ProjectNode) -> String {
    if let (Some(executable), Some(args)) = (project.executable.as_ref(), project.args.as_ref()) {
        if !args.is_empty() {
            return format!("{executable} {}", args.join(" "));
        }
    }

    project
        .command
        .clone()
        .or_else(|| project.executable.clone())
        .unwrap_or_else(|| "Manual review required".into())
}

fn stopped_state_for(project: &ProjectNode) -> ProcessRuntimeState {
    ProcessRuntimeState {
        project_id: project.id.clone(),
        status: RuntimeStatus::Stopped,
        pid: None,
        started_at: None,
        stopped_at: None,
        exit_code: None,
        last_error: None,
        command_preview: command_preview_from_project(project),
    }
}

fn status_event_from_state(
    state: &ProcessRuntimeState,
    message: Option<String>,
) -> RuntimeStatusEvent {
    RuntimeStatusEvent {
        project_id: state.project_id.clone(),
        status: state.status,
        pid: state.pid,
        timestamp: now_iso_or_fallback(),
        message,
        command_preview: state.command_preview.clone(),
    }
}

fn exited_event_from_state(
    state: &ProcessRuntimeState,
    message: Option<String>,
) -> RuntimeProcessExitedEvent {
    RuntimeProcessExitedEvent {
        project_id: state.project_id.clone(),
        status: state.status,
        pid: state.pid,
        timestamp: now_iso_or_fallback(),
        exit_code: state.exit_code,
        message,
        command_preview: state.command_preview.clone(),
    }
}

fn error_event_from_state(
    state: &ProcessRuntimeState,
    message: String,
) -> RuntimeProcessErrorEvent {
    RuntimeProcessErrorEvent {
        project_id: state.project_id.clone(),
        status: state.status,
        pid: state.pid,
        timestamp: now_iso_or_fallback(),
        message,
        command_preview: state.command_preview.clone(),
    }
}

fn unexpected_exit_message(exit_code: Option<i32>) -> String {
    match exit_code {
        Some(exit_code) => format!("Process exited unexpectedly with code {exit_code}"),
        None => "Process exited unexpectedly".into(),
    }
}

fn now_iso_or_fallback() -> String {
    timestamps::now_iso().unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        io::{Cursor, Read},
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::{spawn_child, spawn_log_pump, ManagedProcess, ProcessManager};
    use crate::{
        events::{noop_runtime_event_emitter, RuntimeEventEmitter, LOG_LINE_EVENT},
        models::{GroupNode, ProjectNode, RunRequest, Workspace},
        persistence::{
            test_utils::TestDatabase, GroupRepository, ProjectRepository, RunHistoryRepository,
            WorkspaceRepository,
        },
        runtime::LogBuffer,
    };

    const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
    const POLL_INTERVAL: Duration = Duration::from_millis(50);

    #[test]
    fn starts_persisted_project_and_tracks_runtime_state() {
        let test_database = TestDatabase::new("process-manager-starts-persisted-project");
        let project = seed_runnable_project(&test_database, "project-run");
        let manager = ProcessManager::new();

        let state = manager
            .start_persisted_project(test_database.database(), &project.id)
            .expect("project should start");

        assert_eq!(state.project_id, project.id);
        assert_eq!(state.status, crate::models::RuntimeStatus::Running);
        assert!(state.pid.is_some());
        assert!(state.started_at.is_some());
        assert_eq!(state.command_preview, preview_for_test_command());

        let stored_state = manager
            .get_state(&project.id)
            .expect("project state should be stored");
        assert_eq!(stored_state.status, crate::models::RuntimeStatus::Running);

        manager.shutdown_all();
    }

    #[test]
    fn prevents_double_start_for_the_same_project() {
        let test_database = TestDatabase::new("process-manager-prevents-double-start");
        let project = seed_runnable_project(&test_database, "project-double-start");
        let manager = ProcessManager::new();

        manager
            .start_persisted_project(test_database.database(), &project.id)
            .expect("first start should succeed");

        let second_attempt = manager.start_persisted_project(test_database.database(), &project.id);

        assert!(second_attempt.is_err());
        assert_eq!(
            second_attempt
                .expect_err("second start should fail")
                .to_string(),
            format!("Project '{}' is already running", project.id)
        );

        manager.shutdown_all();
    }

    #[test]
    fn marks_state_as_failed_when_executable_is_missing() {
        let test_database = TestDatabase::new("process-manager-missing-executable");
        let project = seed_project(
            &test_database,
            ProjectNode {
                executable: None,
                args: Some(test_command_args()),
                ..base_project("project-missing-executable")
            },
        );
        let manager = ProcessManager::new();

        let start_result = manager.start_persisted_project(test_database.database(), &project.id);

        assert!(start_result.is_err());
        let state = manager
            .get_state(&project.id)
            .expect("failed project state should be stored");
        assert_eq!(state.status, crate::models::RuntimeStatus::Failed);
        assert_eq!(
            state.last_error.as_deref(),
            Some("Project 'project-missing-executable' does not have an executable configured")
        );
    }

    #[test]
    fn updates_state_when_process_exits_on_its_own() {
        let test_database = TestDatabase::new("process-manager-process-exits");
        let project = seed_project(
            &test_database,
            ProjectNode {
                args: Some(vec!["/C".into(), "exit".into(), "0".into()]),
                ..base_project("project-exits")
            },
        );
        let manager = ProcessManager::new();

        manager
            .start_persisted_project(test_database.database(), &project.id)
            .expect("project should start");

        let final_state =
            wait_for_status(&manager, &project.id, crate::models::RuntimeStatus::Stopped)
                .expect("process should eventually stop");

        assert_eq!(final_state.exit_code, Some(0));
        assert!(final_state.stopped_at.is_some());
    }

    #[cfg(windows)]
    #[test]
    fn treats_non_zero_exit_while_stopping_as_stopped() {
        let project = base_project("project-stopping-non-zero-exit");
        let manager = ProcessManager::new();
        let run_request = RunRequest {
            project_id: project.id.clone(),
            executable: test_shell_path(),
            args: vec!["/C".into(), "exit".into(), "1".into()],
            working_dir: env::temp_dir().display().to_string(),
            env: None,
        };
        let child = spawn_child(&run_request).expect("test process should spawn");
        let pid = child.id();
        let child_handle = Arc::new(Mutex::new(child));
        let mut state = crate::runtime::initial_process_state(project.id.clone(), "cmd /C exit 1");
        state.status = crate::models::RuntimeStatus::Stopping;
        state.pid = Some(pid);

        {
            let mut processes = super::lock_mutex(&manager.processes);
            processes.insert(
                project.id.clone(),
                ManagedProcess {
                    state,
                    child: Some(child_handle.clone()),
                    history_entry_id: None,
                    logs: LogBuffer::new(10),
                },
            );
        }

        super::spawn_exit_monitor(
            manager.processes.clone(),
            manager.event_emitter.clone(),
            manager.history_recorder.clone(),
            project.id.clone(),
            child_handle,
        );

        let final_state =
            wait_for_status(&manager, &project.id, crate::models::RuntimeStatus::Stopped)
                .expect("stopping process should finish as stopped");

        assert_eq!(final_state.exit_code, Some(1));
        assert!(final_state.last_error.is_none());
    }

    #[test]
    fn captures_stdout_and_stderr_in_the_log_buffer() {
        let test_database = TestDatabase::new("process-manager-captures-logs");
        let project = seed_project(
            &test_database,
            ProjectNode {
                args: Some(vec![
                    "/C".into(),
                    "echo out-1 && echo err-1 1>&2 && echo out-2 && echo err-2 1>&2".into(),
                ]),
                ..base_project("project-logs")
            },
        );
        let manager =
            ProcessManager::with_event_emitter_and_capacity(noop_runtime_event_emitter(), 100);

        manager
            .start_persisted_project(test_database.database(), &project.id)
            .expect("project should start");

        wait_for_status(&manager, &project.id, crate::models::RuntimeStatus::Stopped)
            .expect("process should eventually stop");

        let logs = manager.get_logs(&project.id);

        let combined_output = logs
            .iter()
            .map(|line| line.line.as_str())
            .collect::<Vec<_>>()
            .join("");

        assert!(!logs.is_empty());
        assert!(logs
            .iter()
            .any(|line| line.stream == crate::models::RuntimeLogStream::Stdout));
        assert!(logs
            .iter()
            .any(|line| line.stream == crate::models::RuntimeLogStream::Stderr));
        assert!(combined_output.contains("out-1"));
        assert!(combined_output.contains("err-1"));
        assert!(combined_output.contains("out-2"));
        assert!(combined_output.contains("err-2"));
    }

    #[test]
    fn captures_output_chunks_without_trailing_newline() {
        let project = base_project("project-partial-output");
        let manager =
            ProcessManager::with_event_emitter_and_capacity(noop_runtime_event_emitter(), 100);

        {
            let mut processes = super::lock_mutex(&manager.processes);
            processes.insert(
                project.id.clone(),
                ManagedProcess {
                    state: crate::runtime::initial_process_state(project.id.clone(), "test"),
                    child: None,
                    history_entry_id: None,
                    logs: LogBuffer::new(100),
                },
            );
        }

        spawn_log_pump(
            manager.processes.clone(),
            manager.event_emitter.clone(),
            project.id.clone(),
            crate::models::RuntimeLogStream::Stdout,
            Cursor::new(b"prompt without newline".to_vec()),
        );

        let logs = (0..100)
            .find_map(|_| {
                let logs = manager.get_logs(&project.id);
                if logs.is_empty() {
                    thread::sleep(POLL_INTERVAL);
                    None
                } else {
                    Some(logs)
                }
            })
            .expect("partial output chunk should be captured");

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].line, "prompt without newline");
        assert!(logs[0].partial);
    }

    #[test]
    fn preserves_utf8_split_across_output_reads() {
        let project = base_project("project-split-utf8-output");
        let manager =
            ProcessManager::with_event_emitter_and_capacity(noop_runtime_event_emitter(), 10);
        let expected_output = "l\u{ed}nea sin salto";

        {
            let mut processes = super::lock_mutex(&manager.processes);
            processes.insert(
                project.id.clone(),
                ManagedProcess {
                    state: crate::runtime::initial_process_state(project.id.clone(), "test"),
                    child: None,
                    history_entry_id: None,
                    logs: LogBuffer::new(100),
                },
            );
        }

        spawn_log_pump(
            manager.processes.clone(),
            manager.event_emitter.clone(),
            project.id.clone(),
            crate::models::RuntimeLogStream::Stdout,
            OneByteReader::new(expected_output.as_bytes()),
        );

        let logs = (0..100)
            .find_map(|_| {
                let output = manager
                    .get_logs(&project.id)
                    .iter()
                    .map(|line| line.line.as_str())
                    .collect::<Vec<_>>()
                    .join("");

                if output == expected_output {
                    Some(output)
                } else {
                    thread::sleep(POLL_INTERVAL);
                    None
                }
            })
            .expect("split UTF-8 output should be reassembled");

        assert_eq!(logs, expected_output);
    }

    #[test]
    fn emits_runtime_events_for_log_lines() {
        let test_database = TestDatabase::new("process-manager-emits-log-events");
        let project = seed_project(
            &test_database,
            ProjectNode {
                args: Some(vec!["/C".into(), "echo event-log".into()]),
                ..base_project("project-events")
            },
        );
        let emitted = Arc::new(Mutex::new(Vec::<String>::new()));
        let emitted_clone = emitted.clone();
        let emitter: RuntimeEventEmitter = Arc::new(move |event_name, payload| {
            emitted_clone
                .lock()
                .expect("event list should lock")
                .push(format!(
                    "{event_name}:{}",
                    payload["line"].as_str().unwrap_or_default()
                ));
        });
        let manager = ProcessManager::with_event_emitter_and_capacity(emitter, 10);

        manager
            .start_persisted_project(test_database.database(), &project.id)
            .expect("project should start");

        wait_for_status(&manager, &project.id, crate::models::RuntimeStatus::Stopped)
            .expect("process should eventually stop");

        let events = emitted.lock().expect("event list should lock");
        assert!(events.iter().any(|event| event.starts_with(LOG_LINE_EVENT)));
    }

    #[test]
    fn persists_run_history_when_manager_has_database() {
        let test_database = TestDatabase::new("process-manager-persists-history");
        let project = seed_project(
            &test_database,
            ProjectNode {
                args: Some(vec![
                    "/C".into(),
                    "ping".into(),
                    "127.0.0.1".into(),
                    "-n".into(),
                    "10".into(),
                ]),
                ..base_project("project-history")
            },
        );
        let manager = ProcessManager::with_database_and_event_emitter(
            Some(test_database.database()),
            noop_runtime_event_emitter(),
        );

        manager
            .start_persisted_project(test_database.database(), &project.id)
            .expect("project should start");
        manager
            .stop_project(&project)
            .expect("project should stop cleanly");

        let history = RunHistoryRepository::new(test_database.database())
            .list_by_project(&project.id, 10)
            .expect("run history should load");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].stop_reason.as_deref(), Some("manual-stop"));
    }

    #[test]
    fn removes_project_runtime_after_stopping_it() {
        let test_database = TestDatabase::new("process-manager-removes-project-runtime");
        let project = seed_runnable_project(&test_database, "project-remove-runtime");
        let manager = ProcessManager::new();

        manager
            .start_persisted_project(test_database.database(), &project.id)
            .expect("project should start");
        assert!(manager.get_state(&project.id).is_some());

        manager
            .remove_project_runtime(&project)
            .expect("project runtime should be removed");

        assert!(manager.get_state(&project.id).is_none());
        assert!(manager.get_logs(&project.id).is_empty());
    }

    #[test]
    fn clears_previous_logs_when_a_new_start_attempt_fails() {
        let manager = ProcessManager::new();
        let project = base_project("project-failed-retry");
        let mut existing_logs = LogBuffer::new(10);
        existing_logs.append(crate::models::RuntimeLogLine {
            project_id: project.id.clone(),
            stream: crate::models::RuntimeLogStream::Stderr,
            line: "stale error".into(),
            partial: false,
            timestamp: "2026-04-16T12:00:00Z".into(),
        });

        {
            let mut processes = super::lock_mutex(&manager.processes);
            processes.insert(
                project.id.clone(),
                ManagedProcess {
                    state: crate::runtime::initial_process_state(project.id.clone(), "npm start"),
                    child: None,
                    history_entry_id: None,
                    logs: existing_logs,
                },
            );
        }

        manager.store_failed_state(
            &project,
            "npm start",
            "Project failed again",
            "2026-04-16T12:01:00Z",
        );

        assert!(manager.get_logs(&project.id).is_empty());
    }

    #[test]
    fn stop_project_preserves_terminal_state_when_child_is_already_cleared() {
        let manager = ProcessManager::new();
        let project = base_project("project-stop-after-exit");
        let mut failed_state =
            crate::runtime::initial_process_state(project.id.clone(), "npm run dev");
        failed_state.status = crate::models::RuntimeStatus::Failed;
        failed_state.pid = Some(1234);
        failed_state.stopped_at = Some("2026-04-16T12:01:00Z".into());
        failed_state.exit_code = Some(1);
        failed_state.last_error = Some("Process exited unexpectedly with code 1".into());

        {
            let mut processes = super::lock_mutex(&manager.processes);
            processes.insert(
                project.id.clone(),
                ManagedProcess {
                    state: failed_state.clone(),
                    child: None,
                    history_entry_id: None,
                    logs: LogBuffer::new(10),
                },
            );
        }

        let state = manager
            .stop_project(&project)
            .expect("late stop should return the stored terminal state");

        assert_eq!(state, failed_state);
    }

    #[cfg(windows)]
    #[test]
    fn resolves_windows_command_wrappers_before_starting() {
        let working_dir = TempProjectDir::new("process-manager-cmd-wrapper");
        let wrapper_path = working_dir.path().join("npm.cmd");
        fs::write(&wrapper_path, "@echo off\r\necho wrapper-ok\r\n").expect("wrapper should exist");
        let project = ProjectNode {
            executable: Some("npm".into()),
            working_dir: Some(working_dir.path().display().to_string()),
            path: working_dir.path().display().to_string(),
            args: Some(vec!["start".into()]),
            ..base_project("project-wrapper")
        };
        let manager = ProcessManager::new();

        let run_request = manager
            .build_run_request(&project)
            .expect("wrapper executable should resolve");

        assert!(run_request.executable.ends_with("npm.cmd"));
        assert_eq!(
            run_request.working_dir,
            working_dir.path().display().to_string()
        );
    }

    #[cfg(windows)]
    #[test]
    fn spawns_windows_command_wrappers_via_cmd_shell() {
        let working_dir = TempProjectDir::new("process-manager-start-wrapper");
        let wrapper_path = working_dir.path().join("npm.cmd");
        fs::write(
            &wrapper_path,
            "@echo off\r\necho wrapper-ok %1\r\nexit /b 0\r\n",
        )
        .expect("wrapper should exist");
        let run_request = RunRequest {
            project_id: "project-wrapper-start".into(),
            executable: wrapper_path.display().to_string(),
            args: vec!["start".into()],
            working_dir: working_dir.path().display().to_string(),
            env: None,
        };
        let mut child = spawn_child(&run_request).expect("wrapper command should spawn");
        let stdout = child
            .stdout
            .take()
            .expect("wrapper command should expose stdout");
        let mut output = String::new();

        std::io::Read::read_to_string(&mut std::io::BufReader::new(stdout), &mut output)
            .expect("wrapper stdout should be readable");

        let status = child.wait().expect("wrapper command should exit");

        assert!(status.success());
        assert!(output.contains("wrapper-ok start"));
    }

    #[cfg(windows)]
    #[test]
    fn strips_verbatim_prefix_before_spawning_windows_command_wrappers() {
        let working_dir = TempProjectDir::new("process-manager-verbatim-wrapper");
        let wrapper_path = working_dir.path().join("npm.cmd");
        fs::write(
            &wrapper_path,
            "@echo off\r\necho wrapper-ok %1\r\nexit /b 0\r\n",
        )
        .expect("wrapper should exist");
        let run_request = RunRequest {
            project_id: "project-verbatim-wrapper-start".into(),
            executable: format!(r"\\?\{}", wrapper_path.display()),
            args: vec!["start".into()],
            working_dir: working_dir.path().display().to_string(),
            env: None,
        };
        let mut child = spawn_child(&run_request).expect("verbatim wrapper command should spawn");
        let stdout = child
            .stdout
            .take()
            .expect("wrapper command should expose stdout");
        let mut output = String::new();

        std::io::Read::read_to_string(&mut std::io::BufReader::new(stdout), &mut output)
            .expect("wrapper stdout should be readable");

        let status = child.wait().expect("wrapper command should exit");

        assert!(status.success());
        assert!(output.contains("wrapper-ok start"));
    }

    fn seed_runnable_project(test_database: &TestDatabase, project_id: &str) -> ProjectNode {
        seed_project(
            test_database,
            ProjectNode {
                args: Some(test_command_args()),
                ..base_project(project_id)
            },
        )
    }

    fn seed_project(test_database: &TestDatabase, project: ProjectNode) -> ProjectNode {
        let workspace_repository = WorkspaceRepository::new(test_database.database());
        let group_repository = GroupRepository::new(test_database.database());
        let project_repository = ProjectRepository::new(test_database.database());
        let workspace = Workspace {
            id: "workspace-runtime".into(),
            name: "Runtime".into(),
            created_at: "2026-04-16T10:00:00Z".into(),
            updated_at: "2026-04-16T10:00:00Z".into(),
        };
        let group = GroupNode {
            id: "group-runtime".into(),
            workspace_id: workspace.id.clone(),
            parent_group_id: None,
            name: "Runtime".into(),
            color: "#2563eb".into(),
            sort_order: 10,
            created_at: "2026-04-16T10:00:00Z".into(),
            updated_at: "2026-04-16T10:00:00Z".into(),
        };

        workspace_repository
            .create(&workspace)
            .expect("workspace should be created");
        group_repository
            .create(&group)
            .expect("group should be created");
        project_repository
            .create(&project)
            .expect("project should be created");

        project
    }

    fn base_project(project_id: &str) -> ProjectNode {
        let working_dir = env::temp_dir().display().to_string();

        ProjectNode {
            id: project_id.into(),
            workspace_id: "workspace-runtime".into(),
            group_id: "group-runtime".into(),
            name: project_id.into(),
            path: working_dir.clone(),
            detected_type: None,
            color: None,
            package_manager: None,
            executable: Some(test_shell_path()),
            command: Some(preview_for_test_command()),
            args: Some(vec![]),
            env: None,
            working_dir: Some(working_dir),
            detection_confidence: None,
            detection_evidence: None,
            warnings: None,
            created_at: "2026-04-16T10:00:00Z".into(),
            updated_at: "2026-04-16T10:00:00Z".into(),
        }
    }

    fn test_shell_path() -> String {
        env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".into())
    }

    fn test_command_args() -> Vec<String> {
        vec![
            "/C".into(),
            "ping".into(),
            "127.0.0.1".into(),
            "-n".into(),
            "3".into(),
        ]
    }

    fn preview_for_test_command() -> String {
        format!("{} {}", test_shell_path(), test_command_args().join(" "))
    }

    struct TempProjectDir {
        path: PathBuf,
    }

    impl TempProjectDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("current time should be after unix epoch")
                .as_nanos();
            let path = env::temp_dir().join(format!("centralita-{name}-{unique}"));
            fs::create_dir_all(&path).expect("temp project directory should exist");

            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    struct OneByteReader {
        bytes: Vec<u8>,
        index: usize,
    }

    impl OneByteReader {
        fn new(bytes: &[u8]) -> Self {
            Self {
                bytes: bytes.to_vec(),
                index: 0,
            }
        }
    }

    impl Read for OneByteReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if self.index >= self.bytes.len() {
                return Ok(0);
            }

            buffer[0] = self.bytes[self.index];
            self.index += 1;
            Ok(1)
        }
    }

    impl Drop for TempProjectDir {
        fn drop(&mut self) {
            if self.path.exists() {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
    }

    fn wait_for_status(
        manager: &ProcessManager,
        project_id: &str,
        expected_status: crate::models::RuntimeStatus,
    ) -> Option<crate::models::ProcessRuntimeState> {
        let start = std::time::Instant::now();
        while start.elapsed() < WAIT_TIMEOUT {
            if let Some(state) = manager.get_state(&project_id.to_owned()) {
                if state.status == expected_status {
                    return Some(state);
                }
            }

            thread::sleep(POLL_INTERVAL);
        }

        None
    }
}
