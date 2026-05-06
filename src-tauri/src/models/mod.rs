mod common;
pub mod detection;
pub mod group_node;
pub mod health;
pub mod project_node;
pub mod run_history;
pub mod runtime;
pub mod workspace;
pub mod workspace_tree;

pub use common::{
    CommandArgs, DetectedProjectType, DetectionEvidenceKind, EntityId, EnvironmentVariables,
    IsoDateTime, ProjectPackageManager,
};
pub use detection::{CommandValidation, DetectionEvidence, DetectionResult, DetectionWarning};
pub use group_node::GroupNode;
pub use health::{
    HealthCheckConfig, HealthCheckKind, HealthStatus, HttpHealthCheckConfig, ProjectHealthState,
    TcpHealthCheckConfig, WorkspaceHealthStatusCounts, WorkspaceObservabilitySummary,
    WorkspaceRuntimeStatusCounts,
};
pub use project_node::ProjectNode;
pub use run_history::RunHistoryEntry;
pub use runtime::{
    ProcessRuntimeState, RunRequest, RuntimeBulkFailure, RuntimeBulkOperationResult, RuntimeEvent,
    RuntimeLogLine, RuntimeLogStream, RuntimeOperationScope, RuntimeProcessErrorEvent,
    RuntimeProcessExitedEvent, RuntimeStatus, RuntimeStatusEvent, WorkspaceRuntimeStatus,
};
pub use workspace::Workspace;
pub use workspace_tree::{GroupTreeNode, WorkspaceTree};
