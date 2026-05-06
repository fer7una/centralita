import { useState } from 'react'
import type { GroupTreeNode } from '../types'
import { BlockedActionButton } from './BlockedActionButton'
import { ModalFrame } from './ModalFrame'

type WorkspaceTreeViewProps = {
  groups: GroupTreeNode[]
  onDeleteGroup: (groupId: string) => void | Promise<void>
  onDeleteProject: (projectId: string) => void | Promise<void>
  onRenameGroup: (groupId: string, name: string) => void | Promise<void>
  onRenameProject: (projectId: string, name: string) => void | Promise<void>
}

export function WorkspaceTreeView({
  groups,
  onDeleteGroup,
  onDeleteProject,
  onRenameGroup,
  onRenameProject,
}: WorkspaceTreeViewProps) {
  const [renameModal, setRenameModal] = useState<{
    id: string
    kind: 'group' | 'project'
    value: string
  } | null>(null)
  const [deleteModal, setDeleteModal] = useState<{
    id: string
    kind: 'group' | 'project'
    name: string
  } | null>(null)

  async function handleRenameSubmit() {
    if (!renameModal) {
      return
    }

    const nextValue = renameModal.value.trim()
    if (!nextValue) {
      return
    }

    const currentModal = renameModal
    setRenameModal(null)

    if (currentModal.kind === 'group') {
      await onRenameGroup(currentModal.id, nextValue)
      return
    }

    await onRenameProject(currentModal.id, nextValue)
  }

  async function handleDeleteConfirm() {
    if (!deleteModal) {
      return
    }

    const currentModal = deleteModal
    setDeleteModal(null)

    if (currentModal.kind === 'group') {
      await onDeleteGroup(currentModal.id)
      return
    }

    await onDeleteProject(currentModal.id)
  }

  if (groups.length === 0) {
    return (
      <p className="empty-state">
        Todavía no hay grupos ni proyectos en este workspace.
      </p>
    )
  }

  return (
    <>
      <ul className="tree-list">
        {groups.map((group) => (
          <TreeGroup
            group={group}
            key={group.id}
            onRequestDelete={(target) => setDeleteModal(target)}
            onRequestRename={(target) => setRenameModal(target)}
          />
        ))}
      </ul>

      {renameModal ? (
        <ModalFrame
          ariaLabel={
            renameModal.kind === 'group'
              ? 'Renombrar grupo'
              : 'Renombrar proyecto'
          }
          closeLabel="Cerrar renombrado"
          eyebrow="Edición"
          onClose={() => setRenameModal(null)}
          title={
            renameModal.kind === 'group'
              ? 'Renombrar grupo'
              : 'Renombrar proyecto'
          }
        >
          <form
            className="stack"
            onSubmit={(event) => {
              event.preventDefault()
              void handleRenameSubmit()
            }}
          >
            <label className="field">
              <span>Nombre</span>
              <input
                autoFocus
                onChange={(event) =>
                  setRenameModal((current) =>
                    current
                      ? { ...current, value: event.target.value }
                      : current,
                  )
                }
                value={renameModal.value}
              />
            </label>
            <div className="modal-actions">
              <button
                className="secondary"
                onClick={() => setRenameModal(null)}
                type="button"
              >
                Cancelar
              </button>
              <BlockedActionButton
                blockedReason={
                  renameModal.value.trim()
                    ? undefined
                    : 'Escribe un nombre para guardar.'
                }
                disabled={!renameModal.value.trim()}
                type="submit"
              >
                Guardar nombre
              </BlockedActionButton>
            </div>
          </form>
        </ModalFrame>
      ) : null}

      {deleteModal ? (
        <ModalFrame
          ariaLabel={
            deleteModal.kind === 'group'
              ? 'Eliminar grupo'
              : 'Eliminar proyecto'
          }
          closeLabel="Cerrar confirmación"
          eyebrow="Confirmación"
          onClose={() => setDeleteModal(null)}
          title={
            deleteModal.kind === 'group'
              ? 'Eliminar grupo'
              : 'Eliminar proyecto'
          }
        >
          <div className="stack">
            <p className="muted">
              {deleteModal.kind === 'group'
                ? `Se eliminara el grupo "${deleteModal.name}" con todo su contenido.`
                : `Se eliminara el proyecto "${deleteModal.name}".`}
            </p>
            <div className="modal-actions">
              <button
                className="secondary"
                onClick={() => setDeleteModal(null)}
                type="button"
              >
                Cancelar
              </button>
              <button
                className="danger"
                onClick={() => void handleDeleteConfirm()}
                type="button"
              >
                {deleteModal.kind === 'group'
                  ? 'Eliminar grupo'
                  : 'Eliminar proyecto'}
              </button>
            </div>
          </div>
        </ModalFrame>
      ) : null}
    </>
  )
}

type TreeGroupProps = {
  group: GroupTreeNode
  onRequestDelete: (target: {
    id: string
    kind: 'group' | 'project'
    name: string
  }) => void
  onRequestRename: (target: {
    id: string
    kind: 'group' | 'project'
    value: string
  }) => void
}

function TreeGroup({
  group,
  onRequestDelete,
  onRequestRename,
}: TreeGroupProps) {
  return (
    <li className="tree-group">
      <div className="tree-row">
        <div className="tree-label">
          <div>
            <strong>{group.name}</strong>
            <p>Grupo · sortOrder {group.sortOrder}</p>
          </div>
        </div>

        <div className="tree-actions">
          <button
            onClick={() =>
              onRequestRename({
                id: group.id,
                kind: 'group',
                value: group.name,
              })
            }
            type="button"
          >
            Renombrar
          </button>
          <button
            className="danger"
            onClick={() =>
              onRequestDelete({ id: group.id, kind: 'group', name: group.name })
            }
            type="button"
          >
            Borrar
          </button>
        </div>
      </div>

      {group.projects.length > 0 ? (
        <ul className="project-list">
          {group.projects.map((project) => (
            <li className="project-item" key={project.id}>
              <div>
                <strong>{project.name}</strong>
                <p>{project.path}</p>
              </div>
              <div className="tree-actions">
                <button
                  onClick={() =>
                    onRequestRename({
                      id: project.id,
                      kind: 'project',
                      value: project.name,
                    })
                  }
                  type="button"
                >
                  Renombrar
                </button>
                <button
                  className="danger"
                  onClick={() =>
                    onRequestDelete({
                      id: project.id,
                      kind: 'project',
                      name: project.name,
                    })
                  }
                  type="button"
                >
                  Borrar
                </button>
              </div>
            </li>
          ))}
        </ul>
      ) : null}

      {group.groups.length > 0 ? (
        <ul className="tree-list nested">
          {group.groups.map((childGroup) => (
            <TreeGroup
              group={childGroup}
              key={childGroup.id}
              onRequestDelete={onRequestDelete}
              onRequestRename={onRequestRename}
            />
          ))}
        </ul>
      ) : null}
    </li>
  )
}
