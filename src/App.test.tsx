import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react'
import CentralitaApp from './CentralitaApp'
import { WorkspaceRuntimeTreeView } from './components/runtime/WorkspaceRuntimeTreeView'
import * as runtimeApi from './features/runtime/api'
import * as workspaceApi from './features/workspace/api'
import { listenRuntimeEvent, RUNTIME_EVENTS } from './shared/api/tauri'
import type { ProjectNode, Workspace, WorkspaceTree } from './types'

vi.mock('./features/workspace/api', () => ({
  analyzeProjectFolder: vi.fn(),
  createGroup: vi.fn(),
  createProjectFromDetection: vi.fn(),
  createProject: vi.fn(),
  createWorkspace: vi.fn(),
  deleteGroup: vi.fn(),
  deleteProject: vi.fn(),
  deleteWorkspace: vi.fn(),
  getProjectGitInfo: vi.fn(),
  getWorkspaceTree: vi.fn(),
  listWorkspaces: vi.fn(),
  renameWorkspace: vi.fn(),
  updateGroup: vi.fn(),
  updateProject: vi.fn(),
  validateProjectCommand: vi.fn(),
}))

vi.mock('./features/runtime/api', () => ({
  getProjectLogs: vi.fn(),
  getProjectRuntimeStatus: vi.fn(),
  getWorkspaceObservabilitySummary: vi.fn(),
  getWorkspaceRuntimeStatus: vi.fn(),
  listProjectRunHistory: vi.fn(),
  listWorkspaceRunHistory: vi.fn(),
  restartProject: vi.fn(),
  startGroup: vi.fn(),
  startProject: vi.fn(),
  startWorkspace: vi.fn(),
  stopGroup: vi.fn(),
  stopProject: vi.fn(),
  stopWorkspace: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

vi.mock('./shared/api/tauri', () => ({
  RUNTIME_EVENTS: {
    historyAppended: 'runtime://history-appended',
    logLine: 'runtime://log-line',
    processError: 'runtime://process-error',
    processExited: 'runtime://process-exited',
    statusChanged: 'runtime://status-changed',
  },
  listenRuntimeEvent: vi.fn().mockResolvedValue(() => {}),
}))

describe('CentralitaApp', () => {
  const scrollIntoViewMock = vi.fn()
  const scrollToMock = vi.fn()

  function requireTreeRow(name: string) {
    const row = screen.getByRole('button', { name }).closest('.navigator-row')
    expect(row).not.toBeNull()
    return row as HTMLElement
  }

  function requireDragHandle(name: string) {
    return screen.getByRole('button', { name })
  }

  const workspace: Workspace = {
    id: 'workspace-main',
    name: 'Centralita',
    createdAt: '2026-04-14T09:00:00Z',
    updatedAt: '2026-04-14T09:00:00Z',
  }

  const workspaceOps: Workspace = {
    id: 'workspace-ops',
    name: 'Operaciones',
    createdAt: '2026-04-15T09:00:00Z',
    updatedAt: '2026-04-15T09:00:00Z',
  }

  const project: ProjectNode = {
    id: 'project-ui',
    workspaceId: 'workspace-main',
    groupId: 'group-frontend',
    name: 'centralita-ui',
    path: '\\\\?\\C:\\Proyectos\\centralita-ui',
    detectedType: 'reactVite' as const,
    color: null,
    packageManager: 'pnpm' as const,
    executable: 'pnpm',
    command: 'pnpm dev',
    args: ['dev'],
    env: undefined,
    workingDir: '\\\\?\\C:\\Proyectos\\centralita-ui',
    detectionConfidence: 0.9,
    detectionEvidence: [],
    warnings: [],
    createdAt: '2026-04-16T07:00:00Z',
    updatedAt: '2026-04-16T07:00:00Z',
  }

  const workspaceTree: WorkspaceTree = {
    workspace,
    groups: [
      {
        id: 'group-frontend',
        workspaceId: 'workspace-main',
        parentGroupId: null,
        name: 'Frontend',
        color: '#56ba81',
        sortOrder: 10,
        createdAt: '2026-04-14T09:00:00Z',
        updatedAt: '2026-04-14T09:00:00Z',
        groups: [],
        projects: [project],
      },
    ],
  }

  const workspaceOpsTree: WorkspaceTree = {
    workspace: workspaceOps,
    groups: [
      {
        id: 'group-backend',
        workspaceId: 'workspace-ops',
        parentGroupId: null,
        name: 'Backend',
        color: '#2563eb',
        sortOrder: 20,
        createdAt: '2026-04-15T09:00:00Z',
        updatedAt: '2026-04-15T09:00:00Z',
        groups: [],
        projects: [],
      },
    ],
  }

  const workspaceRuntime = {
    workspaceId: 'workspace-main',
    status: 'STOPPED' as const,
    projects: [
      {
        projectId: 'project-ui',
        status: 'STOPPED' as const,
        pid: null,
        startedAt: null,
        stoppedAt: null,
        exitCode: null,
        lastError: null,
        commandPreview: 'pnpm dev',
      },
    ],
  }

  beforeAll(() => {
    Element.prototype.scrollIntoView = scrollIntoViewMock
    Element.prototype.scrollTo = scrollToMock
  })

  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      value: 1024,
    })
    scrollIntoViewMock.mockClear()
    scrollToMock.mockClear()
    vi.mocked(listenRuntimeEvent).mockResolvedValue(() => {})
    vi.mocked(workspaceApi.listWorkspaces).mockResolvedValue([
      workspace,
      workspaceOps,
    ])
    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : workspaceTree,
    )
    vi.mocked(runtimeApi.getWorkspaceRuntimeStatus).mockResolvedValue(
      workspaceRuntime,
    )
    vi.mocked(runtimeApi.getProjectLogs).mockResolvedValue([])
    vi.mocked(runtimeApi.listProjectRunHistory).mockResolvedValue([])
    vi.mocked(runtimeApi.startProject).mockResolvedValue(
      workspaceRuntime.projects[0],
    )
    vi.mocked(runtimeApi.stopProject).mockResolvedValue(
      workspaceRuntime.projects[0],
    )
    vi.mocked(runtimeApi.restartProject).mockResolvedValue(
      workspaceRuntime.projects[0],
    )
    vi.mocked(runtimeApi.startGroup).mockResolvedValue({
      scope: 'group',
      targetId: 'group-frontend',
      status: 'STOPPED',
      requestedProjectIds: [],
      affectedProjectIds: [],
      skippedProjectIds: [],
      failures: [],
    })
    vi.mocked(runtimeApi.stopGroup).mockResolvedValue({
      scope: 'group',
      targetId: 'group-frontend',
      status: 'STOPPED',
      requestedProjectIds: [],
      affectedProjectIds: [],
      skippedProjectIds: [],
      failures: [],
    })
    vi.mocked(runtimeApi.startWorkspace).mockResolvedValue({
      scope: 'workspace',
      targetId: 'workspace-main',
      status: 'STOPPED',
      requestedProjectIds: [],
      affectedProjectIds: [],
      skippedProjectIds: [],
      failures: [],
    })
    vi.mocked(runtimeApi.stopWorkspace).mockResolvedValue({
      scope: 'workspace',
      targetId: 'workspace-main',
      status: 'STOPPED',
      requestedProjectIds: [],
      affectedProjectIds: [],
      skippedProjectIds: [],
      failures: [],
    })
    vi.mocked(workspaceApi.deleteProject).mockResolvedValue(true)
    vi.mocked(workspaceApi.updateGroup).mockResolvedValue(
      workspaceTree.groups[0],
    )
    vi.mocked(workspaceApi.updateProject).mockResolvedValue(project)
    vi.mocked(workspaceApi.getProjectGitInfo).mockResolvedValue({
      isRepository: false,
      branch: null,
    })
    vi.mocked(workspaceApi.createProjectFromDetection).mockResolvedValue(
      project,
    )
    vi.mocked(workspaceApi.validateProjectCommand).mockResolvedValue({
      isRunnable: true,
      commandPreview: 'pnpm dev',
      resolvedExecutable: 'C:\\Tools\\pnpm.cmd',
      issues: [],
    })
  })

  it('renders the left navigator, opens the workspace modal and can expand another workspace', async () => {
    render(<CentralitaApp />)

    expect(
      await screen.findByRole('heading', {
        name: 'La Centralita',
      }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('heading', {
        name: 'Workspaces, grupos y proyectos',
      }),
    ).toBeInTheDocument()
    expect(
      await screen.findByRole('button', {
        name: 'Renombrar workspace',
      }),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '+ Nuevo workspace' }))
    expect(await screen.findByRole('dialog')).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: 'Nuevo workspace' }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Crear workspace' }),
    ).toBeDisabled()

    fireEvent.change(screen.getByPlaceholderText('Nuevo workspace'), {
      target: { value: 'Nuevo workspace' },
    })
    expect(
      screen.getByRole('button', { name: 'Crear workspace' }),
    ).toBeEnabled()

    fireEvent.click(
      screen.getByRole('button', { name: 'Expandir Operaciones' }),
    )
    expect(
      await screen.findByRole('button', { name: 'Abrir grupo Backend' }),
    ).toBeInTheDocument()
    expect(
      within(
        screen.getByRole('button', { name: 'Abrir workspace Centralita' }),
      ).getByText('Workspace'),
    ).toBeInTheDocument()
    expect(
      within(
        screen.getByRole('button', { name: 'Abrir workspace Operaciones' }),
      ).getByText('Workspace'),
    ).toBeInTheDocument()
    expect(
      within(
        screen.getByRole('button', { name: 'Abrir workspace Centralita' }),
      ).getByTitle('STOPPED'),
    ).toBeInTheDocument()
  })

  it('enables back and forward navigation only when history is available', async () => {
    render(<CentralitaApp />)

    const backButton = await screen.findByRole('button', {
      name: 'Navegar atras',
    })
    const forwardButton = screen.getByRole('button', {
      name: 'Navegar adelante',
    })

    expect(backButton).toBeDisabled()
    expect(forwardButton).toBeDisabled()

    fireEvent.click(
      await screen.findByRole('button', { name: 'Abrir grupo Frontend' }),
    )

    await waitFor(() => {
      expect(requireTreeRow('Abrir grupo Frontend')).toHaveClass('selected')
    })
    expect(backButton).toBeEnabled()
    expect(forwardButton).toBeDisabled()

    fireEvent.click(backButton)

    await waitFor(() => {
      expect(requireTreeRow('Abrir workspace Centralita')).toHaveClass(
        'selected',
      )
    })
    expect(backButton).toBeDisabled()
    expect(forwardButton).toBeEnabled()

    fireEvent.click(forwardButton)

    await waitFor(() => {
      expect(requireTreeRow('Abrir grupo Frontend')).toHaveClass('selected')
    })
    expect(backButton).toBeEnabled()
    expect(forwardButton).toBeDisabled()
  })

  it('persists navigator width changes from the accessible resize separator', async () => {
    render(<CentralitaApp />)

    const resizeHandle = await screen.findByRole('separator', {
      name: 'Redimensionar navegador',
    })
    const shell = resizeHandle.closest('.workspace-shell') as HTMLElement

    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      value: 1200,
    })
    Object.defineProperty(shell, 'clientWidth', {
      configurable: true,
      value: 1200,
    })

    await act(async () => {
      fireEvent.keyDown(resizeHandle, { key: 'ArrowRight' })
    })

    expect(window.localStorage.getItem('centralita:navigator-width')).toBe(
      '256',
    )
  })

  it('shows workspace project status cards filtered by the default running status plus errors', async () => {
    const runningProject: ProjectNode = {
      ...project,
      id: 'project-api',
      name: 'centralita-api',
    }
    const failedProject: ProjectNode = {
      ...project,
      id: 'project-worker',
      name: 'centralita-worker',
    }
    const stoppedProject: ProjectNode = {
      ...project,
      id: 'project-docs',
      name: 'centralita-docs',
    }
    const currentTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          projects: [stoppedProject, runningProject, failedProject],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentTree,
    )
    vi.mocked(runtimeApi.getWorkspaceRuntimeStatus).mockResolvedValue({
      workspaceId: 'workspace-main',
      status: 'RUNNING',
      projects: [
        {
          projectId: runningProject.id,
          status: 'RUNNING',
          pid: 1001,
          startedAt: '2026-04-16T07:15:00Z',
          stoppedAt: null,
          exitCode: null,
          lastError: null,
          commandPreview: 'pnpm dev',
        },
        {
          projectId: failedProject.id,
          status: 'FAILED',
          pid: null,
          startedAt: '2026-04-16T07:10:00Z',
          stoppedAt: '2026-04-16T07:11:00Z',
          exitCode: 1,
          lastError: 'Process failed',
          commandPreview: 'pnpm dev',
        },
        {
          projectId: stoppedProject.id,
          status: 'STOPPED',
          pid: null,
          startedAt: null,
          stoppedAt: null,
          exitCode: null,
          lastError: null,
          commandPreview: 'pnpm dev',
        },
      ],
    })

    render(<CentralitaApp />)

    const runningCard = await screen.findByRole('button', {
      name: 'Abrir detalle de proyecto centralita-api',
    })
    expect(await within(runningCard).findByTitle('RUNNING')).toBeInTheDocument()
    expect(
      screen.getByRole('button', {
        name: 'Abrir detalle de proyecto centralita-worker',
      }),
    ).toBeInTheDocument()
    await waitFor(() => {
      expect(
        screen.queryByRole('button', {
          name: 'Abrir detalle de proyecto centralita-docs',
        }),
      ).not.toBeInTheDocument()
    })

    fireEvent.change(screen.getByLabelText('Mostrar'), {
      target: { value: 'ALL' },
    })

    expect(
      screen.getByRole('button', {
        name: 'Abrir detalle de proyecto centralita-docs',
      }),
    ).toBeInTheDocument()
  })

  it('shows the relative group path when workspace project cards are grouped by groups', async () => {
    const nestedProject: ProjectNode = {
      ...project,
      groupId: 'group-feature',
    }
    const currentTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          projects: [],
          groups: [
            {
              id: 'group-feature',
              workspaceId: 'workspace-main',
              parentGroupId: 'group-frontend',
              name: 'Feature',
              color: '#2563eb',
              sortOrder: 20,
              createdAt: '2026-04-16T07:00:00Z',
              updatedAt: '2026-04-16T07:00:00Z',
              projects: [nestedProject],
              groups: [],
            },
          ],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentTree,
    )

    render(<CentralitaApp />)

    expect(
      await screen.findByRole('button', {
        name: 'Abrir detalle de proyecto centralita-ui',
      }),
    ).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Agrupar por'), {
      target: { value: 'GROUPS' },
    })

    expect(await screen.findByText('Frontend/Feature')).toBeInTheDocument()
  })

  it('shows group project card paths relative to the selected group detail', async () => {
    const nestedProject: ProjectNode = {
      ...project,
      groupId: 'group-feature',
    }
    const currentTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          projects: [],
          groups: [
            {
              id: 'group-feature',
              workspaceId: 'workspace-main',
              parentGroupId: 'group-frontend',
              name: 'Feature',
              color: '#2563eb',
              sortOrder: 20,
              createdAt: '2026-04-16T07:00:00Z',
              updatedAt: '2026-04-16T07:00:00Z',
              projects: [nestedProject],
              groups: [],
            },
          ],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentTree,
    )

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Abrir grupo Frontend' }),
    )
    expect(
      await screen.findByRole('button', {
        name: 'Abrir detalle de proyecto centralita-ui',
      }),
    ).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Agrupar por'), {
      target: { value: 'GROUPS' },
    })

    await waitFor(() => {
      expect(
        document.querySelector(
          '.workspace-project-status-group-title span[title="Feature"]',
        ),
      ).not.toBeNull()
    })
    expect(screen.queryByText('Frontend/Feature')).not.toBeInTheDocument()
  })

  it('allows collapsing the active workspace from the navigator toggle', async () => {
    render(<CentralitaApp />)

    expect(
      await screen.findByRole('button', { name: 'Abrir grupo Frontend' }),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Contraer Centralita' }))

    await waitFor(() => {
      expect(
        screen.queryByRole('button', { name: 'Abrir grupo Frontend' }),
      ).not.toBeInTheDocument()
    })
    expect(
      screen.getByRole('button', { name: 'Expandir Centralita' }),
    ).toBeInTheDocument()
  })

  it('scrolls the detail panel to the top when selecting another item', async () => {
    render(<CentralitaApp />)

    await screen.findByRole('button', { name: 'Abrir grupo Frontend' })
    scrollToMock.mockClear()
    scrollIntoViewMock.mockClear()

    fireEvent.click(
      screen.getByRole('button', { name: 'Abrir grupo Frontend' }),
    )

    await waitFor(() => {
      expect(scrollToMock).toHaveBeenCalledWith({ left: 0, top: 0 })
    })
    expect(scrollIntoViewMock).toHaveBeenCalledWith({
      block: 'start',
      inline: 'nearest',
    })

    scrollToMock.mockClear()
    scrollIntoViewMock.mockClear()
    fireEvent.click(screen.getByRole('button', { name: 'Expandir Frontend' }))
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    await waitFor(() => {
      expect(scrollToMock).toHaveBeenCalledWith({ left: 0, top: 0 })
    })
    expect(scrollIntoViewMock).toHaveBeenCalledWith({
      block: 'start',
      inline: 'nearest',
    })
  })

  it('shows the empty workspace message without a collapse toggle or workspace label', async () => {
    const emptyMainTree: WorkspaceTree = {
      workspace,
      groups: [],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : emptyMainTree,
    )

    render(<CentralitaApp />)

    const emptyMessage = await screen.findByText(
      'Todavía no hay grupos ni proyectos en este workspace.',
    )
    const emptyRow = emptyMessage.closest(
      '.navigator-row-empty-state',
    ) as HTMLElement | null

    expect(emptyRow).not.toBeNull()
    if (!emptyRow) {
      throw new Error('Expected empty workspace row to exist')
    }
    expect(
      screen
        .getByRole('button', { name: 'Abrir workspace Centralita' })
        .closest('.navigator-row')
        ?.querySelector('.navigator-toggle-placeholder'),
    ).not.toBeNull()
    expect(
      screen.queryByRole('button', { name: 'Contraer Centralita' }),
    ).not.toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'Expandir Centralita' }),
    ).not.toBeInTheDocument()
    expect(within(emptyRow).queryByText('Workspace')).not.toBeInTheDocument()
  })

  it('keeps the empty workspace message visible for loaded workspaces without content', () => {
    const emptyOpsTree: WorkspaceTree = {
      workspace: workspaceOps,
      groups: [],
    }

    render(
      <WorkspaceRuntimeTreeView
        activeWorkspaceId={workspace.id}
        isLoadingPersistedState={false}
        onEnsureWorkspaceTree={vi.fn().mockResolvedValue(undefined)}
        onRequestMove={vi.fn()}
        onSelectGroup={vi.fn()}
        onSelectProject={vi.fn()}
        onSelectWorkspace={vi.fn()}
        runtimeFilter="ALL"
        selectedItem={{ id: workspace.id, type: 'workspace' }}
        statusByProjectId={{}}
        workspaceStatus="STOPPED"
        workspaceTrees={{
          [workspace.id]: workspaceTree,
          [workspaceOps.id]: emptyOpsTree,
        }}
        workspaces={[workspace, workspaceOps]}
      />,
    )

    expect(
      screen.getByText('Todavía no hay grupos ni proyectos en este workspace.'),
    ).toBeInTheDocument()
    expect(
      screen
        .getByRole('button', { name: 'Abrir workspace Operaciones' })
        .closest('.navigator-row')
        ?.querySelector('.navigator-toggle-placeholder'),
    ).not.toBeNull()
    expect(
      screen.queryByRole('button', { name: 'Expandir Operaciones' }),
    ).not.toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'Contraer Operaciones' }),
    ).not.toBeInTheDocument()
  })

  it('hides the toggle affordance for groups without children and keeps the label left aligned', async () => {
    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Operaciones' }),
    )

    const groupButton = await screen.findByRole('button', {
      name: 'Abrir grupo Backend',
    })
    const groupRow = groupButton.closest('.navigator-row')

    expect(groupRow).not.toBeNull()
    expect(
      groupRow?.querySelector('.navigator-toggle-placeholder'),
    ).not.toBeNull()
    expect(groupRow?.querySelector('.tree-chip')).toBeNull()
    expect(
      screen.queryByRole('button', { name: 'Expandir Backend' }),
    ).not.toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'Contraer Backend' }),
    ).not.toBeInTheDocument()
  })

  it('imports a project from the selected group detail', async () => {
    let currentMainTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          projects: [],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.analyzeProjectFolder).mockResolvedValue({
      detectedType: 'reactVite',
      displayName: '@acme/centralita-web',
      path: 'C:\\Proyectos\\centralita-ui',
      workingDir: 'C:\\Proyectos\\centralita-ui',
      packageManager: 'pnpm',
      executable: 'pnpm',
      command: 'pnpm dev',
      args: ['dev'],
      commandPreview: 'pnpm dev',
      commandValidation: {
        isRunnable: true,
        commandPreview: 'pnpm dev',
        resolvedExecutable: 'C:\\Tools\\pnpm.cmd',
        issues: [],
      },
      confidence: 0.9,
      evidence: [
        {
          kind: 'dependency',
          source: 'package.json',
          detail: 'react dependency found',
          weight: 0.25,
        },
      ],
      warnings: [],
    })
    vi.mocked(workspaceApi.createProjectFromDetection).mockImplementation(
      async (input) => {
        currentMainTree = {
          ...currentMainTree,
          groups: currentMainTree.groups.map((group) =>
            group.id === input.groupId
              ? { ...group, projects: [project] }
              : group,
          ),
        }

        return project
      },
    )

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Abrir grupo Frontend' }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Importar proyecto' }))
    const importDialog = await screen.findByRole('dialog', {
      name: 'Importar proyecto',
    })
    expect(
      within(importDialog).getByRole('heading', { name: 'Importar proyecto' }),
    ).toBeInTheDocument()

    fireEvent.change(
      within(importDialog).getByPlaceholderText(/mi-proyecto/i),
      {
        target: { value: 'C:\\Proyectos\\centralita-ui' },
      },
    )
    fireEvent.click(
      within(importDialog).getByRole('button', { name: 'Analizar carpeta' }),
    )

    expect(
      await screen.findByRole('heading', { name: 'Revisar detección' }),
    ).toBeInTheDocument()
    const reviewDialog = screen.getByRole('dialog', {
      name: 'Revisar detección',
    })
    const reviewBackdrop = reviewDialog.parentElement
    expect(
      within(reviewDialog).queryByText('Color opcional'),
    ).not.toBeInTheDocument()

    expect(reviewBackdrop).not.toBeNull()
    fireEvent.click(reviewBackdrop!)
    expect(
      screen.getByRole('dialog', { name: 'Revisar detección' }),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Guardar proyecto' }))

    await waitFor(() => {
      expect(workspaceApi.createProjectFromDetection).toHaveBeenCalledWith(
        expect.objectContaining({
          groupId: 'group-frontend',
          name: 'centralita-ui',
          workspaceId: 'workspace-main',
        }),
      )
    })

    expect(
      await screen.findByRole('button', { name: 'Contraer Frontend' }),
    ).toBeInTheDocument()
    expect(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    ).toBeInTheDocument()
  })

  it('disables group creation until the name is valid and unique', async () => {
    render(<CentralitaApp />)

    fireEvent.click(await screen.findByRole('button', { name: 'Crear grupo' }))
    const createGroupDialog = await screen.findByRole('dialog', {
      name: 'Crear grupo o subgrupo',
    })
    expect(
      within(createGroupDialog).getByRole('heading', {
        name: 'Crear grupo o subgrupo',
      }),
    ).toBeInTheDocument()
    await waitFor(() => {
      expect(runtimeApi.getProjectLogs).toHaveBeenCalled()
    })
    expect(
      within(createGroupDialog).getByRole('button', { name: 'Crear grupo' }),
    ).toBeDisabled()
    expect(
      within(createGroupDialog).queryByText('Color'),
    ).not.toBeInTheDocument()

    fireEvent.change(
      within(createGroupDialog).getByPlaceholderText('Frontend'),
      {
        target: { value: 'Frontend' },
      },
    )
    expect(
      within(createGroupDialog).getByRole('button', { name: 'Crear grupo' }),
    ).toBeDisabled()
    expect(workspaceApi.createGroup).not.toHaveBeenCalled()

    fireEvent.change(
      within(createGroupDialog).getByPlaceholderText('Frontend'),
      {
        target: { value: 'Backend' },
      },
    )
    expect(
      within(createGroupDialog).getByRole('button', { name: 'Crear grupo' }),
    ).toBeEnabled()
  })

  it('keeps loaded navigator trees visible after renaming a workspace', async () => {
    let currentWorkspaces = [workspace, workspaceOps]
    let currentMainTree = workspaceTree
    const currentOpsTree = workspaceOpsTree

    vi.mocked(workspaceApi.listWorkspaces).mockImplementation(
      async () => currentWorkspaces,
    )
    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? currentOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.renameWorkspace).mockImplementation(
      async (input) => {
        currentWorkspaces = currentWorkspaces.map((item) =>
          item.id === input.id ? { ...item, name: input.name } : item,
        )
        currentMainTree = {
          ...currentMainTree,
          workspace: { ...currentMainTree.workspace, name: input.name },
        }

        return currentMainTree.workspace
      },
    )

    render(<CentralitaApp />)

    expect(
      await screen.findByRole('button', { name: 'Abrir workspace Centralita' }),
    ).toBeInTheDocument()
    fireEvent.click(
      screen.getByRole('button', { name: 'Expandir Operaciones' }),
    )
    expect(
      await screen.findByRole('button', { name: 'Abrir grupo Backend' }),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Renombrar workspace' }))
    const renameDialog = await screen.findByRole('dialog', {
      name: 'Renombrar workspace',
    })
    fireEvent.change(within(renameDialog).getByLabelText('Nombre'), {
      target: { value: 'Centralita Renombrada' },
    })
    fireEvent.click(
      within(renameDialog).getByRole('button', { name: 'Guardar nombre' }),
    )

    await waitFor(() => {
      expect(workspaceApi.renameWorkspace).toHaveBeenCalledWith({
        id: 'workspace-main',
        name: 'Centralita Renombrada',
      })
    })

    expect(
      await screen.findByRole('button', {
        name: 'Abrir workspace Centralita Renombrada',
      }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Abrir grupo Backend' }),
    ).toBeInTheDocument()
  })

  it('refreshes the navigator after creating a group without dropping other loaded trees', async () => {
    let currentMainTree = workspaceTree
    const currentOpsTree = workspaceOpsTree

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? currentOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.createGroup).mockImplementation(async (input) => {
      const createdGroup = {
        id: 'group-qa',
        workspaceId: input.workspaceId,
        parentGroupId: input.parentGroupId,
        name: input.name,
        color: input.color,
        sortOrder: 30,
        createdAt: '2026-04-17T08:00:00Z',
        updatedAt: '2026-04-17T08:00:00Z',
      }

      currentMainTree = {
        ...currentMainTree,
        groups: [
          ...currentMainTree.groups,
          {
            ...createdGroup,
            groups: [],
            projects: [],
          },
        ],
      }

      return createdGroup
    })

    render(<CentralitaApp />)

    expect(
      await screen.findByRole('button', { name: 'Abrir workspace Centralita' }),
    ).toBeInTheDocument()
    fireEvent.click(
      screen.getByRole('button', { name: 'Expandir Operaciones' }),
    )
    expect(
      await screen.findByRole('button', { name: 'Abrir grupo Backend' }),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Crear grupo' }))
    const createGroupDialog = await screen.findByRole('dialog', {
      name: 'Crear grupo o subgrupo',
    })
    fireEvent.change(
      within(createGroupDialog).getByPlaceholderText('Frontend'),
      {
        target: { value: 'QA' },
      },
    )
    fireEvent.click(
      within(createGroupDialog).getByRole('button', { name: 'Crear grupo' }),
    )

    await waitFor(() => {
      expect(workspaceApi.createGroup).toHaveBeenCalledWith({
        color: '#2f855a',
        name: 'QA',
        parentGroupId: null,
        workspaceId: 'workspace-main',
      })
    })

    expect(
      await screen.findByRole('button', { name: 'Abrir grupo QA' }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Abrir grupo Backend' }),
    ).toBeInTheDocument()
  })

  it('creates a subgroup from the selected group without exposing workspace root or sibling groups', async () => {
    let currentMainTree: WorkspaceTree = {
      ...workspaceTree,
      groups: [
        {
          ...workspaceTree.groups[0],
          groups: [
            {
              id: 'group-ui',
              workspaceId: 'workspace-main',
              parentGroupId: 'group-frontend',
              name: 'UI',
              color: '#7c3aed',
              sortOrder: 10,
              createdAt: '2026-04-17T08:00:00Z',
              updatedAt: '2026-04-17T08:00:00Z',
              groups: [],
              projects: [],
            },
          ],
        },
        {
          id: 'group-backend-local',
          workspaceId: 'workspace-main',
          parentGroupId: null,
          name: 'Backend',
          color: '#2563eb',
          sortOrder: 20,
          createdAt: '2026-04-17T08:00:00Z',
          updatedAt: '2026-04-17T08:00:00Z',
          groups: [],
          projects: [],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.createGroup).mockImplementation(async (input) => {
      const createdGroup = {
        id: 'group-design-system',
        workspaceId: input.workspaceId,
        parentGroupId: input.parentGroupId,
        name: input.name,
        color: input.color,
        sortOrder: 30,
        createdAt: '2026-04-17T09:00:00Z',
        updatedAt: '2026-04-17T09:00:00Z',
      }

      currentMainTree = {
        ...currentMainTree,
        groups: currentMainTree.groups.map((group) =>
          group.id === input.parentGroupId
            ? {
                ...group,
                groups: [
                  ...group.groups,
                  { ...createdGroup, groups: [], projects: [] },
                ],
              }
            : group,
        ),
      }

      return createdGroup
    })

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Abrir grupo Frontend' }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Crear grupo' }))

    const createGroupDialog = await screen.findByRole('dialog', {
      name: 'Crear grupo o subgrupo',
    })
    const parentSelect = within(createGroupDialog).getByLabelText('Grupo padre')

    expect(parentSelect).toHaveValue('group-frontend')
    expect(
      within(createGroupDialog).queryByRole('option', {
        name: 'Raíz del workspace',
      }),
    ).not.toBeInTheDocument()
    expect(
      within(createGroupDialog).getByRole('option', { name: 'Frontend' }),
    ).toBeInTheDocument()
    expect(
      within(createGroupDialog).getByRole('option', { name: 'UI' }),
    ).toBeInTheDocument()
    expect(
      within(createGroupDialog).queryByRole('option', { name: 'Backend' }),
    ).not.toBeInTheDocument()

    fireEvent.change(
      within(createGroupDialog).getByPlaceholderText('Frontend'),
      {
        target: { value: 'UI' },
      },
    )
    expect(
      within(createGroupDialog).getByRole('button', { name: 'Crear grupo' }),
    ).toBeDisabled()

    fireEvent.change(
      within(createGroupDialog).getByPlaceholderText('Frontend'),
      {
        target: { value: 'Frontend' },
      },
    )
    expect(
      within(createGroupDialog).getByRole('button', { name: 'Crear grupo' }),
    ).toBeEnabled()
    fireEvent.click(
      within(createGroupDialog).getByRole('button', { name: 'Crear grupo' }),
    )

    await waitFor(() => {
      expect(workspaceApi.createGroup).toHaveBeenCalledWith({
        color: '#2f855a',
        name: 'Frontend',
        parentGroupId: 'group-frontend',
        workspaceId: 'workspace-main',
      })
    })
  })

  it('edits a project without showing the windows path prefix', async () => {
    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Configurar proyecto' }))
    const projectDialog = await screen.findByRole('dialog', {
      name: 'Configuración del proyecto',
    })

    expect(
      within(projectDialog).getByRole('heading', {
        name: 'Configuración del proyecto',
      }),
    ).toBeInTheDocument()
    expect(within(projectDialog).getByLabelText('Ruta')).toHaveValue(
      'C:\\Proyectos\\centralita-ui',
    )
    expect(within(projectDialog).getByLabelText('Working dir')).toHaveValue(
      'C:\\Proyectos\\centralita-ui',
    )
    expect(
      within(projectDialog).getByRole('button', { name: 'Guardar proyecto' }),
    ).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Stop' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Restart' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Limpiar vista' })).toBeDisabled()

    fireEvent.change(within(projectDialog).getByLabelText('Ruta'), {
      target: { value: 'C:\\Proyectos\\centralita-ui-renamed' },
    })
    expect(
      within(projectDialog).getByRole('button', { name: 'Guardar proyecto' }),
    ).toBeEnabled()
    fireEvent.change(within(projectDialog).getByLabelText('Ejecutable'), {
      target: { value: 'npm' },
    })
    fireEvent.change(
      within(projectDialog).getByLabelText(
        'Argumentos de arranque (uno por línea)',
      ),
      {
        target: { value: 'run\ndev' },
      },
    )
    fireEvent.click(
      within(projectDialog).getByRole('button', { name: 'Guardar proyecto' }),
    )

    await waitFor(() => {
      expect(workspaceApi.updateProject).toHaveBeenCalledWith(
        expect.objectContaining({
          command: 'npm run dev',
          executable: 'npm',
          path: 'C:\\Proyectos\\centralita-ui-renamed',
          workingDir: 'C:\\Proyectos\\centralita-ui',
        }),
      )
    })
  })

  it('shows the current git branch in the project detail when the working dir is a repository', async () => {
    vi.mocked(workspaceApi.getProjectGitInfo).mockResolvedValue({
      isRepository: true,
      branch: 'feature/detail',
    })

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    const gitHeading = await screen.findByRole('heading', { name: 'Git' })
    const gitCard = gitHeading.closest('article')
    expect(gitCard).not.toBeNull()
    expect(within(gitCard as HTMLElement).getByText('Rama')).toBeInTheDocument()
    expect(
      within(gitCard as HTMLElement).getByText('feature/detail'),
    ).toBeInTheDocument()
    expect(workspaceApi.getProjectGitInfo).toHaveBeenCalledWith({
      path: 'C:\\Proyectos\\centralita-ui',
    })
  })

  it('does not show the git card when the project working dir is not a repository', async () => {
    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    await waitFor(() => {
      expect(workspaceApi.getProjectGitInfo).toHaveBeenCalled()
    })
    expect(
      screen.queryByRole('heading', { name: 'Git' }),
    ).not.toBeInTheDocument()
  })

  it('refreshes the navigator after editing a project without dropping other loaded trees', async () => {
    let currentMainTree = workspaceTree
    const currentOpsTree = workspaceOpsTree

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? currentOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.updateProject).mockImplementation(async (input) => {
      const updatedProject = {
        ...project,
        name: input.name,
        path: input.path,
        executable: input.executable,
        command: input.command,
        args: input.args,
        workingDir: input.workingDir,
      }

      currentMainTree = {
        ...currentMainTree,
        groups: currentMainTree.groups.map((group) =>
          group.id === 'group-frontend'
            ? { ...group, projects: [updatedProject] }
            : group,
        ),
      }

      return updatedProject
    })

    render(<CentralitaApp />)

    expect(
      await screen.findByRole('button', { name: 'Abrir workspace Centralita' }),
    ).toBeInTheDocument()
    fireEvent.click(
      screen.getByRole('button', { name: 'Expandir Operaciones' }),
    )
    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    expect(
      await screen.findByRole('button', { name: 'Abrir grupo Backend' }),
    ).toBeInTheDocument()

    fireEvent.click(
      screen.getByRole('button', { name: 'Abrir proyecto centralita-ui' }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Configurar proyecto' }))
    const projectDialog = await screen.findByRole('dialog', {
      name: 'Configuración del proyecto',
    })
    fireEvent.change(within(projectDialog).getByLabelText('Nombre'), {
      target: { value: 'centralita-web' },
    })
    fireEvent.click(
      within(projectDialog).getByRole('button', { name: 'Guardar proyecto' }),
    )

    await waitFor(() => {
      expect(workspaceApi.updateProject).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'project-ui',
          name: 'centralita-web',
        }),
      )
    })

    expect(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-web',
      }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Abrir grupo Backend' }),
    ).toBeInTheDocument()
  })

  it('realigns the runtime panel after saving a stopped or failed project', async () => {
    const failedRuntime = {
      workspaceId: 'workspace-main',
      status: 'FAILED' as const,
      projects: [
        {
          projectId: 'project-ui',
          status: 'FAILED' as const,
          pid: null,
          startedAt: null,
          stoppedAt: '2026-04-16T07:30:00Z',
          exitCode: null,
          lastError: 'Failed to start project',
          commandPreview: 'pnpm dev',
        },
      ],
    }
    let currentTree = workspaceTree

    vi.mocked(runtimeApi.getWorkspaceRuntimeStatus).mockResolvedValue(
      failedRuntime,
    )
    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentTree,
    )
    vi.mocked(workspaceApi.updateProject).mockImplementation(async (input) => {
      const updatedProject: typeof project = {
        ...project,
        args: input.args ?? project.args,
        command: input.command ?? project.command,
        executable: input.executable ?? project.executable,
        name: input.name ?? project.name,
        path: input.path ?? project.path,
        workingDir: input.workingDir ?? project.workingDir,
      }

      currentTree = {
        ...workspaceTree,
        groups: workspaceTree.groups.map((group) =>
          group.id === 'group-frontend'
            ? { ...group, projects: [updatedProject] }
            : group,
        ),
      }

      return updatedProject
    })

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    expect(
      await screen.findByText('Failed to start project'),
    ).toBeInTheDocument()
    expect(screen.getAllByText('FAILED').length).toBeGreaterThan(0)

    fireEvent.click(screen.getByRole('button', { name: 'Configurar proyecto' }))
    const projectDialog = await screen.findByRole('dialog', {
      name: 'Configuración del proyecto',
    })
    fireEvent.change(within(projectDialog).getByLabelText('Ejecutable'), {
      target: { value: 'npm' },
    })
    fireEvent.change(
      within(projectDialog).getByLabelText(
        'Argumentos de arranque (uno por línea)',
      ),
      {
        target: { value: 'run\ndev' },
      },
    )
    fireEvent.click(
      within(projectDialog).getByRole('button', { name: 'Guardar proyecto' }),
    )

    await waitFor(() => {
      expect(workspaceApi.updateProject).toHaveBeenCalledWith(
        expect.objectContaining({
          command: 'npm run dev',
          executable: 'npm',
        }),
      )
    })

    await waitFor(() => {
      const runtimePanel = screen
        .getByRole('heading', { name: 'Runtime' })
        .closest('article')

      expect(runtimePanel).not.toBeNull()
      const commandRow = within(runtimePanel!)
        .getByText('Comando')
        .closest('li')
      const runtimeErrorRow = within(runtimePanel!)
        .getByText('Último error runtime')
        .closest('li')

      expect(commandRow).not.toBeNull()
      expect(runtimeErrorRow).not.toBeNull()
      expect(within(commandRow!).getByText('npm run dev')).toBeInTheDocument()
      expect(screen.getAllByText('STOPPED').length).toBeGreaterThan(0)
      expect(
        within(runtimeErrorRow!).getByText('Sin errores registrados'),
      ).toBeInTheDocument()
    })
  })

  it('opens the runtime error history from the error metric', async () => {
    vi.mocked(runtimeApi.getWorkspaceRuntimeStatus).mockResolvedValue({
      workspaceId: 'workspace-main',
      status: 'FAILED' as const,
      projects: [
        {
          projectId: 'project-ui',
          status: 'FAILED' as const,
          pid: null,
          startedAt: '2026-04-16T07:29:00Z',
          stoppedAt: '2026-04-16T07:30:00Z',
          exitCode: null,
          lastError: 'Failed to start project',
          commandPreview: 'pnpm dev',
        },
      ],
    })
    vi.mocked(runtimeApi.listProjectRunHistory).mockResolvedValue([
      {
        id: 'history-error-1',
        projectId: 'project-ui',
        startedAt: '2026-04-16T07:29:00Z',
        endedAt: '2026-04-16T07:30:00Z',
        exitCode: 1,
        finalRuntimeStatus: 'FAILED' as const,
        stopReason: 'process exited',
        errorMessage: 'Vite failed during startup',
        commandPreview: 'pnpm dev',
      },
    ])

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Ver historial de errores',
      }),
    )

    const dialog = await screen.findByRole('dialog', {
      name: 'Historial de errores',
    })

    expect(
      within(dialog).getByText('Failed to start project'),
    ).toBeInTheDocument()
    expect(
      within(dialog).getByText('Vite failed during startup'),
    ).toBeInTheDocument()
    expect(within(dialog).getByText('Actual')).toBeInTheDocument()
    expect(within(dialog).getByText('Historial')).toBeInTheDocument()
  })

  it('clears stale runtime error and logs before a new start attempt', async () => {
    const failedRuntime = {
      workspaceId: 'workspace-main',
      status: 'FAILED' as const,
      projects: [
        {
          projectId: 'project-ui',
          status: 'FAILED' as const,
          pid: null,
          startedAt: null,
          stoppedAt: '2026-04-16T07:30:00Z',
          exitCode: null,
          lastError: 'El sistema no puede encontrar la ruta especificada.',
          commandPreview: 'npm run start',
        },
      ],
    }

    vi.mocked(runtimeApi.getWorkspaceRuntimeStatus).mockResolvedValue(
      failedRuntime,
    )
    vi.mocked(runtimeApi.getProjectLogs).mockResolvedValue([
      {
        projectId: 'project-ui',
        stream: 'stderr',
        line: 'El sistema no puede encontrar la ruta especificada.',
        timestamp: '2026-04-16T07:30:00Z',
      },
    ])

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    expect(
      await screen.findAllByText(
        'El sistema no puede encontrar la ruta especificada.',
      ),
    ).not.toHaveLength(0)

    fireEvent.click(screen.getByRole('button', { name: 'Start' }))

    await waitFor(() => {
      const runtimePanel = screen
        .getByRole('heading', { name: 'Runtime' })
        .closest('article')

      expect(runtimePanel).not.toBeNull()
      const runtimeErrorRow = within(runtimePanel!)
        .getByText('Último error runtime')
        .closest('li')

      expect(runtimeErrorRow).not.toBeNull()
      expect(runtimeApi.startProject).toHaveBeenCalledWith({
        projectId: 'project-ui',
      })
      expect(
        screen.queryByText(
          'El sistema no puede encontrar la ruta especificada.',
        ),
      ).not.toBeInTheDocument()
      expect(
        within(runtimeErrorRow!).getByText('Sin errores registrados'),
      ).toBeInTheDocument()
      expect(screen.getByText('Todavía no hay logs.')).toBeInTheDocument()
    })
  })

  it('keeps the command preview from runtime status events after starting', async () => {
    const listenersByEvent = new Map<
      string,
      Array<(payload: unknown) => void>
    >()
    const failedRuntime = {
      workspaceId: 'workspace-main',
      status: 'FAILED' as const,
      projects: [
        {
          projectId: 'project-ui',
          status: 'FAILED' as const,
          pid: null,
          startedAt: null,
          stoppedAt: '2026-04-16T07:30:00Z',
          exitCode: null,
          lastError: 'Failed to start project',
          commandPreview: 'pnpm dev',
        },
      ],
    }

    vi.mocked(runtimeApi.getWorkspaceRuntimeStatus).mockResolvedValue(
      failedRuntime,
    )
    vi.mocked(listenRuntimeEvent).mockImplementation(
      async (eventName, handler) => {
        const listeners = listenersByEvent.get(eventName) ?? []
        listeners.push(handler as (payload: unknown) => void)
        listenersByEvent.set(eventName, listeners)

        return () => {
          const current = listenersByEvent.get(eventName) ?? []
          listenersByEvent.set(
            eventName,
            current.filter((currentHandler) => currentHandler !== handler),
          )
        }
      },
    )

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    expect(
      await screen.findByText('Failed to start project'),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Start' }))

    const statusListeners =
      listenersByEvent.get(RUNTIME_EVENTS.statusChanged) ?? []
    expect(statusListeners.length).toBeGreaterThan(0)

    await act(async () => {
      statusListeners.forEach((handler) =>
        handler({
          projectId: 'project-ui',
          status: 'RUNNING' as const,
          pid: 4812,
          timestamp: '2026-04-16T07:31:00Z',
          message: 'Process started',
          commandPreview: 'pnpm dev',
        }),
      )
    })

    await waitFor(() => {
      const runtimePanel = screen
        .getByRole('heading', { name: 'Runtime' })
        .closest('article')
      expect(runtimePanel).not.toBeNull()

      const commandRow = within(runtimePanel!)
        .getByText('Comando')
        .closest('li')
      expect(commandRow).not.toBeNull()
      expect(within(commandRow!).getByText('pnpm dev')).toBeInTheDocument()
      expect(
        within(commandRow!).queryByText('Manual review required'),
      ).not.toBeInTheDocument()
    })
  })

  it('deduplicates identical runtime log events in the recent logs view', async () => {
    const listenersByEvent = new Map<
      string,
      Array<(payload: unknown) => void>
    >()

    vi.mocked(listenRuntimeEvent).mockImplementation(
      async (eventName, handler) => {
        const listeners = listenersByEvent.get(eventName) ?? []
        listeners.push(handler as (payload: unknown) => void)
        listenersByEvent.set(eventName, listeners)

        return () => {
          const current = listenersByEvent.get(eventName) ?? []
          listenersByEvent.set(
            eventName,
            current.filter((currentHandler) => currentHandler !== handler),
          )
        }
      },
    )

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    const payload = {
      projectId: 'project-ui',
      stream: 'stderr' as const,
      line: 'duplicated line',
      timestamp: '2026-04-16T07:30:00Z',
    }

    const logListeners = listenersByEvent.get(RUNTIME_EVENTS.logLine) ?? []
    expect(logListeners.length).toBeGreaterThan(0)

    await act(async () => {
      logListeners.forEach((handler) => handler(payload))
      logListeners.forEach((handler) => handler(payload))
    })

    await waitFor(() => {
      expect(screen.getAllByText('duplicated line')).toHaveLength(1)
    })
  })

  it('keeps an active runtime state when a process error is reported without exit', async () => {
    const listenersByEvent = new Map<
      string,
      Array<(payload: unknown) => void>
    >()
    const runningRuntime = {
      workspaceId: 'workspace-main',
      status: 'RUNNING' as const,
      projects: [
        {
          projectId: 'project-ui',
          status: 'RUNNING' as const,
          pid: 4812,
          startedAt: '2026-04-16T07:00:00Z',
          stoppedAt: null,
          exitCode: null,
          lastError: null,
          commandPreview: 'pnpm dev',
        },
      ],
    }

    vi.mocked(runtimeApi.getWorkspaceRuntimeStatus).mockResolvedValue(
      runningRuntime,
    )
    vi.mocked(listenRuntimeEvent).mockImplementation(
      async (eventName, handler) => {
        const listeners = listenersByEvent.get(eventName) ?? []
        listeners.push(handler as (payload: unknown) => void)
        listenersByEvent.set(eventName, listeners)

        return () => {
          const current = listenersByEvent.get(eventName) ?? []
          listenersByEvent.set(
            eventName,
            current.filter((currentHandler) => currentHandler !== handler),
          )
        }
      },
    )

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    await screen.findAllByText('RUNNING')

    const payload = {
      projectId: 'project-ui',
      status: 'FAILED' as const,
      pid: null,
      timestamp: '2026-04-16T07:30:00Z',
      message:
        'Failed to read process output: stream did not contain valid UTF-8',
      commandPreview: 'pnpm dev',
    }
    const errorListeners =
      listenersByEvent.get(RUNTIME_EVENTS.processError) ?? []
    expect(errorListeners.length).toBeGreaterThan(0)

    await act(async () => {
      errorListeners.forEach((handler) => handler(payload))
    })

    const runtimePanel = screen
      .getByRole('heading', { name: 'Runtime' })
      .closest('article')
    expect(runtimePanel).not.toBeNull()

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Start' })).toBeDisabled()
      expect(screen.getByRole('button', { name: 'Stop' })).not.toBeDisabled()
      expect(
        within(runtimePanel!).getByText(payload.message),
      ).toBeInTheDocument()
    })
  })

  it('places memory log lines directly after the runtime state card', async () => {
    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )

    const runtimeHeading = await screen.findByRole('heading', {
      name: 'Runtime',
    })
    const terminalHeading = screen.getByRole('heading', {
      name: 'Terminal integrada',
    })
    const historyHeading = screen.getByRole('heading', {
      name: 'Historial reciente',
    })

    expect(
      runtimeHeading.compareDocumentPosition(terminalHeading) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy()
    expect(
      terminalHeading.compareDocumentPosition(historyHeading) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy()
  })

  it('confirms project deletion with the shared modal before removing it', async () => {
    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Abrir proyecto centralita-ui',
      }),
    )
    fireEvent.click(
      await screen.findByRole('button', { name: 'Eliminar proyecto' }),
    )

    expect(
      await screen.findByRole('dialog', { name: 'Eliminar proyecto' }),
    ).toBeInTheDocument()
    expect(workspaceApi.deleteProject).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }))
    await waitFor(() => {
      expect(
        screen.queryByRole('dialog', { name: 'Eliminar proyecto' }),
      ).not.toBeInTheDocument()
    })

    fireEvent.click(screen.getByRole('button', { name: 'Eliminar proyecto' }))
    fireEvent.click(
      within(
        await screen.findByRole('dialog', { name: 'Eliminar proyecto' }),
      ).getByRole('button', {
        name: 'Eliminar proyecto',
      }),
    )

    await waitFor(() => {
      expect(workspaceApi.deleteProject).toHaveBeenCalledWith({
        id: 'project-ui',
      })
    })
  })

  it('confirms moving a group to another workspace before updating it', async () => {
    render(<CentralitaApp />)

    await screen.findByRole('button', { name: 'Abrir grupo Frontend' })
    const sourceGroup = requireDragHandle('Arrastrar grupo Frontend')
    const targetWorkspaceRow = requireTreeRow('Abrir workspace Operaciones')

    fireEvent.pointerDown(sourceGroup, { button: 0 })
    fireEvent.pointerMove(targetWorkspaceRow)
    fireEvent.pointerUp(targetWorkspaceRow)

    expect(
      await screen.findByRole('dialog', { name: 'Mover grupo' }),
    ).toBeInTheDocument()
    expect(workspaceApi.updateGroup).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole('button', { name: 'Mover' }))

    await waitFor(() => {
      expect(workspaceApi.updateGroup).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'group-frontend',
          parentGroupId: null,
          workspaceId: 'workspace-ops',
        }),
      )
    })
  })

  it('moves descendant subgroups and projects when moving a group to another workspace', async () => {
    const nestedProject: ProjectNode = {
      ...project,
      id: 'project-design',
      groupId: 'group-design',
      name: 'centralita-design',
    }

    const nestedTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          groups: [
            {
              id: 'group-design',
              workspaceId: 'workspace-main',
              parentGroupId: 'group-frontend',
              name: 'Design',
              color: '#f59e0b',
              sortOrder: 15,
              createdAt: '2026-04-17T08:00:00Z',
              updatedAt: '2026-04-17T08:00:00Z',
              groups: [],
              projects: [nestedProject],
            },
          ],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : nestedTree,
    )

    render(<CentralitaApp />)

    await screen.findByRole('button', { name: 'Abrir grupo Frontend' })
    const sourceGroup = requireDragHandle('Arrastrar grupo Frontend')
    const targetWorkspaceRow = requireTreeRow('Abrir workspace Operaciones')

    fireEvent.pointerDown(sourceGroup, { button: 0 })
    fireEvent.pointerMove(targetWorkspaceRow)
    fireEvent.pointerUp(targetWorkspaceRow)
    fireEvent.click(await screen.findByRole('button', { name: 'Mover' }))

    await waitFor(() => {
      expect(workspaceApi.updateGroup).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'group-frontend',
          parentGroupId: null,
          workspaceId: 'workspace-ops',
        }),
      )
      expect(workspaceApi.updateGroup).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'group-design',
          parentGroupId: 'group-frontend',
          workspaceId: 'workspace-ops',
        }),
      )
      expect(workspaceApi.updateProject).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'project-ui',
          groupId: 'group-frontend',
          workspaceId: 'workspace-ops',
        }),
      )
      expect(workspaceApi.updateProject).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'project-design',
          groupId: 'group-design',
          workspaceId: 'workspace-ops',
        }),
      )
    })
  })

  it('keeps the target stable while dragging with pointer events until release', async () => {
    render(<CentralitaApp />)

    await screen.findByRole('button', { name: 'Abrir grupo Frontend' })
    const sourceGroup = requireDragHandle('Arrastrar grupo Frontend')
    const targetWorkspaceRow = requireTreeRow('Abrir workspace Operaciones')

    fireEvent.pointerDown(sourceGroup, { button: 0 })
    fireEvent.pointerMove(targetWorkspaceRow)

    expect(targetWorkspaceRow).toHaveClass('navigator-drop-target-active')

    fireEvent.pointerUp(targetWorkspaceRow)

    expect(
      await screen.findByRole('dialog', { name: 'Mover grupo' }),
    ).toBeInTheDocument()
  })

  it('resynchronizes loaded trees after moving a group to another workspace', async () => {
    let currentMainTree: WorkspaceTree = workspaceTree
    let currentOpsTree: WorkspaceTree = workspaceOpsTree

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? currentOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.updateGroup).mockImplementation(async (input) => {
      const movedGroup = {
        ...currentMainTree.groups[0],
        workspaceId: input.workspaceId,
        parentGroupId: input.parentGroupId,
      }

      currentMainTree = {
        ...currentMainTree,
        groups: currentMainTree.groups.filter((group) => group.id !== input.id),
      }
      currentOpsTree = {
        ...currentOpsTree,
        groups: [
          ...currentOpsTree.groups,
          { ...movedGroup, groups: [], projects: [project] },
        ],
      }

      return movedGroup
    })

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Operaciones' }),
    )
    const sourceGroup = requireDragHandle('Arrastrar grupo Frontend')
    const targetWorkspaceRow = requireTreeRow('Abrir workspace Operaciones')

    fireEvent.pointerDown(sourceGroup, { button: 0 })
    fireEvent.pointerMove(targetWorkspaceRow)
    fireEvent.pointerUp(targetWorkspaceRow)
    fireEvent.click(await screen.findByRole('button', { name: 'Mover' }))

    await waitFor(() => {
      expect(workspaceApi.getWorkspaceTree).toHaveBeenCalledWith(
        'workspace-main',
      )
      expect(workspaceApi.getWorkspaceTree).toHaveBeenCalledWith(
        'workspace-ops',
      )
    })
    expect(
      await screen.findByRole('heading', { name: 'Frontend' }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Abrir workspace Operaciones' }),
    ).toBeInTheDocument()
  })

  it('expands the target parent group after moving another group inside it', async () => {
    let currentMainTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          projects: [],
        },
        {
          id: 'group-backend-local',
          workspaceId: 'workspace-main',
          parentGroupId: null,
          name: 'Backend',
          color: '#2563eb',
          sortOrder: 20,
          createdAt: '2026-04-15T09:00:00Z',
          updatedAt: '2026-04-15T09:00:00Z',
          groups: [],
          projects: [],
        },
        {
          id: 'group-qa',
          workspaceId: 'workspace-main',
          parentGroupId: null,
          name: 'QA',
          color: '#f59e0b',
          sortOrder: 30,
          createdAt: '2026-04-16T09:00:00Z',
          updatedAt: '2026-04-16T09:00:00Z',
          groups: [],
          projects: [],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.updateGroup).mockImplementation(async (input) => {
      const movedGroup = currentMainTree.groups.find(
        (group) => group.id === input.id,
      )
      const targetParent = currentMainTree.groups.find(
        (group) => group.id === input.parentGroupId,
      )

      if (!movedGroup || !targetParent) {
        throw new Error('Move fixture not found')
      }

      currentMainTree = {
        ...currentMainTree,
        groups: currentMainTree.groups
          .filter(
            (group) => group.id !== input.id && group.id !== targetParent.id,
          )
          .concat({
            ...targetParent,
            groups: [
              ...targetParent.groups,
              {
                ...movedGroup,
                parentGroupId: targetParent.id,
              },
            ],
          }),
      }

      return {
        ...movedGroup,
        parentGroupId: input.parentGroupId ?? null,
        workspaceId: input.workspaceId,
      }
    })

    render(<CentralitaApp />)

    await screen.findByRole('button', { name: 'Abrir grupo Backend' })
    expect(
      screen.queryByRole('button', { name: 'Abrir grupo QA' }),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'Expandir Backend' }),
    ).not.toBeInTheDocument()

    const sourceGroup = requireDragHandle('Arrastrar grupo QA')
    const targetGroupRow = requireTreeRow('Abrir grupo Backend')

    fireEvent.pointerDown(sourceGroup, { button: 0 })
    fireEvent.pointerMove(targetGroupRow)
    fireEvent.pointerUp(targetGroupRow)
    fireEvent.click(await screen.findByRole('button', { name: 'Mover' }))

    await waitFor(() => {
      expect(workspaceApi.updateGroup).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'group-qa',
          parentGroupId: 'group-backend-local',
          workspaceId: 'workspace-main',
        }),
      )
    })

    expect(
      await screen.findByRole('button', { name: 'Contraer Backend' }),
    ).toBeInTheDocument()
    expect(
      await screen.findByRole('heading', { name: 'QA' }),
    ).toBeInTheDocument()
  })

  it('does not swallow the first tree click after confirming a drag and drop move', async () => {
    let currentMainTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          projects: [],
        },
        {
          id: 'group-backend-local',
          workspaceId: 'workspace-main',
          parentGroupId: null,
          name: 'Backend',
          color: '#2563eb',
          sortOrder: 20,
          createdAt: '2026-04-15T09:00:00Z',
          updatedAt: '2026-04-15T09:00:00Z',
          groups: [],
          projects: [],
        },
        {
          id: 'group-qa',
          workspaceId: 'workspace-main',
          parentGroupId: null,
          name: 'QA',
          color: '#f59e0b',
          sortOrder: 30,
          createdAt: '2026-04-16T09:00:00Z',
          updatedAt: '2026-04-16T09:00:00Z',
          groups: [],
          projects: [],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : currentMainTree,
    )
    vi.mocked(workspaceApi.updateGroup).mockImplementation(async (input) => {
      const movedGroup = currentMainTree.groups.find(
        (group) => group.id === input.id,
      )
      const targetParent = currentMainTree.groups.find(
        (group) => group.id === input.parentGroupId,
      )

      if (!movedGroup || !targetParent) {
        throw new Error('Move fixture not found')
      }

      currentMainTree = {
        ...currentMainTree,
        groups: currentMainTree.groups
          .filter(
            (group) => group.id !== input.id && group.id !== targetParent.id,
          )
          .concat({
            ...targetParent,
            groups: [
              ...targetParent.groups,
              {
                ...movedGroup,
                parentGroupId: targetParent.id,
              },
            ],
          }),
      }

      return {
        ...movedGroup,
        parentGroupId: input.parentGroupId ?? null,
        workspaceId: input.workspaceId,
      }
    })

    render(<CentralitaApp />)

    await screen.findByRole('button', { name: 'Abrir grupo Backend' })
    const sourceGroup = requireDragHandle('Arrastrar grupo QA')
    const targetGroupRow = requireTreeRow('Abrir grupo Backend')

    fireEvent.pointerDown(sourceGroup, { button: 0 })
    fireEvent.pointerMove(targetGroupRow)
    fireEvent.pointerUp(targetGroupRow)
    fireEvent.click(await screen.findByRole('button', { name: 'Mover' }))

    const collapseTarget = await screen.findByRole('button', {
      name: 'Contraer Backend',
    })

    fireEvent.click(collapseTarget)

    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: 'Expandir Backend' }),
      ).toBeInTheDocument()
    })
    expect(
      screen.queryByRole('button', { name: 'Abrir grupo QA' }),
    ).not.toBeInTheDocument()
  })

  it('confirms moving a project to another group before updating it', async () => {
    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Operaciones' }),
    )
    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )

    await screen.findByRole('button', { name: 'Abrir proyecto centralita-ui' })
    const sourceProject = requireDragHandle('Arrastrar proyecto centralita-ui')
    const targetGroupRow = requireTreeRow('Abrir grupo Backend')

    fireEvent.pointerDown(sourceProject, { button: 0 })
    fireEvent.pointerMove(targetGroupRow)
    fireEvent.pointerUp(targetGroupRow)

    expect(
      await screen.findByRole('dialog', { name: 'Mover proyecto' }),
    ).toBeInTheDocument()
    expect(workspaceApi.updateProject).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole('button', { name: 'Mover' }))

    await waitFor(() => {
      expect(workspaceApi.updateProject).toHaveBeenCalledWith(
        expect.objectContaining({
          groupId: 'group-backend',
          id: 'project-ui',
          workspaceId: 'workspace-ops',
        }),
      )
    })
  })

  it('shows an explicit root dropzone when dragging a subgroup to the workspace root', async () => {
    const nestedTree: WorkspaceTree = {
      workspace,
      groups: [
        {
          ...workspaceTree.groups[0],
          groups: [
            {
              id: 'group-design',
              workspaceId: 'workspace-main',
              parentGroupId: 'group-frontend',
              name: 'Design',
              color: '#f59e0b',
              sortOrder: 15,
              createdAt: '2026-04-17T08:00:00Z',
              updatedAt: '2026-04-17T08:00:00Z',
              groups: [],
              projects: [],
            },
          ],
        },
      ],
    }

    vi.mocked(workspaceApi.getWorkspaceTree).mockImplementation(
      async (workspaceId: string) =>
        workspaceId === 'workspace-ops' ? workspaceOpsTree : nestedTree,
    )

    render(<CentralitaApp />)

    fireEvent.click(
      await screen.findByRole('button', { name: 'Expandir Frontend' }),
    )
    await screen.findByRole('button', { name: 'Abrir grupo Design' })
    const subgroup = requireDragHandle('Arrastrar grupo Design')

    fireEvent.pointerDown(subgroup, { button: 0 })

    const rootDropzone = await screen.findByRole('group', {
      name: 'Soltar en la raíz de Centralita',
    })

    fireEvent.pointerMove(rootDropzone)
    fireEvent.pointerUp(rootDropzone)

    expect(
      await screen.findByRole('dialog', { name: 'Mover grupo' }),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Mover' }))

    await waitFor(() => {
      expect(workspaceApi.updateGroup).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'group-design',
          parentGroupId: null,
          workspaceId: 'workspace-main',
        }),
      )
    })
  })
})
