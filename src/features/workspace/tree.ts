import type {
  GroupTreeNode,
  HealthStatus,
  ProcessRuntimeState,
  ProjectHealthState,
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

export function projectHealthStatus(project: ProjectNode, healthState?: ProjectHealthState | null): HealthStatus {
  if (healthState) {
    return healthState.status
  }

  return project.healthCheck?.enabled ? 'UNKNOWN' : 'UNSUPPORTED'
}

export function projectSupportsHealth(project: ProjectNode) {
  return Boolean(project.healthCheck?.enabled)
}

export function groupSupportsHealth(group: GroupTreeNode): boolean {
  return (
    group.projects.some(projectSupportsHealth) ||
    group.groups.some(groupSupportsHealth)
  )
}

export function groupsSupportHealth(groups: GroupTreeNode[]) {
  return groups.some(groupSupportsHealth)
}

export function aggregateHealthStatus(statuses: HealthStatus[]): HealthStatus {
  if (statuses.length === 0) {
    return 'UNSUPPORTED'
  }
  if (statuses.includes('UNHEALTHY')) {
    return 'UNHEALTHY'
  }
  if (statuses.includes('CHECKING')) {
    return 'CHECKING'
  }
  if (statuses.includes('HEALTHY')) {
    return 'HEALTHY'
  }
  if (statuses.includes('UNKNOWN')) {
    return 'UNKNOWN'
  }

  return 'UNSUPPORTED'
}

export function countHealthStatuses(statuses: HealthStatus[]) {
  return {
    checking: statuses.filter((status) => status === 'CHECKING').length,
    healthy: statuses.filter((status) => status === 'HEALTHY').length,
    unhealthy: statuses.filter((status) => status === 'UNHEALTHY').length,
    unknown: statuses.filter((status) => status === 'UNKNOWN').length,
    unsupported: statuses.filter((status) => status === 'UNSUPPORTED').length,
  }
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

export function groupHealthStatus(
  group: GroupTreeNode,
  healthByProjectId: Record<string, ProjectHealthState>,
): HealthStatus {
  const descendantProjects = [...group.projects, ...flattenProjects(group.groups)]
  const statuses = descendantProjects.map((project) =>
    projectHealthStatus(project, healthByProjectId[project.id]),
  )

  return aggregateHealthStatus(statuses)
}
