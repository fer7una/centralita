import { useEffect, useState } from 'react'
import { RunHistoryList } from '../history/RunHistoryList'
import { ProjectLogsPanel } from '../logs/ProjectLogsPanel'
import { HealthStatusBadge } from '../health/HealthStatusBadge'
import { RuntimeStatusBadge } from './RuntimeStatusBadge'
import type {
  HealthCheckConfig,
  ProjectHealthState,
  ProcessRuntimeState,
  ProjectNode,
  RunHistoryEntry,
  RuntimeLogLine,
} from '../../types'

type ProjectRuntimePanelProps = {
  healthState: ProjectHealthState | null
  logs: RuntimeLogLine[]
  onClearLogs: () => void
  onRefreshHealth: () => void | Promise<unknown>
  onRestart: () => void | Promise<unknown>
  onSaveHealthCheck: (
    healthCheck: HealthCheckConfig | null,
  ) => void | Promise<unknown>
  onStart: () => void | Promise<unknown>
  onStop: () => void | Promise<unknown>
  project: ProjectNode | null
  runHistory: RunHistoryEntry[]
  runtimeState: ProcessRuntimeState | null
}

type HealthCheckDraft = {
  containsText: string
  enabled: boolean
  expectedStatusCodes: string
  failureThreshold: number
  gracePeriodMs: number
  host: string
  intervalMs: number
  method: string
  port: number
  successThreshold: number
  timeoutMs: number
  type: 'http' | 'tcp'
  url: string
}

function createHealthCheckDraft(project: ProjectNode | null): HealthCheckDraft {
  const healthCheck = project?.healthCheck
  if (!healthCheck) {
    return {
      containsText: '',
      enabled: false,
      expectedStatusCodes: '200',
      failureThreshold: 2,
      gracePeriodMs: 3_000,
      host: '127.0.0.1',
      intervalMs: 5_000,
      method: 'GET',
      port: 3000,
      successThreshold: 1,
      timeoutMs: 2_000,
      type: 'http',
      url: '',
    }
  }

  if (healthCheck.type === 'http') {
    return {
      containsText: healthCheck.containsText ?? '',
      enabled: healthCheck.enabled,
      expectedStatusCodes: healthCheck.expectedStatusCodes.join(', ') || '200',
      failureThreshold: healthCheck.failureThreshold,
      gracePeriodMs: healthCheck.gracePeriodMs,
      host: '127.0.0.1',
      intervalMs: healthCheck.intervalMs,
      method: healthCheck.method,
      port: 3000,
      successThreshold: healthCheck.successThreshold,
      timeoutMs: healthCheck.timeoutMs,
      type: 'http',
      url: healthCheck.url,
    }
  }

  return {
    containsText: '',
    enabled: healthCheck.enabled,
    expectedStatusCodes: '200',
    failureThreshold: healthCheck.failureThreshold,
    gracePeriodMs: healthCheck.gracePeriodMs,
    host: healthCheck.host,
    intervalMs: healthCheck.intervalMs,
    method: 'GET',
    port: healthCheck.port,
    successThreshold: healthCheck.successThreshold,
    timeoutMs: healthCheck.timeoutMs,
    type: 'tcp',
    url: '',
  }
}

function buildHealthCheck(draft: HealthCheckDraft): HealthCheckConfig | null {
  if (!draft.enabled) {
    return null
  }

  if (draft.type === 'http') {
    return {
      type: 'http',
      enabled: true,
      intervalMs: Number(draft.intervalMs),
      timeoutMs: Number(draft.timeoutMs),
      gracePeriodMs: Number(draft.gracePeriodMs),
      successThreshold: Number(draft.successThreshold),
      failureThreshold: Number(draft.failureThreshold),
      url: draft.url.trim(),
      method: draft.method.trim() || 'GET',
      expectedStatusCodes: draft.expectedStatusCodes
        .split(',')
        .map((value) => Number(value.trim()))
        .filter((value) => Number.isFinite(value) && value > 0),
      containsText: draft.containsText.trim() || null,
      headers: null,
    }
  }

  return {
    type: 'tcp',
    enabled: true,
    intervalMs: Number(draft.intervalMs),
    timeoutMs: Number(draft.timeoutMs),
    gracePeriodMs: Number(draft.gracePeriodMs),
    successThreshold: Number(draft.successThreshold),
    failureThreshold: Number(draft.failureThreshold),
    host: draft.host.trim(),
    port: Number(draft.port),
  }
}

export function ProjectRuntimePanel({
  healthState,
  logs,
  onClearLogs,
  onRefreshHealth,
  onRestart,
  onSaveHealthCheck,
  onStart,
  onStop,
  project,
  runHistory,
  runtimeState,
}: ProjectRuntimePanelProps) {
  const [draft, setDraft] = useState<HealthCheckDraft>(() =>
    createHealthCheckDraft(project),
  )

  useEffect(() => {
    setDraft(createHealthCheckDraft(project))
  }, [project])

  if (!project) {
    return (
      <section className="card runtime-panel">
        <div className="section-title">
          <h3>Detalle runtime</h3>
        </div>
        <p className="muted">
          Selecciona un proyecto del árbol para ver runtime e historial.
        </p>
      </section>
    )
  }

  const supportsHealth = Boolean(project.healthCheck?.enabled)

  return (
    <section className="card runtime-panel">
      <div className="section-title">
        <div>
          <h3>{project.name}</h3>
          <p>{project.path}</p>
        </div>
        <div className="project-badges">
          <RuntimeStatusBadge status={runtimeState?.status ?? 'STOPPED'} />
          {supportsHealth ? (
            <HealthStatusBadge status={healthState?.status ?? 'UNKNOWN'} />
          ) : null}
        </div>
      </div>

      <div className="hero-actions">
        <button onClick={() => void onStart()} type="button">
          Start
        </button>
        <button onClick={() => void onStop()} type="button">
          Stop
        </button>
        <button onClick={() => void onRestart()} type="button">
          Restart
        </button>
        {supportsHealth ? (
          <button onClick={() => void onRefreshHealth()} type="button">
            Refrescar health
          </button>
        ) : null}
        <button onClick={onClearLogs} type="button">
          Limpiar vista
        </button>
      </div>

      <div className="runtime-grid">
        <article className="info-card">
          <h4>Estado</h4>
          <ul className="detail-list">
            <li>
              <strong>PID</strong>
              <span>{runtimeState?.pid ?? 'Sin proceso activo'}</span>
            </li>
            <li>
              <strong>Comando</strong>
              <span>
                {runtimeState?.commandPreview ??
                  project.command ??
                  'Sin comando derivado'}
              </span>
            </li>
            <li>
              <strong>Working dir</strong>
              <span>{project.workingDir ?? project.path}</span>
            </li>
            <li>
              <strong>Último error runtime</strong>
              <span>
                {runtimeState?.lastError ?? 'Sin errores registrados'}
              </span>
            </li>
            {supportsHealth ? (
              <>
                <li>
                  <strong>Último health error</strong>
                  <span>
                    {healthState?.lastError ?? 'Sin errores registrados'}
                  </span>
                </li>
                <li>
                  <strong>Última comprobación</strong>
                  <span>
                    {healthState?.lastCheckedAt
                      ? new Date(healthState.lastCheckedAt).toLocaleString()
                      : 'Sin comprobaciones'}
                  </span>
                </li>
                <li>
                  <strong>Último healthy</strong>
                  <span>
                    {healthState?.lastHealthyAt
                      ? new Date(healthState.lastHealthyAt).toLocaleString()
                      : 'Todavía no sano'}
                  </span>
                </li>
              </>
            ) : null}
          </ul>
        </article>

        <article className="info-card full-width">
          <div className="section-title">
            <h4>Terminal integrada</h4>
          </div>
          <ProjectLogsPanel lines={logs} />
        </article>

        {supportsHealth ? (
          <article className="info-card">
            <div className="section-title">
              <h4>Health check</h4>
              <button
                onClick={() => void onSaveHealthCheck(buildHealthCheck(draft))}
                type="button"
              >
                Guardar
              </button>
            </div>
            <div className="stack">
              <label className="field checkbox-field">
                <span>Habilitado</span>
                <input
                  checked={draft.enabled}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      enabled: event.target.checked,
                    }))
                  }
                  type="checkbox"
                />
              </label>

            <label className="field">
              <span>Tipo</span>
              <select
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    type: event.target.value as 'http' | 'tcp',
                  }))
                }
                value={draft.type}
              >
                <option value="http">HTTP</option>
                <option value="tcp">TCP</option>
              </select>
            </label>

            {draft.type === 'http' ? (
              <>
                <label className="field">
                  <span>URL</span>
                  <input
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        url: event.target.value,
                      }))
                    }
                    placeholder="http://127.0.0.1:3000/health"
                    value={draft.url}
                  />
                </label>
                <label className="field">
                  <span>Metodo</span>
                  <input
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        method: event.target.value,
                      }))
                    }
                    value={draft.method}
                  />
                </label>
                <label className="field">
                  <span>Status esperados</span>
                  <input
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        expectedStatusCodes: event.target.value,
                      }))
                    }
                    placeholder="200, 204"
                    value={draft.expectedStatusCodes}
                  />
                </label>
                <label className="field">
                  <span>Texto esperado</span>
                  <input
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        containsText: event.target.value,
                      }))
                    }
                    placeholder="ok"
                    value={draft.containsText}
                  />
                </label>
              </>
            ) : (
              <>
                <label className="field">
                  <span>Host</span>
                  <input
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        host: event.target.value,
                      }))
                    }
                    value={draft.host}
                  />
                </label>
                <label className="field">
                  <span>Puerto</span>
                  <input
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        port: Number(event.target.value) || 0,
                      }))
                    }
                    type="number"
                    value={draft.port}
                  />
                </label>
              </>
            )}

            <div className="observability-grid">
              <label className="field">
                <span>Intervalo ms</span>
                <input
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      intervalMs: Number(event.target.value) || 0,
                    }))
                  }
                  type="number"
                  value={draft.intervalMs}
                />
              </label>
              <label className="field">
                <span>Timeout ms</span>
                <input
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      timeoutMs: Number(event.target.value) || 0,
                    }))
                  }
                  type="number"
                  value={draft.timeoutMs}
                />
              </label>
              <label className="field">
                <span>Grace period ms</span>
                <input
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      gracePeriodMs: Number(event.target.value) || 0,
                    }))
                  }
                  type="number"
                  value={draft.gracePeriodMs}
                />
              </label>
              <label className="field">
                <span>Threshold OK</span>
                <input
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      successThreshold: Number(event.target.value) || 1,
                    }))
                  }
                  min={1}
                  type="number"
                  value={draft.successThreshold}
                />
              </label>
              <label className="field">
                <span>Threshold fail</span>
                <input
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      failureThreshold: Number(event.target.value) || 1,
                    }))
                  }
                  min={1}
                  type="number"
                  value={draft.failureThreshold}
                />
              </label>
            </div>
            </div>
          </article>
        ) : null}

        <article className="info-card full-width">
          <div className="section-title">
            <h4>Historial reciente</h4>
            <p>{runHistory.length} ejecuciones</p>
          </div>
          <RunHistoryList entries={runHistory} />
        </article>
      </div>
    </section>
  )
}
