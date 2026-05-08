import { useCallback, useEffect, useRef, useState, type MouseEvent, type PointerEvent } from 'react'
import { groupRuntimeStatus } from '../../features/workspace/tree'
import type {
  GroupTreeNode,
  ProcessRuntimeState,
  RuntimeStatus,
  Workspace,
  WorkspaceTree,
} from '../../types'
import { RuntimeStatusBadge } from './RuntimeStatusBadge'

type TreeSelection =
  | { id: string; type: 'group' }
  | { id: string; type: 'project' }
  | { id: string; type: 'workspace' }
  | null

type ClickLikeEvent = Pick<MouseEvent<HTMLElement>, 'preventDefault' | 'stopPropagation'>

export type NavigatorDragItem =
  | {
      id: string
      name: string
      parentGroupId: string | null
      type: 'group'
      workspaceId: string
    }
  | {
      groupId: string
      id: string
      name: string
      type: 'project'
      workspaceId: string
    }

export type NavigatorDropTarget =
  | { id: string; name: string; type: 'group'; workspaceId: string }
  | { id: string; name: string; type: 'workspace' }

type WorkspaceRuntimeTreeViewProps = {
  activeWorkspaceId: string | null
  isLoadingPersistedState: boolean
  onEnsureWorkspaceTree: (workspaceId: string) => Promise<unknown>
  onRequestMove: (
    source: NavigatorDragItem,
    target: NavigatorDropTarget,
  ) => void | Promise<unknown>
  onSelectGroup: (groupId: string, workspaceId: string) => void | Promise<unknown>
  onSelectProject: (projectId: string, workspaceId: string) => void | Promise<unknown>
  onSelectWorkspace: (workspaceId: string) => void | Promise<unknown>
  runtimeFilter: RuntimeStatus | 'ALL'
  selectedItem: TreeSelection
  statusByProjectId: Record<string, ProcessRuntimeState>
  workspaceStatus: RuntimeStatus
  workspaceTrees: Record<string, WorkspaceTree>
  workspaces: Workspace[]
}

function normalizeWindowsPath(path: string) {
  if (path.startsWith('\\\\?\\UNC\\')) {
    return `\\\\${path.slice('\\\\?\\UNC\\'.length)}`
  }

  if (path.startsWith('\\\\?\\')) {
    return path.slice('\\\\?\\'.length)
  }

  return path
}

function dropTargetKey(target: NavigatorDropTarget) {
  return `${target.type}:${target.id}`
}

function WorkspaceTreeMessage({
  message,
}: {
  message: string
}) {
  return (
    <li className="navigator-item">
      <div className="navigator-row navigator-row-empty-state">
        <div className="navigator-node navigator-node-empty-state">
          <div className="tree-node-copy tree-node-copy-empty-state">
            <strong>{message}</strong>
          </div>
        </div>
      </div>
    </li>
  )
}

function dragItemKey(item: NavigatorDragItem | null) {
  if (!item) {
    return null
  }

  return `${item.type}:${item.id}`
}

function canDropOnTarget(item: NavigatorDragItem | null, target: NavigatorDropTarget) {
  if (!item) {
    return false
  }

  if (item.type === 'project') {
    return target.type === 'group'
  }

  return target.type === 'group' || target.type === 'workspace'
}

function canDropGroupOnWorkspaceRoot(item: NavigatorDragItem | null, workspaceId: string) {
  if (!item || item.type !== 'group') {
    return false
  }

  return !(item.workspaceId === workspaceId && item.parentGroupId === null)
}

function isGroupTargetDisabled(
  item: NavigatorDragItem | null,
  groupId: string,
  ancestorGroupIds: string[],
) {
  if (!item || item.type !== 'group') {
    return false
  }

  return item.id === groupId || ancestorGroupIds.includes(item.id)
}

function findGroupAncestorIds(
  groups: GroupTreeNode[],
  groupId: string,
  ancestorIds: string[] = [],
): string[] | null {
  for (const group of groups) {
    if (group.id === groupId) {
      return ancestorIds
    }

    const childPath = findGroupAncestorIds(group.groups, groupId, [...ancestorIds, group.id])
    if (childPath) {
      return childPath
    }
  }

  return null
}

function findProjectAncestorIds(
  groups: GroupTreeNode[],
  projectId: string,
  ancestorIds: string[] = [],
): string[] | null {
  for (const group of groups) {
    if (group.projects.some((project) => project.id === projectId)) {
      return [...ancestorIds, group.id]
    }

    const childPath = findProjectAncestorIds(group.groups, projectId, [...ancestorIds, group.id])
    if (childPath) {
      return childPath
    }
  }

  return null
}

export function WorkspaceRuntimeTreeView({
  activeWorkspaceId,
  isLoadingPersistedState,
  onEnsureWorkspaceTree,
  onRequestMove,
  onSelectGroup,
  onSelectProject,
  onSelectWorkspace,
  runtimeFilter,
  selectedItem,
  statusByProjectId,
  workspaceStatus,
  workspaceTrees,
  workspaces,
}: WorkspaceRuntimeTreeViewProps) {
  const [expandedWorkspaceIds, setExpandedWorkspaceIds] = useState<string[]>([])
  const [collapsedWorkspaceIds, setCollapsedWorkspaceIds] = useState<string[]>([])
  const [expandedGroupIds, setExpandedGroupIds] = useState<string[]>([])
  const [draggedItem, setDraggedItem] = useState<NavigatorDragItem | null>(null)
  const [activeDropTargetKey, setActiveDropTargetKey] = useState<string | null>(null)
  const draggedItemRef = useRef<NavigatorDragItem | null>(null)
  const dragFeedbackRef = useRef<{ cursor: string; userSelect: string } | null>(null)
  const suppressClickRef = useRef(false)
  const suppressClickTimeoutRef = useRef<number | null>(null)
  const draggedItemKey = dragItemKey(draggedItem)
  const selectedItemId = selectedItem?.id ?? null
  const selectedItemType = selectedItem?.type ?? null

  const setGlobalDragCursor = useCallback((cursor: 'not-allowed' | 'pointer') => {
    if (typeof document === 'undefined') {
      return
    }

    if (!dragFeedbackRef.current) {
      dragFeedbackRef.current = {
        cursor: document.body.style.cursor,
        userSelect: document.body.style.userSelect,
      }
    }

    document.body.style.cursor = cursor
    document.body.style.userSelect = 'none'
  }, [])

  const restoreGlobalDragFeedback = useCallback(() => {
    if (typeof document === 'undefined' || !dragFeedbackRef.current) {
      return
    }

    document.body.style.cursor = dragFeedbackRef.current.cursor
    document.body.style.userSelect = dragFeedbackRef.current.userSelect
    dragFeedbackRef.current = null
  }, [])

  const clearPointerDrag = useCallback(
    (options?: { suppressClick?: boolean }) => {
      draggedItemRef.current = null
      setDraggedItem(null)
      setActiveDropTargetKey(null)
      if (options?.suppressClick) {
        suppressClickRef.current = true
        if (suppressClickTimeoutRef.current !== null) {
          window.clearTimeout(suppressClickTimeoutRef.current)
        }
        suppressClickTimeoutRef.current = window.setTimeout(() => {
          suppressClickRef.current = false
          suppressClickTimeoutRef.current = null
        }, 0)
      } else {
        suppressClickRef.current = false
      }
      restoreGlobalDragFeedback()
    },
    [restoreGlobalDragFeedback],
  )

  const updatePointerDropTarget = useCallback(
    (target: NavigatorDropTarget | null) => {
      const currentDraggedItem = draggedItemRef.current
      if (!currentDraggedItem || !target || !canDropOnTarget(currentDraggedItem, target)) {
        setActiveDropTargetKey(null)
        if (currentDraggedItem) {
          setGlobalDragCursor('not-allowed')
        }
        return
      }

      setActiveDropTargetKey(dropTargetKey(target))
      setGlobalDragCursor('pointer')
    },
    [setGlobalDragCursor],
  )

  useEffect(() => {
    if (!draggedItem) {
      return
    }

    const handleWindowPointerRelease = () => {
      if (!draggedItemRef.current) {
        return
      }

      clearPointerDrag()
    }

    window.addEventListener('pointerup', handleWindowPointerRelease)
    window.addEventListener('pointercancel', handleWindowPointerRelease)
    window.addEventListener('blur', handleWindowPointerRelease)

    return () => {
      window.removeEventListener('pointerup', handleWindowPointerRelease)
      window.removeEventListener('pointercancel', handleWindowPointerRelease)
      window.removeEventListener('blur', handleWindowPointerRelease)
    }
  }, [clearPointerDrag, draggedItem])

  useEffect(
    () => () => {
      if (suppressClickTimeoutRef.current !== null) {
        window.clearTimeout(suppressClickTimeoutRef.current)
      }
      restoreGlobalDragFeedback()
    },
    [restoreGlobalDragFeedback],
  )

  useEffect(() => {
    if (!activeWorkspaceId || !selectedItemId || !selectedItemType || selectedItemType === 'workspace') {
      return
    }

    const activeTree = workspaceTrees[activeWorkspaceId]
    if (!activeTree) {
      return
    }

    const ancestorIds =
      selectedItemType === 'group'
        ? findGroupAncestorIds(activeTree.groups, selectedItemId)
        : findProjectAncestorIds(activeTree.groups, selectedItemId)

    if (!ancestorIds) {
      return
    }

    const frameId = requestAnimationFrame(() => {
      setCollapsedWorkspaceIds((current) =>
        current.includes(activeWorkspaceId)
          ? current.filter((id) => id !== activeWorkspaceId)
          : current,
      )
      setExpandedWorkspaceIds((current) =>
        current.includes(activeWorkspaceId) ? current : [...current, activeWorkspaceId],
      )

      if (ancestorIds.length === 0) {
        return
      }

      setExpandedGroupIds((current) => {
        const nextAncestorIds = ancestorIds.filter((ancestorId) => !current.includes(ancestorId))
        return nextAncestorIds.length > 0 ? [...current, ...nextAncestorIds] : current
      })
    })

    return () => {
      cancelAnimationFrame(frameId)
    }
  }, [activeWorkspaceId, selectedItemId, selectedItemType, workspaceTrees])

  function beginPointerDrag(event: PointerEvent<HTMLElement>, item: NavigatorDragItem) {
    if (event.button !== 0) {
      return
    }

    event.preventDefault()
    event.stopPropagation()
    if (suppressClickTimeoutRef.current !== null) {
      window.clearTimeout(suppressClickTimeoutRef.current)
      suppressClickTimeoutRef.current = null
    }
    suppressClickRef.current = false
    draggedItemRef.current = item
    setDraggedItem(item)
    setActiveDropTargetKey(null)
    setGlobalDragCursor('not-allowed')
  }

  function consumeSuppressedClick(event: ClickLikeEvent) {
    if (!suppressClickRef.current) {
      return false
    }

    suppressClickRef.current = false
    if (suppressClickTimeoutRef.current !== null) {
      window.clearTimeout(suppressClickTimeoutRef.current)
      suppressClickTimeoutRef.current = null
    }
    event.preventDefault()
    event.stopPropagation()
    return true
  }

  function handlePointerTargetMove(
    event: PointerEvent<HTMLElement>,
    target: NavigatorDropTarget | null,
  ) {
    if (!draggedItemRef.current) {
      return
    }

    event.preventDefault()
    updatePointerDropTarget(target)
  }

  function handlePointerTargetLeave(target: NavigatorDropTarget) {
    if (!draggedItemRef.current) {
      return
    }

    if (activeDropTargetKey === dropTargetKey(target)) {
      setActiveDropTargetKey(null)
      setGlobalDragCursor('not-allowed')
    }
  }

  function handlePointerTargetUp(
    event: PointerEvent<HTMLElement>,
    target: NavigatorDropTarget | null,
  ) {
    const currentDraggedItem = draggedItemRef.current
    if (!currentDraggedItem) {
      return
    }

    event.preventDefault()
    event.stopPropagation()

    if (!target || !canDropOnTarget(currentDraggedItem, target)) {
      clearPointerDrag()
      return
    }

    clearPointerDrag({ suppressClick: true })
    void onRequestMove(currentDraggedItem, target)
  }

  function toggleGroup(groupId: string) {
    setExpandedGroupIds((current) =>
      current.includes(groupId)
        ? current.filter((id) => id !== groupId)
        : [...current, groupId],
    )
  }

  async function toggleWorkspace(workspaceId: string) {
    const isExpanded =
      expandedWorkspaceIds.includes(workspaceId) ||
      (workspaceId === activeWorkspaceId && !collapsedWorkspaceIds.includes(workspaceId))

    if (isExpanded) {
      setExpandedWorkspaceIds((current) => current.filter((id) => id !== workspaceId))
      setCollapsedWorkspaceIds((current) =>
        current.includes(workspaceId) ? current : [...current, workspaceId],
      )
      return
    }

    await onEnsureWorkspaceTree(workspaceId)
    setCollapsedWorkspaceIds((current) => current.filter((id) => id !== workspaceId))
    setExpandedWorkspaceIds((current) =>
      current.includes(workspaceId) ? current : [...current, workspaceId],
    )
  }

  async function handleSelectWorkspace(workspaceId: string) {
    const isExpanded =
      expandedWorkspaceIds.includes(workspaceId) ||
      (workspaceId === activeWorkspaceId && !collapsedWorkspaceIds.includes(workspaceId))

    if (!isExpanded) {
      await onEnsureWorkspaceTree(workspaceId)
      setCollapsedWorkspaceIds((current) => current.filter((id) => id !== workspaceId))
      setExpandedWorkspaceIds((current) =>
        current.includes(workspaceId) ? current : [...current, workspaceId],
      )
    }

    await onSelectWorkspace(workspaceId)
  }

  return (
    <div className={draggedItem ? 'navigator-tree navigator-tree-dragging' : 'navigator-tree'}>
      <ul className="navigator-list">
        {workspaces.map((workspace) => {
          const isActiveWorkspace = workspace.id === activeWorkspaceId
          const workspaceDropTarget: NavigatorDropTarget = {
            id: workspace.id,
            name: workspace.name,
            type: 'workspace',
          }
          const isExpanded =
            expandedWorkspaceIds.includes(workspace.id) ||
            (workspace.id === activeWorkspaceId && !collapsedWorkspaceIds.includes(workspace.id))
          const isSelectedWorkspace =
            selectedItem?.type === 'workspace' && selectedItem.id === workspace.id
          const isDropAvailable =
            draggedItem !== null && canDropOnTarget(draggedItem, workspaceDropTarget)
          const isActiveDropTarget = activeDropTargetKey === dropTargetKey(workspaceDropTarget)
          const canDropOnRootZone = canDropGroupOnWorkspaceRoot(draggedItem, workspace.id)
          const workspaceTree = workspaceTrees[workspace.id] ?? null
          const applyActiveFilters = isActiveWorkspace && runtimeFilter !== 'ALL'
          const workspaceGroups = workspaceTree
            ? filterGroups(
                workspaceTree.groups,
                statusByProjectId,
                runtimeFilter,
                applyActiveFilters,
              )
            : []
          const hasVisibleWorkspaceContent = workspaceGroups.length > 0 || canDropOnRootZone
          const canCollapseWorkspace = workspaceTree ? hasVisibleWorkspaceContent : true
          const shouldRenderWorkspaceBody =
            isExpanded || (workspaceTree !== null && !hasVisibleWorkspaceContent)

          const workspaceRowClass = isSelectedWorkspace
            ? `navigator-row navigator-row-workspace selected${
                isDropAvailable ? ' navigator-drop-target-available' : ''
              }${isActiveDropTarget ? ' navigator-drop-target-active' : ''}`
            : `navigator-row navigator-row-workspace${
                isDropAvailable ? ' navigator-drop-target-available' : ''
              }${isActiveDropTarget ? ' navigator-drop-target-active' : ''}`

          return (
            <li className="navigator-item" key={workspace.id}>
              <div
                className={workspaceRowClass}
                onPointerLeave={() => handlePointerTargetLeave(workspaceDropTarget)}
                onPointerMove={(event) => handlePointerTargetMove(event, workspaceDropTarget)}
                onPointerUp={(event) => handlePointerTargetUp(event, workspaceDropTarget)}
              >
                {canCollapseWorkspace ? (
                  <button
                    aria-label={
                      isExpanded ? `Contraer ${workspace.name}` : `Expandir ${workspace.name}`
                    }
                    className="navigator-toggle"
                    onClick={(event) => {
                      if (consumeSuppressedClick(event)) {
                        return
                      }

                      void toggleWorkspace(workspace.id)
                    }}
                    type="button"
                  >
                    {isExpanded ? '-' : '+'}
                  </button>
                ) : (
                  <div className="navigator-toggle-placeholder" aria-hidden="true" />
                )}
                <button
                  aria-label={`Abrir workspace ${workspace.name}`}
                  className="navigator-node"
                  onClick={(event) => {
                    if (consumeSuppressedClick(event)) {
                      return
                    }

                    void handleSelectWorkspace(workspace.id)
                  }}
                  title={workspace.name}
                  type="button"
                >
                  <div className="tree-node-copy">
                    <span className="workspace-name-line">
                      <strong>{workspace.name}</strong>
                      {isActiveWorkspace && isLoadingPersistedState ? (
                        <span
                          aria-label="Cargando estado persistido"
                          className="workspace-loading-spinner"
                          role="status"
                          title="Cargando estado persistido"
                        />
                      ) : null}
                    </span>
                    <p>Workspace</p>
                  </div>
                  {isActiveWorkspace ? (
                    <div className="project-badges">
                      <RuntimeStatusBadge status={workspaceStatus} variant="compact" />
                    </div>
                  ) : null}
                </button>
              </div>

              {shouldRenderWorkspaceBody ? (
                workspaceTree ? (
                  workspaceGroups.length > 0 || canDropOnRootZone ? (
                    <div className="navigator-children navigator-workspace-body">
                      {canDropOnRootZone ? (
                        <div
                          aria-label={`Soltar en la raíz de ${workspace.name}`}
                          role="group"
                          className={
                            activeDropTargetKey === dropTargetKey(workspaceDropTarget)
                              ? 'navigator-root-dropzone navigator-drop-target-active'
                              : 'navigator-root-dropzone navigator-drop-target-available'
                          }
                          onPointerLeave={() => handlePointerTargetLeave(workspaceDropTarget)}
                          onPointerMove={(event) =>
                            handlePointerTargetMove(event, workspaceDropTarget)
                          }
                          onPointerUp={(event) => handlePointerTargetUp(event, workspaceDropTarget)}
                        />
                      ) : null}

                      {workspaceGroups.length > 0 ? (
                        <ul className="navigator-list">
                          {workspaceGroups.map((group) => (
                            <TreeGroup
                              activeDropTargetKey={activeDropTargetKey}
                              ancestorGroupIds={[]}
                              draggedItem={draggedItem}
                              draggedItemKey={draggedItemKey}
                              expandedGroupIds={expandedGroupIds}
                              group={group}
                              key={group.id}
                              onBeginPointerDrag={beginPointerDrag}
                              onConsumeSuppressedClick={consumeSuppressedClick}
                              onPointerTargetLeave={handlePointerTargetLeave}
                              onPointerTargetMove={handlePointerTargetMove}
                              onPointerTargetUp={handlePointerTargetUp}
                              onSelectGroup={onSelectGroup}
                              onSelectProject={onSelectProject}
                              selectedItem={selectedItem}
                              statusByProjectId={statusByProjectId}
                              toggleGroup={toggleGroup}
                              workspaceId={workspace.id}
                              workspaceIsActive={isActiveWorkspace}
                            />
                          ))}
                        </ul>
                      ) : null}
                    </div>
                  ) : (
                    <div className="navigator-children navigator-workspace-body navigator-workspace-empty-body">
                      <ul className="navigator-list">
                        <WorkspaceTreeMessage
                          message={
                            workspaceTree.groups.length === 0
                              ? 'Todavía no hay grupos ni proyectos en este workspace.'
                              : 'No hay proyectos que cumplan los filtros activos.'
                          }
                        />
                      </ul>
                    </div>
                  )
                ) : (
                    <div className="navigator-children navigator-workspace-body navigator-workspace-empty-body">
                      <ul className="navigator-list">
                        <WorkspaceTreeMessage message="Carga este workspace para ver sus grupos y proyectos." />
                      </ul>
                  </div>
                )
              ) : null}
            </li>
          )
        })}
      </ul>
    </div>
  )
}

type TreeGroupProps = Omit<
  WorkspaceRuntimeTreeViewProps,
  | 'activeWorkspaceId'
  | 'isLoadingPersistedState'
  | 'onEnsureWorkspaceTree'
  | 'onRequestMove'
  | 'onSelectWorkspace'
  | 'runtimeFilter'
  | 'workspaceStatus'
  | 'workspaceTrees'
  | 'workspaces'
> & {
  activeDropTargetKey: string | null
  ancestorGroupIds: string[]
  draggedItem: NavigatorDragItem | null
  draggedItemKey: string | null
  expandedGroupIds: string[]
  group: GroupTreeNode
  onBeginPointerDrag: (event: PointerEvent<HTMLElement>, item: NavigatorDragItem) => void
  onConsumeSuppressedClick: (event: ClickLikeEvent) => boolean
  onPointerTargetLeave: (target: NavigatorDropTarget) => void
  onPointerTargetMove: (event: PointerEvent<HTMLElement>, target: NavigatorDropTarget | null) => void
  onPointerTargetUp: (event: PointerEvent<HTMLElement>, target: NavigatorDropTarget | null) => void
  toggleGroup: (groupId: string) => void
  workspaceId: string
  workspaceIsActive: boolean
}

function TreeGroup({
  activeDropTargetKey,
  ancestorGroupIds,
  draggedItem,
  draggedItemKey,
  expandedGroupIds,
  group,
  onBeginPointerDrag,
  onConsumeSuppressedClick,
  onPointerTargetLeave,
  onPointerTargetMove,
  onPointerTargetUp,
  onSelectGroup,
  onSelectProject,
  selectedItem,
  statusByProjectId,
  toggleGroup,
  workspaceId,
  workspaceIsActive,
}: TreeGroupProps) {
  const runtimeStatus = groupRuntimeStatus(group, statusByProjectId)
  const groupDropTarget: NavigatorDropTarget = {
    id: group.id,
    name: group.name,
    type: 'group',
    workspaceId,
  }
  const isSelectedGroup = selectedItem?.type === 'group' && selectedItem.id === group.id
  const hasChildren = group.groups.length > 0 || group.projects.length > 0
  const isExpanded = expandedGroupIds.includes(group.id)
  const isDraggingCurrentGroup = draggedItemKey === `group:${group.id}`
  const isDropDisabled = isGroupTargetDisabled(draggedItem, group.id, ancestorGroupIds)
  const isDropAvailable =
    !isDropDisabled && draggedItem !== null && canDropOnTarget(draggedItem, groupDropTarget)
  const isActiveDropTarget = activeDropTargetKey === dropTargetKey(groupDropTarget)
  const groupRowClass = isSelectedGroup
    ? `navigator-row${isDraggingCurrentGroup ? ' navigator-drag-source' : ''}${
        isDropAvailable ? ' navigator-drop-target-available' : ''
      }${isActiveDropTarget ? ' navigator-drop-target-active' : ''} selected`
    : `navigator-row${isDraggingCurrentGroup ? ' navigator-drag-source' : ''}${
        isDropAvailable ? ' navigator-drop-target-available' : ''
      }${isActiveDropTarget ? ' navigator-drop-target-active' : ''}`

  return (
    <li className="navigator-item">
      <div
        className={groupRowClass}
        onPointerLeave={() => onPointerTargetLeave(groupDropTarget)}
        onPointerMove={(event) =>
          onPointerTargetMove(event, isDropDisabled ? null : groupDropTarget)
        }
        onPointerUp={(event) => onPointerTargetUp(event, isDropDisabled ? null : groupDropTarget)}
      >
        {hasChildren ? (
          <button
            aria-label={isExpanded ? `Contraer ${group.name}` : `Expandir ${group.name}`}
            className="navigator-toggle"
            onClick={(event) => {
              if (onConsumeSuppressedClick(event)) {
                return
              }

              toggleGroup(group.id)
            }}
            type="button"
          >
            {isExpanded ? '-' : '+'}
          </button>
        ) : null}
        {!hasChildren ? (
          <div className="navigator-toggle-placeholder" aria-hidden="true" />
        ) : null}
        <button
          aria-label={`Abrir grupo ${group.name}`}
          className="navigator-node"
          onClick={(event) => {
            if (onConsumeSuppressedClick(event)) {
              return
            }

            void onSelectGroup(group.id, workspaceId)
          }}
          title={group.name}
          type="button"
        >
          <div className="tree-label">
            <div className="tree-node-copy">
              <strong>{group.name}</strong>
              <p>Grupo</p>
            </div>
          </div>
          {workspaceIsActive ? (
            <div className="project-badges">
              <RuntimeStatusBadge status={runtimeStatus} variant="compact" />
            </div>
          ) : null}
        </button>
        <button
          aria-label={`Arrastrar grupo ${group.name}`}
          className="navigator-drag-handle"
          onClick={(event) => {
            event.preventDefault()
          }}
          onPointerDown={(event) =>
            onBeginPointerDrag(event, {
              id: group.id,
              name: group.name,
              parentGroupId: group.parentGroupId,
              type: 'group',
              workspaceId,
            })
          }
          type="button"
        >
          ::
        </button>
      </div>

      {isExpanded ? (
        <ul className="navigator-list navigator-children">
          {group.groups.map((childGroup) => (
            <TreeGroup
              activeDropTargetKey={activeDropTargetKey}
              ancestorGroupIds={[...ancestorGroupIds, group.id]}
              draggedItem={draggedItem}
              draggedItemKey={draggedItemKey}
              expandedGroupIds={expandedGroupIds}
              group={childGroup}
              key={childGroup.id}
              onBeginPointerDrag={onBeginPointerDrag}
              onConsumeSuppressedClick={onConsumeSuppressedClick}
              onPointerTargetLeave={onPointerTargetLeave}
              onPointerTargetMove={onPointerTargetMove}
              onPointerTargetUp={onPointerTargetUp}
              onSelectGroup={onSelectGroup}
              onSelectProject={onSelectProject}
              selectedItem={selectedItem}
              statusByProjectId={statusByProjectId}
              toggleGroup={toggleGroup}
              workspaceId={workspaceId}
              workspaceIsActive={workspaceIsActive}
            />
          ))}

          {group.projects.map((project) => {
            const projectStatus: RuntimeStatus = statusByProjectId[project.id]?.status ?? 'STOPPED'
            const isSelectedProject =
              selectedItem?.type === 'project' && selectedItem.id === project.id
            const isDraggingCurrentProject = draggedItemKey === `project:${project.id}`
            const projectRowClass = isSelectedProject
              ? `navigator-row navigator-row-project selected${
                  isDraggingCurrentProject ? ' navigator-drag-source' : ''
                }`
              : `navigator-row navigator-row-project${
                  isDraggingCurrentProject ? ' navigator-drag-source' : ''
                }`

            return (
              <li className="navigator-item" key={project.id}>
                <div className={projectRowClass}>
                  <button
                    aria-label={`Abrir proyecto ${project.name}`}
                    className="navigator-node navigator-node-project"
                    onClick={(event) => {
                      if (onConsumeSuppressedClick(event)) {
                        return
                      }

                      void onSelectProject(project.id, workspaceId)
                    }}
                    title={`${project.name}\n${normalizeWindowsPath(project.path)}`}
                    type="button"
                  >
                    <div className="tree-node-copy">
                      <strong>{project.name}</strong>
                      <p>{normalizeWindowsPath(project.path)}</p>
                    </div>
                    {workspaceIsActive ? (
                      <div className="project-badges">
                        <RuntimeStatusBadge status={projectStatus} variant="compact" />
                      </div>
                    ) : null}
                  </button>
                  <button
                    aria-label={`Arrastrar proyecto ${project.name}`}
                    className="navigator-drag-handle"
                    onClick={(event) => {
                      event.preventDefault()
                    }}
                    onPointerDown={(event) =>
                      onBeginPointerDrag(event, {
                        groupId: group.id,
                        id: project.id,
                        name: project.name,
                        type: 'project',
                        workspaceId,
                      })
                    }
                    type="button"
                  >
                    ::
                  </button>
                </div>
              </li>
            )
          })}
        </ul>
      ) : null}
    </li>
  )
}

function filterGroups(
  groups: GroupTreeNode[],
  statusByProjectId: Record<string, ProcessRuntimeState>,
  runtimeFilter: RuntimeStatus | 'ALL',
  applyFilters: boolean,
): GroupTreeNode[] {
  return groups
    .map((group) => {
      const projects = applyFilters
        ? group.projects.filter((project) => {
            const runtimeStatus = statusByProjectId[project.id]?.status ?? 'STOPPED'

            return runtimeFilter === 'ALL' || runtimeStatus === runtimeFilter
          })
        : group.projects

      const children = filterGroups(
        group.groups,
        statusByProjectId,
        runtimeFilter,
        applyFilters,
      )

      return { ...group, groups: children, projects }
    })
    .filter((group) =>
      applyFilters ? group.projects.length > 0 || group.groups.length > 0 : true,
    )
}
