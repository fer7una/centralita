use tauri::Manager;

pub mod commands;
pub mod detection;
pub mod events;
pub mod models;
pub mod persistence;
pub mod runtime;
pub mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let database = persistence::initialize(app.handle())
                .map_err(|error| -> Box<dyn std::error::Error> { error })?;
            log::info!("SQLite initialized at {}", database.path().display());
            app.manage(database.clone());
            app.manage(runtime::ProcessManager::with_database_and_event_emitter(
                Some(database),
                events::create_runtime_event_emitter(app.handle().clone()),
            ));

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_dialog::init())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_workspace,
            commands::list_workspaces,
            commands::get_workspace_tree,
            commands::analyze_project_folder,
            commands::validate_project_command,
            commands::get_project_git_info,
            commands::get_project_logs,
            commands::get_project_health_status,
            commands::refresh_project_health,
            commands::update_project_health_check,
            commands::list_project_run_history,
            commands::list_workspace_run_history,
            commands::get_workspace_observability_summary,
            commands::start_project,
            commands::stop_project,
            commands::restart_project,
            commands::get_project_runtime_status,
            commands::get_workspace_runtime_status,
            commands::start_group,
            commands::stop_group,
            commands::start_workspace,
            commands::stop_workspace,
            commands::create_project_from_detection,
            commands::rename_workspace,
            commands::delete_workspace,
            commands::create_group,
            commands::update_group,
            commands::delete_group,
            commands::create_project,
            commands::update_project,
            commands::delete_project
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |app_handle, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            app_handle.state::<runtime::ProcessManager>().shutdown_all();
        }
    });
}
