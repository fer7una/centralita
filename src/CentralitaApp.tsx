import { open } from '@tauri-apps/plugin-dialog'
import {
  Activity,
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  CheckCircle2,
  CircleDashed,
  Clock,
  Eye,
  FolderPlus,
  GitBranch,
  Hash,
  Loader2,
  MinusCircle,
  Play,
  Plus,
  RotateCcw,
  Settings2,
  Square,
  Terminal as TerminalIcon,
  Trash2,
  XCircle,
} from 'lucide-react'
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import type {
  ComponentType,
  CSSProperties,
  KeyboardEvent as ReactKeyboardEvent,
  PointerEvent as ReactPointerEvent,
  ReactNode,
  SVGProps,
} from 'react'
import { BlockedActionButton } from './components/BlockedActionButton'
import { ModalFrame } from './components/ModalFrame'
import { ProjectLogsPanel } from './components/logs/ProjectLogsPanel'
import { RuntimeStatusBadge } from './components/runtime/RuntimeStatusBadge'
import {
  WorkspaceRuntimeTreeView,
  type NavigatorDragItem,
  type NavigatorDropTarget,
} from './components/runtime/WorkspaceRuntimeTreeView'
import {
  findGroup,
  findProject,
  flattenGroups,
  flattenProjects,
  groupNameExists,
  groupRuntimeStatus,
} from './features/workspace/tree'
import {
  getProjectGitInfo,
  validateProjectCommand,
} from './features/workspace/api'
import { useRuntimeStore } from './store/useRuntimeStore'
import { useWorkspaceStore } from './store/useWorkspaceStore'
import type {
  CommandValidation,
  DetectionResult,
  DetectedProjectType,
  GroupTreeNode,
  ProcessRuntimeState,
  ProjectGitInfo,
  ProjectNode,
  ProjectPackageManager,
  RunHistoryEntry,
  RuntimeStatus,
} from './types'

type LucideIcon = ComponentType<SVGProps<SVGSVGElement> & { size?: number }>
type MetricTone = 'running' | 'warning' | 'error' | 'info'

const detectedTypeOptions: Array<{
  label: string
  value: DetectedProjectType
}> = [
  { label: 'Custom', value: 'custom' },
  { label: 'Unknown', value: 'unknown' },
  { label: 'Vite', value: 'vite' },
  { label: 'React/Vite', value: 'reactVite' },
  { label: 'Next.js', value: 'nextJs' },
  { label: 'Express', value: 'express' },
  { label: 'Node', value: 'nodeGeneric' },
  { label: 'Spring Boot Maven', value: 'springBootMaven' },
  { label: 'Spring Boot Gradle', value: 'springBootGradle' },
  { label: 'Java Maven', value: 'javaMaven' },
  { label: 'Java Gradle', value: 'javaGradle' },
  { label: 'Java JAR', value: 'javaJar' },
]

const packageManagerOptions: Array<{
  label: string
  value: ProjectPackageManager
}> = [
  { label: 'npm', value: 'npm' },
  { label: 'pnpm', value: 'pnpm' },
  { label: 'yarn', value: 'yarn' },
  { label: 'Maven', value: 'maven' },
  { label: 'Gradle', value: 'gradle' },
]

const DEFAULT_GROUP_COLOR = '#2f855a'
const NAVIGATOR_WIDTH_STORAGE_KEY = 'centralita:navigator-width'
const NAVIGATOR_DETAIL_MIN_WIDTH_PX = 360
const NAVIGATOR_MIN_WIDTH_RATIO = 0.2
const NAVIGATOR_MAX_WIDTH_RATIO = 0.6
const NAVIGATOR_RESIZE_RESERVED_WIDTH_PX = 20
const NAVIGATOR_RESIZE_STEP_PX = 16
const NAVIGATOR_RESIZE_LARGE_STEP_PX = 40
const MAX_NAVIGATION_HISTORY_ENTRIES = 100

type DetectionReviewDraft = {
  argsText: string
  confidence: number
  detectedType: DetectedProjectType
  evidence: DetectionResult['evidence']
  executable: string
  groupId: string
  name: string
  packageManager: ProjectPackageManager | null
  path: string
  commandValidation: CommandValidation
  warnings: DetectionResult['warnings']
  workingDir: string
}

type TreeSelection =
  | { id: string; type: 'group' }
  | { id: string; type: 'project' }
  | { id: string; type: 'workspace' }
  | null

type NavigationItem = Exclude<TreeSelection, null>

type NavigationEntry = {
  item: NavigationItem
  workspaceId: string
}

type GroupDetailDraft = {
  name: string
}

type ProjectDetailDraft = {
  argsText: string
  executable: string
  name: string
  path: string
  workingDir: string
}

type DeleteModalState = {
  confirmLabel: string
  description: string
  id: string
  kind: 'group' | 'project' | 'workspace'
  title: string
}

type MoveModalState = {
  description: string
  source: NavigatorDragItem
  target: NavigatorDropTarget
  title: string
}

type ActionModalState =
  | 'createGroup'
  | 'editGroup'
  | 'editProject'
  | 'importProject'
  | 'renameWorkspace'
  | null

type ProjectListDisplayMode = 'STATUS' | 'ALL'
type ProjectListGroupMode = 'NONE' | 'GROUPS'
type ProjectListSortMode = 'NAME_ASC' | 'NAME_DESC' | 'STATUS'

type RuntimeErrorHistoryItem = {
  commandPreview: string
  id: string
  message: string
  occurredAt: string | null
  source: 'current' | 'history'
}

type ProjectGroupSection = {
  group: GroupTreeNode | null
  label: string
  projects: ProjectNode[]
}

const projectStatusFilterOptions: Array<{
  label: string
  value: RuntimeStatus
}> = [
  { label: 'RUNNING', value: 'RUNNING' },
  { label: 'ERROR', value: 'FAILED' },
  { label: 'STARTING', value: 'STARTING' },
  { label: 'STOPPING', value: 'STOPPING' },
  { label: 'STOPPED', value: 'STOPPED' },
]

const projectStatusSortOrder: Record<RuntimeStatus, number> = {
  RUNNING: 0,
  FAILED: 1,
  STARTING: 2,
  STOPPING: 3,
  STOPPED: 4,
}

const projectNameCollator = new Intl.Collator('es', {
  numeric: true,
  sensitivity: 'base',
})

function normalizeWindowsPath(path: string) {
  if (path.startsWith('\\\\?\\UNC\\')) {
    return `\\\\${path.slice('\\\\?\\UNC\\'.length)}`
  }

  if (path.startsWith('\\\\?\\')) {
    return path.slice('\\\\?\\'.length)
  }

  return path
}

function cleanPathInput(path: string) {
  return normalizeWindowsPath(path.trim())
}

function rootDirectoryName(path: string) {
  const normalizedPath = cleanPathInput(path).replace(/[\\/]+$/, '')
  const segments = normalizedPath.split(/[\\/]/).filter(Boolean)

  return segments.at(-1) ?? ''
}

function createReviewDraft(
  result: DetectionResult,
  defaultGroupId: string,
): DetectionReviewDraft {
  const initialName = rootDirectoryName(result.path) || result.displayName

  return {
    argsText: result.args.join('\n'),
    confidence: result.confidence,
    detectedType: result.detectedType,
    evidence: result.evidence,
    executable: result.executable ?? '',
    groupId: defaultGroupId,
    name: initialName,
    packageManager: result.packageManager,
    path: normalizeWindowsPath(result.path),
    commandValidation: result.commandValidation,
    warnings: result.warnings,
    workingDir: normalizeWindowsPath(result.workingDir ?? result.path),
  }
}

function createGroupDraft(name: string): GroupDetailDraft {
  return { name }
}

function createProjectDraft(project: ProjectNode): ProjectDetailDraft {
  return {
    argsText: project.args?.join('\n') ?? '',
    executable: project.executable ?? '',
    name: project.name,
    path: normalizeWindowsPath(project.path),
    workingDir: normalizeWindowsPath(project.workingDir ?? project.path),
  }
}

function parseArgsInput(argsText: string) {
  return argsText
    .split('\n')
    .map((value) => value.trim())
    .filter(Boolean)
}

function areStringArraysEqual(left: string[], right: string[]) {
  return (
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  )
}

function clampNumber(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}

function readStoredNavigatorWidth() {
  try {
    const storedValue = window.localStorage.getItem(NAVIGATOR_WIDTH_STORAGE_KEY)
    const parsedValue = storedValue ? Number(storedValue) : Number.NaN

    return Number.isFinite(parsedValue) && parsedValue > 0
      ? Math.round(parsedValue)
      : null
  } catch {
    return null
  }
}

function writeStoredNavigatorWidth(width: number) {
  try {
    window.localStorage.setItem(
      NAVIGATOR_WIDTH_STORAGE_KEY,
      String(Math.round(width)),
    )
  } catch {
    // localStorage may be unavailable in restricted environments.
  }
}

function getNavigatorMinWidth() {
  return Math.round(window.innerWidth * NAVIGATOR_MIN_WIDTH_RATIO)
}

function formatCommandPreview(executable: string, args: string[]) {
  if (!executable.trim()) {
    return 'Manual review required'
  }

  const serializedArgs = args.map((value) =>
    value.includes(' ') ? `"${value}"` : value,
  )
  return [executable.trim(), ...serializedArgs].join(' ')
}

function confidenceLabel(confidence: number) {
  if (confidence >= 0.8) {
    return 'Alta'
  }
  if (confidence >= 0.55) {
    return 'Media'
  }

  return 'Baja'
}

function formatDateTime(value?: string | null, emptyLabel = 'Sin datos') {
  return value ? new Date(value).toLocaleString() : emptyLabel
}

function projectCommandPreview(
  project: ProjectNode,
  executable: string,
  argsText: string,
) {
  const parsedArgs = parseArgsInput(argsText)
  const nextPreview = formatCommandPreview(executable, parsedArgs)

  return nextPreview === 'Manual review required'
    ? (project.command ?? 'Sin comando derivado')
    : nextPreview
}

function groupHasDescendant(
  group: GroupTreeNode,
  descendantId: string,
): boolean {
  return group.groups.some(
    (childGroup) =>
      childGroup.id === descendantId ||
      groupHasDescendant(childGroup, descendantId),
  )
}

function runtimeStatusIcon(status: RuntimeStatus): LucideIcon {
  switch (status) {
    case 'RUNNING':
      return Activity
    case 'STARTING':
    case 'STOPPING':
      return Loader2
    case 'FAILED':
      return XCircle
    case 'STOPPED':
    default:
      return MinusCircle
  }
}

function runtimeStatusTone(status: RuntimeStatus): MetricTone | undefined {
  switch (status) {
    case 'RUNNING':
      return 'running'
    case 'STARTING':
    case 'STOPPING':
      return 'warning'
    case 'FAILED':
      return 'error'
    default:
      return undefined
  }
}

function runHistoryTone(
  status: RuntimeStatus,
): 'success' | 'error' | 'neutral' {
  if (status === 'FAILED') {
    return 'error'
  }
  if (status === 'RUNNING' || status === 'STOPPED') {
    return 'success'
  }
  return 'neutral'
}

function runHistoryIcon(tone: 'success' | 'error' | 'neutral'): LucideIcon {
  if (tone === 'error') {
    return XCircle
  }
  if (tone === 'success') {
    return CheckCircle2
  }
  return CircleDashed
}

function buildRuntimeErrorHistory(
  runtimeState: ProcessRuntimeState | null,
  runHistory: RunHistoryEntry[],
): RuntimeErrorHistoryItem[] {
  const historyItems = runHistory.flatMap((entry) => {
    const errorMessage = entry.errorMessage?.trim()
    if (!errorMessage && entry.finalRuntimeStatus !== 'FAILED') {
      return []
    }

    return [
      {
        commandPreview: entry.commandPreview,
        id: `history:${entry.id}`,
        message:
          errorMessage ||
          entry.stopReason?.trim() ||
          'Ejecucion finalizada con estado FAILED',
        occurredAt: entry.endedAt ?? entry.startedAt ?? null,
        source: 'history' as const,
      },
    ]
  })

  const currentMessage = runtimeState?.lastError?.trim()
  if (!runtimeState || !currentMessage) {
    return historyItems
  }

  const hasCurrentErrorInHistory = historyItems.some(
    (item) =>
      item.commandPreview === runtimeState.commandPreview &&
      item.message === currentMessage,
  )

  if (hasCurrentErrorInHistory) {
    return historyItems
  }

  return [
    {
      commandPreview: runtimeState.commandPreview,
      id: `current:${runtimeState.projectId}`,
      message: currentMessage,
      occurredAt: runtimeState.stoppedAt ?? runtimeState.startedAt ?? null,
      source: 'current',
    },
    ...historyItems,
  ]
}

function getProjectRuntimeStatus(
  project: ProjectNode,
  statusByProjectId: Record<string, ProcessRuntimeState>,
) {
  return statusByProjectId[project.id]?.status ?? 'STOPPED'
}

function getDefaultProjectStatusFilter(
  projects: ProjectNode[],
  statusByProjectId: Record<string, ProcessRuntimeState>,
): RuntimeStatus {
  return projects.some(
    (project) =>
      getProjectRuntimeStatus(project, statusByProjectId) === 'RUNNING',
  )
    ? 'RUNNING'
    : 'STOPPED'
}

function shouldShowProjectStatusCard(
  project: ProjectNode,
  displayMode: ProjectListDisplayMode,
  statusFilter: RuntimeStatus,
  statusByProjectId: Record<string, ProcessRuntimeState>,
) {
  if (displayMode === 'ALL') {
    return true
  }

  const status = getProjectRuntimeStatus(project, statusByProjectId)

  return status === statusFilter || status === 'FAILED'
}

function sortProjectStatusCards(
  projects: ProjectNode[],
  sortMode: ProjectListSortMode,
  statusByProjectId: Record<string, ProcessRuntimeState>,
) {
  return [...projects].sort((left, right) => {
    if (sortMode === 'NAME_ASC') {
      return projectNameCollator.compare(left.name, right.name)
    }

    if (sortMode === 'NAME_DESC') {
      return projectNameCollator.compare(right.name, left.name)
    }

    const leftStatus = getProjectRuntimeStatus(left, statusByProjectId)
    const rightStatus = getProjectRuntimeStatus(right, statusByProjectId)
    const statusComparison =
      projectStatusSortOrder[leftStatus] - projectStatusSortOrder[rightStatus]

    return statusComparison === 0
      ? projectNameCollator.compare(left.name, right.name)
      : statusComparison
  })
}

function groupProjectStatusCards(
  projects: ProjectNode[],
  groups: GroupTreeNode[],
  rootGroupId?: string,
): ProjectGroupSection[] {
  const visibleProjectIds = new Set(projects.map((project) => project.id))
  const groupedProjectIds = new Set<string>()
  const groupsById = new Map(groups.map((group) => [group.id, group]))

  function groupPath(group: GroupTreeNode) {
    const path: string[] = []
    const visitedGroupIds = new Set<string>()
    let current: GroupTreeNode | undefined = group

    while (current && !visitedGroupIds.has(current.id)) {
      visitedGroupIds.add(current.id)
      if (current.id === rootGroupId) {
        break
      }

      path.unshift(current.name)
      current = current.parentGroupId
        ? groupsById.get(current.parentGroupId)
        : undefined
    }

    return path.length > 0 ? path.join('/') : 'Este grupo'
  }

  const sections = groups
    .map((group) => {
      const groupProjects = group.projects.filter((project) =>
        visibleProjectIds.has(project.id),
      )
      groupProjects.forEach((project) => groupedProjectIds.add(project.id))

      return { group, label: groupPath(group), projects: groupProjects }
    })
    .filter((section) => section.projects.length > 0)

  const orphanProjects = projects.filter(
    (project) => !groupedProjectIds.has(project.id),
  )

  return orphanProjects.length > 0
    ? [
        ...sections,
        { group: null, label: 'Sin grupo', projects: orphanProjects },
      ]
    : sections
}

function navigationEntryKey(entry: NavigationEntry) {
  return `${entry.workspaceId}:${entry.item.type}:${entry.item.id}`
}

function navigationEntriesEqual(
  left: NavigationEntry | null,
  right: NavigationEntry | null,
) {
  return Boolean(
    left && right && navigationEntryKey(left) === navigationEntryKey(right),
  )
}

type MetricTileProps = {
  children?: ReactNode
  icon: LucideIcon
  label: string
  tone?: MetricTone
  value: string
}

function MetricTile({
  children,
  icon: Icon,
  label,
  tone,
  value,
}: MetricTileProps) {
  const className = tone ? `metric-tile is-state-${tone}` : 'metric-tile'

  return (
    <div className={className} title={`${label}: ${value}`}>
      <span className="metric-tile-icon">
        <Icon aria-hidden="true" size={16} />
      </span>
      <span className="metric-tile-label">{label}</span>
      <span className="metric-tile-value-row">
        <span className="metric-tile-value">{value}</span>
        {children}
      </span>
    </div>
  )
}

function CentralitaApp() {
  const workspaceStore = useWorkspaceStore()
  const runtimeStore = useRuntimeStore(
    workspaceStore.activeWorkspaceId,
    workspaceStore.groups,
  )

  const [isWorkspaceModalOpen, setIsWorkspaceModalOpen] = useState(false)
  const [selection, setSelection] = useState<TreeSelection>(null)
  const [navigationHistory, setNavigationHistory] = useState<{
    entries: NavigationEntry[]
    index: number
  }>({ entries: [], index: -1 })
  const [workspaceName, setWorkspaceName] = useState('')
  const [workspaceRenameDraft, setWorkspaceRenameDraft] = useState<{
    value: string
    workspaceId: string
  } | null>(null)
  const [groupName, setGroupName] = useState('')
  const [parentGroupId, setParentGroupId] = useState('')
  const [createGroupRootGroupId, setCreateGroupRootGroupId] = useState<
    string | null
  >(null)
  const [groupDetailDraft, setGroupDetailDraft] = useState<{
    draft: GroupDetailDraft
    groupId: string
  } | null>(null)
  const [projectDetailDraft, setProjectDetailDraft] = useState<{
    draft: ProjectDetailDraft
    projectId: string
  } | null>(null)
  const [actionModal, setActionModal] = useState<ActionModalState>(null)
  const [deleteModal, setDeleteModal] = useState<DeleteModalState | null>(null)
  const [moveModal, setMoveModal] = useState<MoveModalState | null>(null)
  const [errorHistoryProjectId, setErrorHistoryProjectId] = useState<
    string | null
  >(null)
  const [importPath, setImportPath] = useState('')
  const [projectListDisplayMode, setProjectListDisplayMode] =
    useState<ProjectListDisplayMode>('STATUS')
  const [projectListGroupMode, setProjectListGroupMode] =
    useState<ProjectListGroupMode>('NONE')
  const [projectListSortMode, setProjectListSortMode] =
    useState<ProjectListSortMode>('STATUS')
  const [projectListStatusFilter, setProjectListStatusFilter] =
    useState<RuntimeStatus | null>(null)
  const [reviewDraft, setReviewDraft] = useState<DetectionReviewDraft | null>(
    null,
  )
  const [isReviewValidationPending, setIsReviewValidationPending] =
    useState(false)
  const reviewValidationRequestIdRef = useRef(0)
  const [projectGitInfoState, setProjectGitInfoState] = useState<{
    info: ProjectGitInfo | null
    projectId: string
  } | null>(null)
  const projectGitInfoRequestIdRef = useRef(0)
  const [navigatorWidth, setNavigatorWidth] = useState<number | null>(() =>
    readStoredNavigatorWidth(),
  )
  const [isNavigatorResizing, setIsNavigatorResizing] = useState(false)
  const navigatorWidthRef = useRef(navigatorWidth)
  const navigatorDragRef = useRef<{
    maxWidth: number
    minWidth: number
    pointerId: number
    startWidth: number
    startX: number
  } | null>(null)
  const workspaceShellRef = useRef<HTMLElement | null>(null)
  const explorerSidebarRef = useRef<HTMLElement | null>(null)
  const detailColumnRef = useRef<HTMLElement | null>(null)

  const activeWorkspace = workspaceStore.selectedWorkspace
  const allGroups = flattenGroups(workspaceStore.groups)
  const allProjects = flattenProjects(workspaceStore.groups)
  const hasNoConfiguredWorkspaces =
    !workspaceStore.isLoading && workspaceStore.workspaces.length === 0

  const resolvedSelection = useMemo<TreeSelection>(() => {
    if (!activeWorkspace) {
      return null
    }

    if (!selection) {
      return { id: activeWorkspace.id, type: 'workspace' }
    }

    if (selection.type === 'workspace') {
      return selection.id === activeWorkspace.id
        ? selection
        : { id: activeWorkspace.id, type: 'workspace' }
    }

    if (selection.type === 'group') {
      return findGroup(workspaceStore.groups, selection.id)
        ? selection
        : { id: activeWorkspace.id, type: 'workspace' }
    }

    return findProject(workspaceStore.groups, selection.id)
      ? selection
      : { id: activeWorkspace.id, type: 'workspace' }
  }, [activeWorkspace, selection, workspaceStore.groups])

  const selectedGroup =
    resolvedSelection?.type === 'group'
      ? findGroup(workspaceStore.groups, resolvedSelection.id)
      : null
  const selectedProject =
    resolvedSelection?.type === 'project'
      ? findProject(workspaceStore.groups, resolvedSelection.id)
      : null
  const createGroupRootGroup = createGroupRootGroupId
    ? findGroup(workspaceStore.groups, createGroupRootGroupId)
    : null
  const createGroupParentOptions = createGroupRootGroup
    ? [createGroupRootGroup, ...flattenGroups(createGroupRootGroup.groups)]
    : allGroups

  const workspaceRenameValue =
    activeWorkspace && workspaceRenameDraft?.workspaceId === activeWorkspace.id
      ? workspaceRenameDraft.value
      : (activeWorkspace?.name ?? '')
  const groupDetailValue =
    selectedGroup && groupDetailDraft?.groupId === selectedGroup.id
      ? groupDetailDraft.draft
      : selectedGroup
        ? createGroupDraft(selectedGroup.name)
        : null
  const projectDetailValue =
    selectedProject && projectDetailDraft?.projectId === selectedProject.id
      ? projectDetailDraft.draft
      : selectedProject
        ? createProjectDraft(selectedProject)
        : null

  const selectedRuntimeState = selectedProject
    ? (runtimeStore.statusByProjectId[selectedProject.id] ?? null)
    : null
  const selectedRuntimeLogs = selectedProject
    ? (runtimeStore.logsByProjectId[selectedProject.id] ?? [])
    : []
  const selectedRunHistory = selectedProject
    ? (runtimeStore.historyByProjectId[selectedProject.id] ?? [])
    : []
  const selectedProjectGitLookupPath = selectedProject
    ? cleanPathInput(selectedProject.workingDir ?? selectedProject.path)
    : ''
  const selectedProjectGitInfo =
    selectedProject && projectGitInfoState?.projectId === selectedProject.id
      ? projectGitInfoState.info
      : null
  const resolvedSelectionKey = resolvedSelection
    ? `${resolvedSelection.type}:${resolvedSelection.id}`
    : 'none'
  const currentNavigationEntry = useMemo<NavigationEntry | null>(() => {
    if (!activeWorkspace || !resolvedSelection) {
      return null
    }

    return {
      item: resolvedSelection,
      workspaceId: activeWorkspace.id,
    }
  }, [activeWorkspace, resolvedSelection])
  const previousNavigationIndex = findAvailableNavigationHistoryIndex(-1)
  const nextNavigationIndex = findAvailableNavigationHistoryIndex(1)
  const hasReviewDraft = reviewDraft !== null
  const reviewDraftExecutable = reviewDraft?.executable ?? ''
  const reviewDraftArgsText = reviewDraft?.argsText ?? ''
  const reviewDraftPath = reviewDraft?.path ?? ''
  const reviewDraftWorkingDir = reviewDraft?.workingDir ?? ''
  const workspaceShellStyle =
    navigatorWidth === null
      ? undefined
      : ({
          '--navigator-width': `${navigatorWidth}px`,
        } as CSSProperties)

  function isNavigationEntryAvailable(entry: NavigationEntry) {
    if (
      !workspaceStore.workspaces.some(
        (workspace) => workspace.id === entry.workspaceId,
      )
    ) {
      return false
    }

    if (entry.item.type === 'workspace') {
      return entry.item.id === entry.workspaceId
    }

    const workspaceTree = workspaceStore.treesByWorkspaceId[entry.workspaceId]
    if (!workspaceTree) {
      return true
    }

    return entry.item.type === 'group'
      ? Boolean(findGroup(workspaceTree.groups, entry.item.id))
      : Boolean(findProject(workspaceTree.groups, entry.item.id))
  }

  function findAvailableNavigationHistoryIndex(direction: -1 | 1) {
    for (
      let index = navigationHistory.index + direction;
      index >= 0 && index < navigationHistory.entries.length;
      index += direction
    ) {
      if (isNavigationEntryAvailable(navigationHistory.entries[index])) {
        return index
      }
    }

    return null
  }

  function pushNavigationEntry(entry: NavigationEntry) {
    setNavigationHistory((current) => {
      const currentEntry =
        current.index >= 0 ? current.entries[current.index] : null

      if (navigationEntriesEqual(currentEntry, entry)) {
        return current
      }

      const retainedEntries =
        current.index >= 0 ? current.entries.slice(0, current.index + 1) : []
      const baseEntries =
        retainedEntries.length === 0 &&
        currentNavigationEntry &&
        !navigationEntriesEqual(currentNavigationEntry, entry)
          ? [currentNavigationEntry]
          : retainedEntries
      const lastBaseEntry = baseEntries[baseEntries.length - 1] ?? null
      const nextEntries = navigationEntriesEqual(lastBaseEntry, entry)
        ? baseEntries
        : [...baseEntries, entry]
      const overflowCount = Math.max(
        0,
        nextEntries.length - MAX_NAVIGATION_HISTORY_ENTRIES,
      )
      const trimmedEntries =
        overflowCount > 0 ? nextEntries.slice(overflowCount) : nextEntries

      return {
        entries: trimmedEntries,
        index: trimmedEntries.length - 1,
      }
    })
  }

  async function applyNavigationEntry(
    entry: NavigationEntry,
    options: { recordHistory: boolean },
  ) {
    setErrorHistoryProjectId(null)

    if (entry.item.type === 'workspace') {
      resetImportFlow()
    }

    if (entry.workspaceId !== workspaceStore.activeWorkspaceId) {
      await workspaceStore.actions.selectWorkspace(entry.workspaceId)
    }

    if (entry.item.type === 'project') {
      runtimeStore.actions.selectProject(entry.item.id)
    }

    setSelection(entry.item)

    if (options.recordHistory) {
      pushNavigationEntry(entry)
    }
  }

  async function handleNavigateHistory(targetIndex: number | null) {
    if (targetIndex === null) {
      return
    }

    const targetEntry = navigationHistory.entries[targetIndex]
    if (!targetEntry || !isNavigationEntryAvailable(targetEntry)) {
      return
    }

    await applyNavigationEntry(targetEntry, { recordHistory: false })
    setNavigationHistory((current) =>
      targetIndex >= 0 && targetIndex < current.entries.length
        ? { ...current, index: targetIndex }
        : current,
    )
  }

  const getNavigatorResizeBounds = useCallback(() => {
    const shellWidth = workspaceShellRef.current?.clientWidth ?? 0
    const minWidth = getNavigatorMinWidth()

    if (shellWidth <= 0) {
      return { maxWidth: minWidth, minWidth }
    }

    const maxWidth = Math.max(
      minWidth,
      Math.min(
        Math.round(shellWidth * NAVIGATOR_MAX_WIDTH_RATIO),
        shellWidth -
          NAVIGATOR_RESIZE_RESERVED_WIDTH_PX -
          NAVIGATOR_DETAIL_MIN_WIDTH_PX,
      ),
    )

    return { maxWidth, minWidth }
  }, [])

  const getCurrentNavigatorWidth = useCallback(() => {
    if (navigatorWidthRef.current !== null) {
      return navigatorWidthRef.current
    }

    const renderedWidth =
      explorerSidebarRef.current?.getBoundingClientRect().width ?? 0

    return renderedWidth > 0
      ? Math.round(renderedWidth)
      : getNavigatorResizeBounds().minWidth
  }, [getNavigatorResizeBounds])

  const applyNavigatorWidth = useCallback(
    (width: number, shouldPersist: boolean) => {
      const nextWidth = Math.round(width)

      navigatorWidthRef.current = nextWidth
      setNavigatorWidth(nextWidth)

      if (shouldPersist) {
        writeStoredNavigatorWidth(nextWidth)
      }
    },
    [],
  )

  function handleNavigatorResizePointerDown(
    event: ReactPointerEvent<HTMLDivElement>,
  ) {
    if (event.button !== 0) {
      return
    }

    const { maxWidth, minWidth } = getNavigatorResizeBounds()
    const startWidth = clampNumber(
      getCurrentNavigatorWidth(),
      minWidth,
      maxWidth,
    )

    navigatorDragRef.current = {
      maxWidth,
      minWidth,
      pointerId: event.pointerId,
      startWidth,
      startX: event.clientX,
    }
    setIsNavigatorResizing(true)
    event.currentTarget.setPointerCapture(event.pointerId)
    event.preventDefault()
  }

  function handleNavigatorResizePointerMove(
    event: ReactPointerEvent<HTMLDivElement>,
  ) {
    const currentDrag = navigatorDragRef.current

    if (!currentDrag || currentDrag.pointerId !== event.pointerId) {
      return
    }

    const nextWidth = clampNumber(
      currentDrag.startWidth + event.clientX - currentDrag.startX,
      currentDrag.minWidth,
      currentDrag.maxWidth,
    )

    applyNavigatorWidth(nextWidth, false)
  }

  function finishNavigatorResize(event: ReactPointerEvent<HTMLDivElement>) {
    const currentDrag = navigatorDragRef.current

    if (!currentDrag || currentDrag.pointerId !== event.pointerId) {
      return
    }

    navigatorDragRef.current = null
    setIsNavigatorResizing(false)

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }

    if (navigatorWidthRef.current !== null) {
      writeStoredNavigatorWidth(navigatorWidthRef.current)
    }
  }

  function handleNavigatorResizeKeyDown(
    event: ReactKeyboardEvent<HTMLDivElement>,
  ) {
    const { maxWidth, minWidth } = getNavigatorResizeBounds()
    const currentWidth = clampNumber(
      getCurrentNavigatorWidth(),
      minWidth,
      maxWidth,
    )
    const step = event.shiftKey
      ? NAVIGATOR_RESIZE_LARGE_STEP_PX
      : NAVIGATOR_RESIZE_STEP_PX
    let nextWidth: number | null = null

    if (event.key === 'ArrowLeft') {
      nextWidth = currentWidth - step
    } else if (event.key === 'ArrowRight') {
      nextWidth = currentWidth + step
    } else if (event.key === 'Home') {
      nextWidth = minWidth
    } else if (event.key === 'End') {
      nextWidth = maxWidth
    }

    if (nextWidth === null) {
      return
    }

    event.preventDefault()
    applyNavigatorWidth(clampNumber(nextWidth, minWidth, maxWidth), true)
  }

  useEffect(() => {
    navigatorWidthRef.current = navigatorWidth
  }, [navigatorWidth])

  useLayoutEffect(() => {
    function handleResize() {
      const currentWidth = navigatorWidthRef.current

      if (currentWidth === null) {
        return
      }

      const { maxWidth, minWidth } = getNavigatorResizeBounds()
      const nextWidth = clampNumber(currentWidth, minWidth, maxWidth)

      if (nextWidth !== currentWidth) {
        applyNavigatorWidth(nextWidth, true)
      }
    }

    handleResize()
    window.addEventListener('resize', handleResize)

    return () => {
      window.removeEventListener('resize', handleResize)
    }
  }, [applyNavigatorWidth, getNavigatorResizeBounds])

  useLayoutEffect(() => {
    const detailColumn = detailColumnRef.current
    if (!detailColumn) {
      return
    }

    detailColumn.scrollTo({ left: 0, top: 0 })
    detailColumn.scrollIntoView({ block: 'start', inline: 'nearest' })
  }, [resolvedSelectionKey])

  useEffect(() => {
    if (!selectedProject) {
      return
    }

    const currentRequestId = projectGitInfoRequestIdRef.current + 1
    projectGitInfoRequestIdRef.current = currentRequestId
    const projectId = selectedProject.id
    const lookupPath = selectedProjectGitLookupPath

    if (!lookupPath) {
      return
    }

    void getProjectGitInfo({ path: lookupPath })
      .then((info) => {
        if (projectGitInfoRequestIdRef.current !== currentRequestId) {
          return
        }

        setProjectGitInfoState({
          info: info.isRepository ? info : null,
          projectId,
        })
      })
      .catch(() => {
        if (projectGitInfoRequestIdRef.current !== currentRequestId) {
          return
        }

        setProjectGitInfoState({ info: null, projectId })
      })
  }, [selectedProject, selectedProjectGitLookupPath])

  useEffect(() => {
    if (!hasReviewDraft) {
      return
    }

    const currentRequestId = reviewValidationRequestIdRef.current + 1
    reviewValidationRequestIdRef.current = currentRequestId
    const validationInput = {
      args: parseArgsInput(reviewDraftArgsText),
      executable: reviewDraftExecutable.trim() || null,
      path: cleanPathInput(reviewDraftPath),
      workingDir: cleanPathInput(reviewDraftWorkingDir) || null,
    }
    const timeoutId = window.setTimeout(() => {
      setIsReviewValidationPending(true)
      void validateProjectCommand(validationInput)
        .then((commandValidation) => {
          if (reviewValidationRequestIdRef.current !== currentRequestId) {
            return
          }

          setReviewDraft((current) =>
            current
              ? {
                  ...current,
                  commandValidation,
                }
              : current,
          )
          setIsReviewValidationPending(false)
        })
        .catch((error) => {
          if (reviewValidationRequestIdRef.current !== currentRequestId) {
            return
          }

          setReviewDraft((current) =>
            current
              ? {
                  ...current,
                  commandValidation: {
                    isRunnable: false,
                    commandPreview: formatCommandPreview(
                      current.executable,
                      parseArgsInput(current.argsText),
                    ),
                    resolvedExecutable: null,
                    issues: [
                      error instanceof Error
                        ? error.message
                        : 'No se pudo validar el comando propuesto.',
                    ],
                  },
                }
              : current,
          )
          setIsReviewValidationPending(false)
        })
    }, 150)

    return () => {
      window.clearTimeout(timeoutId)
    }
  }, [
    hasReviewDraft,
    reviewDraftArgsText,
    reviewDraftExecutable,
    reviewDraftPath,
    reviewDraftWorkingDir,
  ])

  const reviewCommandPreview = reviewDraft
    ? formatCommandPreview(
        reviewDraft.executable,
        parseArgsInput(reviewDraft.argsText),
      )
    : 'Manual review required'
  const projectCommand =
    selectedProject && projectDetailValue
      ? projectCommandPreview(
          selectedProject,
          projectDetailValue.executable,
          projectDetailValue.argsText,
        )
      : 'Sin comando derivado'
  const workspaceRenameCandidate = workspaceRenameValue.trim()
  const saveWorkspaceNameBlockedReason = !activeWorkspace
    ? 'No hay workspace activo.'
    : !workspaceRenameCandidate
      ? 'Escribe un nombre para el workspace.'
      : workspaceRenameCandidate === activeWorkspace.name
        ? 'El nombre del workspace no ha cambiado.'
        : undefined
  const canSaveWorkspaceName = !saveWorkspaceNameBlockedReason
  const groupNameCandidateForCreate = groupName.trim()
  const createGroupParentGroupId = parentGroupId || null
  const createGroupBlockedReason = !activeWorkspace
    ? 'No hay workspace activo.'
    : !groupNameCandidateForCreate
      ? 'Escribe un nombre para el grupo.'
      : groupNameExists(workspaceStore.groups, groupNameCandidateForCreate, {
            parentGroupId: createGroupParentGroupId,
          })
        ? 'Ya existe un grupo con ese nombre.'
        : createGroupRootGroupId &&
            !createGroupParentOptions.some(
              (group) => group.id === parentGroupId,
            )
          ? 'Selecciona un grupo padre válido.'
          : undefined
  const canCreateGroup = !createGroupBlockedReason
  const saveDetectionProjectBlockedReason = !activeWorkspace
    ? 'No hay workspace activo.'
    : !reviewDraft
      ? 'No hay detección de proyecto para guardar.'
      : !reviewDraft.groupId
        ? 'Selecciona un grupo destino.'
        : !reviewDraft.name.trim()
          ? 'Escribe un nombre para el proyecto.'
          : !cleanPathInput(reviewDraft.path)
            ? 'Indica la ruta del proyecto.'
            : isReviewValidationPending
              ? 'Espera a que termine la validación del comando.'
              : !reviewDraft.commandValidation.isRunnable
                ? (reviewDraft.commandValidation.issues[0] ??
                  'El comando propuesto no supera la validación.')
                : undefined
  const canSaveDetectionProject = !saveDetectionProjectBlockedReason
  const selectedGroupProjectCount = selectedGroup
    ? flattenProjects([selectedGroup]).length
    : 0
  const selectedGroupSubgroupCount = selectedGroup
    ? flattenGroups(selectedGroup.groups).length
    : 0
  const selectedGroupStatus = selectedGroup
    ? groupRuntimeStatus(selectedGroup, runtimeStore.statusByProjectId)
    : 'STOPPED'
  const groupNameCandidate = groupDetailValue?.name.trim() ?? ''
  const saveGroupBlockedReason = !selectedGroup
    ? 'No hay grupo seleccionado.'
    : !groupDetailValue
      ? 'No hay cambios de grupo para guardar.'
      : !groupNameCandidate
        ? 'Escribe un nombre para el grupo.'
        : groupNameExists(workspaceStore.groups, groupNameCandidate, {
              excludeGroupId: selectedGroup.id,
              parentGroupId: selectedGroup.parentGroupId,
            })
          ? 'Ya existe un grupo con ese nombre.'
          : groupNameCandidate === selectedGroup.name
            ? 'El grupo no tiene cambios pendientes.'
            : undefined
  const canSaveGroup = !saveGroupBlockedReason
  const projectStatus = selectedRuntimeState?.status ?? 'STOPPED'
  const projectDraftName = projectDetailValue?.name.trim() ?? ''
  const projectDraftPath = projectDetailValue
    ? cleanPathInput(projectDetailValue.path)
    : ''
  const projectDraftWorkingDir = projectDetailValue
    ? cleanPathInput(projectDetailValue.workingDir)
    : ''
  const projectDraftExecutable = projectDetailValue?.executable.trim() ?? ''
  const projectDraftArgs = projectDetailValue
    ? parseArgsInput(projectDetailValue.argsText)
    : []
  const originalProjectPath = selectedProject
    ? cleanPathInput(selectedProject.path)
    : ''
  const originalProjectWorkingDir = selectedProject
    ? cleanPathInput(selectedProject.workingDir ?? selectedProject.path)
    : ''
  const originalProjectExecutable = selectedProject?.executable?.trim() ?? ''
  const originalProjectArgs = selectedProject?.args ?? []
  const projectHasPendingChanges = Boolean(
    selectedProject &&
    projectDetailValue &&
    (projectDraftName !== selectedProject.name ||
      projectDraftPath !== originalProjectPath ||
      projectDraftWorkingDir !== originalProjectWorkingDir ||
      projectDraftExecutable !== originalProjectExecutable ||
      !areStringArraysEqual(projectDraftArgs, originalProjectArgs)),
  )
  const saveProjectBlockedReason = !selectedProject
    ? 'No hay proyecto seleccionado.'
    : !projectDetailValue
      ? 'No hay cambios de proyecto para guardar.'
      : !projectDraftName
        ? 'Escribe un nombre para el proyecto.'
        : !projectDraftPath
          ? 'Indica la ruta del proyecto.'
          : !projectHasPendingChanges
            ? 'El proyecto no tiene cambios pendientes.'
            : undefined
  const canSaveProject = !saveProjectBlockedReason
  const startWorkspaceBlockedReason =
    allProjects.length === 0
      ? 'El workspace no contiene proyectos.'
      : runtimeStore.workspaceStatus !== 'STOPPED' &&
          runtimeStore.workspaceStatus !== 'FAILED'
        ? 'El workspace no está detenido ni fallido.'
        : undefined
  const canStartWorkspace = !startWorkspaceBlockedReason
  const stopWorkspaceBlockedReason =
    allProjects.length === 0
      ? 'El workspace no contiene proyectos.'
      : runtimeStore.workspaceStatus === 'STOPPED'
        ? 'El workspace ya está detenido.'
        : runtimeStore.workspaceStatus === 'STOPPING'
          ? 'El workspace ya se está deteniendo.'
          : undefined
  const canStopWorkspace = !stopWorkspaceBlockedReason
  const startGroupBlockedReason =
    selectedGroupProjectCount === 0
      ? 'El grupo no contiene proyectos.'
      : selectedGroupStatus !== 'STOPPED' && selectedGroupStatus !== 'FAILED'
        ? 'El grupo no está detenido ni fallido.'
        : undefined
  const canStartGroup = !startGroupBlockedReason
  const stopGroupBlockedReason =
    selectedGroupProjectCount === 0
      ? 'El grupo no contiene proyectos.'
      : selectedGroupStatus === 'STOPPED'
        ? 'El grupo ya está detenido.'
        : selectedGroupStatus === 'STOPPING'
          ? 'El grupo ya se está deteniendo.'
          : undefined
  const canStopGroup = !stopGroupBlockedReason
  const startProjectBlockedReason =
    projectStatus === 'STOPPED' || projectStatus === 'FAILED'
      ? undefined
      : 'El proyecto no está detenido ni fallido.'
  const canStartProject = !startProjectBlockedReason
  const stopProjectBlockedReason =
    projectStatus === 'RUNNING' || projectStatus === 'STARTING'
      ? undefined
      : 'El proyecto no está en ejecución ni arrancando.'
  const canStopProject = !stopProjectBlockedReason
  const restartProjectBlockedReason =
    projectStatus === 'STOPPED'
      ? 'El proyecto está detenido.'
      : projectStatus === 'STOPPING'
        ? 'El proyecto se está deteniendo.'
        : undefined
  const canRestartProject = !restartProjectBlockedReason
  const clearProjectLogsBlockedReason =
    selectedRuntimeLogs.length > 0
      ? undefined
      : 'No hay logs para limpiar en la vista.'
  const canClearProjectLogs = !clearProjectLogsBlockedReason
  const analyzeImportBlockedReason = !importPath.trim()
    ? 'Selecciona o escribe una carpeta para analizar.'
    : workspaceStore.isAnalyzing
      ? 'Ya hay un análisis de carpeta en curso.'
      : undefined
  const createWorkspaceBlockedReason = workspaceName.trim()
    ? undefined
    : 'Escribe un nombre para el workspace.'

  function findLoadedGroup(groupId: string) {
    for (const tree of Object.values(workspaceStore.treesByWorkspaceId)) {
      const group = findGroup(tree.groups, groupId)
      if (group) {
        return group
      }
    }

    return null
  }

  function resolveWorkspaceName(workspaceId: string) {
    return (
      workspaceStore.workspaces.find(
        (workspace) => workspace.id === workspaceId,
      )?.name ?? workspaceId
    )
  }

  function buildMoveModalState(
    source: NavigatorDragItem,
    target: NavigatorDropTarget,
  ): MoveModalState | null {
    if (source.type === 'project') {
      if (target.type !== 'group') {
        return null
      }

      if (
        source.workspaceId === target.workspaceId &&
        source.groupId === target.id
      ) {
        return null
      }

      const targetWorkspaceLabel =
        source.workspaceId === target.workspaceId
          ? ''
          : ` del workspace "${resolveWorkspaceName(target.workspaceId)}"`

      return {
        description: `Se moverá el proyecto "${source.name}" al grupo "${target.name}"${targetWorkspaceLabel}.`,
        source,
        target,
        title: 'Mover proyecto',
      }
    }

    if (target.type === 'workspace') {
      if (source.workspaceId === target.id && source.parentGroupId === null) {
        return null
      }

      return {
        description: `Se moverá el grupo "${source.name}" a la raíz del workspace "${target.name}".`,
        source,
        target,
        title: 'Mover grupo',
      }
    }

    if (source.id === target.id) {
      return null
    }

    const sourceGroup = findLoadedGroup(source.id)
    if (!sourceGroup || groupHasDescendant(sourceGroup, target.id)) {
      return null
    }

    if (
      source.workspaceId === target.workspaceId &&
      source.parentGroupId === target.id
    ) {
      return null
    }

    const targetWorkspaceLabel =
      source.workspaceId === target.workspaceId
        ? ''
        : ` del workspace "${resolveWorkspaceName(target.workspaceId)}"`

    return {
      description: `Se moverá el grupo "${source.name}" dentro del grupo "${target.name}"${targetWorkspaceLabel}.`,
      source,
      target,
      title: 'Mover grupo',
    }
  }

  async function handlePickFolder() {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: 'Selecciona la carpeta del proyecto',
    })

    if (typeof selectedPath !== 'string') {
      return
    }

    setImportPath(normalizeWindowsPath(selectedPath))
    workspaceStore.actions.clearAnalysis()
  }

  function resetImportFlow() {
    reviewValidationRequestIdRef.current += 1
    setImportPath('')
    setIsReviewValidationPending(false)
    setReviewDraft(null)
    workspaceStore.actions.clearAnalysis()
  }

  async function handleSelectWorkspace(workspaceId: string) {
    await applyNavigationEntry(
      {
        item: { id: workspaceId, type: 'workspace' },
        workspaceId,
      },
      { recordHistory: true },
    )
  }

  async function handleCreateWorkspace() {
    if (!workspaceName.trim()) {
      return
    }

    await workspaceStore.actions.createWorkspace(workspaceName.trim())
    setWorkspaceName('')
    setIsWorkspaceModalOpen(false)
  }

  function openCreateGroupModal(rootGroupId: string | null) {
    setGroupName('')
    setParentGroupId(rootGroupId ?? '')
    setCreateGroupRootGroupId(rootGroupId)
    setActionModal('createGroup')
  }

  function closeCreateGroupModal() {
    setActionModal(null)
    setCreateGroupRootGroupId(null)
    setParentGroupId('')
  }

  function openDeleteModal(target: DeleteModalState) {
    setDeleteModal(target)
  }

  async function handleConfirmDelete() {
    if (!deleteModal) {
      return
    }

    const currentModal = deleteModal
    setDeleteModal(null)

    if (currentModal.kind === 'workspace') {
      await workspaceStore.actions.deleteWorkspace(currentModal.id)
      return
    }

    if (currentModal.kind === 'group') {
      await workspaceStore.actions.deleteGroup(currentModal.id)
      return
    }

    await workspaceStore.actions.deleteProject(currentModal.id)
  }

  function handleRequestMove(
    source: NavigatorDragItem,
    target: NavigatorDropTarget,
  ) {
    const nextMoveModal = buildMoveModalState(source, target)
    if (!nextMoveModal) {
      return
    }

    setMoveModal(nextMoveModal)
  }

  async function handleConfirmMove() {
    if (!moveModal) {
      return
    }

    const currentMove = moveModal
    setMoveModal(null)

    if (currentMove.source.type === 'group') {
      const targetWorkspaceId =
        currentMove.target.type === 'group'
          ? currentMove.target.workspaceId
          : currentMove.target.id

      await workspaceStore.actions.moveGroupTree(currentMove.source.id, {
        parentGroupId:
          currentMove.target.type === 'group' ? currentMove.target.id : null,
        workspaceId: targetWorkspaceId,
      })
      await handleSelectGroup(currentMove.source.id, targetWorkspaceId)
      return
    }

    if (currentMove.target.type !== 'group') {
      return
    }

    await workspaceStore.actions.updateProject(currentMove.source.id, {
      groupId: currentMove.target.id,
      workspaceId: currentMove.target.workspaceId,
    })
    await handleSelectProject(
      currentMove.source.id,
      currentMove.target.workspaceId,
    )
  }

  async function handleSelectGroup(groupId: string, workspaceId: string) {
    await applyNavigationEntry(
      {
        item: { id: groupId, type: 'group' },
        workspaceId,
      },
      { recordHistory: true },
    )
  }

  async function handleSelectProject(projectId: string, workspaceId: string) {
    await applyNavigationEntry(
      {
        item: { id: projectId, type: 'project' },
        workspaceId,
      },
      { recordHistory: true },
    )
  }

  function renderDetectionReviewModal() {
    if (!reviewDraft || !activeWorkspace) {
      return null
    }

    function handleSaveDetectionProject() {
      if (!reviewDraft || !activeWorkspace) {
        return
      }

      const args = parseArgsInput(reviewDraft.argsText)
      const nextCommand =
        reviewCommandPreview === 'Manual review required'
          ? null
          : reviewCommandPreview
      const targetWorkspaceId = activeWorkspace.id

      void (async () => {
        const createdProject =
          await workspaceStore.actions.createProjectFromDetection({
            workspaceId: targetWorkspaceId,
            groupId: reviewDraft.groupId,
            name: reviewDraft.name.trim(),
            path: cleanPathInput(reviewDraft.path),
            detectedType: reviewDraft.detectedType,
            color: null,
            packageManager: reviewDraft.packageManager,
            executable: reviewDraft.executable.trim() || null,
            command: nextCommand,
            args: args.length > 0 ? args : undefined,
            workingDir: cleanPathInput(reviewDraft.workingDir) || null,
            detectionConfidence: reviewDraft.confidence,
            detectionEvidence: reviewDraft.evidence,
            warnings: reviewDraft.warnings,
          })
        resetImportFlow()
        await handleSelectProject(createdProject.id, targetWorkspaceId)
      })()
    }

    return (
      <ModalFrame
        ariaLabel="Revisar detección"
        className="modal-card-detection-review"
        closeOnBackdropClick={false}
        closeLabel="Cerrar revisión de detección"
        eyebrow="Proyecto detectado"
        onClose={resetImportFlow}
        title="Revisar detección"
      >
        <div className="review-modal-scroll">
          <div className="section-title">
            <div>
              <p className="eyebrow">Proyecto detectado</p>
              <h3>Ajustes detectados</h3>
              <p className="review-detection-confidence">
                Confianza {confidenceLabel(reviewDraft.confidence)} ·{' '}
                {(reviewDraft.confidence * 100).toFixed(0)}%
              </p>
            </div>
          </div>

          <div className="review-grid">
            <label className="field">
              <span>Nombre detectado</span>
              <input
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? { ...current, name: event.target.value }
                      : current,
                  )
                }
                value={reviewDraft.name}
              />
            </label>

            <label className="field">
              <span>Grupo destino</span>
              <select
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? { ...current, groupId: event.target.value }
                      : current,
                  )
                }
                value={reviewDraft.groupId}
              >
                <option value="">Selecciona un grupo</option>
                {allGroups.map((group) => (
                  <option key={group.id} value={group.id}>
                    {group.name}
                  </option>
                ))}
              </select>
            </label>

            <label className="field full-width">
              <span>Ruta</span>
              <input
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? { ...current, path: event.target.value }
                      : current,
                  )
                }
                value={reviewDraft.path}
              />
            </label>

            <label className="field">
              <span>Tipo detectado</span>
              <select
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? {
                          ...current,
                          detectedType: event.target
                            .value as DetectedProjectType,
                        }
                      : current,
                  )
                }
                value={reviewDraft.detectedType}
              >
                {detectedTypeOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>

            <label className="field">
              <span>Package manager / runner</span>
              <select
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? {
                          ...current,
                          packageManager:
                            event.target.value === ''
                              ? null
                              : (event.target.value as ProjectPackageManager),
                        }
                      : current,
                  )
                }
                value={reviewDraft.packageManager ?? ''}
              >
                <option value="">Sin asignar</option>
                {packageManagerOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>

            <label className="field">
              <span>Ejecutable</span>
              <input
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? { ...current, executable: event.target.value }
                      : current,
                  )
                }
                placeholder="pnpm"
                value={reviewDraft.executable}
              />
            </label>

            <label className="field">
              <span>Working dir</span>
              <input
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? { ...current, workingDir: event.target.value }
                      : current,
                  )
                }
                value={reviewDraft.workingDir}
              />
            </label>

            <label className="field full-width review-args-field">
              <span>Argumentos sugeridos (uno por línea)</span>
              <textarea
                onChange={(event) =>
                  setReviewDraft((current) =>
                    current
                      ? { ...current, argsText: event.target.value }
                      : current,
                  )
                }
                rows={4}
                value={reviewDraft.argsText}
              />
            </label>
          </div>

          <div className="info-grid review-info-grid">
            <article className="info-card review-info-card">
              <h4>Comando resultante</h4>
              <p className="review-command-preview">{reviewCommandPreview}</p>
              {isReviewValidationPending ? (
                <p className="muted">Validando comando propuesto...</p>
              ) : reviewDraft.commandValidation.isRunnable ? (
                <>
                  <p className="muted">Comando listo para ejecutar.</p>
                  {reviewDraft.commandValidation.resolvedExecutable ? (
                    <p className="muted">
                      Ejecutable resuelto:{' '}
                      {normalizeWindowsPath(
                        reviewDraft.commandValidation.resolvedExecutable,
                      )}
                    </p>
                  ) : null}
                </>
              ) : (
                <ul className="detail-list review-detail-list warning-list">
                  {reviewDraft.commandValidation.issues.map((issue) => (
                    <li key={issue}>
                      <strong>validación</strong>
                      <span>{issue}</span>
                    </li>
                  ))}
                </ul>
              )}
              {!allGroups.length ? (
                <p className="warning-inline">
                  Crea al menos un grupo antes de guardar el proyecto.
                </p>
              ) : null}
            </article>

            <article className="info-card review-info-card">
              <h4>Evidencias</h4>
              <ul className="detail-list review-detail-list">
                {reviewDraft.evidence.map((item) => (
                  <li key={`${item.source}-${item.kind}-${item.detail}`}>
                    <strong>{item.source}</strong>
                    <span>{item.detail}</span>
                  </li>
                ))}
              </ul>
            </article>

            <article className="info-card review-info-card">
              <h4>Warnings</h4>
              {reviewDraft.warnings.length > 0 ? (
                <ul className="detail-list review-detail-list warning-list">
                  {reviewDraft.warnings.map((warning) => (
                    <li key={`${warning.code}-${warning.source ?? 'global'}`}>
                      <strong>{warning.code}</strong>
                      <span>{warning.message}</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="muted">Sin warnings relevantes.</p>
              )}
            </article>
          </div>
        </div>

        <div className="modal-actions review-modal-actions">
          <button className="secondary" onClick={resetImportFlow} type="button">
            Descartar
          </button>
          <BlockedActionButton
            blockedReason={saveDetectionProjectBlockedReason}
            disabled={!canSaveDetectionProject}
            onClick={handleSaveDetectionProject}
            type="button"
          >
            Guardar proyecto
          </BlockedActionButton>
        </div>
      </ModalFrame>
    )
  }

  function renderProjectStatusCard(project: ProjectNode) {
    const runtimeState = runtimeStore.statusByProjectId[project.id] ?? null
    const runtimeStatus = runtimeState?.status ?? 'STOPPED'
    const groupName =
      allGroups.find((group) => group.id === project.groupId)?.name ??
      'Grupo no cargado'
    const workingDir = normalizeWindowsPath(project.workingDir ?? project.path)
    const lastError = runtimeState?.lastError ?? null

    return (
      <button
        aria-label={`Abrir detalle de proyecto ${project.name}`}
        className="workspace-project-status-card"
        key={project.id}
        onClick={() =>
          void handleSelectProject(project.id, project.workspaceId)
        }
        title={lastError ?? workingDir}
        type="button"
      >
        <span className="workspace-project-status-main">
          <RuntimeStatusBadge status={runtimeStatus} />
          <span className="workspace-project-status-copy">
            <strong>{project.name}</strong>
            <span>{groupName}</span>
          </span>
        </span>
        <span className="workspace-project-status-meta">
          {lastError ? (
            <span className="workspace-project-status-error">{lastError}</span>
          ) : (
            <span>{workingDir}</span>
          )}
        </span>
      </button>
    )
  }

  function renderProjectStatusControls(effectiveStatusFilter: RuntimeStatus) {
    return (
      <div className="workspace-project-status-controls">
        <label className="field workspace-project-status-control">
          <span>Mostrar</span>
          <select
            onChange={(event) =>
              setProjectListDisplayMode(
                event.target.value as ProjectListDisplayMode,
              )
            }
            value={projectListDisplayMode}
          >
            <option value="STATUS">Por estado</option>
            <option value="ALL">Todos</option>
          </select>
        </label>

        {projectListDisplayMode === 'STATUS' ? (
          <label className="field workspace-project-status-control">
            <span>Estado</span>
            <select
              onChange={(event) =>
                setProjectListStatusFilter(event.target.value as RuntimeStatus)
              }
              value={effectiveStatusFilter}
            >
              {projectStatusFilterOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        ) : null}

        <label className="field workspace-project-status-control">
          <span>Agrupar por</span>
          <select
            onChange={(event) =>
              setProjectListGroupMode(
                event.target.value as ProjectListGroupMode,
              )
            }
            value={projectListGroupMode}
          >
            <option value="NONE">No agrupar</option>
            <option value="GROUPS">Grupos</option>
          </select>
        </label>

        {projectListGroupMode === 'NONE' ? (
          <label className="field workspace-project-status-control">
            <span>Ordenar por</span>
            <select
              onChange={(event) =>
                setProjectListSortMode(
                  event.target.value as ProjectListSortMode,
                )
              }
              value={projectListSortMode}
            >
              <option value="NAME_ASC">Nombre A-Z</option>
              <option value="NAME_DESC">Nombre Z-A</option>
              <option value="STATUS">Estado</option>
            </select>
          </label>
        ) : null}
      </div>
    )
  }

  function renderProjectStatusOverview({
    emptyMessage,
    groups,
    projects,
    rootGroupId,
  }: {
    emptyMessage: string
    groups: GroupTreeNode[]
    projects: ProjectNode[]
    rootGroupId?: string
  }) {
    if (projects.length === 0) {
      return <p className="empty-state">{emptyMessage}</p>
    }

    const defaultStatusFilter = getDefaultProjectStatusFilter(
      projects,
      runtimeStore.statusByProjectId,
    )
    const effectiveStatusFilter = projectListStatusFilter ?? defaultStatusFilter
    const visibleProjects = projects.filter((project) =>
      shouldShowProjectStatusCard(
        project,
        projectListDisplayMode,
        effectiveStatusFilter,
        runtimeStore.statusByProjectId,
      ),
    )
    const sortedProjects =
      projectListGroupMode === 'NONE'
        ? sortProjectStatusCards(
            visibleProjects,
            projectListSortMode,
            runtimeStore.statusByProjectId,
          )
        : visibleProjects
    const groupedSections =
      projectListGroupMode === 'GROUPS'
        ? groupProjectStatusCards(sortedProjects, groups, rootGroupId)
        : []

    return (
      <section className="workspace-project-status-section">
        {renderProjectStatusControls(effectiveStatusFilter)}
        {visibleProjects.length === 0 ? (
          <p className="empty-state">
            No hay proyectos que coincidan con el filtro seleccionado.
          </p>
        ) : projectListGroupMode === 'GROUPS' ? (
          <div className="workspace-project-status-groups">
            {groupedSections.map((section) => (
              <section
                className="workspace-project-status-group"
                key={section.group?.id ?? 'ungrouped'}
              >
                <div className="workspace-project-status-group-title">
                  <span title={section.label}>{section.label}</span>
                  <span>{section.projects.length}</span>
                </div>
                <div className="workspace-project-status-list">
                  {section.projects.map(renderProjectStatusCard)}
                </div>
              </section>
            ))}
          </div>
        ) : (
          <div className="workspace-project-status-list">
            {sortedProjects.map(renderProjectStatusCard)}
          </div>
        )}
      </section>
    )
  }

  function renderWorkspaceDetail() {
    if (!activeWorkspace) {
      return (
        <section className="card detail-panel">
          <div className="section-title">
            <div>
              <p className="eyebrow">Inicio</p>
              <h2>Crear el primer workspace</h2>
            </div>
          </div>
          <p className="muted">
            Usa el botón <strong>+ Nuevo workspace</strong> del panel Árbol para
            crear tu primer workspace.
          </p>
        </section>
      )
    }

    return (
      <section className="detail-stack">
        <header className="project-header">
          <div className="project-header-main">
            <div className="project-header-status-stack">
              <RuntimeStatusBadge
                status={runtimeStore.workspaceStatus}
                variant="compact"
              />
            </div>
            <div className="project-header-copy">
              <p className="eyebrow">Workspace</p>
              <h2 className="project-header-name workspace-detail-title">
                <span>{activeWorkspace.name}</span>
                {workspaceStore.isLoading ? (
                  <span
                    aria-label="Cargando estado persistido"
                    className="workspace-loading-spinner"
                    role="status"
                    title="Cargando estado persistido"
                  />
                ) : null}
              </h2>
              <div className="project-header-meta">
                <span
                  className="project-header-meta-item"
                  title={`Actualizado ${formatDateTime(activeWorkspace.updatedAt)}`}
                >
                  <Clock aria-hidden="true" className="icon" size={14} />
                  <span>{formatDateTime(activeWorkspace.updatedAt)}</span>
                </span>
              </div>
            </div>
          </div>
          <div className="project-action-cluster">
            <BlockedActionButton
              aria-label="Start all"
              blockedReason={startWorkspaceBlockedReason}
              className="icon-button"
              disabled={!canStartWorkspace}
              onClick={() =>
                void runtimeStore.actions.startWorkspace(activeWorkspace.id)
              }
              title="Start all"
              type="button"
            >
              <Play aria-hidden="true" size={18} />
            </BlockedActionButton>
            <BlockedActionButton
              aria-label="Stop all"
              blockedReason={stopWorkspaceBlockedReason}
              className="icon-button"
              disabled={!canStopWorkspace}
              onClick={() =>
                void runtimeStore.actions.stopWorkspace(activeWorkspace.id)
              }
              title="Stop all"
              type="button"
            >
              <Square aria-hidden="true" size={18} />
            </BlockedActionButton>
            <button
              aria-label="Recargar workspace"
              className="icon-button"
              onClick={() =>
                void Promise.all([
                  workspaceStore.actions.refresh(),
                  runtimeStore.actions.syncWorkspaceRuntime(),
                ])
              }
              title="Recargar workspace"
              type="button"
            >
              <RotateCcw aria-hidden="true" size={18} />
            </button>
            <span aria-hidden="true" className="action-divider" />
            <button
              aria-label="Renombrar workspace"
              className="icon-button"
              onClick={() => setActionModal('renameWorkspace')}
              title="Renombrar workspace"
              type="button"
            >
              <Settings2 aria-hidden="true" size={18} />
            </button>
            <button
              aria-label="Crear grupo"
              className="icon-button"
              onClick={() => openCreateGroupModal(null)}
              title="Crear grupo"
              type="button"
            >
              <Plus aria-hidden="true" size={18} />
            </button>
            <button
              aria-label="Eliminar workspace"
              className="icon-button danger"
              onClick={() =>
                openDeleteModal({
                  confirmLabel: 'Eliminar workspace',
                  description: `Se eliminara el workspace "${activeWorkspace.name}" con todos sus grupos y proyectos.`,
                  id: activeWorkspace.id,
                  kind: 'workspace',
                  title: 'Eliminar workspace',
                })
              }
              title="Eliminar workspace"
              type="button"
            >
              <Trash2 aria-hidden="true" size={18} />
            </button>
          </div>
        </header>

        <div className="project-metrics-strip">
          <MetricTile
            icon={runtimeStatusIcon(runtimeStore.workspaceStatus)}
            label="Estado"
            tone={runtimeStatusTone(runtimeStore.workspaceStatus)}
            value={runtimeStore.workspaceStatus}
          />
          <MetricTile
            icon={Hash}
            label="Grupos"
            value={String(allGroups.length)}
          />
          <MetricTile
            icon={Hash}
            label="Proyectos"
            value={String(allProjects.length)}
          />
          <MetricTile
            icon={Clock}
            label="Actualizado"
            value={formatDateTime(activeWorkspace.updatedAt)}
          />
        </div>

        {renderProjectStatusOverview({
          emptyMessage: 'Este workspace no contiene proyectos.',
          groups: allGroups,
          projects: allProjects,
        })}
      </section>
    )
  }

  function renderGroupDetail() {
    if (!selectedGroup || !groupDetailValue) {
      return null
    }

    const runtimeStatus = selectedGroupStatus

    return (
      <section className="detail-stack">
        <header className="project-header">
          <div className="project-header-main">
            <div className="project-header-status-stack">
              <RuntimeStatusBadge status={runtimeStatus} variant="compact" />
            </div>
            <div className="project-header-copy">
              <p className="eyebrow">Grupo</p>
              <h2 className="project-header-name">{selectedGroup.name}</h2>
              <div className="project-header-meta">
                <span
                  className="project-header-meta-item"
                  title={`Actualizado ${formatDateTime(selectedGroup.updatedAt)}`}
                >
                  <Clock aria-hidden="true" className="icon" size={14} />
                  <span>{formatDateTime(selectedGroup.updatedAt)}</span>
                </span>
              </div>
            </div>
          </div>
          <div className="project-action-cluster">
            <BlockedActionButton
              aria-label="Start group"
              blockedReason={startGroupBlockedReason}
              className="icon-button"
              disabled={!canStartGroup}
              onClick={() =>
                void runtimeStore.actions.startGroup(selectedGroup.id)
              }
              title="Start group"
              type="button"
            >
              <Play aria-hidden="true" size={18} />
            </BlockedActionButton>
            <BlockedActionButton
              aria-label="Stop group"
              blockedReason={stopGroupBlockedReason}
              className="icon-button"
              disabled={!canStopGroup}
              onClick={() =>
                void runtimeStore.actions.stopGroup(selectedGroup.id)
              }
              title="Stop group"
              type="button"
            >
              <Square aria-hidden="true" size={18} />
            </BlockedActionButton>
            <span aria-hidden="true" className="action-divider" />
            <button
              aria-label="Configurar grupo"
              className="icon-button"
              onClick={() => setActionModal('editGroup')}
              title="Configurar grupo"
              type="button"
            >
              <Settings2 aria-hidden="true" size={18} />
            </button>
            <button
              aria-label="Importar proyecto"
              className="icon-button"
              onClick={() => setActionModal('importProject')}
              title="Importar proyecto"
              type="button"
            >
              <FolderPlus aria-hidden="true" size={18} />
            </button>
            <button
              aria-label="Crear grupo"
              className="icon-button"
              onClick={() => openCreateGroupModal(selectedGroup.id)}
              title="Crear grupo"
              type="button"
            >
              <Plus aria-hidden="true" size={18} />
            </button>
            <button
              aria-label="Eliminar grupo"
              className="icon-button danger"
              onClick={() =>
                openDeleteModal({
                  confirmLabel: 'Eliminar grupo',
                  description: `Se eliminara el grupo "${selectedGroup.name}" con todos sus subgrupos y proyectos.`,
                  id: selectedGroup.id,
                  kind: 'group',
                  title: 'Eliminar grupo',
                })
              }
              title="Eliminar grupo"
              type="button"
            >
              <Trash2 aria-hidden="true" size={18} />
            </button>
          </div>
        </header>

        <div className="project-metrics-strip">
          <MetricTile
            icon={runtimeStatusIcon(runtimeStatus)}
            label="Estado"
            tone={runtimeStatusTone(runtimeStatus)}
            value={runtimeStatus}
          />
          <MetricTile
            icon={Hash}
            label="Subgrupos"
            value={String(selectedGroupSubgroupCount)}
          />
          <MetricTile
            icon={Hash}
            label="Proyectos"
            value={String(selectedGroupProjectCount)}
          />
          <MetricTile
            icon={Clock}
            label="Actualizado"
            value={formatDateTime(selectedGroup.updatedAt)}
          />
        </div>

        {renderProjectStatusOverview({
          emptyMessage: 'Este grupo no contiene proyectos visibles.',
          groups: [selectedGroup, ...flattenGroups(selectedGroup.groups)],
          projects: flattenProjects([selectedGroup]),
          rootGroupId: selectedGroup.id,
        })}
      </section>
    )
  }

  function renderProjectDetail() {
    if (!selectedProject || !projectDetailValue) {
      return null
    }

    const runtimeStatus = selectedRuntimeState?.status ?? 'STOPPED'
    const workingDir = normalizeWindowsPath(
      selectedProject.workingDir ?? selectedProject.path,
    )
    const runtimeError = selectedRuntimeState?.lastError ?? null

    return (
      <section className="detail-stack">
        <header className="project-header">
          <div className="project-header-main">
            <div className="project-header-status-stack">
              <RuntimeStatusBadge status={runtimeStatus} variant="compact" />
            </div>
            <div className="project-header-copy">
              <p className="eyebrow">Proyecto</p>
              <span className="project-header-name">
                {selectedProject.name}
              </span>
              <div className="project-header-meta">
                <span className="project-header-meta-item" title={workingDir}>
                  <span>{workingDir}</span>
                </span>
                {selectedProjectGitInfo?.isRepository ? (
                  <span
                    className="project-header-meta-item"
                    title={selectedProjectGitInfo.branch ?? 'Sin rama activa'}
                  >
                    <GitBranch aria-hidden="true" className="icon" size={14} />
                    <span>
                      {selectedProjectGitInfo.branch ?? 'Sin rama activa'}
                    </span>
                  </span>
                ) : null}
                <span
                  className="project-header-meta-item"
                  title={`Actualizado ${formatDateTime(selectedProject.updatedAt)}`}
                >
                  <Clock aria-hidden="true" className="icon" size={14} />
                  <span>{formatDateTime(selectedProject.updatedAt)}</span>
                </span>
              </div>
            </div>
          </div>
          <div className="project-action-cluster">
            <BlockedActionButton
              aria-label="Start"
              blockedReason={startProjectBlockedReason}
              className="icon-button"
              disabled={!canStartProject}
              onClick={() =>
                void runtimeStore.actions.startProject(selectedProject.id)
              }
              title="Start"
              type="button"
            >
              <Play aria-hidden="true" size={18} />
            </BlockedActionButton>
            <BlockedActionButton
              aria-label="Stop"
              blockedReason={stopProjectBlockedReason}
              className="icon-button"
              disabled={!canStopProject}
              onClick={() =>
                void runtimeStore.actions.stopProject(selectedProject.id)
              }
              title="Stop"
              type="button"
            >
              <Square aria-hidden="true" size={18} />
            </BlockedActionButton>
            <BlockedActionButton
              aria-label="Restart"
              blockedReason={restartProjectBlockedReason}
              className="icon-button"
              disabled={!canRestartProject}
              onClick={() =>
                void runtimeStore.actions.restartProject(selectedProject.id)
              }
              title="Restart"
              type="button"
            >
              <RotateCcw aria-hidden="true" size={18} />
            </BlockedActionButton>
            <BlockedActionButton
              aria-label="Limpiar vista"
              blockedReason={clearProjectLogsBlockedReason}
              className="icon-button"
              disabled={!canClearProjectLogs}
              onClick={() => runtimeStore.actions.clearSelectedLogs()}
              title="Limpiar vista"
              type="button"
            >
              <TerminalIcon aria-hidden="true" size={18} />
            </BlockedActionButton>
            <span aria-hidden="true" className="action-divider" />
            <button
              aria-label="Configurar proyecto"
              className="icon-button"
              onClick={() => setActionModal('editProject')}
              title="Configurar proyecto"
              type="button"
            >
              <Settings2 aria-hidden="true" size={18} />
            </button>
            <button
              aria-label="Eliminar proyecto"
              className="icon-button danger"
              onClick={() =>
                openDeleteModal({
                  confirmLabel: 'Eliminar proyecto',
                  description: `Se eliminara el proyecto "${selectedProject.name}" de este workspace.`,
                  id: selectedProject.id,
                  kind: 'project',
                  title: 'Eliminar proyecto',
                })
              }
              title="Eliminar proyecto"
              type="button"
            >
              <Trash2 aria-hidden="true" size={18} />
            </button>
          </div>
        </header>

        <div className="project-metrics-strip">
          <MetricTile
            icon={runtimeStatusIcon(runtimeStatus)}
            label="Estado"
            tone={runtimeStatusTone(runtimeStatus)}
            value={runtimeStatus}
          />
          <MetricTile
            icon={Hash}
            label="PID"
            value={
              selectedRuntimeState?.pid != null
                ? String(selectedRuntimeState.pid)
                : '—'
            }
          />
          <MetricTile
            icon={AlertTriangle}
            label="Último error"
            tone={runtimeError ? 'error' : undefined}
            value={runtimeError ? 'Sí' : 'Sin errores'}
          >
            {runtimeError ? (
              <button
                aria-label="Ver historial de errores"
                className="metric-tile-action"
                onClick={() => setErrorHistoryProjectId(selectedProject.id)}
                title="Ver historial de errores"
                type="button"
              >
                <Eye aria-hidden="true" size={14} />
              </button>
            ) : null}
          </MetricTile>
        </div>

        {selectedProjectGitInfo?.isRepository ? (
          <article className="card detail-panel">
            <div className="section-title">
              <div>
                <p className="eyebrow">Repositorio</p>
                <h3>Git</h3>
              </div>
            </div>
            <ul className="detail-list">
              <li>
                <strong>Rama</strong>
                <span>
                  {selectedProjectGitInfo.branch ?? 'Sin rama activa'}
                </span>
              </li>
            </ul>
          </article>
        ) : null}

        <article className="card detail-panel">
          <div className="section-title">
            <div>
              <p className="eyebrow">Estado</p>
              <h3>Runtime</h3>
            </div>
          </div>
          <ul className="detail-list">
            <li>
              <strong>PID</strong>
              <span>{selectedRuntimeState?.pid ?? 'Sin proceso activo'}</span>
            </li>
            <li>
              <strong>Comando</strong>
              <span>
                {selectedRuntimeState?.commandPreview ?? projectCommand}
              </span>
            </li>
            <li>
              <strong>Working dir</strong>
              <span>{workingDir}</span>
            </li>
            <li>
              <strong>Último error runtime</strong>
              <span>{runtimeError ?? 'Sin errores registrados'}</span>
            </li>
          </ul>
        </article>

        <section className="card detail-panel">
          <div className="section-title">
            <div>
              <p className="eyebrow">Logs</p>
              <h3>Terminal integrada</h3>
            </div>
          </div>
          <ProjectLogsPanel lines={selectedRuntimeLogs} />
        </section>

        <section className="detail-columns detail-columns-full">
          <article className="card detail-panel">
            <div className="section-title">
              <div>
                <p className="eyebrow">Historial</p>
                <h3>Historial reciente</h3>
              </div>
              <p className="muted">{selectedRunHistory.length} ejecuciones</p>
            </div>
            {selectedRunHistory.length > 0 ? (
              <ul className="project-history-list">
                {selectedRunHistory.map((entry) => {
                  const tone = runHistoryTone(entry.finalRuntimeStatus)
                  const Icon = runHistoryIcon(tone)
                  return (
                    <li
                      className={`project-history-row is-${tone}`}
                      key={entry.id}
                    >
                      <Icon aria-hidden="true" className="icon" size={16} />
                      <span
                        className="project-history-cmd"
                        title={entry.commandPreview}
                      >
                        {entry.commandPreview}
                      </span>
                      <span className="project-history-meta">
                        {formatDateTime(entry.startedAt)}
                        {entry.exitCode !== null && entry.exitCode !== undefined
                          ? ` · exit ${entry.exitCode}`
                          : ''}
                      </span>
                    </li>
                  )
                })}
              </ul>
            ) : (
              <p className="muted">
                Todavía no hay ejecuciones registradas para este proyecto.
              </p>
            )}
          </article>
        </section>
      </section>
    )
  }

  function renderActionModal() {
    if (actionModal === 'renameWorkspace' && activeWorkspace) {
      return (
        <ModalFrame
          ariaLabel="Renombrar workspace"
          closeLabel="Cerrar renombrado del workspace"
          eyebrow="Edición"
          onClose={() => setActionModal(null)}
          title="Renombrar workspace"
        >
          <form
            className="stack"
            onSubmit={(event) => {
              event.preventDefault()
              if (!canSaveWorkspaceName) {
                return
              }

              void workspaceStore.actions
                .renameWorkspace(activeWorkspace.id, workspaceRenameCandidate)
                .then(() => setActionModal(null))
            }}
          >
            <label className="field">
              <span>Nombre</span>
              <input
                autoFocus
                onChange={(event) =>
                  setWorkspaceRenameDraft({
                    value: event.target.value,
                    workspaceId: activeWorkspace.id,
                  })
                }
                value={workspaceRenameValue}
              />
            </label>
            <div className="modal-actions">
              <button
                className="secondary"
                onClick={() => setActionModal(null)}
                type="button"
              >
                Cancelar
              </button>
              <BlockedActionButton
                blockedReason={saveWorkspaceNameBlockedReason}
                disabled={!canSaveWorkspaceName}
                type="submit"
              >
                Guardar nombre
              </BlockedActionButton>
            </div>
          </form>
        </ModalFrame>
      )
    }

    if (actionModal === 'createGroup' && activeWorkspace) {
      return (
        <ModalFrame
          ariaLabel="Crear grupo o subgrupo"
          closeLabel="Cerrar creación de grupo"
          eyebrow="Estructura"
          onClose={closeCreateGroupModal}
          title="Crear grupo o subgrupo"
        >
          <form
            className="stack"
            onSubmit={(event) => {
              event.preventDefault()
              if (!canCreateGroup) {
                return
              }

              void workspaceStore.actions
                .createGroup({
                  workspaceId: activeWorkspace.id,
                  parentGroupId: createGroupParentGroupId,
                  name: groupName.trim(),
                  color: DEFAULT_GROUP_COLOR,
                })
                .then(() => {
                  setGroupName('')
                  setParentGroupId('')
                  closeCreateGroupModal()
                })
            }}
          >
            <label className="field">
              <span>Nombre del grupo</span>
              <input
                autoFocus
                onChange={(event) => setGroupName(event.target.value)}
                placeholder="Frontend"
                value={groupName}
              />
            </label>
            <label className="field">
              <span>Grupo padre</span>
              <select
                onChange={(event) => setParentGroupId(event.target.value)}
                value={parentGroupId}
              >
                {createGroupRootGroup ? null : (
                  <option value="">Raíz del workspace</option>
                )}
                {createGroupParentOptions.map((group) => (
                  <option key={group.id} value={group.id}>
                    {group.name}
                  </option>
                ))}
              </select>
            </label>
            <div className="modal-actions">
              <button
                className="secondary"
                onClick={closeCreateGroupModal}
                type="button"
              >
                Cancelar
              </button>
              <BlockedActionButton
                blockedReason={createGroupBlockedReason}
                disabled={!canCreateGroup}
                type="submit"
              >
                Crear grupo
              </BlockedActionButton>
            </div>
          </form>
        </ModalFrame>
      )
    }

    if (actionModal === 'editGroup' && selectedGroup && groupDetailValue) {
      return (
        <ModalFrame
          ariaLabel="Configuración del grupo"
          closeLabel="Cerrar configuración del grupo"
          eyebrow="Edición"
          onClose={() => setActionModal(null)}
          title="Configuración del grupo"
        >
          <form
            className="stack"
            onSubmit={(event) => {
              event.preventDefault()
              if (!canSaveGroup) {
                return
              }

              void workspaceStore.actions
                .updateGroup(selectedGroup.id, {
                  name: groupNameCandidate,
                })
                .then(() => setActionModal(null))
            }}
          >
            <label className="field">
              <span>Nombre</span>
              <input
                autoFocus
                onChange={(event) =>
                  setGroupDetailDraft({
                    draft: { ...groupDetailValue, name: event.target.value },
                    groupId: selectedGroup.id,
                  })
                }
                value={groupDetailValue.name}
              />
            </label>
            <div className="modal-actions">
              <button
                className="secondary"
                onClick={() => setActionModal(null)}
                type="button"
              >
                Cancelar
              </button>
              <BlockedActionButton
                blockedReason={saveGroupBlockedReason}
                disabled={!canSaveGroup}
                type="submit"
              >
                Guardar grupo
              </BlockedActionButton>
            </div>
          </form>
        </ModalFrame>
      )
    }

    if (actionModal === 'importProject' && selectedGroup) {
      return (
        <ModalFrame
          ariaLabel="Importar proyecto"
          closeLabel="Cerrar importación del proyecto"
          eyebrow="Importación"
          onClose={() => setActionModal(null)}
          title="Importar proyecto"
        >
          <div className="stack">
            <label className="field">
              <span>Carpeta seleccionada</span>
              <input
                autoFocus
                onChange={(event) => setImportPath(event.target.value)}
                placeholder="C:\Proyectos\mi-proyecto"
                value={importPath}
              />
            </label>
            <div className="hero-actions">
              <button onClick={() => void handlePickFolder()} type="button">
                Seleccionar carpeta
              </button>
              <BlockedActionButton
                className="modal-action-primary"
                blockedReason={analyzeImportBlockedReason}
                disabled={Boolean(analyzeImportBlockedReason)}
                onClick={() => {
                  void workspaceStore.actions
                    .analyzeProjectFolder(cleanPathInput(importPath))
                    .then((result) => {
                      setReviewDraft(
                        createReviewDraft(result, selectedGroup.id),
                      )
                      setImportPath(normalizeWindowsPath(result.path))
                      setActionModal(null)
                    })
                }}
                type="button"
              >
                Analizar carpeta
              </BlockedActionButton>
            </div>
            <p className="muted">
              El análisis inspecciona ficheros clave para proponer la
              configuración inicial del proyecto antes de guardarlo.
            </p>
          </div>
        </ModalFrame>
      )
    }

    if (
      actionModal === 'editProject' &&
      selectedProject &&
      projectDetailValue
    ) {
      return (
        <ModalFrame
          ariaLabel="Configuración del proyecto"
          className="modal-card-wide"
          closeLabel="Cerrar configuración del proyecto"
          eyebrow="Edición"
          onClose={() => setActionModal(null)}
          title="Configuración del proyecto"
        >
          <form
            className="review-grid"
            onSubmit={(event) => {
              event.preventDefault()
              if (!canSaveProject) {
                return
              }

              const args = parseArgsInput(projectDetailValue.argsText)
              const executable = projectDetailValue.executable.trim()
              const nextCommand = formatCommandPreview(executable, args)
              const projectId = selectedProject.id

              void (async () => {
                await workspaceStore.actions.updateProject(projectId, {
                  args: args.length > 0 ? args : undefined,
                  command:
                    nextCommand === 'Manual review required'
                      ? null
                      : nextCommand,
                  executable: executable || null,
                  name: projectDetailValue.name.trim(),
                  path: cleanPathInput(projectDetailValue.path),
                  workingDir:
                    cleanPathInput(projectDetailValue.workingDir) || null,
                })
                runtimeStore.actions.clearSavedProjectRuntimeSnapshot(projectId)
                setActionModal(null)
              })()
            }}
          >
            <label className="field">
              <span>Nombre</span>
              <input
                autoFocus
                onChange={(event) =>
                  setProjectDetailDraft({
                    draft: { ...projectDetailValue, name: event.target.value },
                    projectId: selectedProject.id,
                  })
                }
                value={projectDetailValue.name}
              />
            </label>
            <label className="field">
              <span>Ruta</span>
              <input
                onChange={(event) =>
                  setProjectDetailDraft({
                    draft: { ...projectDetailValue, path: event.target.value },
                    projectId: selectedProject.id,
                  })
                }
                value={projectDetailValue.path}
              />
            </label>
            <label className="field">
              <span>Working dir</span>
              <input
                onChange={(event) =>
                  setProjectDetailDraft({
                    draft: {
                      ...projectDetailValue,
                      workingDir: event.target.value,
                    },
                    projectId: selectedProject.id,
                  })
                }
                value={projectDetailValue.workingDir}
              />
            </label>
            <label className="field">
              <span>Ejecutable</span>
              <input
                onChange={(event) =>
                  setProjectDetailDraft({
                    draft: {
                      ...projectDetailValue,
                      executable: event.target.value,
                    },
                    projectId: selectedProject.id,
                  })
                }
                placeholder="pnpm"
                value={projectDetailValue.executable}
              />
            </label>
            <label className="field full-width">
              <span>Argumentos de arranque (uno por línea)</span>
              <textarea
                onChange={(event) =>
                  setProjectDetailDraft({
                    draft: {
                      ...projectDetailValue,
                      argsText: event.target.value,
                    },
                    projectId: selectedProject.id,
                  })
                }
                rows={4}
                value={projectDetailValue.argsText}
              />
            </label>
            <article className="info-card full-width">
              <h4>Comando resultante</h4>
              <p>{projectCommand}</p>
            </article>
            <div className="modal-actions full-width">
              <button
                className="secondary"
                onClick={() => setActionModal(null)}
                type="button"
              >
                Cancelar
              </button>
              <BlockedActionButton
                blockedReason={saveProjectBlockedReason}
                disabled={!canSaveProject}
                type="submit"
              >
                Guardar proyecto
              </BlockedActionButton>
            </div>
          </form>
        </ModalFrame>
      )
    }

    return null
  }

  function renderRightColumn() {
    if (resolvedSelection?.type === 'group') {
      return renderGroupDetail()
    }

    if (resolvedSelection?.type === 'project') {
      return renderProjectDetail()
    }

    return renderWorkspaceDetail()
  }

  function renderWorkspaceModal() {
    if (!isWorkspaceModalOpen) {
      return null
    }

    return (
      <ModalFrame
        ariaLabel="Nuevo workspace"
        closeLabel="Cerrar workspace"
        eyebrow="Workspace"
        onClose={() => setIsWorkspaceModalOpen(false)}
        title="Nuevo workspace"
      >
        <form
          className="stack"
          onSubmit={(event) => {
            event.preventDefault()
            void handleCreateWorkspace()
          }}
        >
          <label className="field">
            <span>Nombre del workspace</span>
            <input
              autoFocus
              onChange={(event) => setWorkspaceName(event.target.value)}
              placeholder="Nuevo workspace"
              value={workspaceName}
            />
          </label>
          <div className="modal-actions">
            <BlockedActionButton
              blockedReason={createWorkspaceBlockedReason}
              disabled={Boolean(createWorkspaceBlockedReason)}
              type="submit"
            >
              Crear workspace
            </BlockedActionButton>
          </div>
        </form>
      </ModalFrame>
    )
  }

  function renderDeleteModal() {
    if (!deleteModal) {
      return null
    }

    return (
      <ModalFrame
        ariaLabel={deleteModal.title}
        closeLabel={`Cerrar ${deleteModal.title.toLowerCase()}`}
        eyebrow="Confirmación"
        onClose={() => setDeleteModal(null)}
        title={deleteModal.title}
      >
        <div className="stack">
          <p className="muted">{deleteModal.description}</p>
          <div className="modal-actions">
            <button
              className="secondary"
              onClick={() => setDeleteModal(null)}
              type="button"
            >
              Cancelar
            </button>
            <button
              className="danger"
              onClick={() => void handleConfirmDelete()}
              type="button"
            >
              {deleteModal.confirmLabel}
            </button>
          </div>
        </div>
      </ModalFrame>
    )
  }

  function renderMoveModal() {
    if (!moveModal) {
      return null
    }

    return (
      <ModalFrame
        ariaLabel={moveModal.title}
        closeLabel={`Cerrar ${moveModal.title.toLowerCase()}`}
        eyebrow="Confirmación"
        onClose={() => setMoveModal(null)}
        title={moveModal.title}
      >
        <div className="stack">
          <p className="muted">{moveModal.description}</p>
          <div className="modal-actions">
            <button
              className="secondary"
              onClick={() => setMoveModal(null)}
              type="button"
            >
              Cancelar
            </button>
            <button onClick={() => void handleConfirmMove()} type="button">
              Mover
            </button>
          </div>
        </div>
      </ModalFrame>
    )
  }

  function renderRuntimeErrorHistoryModal() {
    if (!selectedProject || errorHistoryProjectId !== selectedProject.id) {
      return null
    }

    const runtimeErrorHistory = buildRuntimeErrorHistory(
      selectedRuntimeState,
      selectedRunHistory,
    )

    return (
      <ModalFrame
        ariaLabel="Historial de errores"
        className="modal-card-wide"
        closeLabel="Cerrar historial de errores"
        eyebrow="Errores"
        onClose={() => setErrorHistoryProjectId(null)}
        title="Historial de errores"
      >
        <div className="stack">
          <p className="muted">{selectedProject.name}</p>
          {runtimeErrorHistory.length > 0 ? (
            <ul className="runtime-error-history-list">
              {runtimeErrorHistory.map((item) => (
                <li className="runtime-error-history-item" key={item.id}>
                  <div className="runtime-error-history-header">
                    <span className="runtime-error-history-source">
                      {item.source === 'current' ? 'Actual' : 'Historial'}
                    </span>
                    <strong title={item.commandPreview}>
                      {item.commandPreview}
                    </strong>
                    <span>{formatDateTime(item.occurredAt, 'Sin fecha')}</span>
                  </div>
                  <p>{item.message}</p>
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted">No hay errores registrados en el historial.</p>
          )}
        </div>
      </ModalFrame>
    )
  }

  return (
    <>
      <main
        className={`workspace-shell app-shell${
          isNavigatorResizing ? ' is-resizing-navigator' : ''
        }`}
        ref={workspaceShellRef}
        style={workspaceShellStyle}
      >
        <aside className="sidebar explorer-sidebar" ref={explorerSidebarRef}>
          <div className="sidebar-header">
            <div className="app-navigation-header">
              <div className="app-title-block">
                <p className="eyebrow">Navegador</p>
                <h1>La Centralita</h1>
              </div>
              <div
                aria-label="Historial de navegacion"
                className="app-navigation-controls"
              >
                <button
                  aria-label="Navegar atras"
                  className="icon-button app-navigation-button"
                  disabled={previousNavigationIndex === null}
                  onClick={() =>
                    void handleNavigateHistory(previousNavigationIndex)
                  }
                  title="Navegar atras"
                  type="button"
                >
                  <ArrowLeft aria-hidden="true" size={17} />
                </button>
                <button
                  aria-label="Navegar adelante"
                  className="icon-button app-navigation-button"
                  disabled={nextNavigationIndex === null}
                  onClick={() =>
                    void handleNavigateHistory(nextNavigationIndex)
                  }
                  title="Navegar adelante"
                  type="button"
                >
                  <ArrowRight aria-hidden="true" size={17} />
                </button>
              </div>
            </div>
            <p className="lead"></p>
          </div>

          <section className="card tree-panel explorer-panel">
            <div className="section-title navigator-section-title">
              <div className="navigator-header-title">
                <p className="eyebrow">Árbol</p>
                <h2>Workspaces, grupos y proyectos</h2>
              </div>
              <div className="navigator-header-actions">
                <button
                  aria-label="+ Nuevo workspace"
                  className={
                    hasNoConfiguredWorkspaces
                      ? 'workspace-create-button is-expanded'
                      : 'workspace-create-button'
                  }
                  onClick={() => setIsWorkspaceModalOpen(true)}
                  type="button"
                >
                  <span aria-hidden="true" className="workspace-create-icon">
                    +
                  </span>
                  <span aria-hidden="true" className="workspace-create-label">
                    Nuevo workspace
                  </span>
                </button>
                {hasNoConfiguredWorkspaces ? null : (
                  <div className="navigator-filter-controls">
                    <label className="field navigator-filter-field">
                      <span>Filtrar por estado</span>
                      <select
                        onChange={(event) =>
                          runtimeStore.actions.setRuntimeFilter(
                            event.target
                              .value as typeof runtimeStore.runtimeFilter,
                          )
                        }
                        value={runtimeStore.runtimeFilter}
                      >
                        <option value="ALL">Todos</option>
                        <option value="RUNNING">RUNNING</option>
                        <option value="STARTING">STARTING</option>
                        <option value="STOPPING">STOPPING</option>
                        <option value="STOPPED">STOPPED</option>
                        <option value="FAILED">FAILED</option>
                      </select>
                    </label>
                  </div>
                )}
              </div>
            </div>
            <WorkspaceRuntimeTreeView
              activeWorkspaceId={workspaceStore.activeWorkspaceId}
              isLoadingPersistedState={workspaceStore.isLoading}
              onEnsureWorkspaceTree={(workspaceId) =>
                workspaceStore.actions.ensureWorkspaceTree(workspaceId)
              }
              onRequestMove={(source, target) =>
                handleRequestMove(source, target)
              }
              onSelectGroup={(groupId, workspaceId) =>
                void handleSelectGroup(groupId, workspaceId)
              }
              onSelectProject={(projectId, workspaceId) =>
                void handleSelectProject(projectId, workspaceId)
              }
              onSelectWorkspace={(workspaceId) =>
                handleSelectWorkspace(workspaceId)
              }
              runtimeFilter={runtimeStore.runtimeFilter}
              selectedItem={resolvedSelection}
              statusByProjectId={runtimeStore.statusByProjectId}
              workspaceStatus={runtimeStore.workspaceStatus}
              workspaceTrees={workspaceStore.treesByWorkspaceId}
              workspaces={workspaceStore.workspaces}
            />
          </section>
        </aside>

        <div
          aria-label="Redimensionar navegador"
          aria-orientation="vertical"
          className="navigator-resize-handle"
          onKeyDown={handleNavigatorResizeKeyDown}
          onPointerCancel={finishNavigatorResize}
          onPointerDown={handleNavigatorResizePointerDown}
          onPointerMove={handleNavigatorResizePointerMove}
          onPointerUp={finishNavigatorResize}
          role="separator"
          tabIndex={0}
          title="Redimensionar navegador"
        />

        <section className="main-panel detail-column" ref={detailColumnRef}>
          {workspaceStore.error ? (
            <p className="error-banner">{workspaceStore.error}</p>
          ) : null}
          {runtimeStore.error ? (
            <p className="error-banner">{runtimeStore.error}</p>
          ) : null}
          {workspaceStore.isAnalyzing ? (
            <p className="empty-state">Analizando carpeta seleccionada...</p>
          ) : null}
          {renderRightColumn()}
        </section>
      </main>
      {renderWorkspaceModal()}
      {renderActionModal()}
      {renderDetectionReviewModal()}
      {renderDeleteModal()}
      {renderMoveModal()}
      {renderRuntimeErrorHistoryModal()}
    </>
  )
}

export default CentralitaApp
