import {
  startTransition,
  useEffect,
  useEffectEvent,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from 'react'
import {
  getProjectLogs,
  getWorkspaceRuntimeStatus,
  listProjectRunHistory,
  restartProject,
  startGroup,
  startProject,
  startWorkspace,
  stopGroup,
  stopProject,
  stopWorkspace,
} from '../features/runtime/api'
import { aggregateRuntimeStatus, flattenProjects } from '../features/workspace/tree'
import { listenRuntimeEvent, RUNTIME_EVENTS } from '../shared/api/tauri'
import type {
  GroupTreeNode,
  ProcessRuntimeState,
  RunHistoryEntry,
  RuntimeLogLine,
  RuntimeProcessErrorEvent,
  RuntimeProcessExitedEvent,
  RuntimeStatus,
  RuntimeStatusEvent,
} from '../types'

const MAX_LOG_LINES = 500
const HISTORY_LIMIT = 10

export type RuntimeFilter = RuntimeStatus | 'ALL'

type RuntimeState = {
  error: string | null
  historyByProjectId: Record<string, RunHistoryEntry[]>
  logsByProjectId: Record<string, RuntimeLogLine[]>
  runtimeFilter: RuntimeFilter
  selectedProjectId: string | null
  statusByProjectId: Record<string, ProcessRuntimeState>
}

const initialState: RuntimeState = {
  error: null,
  historyByProjectId: {},
  logsByProjectId: {},
  runtimeFilter: 'ALL',
  selectedProjectId: null,
  statusByProjectId: {},
}

function mergeLogLine(lines: RuntimeLogLine[], line: RuntimeLogLine) {
  const lastLine = lines.at(-1)
  if (
    lastLine?.timestamp === line.timestamp &&
    lastLine.stream === line.stream &&
    lastLine.line === line.line &&
    Boolean(lastLine.partial) === Boolean(line.partial)
  ) {
    return lines
  }

  const nextLines = [...lines, line]
  return nextLines.length > MAX_LOG_LINES
    ? nextLines.slice(-MAX_LOG_LINES)
    : nextLines
}

function dedupeLogLines(lines: RuntimeLogLine[]) {
  const seen = new Set<string>()

  return lines.filter((line) => {
    const key = `${line.timestamp}|${line.stream}|${line.partial ? 'partial' : 'line'}|${
      line.line
    }`
    if (seen.has(key)) {
      return false
    }

    seen.add(key)
    return true
  })
}

function mergeHistoryEntry(entries: RunHistoryEntry[], entry: RunHistoryEntry) {
  const nextEntries = [entry, ...entries.filter((item) => item.id !== entry.id)]
  return nextEntries
    .sort((left, right) => right.startedAt.localeCompare(left.startedAt))
    .slice(0, HISTORY_LIMIT)
}

function indexStates(projects: ProcessRuntimeState[]) {
  return Object.fromEntries(
    projects.map((project) => [project.projectId, project]),
  )
}

function isActiveRuntimeStatus(status: RuntimeStatus) {
  return status === 'STARTING' || status === 'RUNNING' || status === 'STOPPING'
}

function clearSavedProjectRuntimeSnapshot(
  current: RuntimeState,
  projectId: string,
) {
  const snapshot = current.statusByProjectId[projectId]
  const nextStatusByProjectId = { ...current.statusByProjectId }
  const nextLogsByProjectId = { ...current.logsByProjectId }

  if (
    snapshot &&
    snapshot.status !== 'STARTING' &&
    snapshot.status !== 'RUNNING' &&
    snapshot.status !== 'STOPPING'
  ) {
    delete nextStatusByProjectId[projectId]
  }

  nextLogsByProjectId[projectId] = []

  return {
    ...current,
    logsByProjectId: nextLogsByProjectId,
    statusByProjectId: nextStatusByProjectId,
  }
}

async function syncWorkspaceRuntimeState(
  activeWorkspaceId: string | null,
  projectIds: string[],
  setState: Dispatch<SetStateAction<RuntimeState>>,
) {
  if (!activeWorkspaceId) {
    startTransition(() =>
      setState((current) => ({
        ...initialState,
        runtimeFilter: current.runtimeFilter,
      })),
    )
    return
  }

  try {
    const workspaceRuntime = await getWorkspaceRuntimeStatus({
      workspaceId: activeWorkspaceId,
    })

    startTransition(() => {
      setState((current) => ({
        ...current,
        error: null,
        selectedProjectId:
          current.selectedProjectId &&
          projectIds.includes(current.selectedProjectId)
            ? current.selectedProjectId
            : (projectIds[0] ?? null),
        statusByProjectId: indexStates(workspaceRuntime.projects),
      }))
    })
  } catch (error) {
    setState((current) => ({
      ...current,
      error:
        error instanceof Error
          ? error.message
          : 'No se pudo cargar el runtime.',
    }))
  }
}

export function useRuntimeStore(
  activeWorkspaceId: string | null,
  groups: GroupTreeNode[],
) {
  const [state, setState] = useState<RuntimeState>(initialState)
  const runtimeSyncRequestIdRef = useRef(0)
  const projectArtifactsRequestIdRef = useRef(0)
  const projects = flattenProjects(groups)
  const projectIdsKey = projects.map((project) => project.id).join('|')

  async function loadProjectHistory(projectId: string) {
    try {
      const history = await listProjectRunHistory({
        projectId,
        limit: HISTORY_LIMIT,
      })
      startTransition(() => {
        setState((current) => ({
          ...current,
          historyByProjectId: {
            ...current.historyByProjectId,
            [projectId]: history,
          },
        }))
      })
    } catch (error) {
      setState((current) => ({
        ...current,
        error:
          error instanceof Error
            ? error.message
            : 'No se pudo cargar el historial.',
      }))
    }
  }

  async function loadProjectArtifacts(projectId: string, requestId: number) {
    try {
      const [logs, history] = await Promise.all([
        getProjectLogs({ projectId }),
        listProjectRunHistory({ projectId, limit: HISTORY_LIMIT }),
      ])

      startTransition(() => {
        setState((current) => {
          if (
            projectArtifactsRequestIdRef.current !== requestId ||
            current.selectedProjectId !== projectId
          ) {
            return current
          }

          return {
            ...current,
            error: null,
            historyByProjectId: {
              ...current.historyByProjectId,
              [projectId]: history,
            },
            logsByProjectId: {
              ...current.logsByProjectId,
              [projectId]: dedupeLogLines(logs),
            },
          }
        })
      })
    } catch (error) {
      setState((current) => {
        if (
          projectArtifactsRequestIdRef.current !== requestId ||
          current.selectedProjectId !== projectId
        ) {
          return current
        }

        return {
          ...current,
          error:
            error instanceof Error
              ? error.message
              : 'No se pudieron cargar logs e historial.',
        }
      })
    }
  }

  const handleStatusChanged = useEffectEvent((payload: RuntimeStatusEvent) => {
    startTransition(() => {
      setState((current) => {
        const previousStatus = current.statusByProjectId[payload.projectId]
        const nextLogsByProjectId =
          payload.status === 'STARTING'
            ? {
                ...current.logsByProjectId,
                [payload.projectId]: [],
              }
            : current.logsByProjectId
        const nextStatus: ProcessRuntimeState = {
          ...(previousStatus ?? {
            projectId: payload.projectId,
            commandPreview: payload.commandPreview,
          }),
          commandPreview: payload.commandPreview,
          status: payload.status,
          pid: payload.pid ?? previousStatus?.pid ?? null,
          lastError:
            payload.status === 'FAILED'
              ? (payload.message ?? previousStatus?.lastError ?? null)
              : payload.status === 'STOPPED'
                ? null
                : payload.status === 'STARTING' || payload.status === 'RUNNING'
                  ? null
                  : (previousStatus?.lastError ?? null),
          exitCode:
            payload.status === 'STARTING' || payload.status === 'RUNNING'
              ? null
              : (previousStatus?.exitCode ?? null),
          startedAt:
            payload.status === 'STARTING'
              ? payload.timestamp
              : payload.status === 'RUNNING'
                ? (previousStatus?.startedAt ?? payload.timestamp)
                : (previousStatus?.startedAt ?? null),
          stoppedAt:
            payload.status === 'STARTING' || payload.status === 'RUNNING'
              ? null
              : (previousStatus?.stoppedAt ?? null),
        }

        return {
          ...current,
          logsByProjectId: nextLogsByProjectId,
          statusByProjectId: {
            ...current.statusByProjectId,
            [payload.projectId]: nextStatus,
          },
        }
      })
    })
  })

  const handleProcessExited = useEffectEvent(
    (payload: RuntimeProcessExitedEvent) => {
      startTransition(() => {
        setState((current) => ({
          ...current,
          statusByProjectId: {
            ...current.statusByProjectId,
            [payload.projectId]: {
              ...(current.statusByProjectId[payload.projectId] ?? {
                projectId: payload.projectId,
                commandPreview: payload.commandPreview,
              }),
              commandPreview: payload.commandPreview,
              status: payload.status,
              pid: payload.pid ?? null,
              exitCode: payload.exitCode ?? null,
              stoppedAt: payload.timestamp,
              lastError:
                payload.status === 'FAILED' ? (payload.message ?? null) : null,
            },
          },
        }))
      })

      void loadProjectHistory(payload.projectId)
    },
  )

  const handleProcessError = useEffectEvent(
    (payload: RuntimeProcessErrorEvent) => {
      startTransition(() => {
        setState((current) => {
          const previousStatus = current.statusByProjectId[payload.projectId]
          const shouldPreserveActiveStatus =
            previousStatus &&
            isActiveRuntimeStatus(previousStatus.status) &&
            payload.status === 'FAILED'

          return {
            ...current,
            statusByProjectId: {
              ...current.statusByProjectId,
              [payload.projectId]: {
                ...(previousStatus ?? {
                  projectId: payload.projectId,
                  commandPreview: payload.commandPreview,
                }),
                commandPreview: payload.commandPreview,
                status: shouldPreserveActiveStatus
                  ? previousStatus.status
                  : payload.status,
                pid: payload.pid ?? previousStatus?.pid ?? null,
                lastError: payload.message,
              },
            },
          }
        })
      })

      void loadProjectHistory(payload.projectId)
    },
  )

  const handleHistoryAppended = useEffectEvent((payload: RunHistoryEntry) => {
    startTransition(() => {
      setState((current) => ({
        ...current,
        historyByProjectId: {
          ...current.historyByProjectId,
          [payload.projectId]: mergeHistoryEntry(
            current.historyByProjectId[payload.projectId] ?? [],
            payload,
          ),
        },
      }))
    })
  })

  const handleLogLine = useEffectEvent((payload: RuntimeLogLine) => {
    startTransition(() => {
      setState((current) => ({
        ...current,
        logsByProjectId: {
          ...current.logsByProjectId,
          [payload.projectId]: mergeLogLine(
            current.logsByProjectId[payload.projectId] ?? [],
            payload,
          ),
        },
      }))
    })
  })

  useEffect(() => {
    const projectIds = projectIdsKey.length > 0 ? projectIdsKey.split('|') : []
    const requestId = runtimeSyncRequestIdRef.current + 1
    runtimeSyncRequestIdRef.current = requestId

    if (!activeWorkspaceId) {
      startTransition(() =>
        setState((current) => ({
          ...initialState,
          runtimeFilter: current.runtimeFilter,
        })),
      )
      return
    }

    void (async () => {
      try {
        const workspaceRuntime = await getWorkspaceRuntimeStatus({
          workspaceId: activeWorkspaceId,
        })

        startTransition(() => {
          setState((current) => {
            if (runtimeSyncRequestIdRef.current !== requestId) {
              return current
            }

            return {
              ...current,
              error: null,
              selectedProjectId:
                current.selectedProjectId &&
                projectIds.includes(current.selectedProjectId)
                  ? current.selectedProjectId
                  : (projectIds[0] ?? null),
              statusByProjectId: indexStates(workspaceRuntime.projects),
            }
          })
        })
      } catch (error) {
        setState((current) => {
          if (runtimeSyncRequestIdRef.current !== requestId) {
            return current
          }

          return {
            ...current,
            error:
              error instanceof Error
                ? error.message
                : 'No se pudo cargar el runtime.',
          }
        })
      }
    })()
  }, [activeWorkspaceId, projectIdsKey])

  useEffect(() => {
    if (!state.selectedProjectId) {
      projectArtifactsRequestIdRef.current += 1
      return
    }

    const requestId = projectArtifactsRequestIdRef.current + 1
    projectArtifactsRequestIdRef.current = requestId
    void loadProjectArtifacts(state.selectedProjectId, requestId)
  }, [state.selectedProjectId])

  useEffect(() => {
    const unlisteners: Array<() => void> = []
    let disposed = false

    void Promise.all([
      listenRuntimeEvent(RUNTIME_EVENTS.statusChanged, (payload) =>
        handleStatusChanged(payload),
      ),
      listenRuntimeEvent(RUNTIME_EVENTS.logLine, (payload) =>
        handleLogLine(payload),
      ),
      listenRuntimeEvent(RUNTIME_EVENTS.processExited, (payload) =>
        handleProcessExited(payload),
      ),
      listenRuntimeEvent(RUNTIME_EVENTS.processError, (payload) =>
        handleProcessError(payload),
      ),
      listenRuntimeEvent(RUNTIME_EVENTS.historyAppended, (payload) =>
        handleHistoryAppended(payload),
      ),
    ]).then((nextUnlisteners) => {
      if (disposed) {
        nextUnlisteners.forEach((unlisten) => {
          void unlisten()
        })
        return
      }

      unlisteners.push(...nextUnlisteners)
    })

    return () => {
      disposed = true
      unlisteners.forEach((unlisten) => {
        void unlisten()
      })
    }
  }, [])

  const runtimeStatuses = projects.map(
    (project) => state.statusByProjectId[project.id]?.status ?? 'STOPPED',
  )
  const workspaceStatus: RuntimeStatus = aggregateRuntimeStatus(runtimeStatuses)

  return {
    error: state.error,
    historyByProjectId: state.historyByProjectId,
    logsByProjectId: state.logsByProjectId,
    runtimeFilter: state.runtimeFilter,
    selectedProjectId: state.selectedProjectId,
    statusByProjectId: state.statusByProjectId,
    workspaceStatus,
    actions: {
      clearSavedProjectRuntimeSnapshot: (projectId: string) =>
        setState((current) =>
          clearSavedProjectRuntimeSnapshot(current, projectId),
        ),
      clearSelectedLogs: () => {
        if (!state.selectedProjectId) {
          return
        }

        setState((current) => ({
          ...current,
          logsByProjectId: {
            ...current.logsByProjectId,
            [state.selectedProjectId!]: [],
          },
        }))
      },
      selectProject: (projectId: string) =>
        setState((current) =>
          current.selectedProjectId === projectId
            ? current
            : { ...current, selectedProjectId: projectId },
        ),
      setRuntimeFilter: (runtimeFilter: RuntimeFilter) =>
        setState((current) => ({ ...current, runtimeFilter })),
      startGroup: (groupId: string) => startGroup({ groupId }),
      startProject: (projectId: string) => {
        setState((current) =>
          clearSavedProjectRuntimeSnapshot(current, projectId),
        )
        return startProject({ projectId })
      },
      startWorkspace: (workspaceId: string) => startWorkspace({ workspaceId }),
      stopGroup: (groupId: string) => stopGroup({ groupId }),
      stopProject: (projectId: string) => stopProject({ projectId }),
      stopWorkspace: (workspaceId: string) => stopWorkspace({ workspaceId }),
      syncWorkspaceRuntime: () => {
        const projectIds =
          projectIdsKey.length > 0 ? projectIdsKey.split('|') : []
        return syncWorkspaceRuntimeState(
          activeWorkspaceId,
          projectIds,
          setState,
        )
      },
      restartProject: (projectId: string) => {
        setState((current) =>
          clearSavedProjectRuntimeSnapshot(current, projectId),
        )
        return restartProject({ projectId })
      },
    },
  }
}
