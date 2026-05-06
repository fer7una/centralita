# Sprint 4

Estado: completado.

Sprint 4 cubre la observabilidad operativa del runtime. El objetivo es extender la ejecucion local del Sprint 3 con health checks, historial persistido y filtros utiles para distinguir entre proceso vivo, servicio sano y ejecuciones recientes.

## Estado actual

Avance confirmado:
- `Tarea 1`: completada;
- `Tarea 2`: completada;
- `Tarea 3`: completada;
- `Tarea 4`: completada;
- `Tarea 5`: completada;
- `Tarea 6`: completada.

Resultado final del sprint:
- health checks HTTP y TCP persistidos por proyecto;
- polling de salud en backend con `intervalMs`, `timeoutMs`, `gracePeriodMs` y thresholds;
- estados `HealthStatus` y `ProjectHealthState` disponibles en backend y frontend;
- historial persistido de ejecuciones consultable por proyecto y workspace;
- comandos y eventos Tauri de observabilidad expuestos para salud e historial;
- UI minima con badges de salud, filtros por runtime y health, edicion de health check e historial reciente;
- compatibilidad mantenida con el runtime, logs y acciones de Sprint 3.

## Objetivo operativo

Al cerrar Sprint 4 debe ser posible:
- definir un health check opcional por proyecto;
- evaluar automaticamente la salud de proyectos en ejecucion;
- distinguir entre proceso corriendo y servicio saludable;
- persistir historial basico de arranques, paradas, fallos y codigos de salida;
- consultar ejecuciones recientes por proyecto y por workspace;
- filtrar el arbol por runtime status y health status;
- ver resumen agregado de salud por grupo y workspace;
- refrescar manualmente el estado de salud sin romper el flujo de Sprint 3.

## Alcance cerrado

Incluye:
- modelo de health checks;
- polling de salud en background para proyectos `RUNNING`;
- estados extendidos de runtime y salud;
- historial persistido de ejecuciones;
- filtros de observabilidad en frontend;
- resumen agregado por grupo y workspace;
- UI minima de configuracion y lectura de salud;
- tests de polling, persistencia y agregacion.

No incluye:
- dependencias entre proyectos;
- orden configurable de arranque;
- auto-restart por `UNHEALTHY`;
- Docker Compose;
- perfiles por entorno;
- apertura en IDE;
- importacion masiva;
- soporte avanzado de monorepos.

## Backlog operativo propuesto

### Tarea 1. Definir el modelo de salud e historial
Objetivo:
fijar el contrato compartido de salud e historial para backend y frontend.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- `HealthStatus`;
- `HealthCheckConfig` para HTTP y TCP;
- `ProjectHealthState`;
- `RunHistoryEntry`;
- thresholds, timestamps y errores utiles tipados de forma consistente.

### Tarea 2. Persistir configuracion de health e historial
Objetivo:
anadir persistencia compatible con datos de S1-S3.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- campos de health check consolidados en `projects`;
- tabla `run_history`;
- migraciones compatibles con datos previos;
- verificaciones basicas de migracion donde aplica.

### Tarea 3. Registrar historial de ejecuciones desde el runtime
Objetivo:
dejar rastro util de arranques, paradas, fallos y salidas inesperadas.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- escritura y cierre de entradas de `run_history`;
- persistencia de `endedAt`, `exitCode`, `finalRuntimeStatus` y `stopReason`;
- reflejo de fallo de arranque y salida inesperada;
- cobertura basica de persistencia de historial.

### Tarea 4. Implementar `HealthManager` y polling controlado
Objetivo:
evaluar si un servicio esta sano mientras el proceso sigue corriendo.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- checks HTTP y TCP;
- respeto de `gracePeriodMs`, `intervalMs` y `timeoutMs`;
- thresholds de exito y fallo;
- `ProjectHealthState` por proyecto;
- parada del polling cuando el proceso termina.

### Tarea 5. Exponer comandos y eventos Tauri de observabilidad
Objetivo:
hacer consumible la capa de salud e historial desde frontend.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- `get_project_health_status`;
- `refresh_project_health`;
- `update_project_health_check`;
- `list_project_run_history`;
- `list_workspace_run_history`;
- eventos de salud disponibles para React;
- contratos TypeScript actualizados.

### Tarea 6. Integrar store y UI minima de observabilidad
Objetivo:
volver utiles la salud y el historial sin convertir la app en un dashboard pesado.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- badges de salud por proyecto;
- filtros por runtime y `HealthStatus`;
- edicion minima de health check;
- historial reciente legible;
- limpieza correcta de listeners y sincronizacion basica de estado.

## Decisiones activas

- proceso vivo no implica servicio saludable;
- un health check sigue siendo opcional para ejecutar un proyecto;
- el polling vive en Rust y no en React;
- solo se evalua salud de proyectos en `RUNNING`;
- no se lanzan checks concurrentes del mismo proyecto;
- `UNSUPPORTED` se usa cuando no hay health check configurado;
- `UNKNOWN` se reserva para checks configurados aun no evaluados o en estado neutro;
- el historial sirve para diagnostico, no como auditoria completa.

## Reglas operativas consolidadas

- HTTP es saludable si responde dentro de `timeoutMs`, devuelve un codigo esperado y cumple `containsText` cuando existe.
- TCP es saludable si `host:port` acepta conexion dentro de `timeoutMs`.
- El proceso entra en grace period al arrancar antes de iniciar polling.
- Si el proceso termina, el polling se detiene y la salud vuelve al estado neutro definido.
- La agregacion de salud prioriza `UNHEALTHY`, luego `CHECKING`, luego `HEALTHY` y finalmente `UNSUPPORTED` cuando nadie soporta checks.

## Limitaciones asumidas al cerrar

- no existe restart automatico por `UNHEALTHY`;
- no hay politicas avanzadas de retry ni alertas externas;
- el historial sigue siendo funcional y diagnostico, no exportable ni analitico;
- no se introducen dashboards complejos ni metricas avanzadas;
- la compatibilidad con Sprint 3 se prioriza sobre automatismos agresivos.

## Referencias vigentes

- especificacion maestra: `docs/orquestador-especificacion-maestra.md`;
- reglas operativas y formato de entrega: `AGENTS.md`.
