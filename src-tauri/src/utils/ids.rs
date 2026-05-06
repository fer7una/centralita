use uuid::Uuid;

pub fn workspace_id() -> String {
    prefixed_id("workspace")
}

pub fn group_id() -> String {
    prefixed_id("group")
}

pub fn project_id() -> String {
    prefixed_id("project")
}

pub fn run_history_id() -> String {
    prefixed_id("run")
}

fn prefixed_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7())
}
