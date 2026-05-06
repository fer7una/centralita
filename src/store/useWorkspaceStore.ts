import { startTransition, useCallback, useEffect, useRef, useState } from 'react'
import {
  analyzeProjectFolder,
  createGroup,
  createProjectFromDetection,
  createProject,
  createWorkspace,
  deleteGroup,
  deleteProject,
  deleteWorkspace,
  getWorkspaceTree,
  listWorkspaces,
  renameWorkspace,
  updateGroup,
  updateProject,
} from '../features/workspace/api'
import { findGroup, findProject, flattenGroups, flattenProjects, groupNameExists } from '../features/workspace/tree'
import type {
  CreateProjectFromDetectionInput,
  DetectionResult,
  CreateGroupInput,
  CreateProjectInput,
  GroupTreeNode,
  ProjectNode,
  Workspace,
  WorkspaceTree,
} from '../types'

type WorkspaceState = {
  activeWorkspaceId: string | null
  analysisResult: DetectionResult | null
  error: string | null
  groups: GroupTreeNode[]
  isAnalyzing: boolean
  isLoading: boolean
  tree: WorkspaceTree | null
  treesByWorkspaceId: Record<string, WorkspaceTree>
  workspaces: Workspace[]
}

const initialState: WorkspaceState = {
  activeWorkspaceId: null,
  analysisResult: null,
  error: null,
  groups: [],
  isAnalyzing: false,
  isLoading: true,
  tree: null,
  treesByWorkspaceId: {},
  workspaces: [],
}

export function useWorkspaceStore() {
  const [state, setState] = useState<WorkspaceState>(initialState)
  const treesByWorkspaceIdRef = useRef(state.treesByWorkspaceId)

  useEffect(() => {
    treesByWorkspaceIdRef.current = state.treesByWorkspaceId
  }, [state.treesByWorkspaceId])

  const loadWorkspaces = useCallback(async (preferredWorkspaceId?: string | null) => {
    setState((current) => ({ ...current, error: null, isLoading: true }))

    try {
      const workspaces = await listWorkspaces()
      const nextActiveWorkspaceId =
        preferredWorkspaceId && workspaces.some((workspace) => workspace.id === preferredWorkspaceId)
          ? preferredWorkspaceId
          : workspaces[0]?.id ?? null
      const loadedWorkspaceIds = Array.from(
        new Set(
          [nextActiveWorkspaceId, ...Object.keys(treesByWorkspaceIdRef.current)].filter(
            (workspaceId): workspaceId is string =>
              Boolean(workspaceId) &&
              workspaces.some((workspace) => workspace.id === workspaceId),
          ),
        ),
      )
      const nextTreeEntries = await Promise.all(
        loadedWorkspaceIds.map(async (workspaceId) => [
          workspaceId,
          await getWorkspaceTree(workspaceId),
        ]),
      )
      const nextTreesByWorkspaceId = Object.fromEntries(nextTreeEntries)
      const nextTree = nextActiveWorkspaceId ? nextTreesByWorkspaceId[nextActiveWorkspaceId] ?? null : null

      startTransition(() => {
        setState({
          activeWorkspaceId: nextActiveWorkspaceId,
          analysisResult: null,
          error: null,
          groups: nextTree?.groups ?? [],
          isAnalyzing: false,
          isLoading: false,
          tree: nextTree,
          treesByWorkspaceId: nextTreesByWorkspaceId,
          workspaces,
        })
      })
    } catch (error) {
      setState((current) => ({
        ...current,
        error: error instanceof Error ? error.message : 'No se pudo cargar el workspace.',
        isLoading: false,
      }))
    }
  }, [])

  useEffect(() => {
    void loadWorkspaces()
  }, [loadWorkspaces])

  async function selectWorkspace(workspaceId: string) {
    setState((current) => ({ ...current, error: null, isLoading: true }))

    try {
      const tree = await getWorkspaceTree(workspaceId)

      startTransition(() => {
        setState((current) => ({
          ...current,
          activeWorkspaceId: workspaceId,
          analysisResult: null,
          error: null,
          groups: tree.groups,
          isLoading: false,
          tree,
          treesByWorkspaceId: {
            ...current.treesByWorkspaceId,
            [workspaceId]: tree,
          },
        }))
      })
    } catch (error) {
      setState((current) => ({
        ...current,
        error: error instanceof Error ? error.message : 'No se pudo abrir el workspace.',
        isLoading: false,
      }))
    }
  }

  async function ensureWorkspaceTree(workspaceId: string) {
    const cachedTree = state.treesByWorkspaceId[workspaceId]
    if (cachedTree) {
      return cachedTree
    }

    try {
      const tree = await getWorkspaceTree(workspaceId)
      startTransition(() => {
        setState((current) => ({
          ...current,
          error: null,
          treesByWorkspaceId: {
            ...current.treesByWorkspaceId,
            [workspaceId]: tree,
          },
        }))
      })

      return tree
    } catch (error) {
      setState((current) => ({
        ...current,
        error: error instanceof Error ? error.message : 'No se pudo cargar el árbol del workspace.',
      }))
      throw error
    }
  }

  function findGroupInKnownTrees(groupId: string) {
    for (const tree of Object.values(state.treesByWorkspaceId)) {
      const group = findGroup(tree.groups, groupId)
      if (group) {
        return group
      }
    }

    return null
  }

  function findProjectInKnownTrees(projectId: string) {
    for (const tree of Object.values(state.treesByWorkspaceId)) {
      const project = findProject(tree.groups, projectId)
      if (project) {
        return project
      }
    }

    return null
  }

  async function getGroupsForWorkspace(workspaceId: string) {
    if (workspaceId === state.activeWorkspaceId) {
      return state.groups
    }

    const cachedTree = state.treesByWorkspaceId[workspaceId]
    if (cachedTree) {
      return cachedTree.groups
    }

    const tree = await getWorkspaceTree(workspaceId)
    startTransition(() => {
      setState((current) => ({
        ...current,
        error: null,
        treesByWorkspaceId: {
          ...current.treesByWorkspaceId,
          [workspaceId]: tree,
        },
      }))
    })

    return tree.groups
  }

  async function handleCreateWorkspace(name: string) {
    await createWorkspace({ name })
    await loadWorkspaces()
  }

  async function handleRenameWorkspace(workspaceId: string, name: string) {
    await renameWorkspace({ id: workspaceId, name })
    await loadWorkspaces(workspaceId)
  }

  async function handleDeleteWorkspace(workspaceId: string) {
    await deleteWorkspace({ id: workspaceId })
    await loadWorkspaces(state.activeWorkspaceId === workspaceId ? null : state.activeWorkspaceId)
  }

  async function handleCreateGroup(input: CreateGroupInput) {
    if (
      groupNameExists(state.groups, input.name, {
        parentGroupId: input.parentGroupId,
      })
    ) {
      setState((current) => ({
        ...current,
        error: `Ya existe un grupo con el nombre "${input.name.trim()}".`,
      }))
      return
    }

    setState((current) => ({ ...current, error: null }))
    await createGroup(input)
    await loadWorkspaces(state.activeWorkspaceId)
  }

  async function handleRenameGroup(groupId: string, name: string) {
    await handleUpdateGroup(groupId, { name })
  }

  async function handleUpdateGroup(
    groupId: string,
    updates: {
      color?: string
      name?: string
      parentGroupId?: string | null
      workspaceId?: string
    },
  ) {
    const group = findGroupInKnownTrees(groupId)
    if (!group) {
      return
    }

    const nextWorkspaceId = updates.workspaceId ?? group.workspaceId
    const nextParentGroupId = Object.prototype.hasOwnProperty.call(updates, 'parentGroupId')
      ? (updates.parentGroupId ?? null)
      : group.parentGroupId
    const nextName = updates.name ?? group.name
    const targetGroups = await getGroupsForWorkspace(nextWorkspaceId)
    const duplicateNameExists = groupNameExists(
      targetGroups,
      nextName,
      {
        excludeGroupId: nextWorkspaceId === group.workspaceId ? groupId : undefined,
        parentGroupId: nextParentGroupId,
      },
    )

    if (duplicateNameExists) {
      setState((current) => ({
        ...current,
        error: `Ya existe un grupo con el nombre "${nextName.trim()}".`,
      }))
      return
    }

    setState((current) => ({ ...current, error: null }))
    await updateGroup({
      id: group.id,
      workspaceId: nextWorkspaceId,
      parentGroupId: nextParentGroupId,
      name: nextName,
      color: updates.color ?? group.color,
      sortOrder: group.sortOrder,
    })
    await loadWorkspaces(state.activeWorkspaceId)
  }

  async function handleMoveGroupTree(
    groupId: string,
    target: {
      parentGroupId: string | null
      workspaceId: string
    },
  ) {
    const group = findGroupInKnownTrees(groupId)
    if (!group) {
      return
    }

    const targetGroups = await getGroupsForWorkspace(target.workspaceId)
    const duplicateNameExists = groupNameExists(
      targetGroups,
      group.name,
      {
        excludeGroupId: target.workspaceId === group.workspaceId ? groupId : undefined,
        parentGroupId: target.parentGroupId,
      },
    )

    if (duplicateNameExists) {
      setState((current) => ({
        ...current,
        error: `Ya existe un grupo con el nombre "${group.name.trim()}".`,
      }))
      return
    }

    const descendantGroups = flattenGroups(group.groups)
    const descendantProjects = [...group.projects, ...flattenProjects(group.groups)]

    setState((current) => ({ ...current, error: null }))

    await updateGroup({
      id: group.id,
      workspaceId: target.workspaceId,
      parentGroupId: target.parentGroupId,
      name: group.name,
      color: group.color,
      sortOrder: group.sortOrder,
    })

    if (target.workspaceId !== group.workspaceId) {
      for (const descendantGroup of descendantGroups) {
        await updateGroup({
          id: descendantGroup.id,
          workspaceId: target.workspaceId,
          parentGroupId: descendantGroup.parentGroupId,
          name: descendantGroup.name,
          color: descendantGroup.color,
          sortOrder: descendantGroup.sortOrder,
        })
      }

      for (const project of descendantProjects) {
        await updateProject({
          id: project.id,
          workspaceId: target.workspaceId,
          groupId: project.groupId,
          name: project.name,
          path: project.path,
          detectedType: project.detectedType,
          color: project.color,
          packageManager: project.packageManager,
          executable: project.executable,
          command: project.command,
          args: project.args,
          env: project.env,
          workingDir: project.workingDir,
          detectionConfidence: project.detectionConfidence,
          detectionEvidence: project.detectionEvidence,
          warnings: project.warnings,
          healthCheck: project.healthCheck,
        })
      }
    }

    await loadWorkspaces(state.activeWorkspaceId)
  }

  async function handleDeleteGroup(groupId: string) {
    await deleteGroup({ id: groupId })
    await loadWorkspaces(state.activeWorkspaceId)
  }

  async function handleCreateProject(input: CreateProjectInput) {
    const project = await createProject(input)
    await loadWorkspaces(state.activeWorkspaceId)
    return project
  }

  async function handleAnalyzeProjectFolder(path: string) {
    setState((current) => ({
      ...current,
      error: null,
      isAnalyzing: true,
    }))

    try {
      const analysisResult = await analyzeProjectFolder({ path })

      startTransition(() => {
        setState((current) => ({
          ...current,
          analysisResult,
          error: null,
          isAnalyzing: false,
        }))
      })

      return analysisResult
    } catch (error) {
      setState((current) => ({
        ...current,
        analysisResult: null,
        error: error instanceof Error ? error.message : 'No se pudo analizar la carpeta.',
        isAnalyzing: false,
      }))
      throw error
    }
  }

  async function handleCreateProjectFromDetection(input: CreateProjectFromDetectionInput) {
    const project = await createProjectFromDetection(input)
    startTransition(() => {
      setState((current) => ({
        ...current,
        analysisResult: null,
      }))
    })
    await loadWorkspaces(state.activeWorkspaceId)
    return project
  }

  async function handleRenameProject(projectId: string, name: string) {
    await handleUpdateProject(projectId, { name })
  }

  async function handleUpdateProject(
    projectId: string,
    updates: Partial<
      Pick<
        ProjectNode,
        | 'args'
        | 'color'
        | 'command'
        | 'env'
        | 'executable'
        | 'groupId'
        | 'healthCheck'
        | 'name'
        | 'path'
        | 'workspaceId'
        | 'workingDir'
      >
    >,
  ) {
    const project = findProjectInKnownTrees(projectId)
    if (!project) {
      return
    }

    const nextWorkspaceId = updates.workspaceId ?? project.workspaceId

    await updateProject({
      id: project.id,
      workspaceId: nextWorkspaceId,
      groupId: updates.groupId ?? project.groupId,
      name: updates.name ?? project.name,
      path: updates.path ?? project.path,
      detectedType: project.detectedType,
      color: updates.color ?? project.color,
      packageManager: project.packageManager,
      executable: updates.executable ?? project.executable,
      command: updates.command ?? project.command,
      args: updates.args ?? project.args,
      env: updates.env ?? project.env,
      workingDir: updates.workingDir ?? project.workingDir,
      detectionConfidence: project.detectionConfidence,
      detectionEvidence: project.detectionEvidence,
      warnings: project.warnings,
      healthCheck: updates.healthCheck ?? project.healthCheck,
    })
    await loadWorkspaces(state.activeWorkspaceId)
  }

  async function handleDeleteProject(projectId: string) {
    await deleteProject({ id: projectId })
    await loadWorkspaces(state.activeWorkspaceId)
  }

  return {
    activeWorkspaceId: state.activeWorkspaceId,
    analysisResult: state.analysisResult,
    error: state.error,
    flatGroups: flattenGroups(state.groups),
    groups: state.groups,
    isAnalyzing: state.isAnalyzing,
    isLoading: state.isLoading,
    selectedWorkspace: state.tree?.workspace ?? null,
    tree: state.tree,
    treesByWorkspaceId: state.treesByWorkspaceId,
    workspaces: state.workspaces,
    actions: {
      analyzeProjectFolder: handleAnalyzeProjectFolder,
      clearAnalysis: () =>
        setState((current) => ({
          ...current,
          analysisResult: null,
        })),
      createGroup: handleCreateGroup,
      createProjectFromDetection: handleCreateProjectFromDetection,
      createProject: handleCreateProject,
      createWorkspace: handleCreateWorkspace,
      deleteGroup: handleDeleteGroup,
      deleteProject: handleDeleteProject,
      deleteWorkspace: handleDeleteWorkspace,
      refresh: () => loadWorkspaces(state.activeWorkspaceId),
      moveGroupTree: handleMoveGroupTree,
      renameGroup: handleRenameGroup,
      renameProject: handleRenameProject,
      renameWorkspace: handleRenameWorkspace,
      ensureWorkspaceTree,
      selectWorkspace,
      updateGroup: handleUpdateGroup,
      updateProject: handleUpdateProject,
    },
  }
}
