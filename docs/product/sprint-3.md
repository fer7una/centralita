# Sprint 3

Estado: completado.

Sprint 3 cubre el runtime local de procesos. El objetivo es convertir la configuracion persistida del Sprint 2 en ejecucion real controlada por la app, con estado en memoria, captura de logs y eventos backend -> frontend.

## Estado actual

Avance confirmado:
- `Tarea 1`: completada;
- `Tarea 2`: completada;
- `Tarea 3`: completada;
- `Tarea 4`: completada;
- `Tarea 5`: completada;
- `Tarea 6`: completada;
- `Tarea 7`: completada;
- `Tarea 8`: completada;
- `Tarea 9`: completada.

Bloques ejecutados:
- `bloque A`: completado;
- `bloque B`: completado;
- `bloque C`: completado;
- `bloque D`: completado.

Resultado final del sprint:
- modelo runtime compartido disponible en Rust y TypeScript;
- `ProcessManager` backend disponible para start individual desde persistencia;
- captura separada de `stdout` y `stderr` disponible;
- buffer circular de logs por proyecto disponible con limite inicial de `500` lineas;
- eventos Tauri `runtime://status-changed`, `runtime://log-line`, `runtime://process-exited` y `runtime://process-error` disponibles;
- comandos Tauri individuales y masivos disponibles para proyecto, grupo y workspace;
- parada y reinicio individual con estrategia de `kill tree` en Windows mediante `taskkill /T`, con fallback forzado;
- estado runtime consumido desde frontend con panel de detalle, consola de logs y acciones `start/stop/restart`;
- limpieza best-effort de procesos vivos al cerrar la app;
- validacion final ejecutada sobre `cargo test`, `cargo clippy -D warnings`, `npm run typecheck`, `npm run lint`, `npm run test:run`, `npm run build` y arranque de `npm run tauri:dev`.

## Backlog operativo propuesto

### Tarea 1. Definir el modelo runtime y tipos compartidos
Objetivo:
dejar fijado el contrato base del runtime para backend y frontend.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- `RuntimeStatus`, `RunRequest`, `ProcessRuntimeState`, `RuntimeLogLine` y `RuntimeEvent`;
- helper backend para estado inicial `STOPPED`;
- round-trip tests de serializacion.

### Tarea 2. Implementar el `ProcessManager` y el arranque individual
Objetivo:
arrancar un proyecto persistido y mantener su estado en memoria.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- carga de `ProjectNode` desde SQLite;
- validacion de `executable` y `workingDir`;
- prevencion de doble start;
- registro de `pid`, `startedAt`, `status` y `commandPreview`;
- deteccion de fallo de arranque y marcado a `FAILED`.

### Tarea 3. Capturar logs y emitir eventos runtime
Objetivo:
hacer observable la ejecucion en tiempo real sin persistir aun historial.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- captura separada de `stdout` y `stderr`;
- buffer circular por proyecto con limite inicial de `500` lineas;
- emision de eventos Tauri por cambio de estado, linea de log, salida de proceso y error;
- comando `get_project_logs` para consultar logs recientes;
- tests de buffer, captura y emision de eventos.

### Tarea 4. Implementar `stop` y `restart` individual
Objetivo:
detener o reiniciar procesos sin dejar hijos vivos en casos comunes.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- `stop_project` cambia a `STOPPING` y termina en `STOPPED`;
- `restart_project` reutiliza `stop + start` de forma controlada;
- estrategia de `kill tree` en Windows con `taskkill /T` y fallback forzado;
- parada idempotente para proyectos ya detenidos.

### Tarea 5. Exponer comandos Tauri individuales de runtime
Objetivo:
registrar la API minima para control individual desde frontend.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- comandos `start_project`, `stop_project`, `restart_project`, `get_project_runtime_status` y `get_project_logs`;
- registro en `invoke_handler` con `ProcessManager` compartido;
- errores serializados de forma consistente hacia frontend.

### Tarea 6. Implementar estado runtime en frontend y panel de logs
Objetivo:
consumir el runtime backend desde la UI con un primer panel funcional.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- store runtime en frontend con sincronizacion inicial por workspace;
- suscripcion a eventos Tauri y reconciliacion de estado en vivo;
- panel de detalle con estado, PID, comando, ultimo error y logs recientes;
- render de estado runtime en arbol de workspace y seleccion de proyecto.

### Tarea 7. Implementar ejecucion por grupo y workspace
Objetivo:
orquestar acciones masivas reutilizando el runtime individual.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- `start_group`, `stop_group`, `start_workspace` y `stop_workspace`;
- resumen de resultados con proyectos afectados, omitidos y fallos;
- acciones masivas visibles desde la UI principal.

### Tarea 8. Completar pruebas y endurecer casos borde
Objetivo:
cerrar el sprint con cobertura minima sobre arranque, parada, logs y errores.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- tests backend para arranque, doble start, salida espontanea, logs y eventos;
- ajuste de tests frontend para el nuevo runtime store;
- limpieza de warnings `clippy` preexistentes en `detection/scan.rs`.

### Tarea 9. Validacion integral y cierre de sprint
Objetivo:
verificar flujo completo de runtime en Windows y Tauri.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- `npm run tauri:dev` validado en Windows, manteniendose vivo 45 segundos antes de detener el proceso de prueba;
- cierre de app conectado a `shutdown_all()` para detener procesos vivos al salir;
- documentacion del sprint actualizada y consolidada.

## Decisiones activas

- el runtime sigue siendo backend-first: React no ejecuta procesos;
- los logs son memoria temporal, no persistencia;
- el limite inicial del buffer por proyecto queda fijado en `500` lineas;
- los eventos Tauri se emiten por canal especifico y no como stream unico agregado;
- `get_project_logs` se expone antes de la UI para desacoplar observabilidad del panel visual;
- en Windows la parada prioriza `taskkill /T` para reducir procesos hijos huerfanos;
- al cerrar la app se ejecuta `shutdown_all()` como limpieza best-effort.

## Limitaciones asumidas al cerrar

- los logs siguen siendo solo memoria temporal y no se persisten entre sesiones;
- no se implementan health checks ni autorestart en este sprint;
- la estrategia de `kill tree` queda optimizada para Windows y mantiene fallback simple en otros sistemas.
