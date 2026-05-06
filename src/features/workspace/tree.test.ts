import { groupNameExists } from './tree'
import type { GroupTreeNode } from '../../types'

describe('groupNameExists', () => {
  const groups: GroupTreeNode[] = [
    {
      id: 'group-frontend',
      workspaceId: 'workspace-main',
      parentGroupId: null,
      name: 'Frontend',
      color: '#2563eb',
      sortOrder: 10,
      createdAt: '2026-04-17T09:00:00Z',
      updatedAt: '2026-04-17T09:00:00Z',
      projects: [],
      groups: [
        {
          id: 'group-ui',
          workspaceId: 'workspace-main',
          parentGroupId: 'group-frontend',
          name: 'UI',
          color: '#0f172a',
          sortOrder: 10,
          createdAt: '2026-04-17T10:00:00Z',
          updatedAt: '2026-04-17T10:00:00Z',
          projects: [],
          groups: [],
        },
      ],
    },
    {
      id: 'group-backend',
      workspaceId: 'workspace-main',
      parentGroupId: null,
      name: 'Backend',
      color: '#2f855a',
      sortOrder: 20,
      createdAt: '2026-04-17T11:00:00Z',
      updatedAt: '2026-04-17T11:00:00Z',
      projects: [],
      groups: [],
    },
  ]

  it('matches duplicate group names only within the same parent', () => {
    expect(
      groupNameExists(groups, ' frontend ', { parentGroupId: null }),
    ).toBe(true)
    expect(
      groupNameExists(groups, ' frontend ', {
        parentGroupId: 'group-backend',
      }),
    ).toBe(false)
    expect(
      groupNameExists(groups, ' ui ', { parentGroupId: 'group-frontend' }),
    ).toBe(true)
  })

  it('ignores the edited group while checking its siblings', () => {
    expect(
      groupNameExists(groups, ' frontend ', {
        excludeGroupId: 'group-frontend',
        parentGroupId: null,
      }),
    ).toBe(false)
  })
})
