import { ChevronDown, ChevronUp, Search, X } from 'lucide-react'
import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import type { RuntimeLogLine } from '../../types'
import { prepareTerminalLogLines } from './logEntries'

type ProjectLogsPanelProps = {
  lines: RuntimeLogLine[]
}

type TerminalSearchMatch = {
  end: number
  index: number
  start: number
}

type TerminalSearchLine = {
  line: string
  matches: TerminalSearchMatch[]
}

type TerminalSearchResult = {
  lines: TerminalSearchLine[]
  totalMatches: number
}

export function ProjectLogsPanel({ lines }: ProjectLogsPanelProps) {
  const logConsoleRef = useRef<HTMLDivElement>(null)
  const searchInputRef = useRef<HTMLInputElement>(null)
  const terminalLines = useMemo(() => prepareTerminalLogLines(lines), [lines])
  const [isSearchOpen, setIsSearchOpen] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [requestedMatchIndex, setRequestedMatchIndex] = useState(0)
  const searchResult = useMemo(
    () => findTerminalSearchMatches(terminalLines, searchQuery),
    [terminalLines, searchQuery],
  )
  const activeMatchIndex =
    searchResult.totalMatches > 0
      ? Math.min(requestedMatchIndex, searchResult.totalMatches - 1)
      : -1
  const searchCounter =
    searchResult.totalMatches > 0
      ? `${activeMatchIndex + 1}/${searchResult.totalMatches}`
      : '0/0'

  useEffect(() => {
    const openSearch = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'f') {
        return
      }

      event.preventDefault()
      setIsSearchOpen(true)
    }

    window.addEventListener('keydown', openSearch)

    return () => window.removeEventListener('keydown', openSearch)
  }, [])

  useLayoutEffect(() => {
    if (!isSearchOpen) {
      return
    }

    searchInputRef.current?.focus()
    searchInputRef.current?.select()
  }, [isSearchOpen])

  useLayoutEffect(() => {
    const logConsole = logConsoleRef.current

    if (!logConsole || searchQuery.length > 0) {
      return
    }

    logConsole.scrollTop = logConsole.scrollHeight
  }, [searchQuery, terminalLines])

  useLayoutEffect(() => {
    if (activeMatchIndex < 0) {
      return
    }

    const activeMatch = logConsoleRef.current?.querySelector(
      '[data-terminal-search-active="true"]',
    )

    if (
      activeMatch instanceof HTMLElement &&
      typeof activeMatch.scrollIntoView === 'function'
    ) {
      activeMatch.scrollIntoView({ block: 'center', inline: 'nearest' })
    }
  }, [activeMatchIndex, searchQuery, searchResult.totalMatches])

  const selectNextMatch = () => {
    setRequestedMatchIndex((currentIndex) =>
      searchResult.totalMatches > 0
        ? (Math.min(currentIndex, searchResult.totalMatches - 1) + 1) %
          searchResult.totalMatches
        : 0,
    )
  }

  const selectPreviousMatch = () => {
    setRequestedMatchIndex((currentIndex) =>
      searchResult.totalMatches > 0
        ? (Math.min(currentIndex, searchResult.totalMatches - 1) -
            1 +
            searchResult.totalMatches) %
          searchResult.totalMatches
        : 0,
    )
  }

  const closeSearch = () => {
    setIsSearchOpen(false)
    setSearchQuery('')
    setRequestedMatchIndex(0)
  }

  return (
    <div className="log-terminal">
      <div className="log-search-control">
        {isSearchOpen ? (
          <div className="log-search-box">
            <Search aria-hidden="true" size={15} />
            <input
              aria-label="Buscar en logs de terminal"
              onChange={(event) => {
                setSearchQuery(event.target.value)
                setRequestedMatchIndex(0)
              }}
              onKeyDown={(event) => {
                if (event.key === 'Escape') {
                  closeSearch()
                  return
                }

                if (event.key === 'Enter') {
                  event.preventDefault()
                  if (event.shiftKey) {
                    selectPreviousMatch()
                  } else {
                    selectNextMatch()
                  }
                }
              }}
              placeholder="Buscar"
              ref={searchInputRef}
              type="search"
              value={searchQuery}
            />
            <span aria-live="polite" className="log-search-count">
              {searchCounter}
            </span>
            <button
              aria-label="Coincidencia anterior"
              disabled={searchResult.totalMatches === 0}
              onClick={selectPreviousMatch}
              type="button"
            >
              <ChevronUp aria-hidden="true" size={15} />
            </button>
            <button
              aria-label="Coincidencia siguiente"
              disabled={searchResult.totalMatches === 0}
              onClick={selectNextMatch}
              type="button"
            >
              <ChevronDown aria-hidden="true" size={15} />
            </button>
            <button
              aria-label="Cerrar búsqueda"
              onClick={closeSearch}
              type="button"
            >
              <X aria-hidden="true" size={15} />
            </button>
          </div>
        ) : (
          <button
            aria-label="Buscar en logs de terminal"
            className="log-search-toggle"
            onClick={() => setIsSearchOpen(true)}
            type="button"
          >
            <Search aria-hidden="true" size={16} />
          </button>
        )}
      </div>

      <div
        aria-label="Logs de terminal"
        className="log-console"
        ref={logConsoleRef}
        role="log"
      >
        {searchResult.lines.length === 0 ? (
          <span>Todavía no hay logs.</span>
        ) : (
          searchResult.lines.map((line, index) => (
            <div key={index}>
              <span>
                {renderTerminalSearchLine(line, activeMatchIndex)}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

function findTerminalSearchMatches(
  terminalLines: { line: string }[],
  searchQuery: string,
): TerminalSearchResult {
  if (searchQuery.length === 0) {
    return {
      lines: terminalLines.map((line) => ({ line: line.line, matches: [] })),
      totalMatches: 0,
    }
  }

  const normalizedQuery = searchQuery.toLocaleLowerCase()
  let totalMatches = 0

  const searchLines = terminalLines.map((line) => {
    const normalizedLine = line.line.toLocaleLowerCase()
    const matches: TerminalSearchMatch[] = []
    let startIndex = 0

    while (startIndex <= normalizedLine.length) {
      const matchIndex = normalizedLine.indexOf(normalizedQuery, startIndex)

      if (matchIndex === -1) {
        break
      }

      matches.push({
        end: matchIndex + searchQuery.length,
        index: totalMatches,
        start: matchIndex,
      })
      totalMatches += 1
      startIndex = matchIndex + Math.max(searchQuery.length, 1)
    }

    return {
      line: line.line,
      matches,
    }
  })

  return {
    lines: searchLines,
    totalMatches,
  }
}

function renderTerminalSearchLine(
  searchLine: TerminalSearchLine,
  activeMatchIndex: number,
): ReactNode {
  if (searchLine.matches.length === 0) {
    return searchLine.line
  }

  const segments: ReactNode[] = []
  let currentIndex = 0

  for (const match of searchLine.matches) {
    if (match.start > currentIndex) {
      segments.push(searchLine.line.slice(currentIndex, match.start))
    }

    const isActive = match.index === activeMatchIndex
    segments.push(
      <mark
        className={`log-search-match${isActive ? ' is-active' : ''}`}
        data-terminal-search-active={isActive ? 'true' : undefined}
        key={`match-${match.index}`}
      >
        {searchLine.line.slice(match.start, match.end)}
      </mark>,
    )
    currentIndex = match.end
  }

  if (currentIndex < searchLine.line.length) {
    segments.push(searchLine.line.slice(currentIndex))
  }

  return segments
}
