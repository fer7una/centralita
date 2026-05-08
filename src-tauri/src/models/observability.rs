use serde::{Deserialize, Serialize};

use crate::models::{EntityId, RuntimeStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRuntimeStatusCounts {
    pub stopped: u32,
    pub starting: u32,
    pub running: u32,
    pub stopping: u32,
    pub failed: u32,
}

impl WorkspaceRuntimeStatusCounts {
    pub fn from_statuses<I>(statuses: I) -> Self
    where
        I: IntoIterator<Item = RuntimeStatus>,
    {
        let mut counts = Self {
            stopped: 0,
            starting: 0,
            running: 0,
            stopping: 0,
            failed: 0,
        };

        for status in statuses {
            match status {
                RuntimeStatus::Stopped => counts.stopped += 1,
                RuntimeStatus::Starting => counts.starting += 1,
                RuntimeStatus::Running => counts.running += 1,
                RuntimeStatus::Stopping => counts.stopping += 1,
                RuntimeStatus::Failed => counts.failed += 1,
            }
        }

        counts
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceObservabilitySummary {
    pub workspace_id: EntityId,
    pub total_projects: u32,
    pub runtime_status: RuntimeStatus,
    pub runtime_counts: WorkspaceRuntimeStatusCounts,
}

#[cfg(test)]
mod tests {
    use super::WorkspaceRuntimeStatusCounts;
    use crate::models::RuntimeStatus;

    #[test]
    fn counts_runtime_statuses() {
        let runtime_counts = WorkspaceRuntimeStatusCounts::from_statuses([
            RuntimeStatus::Running,
            RuntimeStatus::Running,
            RuntimeStatus::Failed,
        ]);

        assert_eq!(runtime_counts.running, 2);
        assert_eq!(runtime_counts.failed, 1);
    }
}
