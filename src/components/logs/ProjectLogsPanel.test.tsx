import { render, screen } from '@testing-library/react'
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
})
