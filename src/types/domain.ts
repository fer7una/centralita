export type EntityId = string
export type IsoDateTime = string
export type CommandArgs = string[]
export type EnvironmentVariables = Record<string, string>

export type DetectedProjectType =
  | 'javaMaven'
  | 'javaGradle'
  | 'springBootMaven'
  | 'springBootGradle'
  | 'javaJar'
  | 'nodeGeneric'
  | 'vite'
  | 'reactVite'
  | 'nextJs'
  | 'express'
  | 'custom'
  | 'unknown'

export type ProjectPackageManager = 'npm' | 'pnpm' | 'yarn' | 'maven' | 'gradle'
export type RuntimeStatus =
  | 'STOPPED'
  | 'STARTING'
  | 'RUNNING'
  | 'STOPPING'
  | 'FAILED'
export type RuntimeLogStream = 'stdout' | 'stderr'

export type DetectionEvidenceKind =
  | 'structuralFile'
  | 'manifest'
  | 'config'
  | 'dependency'
  | 'plugin'
  | 'script'
  | 'lockfile'
  | 'wrapper'
  | 'artifact'
  | 'entryPoint'
  | 'workspace'
  | 'fallback'

export interface DetectionEvidence {
  kind: DetectionEvidenceKind
  source: string
  detail: string
  weight: number
}

export interface DetectionWarning {
  code: string
  message: string
  source: string | null
}

export interface CommandValidation {
  isRunnable: boolean
  commandPreview: string
  resolvedExecutable: string | null
  issues: string[]
}

export interface RunRequest {
  projectId: EntityId
  executable: string
  args: CommandArgs
  workingDir: string
  env?: EnvironmentVariables | null
}

export interface ProcessRuntimeState {
  projectId: EntityId
  status: RuntimeStatus
  pid?: number | null
  startedAt?: IsoDateTime | null
  stoppedAt?: IsoDateTime | null
  exitCode?: number | null
  lastError?: string | null
  commandPreview: string
}

export interface RunHistoryEntry {
  id: EntityId
  projectId: EntityId
  startedAt: IsoDateTime
  endedAt?: IsoDateTime | null
  exitCode?: number | null
  finalRuntimeStatus: RuntimeStatus
  stopReason?: string | null
  errorMessage?: string | null
  commandPreview: string
}

export interface WorkspaceRuntimeStatusCounts {
  stopped: number
  starting: number
  running: number
  stopping: number
  failed: number
}

export interface WorkspaceObservabilitySummary {
  workspaceId: EntityId
  totalProjects: number
  runtimeStatus: RuntimeStatus
  runtimeCounts: WorkspaceRuntimeStatusCounts
}

export interface WorkspaceRuntimeStatus {
  workspaceId: EntityId
  status: RuntimeStatus
  projects: ProcessRuntimeState[]
}

export interface RuntimeLogLine {
  projectId: EntityId
  stream: RuntimeLogStream
  line: string
  partial?: boolean
  timestamp: IsoDateTime
}

export interface RuntimeStatusEvent {
  projectId: EntityId
  status: RuntimeStatus
  pid?: number | null
  timestamp: IsoDateTime
  message?: string | null
  commandPreview: string
}

export interface RuntimeProcessExitedEvent {
  projectId: EntityId
  status: RuntimeStatus
  pid?: number | null
  timestamp: IsoDateTime
  exitCode?: number | null
  message?: string | null
  commandPreview: string
}

export interface RuntimeProcessErrorEvent {
  projectId: EntityId
  status: RuntimeStatus
  pid?: number | null
  timestamp: IsoDateTime
  message: string
  commandPreview: string
}

export type RuntimeOperationScope = 'project' | 'group' | 'workspace'

export interface RuntimeBulkFailure {
  projectId: EntityId
  message: string
}

export interface RuntimeBulkOperationResult {
  scope: RuntimeOperationScope
  targetId: EntityId
  status: RuntimeStatus
  requestedProjectIds: EntityId[]
  affectedProjectIds: EntityId[]
  skippedProjectIds: EntityId[]
  failures: RuntimeBulkFailure[]
}

export type RuntimeEvent =
  | { type: 'projectStarting'; payload: RuntimeStatusEvent }
  | { type: 'projectStarted'; payload: RuntimeStatusEvent }
  | { type: 'projectStopping'; payload: RuntimeStatusEvent }
  | { type: 'projectStopped'; payload: RuntimeProcessExitedEvent }
  | { type: 'projectFailed'; payload: RuntimeProcessErrorEvent }
  | { type: 'projectLogLine'; payload: RuntimeLogLine }

export interface Workspace {
  id: EntityId
  name: string
  createdAt: IsoDateTime
  updatedAt: IsoDateTime
}

export interface GroupNode {
  id: EntityId
  workspaceId: EntityId
  parentGroupId: EntityId | null
  name: string
  color: string
  sortOrder: number
  createdAt: IsoDateTime
  updatedAt: IsoDateTime
}

export interface ProjectNode {
  id: EntityId
  workspaceId: EntityId
  groupId: EntityId
  name: string
  path: string
  detectedType: DetectedProjectType | null
  color: string | null
  packageManager?: ProjectPackageManager | null
  executable?: string | null
  command: string | null
  args?: CommandArgs
  env?: EnvironmentVariables
  workingDir: string | null
  detectionConfidence?: number | null
  detectionEvidence?: DetectionEvidence[] | null
  warnings?: DetectionWarning[] | null
  createdAt: IsoDateTime
  updatedAt: IsoDateTime
}

export interface ProjectGitInfo {
  isRepository: boolean
  branch: string | null
}

export interface GroupTreeNode extends GroupNode {
  groups: GroupTreeNode[]
  projects: ProjectNode[]
}

export interface WorkspaceTree {
  workspace: Workspace
  groups: GroupTreeNode[]
}

export interface CreateWorkspaceInput {
  name: string
}

export interface RenameWorkspaceInput {
  id: EntityId
  name: string
}

export interface DeleteEntityInput {
  id: EntityId
}

export interface GetWorkspaceTreeInput {
  workspaceId: EntityId
}

export interface CreateGroupInput {
  workspaceId: EntityId
  parentGroupId: EntityId | null
  name: string
  color: string
  sortOrder?: number
}

export interface UpdateGroupInput {
  id: EntityId
  workspaceId: EntityId
  parentGroupId: EntityId | null
  name: string
  color: string
  sortOrder: number
}

export interface CreateProjectInput {
  workspaceId: EntityId
  groupId: EntityId
  name: string
  path: string
  detectedType: DetectedProjectType | null
  color: string | null
  packageManager?: ProjectPackageManager | null
  executable?: string | null
  command: string | null
  args?: CommandArgs
  env?: EnvironmentVariables
  workingDir: string | null
  detectionConfidence?: number | null
  detectionEvidence?: DetectionEvidence[] | null
  warnings?: DetectionWarning[] | null
}

export interface UpdateProjectInput {
  id: EntityId
  workspaceId: EntityId
  groupId: EntityId
  name: string
  path: string
  detectedType: DetectedProjectType | null
  color: string | null
  packageManager?: ProjectPackageManager | null
  executable?: string | null
  command: string | null
  args?: CommandArgs
  env?: EnvironmentVariables
  workingDir: string | null
  detectionConfidence?: number | null
  detectionEvidence?: DetectionEvidence[] | null
  warnings?: DetectionWarning[] | null
}

export interface DetectionResult {
  detectedType: DetectedProjectType
  displayName: string
  path: string
  workingDir: string | null
  packageManager: ProjectPackageManager | null
  executable: string | null
  command: string | null
  args: CommandArgs
  commandPreview: string
  commandValidation: CommandValidation
  confidence: number
  evidence: DetectionEvidence[]
  warnings: DetectionWarning[]
}

export interface AnalyzeProjectFolderInput {
  path: string
}

export interface ValidateProjectCommandInput {
  path: string
  executable?: string | null
  args?: CommandArgs
  workingDir?: string | null
}

export interface GetProjectGitInfoInput {
  path: string
}

export interface GetProjectLogsInput {
  projectId: EntityId
}

export interface ProjectRuntimeInput {
  projectId: EntityId
}

export interface WorkspaceRuntimeInput {
  workspaceId: EntityId
}

export interface GroupRuntimeInput {
  groupId: EntityId
}

export interface CreateProjectFromDetectionInput {
  workspaceId: EntityId
  groupId: EntityId
  name: string
  path: string
  detectedType: DetectedProjectType
  color: string | null
  packageManager: ProjectPackageManager | null
  executable: string | null
  command: string | null
  args?: CommandArgs
  env?: EnvironmentVariables
  workingDir: string | null
  detectionConfidence: number
  detectionEvidence: DetectionEvidence[]
  warnings?: DetectionWarning[]
}

export interface ListProjectRunHistoryInput {
  projectId: EntityId
  limit?: number
}

export interface ListWorkspaceRunHistoryInput {
  workspaceId: EntityId
  limit?: number
}
