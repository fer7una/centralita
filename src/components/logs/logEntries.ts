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
  const rows: string[][] = [[]]
  let rowIndex = 0
  let columnIndex = 0

  for (const character of value) {
    const code = character.codePointAt(0)

    if (code === undefined) {
      continue
    }

    if (code === 10) {
      rowIndex += 1
      rows[rowIndex] = rows[rowIndex] ?? []
      columnIndex = 0
      continue
    }

    if (code === 13) {
      columnIndex = 0
      continue
    }

    if (code === 8) {
      if (columnIndex > 0) {
        const currentRow = rows[rowIndex] ?? []
        rows[rowIndex] = currentRow
        currentRow.splice(columnIndex - 1, 1)
        columnIndex -= 1
      }
      continue
    }

    if (code === 9 || code >= 32) {
      const currentRow = rows[rowIndex] ?? []
      rows[rowIndex] = currentRow

      while (currentRow.length < columnIndex) {
        currentRow.push(' ')
      }

      currentRow[columnIndex] = character
      columnIndex += 1
    }
  }

  return rows.map((row) => row.join('')).join('\n')
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

export function prepareTerminalLogLines(
  lines: RuntimeLogLine[],
): PreparedLogLine[] {
  const terminalText = applyTerminalControlCharacters(
    stripTerminalSequences(toTerminalText(lines)),
  )

  if (terminalText.length === 0) {
    return []
  }

  return terminalText.split('\n').map((line) => ({ line }))
}

export function countLogLinesInMemory(lines: RuntimeLogLine[]) {
  return prepareTerminalLogLines(lines).length
}

export function prepareTerminalLogText(lines: RuntimeLogLine[]) {
  return prepareTerminalLogLines(lines)
    .map((line) => line.line)
    .join('\n')
}
