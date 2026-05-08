import { invoke as tauriInvoke } from '@tauri-apps/api/core'

export const TAURI_COMMANDS = {
  analyzeProjectFolder: 'analyze_project_folder',
  createGroup: 'create_group',
  createProject: 'create_project',
  createProjectFromDetection: 'create_project_from_detection',
  createWorkspace: 'create_workspace',
  deleteGroup: 'delete_group',
  deleteProject: 'delete_project',
  deleteWorkspace: 'delete_workspace',
  getProjectGitInfo: 'get_project_git_info',
  getProjectLogs: 'get_project_logs',
  getProjectRuntimeStatus: 'get_project_runtime_status',
  getWorkspaceObservabilitySummary: 'get_workspace_observability_summary',
  getWorkspaceRuntimeStatus: 'get_workspace_runtime_status',
  getWorkspaceTree: 'get_workspace_tree',
  listProjectRunHistory: 'list_project_run_history',
  listWorkspaceRunHistory: 'list_workspace_run_history',
  listWorkspaces: 'list_workspaces',
  renameWorkspace: 'rename_workspace',
  restartProject: 'restart_project',
  startGroup: 'start_group',
  startProject: 'start_project',
  startWorkspace: 'start_workspace',
  stopGroup: 'stop_group',
  stopProject: 'stop_project',
  stopWorkspace: 'stop_workspace',
  updateGroup: 'update_group',
  updateProject: 'update_project',
  validateProjectCommand: 'validate_project_command',
} as const

export type TauriCommandName =
  (typeof TAURI_COMMANDS)[keyof typeof TAURI_COMMANDS]

export function invokeCommand<Response>(
  command: TauriCommandName,
  args?: Record<string, unknown>,
) {
  return tauriInvoke<Response>(command, args)
}
