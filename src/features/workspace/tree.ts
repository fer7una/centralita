import type {
  GroupTreeNode,
  ProcessRuntimeState,
  ProjectNode,
  RuntimeStatus,
} from '../../types'

export function flattenGroups(groups: GroupTreeNode[]): GroupTreeNode[] {
  return groups.flatMap((group) => [group, ...flattenGroups(group.groups)])
}

export function flattenProjects(groups: GroupTreeNode[]): ProjectNode[] {
  return groups.flatMap((group) => [...group.projects, ...flattenProjects(group.groups)])
}

function normalizeEntityName(name: string) {
  return name.trim().toLocaleLowerCase()
}

export function groupNameExists(
  groups: GroupTreeNode[],
  name: string,
  options?: { excludeGroupId?: string; parentGroupId?: string | null },
) {
  const normalizedName = normalizeEntityName(name)
  const shouldMatchParent = Object.prototype.hasOwnProperty.call(
    options ?? {},
    'parentGroupId',
  )

  return flattenGroups(groups).some((group) => {
    if (options?.excludeGroupId && group.id === options.excludeGroupId) {
      return false
    }
    if (shouldMatchParent && group.parentGroupId !== options?.parentGroupId) {
      return false
    }

    return normalizeEntityName(group.name) === normalizedName
  })
}

export function findGroup(groups: GroupTreeNode[], groupId: string): GroupTreeNode | null {
  for (const group of groups) {
    if (group.id === groupId) {
      return group
    }

    const nested = findGroup(group.groups, groupId)
    if (nested) {
      return nested
    }
  }

  return null
}

export function findProject(groups: GroupTreeNode[], projectId: string): ProjectNode | null {
  for (const group of groups) {
    const project = group.projects.find((item) => item.id === projectId)
    if (project) {
      return project
    }

    const nested = findProject(group.groups, projectId)
    if (nested) {
      return nested
    }
  }

  return null
}

export function aggregateRuntimeStatus(statuses: RuntimeStatus[]): RuntimeStatus {
  if (statuses.length === 0) {
    return 'STOPPED'
  }
  if (statuses.every((status) => status === 'STOPPED')) {
    return 'STOPPED'
  }
  if (statuses.includes('STOPPING')) {
    return 'STOPPING'
  }
  if (statuses.includes('STARTING')) {
    return 'STARTING'
  }
  if (statuses.includes('FAILED')) {
    return 'FAILED'
  }
  if (statuses.includes('RUNNING')) {
    return 'RUNNING'
  }

  return 'STOPPED'
}

export function groupRuntimeStatus(
  group: GroupTreeNode,
  statusByProjectId: Record<string, ProcessRuntimeState>,
): RuntimeStatus {
  const descendantProjects = [...group.projects, ...flattenProjects(group.groups)]
  const statuses = descendantProjects.map(
    (project) => statusByProjectId[project.id]?.status ?? 'STOPPED',
  )

  return aggregateRuntimeStatus(statuses)
}
