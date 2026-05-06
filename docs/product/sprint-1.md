# Sprint 1

Estado: cerrado.

Sprint 1 consolido el nucleo funcional del workspace: modelo de dominio base, persistencia local y operaciones minimas para gestionar workspaces, grupos, subgrupos y proyectos manuales.

Entregables consolidados:
- creacion y listado de workspaces;
- creacion de grupos raiz y subgrupos;
- alta manual de proyectos;
- edicion basica de nombre, color y orden;
- borrado de grupos y proyectos con borrado descendente en grupos;
- recuperacion del estado persistido tras reinicio.

Contratos minimos cerrados en este sprint:
- `create_workspace`;
- `list_workspaces`;
- `get_workspace_tree`;
- `rename_workspace`;
- `delete_workspace`;
- `create_group`;
- `update_group`;
- `delete_group`;
- `create_project`;
- `update_project`;
- `delete_project`.

Referencias vigentes:
- especificacion y estado global: `docs/orquestador-especificacion-maestra.md`;
- reglas operativas y validaciones obligatorias: `AGENTS.md`.

Este archivo se mantiene como referencia historica del cierre del sprint 1. El detalle normativo y la definicion vigente del producto permanecen centralizados en la especificacion maestra.
