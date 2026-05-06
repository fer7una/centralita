# Architecture

Centralita is a portable Windows desktop monolith built with React, TypeScript,
Vite, Rust, and Tauri 2.

## Runtime Layers

The expected dependency flow is:

```text
React UI
  -> frontend application and feature logic
  -> shared/api/tauri typed IPC client
  -> Tauri commands
  -> Rust use cases and runtime services
  -> Rust domain models
  -> Rust infrastructure adapters
```

React code must not call Tauri infrastructure APIs directly from components or
stores. Frontend command and event access belongs in `src/shared/api/tauri`.

## Frontend Map

- `src/CentralitaApp.tsx`: current application composition shell. This file is
  intentionally a migration candidate and should be reduced incrementally.
- `src/features/`: feature-level frontend logic and API facades.
- `src/components/`: existing reusable UI blocks. New broad UI blocks should
  move toward `widgets/` or `shared/ui/` when introduced.
- `src/store/`: current application hooks for workspace and runtime state.
- `src/shared/api/tauri/`: the only frontend boundary that imports Tauri IPC
  APIs directly.
- `src/shared/types/`: shared frontend type export surface during the migration.
- `src/types/`: current DTO and domain-shaped TypeScript contracts.

## IPC Rules

- Raw command names are defined in `src/shared/api/tauri/commands.ts`.
- Runtime event names are defined in `src/shared/api/tauri/events.ts`.
- Feature APIs call `invokeCommand`, not `@tauri-apps/api/core` directly.
- Stores and components subscribe through typed event wrappers, not
  `@tauri-apps/api/event` directly.
- IPC DTO changes must update TypeScript contracts and Rust command DTOs in the
  same task.

## Incremental Migration Rules

- Do not split `CentralitaApp.tsx` and move Rust architecture in the same task.
- Do not create parallel IPC clients.
- Preserve Tauri command names unless a coordinated IPC migration requires a
  breaking change.
- Keep behavior changes separate from structural moves where practical.
