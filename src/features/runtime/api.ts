import { invokeCommand, TAURI_COMMANDS } from '../../shared/api/tauri'
import type {
  GroupRuntimeInput,
  GetProjectLogsInput,
  ProcessRuntimeState,
  ProjectRuntimeInput,
  RunHistoryEntry,
  RuntimeBulkOperationResult,
  RuntimeLogLine,
  WorkspaceObservabilitySummary,
  WorkspaceRuntimeInput,
  WorkspaceRuntimeStatus,
  ListProjectRunHistoryInput,
  ListWorkspaceRunHistoryInput,
} from '../../types'

export function startProject(input: ProjectRuntimeInput) {
  return invokeCommand<ProcessRuntimeState>(TAURI_COMMANDS.startProject, {
    input,
  })
}

export function stopProject(input: ProjectRuntimeInput) {
  return invokeCommand<ProcessRuntimeState>(TAURI_COMMANDS.stopProject, {
    input,
  })
}

export function restartProject(input: ProjectRuntimeInput) {
  return invokeCommand<ProcessRuntimeState>(TAURI_COMMANDS.restartProject, {
    input,
  })
}

export function getProjectRuntimeStatus(input: ProjectRuntimeInput) {
  return invokeCommand<ProcessRuntimeState>(
    TAURI_COMMANDS.getProjectRuntimeStatus,
    { input },
  )
}

export function getWorkspaceRuntimeStatus(input: WorkspaceRuntimeInput) {
  return invokeCommand<WorkspaceRuntimeStatus>(
    TAURI_COMMANDS.getWorkspaceRuntimeStatus,
    { input },
  )
}

export function getProjectLogs(input: GetProjectLogsInput) {
  return invokeCommand<RuntimeLogLine[]>(TAURI_COMMANDS.getProjectLogs, {
    input,
  })
}

export function listProjectRunHistory(input: ListProjectRunHistoryInput) {
  return invokeCommand<RunHistoryEntry[]>(
    TAURI_COMMANDS.listProjectRunHistory,
    { input },
  )
}

export function listWorkspaceRunHistory(input: ListWorkspaceRunHistoryInput) {
  return invokeCommand<RunHistoryEntry[]>(
    TAURI_COMMANDS.listWorkspaceRunHistory,
    { input },
  )
}

export function getWorkspaceObservabilitySummary(input: WorkspaceRuntimeInput) {
  return invokeCommand<WorkspaceObservabilitySummary>(
    TAURI_COMMANDS.getWorkspaceObservabilitySummary,
    { input },
  )
}

export function startGroup(input: GroupRuntimeInput) {
  return invokeCommand<RuntimeBulkOperationResult>(TAURI_COMMANDS.startGroup, {
    input,
  })
}

export function stopGroup(input: GroupRuntimeInput) {
  return invokeCommand<RuntimeBulkOperationResult>(TAURI_COMMANDS.stopGroup, {
    input,
  })
}

export function startWorkspace(input: WorkspaceRuntimeInput) {
  return invokeCommand<RuntimeBulkOperationResult>(
    TAURI_COMMANDS.startWorkspace,
    { input },
  )
}

export function stopWorkspace(input: WorkspaceRuntimeInput) {
  return invokeCommand<RuntimeBulkOperationResult>(
    TAURI_COMMANDS.stopWorkspace,
    { input },
  )
}
