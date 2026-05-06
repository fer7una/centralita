use serde::{Deserialize, Serialize};

use crate::models::{EntityId, IsoDateTime};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: EntityId,
    pub name: String,
    pub created_at: IsoDateTime,
    pub updated_at: IsoDateTime,
}

#[cfg(test)]
mod tests {
    use super::Workspace;

    #[test]
    fn round_trips_plain_workspace_shape() {
        let workspace = Workspace {
            id: "workspace-main".into(),
            name: "Centralita".into(),
            created_at: "2026-04-14T09:00:00Z".into(),
            updated_at: "2026-04-14T09:00:00Z".into(),
        };

        let json = serde_json::to_value(&workspace).expect("workspace should serialize");
        let decoded: Workspace =
            serde_json::from_value(json.clone()).expect("workspace should deserialize");

        assert_eq!(decoded, workspace);
        assert!(json.get("rootGroupIds").is_none());
        assert!(json.get("rootProjectIds").is_none());
    }
}
