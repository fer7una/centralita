use serde::{Deserialize, Serialize};

use crate::models::{GroupNode, ProjectNode, Workspace};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTree {
    pub workspace: Workspace,
    #[serde(default)]
    pub groups: Vec<GroupTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupTreeNode {
    #[serde(flatten)]
    pub group: GroupNode,
    #[serde(default)]
    pub groups: Vec<GroupTreeNode>,
    #[serde(default)]
    pub projects: Vec<ProjectNode>,
}
