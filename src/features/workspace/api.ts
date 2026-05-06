import { invokeCommand, TAURI_COMMANDS } from '../../shared/api/tauri'
import type {
  AnalyzeProjectFolderInput,
  CommandValidation,
  CreateProjectFromDetectionInput,
  CreateGroupInput,
  CreateProjectInput,
  CreateWorkspaceInput,
  DetectionResult,
  DeleteEntityInput,
  GetProjectGitInfoInput,
  GroupNode,
  ProjectGitInfo,
  ProjectNode,
  RenameWorkspaceInput,
  UpdateGroupInput,
  ValidateProjectCommandInput,
  UpdateProjectInput,
  Workspace,
  WorkspaceTree,
} from '../../types'

export function listWorkspaces() {
  return invokeCommand<Workspace[]>(TAURI_COMMANDS.listWorkspaces)
}

export function createWorkspace(input: CreateWorkspaceInput) {
  return invokeCommand<Workspace>(TAURI_COMMANDS.createWorkspace, { input })
}

export function renameWorkspace(input: RenameWorkspaceInput) {
  return invokeCommand<Workspace>(TAURI_COMMANDS.renameWorkspace, { input })
}

export function deleteWorkspace(input: DeleteEntityInput) {
  return invokeCommand<boolean>(TAURI_COMMANDS.deleteWorkspace, { input })
}

export function getWorkspaceTree(workspaceId: string) {
  return invokeCommand<WorkspaceTree>(TAURI_COMMANDS.getWorkspaceTree, {
    input: { workspaceId },
  })
}

export function analyzeProjectFolder(input: AnalyzeProjectFolderInput) {
  return invokeCommand<DetectionResult>(TAURI_COMMANDS.analyzeProjectFolder, {
    input,
  })
}

export function validateProjectCommand(input: ValidateProjectCommandInput) {
  return invokeCommand<CommandValidation>(
    TAURI_COMMANDS.validateProjectCommand,
    { input },
  )
}

export function getProjectGitInfo(input: GetProjectGitInfoInput) {
  return invokeCommand<ProjectGitInfo>(TAURI_COMMANDS.getProjectGitInfo, {
    input,
  })
}

export function createProjectFromDetection(
  input: CreateProjectFromDetectionInput,
) {
  return invokeCommand<ProjectNode>(TAURI_COMMANDS.createProjectFromDetection, {
    input,
  })
}

export function createGroup(input: CreateGroupInput) {
  return invokeCommand<GroupNode>(TAURI_COMMANDS.createGroup, { input })
}

export function updateGroup(input: UpdateGroupInput) {
  return invokeCommand<GroupNode>(TAURI_COMMANDS.updateGroup, { input })
}

export function deleteGroup(input: DeleteEntityInput) {
  return invokeCommand<boolean>(TAURI_COMMANDS.deleteGroup, { input })
}

export function createProject(input: CreateProjectInput) {
  return invokeCommand<ProjectNode>(TAURI_COMMANDS.createProject, { input })
}

export function updateProject(input: UpdateProjectInput) {
  return invokeCommand<ProjectNode>(TAURI_COMMANDS.updateProject, { input })
}

export function deleteProject(input: DeleteEntityInput) {
  return invokeCommand<boolean>(TAURI_COMMANDS.deleteProject, { input })
}
