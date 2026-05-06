import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ProjectHealthState,
  RunHistoryEntry,
  RuntimeLogLine,
  RuntimeProcessErrorEvent,
  RuntimeProcessExitedEvent,
  RuntimeStatusEvent,
} from '../../types'

export const RUNTIME_EVENTS = {
  healthChanged: 'runtime://health-changed',
  historyAppended: 'runtime://history-appended',
  logLine: 'runtime://log-line',
  processError: 'runtime://process-error',
  processExited: 'runtime://process-exited',
  statusChanged: 'runtime://status-changed',
} as const

type RuntimeEventMap = {
  [RUNTIME_EVENTS.healthChanged]: ProjectHealthState
  [RUNTIME_EVENTS.historyAppended]: RunHistoryEntry
  [RUNTIME_EVENTS.logLine]: RuntimeLogLine
  [RUNTIME_EVENTS.processError]: RuntimeProcessErrorEvent
  [RUNTIME_EVENTS.processExited]: RuntimeProcessExitedEvent
  [RUNTIME_EVENTS.statusChanged]: RuntimeStatusEvent
}

export type RuntimeEventName = keyof RuntimeEventMap

export function listenRuntimeEvent<EventName extends RuntimeEventName>(
  eventName: EventName,
  handler: (payload: RuntimeEventMap[EventName]) => void,
): Promise<UnlistenFn> {
  return tauriListen<RuntimeEventMap[EventName]>(eventName, (event) => {
    handler(event.payload)
  })
}
