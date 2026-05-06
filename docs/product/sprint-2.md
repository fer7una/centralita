# Sprint 2

Estado: cerrado.

Sprint 2 cubre la importacion asistida de proyectos desde carpeta local. El objetivo no es ejecutar procesos todavia, sino analizar una ruta, detectar el tipo de proyecto, proponer una configuracion inicial editable y persistirla en el workspace.

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
- `Tarea 9`: completada;
- `Tarea 10`: completada.

Bloques ejecutados:
- `bloque A`: completado;
- `bloque B`: completado.
- `bloque C`: completado;
- `bloque D`: completado.
- `bloque E`: completado;
- `bloque F`: completado;
- `bloque G`: completado.

Resultado del sprint:
- analisis de carpeta local disponible desde Tauri;
- deteccion Node y Java disponible con evidencias y warnings;
- revision previa al guardado disponible desde la UI;
- persistencia del proyecto revisado disponible desde el flujo de importacion.

## Punto de partida del repositorio

Estado actual confirmado:
- existe CRUD manual de workspaces, grupos, subgrupos y proyectos;
- existe persistencia SQLite para `projects`, `groups` y `workspaces`;
- existe UI minima para alta manual de proyectos;
- existe modelo base de deteccion y scanner seguro en backend;
- existe comando Tauri `analyze_project_folder`;
- existen reglas de deteccion Node y Java sobre el scanner.

Esto implica que Sprint 2 debe abrir una nueva vertical funcional entre persistencia, backend de deteccion y flujo UI de revision previa.

## Backlog operativo propuesto

### Tarea 1. Ampliar el modelo de dominio y la persistencia para deteccion
Objetivo:
dejar preparado `ProjectNode` para guardar la configuracion revisada del analisis.

Alcance:
- ampliar tipos frontend y modelos Rust;
- introducir migracion SQLite para nuevos campos;
- adaptar repositorios y serializacion.

Campos a incorporar:
- `packageManager` nullable;
- `executable` nullable;
- `detectionConfidence` nullable;
- `detectionEvidence` nullable serializado;
- `warnings` nullable serializado.

Archivos previsibles:
- `src/types/domain.ts`;
- `src-tauri/src/models/project_node.rs`;
- `src-tauri/src/persistence/migrations.rs`;
- `src-tauri/src/persistence/project_repository.rs`;
- contratos de `src-tauri/src/commands/mod.rs`.

Criterio de cierre:
- el esquema migra sin perder compatibilidad con datos de Sprint 1;
- `create_project` y `update_project` aceptan y devuelven la nueva forma;
- los tests de repositorio siguen cubriendo lectura y escritura.

Estado actual:
- completada.

### Tarea 2. Definir el modelo de deteccion y sus DTOs
Objetivo:
crear el contrato estable que va a devolver el backend antes de persistir un proyecto.

Alcance:
- tipos `DetectionResult`, `DetectionEvidence` y warning;
- enumeraciones de tipos detectables y package managers;
- conversiones seguras entre backend y frontend.

Decisiones minimas:
- separar `detectedType` de la configuracion persistida;
- mantener `commandPreview` como salida derivada para UX, no como unico dato fuente;
- conservar `editableFields` en el mismo DTO para no duplicar mapping en frontend.

Archivos previsibles:
- `src/types/domain.ts`;
- `src-tauri/src/models/`;
- `src/features/workspace/api.ts`.

Criterio de cierre:
- existe un DTO unico y coherente para analizar y revisar;
- el contrato cubre confianza, evidencias, warnings y configuracion editable;
- el tipado soporta `Next.js`, `Express`, `Java JAR` y `Custom`.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- tipado compartido de `DetectionResult`, `DetectionEvidence`, warnings y package manager;
- comando Tauri `analyze_project_folder`;
- funcion frontend `analyzeProjectFolder`;
- analizador backend que ya devuelve resultados reales sobre el scanner.

### Tarea 3. Implementar el scanner seguro de carpetas
Objetivo:
inspeccionar una ruta local sin ejecutar nada ni recorrer contenido no relevante.

Alcance:
- validacion de ruta;
- lectura limitada de directorios y ficheros;
- exclusiones por defecto para carpetas pesadas;
- deteccion de errores de permisos y rutas invalidas.

Restricciones obligatorias:
- no ejecutar binarios ni scripts;
- no modificar el filesystem;
- ignorar `node_modules`, `target`, `build`, `.git`, `.idea`, `dist`, `.next`;
- limitar lectura de ficheros grandes.

Archivos previsibles:
- `src-tauri/src/detection/scan/`;
- `src-tauri/src/utils/`;
- tests con fixtures.

Criterio de cierre:
- el scanner devuelve un snapshot suficiente para reglas de deteccion;
- maneja carpeta vacia, permisos insuficientes y ficheros invalidos;
- queda cubierto por tests de unidad con fixtures.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- modulo `src-tauri/src/detection/scan.rs` con snapshot reutilizable;
- exclusiones por defecto para carpetas pesadas;
- limite de profundidad, numero de entradas y tamano de lectura;
- warnings para symlinks, lectura fallida y ficheros grandes;
- fixtures Node y Java para bloques posteriores.

### Tarea 4. Implementar reglas Node y JS/TS
Objetivo:
detectar correctamente `Node generic`, `Vite`, `React + Vite`, `Next.js` y `Express`.

Alcance:
- parseo defensivo de `package.json`;
- inferencia de `packageManager` por lockfile;
- reconocimiento de scripts y dependencias clave;
- warnings para monorepo simple o scripts insuficientes.

Prioridades de deteccion:
- `React + Vite` por encima de `Vite`;
- `Vite` por encima de `Node generic`;
- `Next.js` y `Express` no deben degradarse a `Node generic` cuando hay evidencia fuerte.

Archivos previsibles:
- `src-tauri/src/detection/node/`;
- fixtures Node en `src-tauri`;
- tipos compartidos si procede.

Criterio de cierre:
- se detectan correctamente casos comunes con score explicable;
- se infiere `npm`, `pnpm` o `yarn` segun lockfile;
- los tests cubren `package.json` invalido y proyecto sin scripts utiles.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- deteccion de `Node generic`, `Vite`, `React + Vite`, `Next.js` y `Express`;
- inferencia de `npm`, `pnpm` y `yarn`;
- warnings para `package.json` invalido, monorepo simple y ausencia de script util;
- tests con fixtures Node.

### Tarea 5. Implementar reglas Java
Objetivo:
detectar `Maven`, `Gradle`, `Spring Boot Maven`, `Spring Boot Gradle` y `Java JAR`.

Alcance:
- inspeccion de `pom.xml`, `build.gradle`, `build.gradle.kts`, wrappers y artefactos `.jar`;
- preferencia por wrapper local;
- deteccion especifica de Spring Boot mediante dependencias, plugins o parent.

Archivos previsibles:
- `src-tauri/src/detection/java/`;
- fixtures Java en `src-tauri`.

Criterio de cierre:
- Spring Boot no cae a Maven o Gradle generico si hay evidencia suficiente;
- la inferencia de runner prioriza wrapper local;
- existe fallback razonable a `java -jar` cuando aplique.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- deteccion de `Java Maven`, `Java Gradle`, `Spring Boot Maven`, `Spring Boot Gradle` y `Java JAR`;
- preferencia por wrappers locales, incluidos wrappers tipicos de Windows;
- fallback a `java -jar` cuando no existe una via mejor;
- tests con fixtures Java.

### Tarea 6. Implementar scoring, desempate y resultado final
Objetivo:
resolver candidatos de deteccion de forma explicable y extensible.

Alcance:
- motor de scoring por evidencias;
- reglas de prioridad por especificidad;
- warnings cuando persista ambiguedad.

Salida esperada:
- score `0..1`;
- nivel interpretable de confianza;
- listado de evidencias ponderadas;
- warnings si hay empate, monorepo simple o baja confianza.

Archivos previsibles:
- `src-tauri/src/detection/core/`;
- tests de desempate y ambiguedad.

Criterio de cierre:
- el resultado final es estable y reproducible;
- las evidencias justifican el tipo detectado;
- un caso ambiguo no termina silenciosamente como alta confianza.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- scoring por evidencias con pesos acumulados;
- prioridad por especificidad entre candidatos;
- warnings para ambiguedad, monorepo simple y baja claridad de comando;
- resultado estable y reproducible sobre fixtures del sprint.

### Tarea 7. Exponer comandos Tauri de analisis y alta desde deteccion
Objetivo:
conectar la nueva logica backend con la UI.

Contratos minimos:
- `analyze_project_folder(path)`;
- `create_project_from_detection(input)`.

Opcional si simplifica la UX:
- `recalculate_detection(path)`.

Alcance:
- registrar comandos en `src-tauri/src/lib.rs`;
- validar inputs;
- reutilizar persistencia existente al crear proyecto.

Criterio de cierre:
- la app puede pedir un analisis sin persistir;
- el backend puede persistir un proyecto revisado desde el DTO de deteccion;
- los errores llegan a frontend con mensajes utilizables.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- `analyze_project_folder(path)` expuesto por Tauri;
- `create_project_from_detection(input)` expuesto por Tauri;
- errores del analisis y del guardado propagados al frontend.

### Tarea 8. Adaptar la capa frontend de tipos, API y estado
Objetivo:
preparar React para el flujo de importacion sin mezclar aun demasiada presentacion.

Alcance:
- extender `src/types`;
- anadir funciones API nuevas;
- incorporar estado temporal de analisis y revision;
- mantener compatible el CRUD actual.

Archivos previsibles:
- `src/types/domain.ts`;
- `src/features/workspace/api.ts`;
- `src/store/useWorkspaceStore.ts`.

Criterio de cierre:
- el store soporta solicitar analisis, conservar resultado y confirmar persistencia;
- los errores de analisis no rompen el flujo del workspace activo;
- el alta manual actual sigue funcionando mientras conviva con el nuevo flujo.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- tipos y API frontend adaptados al flujo de deteccion;
- store con estado temporal de analisis;
- integracion de selector de carpeta y persistencia desde deteccion.

### Tarea 9. Implementar la UX de importacion y revision previa
Objetivo:
reemplazar o complementar el alta manual con un flujo guiado de seleccion, analisis, revision y guardado.

Alcance:
- selector de carpeta compatible con Tauri 2;
- modal o panel de revision;
- edicion de nombre, tipo, ejecutable, args, working dir, grupo destino y color;
- visualizacion de confianza, evidencias y warnings.

Restriccion de producto:
- no lanzar el proyecto al importarlo;
- no ocultar que la deteccion puede fallar o ser ambigua.

Archivos previsibles:
- `src/CentralitaApp.tsx` o componentes nuevos en `src/components/`;
- estilos asociados;
- tests de UI.

Criterio de cierre:
- el usuario puede seleccionar carpeta, revisar la propuesta y guardarla;
- el grupo destino puede cambiarse antes de persistir;
- la UI hace visible la confianza y las evidencias principales.

Estado actual:
- completada.

Entrega cerrada en esta tarea:
- selector de carpeta nativo mediante plugin oficial de dialogo de Tauri;
- pantalla de revision previa con nombre, tipo, ejecutable, args, working dir, grupo y color;
- evidencias, warnings y comando resultante visibles antes de guardar.

### Tarea 10. Completar fixtures y validacion integral del sprint
Objetivo:
cerrar Sprint 2 con cobertura minima sobre los casos comunes definidos en la especificacion.

Alcance:
- fixtures representativos Node y Java;
- tests de reglas, scoring y contratos;
- tests frontend del flujo de revision;
- validacion completa del repositorio.

Validaciones obligatorias al cierre:
- `npm run lint`;
- `npm run typecheck`;
- `npm run test:run`;
- `npm run build`;
- `npm run tauri:dev`.

Criterio de cierre:
- los casos comunes Java y Node quedan cubiertos con tests;
- la app arranca en Windows con `npm run tauri:dev`;
- el sprint queda verificable de extremo a extremo.

Estado actual:
- completada.

Validaciones de cierre ejecutadas:
- `cargo test`;
- `npm run validate`;
- `npm run tauri:dev`.

## Orden recomendado de ejecucion

1. Tarea 1.
2. Tarea 2.
3. Tarea 3.
4. Tarea 4.
5. Tarea 5.
6. Tarea 6.
7. Tarea 7.
8. Tarea 8.
9. Tarea 9.
10. Tarea 10.

## Corte recomendado en subtareas de implementacion

Para mantener tareas pequenas, cerradas y revisables, Sprint 2 conviene ejecutarlo al menos en estos bloques:
- bloque A: modelo, migracion y DTOs;
- bloque B: scanner seguro;
- bloque C: deteccion Node;
- bloque D: deteccion Java;
- bloque E: scoring y comandos Tauri;
- bloque F: frontend de revision;
- bloque G: fixtures, tests y validacion final.

## Riesgos principales

- ampliar `ProjectNode` sin migracion compatible puede romper la persistencia existente;
- intentar resolver monorepos complejos en este sprint desviaria alcance;
- mezclar selector de carpeta, deteccion y persistencia en un unico cambio haria dificil revisar y aislar fallos;
- introducir UX compleja antes de estabilizar DTOs y comandos aumentaria retrabajo.

## Resultado esperado del sprint

Al cerrar Sprint 2, Centralita debe poder analizar una carpeta local, proponer una configuracion inicial razonable y persistir el proyecto revisado por el usuario, sin ejecutar procesos ni modificar el filesystem durante la deteccion.
