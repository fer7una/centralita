use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    detection,
    models::{
        CommandArgs, CommandValidation, DetectedProjectType, DetectionEvidence, DetectionResult,
        DetectionWarning, EntityId, EnvironmentVariables, GroupNode, HealthCheckConfig,
        ProjectHealthState, ProjectNode, ProjectPackageManager, RunHistoryEntry,
        RuntimeBulkFailure, RuntimeBulkOperationResult, RuntimeLogLine, RuntimeOperationScope,
        RuntimeStatus, Workspace, WorkspaceObservabilitySummary, WorkspaceRuntimeStatus,
        WorkspaceTree,
    },
    persistence::{
        AppDatabase, GroupRepository, ProjectRepository, RunHistoryRepository, WorkspaceRepository,
        WorkspaceTreeRepository,
    },
    runtime::{build_workspace_observability_summary, ProcessManager},
    utils::{ids, timestamps},
};

type CommandResult<T> = Result<T, String>;
const BULK_STOP_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorkspaceInput {
    pub id: EntityId,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEntityInput {
    pub id: EntityId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWorkspaceTreeInput {
    pub workspace_id: EntityId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeProjectFolderInput {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateProjectCommandInput {
    pub path: String,
    pub executable: Option<String>,
    pub args: Option<CommandArgs>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectGitInfoInput {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGitInfo {
    pub is_repository: bool,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectLogsInput {
    pub project_id: EntityId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRuntimeInput {
    pub project_id: EntityId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRuntimeInput {
    pub workspace_id: EntityId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRuntimeInput {
    pub group_id: EntityId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectFromDetectionInput {
    pub workspace_id: EntityId,
    pub group_id: EntityId,
    pub name: String,
    pub path: String,
    pub detected_type: DetectedProjectType,
    pub color: Option<String>,
    pub package_manager: Option<ProjectPackageManager>,
    pub executable: Option<String>,
    pub command: Option<String>,
    pub args: Option<CommandArgs>,
    pub env: Option<EnvironmentVariables>,
    pub working_dir: Option<String>,
    pub detection_confidence: f64,
    pub detection_evidence: Vec<DetectionEvidence>,
    pub warnings: Option<Vec<DetectionWarning>>,
    pub health_check: Option<HealthCheckConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupInput {
    pub workspace_id: EntityId,
    pub parent_group_id: Option<EntityId>,
    pub name: String,
    pub color: String,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGroupInput {
    pub id: EntityId,
    pub workspace_id: EntityId,
    pub parent_group_id: Option<EntityId>,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub workspace_id: EntityId,
    pub group_id: EntityId,
    pub name: String,
    pub path: String,
    pub detected_type: Option<DetectedProjectType>,
    pub color: Option<String>,
    pub package_manager: Option<ProjectPackageManager>,
    pub executable: Option<String>,
    pub command: Option<String>,
    pub args: Option<CommandArgs>,
    pub env: Option<EnvironmentVariables>,
    pub working_dir: Option<String>,
    pub detection_confidence: Option<f64>,
    pub detection_evidence: Option<Vec<DetectionEvidence>>,
    pub warnings: Option<Vec<DetectionWarning>>,
    pub health_check: Option<HealthCheckConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub id: EntityId,
    pub workspace_id: EntityId,
    pub group_id: EntityId,
    pub name: String,
    pub path: String,
    pub detected_type: Option<DetectedProjectType>,
    pub color: Option<String>,
    pub package_manager: Option<ProjectPackageManager>,
    pub executable: Option<String>,
    pub command: Option<String>,
    pub args: Option<CommandArgs>,
    pub env: Option<EnvironmentVariables>,
    pub working_dir: Option<String>,
    pub detection_confidence: Option<f64>,
    pub detection_evidence: Option<Vec<DetectionEvidence>>,
    pub warnings: Option<Vec<DetectionWarning>>,
    pub health_check: Option<HealthCheckConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectHealthCheckInput {
    pub project_id: EntityId,
    pub health_check: Option<HealthCheckConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProjectRunHistoryInput {
    pub project_id: EntityId,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorkspaceRunHistoryInput {
    pub workspace_id: EntityId,
    pub limit: Option<usize>,
}

#[tauri::command]
pub fn create_workspace(
    input: CreateWorkspaceInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<Workspace> {
    let repository = WorkspaceRepository::new(database.inner().clone());
    let timestamp = timestamps::now_iso().map_err(|error| error.to_string())?;
    let workspace = Workspace {
        id: ids::workspace_id(),
        name: input.name.trim().to_owned(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    repository
        .create(&workspace)
        .map_err(|error| error.to_string())?;

    Ok(workspace)
}

#[tauri::command]
pub fn list_workspaces(database: State<'_, AppDatabase>) -> CommandResult<Vec<Workspace>> {
    WorkspaceRepository::new(database.inner().clone())
        .list()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_workspace_tree(
    input: GetWorkspaceTreeInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<WorkspaceTree> {
    WorkspaceTreeRepository::new(database.inner().clone())
        .get(&input.workspace_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Workspace '{}' not found", input.workspace_id))
}

#[tauri::command]
pub fn analyze_project_folder(input: AnalyzeProjectFolderInput) -> CommandResult<DetectionResult> {
    detection::analyze_project_folder(&input.path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn validate_project_command(
    input: ValidateProjectCommandInput,
) -> CommandResult<CommandValidation> {
    Ok(detection::validate_command(
        &input.path,
        input.working_dir.as_deref(),
        input.executable.as_deref(),
        input.args.as_deref().unwrap_or(&[]),
    ))
}

#[tauri::command]
pub fn get_project_git_info(input: GetProjectGitInfoInput) -> CommandResult<ProjectGitInfo> {
    Ok(read_project_git_info(Path::new(&input.path)))
}

#[tauri::command]
pub fn get_project_logs(
    input: GetProjectLogsInput,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<Vec<RuntimeLogLine>> {
    Ok(process_manager.get_logs(&input.project_id))
}

#[tauri::command]
pub fn get_project_health_status(
    input: ProjectRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<ProjectHealthState> {
    let repository = ProjectRepository::new(database.inner().clone());
    let project = repository
        .find_by_id(&input.project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.project_id))?;

    Ok(process_manager.project_health_state(&project))
}

#[tauri::command]
pub fn refresh_project_health(
    input: ProjectRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<ProjectHealthState> {
    let repository = ProjectRepository::new(database.inner().clone());
    let project = repository
        .find_by_id(&input.project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.project_id))?;

    process_manager
        .refresh_project_health(&project)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_project_health_check(
    input: UpdateProjectHealthCheckInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<ProjectHealthState> {
    let repository = ProjectRepository::new(database.inner().clone());
    let timestamp = timestamps::now_iso().map_err(|error| error.to_string())?;
    let health_check = input.health_check.map(|config| config.normalized());

    let was_updated = repository
        .update_health_check(&input.project_id, &health_check, &timestamp)
        .map_err(|error| error.to_string())?;
    if !was_updated {
        return Err(format!("Project '{}' not found", input.project_id));
    }

    let updated_project = repository
        .find_by_id(&input.project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.project_id))?;

    Ok(process_manager.update_project_health_check(&updated_project))
}

#[tauri::command]
pub fn list_project_run_history(
    input: ListProjectRunHistoryInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<Vec<RunHistoryEntry>> {
    RunHistoryRepository::new(database.inner().clone())
        .list_by_project(&input.project_id, input.limit.unwrap_or(20))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_workspace_run_history(
    input: ListWorkspaceRunHistoryInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<Vec<RunHistoryEntry>> {
    RunHistoryRepository::new(database.inner().clone())
        .list_by_workspace(&input.workspace_id, input.limit.unwrap_or(20))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_workspace_observability_summary(
    input: WorkspaceRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<WorkspaceObservabilitySummary> {
    let projects = ProjectRepository::new(database.inner().clone())
        .list_by_workspace(&input.workspace_id)
        .map_err(|error| error.to_string())?;
    let runtime_projects = projects
        .iter()
        .map(|project| process_manager.project_state(project))
        .collect::<Vec<_>>();
    let health_states = process_manager.project_health_states(&projects);
    let workspace_runtime = WorkspaceRuntimeStatus {
        workspace_id: input.workspace_id,
        status: aggregate_runtime_status(runtime_projects.iter().map(|project| project.status)),
        projects: runtime_projects,
    };

    Ok(build_workspace_observability_summary(
        &workspace_runtime,
        &health_states,
    ))
}

#[tauri::command]
pub fn start_project(
    input: ProjectRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<crate::models::ProcessRuntimeState> {
    let repository = ProjectRepository::new(database.inner().clone());
    let project = repository
        .find_by_id(&input.project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.project_id))?;

    process_manager
        .start_project(&project)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn stop_project(
    input: ProjectRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<crate::models::ProcessRuntimeState> {
    let repository = ProjectRepository::new(database.inner().clone());
    let project = repository
        .find_by_id(&input.project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.project_id))?;

    let process_manager = process_manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || process_manager.stop_project(&project))
        .await
        .map_err(|error| format!("Stop task failed: {error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn restart_project(
    input: ProjectRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<crate::models::ProcessRuntimeState> {
    let repository = ProjectRepository::new(database.inner().clone());
    let project = repository
        .find_by_id(&input.project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.project_id))?;

    let process_manager = process_manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        process_manager.stop_project(&project)?;
        process_manager.start_project(&project)
    })
    .await
    .map_err(|error| format!("Restart task failed: {error}"))?
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_project_runtime_status(
    input: ProjectRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<crate::models::ProcessRuntimeState> {
    let repository = ProjectRepository::new(database.inner().clone());
    let project = repository
        .find_by_id(&input.project_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.project_id))?;

    Ok(process_manager.project_state(&project))
}

#[tauri::command]
pub fn get_workspace_runtime_status(
    input: WorkspaceRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<WorkspaceRuntimeStatus> {
    let projects = ProjectRepository::new(database.inner().clone())
        .list_by_workspace(&input.workspace_id)
        .map_err(|error| error.to_string())?;
    let runtime_projects = projects
        .iter()
        .map(|project| process_manager.project_state(project))
        .collect::<Vec<_>>();

    Ok(WorkspaceRuntimeStatus {
        workspace_id: input.workspace_id,
        status: aggregate_runtime_status(runtime_projects.iter().map(|project| project.status)),
        projects: runtime_projects,
    })
}

#[tauri::command]
pub fn start_group(
    input: GroupRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<RuntimeBulkOperationResult> {
    let group_repository = GroupRepository::new(database.inner().clone());
    let group = group_repository
        .find_by_id(&input.group_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Group '{}' not found", input.group_id))?;
    let groups = group_repository
        .list_by_workspace(&group.workspace_id)
        .map_err(|error| error.to_string())?;
    let projects = ProjectRepository::new(database.inner().clone())
        .list_by_workspace(&group.workspace_id)
        .map_err(|error| error.to_string())?;
    let scoped_projects = filter_projects_for_group(&projects, &groups, &group.id);

    Ok(execute_bulk_start(
        RuntimeOperationScope::Group,
        input.group_id,
        scoped_projects,
        process_manager.inner(),
    ))
}

#[tauri::command]
pub async fn stop_group(
    input: GroupRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<RuntimeBulkOperationResult> {
    let group_repository = GroupRepository::new(database.inner().clone());
    let group = group_repository
        .find_by_id(&input.group_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Group '{}' not found", input.group_id))?;
    let groups = group_repository
        .list_by_workspace(&group.workspace_id)
        .map_err(|error| error.to_string())?;
    let projects = ProjectRepository::new(database.inner().clone())
        .list_by_workspace(&group.workspace_id)
        .map_err(|error| error.to_string())?;
    let scoped_projects = filter_projects_for_group(&projects, &groups, &group.id);

    let process_manager = process_manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        execute_bulk_stop(
            RuntimeOperationScope::Group,
            input.group_id,
            scoped_projects,
            &process_manager,
        )
    })
    .await
    .map_err(|error| format!("Stop group task failed: {error}"))
}

#[tauri::command]
pub fn start_workspace(
    input: WorkspaceRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<RuntimeBulkOperationResult> {
    let projects = ProjectRepository::new(database.inner().clone())
        .list_by_workspace(&input.workspace_id)
        .map_err(|error| error.to_string())?;

    Ok(execute_bulk_start(
        RuntimeOperationScope::Workspace,
        input.workspace_id,
        projects,
        process_manager.inner(),
    ))
}

#[tauri::command]
pub async fn stop_workspace(
    input: WorkspaceRuntimeInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<RuntimeBulkOperationResult> {
    let projects = ProjectRepository::new(database.inner().clone())
        .list_by_workspace(&input.workspace_id)
        .map_err(|error| error.to_string())?;

    let process_manager = process_manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        execute_bulk_stop(
            RuntimeOperationScope::Workspace,
            input.workspace_id,
            projects,
            &process_manager,
        )
    })
    .await
    .map_err(|error| format!("Stop workspace task failed: {error}"))
}

#[tauri::command]
pub fn create_project_from_detection(
    input: CreateProjectFromDetectionInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<ProjectNode> {
    let repository = ProjectRepository::new(database.inner().clone());
    let timestamp = timestamps::now_iso().map_err(|error| error.to_string())?;
    let project = ProjectNode {
        id: ids::project_id(),
        workspace_id: input.workspace_id,
        group_id: input.group_id,
        name: input.name.trim().to_owned(),
        path: input.path,
        detected_type: Some(input.detected_type),
        color: input.color,
        package_manager: input.package_manager,
        executable: input.executable,
        command: input.command,
        args: input.args,
        env: input.env,
        working_dir: input.working_dir,
        detection_confidence: Some(input.detection_confidence),
        detection_evidence: Some(input.detection_evidence),
        warnings: input.warnings,
        health_check: input.health_check.map(|config| config.normalized()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    repository
        .create(&project)
        .map_err(|error| error.to_string())?;

    Ok(project)
}

#[tauri::command]
pub fn rename_workspace(
    input: RenameWorkspaceInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<Workspace> {
    let repository = WorkspaceRepository::new(database.inner().clone());
    let Some(existing_workspace) = repository
        .find_by_id(&input.id)
        .map_err(|error| error.to_string())?
    else {
        return Err(format!("Workspace '{}' not found", input.id));
    };
    let updated_workspace = Workspace {
        id: existing_workspace.id,
        name: input.name.trim().to_owned(),
        created_at: existing_workspace.created_at,
        updated_at: timestamps::now_iso().map_err(|error| error.to_string())?,
    };

    repository
        .update(&updated_workspace)
        .map_err(|error| error.to_string())?;

    Ok(updated_workspace)
}

#[tauri::command]
pub fn delete_workspace(
    input: DeleteEntityInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<bool> {
    let project_repository = ProjectRepository::new(database.inner().clone());
    let projects = project_repository
        .list_by_workspace(&input.id)
        .map_err(|error| error.to_string())?;

    clear_runtime_for_projects(&projects, process_manager.inner())?;

    WorkspaceRepository::new(database.inner().clone())
        .delete(&input.id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_group(
    input: CreateGroupInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<GroupNode> {
    let repository = GroupRepository::new(database.inner().clone());
    let timestamp = timestamps::now_iso().map_err(|error| error.to_string())?;
    let sort_order = match input.sort_order {
        Some(sort_order) => sort_order,
        None => next_group_sort_order(
            &repository,
            &input.workspace_id,
            input.parent_group_id.as_deref(),
        )
        .map_err(|error| error.to_string())?,
    };
    let group = GroupNode {
        id: ids::group_id(),
        workspace_id: input.workspace_id,
        parent_group_id: input.parent_group_id,
        name: input.name.trim().to_owned(),
        color: input.color,
        sort_order,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    repository
        .create(&group)
        .map_err(|error| error.to_string())?;

    Ok(group)
}

#[tauri::command]
pub fn update_group(
    input: UpdateGroupInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<GroupNode> {
    let repository = GroupRepository::new(database.inner().clone());
    let Some(existing_group) = repository
        .find_by_id(&input.id)
        .map_err(|error| error.to_string())?
    else {
        return Err(format!("Group '{}' not found", input.id));
    };
    let updated_group = GroupNode {
        id: existing_group.id,
        workspace_id: input.workspace_id,
        parent_group_id: input.parent_group_id,
        name: input.name.trim().to_owned(),
        color: input.color,
        sort_order: input.sort_order,
        created_at: existing_group.created_at,
        updated_at: timestamps::now_iso().map_err(|error| error.to_string())?,
    };

    repository
        .update(&updated_group)
        .map_err(|error| error.to_string())?;

    Ok(updated_group)
}

#[tauri::command]
pub fn delete_group(
    input: DeleteEntityInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<bool> {
    let group_repository = GroupRepository::new(database.inner().clone());
    let group = group_repository
        .find_by_id(&input.id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Group '{}' not found", input.id))?;
    let groups = group_repository
        .list_by_workspace(&group.workspace_id)
        .map_err(|error| error.to_string())?;
    let projects = ProjectRepository::new(database.inner().clone())
        .list_by_workspace(&group.workspace_id)
        .map_err(|error| error.to_string())?;
    let scoped_projects = filter_projects_for_group(&projects, &groups, &group.id);

    clear_runtime_for_projects(&scoped_projects, process_manager.inner())?;

    GroupRepository::new(database.inner().clone())
        .delete(&input.id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_project(
    input: CreateProjectInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<ProjectNode> {
    let repository = ProjectRepository::new(database.inner().clone());
    let timestamp = timestamps::now_iso().map_err(|error| error.to_string())?;
    let project = ProjectNode {
        id: ids::project_id(),
        workspace_id: input.workspace_id,
        group_id: input.group_id,
        name: input.name.trim().to_owned(),
        path: input.path,
        detected_type: input.detected_type,
        color: input.color,
        package_manager: input.package_manager,
        executable: input.executable,
        command: input.command,
        args: input.args,
        env: input.env,
        working_dir: input.working_dir,
        detection_confidence: input.detection_confidence,
        detection_evidence: input.detection_evidence,
        warnings: input.warnings,
        health_check: input.health_check.map(|config| config.normalized()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    repository
        .create(&project)
        .map_err(|error| error.to_string())?;

    Ok(project)
}

#[tauri::command]
pub fn update_project(
    input: UpdateProjectInput,
    database: State<'_, AppDatabase>,
) -> CommandResult<ProjectNode> {
    let repository = ProjectRepository::new(database.inner().clone());
    let Some(existing_project) = repository
        .find_by_id(&input.id)
        .map_err(|error| error.to_string())?
    else {
        return Err(format!("Project '{}' not found", input.id));
    };
    let updated_project = ProjectNode {
        id: existing_project.id,
        workspace_id: input.workspace_id,
        group_id: input.group_id,
        name: input.name.trim().to_owned(),
        path: input.path,
        detected_type: input.detected_type,
        color: input.color,
        package_manager: input.package_manager,
        executable: input.executable,
        command: input.command,
        args: input.args,
        env: input.env,
        working_dir: input.working_dir,
        detection_confidence: input.detection_confidence,
        detection_evidence: input.detection_evidence,
        warnings: input.warnings,
        health_check: input.health_check.map(|config| config.normalized()),
        created_at: existing_project.created_at,
        updated_at: timestamps::now_iso().map_err(|error| error.to_string())?,
    };

    repository
        .update(&updated_project)
        .map_err(|error| error.to_string())?;

    Ok(updated_project)
}

#[tauri::command]
pub fn delete_project(
    input: DeleteEntityInput,
    database: State<'_, AppDatabase>,
    process_manager: State<'_, ProcessManager>,
) -> CommandResult<bool> {
    let repository = ProjectRepository::new(database.inner().clone());
    let project = repository
        .find_by_id(&input.id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", input.id))?;

    clear_runtime_for_projects(&[project], process_manager.inner())?;

    repository
        .delete(&input.id)
        .map_err(|error| error.to_string())
}

fn next_group_sort_order(
    repository: &GroupRepository,
    workspace_id: &EntityId,
    parent_group_id: Option<&str>,
) -> CommandResult<i64> {
    let groups = repository
        .list_by_workspace(workspace_id)
        .map_err(|error| error.to_string())?;

    Ok(groups
        .into_iter()
        .filter(|group| group.parent_group_id.as_deref() == parent_group_id)
        .map(|group| group.sort_order)
        .max()
        .unwrap_or(0)
        + 10)
}

fn aggregate_runtime_status<I>(statuses: I) -> RuntimeStatus
where
    I: IntoIterator<Item = RuntimeStatus>,
{
    let statuses = statuses.into_iter().collect::<Vec<_>>();
    if statuses.is_empty() {
        return RuntimeStatus::Stopped;
    }

    if statuses
        .iter()
        .all(|status| *status == RuntimeStatus::Stopped)
    {
        return RuntimeStatus::Stopped;
    }
    if statuses.contains(&RuntimeStatus::Stopping) {
        return RuntimeStatus::Stopping;
    }
    if statuses.contains(&RuntimeStatus::Starting) {
        return RuntimeStatus::Starting;
    }
    if statuses.contains(&RuntimeStatus::Failed) {
        return RuntimeStatus::Failed;
    }
    if statuses.contains(&RuntimeStatus::Running) {
        return RuntimeStatus::Running;
    }

    RuntimeStatus::Stopped
}

fn filter_projects_for_group(
    projects: &[ProjectNode],
    groups: &[GroupNode],
    group_id: &EntityId,
) -> Vec<ProjectNode> {
    let group_ids = descendant_group_ids(groups, group_id);
    projects
        .iter()
        .filter(|project| group_ids.contains(&project.group_id))
        .cloned()
        .collect()
}

fn descendant_group_ids(groups: &[GroupNode], root_group_id: &EntityId) -> Vec<EntityId> {
    let mut pending = vec![root_group_id.clone()];
    let mut descendants = Vec::new();

    while let Some(current_group_id) = pending.pop() {
        descendants.push(current_group_id.clone());

        for group in groups
            .iter()
            .filter(|group| group.parent_group_id.as_ref() == Some(&current_group_id))
        {
            pending.push(group.id.clone());
        }
    }

    descendants
}

fn execute_bulk_start(
    scope: RuntimeOperationScope,
    target_id: EntityId,
    projects: Vec<ProjectNode>,
    process_manager: &ProcessManager,
) -> RuntimeBulkOperationResult {
    let mut affected_project_ids = Vec::new();
    let mut skipped_project_ids = Vec::new();
    let mut failures = Vec::new();
    let requested_project_ids = projects
        .iter()
        .map(|project| project.id.clone())
        .collect::<Vec<_>>();

    for project in projects {
        let current_state = process_manager.project_state(&project);
        if matches!(
            current_state.status,
            RuntimeStatus::Running | RuntimeStatus::Starting
        ) {
            skipped_project_ids.push(project.id.clone());
            continue;
        }

        match process_manager.start_project(&project) {
            Ok(_) => affected_project_ids.push(project.id.clone()),
            Err(error) => failures.push(RuntimeBulkFailure {
                project_id: project.id.clone(),
                message: error.to_string(),
            }),
        }
    }

    RuntimeBulkOperationResult {
        scope,
        target_id,
        status: if failures.is_empty() {
            RuntimeStatus::Running
        } else if affected_project_ids.is_empty() {
            RuntimeStatus::Failed
        } else {
            RuntimeStatus::Starting
        },
        requested_project_ids,
        affected_project_ids,
        skipped_project_ids,
        failures,
    }
}

fn execute_bulk_stop(
    scope: RuntimeOperationScope,
    target_id: EntityId,
    projects: Vec<ProjectNode>,
    process_manager: &ProcessManager,
) -> RuntimeBulkOperationResult {
    let mut skipped_project_ids = Vec::new();
    let mut failures = Vec::new();
    let mut stop_candidates = Vec::new();
    let requested_project_ids = projects
        .iter()
        .map(|project| project.id.clone())
        .collect::<Vec<_>>();

    for project in projects {
        let current_state = process_manager.project_state(&project);
        if matches!(
            current_state.status,
            RuntimeStatus::Stopped | RuntimeStatus::Failed
        ) && current_state.pid.is_none()
        {
            skipped_project_ids.push(project.id.clone());
            continue;
        }

        stop_candidates.push(project);
    }

    let mut affected_project_ids = stop_candidates
        .iter()
        .map(|project| project.id.clone())
        .collect::<Vec<_>>();

    for chunk in stop_candidates.chunks(BULK_STOP_CONCURRENCY) {
        let (sender, receiver) = mpsc::channel();
        thread::scope(|scope| {
            for project in chunk {
                let sender = sender.clone();
                scope.spawn(move || {
                    let result = process_manager
                        .stop_project(project)
                        .map(|_| ())
                        .map_err(|error| error.to_string());
                    let _ = sender.send((project.id.clone(), result));
                });
            }
        });
        drop(sender);

        for (project_id, result) in receiver {
            if let Err(message) = result {
                failures.push(RuntimeBulkFailure {
                    project_id: project_id.clone(),
                    message,
                });
                affected_project_ids.retain(|affected_id| affected_id != &project_id);
            }
        }
    }

    RuntimeBulkOperationResult {
        scope,
        target_id,
        status: if failures.is_empty() {
            RuntimeStatus::Stopped
        } else if affected_project_ids.is_empty() {
            RuntimeStatus::Failed
        } else {
            RuntimeStatus::Stopping
        },
        requested_project_ids,
        affected_project_ids,
        skipped_project_ids,
        failures,
    }
}

fn clear_runtime_for_projects(
    projects: &[ProjectNode],
    process_manager: &ProcessManager,
) -> CommandResult<()> {
    for project in projects {
        process_manager
            .remove_project_runtime(project)
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn read_project_git_info(path: &Path) -> ProjectGitInfo {
    let Some(git_dir) = find_git_dir(path) else {
        return ProjectGitInfo {
            is_repository: false,
            branch: None,
        };
    };

    ProjectGitInfo {
        is_repository: true,
        branch: read_git_branch(&git_dir),
    }
}

fn find_git_dir(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };

    loop {
        let dot_git = current.join(".git");

        if dot_git.is_dir() {
            return Some(dot_git);
        }

        if dot_git.is_file() {
            return resolve_gitdir_file(&dot_git, &current);
        }

        if !current.pop() {
            return None;
        }
    }
}

fn resolve_gitdir_file(dot_git: &Path, work_tree: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(dot_git).ok()?;
    let git_dir = content.trim().strip_prefix("gitdir:")?.trim();
    let git_dir_path = PathBuf::from(git_dir);

    Some(if git_dir_path.is_absolute() {
        git_dir_path
    } else {
        work_tree.join(git_dir_path)
    })
}

fn read_git_branch(git_dir: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let reference = head.trim().strip_prefix("ref: refs/heads/")?.trim();

    if reference.is_empty() {
        None
    } else {
        Some(reference.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{read_project_git_info, ProjectGitInfo};

    fn temp_project_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("centralita-{}-{}", name, uuid::Uuid::now_v7()))
    }

    #[test]
    fn reads_branch_from_git_head() {
        let project_dir = temp_project_path("git-branch");
        let git_dir = project_dir.join(".git");
        fs::create_dir_all(&git_dir).expect("git dir should be created");
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature/detail\n")
            .expect("HEAD should be written");

        let info = read_project_git_info(&project_dir);

        assert_eq!(
            info,
            ProjectGitInfo {
                is_repository: true,
                branch: Some("feature/detail".into()),
            }
        );

        fs::remove_dir_all(&project_dir).expect("temp project should be removed");
    }

    #[test]
    fn returns_no_repository_when_git_metadata_is_absent() {
        let project_dir = temp_project_path("no-git");
        fs::create_dir_all(&project_dir).expect("project dir should be created");

        let info = read_project_git_info(&project_dir);

        assert_eq!(
            info,
            ProjectGitInfo {
                is_repository: false,
                branch: None,
            }
        );

        fs::remove_dir_all(&project_dir).expect("temp project should be removed");
    }
}
