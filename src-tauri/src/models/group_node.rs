use serde::{Deserialize, Serialize};

use crate::models::{EntityId, IsoDateTime};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupNode {
    pub id: EntityId,
    pub workspace_id: EntityId,
    pub parent_group_id: Option<EntityId>,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub created_at: IsoDateTime,
    pub updated_at: IsoDateTime,
}

#[cfg(test)]
mod tests {
    use super::GroupNode;

    #[test]
    fn serializes_required_persistence_fields() {
        let group = GroupNode {
            id: "group-dev".into(),
            workspace_id: "workspace-main".into(),
            parent_group_id: Some("group-platform".into()),
            name: "Development".into(),
            color: "#3b82f6".into(),
            sort_order: 20,
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };

        let json = serde_json::to_value(&group).expect("group should serialize");
        let decoded: GroupNode =
            serde_json::from_value(json.clone()).expect("group should deserialize");

        assert_eq!(decoded, group);
        assert_eq!(json["sortOrder"], 20);
        assert_eq!(json["color"], "#3b82f6");
        assert!(json.get("childGroupIds").is_none());
        assert!(json.get("projectIds").is_none());
    }
}
