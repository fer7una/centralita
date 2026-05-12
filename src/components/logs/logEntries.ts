import type { RuntimeLogLine } from '../../types'

type PreparedLogLine = {
  line: string
}

function stripTerminalSequences(value: string) {
  let output = ''

  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index)
    if (code === 27 || code === 155) {
      index = skipTerminalSequence(value, index)
      continue
    }

    output += value[index]
  }

  return output
}

function skipTerminalSequence(value: string, startIndex: number) {
  const code = value.charCodeAt(startIndex)
  let index = startIndex

  if (code === 27) {
    index += 1
    const marker = value[index]

    if (marker === '[') {
      return skipUntilFinalByte(value, index + 1)
    }

    if (marker === ']') {
      return skipOperatingSystemCommand(value, index + 1)
    }

    return Math.min(index, value.length - 1)
  }

  return skipUntilFinalByte(value, index + 1)
}

function skipUntilFinalByte(value: string, startIndex: number) {
  for (let index = startIndex; index < value.length; index += 1) {
    const code = value.charCodeAt(index)
    if (code >= 64 && code <= 126) {
      return index
    }
  }

  return value.length - 1
}

function skipOperatingSystemCommand(value: string, startIndex: number) {
  for (let index = startIndex; index < value.length; index += 1) {
    const code = value.charCodeAt(index)
    if (code === 7) {
      return index
    }

    if (code === 27 && value[index + 1] === '\\') {
      return index + 1
    }
  }

  return value.length - 1
}

function applyTerminalControlCharacters(value: string) {
  const rows = ['']
  let rowIndex = 0
  let columnIndex = 0

  for (const character of value) {
    const code = character.charCodeAt(0)

    if (code === 10) {
      rowIndex += 1
      rows[rowIndex] = rows[rowIndex] ?? ''
      columnIndex = 0
      continue
    }

    if (code === 13) {
      columnIndex = 0
      continue
    }

    if (code === 8) {
      if (columnIndex > 0) {
        const currentRow = rows[rowIndex] ?? ''
        rows[rowIndex] =
          currentRow.slice(0, columnIndex - 1) + currentRow.slice(columnIndex)
        columnIndex -= 1
      }
      continue
    }

    if (code === 9 || code >= 32) {
      const currentRow = rows[rowIndex] ?? ''
      rows[rowIndex] =
        currentRow.length > columnIndex
          ? `${currentRow.slice(0, columnIndex)}${character}${currentRow.slice(
              columnIndex + 1,
            )}`
          : `${currentRow.padEnd(columnIndex, ' ')}${character}`
      columnIndex += 1
    }
  }

  return rows.join('\n')
}

function toTerminalText(lines: RuntimeLogLine[]) {
  return lines.reduce((output, entry) => {
    if (entry.partial) {
      return output + entry.line
    }

    const separator =
      output.length > 0 && !output.endsWith('\n') && !output.endsWith('\r')
        ? '\n'
        : ''

    return `${output}${separator}${entry.line}`
  }, '')
}

function toPreparedLogLines(lines: RuntimeLogLine[]): PreparedLogLine[] {
  const terminalText = applyTerminalControlCharacters(
    stripTerminalSequences(toTerminalText(lines)),
  )

  if (terminalText.length === 0) {
    return []
  }

  return terminalText.split('\n').map((line) => ({ line }))
}

export function countLogLinesInMemory(lines: RuntimeLogLine[]) {
  return toPreparedLogLines(lines).length
}

export function prepareTerminalLogText(lines: RuntimeLogLine[]) {
  return toPreparedLogLines(lines)
    .map((line) => line.line)
    .join('\n')
}
