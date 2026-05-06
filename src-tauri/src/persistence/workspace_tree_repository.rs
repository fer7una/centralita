use std::collections::HashMap;

use crate::{
    models::{EntityId, GroupNode, GroupTreeNode, ProjectNode, Workspace, WorkspaceTree},
    persistence::{
        AppDatabase, GroupRepository, PersistenceResult, ProjectRepository, WorkspaceRepository,
    },
};

#[derive(Debug, Clone)]
pub struct WorkspaceTreeRepository {
    database: AppDatabase,
}

impl WorkspaceTreeRepository {
    pub fn new(database: AppDatabase) -> Self {
        Self { database }
    }

    pub fn get(&self, workspace_id: &EntityId) -> PersistenceResult<Option<WorkspaceTree>> {
        let workspace_repository = WorkspaceRepository::new(self.database.clone());
        let group_repository = GroupRepository::new(self.database.clone());
        let project_repository = ProjectRepository::new(self.database.clone());
        let Some(workspace) = workspace_repository.find_by_id(workspace_id)? else {
            return Ok(None);
        };

        let groups = group_repository.list_by_workspace(workspace_id)?;
        let projects = project_repository.list_by_workspace(workspace_id)?;

        Ok(Some(build_workspace_tree(workspace, groups, projects)))
    }
}

pub(crate) fn build_workspace_tree(
    workspace: Workspace,
    mut groups: Vec<GroupNode>,
    mut projects: Vec<ProjectNode>,
) -> WorkspaceTree {
    groups.sort_by(|left, right| {
        left.parent_group_id
            .cmp(&right.parent_group_id)
            .then(left.sort_order.cmp(&right.sort_order))
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
    projects.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));

    let mut groups_by_parent: HashMap<Option<EntityId>, Vec<GroupNode>> = HashMap::new();
    for group in groups {
        groups_by_parent
            .entry(group.parent_group_id.clone())
            .or_default()
            .push(group);
    }

    let mut projects_by_group: HashMap<EntityId, Vec<ProjectNode>> = HashMap::new();
    for project in projects {
        projects_by_group
            .entry(project.group_id.clone())
            .or_default()
            .push(project);
    }

    WorkspaceTree {
        workspace,
        groups: build_group_children(None, &groups_by_parent, &projects_by_group),
    }
}

fn build_group_children(
    parent_group_id: Option<&str>,
    groups_by_parent: &HashMap<Option<EntityId>, Vec<GroupNode>>,
    projects_by_group: &HashMap<EntityId, Vec<ProjectNode>>,
) -> Vec<GroupTreeNode> {
    groups_by_parent
        .get(&parent_group_id.map(ToOwned::to_owned))
        .map(|groups| {
            groups
                .iter()
                .map(|group| GroupTreeNode {
                    group: group.clone(),
                    groups: build_group_children(
                        Some(&group.id),
                        groups_by_parent,
                        projects_by_group,
                    ),
                    projects: projects_by_group
                        .get(&group.id)
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::{
        models::{DetectedProjectType, GroupNode, ProjectNode, Workspace},
        persistence::{
            test_utils::TestDatabase, GroupRepository, ProjectRepository, WorkspaceRepository,
            WorkspaceTreeRepository,
        },
    };

    #[test]
    fn reconstructs_workspace_tree_with_nested_groups_and_order() {
        let test_database = TestDatabase::new("workspace-tree-repository");
        let workspace_repository = WorkspaceRepository::new(test_database.database());
        let group_repository = GroupRepository::new(test_database.database());
        let project_repository = ProjectRepository::new(test_database.database());
        let tree_repository = WorkspaceTreeRepository::new(test_database.database());
        let workspace = Workspace {
            id: "workspace-main".into(),
            name: "Centralita".into(),
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let backend_group = GroupNode {
            id: "group-backend".into(),
            workspace_id: workspace.id.clone(),
            parent_group_id: None,
            name: "Backend".into(),
            color: "#0f172a".into(),
            sort_order: 20,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let frontend_group = GroupNode {
            id: "group-frontend".into(),
            workspace_id: workspace.id.clone(),
            parent_group_id: None,
            name: "Frontend".into(),
            color: "#2563eb".into(),
            sort_order: 10,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let ui_group = GroupNode {
            id: "group-ui".into(),
            workspace_id: workspace.id.clone(),
            parent_group_id: Some(frontend_group.id.clone()),
            name: "UI".into(),
            color: "#7c3aed".into(),
            sort_order: 5,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let api_project = ProjectNode {
            id: "project-api".into(),
            workspace_id: workspace.id.clone(),
            group_id: backend_group.id.clone(),
            name: "API".into(),
            path: r"C:\Projects\api".into(),
            detected_type: Some(DetectedProjectType::NodeGeneric),
            color: None,
            package_manager: None,
            executable: None,
            command: Some("npm start".into()),
            args: None,
            env: None,
            working_dir: Some(r"C:\Projects\api".into()),
            detection_confidence: None,
            detection_evidence: None,
            warnings: None,
            health_check: None,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };
        let ui_project = ProjectNode {
            id: "project-ui".into(),
            workspace_id: workspace.id.clone(),
            group_id: ui_group.id.clone(),
            name: "UI".into(),
            path: r"C:\Projects\ui".into(),
            detected_type: Some(DetectedProjectType::ReactVite),
            color: None,
            package_manager: None,
            executable: None,
            command: Some("npm run dev".into()),
            args: None,
            env: None,
            working_dir: Some(r"C:\Projects\ui".into()),
            detection_confidence: None,
            detection_evidence: None,
            warnings: None,
            health_check: None,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };

        workspace_repository
            .create(&workspace)
            .expect("workspace should exist");
        group_repository
            .create(&backend_group)
            .expect("backend group should exist");
        group_repository
            .create(&frontend_group)
            .expect("frontend group should exist");
        group_repository
            .create(&ui_group)
            .expect("ui subgroup should exist");
        project_repository
            .create(&api_project)
            .expect("api project should exist");
        project_repository
            .create(&ui_project)
            .expect("ui project should exist");

        let tree = tree_repository
            .get(&workspace.id)
            .expect("tree should load")
            .expect("workspace tree should exist");

        assert_eq!(tree.workspace, workspace);
        assert_eq!(tree.groups.len(), 2);
        assert_eq!(tree.groups[0].group.id, frontend_group.id);
        assert_eq!(tree.groups[1].group.id, backend_group.id);
        assert_eq!(tree.groups[0].groups[0].group.id, ui_group.id);
        assert_eq!(tree.groups[0].groups[0].projects, vec![ui_project]);
        assert_eq!(tree.groups[1].projects, vec![api_project]);
    }
}
