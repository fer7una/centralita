import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { RuntimeLogLine } from '../../types'
import { ProjectLogsPanel } from './ProjectLogsPanel'
import { countLogLinesInMemory, prepareTerminalLogText } from './logEntries'

function logLine(
  line: string,
  timestamp = '2026-04-16T07:30:00Z',
  partial = false,
): RuntimeLogLine {
  return {
    projectId: 'project-ui',
    stream: 'stderr',
    line,
    partial,
    timestamp,
  }
}

describe('ProjectLogsPanel', () => {
  it('preserves blank terminal rows when counting memory lines', () => {
    expect(countLogLinesInMemory([logLine('first'), logLine('   '), logLine('\t')])).toBe(3)
  })

  it('strips terminal control sequences before rendering', () => {
    render(<ProjectLogsPanel lines={[logLine('\u001B[31merror from vite\u001B[39m')]} />)

    expect(screen.getByText('error from vite')).toBeInTheDocument()
    expect(
      screen.queryByText((content) => content.includes(String.fromCharCode(27))),
    ).not.toBeInTheDocument()
  })

  it('renders stack trace output as one terminal buffer', () => {
    const terminalText = prepareTerminalLogText([
      logLine('Error: boot failed'),
      logLine('    at startServer (server.ts:10:1)', '2026-04-16T07:30:01Z'),
      logLine('Caused by: missing env', '2026-04-16T07:30:02Z'),
    ])

    expect(terminalText).toBe(
      [
        'Error: boot failed',
        '    at startServer (server.ts:10:1)',
        'Caused by: missing env',
      ].join('\n'),
    )
  })

  it('renders partial runtime chunks without adding artificial line breaks', () => {
    const terminalText = prepareTerminalLogText([
      logLine('Starting ', '2026-04-16T07:30:00Z', true),
      logLine('Vite...', '2026-04-16T07:30:01Z', true),
    ])

    expect(terminalText).toBe('Starting Vite...')
  })

  it('keeps the latest carriage-return terminal frame', () => {
    const terminalText = prepareTerminalLogText([
      logLine('Progress 10%\rProgress 40%\rProgress 100%\n', '2026-04-16T07:30:00Z', true),
    ])

    expect(terminalText).toBe('Progress 100%\n')
  })

  it('preserves unicode output while applying terminal formatting', () => {
    const terminalText = prepareTerminalLogText([
      logLine('😀a é ñ 日本語 عربى ✓', '2026-04-16T07:30:00Z', true),
    ])

    expect(terminalText).toBe('😀a é ñ 日本語 عربى ✓')
  })

  it('scrolls to the latest terminal output after new lines render', async () => {
    const { rerender } = render(<ProjectLogsPanel lines={[logLine('first')]} />)
    const logConsole = screen.getByRole('log', { name: 'Logs de terminal' })

    Object.defineProperty(logConsole, 'scrollHeight', {
      configurable: true,
      value: 720,
    })

    rerender(
      <ProjectLogsPanel
        lines={[logLine('first'), logLine('second', '2026-04-16T07:30:01Z')]}
      />,
    )

    await waitFor(() => expect(logConsole.scrollTop).toBe(720))
  })

  it('opens terminal search with Ctrl+F and searches the whole terminal buffer', async () => {
    render(
      <ProjectLogsPanel
        lines={[
          logLine('Boot ready'),
          logLine('Error from dev server', '2026-04-16T07:30:01Z'),
          logLine('Later error from watcher', '2026-04-16T07:30:02Z'),
        ]}
      />,
    )

    fireEvent.keyDown(window, { ctrlKey: true, key: 'f' })
    const searchInput = screen.getByRole('searchbox', {
      name: 'Buscar en logs de terminal',
    })

    await waitFor(() => expect(searchInput).toHaveFocus())

    fireEvent.change(searchInput, { target: { value: 'error' } })

    expect(screen.getByText('1/2')).toBeInTheDocument()
    expect(document.querySelectorAll('.log-search-match')).toHaveLength(2)

    fireEvent.click(screen.getByLabelText('Coincidencia siguiente'))

    expect(screen.getByText('2/2')).toBeInTheDocument()
  })

  it('searches terminal output by unicode characters', () => {
    render(
      <ProjectLogsPanel
        lines={[
          logLine(
            '\u00bf\u00e1\u00e9\u00ed\u00f3\u00fa\u00f1',
            '2026-04-16T07:30:00Z',
          ),
        ]}
      />,
    )

    fireEvent.click(screen.getByLabelText('Buscar en logs de terminal'))
    fireEvent.change(
      screen.getByRole('searchbox', { name: 'Buscar en logs de terminal' }),
      { target: { value: '\u00f1' } },
    )

    expect(screen.getByText('1/1')).toBeInTheDocument()
    expect(document.querySelectorAll('.log-search-match')).toHaveLength(1)
  })
})
