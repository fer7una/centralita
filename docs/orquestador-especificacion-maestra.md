# Orquestador de Proyectos Locales — Especificación Maestra

## Estado del documento
Documento maestro vivo del proyecto. Sustituye la acumulación de especificaciones por sprint en el chat.

Estado actual:
- Sprint 0: completado
- Sprint 1: completado
- Sprint 2: completado
- Sprint 3: completado
- Sprint 4: completado

---

## 1. Visión del producto
Aplicación de escritorio portable para organizar proyectos locales por grupos y subgrupos, detectar automáticamente su tipo, inferir cómo ejecutarlos y, en fases posteriores, orquestar su ejecución y parada desde una única interfaz.

El sistema debe permitir:
- Crear y guardar workspaces locales.
- Organizar proyectos por grupos y subgrupos ilimitados.
- Asignar colores personalizados a grupos y subgrupos.
- Añadir proyectos seleccionando una carpeta.
- Detectar automáticamente el tipo de proyecto y sugerir su configuración de ejecución.
- Ejecutar un proyecto, un grupo o todo el workspace.
- Detener un proyecto, un grupo o todo el workspace.
- Mostrar en tiempo real qué se está ejecutando y qué no.
- Consultar logs y estado operativo.

Principio de producto:
- La app es el orquestador principal.
- No depende de que IntelliJ o VS Code estén abiertos.
- El usuario conserva siempre la última palabra sobre la configuración detectada.

---

## 2. Stack recomendado
### Base tecnológica
- Desktop shell: **Tauri 2**
- Frontend: **React + TypeScript + Vite**
- Estado UI: **Zustand**
- Persistencia local: **SQLite**
- Core local: **Rust**
- Comunicación backend/frontend: comandos y eventos de Tauri

### Motivos de esta elección
- Aplicación de escritorio ligera y portable.
- UI moderna y suficientemente flexible para árbol, paneles y consola.
- Backend nativo robusto para filesystem, procesos y persistencia.
- Separación limpia entre interfaz, dominio, detección y runtime.
- Buen encaje con VS Code y flujo asistido por Codex.

### Alternativas descartadas
- **Electron**: válido, pero menos atractivo para este caso por peso y orientación.
- **Python + Tkinter/PySide**: útil para prototipo, peor encaje como producto final mantenible.
- **Bash**: no válido como base del producto.

---

## 3. Principios de arquitectura
1. La fuente de verdad del estado de ejecución vive en el backend local, no en React.
2. El árbol del workspace y el runtime deben estar desacoplados.
3. Toda detección automática debe poder sobrescribirse manualmente.
4. La app debe funcionar aunque los IDEs estén cerrados.
5. La persistencia del árbol se modela en tablas planas; la jerarquía se reconstruye al leer.
6. Las tareas delegadas a Codex deben ser pequeñas, cerradas y verificables.

---

## 4. Funcionalidades objetivo del producto
### Núcleo
- Workspaces locales
- Grupos y subgrupos
- Colores personalizados
- CRUD de proyectos

### Importación inteligente
- Selección de carpeta
- Detección automática de tipo de proyecto
- Inferencia de runner/gestor de paquetes
- Propuesta de nombre, working dir y comando
- Confirmación/edición antes de persistir

### Orquestación futura
- Start/stop por proyecto
- Start/stop por grupo
- Start/stop global
- Estado en vivo
- Logs por proyecto
- Historial de ejecuciones
- Health checks

---

## 5. Modelo de dominio base
### Workspace
- `id`
- `name`
- `createdAt`
- `updatedAt`

### GroupNode
- `id`
- `workspaceId`
- `parentGroupId` nullable
- `name`
- `color`
- `sortOrder`
- `createdAt`
- `updatedAt`

### ProjectNode
- `id`
- `workspaceId`
- `groupId`
- `name`
- `path`
- `detectedType` nullable
- `color` nullable
- `command` nullable
- `args[]` opcional
- `env` opcional
- `workingDir` nullable
- `createdAt`
- `updatedAt`

### Reglas de dominio
1. Un grupo pertenece a un workspace.
2. Un subgrupo pertenece a un grupo padre del mismo workspace.
3. Un proyecto pertenece a un grupo.
4. En el MVP, un proyecto solo puede vivir en un grupo.
5. `sortOrder` manda entre hermanos.
6. El borrado de un grupo arrastra subgrupos y proyectos descendientes.

---

## 6. Persistencia
### Decisión vigente
Persistencia local con **SQLite**.

### Motivo
El producto ya requiere:
- relaciones jerárquicas,
- orden entre hermanos,
- evolución futura a historial,
- metadatos adicionales,
- migraciones.

### Esquema base
#### `workspaces`
- `id TEXT PRIMARY KEY`
- `name TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

#### `groups`
- `id TEXT PRIMARY KEY`
- `workspace_id TEXT NOT NULL`
- `parent_group_id TEXT NULL`
- `name TEXT NOT NULL`
- `color TEXT NOT NULL`
- `sort_order INTEGER NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

#### `projects`
- `id TEXT PRIMARY KEY`
- `workspace_id TEXT NOT NULL`
- `group_id TEXT NOT NULL`
- `name TEXT NOT NULL`
- `path TEXT NOT NULL`
- `detected_type TEXT NULL`
- `color TEXT NULL`
- `command TEXT NULL`
- `args_json TEXT NULL`
- `env_json TEXT NULL`
- `working_dir TEXT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

### Evolución prevista de persistencia
Con Sprint 2 y posteriores conviene añadir o consolidar:
- `package_manager`
- `executable`
- `detection_confidence`
- `detection_evidence_json`
- `warnings_json`

---

## 7. Arquitectura interna propuesta
### Frontend
- `src/app/`
- `src/components/`
- `src/features/`
- `src/hooks/`
- `src/lib/`
- `src/store/`
- `src/styles/`
- `src/test/`
- `src/types/`

### Backend local (Rust)
- `src-tauri/src/commands/`
- `src-tauri/src/detection/`
- `src-tauri/src/events/`
- `src-tauri/src/models/`
- `src-tauri/src/persistence/`
- `src-tauri/src/runtime/`
- `src-tauri/src/utils/`

### Otras carpetas clave
- `.codex/`
- `docs/architecture/`
- `docs/product/`
- `docs/decisions/`
- `scripts/`

---

## 8. Flujo de trabajo con VS Code y Codex
### Principios
1. Una tarea por cambio funcional.
2. Una rama por tarea.
3. Validación automática mínima antes de cerrar cada tarea.
4. Nada de pedir a Codex una feature gigante sin límites.
5. El dominio y el runtime se prueban antes de tocar demasiado la UI.

### Estrategia de ramas
- `main`: siempre estable
- `develop`: integración del sprint actual
- `feat/...`
- `fix/...`
- `refactor/...`
- `docs/...`
- `chore/...`

### Convención de commits
- `feat:`
- `fix:`
- `chore:`
- `docs:`
- `refactor:`
- `test:`

### Reglas operativas para Codex
- Tareas pequeñas y cerradas.
- Indicar siempre qué archivos puede tocar y cuáles no.
- Exigir criterios de aceptación claros.
- Exigir validaciones obligatorias.
- Revisar diff antes de mergear.
- No permitir refactors fuera de alcance.

### Qué revisar manualmente
- decisiones de arquitectura,
- seguridad y gestión de procesos,
- contratos entre frontend y backend,
- política de borrado,
- diseño del modelo de ejecución.

---

## 9. Sprint 0 — completado
### Objetivo alcanzado
Dejar listo el repositorio, el entorno de desarrollo, la estructura del código y las reglas de trabajo para Codex.

### Resultado esperado ya cumplido
- Scaffold funcional Tauri 2 + React + TypeScript + Vite
- Estructura de carpetas estable
- Tooling de lint, format, typecheck y test
- `AGENTS.md` creado
- `.codex/config.toml` creado
- README y documentación base creados
- Flujo de trabajo con VS Code + Codex preparado

### Definition of Done del S0
- `tauri dev` arranca
- `npm run build` pasa
- `npm run validate` pasa
- `cargo test` pasa
- `AGENTS.md` presente
- `.codex/config.toml` presente
- documentación base presente

---

## 10. Sprint 1 — completado
### Objetivo alcanzado
Implementar el núcleo del dominio y la persistencia local del workspace.

### Resultado esperado ya cumplido
- Crear workspaces
- Crear grupos en raíz
- Crear subgrupos
- Crear proyectos manualmente
- Listar el árbol completo
- Editar nombre, color y orden básico
- Borrar grupos y proyectos
- Persistir y recuperar el estado tras reinicio

### Contratos backend mínimos del S1
- `create_workspace`
- `list_workspaces`
- `get_workspace_tree`
- `rename_workspace`
- `delete_workspace`
- `create_group`
- `update_group`
- `delete_group`
- `create_project`
- `update_project`
- `delete_project`

### Política de borrado vigente
- Borrar grupo = borrar subgrupos y proyectos descendientes

### Definition of Done del S1
- CRUD funcional desde UI mínima
- SQLite persistiendo correctamente
- árbol válido reconstruido desde tablas planas
- tests básicos pasando
- persistencia recuperable tras reinicio

---

## 11. Sprint 2 — completado
Estado actual del sprint:
- completado
- base de persistencia y DTOs de detección introducida
- scanner seguro de carpetas implementado
- reglas de detección Node y Java implementadas
- comandos Tauri de análisis y guardado desde detección disponibles
- UX de revisión previa al guardado implementada

### Objetivo
Implementar el sistema que, al seleccionar una carpeta local, inspecciona su contenido, detecta el tipo de proyecto, infiere cómo ejecutarlo y propone una configuración inicial editable antes de guardar el proyecto en el workspace.

### Resultado esperado
Debe ser posible:
- seleccionar una carpeta local,
- escanear su estructura,
- clasificar el proyecto,
- inferir un comando recomendado,
- inferir runner o package manager,
- proponer nombre y working dir,
- mostrar confianza y evidencias,
- permitir edición previa al guardado,
- persistir el proyecto con la configuración revisada.

### Alcance
Incluye:
- selector de carpeta,
- scanner seguro,
- reglas de detección,
- scoring y desempate,
- DTO de resultado,
- UI de revisión previa al guardado,
- tests con fixtures.

No incluye:
- ejecución real de procesos,
- logs,
- health checks,
- autoarranque tras importar,
- importación masiva,
- soporte Docker operativo.

### Filosofía de detección
La detección debe:
- exponer evidencias,
- devolver score de confianza,
- permitir override manual,
- ser extensible por reglas.

El sistema ayuda, no impone.

### Tipos detectables en S2
#### Java
- Maven
- Gradle
- Spring Boot Maven
- Spring Boot Gradle
- Java JAR ejecutable

#### JS/TS
- Node genérico
- Vite
- React + Vite
- Next.js
- Express

#### Otros
- Custom / Unknown

### Ficheros clave a inspeccionar
- `package.json`
- `package-lock.json`
- `pnpm-lock.yaml`
- `yarn.lock`
- `vite.config.ts`
- `vite.config.js`
- `next.config.js`
- `next.config.mjs`
- `pom.xml`
- `build.gradle`
- `build.gradle.kts`
- `settings.gradle`
- `settings.gradle.kts`
- `mvnw`
- `gradlew`
- `src/main/resources/application.properties`
- `src/main/resources/application.yml`
- artefactos `.jar` cuando proceda

### Reglas de detección resumidas
#### Vite
Se detecta si existe config de Vite, dependencia `vite` o scripts que invoquen `vite`.

#### React + Vite
Vite + dependencia `react` + plugin React o estructura compatible.

#### Next.js
Dependencia `next` o archivo `next.config.*`.

#### Express
Dependencia `express` y script o entrypoint compatible.

#### Node genérico
`package.json` presente sin encajar en una regla más específica.

#### Maven
`pom.xml` presente; prioridad a `mvnw` si existe.

#### Gradle
`build.gradle` o `build.gradle.kts`; prioridad a `gradlew` si existe.

#### Spring Boot
Presencia de Spring Boot en dependencias, plugins o parent dentro de Maven/Gradle.

#### Java JAR
`.jar` ejecutable utilizable cuando no exista una vía mejor orientada a desarrollo.

### Inferencia de package manager / runner
#### Node
Prioridad:
1. `pnpm-lock.yaml`
2. `yarn.lock`
3. `package-lock.json`
4. npm por defecto

#### Java
Prioridad:
1. wrapper local
2. herramienta global
3. `java -jar` si aplica

### Comandos sugeridos por defecto
#### Vite / ReactVite / Next
- `pnpm dev`
- `yarn dev`
- `npm run dev`

#### Node / Express
Prioridad:
1. `dev`
2. `start`
3. `serve`
4. edición manual si no hay script válido

#### Spring Boot Maven
- wrapper: `mvnw spring-boot:run`
- fallback: `mvn spring-boot:run`

#### Spring Boot Gradle
- wrapper: `gradlew bootRun`
- fallback: `gradle bootRun`

#### Java JAR
- `java -jar <ruta>`

### Nota de modelado para S2
Conviene introducir o consolidar:
- `executable`
- `args[]`
- `workingDir`
- `packageManager`
- `detectedType`
- `detectionConfidence`
- `detectionEvidence[]`

### DTO principal de detección
#### `DetectionResult`
- `detectedType`
- `displayName`
- `path`
- `workingDir`
- `packageManager` nullable
- `executable` nullable
- `args[]`
- `commandPreview`
- `confidence` (0..1)
- `evidence[]`
- `warnings[]`
- campos editables

#### `DetectionEvidence`
- `kind`
- `source`
- `detail`
- `weight`

### Modelo simple de scoring
- fichero estructural fuerte: `+0.35`
- wrapper relevante: `+0.15`
- dependencia o plugin inequívoco: `+0.25`
- script compatible: `+0.15`
- lockfile o pista secundaria: `+0.10`

Umbrales:
- `>= 0.80`: alta confianza
- `0.55 - 0.79`: confianza media
- `< 0.55`: baja confianza

### Reglas de desempate
1. Gana la regla más específica.
2. Si empatan, gana la de mayor score.
3. Si persiste ambigüedad, warning y revisión manual.

### UX mínima del S2
Flujo:
1. Pulsar “Añadir proyecto”.
2. Seleccionar carpeta.
3. Analizar carpeta.
4. Mostrar revisión en modal o pantalla.
5. Permitir edición.
6. Confirmar y persistir en el grupo elegido.

Campos visibles:
- nombre detectado
- ruta
- tipo detectado
- runner o package manager
- comando sugerido
- working dir
- confianza
- evidencias
- warnings
- grupo destino

Campos editables:
- nombre
- tipo
- ejecutable
- args
- working dir
- grupo destino
- color opcional

### Restricciones técnicas del S2
- No ejecutar nada durante la detección.
- No modificar el filesystem.
- No recorrer carpetas gigantes sin límite.
- Ignorar por defecto: `node_modules`, `target`, `build`, `.git`, `.idea`, `dist`, `.next`.
- Limitar lectura de ficheros grandes.

### Casos borde a cubrir
- carpeta vacía
- `package.json` inválido
- `pom.xml` inválido
- proyecto híbrido
- monorepo simple
- proyecto sin scripts útiles
- permisos de lectura insuficientes

### Decisión sobre monorepos en S2
- No se resuelven monorepos complejos de forma completa.
- Si hay múltiples candidatos, se devuelve warning.
- Se propone el mejor candidato simple o se cae a `Custom`.

### Contratos backend mínimos del S2
- `analyze_project_folder(path)`
- `create_project_from_detection(input)`

Opcional útil:
- `recalculate_detection(path)`

### Orden recomendado de implementación del S2
1. Modelo de detección
2. Scanner seguro
3. Reglas Node/Vite/Next/Express
4. Reglas Maven/Gradle/Spring Boot/JAR
5. Scoring y desempate
6. Comandos Tauri
7. UI de importación y revisión
8. Fixtures y tests

### Checklist funcional del S2
- Seleccionar carpeta y recibir análisis
- Detectar Vite correctamente
- Detectar ReactVite sin degradarlo a Node genérico
- Detectar Spring Boot con comando razonable
- Inferir bien npm/pnpm/yarn
- Permitir edición previa al guardado
- Persistir el proyecto configurado
- Mostrar evidencias y confianza de forma comprensible

### Definition of Done del S2
- La app puede analizar carpetas locales sin ejecutar nada.
- Devuelve un `DetectionResult` consistente y explicable.
- Permite confirmar o corregir antes de persistir.
- Persiste el proyecto con la configuración revisada.
- Los casos comunes Java y Node quedan cubiertos con tests.

---

## 12. Sprint 3 — especificación vigente
Estado actual del sprint:
- completado
- `S3-01` completado
- `S3-02` completado
- `S3-03` completado
- `S3-04` completado
- `S3-05` completado
- `S3-06` completado
- `S3-07` completado
- `S3-08` completado
- `S3-09` completado

### Objetivo
Implementar el runtime local de procesos para que la aplicación pueda arrancar, detener, reiniciar y observar proyectos ya persistidos, tanto de forma individual como por grupo o workspace completo.

El Sprint 3 convierte la configuración generada en S2 en ejecución real controlada por la app.

### Resultado esperado
Al finalizar el Sprint 3 debe ser posible:
- iniciar un proyecto desde su configuración persistida,
- detener un proyecto en ejecución,
- reiniciar un proyecto,
- iniciar todos los proyectos de un grupo,
- detener todos los proyectos de un grupo,
- iniciar todos los proyectos de un workspace,
- detener todos los proyectos de un workspace,
- capturar `stdout` y `stderr`,
- mostrar logs en tiempo real en la UI,
- consultar el estado operativo de cada proyecto,
- detectar salida inesperada de un proceso,
- limpiar procesos al cerrar la aplicación.

### Alcance
Incluye:
- modelo de runtime y estados,
- gestor central de procesos vivos,
- comandos Tauri de start/stop/restart/status,
- eventos backend -> frontend,
- captura de logs,
- parada segura y parada forzada,
- ejecución individual, por grupo y global,
- UI mínima de control y observación.

No incluye:
- health checks HTTP/TCP,
- historial persistido de ejecuciones,
- dependencias entre proyectos,
- orden avanzado de arranque,
- auto-restart,
- perfiles por entorno,
- ejecución mediante Docker Compose.

### Filosofía del Sprint 3
El runtime debe ser controlado, observable y reversible.

Reglas:
- la UI no ejecuta procesos directamente,
- React solo consume comandos y eventos,
- el backend Rust es la fuente de verdad del estado runtime,
- no se debe ejecutar nada que no venga de una configuración revisada por el usuario,
- no se debe usar shell libre si puede evitarse,
- siempre se debe separar `executable` y `args[]`.

### Modelo de runtime
#### `RuntimeStatus`
- `STOPPED`
- `STARTING`
- `RUNNING`
- `STOPPING`
- `FAILED`

Estados futuros fuera de S3:
- `RUNNING_HEALTHY`
- `RUNNING_UNHEALTHY`

#### `RunRequest`
- `projectId`
- `executable`
- `args[]`
- `workingDir`
- `env` opcional

#### `ProcessRuntimeState`
- `projectId`
- `status`
- `pid` nullable
- `startedAt` nullable
- `stoppedAt` nullable
- `exitCode` nullable
- `lastError` nullable
- `commandPreview`

#### `RuntimeLogLine`
- `projectId`
- `stream`: `stdout` | `stderr`
- `line`
- `timestamp`

#### `RuntimeEvent`
- `ProjectStarting`
- `ProjectStarted`
- `ProjectStopping`
- `ProjectStopped`
- `ProjectFailed`
- `ProjectLogLine`

### Gestión interna de procesos
El backend debe mantener un registro en memoria de procesos vivos:

- clave: `projectId`
- valor: handle del proceso, metadatos, estado y buffer de logs

Requisitos:
- impedir doble arranque del mismo proyecto,
- actualizar estado al iniciar, parar o fallar,
- capturar salida estándar y error estándar,
- emitir eventos Tauri por cada cambio relevante,
- detectar cuando el proceso termina por sí solo,
- diferenciar parada voluntaria de fallo inesperado,
- limpiar el registro cuando el proceso termina.

### Política de logs
En S3 los logs pueden ser memoria temporal, no persistencia.

Recomendación:
- mantener un buffer circular por proyecto,
- límite inicial: 500-1000 líneas por proyecto,
- emitir cada línea nueva al frontend,
- permitir consultar logs recientes con un comando.

### Política de start
Al arrancar un proyecto:
1. cargar configuración persistida,
2. validar que existe `executable` o `command` compatible,
3. validar `workingDir`,
4. construir `RunRequest`,
5. marcar `STARTING`,
6. lanzar proceso,
7. capturar `pid`,
8. marcar `RUNNING`,
9. emitir eventos.

Si falla el arranque:
- marcar `FAILED`,
- guardar `lastError`,
- emitir evento de fallo.

### Política de stop
Al detener un proyecto:
1. marcar `STOPPING`,
2. intentar parada normal,
3. esperar un tiempo corto,
4. si sigue vivo, aplicar parada forzada,
5. matar árbol de procesos cuando sea posible,
6. marcar `STOPPED`,
7. limpiar handle interno.

### Kill tree
S3 debe contemplar que muchos comandos lanzan procesos hijos.

Casos típicos:
- `npm run dev` -> proceso hijo de Vite/Next,
- `mvn spring-boot:run` -> JVM hija,
- `gradlew bootRun` -> proceso Java hijo.

La parada debe intentar matar el árbol completo.

Prioridad:
1. solución nativa/multiplataforma desde Rust si es viable,
2. fallback específico por sistema operativo,
3. en Windows, evitar dejar hijos vivos como Vite o Java corriendo después de cerrar el proceso padre.

### Ejecución masiva
#### Start group
- obtener proyectos descendientes del grupo,
- iniciar solo los que estén `STOPPED` o `FAILED`,
- evitar duplicados,
- devolver resumen de arranque.

#### Stop group
- obtener proyectos descendientes del grupo,
- detener solo los que estén vivos,
- devolver resumen de parada.

#### Start workspace
- iniciar todos los proyectos del workspace que no estén ya corriendo.

#### Stop workspace
- detener todos los proyectos vivos del workspace.

En S3 el arranque masivo puede ser paralelo simple. El orden configurable se pospone.

### Estado agregado
La UI debe poder representar estado en grupos y workspace:

- si todos están parados -> `STOPPED`,
- si alguno está arrancando -> `STARTING`,
- si alguno está fallando -> `FAILED`,
- si alguno está corriendo -> `RUNNING`,
- si alguno está deteniéndose -> `STOPPING`.

La regla exacta puede ajustarse, pero debe ser consistente y documentada.

### Contratos backend mínimos del S3
- `start_project(project_id)`
- `stop_project(project_id)`
- `restart_project(project_id)`
- `get_project_runtime_status(project_id)`
- `get_workspace_runtime_status(workspace_id)`
- `get_project_logs(project_id)`
- `start_group(group_id)`
- `stop_group(group_id)`
- `start_workspace(workspace_id)`
- `stop_workspace(workspace_id)`

### Eventos Tauri mínimos
- `runtime://status-changed`
- `runtime://log-line`
- `runtime://process-exited`
- `runtime://process-error`

Payloads esperados:
- `projectId`,
- `status`,
- `pid` cuando exista,
- `timestamp`,
- `message` o `line` cuando aplique.

### Cambios de persistencia en S3
S3 no necesita persistir historial todavía.

Sí debe asegurar que `projects` dispone de configuración suficiente para ejecutar:
- `executable`,
- `args_json`,
- `working_dir`,
- `env_json`,
- `command` como vista derivada o compatibilidad.

Si estos campos no quedaron consolidados en S2, deben cerrarse al inicio de S3.

### UX mínima del S3
#### En árbol de workspace
Cada proyecto debe mostrar:
- estado visual,
- botón start,
- botón stop,
- botón restart.

Cada grupo debe mostrar:
- acción start group,
- acción stop group,
- estado agregado.

Workspace:
- start all,
- stop all,
- resumen de estados.

#### Panel de detalle del proyecto
Debe mostrar:
- estado actual,
- PID si existe,
- comando efectivo,
- working dir,
- último error,
- consola de logs.

#### Consola/logs
- mostrar stdout y stderr,
- autoscroll opcional,
- botón limpiar vista local,
- indicar si el proceso terminó.

### Arquitectura interna recomendada para S3
#### Rust
- `runtime/process_manager.rs`
- `runtime/process_state.rs`
- `runtime/log_buffer.rs`
- `runtime/kill_tree.rs`
- `commands/runtime_commands.rs`
- `events/runtime_events.rs`

#### Frontend
- `features/runtime/`
- `components/runtime/`
- `store/runtimeStore.ts`
- `components/logs/`

### Restricciones técnicas
- no ejecutar comandos mediante shell libre salvo caso justificado,
- validar `workingDir` antes de arrancar,
- no permitir doble start del mismo proyecto,
- no bloquear el hilo principal,
- no congelar la UI si un proceso genera muchos logs,
- limitar buffers de logs,
- limpiar listeners al desmontar componentes.

### Casos borde a cubrir
- proyecto sin comando ejecutable,
- working dir inexistente,
- executable inexistente o no encontrado en PATH,
- proyecto ya corriendo,
- stop de proyecto ya parado,
- proceso que termina solo,
- proceso que no responde a stop normal,
- proceso con muchos logs,
- cierre de app con procesos vivos.

### Decisiones pospuestas
Quedan fuera de S3:
- health checks,
- persistencia de historial,
- orden configurable de arranque,
- dependencias entre proyectos,
- perfiles por entorno,
- reinicio automático,
- monorepo avanzado.

### Orden recomendado de implementación del S3
1. Modelo runtime y eventos.
2. Process manager individual.
3. Captura de logs y buffer circular.
4. Stop/kill tree.
5. Comandos Tauri individuales.
6. Estado runtime en frontend.
7. UI mínima de start/stop/restart y logs.
8. Ejecución por grupo y workspace.
9. Tests y casos borde.

### Checklist funcional del S3
- iniciar proyecto Vite importado desde S2,
- detenerlo sin dejar proceso hijo vivo,
- iniciar proyecto Spring Boot importado desde S2,
- detenerlo sin dejar JVM viva,
- ver logs en tiempo real,
- ver estado actualizado en árbol y detalle,
- reiniciar proyecto,
- iniciar y detener grupo,
- iniciar y detener workspace completo,
- manejar fallo de arranque con estado `FAILED`.

### Definition of Done del S3
S3 está terminado solo si:
- la ejecución individual funciona de extremo a extremo,
- la parada individual no deja procesos huérfanos en casos comunes,
- los logs llegan al frontend en tiempo real,
- el estado runtime es consistente,
- la ejecución masiva por grupo y workspace funciona,
- los errores se muestran de forma comprensible,
- los tests básicos pasan,
- la app limpia procesos vivos al cerrar o al menos advierte y ofrece detenerlos.

### Tareas listas para Codex del Sprint 3
#### Prompt S3-01
```text
Objetivo:
Definir el modelo runtime del Sprint 3: RuntimeStatus, RunRequest, ProcessRuntimeState, RuntimeLogLine y RuntimeEvent en Rust y TypeScript.

Contexto:
La app ya puede persistir proyectos e importarlos desde detección. Ahora añadimos ejecución real de procesos.

Puedes tocar:
- src-tauri/src/models/*
- src-tauri/src/runtime/*
- src/types/*

No toques:
- detection
- UI salvo tipos compartidos
- persistencia salvo que sea estrictamente necesario documentar compatibilidad

Criterios de aceptación:
- existen modelos runtime en Rust
- existen tipos equivalentes en TypeScript
- los estados cubren STOPPED, STARTING, RUNNING, STOPPING y FAILED
- los eventos incluyen cambios de estado y líneas de log

Validación:
- cargo test
- npm run typecheck

Entrega:
- resumen
- archivos modificados
- decisiones de modelado
```

#### Prompt S3-02
```text
Objetivo:
Implementar un ProcessManager backend capaz de iniciar un proyecto individual desde su configuración persistida y mantener su estado en memoria.

Contexto:
El runtime backend debe ser la fuente de verdad. React no ejecuta procesos directamente.

Puedes tocar:
- src-tauri/src/runtime/*
- src-tauri/src/persistence/* si hace falta leer ProjectNode
- src-tauri/src/models/*

No toques:
- UI
- detection

Criterios de aceptación:
- start individual lanza proceso con executable, args y workingDir
- impide doble arranque del mismo projectId
- registra pid, status, startedAt y commandPreview
- detecta fallo de arranque y marca FAILED
- incluye tests donde sea viable

Validación:
- cargo test
- cargo clippy --all-targets --all-features -- -D warnings

Entrega:
- resumen
- archivos modificados
- limitaciones conocidas
```

#### Prompt S3-03
```text
Objetivo:
Añadir captura de stdout/stderr, buffer circular de logs por proyecto y eventos Tauri para enviar logs al frontend.

Contexto:
La UI necesita observar la ejecución en tiempo real sin bloquearse.

Puedes tocar:
- src-tauri/src/runtime/*
- src-tauri/src/events/*
- src-tauri/src/commands/* si necesitas exponer get_project_logs
- src/types/*

No toques:
- detection
- UI visual compleja

Criterios de aceptación:
- stdout y stderr se capturan por separado
- cada línea genera RuntimeLogLine
- hay buffer limitado por proyecto
- se emite evento al frontend por línea
- existe comando para consultar logs recientes

Validación:
- cargo test
- npm run typecheck

Entrega:
- resumen
- archivos modificados
- decisiones sobre límites de buffer
```

#### Prompt S3-04
```text
Objetivo:
Implementar stop/restart individual con parada normal, fallback forzado y estrategia de kill tree cuando sea posible.

Contexto:
Muchos comandos como npm, Maven o Gradle generan procesos hijos. La app no debe dejar Vite o Java corriendo al detener.

Puedes tocar:
- src-tauri/src/runtime/*
- src-tauri/src/commands/*

No toques:
- detection
- UI

Criterios de aceptación:
- stop_project cambia estado a STOPPING y luego STOPPED
- stop de un proyecto ya parado no rompe
- restart equivale a stop + start controlado
- se intenta matar árbol de procesos
- se documentan limitaciones por sistema operativo

Validación:
- cargo test
- cargo clippy --all-targets --all-features -- -D warnings

Entrega:
- resumen
- archivos modificados
- riesgos de kill tree detectados
```

#### Prompt S3-05
```text
Objetivo:
Exponer comandos Tauri individuales de runtime: start_project, stop_project, restart_project, get_project_runtime_status y get_project_logs.

Contexto:
El frontend necesita controlar proyectos individuales y observar su estado.

Puedes tocar:
- src-tauri/src/commands/*
- src-tauri/src/lib.rs o main correspondiente
- src-tauri/src/runtime/*
- src/types/*

No toques:
- detection salvo imports necesarios
- UI compleja

Criterios de aceptación:
- comandos registrados en Tauri
- errores devueltos de forma consistente
- contratos TypeScript actualizados
- comandos usan ProcessManager compartido

Validación:
- cargo test
- npm run typecheck

Entrega:
- resumen
- archivos modificados
- contratos expuestos
```

#### Prompt S3-06
```text
Objetivo:
Implementar estado runtime en frontend y UI mínima para start/stop/restart individual con panel de logs.

Contexto:
La UI debe consumir comandos y eventos del runtime backend. Diseño provisional, funcionalidad primero.

Puedes tocar:
- src/features/runtime/*
- src/components/runtime/*
- src/components/logs/*
- src/store/*
- src/app/*
- src/types/*

No toques:
- detection
- persistencia backend

Criterios de aceptación:
- cada proyecto muestra estado runtime
- se puede iniciar, detener y reiniciar desde UI
- se reciben eventos de log en tiempo real
- el panel de detalle muestra pid, comando y último error
- los listeners se limpian al desmontar

Validación:
- npm run typecheck
- npm run lint
- npm run test:run

Entrega:
- resumen
- archivos modificados
- limitaciones conocidas de UI
```

#### Prompt S3-07
```text
Objetivo:
Implementar ejecución masiva por grupo y workspace: start_group, stop_group, start_workspace y stop_workspace.

Contexto:
El dominio ya tiene árbol de grupos y proyectos. El runtime individual ya existe.

Puedes tocar:
- src-tauri/src/runtime/*
- src-tauri/src/commands/*
- src-tauri/src/persistence/* si hace falta consultar descendientes
- src/features/runtime/*
- src/components/* relacionados

No toques:
- detection
- importación de proyectos

Criterios de aceptación:
- se pueden iniciar y detener todos los proyectos de un grupo
- se pueden iniciar y detener todos los proyectos de un workspace
- no reinicia proyectos ya corriendo salvo que se pida explícitamente
- devuelve resumen con éxitos y fallos
- la UI muestra acciones masivas básicas

Validación:
- cargo test
- npm run typecheck
- npm run lint

Entrega:
- resumen
- archivos modificados
- casos borde cubiertos
```

### Salida esperada del Sprint 3
Al cerrar S3 la aplicación dejará de ser solo un gestor/importador y pasará a ser un orquestador operativo: podrá ejecutar proyectos reales, detenerlos, observar su salida y coordinar acciones individuales o masivas desde una única ventana.

---

## 13. Sprint 4 — completado
Estado actual del sprint:
- completado
- health checks HTTP/TCP persistidos y configurables
- historial de ejecuciones persistido y consultable
- runtime con polling de salud, thresholds y grace period
- comandos y eventos Tauri de observabilidad disponibles
- UI mínima de badges, filtros, configuración e historial integrada

### Objetivo
Implementar la capa de observabilidad operativa para que la aplicación no solo sepa si un proceso está vivo, sino también si el servicio está realmente disponible, cómo ha evolucionado su ejecución reciente y cómo filtrar el workspace por estado con utilidad real.

El Sprint 4 convierte el runtime de S3 en una base operativa más fiable: procesos, salud, historial y visibilidad.

### Resultado esperado
Al finalizar el Sprint 4 debe ser posible:
- definir un health check opcional por proyecto,
- evaluar automáticamente la salud de proyectos en ejecución,
- distinguir entre proceso corriendo y servicio sano,
- persistir un historial básico de ejecuciones,
- consultar últimos arranques, paradas, fallos y códigos de salida,
- filtrar el árbol por estado runtime y salud,
- ver indicadores agregados de salud por grupo y workspace,
- refrescar manualmente el estado de salud,
- seguir usando el runtime de S3 sin romper compatibilidad.

### Alcance
Incluye:
- modelo de health checks,
- evaluación de salud en background para proyectos corriendo,
- estados extendidos de runtime/salud,
- historial persistido de ejecuciones,
- filtros de estado y salud en frontend,
- resumen agregado por grupo y workspace,
- UI mínima de configuración y lectura de salud,
- tests de polling, persistencia y agregación.

No incluye:
- dependencias entre proyectos,
- orden configurable de arranque,
- auto-restart por health check,
- Docker Compose,
- perfiles por entorno,
- apertura en IDE,
- importación masiva,
- soporte avanzado de monorepos.

### Filosofía del Sprint 4
En S3 sabíamos si un proceso seguía vivo.
En S4 debemos saber si realmente "está bien".

Principios:
- proceso vivo no implica servicio saludable,
- la salud debe ser observable y explicable,
- un health check es opcional, nunca obligatorio para ejecutar,
- el historial debe servir para diagnóstico, no para decorar,
- la UI debe permitir filtrar el ruido y encontrar fallos rápido.

### Evolución del modelo runtime
#### `RuntimeStatus`
Se mantienen:
- `STOPPED`
- `STARTING`
- `RUNNING`
- `STOPPING`
- `FAILED`

#### `HealthStatus`
Nuevo en S4:
- `UNKNOWN`
- `CHECKING`
- `HEALTHY`
- `UNHEALTHY`
- `UNSUPPORTED`

Regla base:
- si un proyecto no define health check => `UNSUPPORTED` o `UNKNOWN` según prefieras UX, pero debe ser consistente en toda la app.

Recomendación:
- usar `UNSUPPORTED` cuando no haya health check configurado,
- usar `UNKNOWN` cuando sí exista check pero aún no se haya evaluado.

### Modelo de health check
#### `HealthCheckConfig`
- `type`: `http` | `tcp`
- `enabled`
- `intervalMs`
- `timeoutMs`
- `gracePeriodMs`
- `successThreshold`
- `failureThreshold`

#### `HttpHealthCheckConfig`
- `url`
- `method` (por defecto `GET`)
- `expectedStatusCodes[]`
- `headers` opcional
- `containsText` opcional

#### `TcpHealthCheckConfig`
- `host`
- `port`

#### `ProjectHealthState`
- `projectId`
- `status`
- `lastCheckedAt` nullable
- `lastHealthyAt` nullable
- `lastError` nullable
- `consecutiveSuccesses`
- `consecutiveFailures`

### Reglas de salud
#### HTTP
Se considera saludable si:
- responde dentro de `timeoutMs`,
- el código HTTP está dentro de los esperados,
- y si `containsText` existe, el body lo contiene.

#### TCP
Se considera saludable si:
- el host:port acepta conexión dentro de `timeoutMs`.

### Ciclo de evaluación
1. un proyecto arranca,
2. si tiene health check habilitado, entra en periodo de gracia,
3. tras `gracePeriodMs`, se inicia polling,
4. el estado pasa por `CHECKING`,
5. al cumplir umbrales, se marca `HEALTHY` o `UNHEALTHY`,
6. si el proceso termina, el polling se detiene y la salud vuelve a `UNKNOWN` o estado neutro definido.

### Decisiones de polling en S4
- solo se hace polling de proyectos en `RUNNING`,
- no se evalúan proyectos `STOPPED`,
- no se deben lanzar checks concurrentes del mismo proyecto,
- los intervalos deben tener límites mínimos razonables,
- el polling debe vivir en Rust, no en React.

### Persistencia de historial
#### Objetivo
Guardar suficiente información para diagnóstico sin montar todavía un sistema de auditoría enorme.

#### Tabla sugerida `run_history`
- `id TEXT PRIMARY KEY`
- `project_id TEXT NOT NULL`
- `started_at TEXT NOT NULL`
- `ended_at TEXT NULL`
- `exit_code INTEGER NULL`
- `final_runtime_status TEXT NOT NULL`
- `final_health_status TEXT NULL`
- `stop_reason TEXT NULL`
- `error_message TEXT NULL`
- `command_preview TEXT NOT NULL`

#### Eventos que deben dejar rastro
- arranque exitoso,
- fallo de arranque,
- parada voluntaria,
- salida inesperada,
- fin con exit code,
- último health status conocido al terminar.

### Contratos backend mínimos del S4
- `get_project_health_status(project_id)`
- `refresh_project_health(project_id)`
- `update_project_health_check(project_id, input)`
- `list_project_run_history(project_id, limit)`
- `list_workspace_run_history(workspace_id, limit)`
- `get_workspace_observability_summary(workspace_id)`

Sigue vigente todo el contrato runtime de S3.

### Eventos Tauri mínimos del S4
- `runtime://health-changed`
- `runtime://history-appended` (opcional si aporta valor inmediato)
- `runtime://summary-changed` (opcional; si no, el frontend puede recalcular desde estados recibidos)

Payload mínimo de salud:
- `projectId`
- `healthStatus`
- `lastCheckedAt`
- `lastError`

### Cambios de persistencia en S4
Añadir o consolidar en `projects`:
- `healthcheck_type`
- `healthcheck_enabled`
- `healthcheck_interval_ms`
- `healthcheck_timeout_ms`
- `healthcheck_grace_period_ms`
- `healthcheck_success_threshold`
- `healthcheck_failure_threshold`
- `healthcheck_payload_json`

Añadir tabla:
- `run_history`

### Estado agregado extendido
A nivel de grupo y workspace se deben calcular dos dimensiones:
1. estado runtime agregado,
2. estado de salud agregado.

#### Regla sugerida de salud agregada
- si algún proyecto está `UNHEALTHY` => grupo/workspace `UNHEALTHY`,
- si no hay `UNHEALTHY` pero alguno está `CHECKING` => `CHECKING`,
- si todos los proyectos con health check están `HEALTHY` => `HEALTHY`,
- si ninguno soporta health check => `UNSUPPORTED`,
- si hay mezcla de `HEALTHY` y `UNSUPPORTED`, mostrar `HEALTHY` con contador detallado o un agregado mixto documentado.

La clave es documentarlo y no cambiarlo cada martes porque un badge quedaba más mono.

### UX mínima del Sprint 4
#### Árbol de workspace
Debe permitir:
- filtrar por runtime status,
- filtrar por health status,
- ver badges de salud por proyecto,
- ver resumen agregado por grupo y workspace,
- localizar rápido fallidos o unhealthy.

#### Detalle de proyecto
Debe mostrar:
- configuración de health check,
- último resultado de salud,
- último error de health,
- timestamps de última comprobación y último healthy,
- historial reciente de ejecuciones.

#### Historial
Vista mínima:
- lista de ejecuciones recientes,
- start/end,
- exit code,
- motivo de parada o fallo,
- comando ejecutado.

#### Configuración de health check
Debe permitir editar al menos:
- habilitado,
- tipo HTTP/TCP,
- URL o host/port,
- intervalo,
- timeout,
- grace period,
- thresholds.

### Arquitectura interna recomendada para S4
#### Rust
- `runtime/health_manager.rs`
- `runtime/health_check.rs`
- `runtime/history_recorder.rs`
- `runtime/observability_summary.rs`
- `commands/health_commands.rs`
- `commands/history_commands.rs`
- `models/health.rs`
- `models/history.rs`
- `persistence/run_history_repository.rs`

#### Frontend
- `features/health/`
- `features/history/`
- `components/health/`
- `components/history/`
- `store/observabilityStore.ts`

### Restricciones técnicas
- no bloquear el runtime por polling de salud,
- no inundar la UI con eventos innecesarios,
- limitar frecuencia mínima de checks,
- evitar múltiples timers por proyecto sin control,
- no marcar unhealthy durante el grace period,
- no persistir ruido redundante en historial,
- si un check falla por timeout o network error, registrar causa útil.

### Casos borde a cubrir
- proceso corriendo sin health check,
- health check mal configurado,
- URL inválida,
- puerto inaccesible,
- servicio arranca lento pero termina sano,
- flapping: alternancia entre healthy y unhealthy,
- cierre de app con polling activo,
- historial vacío,
- proyectos que nunca llegaron a arrancar.

### Decisiones pospuestas
Quedan fuera de S4:
- restart automático por unhealthy,
- dependencias entre proyectos y waits encadenados,
- orden configurable de arranque,
- políticas avanzadas de retry,
- dashboards complejos y métricas avanzadas,
- exportación de historial,
- alertas externas.

### Orden recomendado de implementación del S4
1. Modelo de health e historial.
2. Migraciones SQLite para health config y run_history.
3. Registro persistente de ejecuciones desde el runtime existente.
4. Health manager con checks HTTP/TCP y polling controlado.
5. Eventos Tauri de salud.
6. Comandos Tauri de salud e historial.
7. Store frontend de observabilidad.
8. UI mínima de filtros, badges, configuración de health e historial.
9. Tests de polling, thresholds, persistencia y agregación.

### Checklist funcional del S4
- configurar health check HTTP para un proyecto web,
- ver transición `CHECKING -> HEALTHY`,
- ver `UNHEALTHY` si el endpoint falla,
- configurar health check TCP para un servicio de puerto,
- ver historial persistido tras arrancar y detener proyectos,
- filtrar árbol para ver solo `RUNNING`, `FAILED` o `UNHEALTHY`,
- ver resumen agregado por grupo y workspace,
- refrescar salud manualmente,
- mantener compatibilidad con start/stop/logs del S3.

### Definition of Done del S4
S4 está terminado solo si:
- los proyectos pueden tener health check opcional persistido,
- la salud se evalúa automáticamente mientras el proceso corre,
- la UI distingue claramente runtime y salud,
- existe historial persistido consultable,
- los filtros de observabilidad funcionan,
- el estado agregado por grupo/workspace es consistente,
- los tests básicos de health e historial pasan,
- no se rompe el runtime del Sprint 3.

### Tareas listas para Codex del Sprint 4
#### Prompt S4-01
```text
Objetivo:
Definir el modelo de salud e historial del Sprint 4: HealthStatus, HealthCheckConfig, ProjectHealthState y RunHistoryEntry en Rust y TypeScript.

Contexto:
S0-S3 están cerrados. La app ya ejecuta proyectos, captura logs y coordina start/stop. Ahora añadimos observabilidad operativa.

Puedes tocar:
- src-tauri/src/models/*
- src-tauri/src/runtime/*
- src/types/*

No toques:
- detection
- UI salvo tipos compartidos
- lógica de ejecución existente salvo integración mínima de tipos

Criterios de aceptación:
- existen modelos de health e historial en Rust
- existen tipos equivalentes en TypeScript
- el modelo contempla health HTTP/TCP, thresholds y timestamps
- el modelo de historial soporta exit code, stop reason y error message

Validación:
- cargo test
- npm run typecheck

Entrega:
- resumen
- archivos modificados
- decisiones de modelado
```

#### Prompt S4-02
```text
Objetivo:
Añadir migraciones SQLite para configurar health checks en projects y crear la tabla run_history.

Contexto:
La observabilidad del Sprint 4 necesita persistencia, pero sin romper los datos de S1-S3.

Puedes tocar:
- src-tauri/src/persistence/*
- src-tauri/src/models/*
- utilidades de migración

No toques:
- UI
- detection
- lógica de ejecución salvo lo necesario para persistencia

Criterios de aceptación:
- existe migración para campos de health check en projects
- existe tabla run_history
- la app migra sin romper datos anteriores
- tests o verificaciones de migración incluidos donde sea viable

Validación:
- cargo test
- cargo clippy --all-targets --all-features -- -D warnings

Entrega:
- resumen
- archivos modificados
- estrategia de compatibilidad adoptada
```

#### Prompt S4-03
```text
Objetivo:
Registrar historial persistido de ejecuciones desde el runtime existente de Sprint 3.

Contexto:
Cada arranque, parada, fallo o salida inesperada debe dejar rastro útil en run_history.

Puedes tocar:
- src-tauri/src/runtime/*
- src-tauri/src/persistence/*
- src-tauri/src/models/*

No toques:
- UI
- detection

Criterios de aceptación:
- se crea un registro al arrancar o al menos al consolidar una ejecución
- se actualiza ended_at, exit_code, final_runtime_status y stop_reason al terminar
- fallo de arranque y salida inesperada quedan reflejados
- tests básicos de persistencia incluidos

Validación:
- cargo test
- cargo clippy --all-targets --all-features -- -D warnings

Entrega:
- resumen
- archivos modificados
- decisiones sobre momento de escritura del historial
```

#### Prompt S4-04
```text
Objetivo:
Implementar un HealthManager backend con checks HTTP/TCP, polling controlado, grace period y thresholds.

Contexto:
La app ya sabe ejecutar procesos. Ahora debe evaluar si el servicio está sano mientras corre.

Puedes tocar:
- src-tauri/src/runtime/*
- src-tauri/src/models/*
- src-tauri/src/utils/*

No toques:
- UI
- detection
- contracts frontend salvo tipos compartidos

Criterios de aceptación:
- soporta checks HTTP y TCP
- no evalúa durante grace period
- respeta intervalMs y timeoutMs
- aplica success/failure thresholds
- expone ProjectHealthState por proyecto
- se detiene correctamente cuando el proceso termina

Validación:
- cargo test
- cargo clippy --all-targets --all-features -- -D warnings

Entrega:
- resumen
- archivos modificados
- límites y decisiones de polling
```

#### Prompt S4-05
```text
Objetivo:
Exponer comandos y eventos Tauri de Sprint 4 para salud e historial: get_project_health_status, refresh_project_health, update_project_health_check, list_project_run_history y list_workspace_run_history.

Contexto:
El frontend necesita observar, configurar y consultar la nueva capa de observabilidad.

Puedes tocar:
- src-tauri/src/commands/*
- src-tauri/src/runtime/*
- src-tauri/src/persistence/*
- src/types/*

No toques:
- detection
- UI visual compleja

Criterios de aceptación:
- comandos registrados en Tauri
- eventos de health disponibles para el frontend
- contratos TS actualizados
- errores manejados de forma consistente

Validación:
- cargo test
- npm run typecheck

Entrega:
- resumen
- archivos modificados
- contratos expuestos
```

#### Prompt S4-06
```text
Objetivo:
Implementar store y UI mínima de observabilidad para Sprint 4: badges de salud, filtros por runtime/health, configuración de health check e historial reciente.

Contexto:
El backend ya expone runtime, salud e historial. La UI debe hacerlos útiles sin intentar ser un dashboard de la NASA.

Puedes tocar:
- src/features/health/*
- src/features/history/*
- src/components/health/*
- src/components/history/*
- src/store/*
- src/app/*
- src/types/*

No toques:
- detection
- lógica de ejecución backend

Criterios de aceptación:
- el árbol puede filtrarse por runtime y health status
- cada proyecto muestra badge de salud
- el detalle permite editar configuración de health check
- el historial reciente se muestra de forma legible
- los listeners se limpian correctamente

Validación:
- npm run typecheck
- npm run lint
- npm run test:run

Entrega:
- resumen
- archivos modificados
- limitaciones conocidas de UI
```

### Salida esperada del Sprint 4
Al cerrar S4 la aplicación dará un salto de madurez: dejará de limitarse a arrancar y parar procesos para empezar a comportarse como un orquestador que entiende si los servicios viven, si responden y qué ha pasado en sus últimas ejecuciones.

---

## 14. Roadmap posterior
### Sprint 5+
Posibles mejoras:
- dependencias entre proyectos
- arranque secuencial/paralelo configurable
- apertura en IDE
- importación masiva
- soporte serio de monorepos
- soporte Docker Compose
- perfiles por entorno
- restart automático y políticas de retry
- alertas y exportación de historial

---

## 15. Decisiones vigentes a respetar
- La app se desarrolla desde VS Code.
- Se usará Codex como apoyo de implementación.
- Tauri 2 es la base del escritorio.
- SQLite es la persistencia local.
- El documento maestro de especificación será este fichero Markdown.
- A partir de ahora las actualizaciones deben consolidarse aquí en vez de duplicarse en el chat.

---

## 16. Regla de mantenimiento del documento
Cuando un sprint avance o cambie una decisión de arquitectura:
1. actualizar este documento,
2. marcar el estado del sprint,
3. mover decisiones cerradas a su sección correspondiente,
4. evitar duplicar bloques antiguos si ya están consolidados.

Este archivo debe mantenerse compacto, acumulativo y orientado a ejecución.
