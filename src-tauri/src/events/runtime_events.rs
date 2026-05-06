use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::models::{
    ProjectHealthState, RunHistoryEntry, RuntimeLogLine, RuntimeProcessErrorEvent,
    RuntimeProcessExitedEvent, RuntimeStatusEvent,
};

pub const STATUS_CHANGED_EVENT: &str = "runtime://status-changed";
pub const LOG_LINE_EVENT: &str = "runtime://log-line";
pub const PROCESS_EXITED_EVENT: &str = "runtime://process-exited";
pub const PROCESS_ERROR_EVENT: &str = "runtime://process-error";
pub const HEALTH_CHANGED_EVENT: &str = "runtime://health-changed";
pub const HISTORY_APPENDED_EVENT: &str = "runtime://history-appended";

pub type RuntimeEventEmitter = Arc<dyn Fn(&'static str, serde_json::Value) + Send + Sync>;

pub fn create_runtime_event_emitter<R: Runtime>(app: AppHandle<R>) -> RuntimeEventEmitter {
    Arc::new(move |event_name, payload| {
        if let Err(error) = app.emit(event_name, payload) {
            log::error!("Failed to emit runtime event '{event_name}': {error}");
        }
    })
}

pub fn noop_runtime_event_emitter() -> RuntimeEventEmitter {
    Arc::new(|_, _| {})
}

pub fn emit_status_changed(emitter: &RuntimeEventEmitter, payload: &RuntimeStatusEvent) {
    emit(emitter, STATUS_CHANGED_EVENT, payload);
}

pub fn emit_log_line(emitter: &RuntimeEventEmitter, payload: &RuntimeLogLine) {
    emit(emitter, LOG_LINE_EVENT, payload);
}

pub fn emit_process_exited(emitter: &RuntimeEventEmitter, payload: &RuntimeProcessExitedEvent) {
    emit(emitter, PROCESS_EXITED_EVENT, payload);
}

pub fn emit_process_error(emitter: &RuntimeEventEmitter, payload: &RuntimeProcessErrorEvent) {
    emit(emitter, PROCESS_ERROR_EVENT, payload);
}

pub fn emit_health_changed(emitter: &RuntimeEventEmitter, payload: &ProjectHealthState) {
    emit(emitter, HEALTH_CHANGED_EVENT, payload);
}

pub fn emit_history_appended(emitter: &RuntimeEventEmitter, payload: &RunHistoryEntry) {
    emit(emitter, HISTORY_APPENDED_EVENT, payload);
}

fn emit<T>(emitter: &RuntimeEventEmitter, event_name: &'static str, payload: &T)
where
    T: Serialize,
{
    match serde_json::to_value(payload) {
        Ok(value) => emitter(event_name, value),
        Err(error) => {
            log::error!("Failed to serialize runtime event '{event_name}': {error}");
        }
    }
}
