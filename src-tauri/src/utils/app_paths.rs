use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use crate::persistence::PersistenceResult;

const DATABASE_FILE_NAME: &str = "centralita.sqlite3";

pub fn database_path<R: Runtime>(app: &AppHandle<R>) -> PersistenceResult<PathBuf> {
    let app_data_dir = app.path().app_data_dir()?;

    Ok(app_data_dir.join(DATABASE_FILE_NAME))
}
