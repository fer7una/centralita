# AGENTS.md

## Objetivo del repositorio

Centralita es una aplicacion de escritorio portable para Windows orientada a organizar, agrupar y ejecutar proyectos locales desde una unica interfaz.

## Stack oficial

- Tauri 2
- React 19
- TypeScript
- Vite
- Rust
- ESLint
- Prettier
- Vitest

## Plataforma prioritaria

- Windows es la plataforma objetivo principal.
- Las decisiones de desarrollo, pruebas y DX deben optimizarse primero para Windows.
- Toda propuesta debe asumir Tauri 2 como runtime de escritorio.
- No introducir soluciones que degraden el flujo principal `npm run tauri:dev` en Windows.

## Principios de trabajo

- Tareas pequenas, cerradas y revisables.
- Un objetivo funcional por tarea.
- Cambios incrementales antes que refactors amplios.
- No cambiar arquitectura, carpetas o contratos sin peticion explicita.
- No mezclar en una sola tarea frontend, runtime Tauri, persistencia y tooling salvo necesidad real.
- Antes de editar, entender el alcance y los archivos afectados.
- Preferir estructura clara, localizable y facil de revisar frente a abstracciones genericas.
- No introducir acoplamiento oculto, dependencias circulares, archivos sobredimensionados ni "utils" sin propietario claro.

## Arquitectura obligatoria

El proyecto debe evolucionar de forma incremental hacia esta direccion de dependencias:

```text
React UI
  -> frontend application layer
  -> typed Tauri IPC client
  -> Tauri commands
  -> Rust application services / use cases
  -> Rust domain
  -> Rust infrastructure
```

Reglas:

- React no debe conocer detalles de base de datos, filesystem, procesos del SO ni infraestructura Rust.
- El dominio Rust no debe depender de Tauri.
- Los comandos Tauri son adaptadores IPC: reciben DTOs, llaman casos de uso o servicios y mapean errores.
- La infraestructura implementa persistencia, filesystem, OS APIs, HTTP, configuracion y plugins; no debe dirigir el dominio.
- Frontend y backend se comunican solo mediante contratos IPC explicitos y tipados.
- No crear arquitecturas paralelas. Si existe un patron local, extenderlo antes de inventar otro.
- Mantener `docs/ARCHITECTURE.md` actualizado cuando cambien capas, limites o contratos.

## Reglas frontend

- Seguir Feature-Sliced Design como direccion objetivo: `app`, `pages`, `widgets`, `features`, `entities`, `shared`.
- Direccion permitida: `app -> pages/widgets/features/entities/shared`, `pages -> widgets/features/entities/shared`, `widgets -> features/entities/shared`, `features -> entities/shared`, `entities -> shared`, `shared -> nada superior`.
- `src/shared/api/tauri/` es la unica frontera frontend autorizada para importar APIs de Tauri.
- Los componentes no deben llamar `invoke` ni `listen` directamente.
- Los nombres raw de comandos viven en `src/shared/api/tauri/commands.ts`.
- Los nombres raw de eventos viven en `src/shared/api/tauri/events.ts`.
- Las APIs de feature deben llamar wrappers tipados, no `@tauri-apps/api/core`.
- Evitar cadenas profundas `../../../../` cuando una capa publica o alias sea mas claro.
- Separar UI, estado, validacion, transformaciones y acceso IPC.
- `src/CentralitaApp.tsx` es un candidato de migracion gradual: no dividirlo junto con cambios Rust o de persistencia en la misma tarea salvo necesidad real.

## Reglas Rust y Tauri

- `src-tauri/src` debe seguir siendo una shell/adaptador Tauri lo mas fina posible.
- No poner reglas de negocio dentro de comandos Tauri.
- Usar tipos y errores explicitos para dominio/aplicacion; evitar `anyhow` fuera de bootstrap/binarios.
- Mantener tipos Tauri fuera de dominio y aplicacion.
- La persistencia local actual es SQLite mediante `rusqlite` con archivo `centralita.sqlite3`.
- No cambiar motor de base de datos, ubicacion de datos o esquema sin tarea explicita, migracion e impacto documentado.
- Las migraciones deben considerar datos existentes, rollback razonable, idempotencia e integridad referencial.

## Contrato IPC

- Todo comando debe tener input/output DTO claro.
- Todo error que cruce Rust -> React debe ser serializable y seguro para UI.
- El frontend no debe depender de errores raw stringly-typed cuando exista un contrato mejor.
- Cambios breaking de IPC deben actualizar Rust, TypeScript, tests y documentacion en la misma tarea.
- No exponer filesystem, shell, HTTP u OS powers al frontend sin comando controlado y permisos Tauri minimos.

## Limites de edicion

- Tocar solo los archivos explicitamente permitidos por la tarea.
- No modificar codigo fuente si la tarea es documental o de proceso.
- No tocar runtime Rust salvo que la tarea lo pida de forma explicita.
- No mover ni renombrar carpetas base sin justificarlo.
- No cambiar configuracion global, CI, secretos o infraestructura salvo requerimiento directo.
- No introducir dependencias nuevas sin una razon concreta y visible en la entrega.

## Reglas especificas para este repo

- Mantener compatibilidad con `npm run tauri:dev`.
- Mantener el puerto de desarrollo de Vite en `1420` salvo peticion contraria.
- Preservar la configuracion de Tauri 2 y su flujo con `beforeDevCommand` y `beforeBuildCommand`.
- Si una tarea afecta tooling frontend, validar tambien que no rompe el arranque de Tauri cuando el alcance lo justifique.
- No conceder permisos Tauri amplios de filesystem, shell, HTTP u OS sin justificacion visible.
- No introducir dependencias nuevas para utilidades triviales.
- Si se toca persistencia, revisar `src-tauri/src/persistence/` y las migraciones antes de editar.

## Validaciones obligatorias

Ejecutar las validaciones que correspondan al alcance antes de cerrar una tarea.

### Minimo obligatorio para cambios de frontend o tooling

- `npm run lint`
- `npm run typecheck`
- `npm run test:run`
- `npm run build`

### Obligatorio adicional para cambios que toquen integracion Tauri

- `npm run tauri:dev`

### Obligatorio adicional para cambios Rust

- `cargo test`

Si una validacion no puede ejecutarse, debe indicarse de forma explicita en la entrega junto con el motivo.

## Flujo de ejecucion esperado para Codex

1. Leer el objetivo y fijar alcance.
2. Confirmar restricciones y archivos tocables.
3. Inspeccionar solo el contexto necesario.
4. Proponer o ejecutar cambios pequenos.
5. Validar antes de cerrar.
6. Entregar resultado con formato estandar.

## Criterios para dividir trabajo

Dividir una tarea en subtareas cuando ocurra cualquiera de estos casos:

- Cambia mas de una capa principal del sistema.
- Requiere introducir dependencias nuevas y ademas tocar UI o runtime.
- Implica mas de 5 archivos funcionales.
- Necesita validaciones largas o riesgosas que conviene aislar.

## Formato de respuesta por tarea

La entrega final de cada tarea debe incluir siempre:

- `Resumen:` que se hizo y con que alcance.
- `Archivos modificados:` lista plana de archivos tocados.
- `Validaciones ejecutadas:` comandos lanzados y estado.
- `Incidencias:` bloqueos, riesgos, dudas o efectos laterales.

Si no hubo incidencias, indicar `Incidencias: ninguna`.

## Politica de seguridad y cambios destructivos

- No borrar archivos o carpetas sin necesidad clara.
- No usar comandos destructivos de Git para limpiar cambios.
- No asumir que procesos locales pueden cerrarse sin verificar impacto.
- Si una accion puede romper el flujo local del usuario, explicarlo antes o dejarlo documentado en la entrega.

## Politica de documentacion

- Toda decision de proceso repetible debe quedar en `AGENTS.md` o en `docs/`.
- El contenido documental debe ser breve, operativo y orientado a ejecucion.
- Evitar texto aspiracional o ambiguo; priorizar reglas verificables.
