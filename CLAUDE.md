# CLAUDE.md

## Alcance

Estas instrucciones aplican a cualquier agente que trabaje en Centralita. Si hay
conflicto, seguir primero `AGENTS.md` y despues este archivo.

Centralita es una aplicacion de escritorio portable para Windows basada en
Tauri 2, React 19, TypeScript, Vite, Rust, ESLint, Prettier y Vitest.

## Prioridad de arquitectura

Mantener esta direccion de dependencias:

```text
React UI
  -> frontend application layer
  -> typed Tauri IPC client
  -> Tauri commands
  -> Rust application services / use cases
  -> Rust domain
  -> Rust infrastructure
```

Reglas obligatorias:

- React no conoce SQLite, filesystem, procesos, OS APIs ni infraestructura Rust.
- El dominio Rust no conoce Tauri.
- Los comandos Tauri son adaptadores finos, no contenedores de negocio.
- Infraestructura implementa detalles externos; no dirige reglas de dominio.
- El contrato React/Rust pasa por IPC tipado y revisable.

## Frontend

- Evolucionar de forma incremental hacia Feature-Sliced Design.
- Respetar direccion de capas: `app`, `pages`, `widgets`, `features`,
  `entities`, `shared`.
- `shared` no importa capas superiores.
- Componentes y stores no deben importar `@tauri-apps/api/core` ni
  `@tauri-apps/api/event`.
- Usar `src/shared/api/tauri/commands.ts` para nombres de comandos.
- Usar `src/shared/api/tauri/events.ts` para nombres de eventos.
- Las APIs de feature deben exponer funciones de dominio del frontend, no raw
  `invoke`.
- No convertir `shared/lib` ni `utils` en cajones genericos.
- Mantener componentes pequenos y separados por responsabilidad.
- `src/CentralitaApp.tsx` puede reducirse gradualmente, pero no mezclar esa
  migracion con cambios Rust, persistencia o tooling salvo necesidad real.

## Rust, Tauri y persistencia

- Mantener `src-tauri/src` como shell/adaptador Tauri.
- Mover reglas de negocio a servicios/casos de uso o dominio cuando aparezcan.
- Evitar tipos Tauri en dominio/aplicacion.
- La persistencia local actual es SQLite con `rusqlite`.
- El archivo de datos es `centralita.sqlite3` bajo `app_data_dir()` de Tauri.
- No cambiar SQLite, rutas de datos, permisos Tauri ni migraciones sin tarea
  explicita y documentacion del impacto.
- Revisar integridad, idempotencia y datos existentes antes de editar
  migraciones.

## Seguridad y permisos

- No exponer filesystem, shell, HTTP ni OS powers al frontend sin comando
  controlado.
- Mantener capacidades Tauri minimas y explicitas.
- No registrar secretos, tokens, credenciales, rutas sensibles innecesarias ni
  datos personales.
- No introducir bypasses de autenticacion/autorizacion, telemetria oculta,
  persistencia oculta ni comportamiento condicionado por usuarios/fechas
  secretas.

## Validaciones

Para cambios frontend o tooling:

- `npm run lint`
- `npm run typecheck`
- `npm run test:run`
- `npm run build`

Para integracion Tauri:

- `npm run tauri:dev`

Para Rust:

- `cargo test`

Si una validacion no puede ejecutarse, indicar motivo y riesgo residual.

## Documentacion

- Actualizar `docs/ARCHITECTURE.md` cuando cambien capas, fronteras, IPC,
  persistencia o permisos.
- Mantener README orientado a instalacion, ejecucion, test y build.
- Las decisiones repetibles pertenecen en `AGENTS.md`, `CLAUDE.md` o `docs/`.

## Entrega

La respuesta final debe incluir:

- `Resumen:`
- `Archivos modificados:`
- `Validaciones ejecutadas:`
- `Incidencias:`

Si no hubo incidencias, escribir `Incidencias: ninguna`.
