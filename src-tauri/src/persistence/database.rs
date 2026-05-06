use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use rusqlite::Connection;
use tauri::{AppHandle, Runtime};

use crate::{persistence::migrations, utils::app_paths};

pub type PersistenceResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct AppDatabase {
    path: Arc<PathBuf>,
}

impl AppDatabase {
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    pub fn connect(&self) -> PersistenceResult<Connection> {
        open_connection(self.path())
    }
}

pub fn initialize<R: Runtime>(app: &AppHandle<R>) -> PersistenceResult<AppDatabase> {
    let database_path = app_paths::database_path(app)?;
    initialize_at_path(&database_path)
}

pub fn initialize_at_path(path: &Path) -> PersistenceResult<AppDatabase> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut connection = open_connection(path)?;
    migrations::apply_all(&mut connection)?;
    drop(connection);

    Ok(AppDatabase {
        path: Arc::new(path.to_path_buf()),
    })
}

fn open_connection(path: &Path) -> PersistenceResult<Connection> {
    let connection = Connection::open(path)?;
    configure_connection(&connection)?;

    Ok(connection)
}

fn configure_connection(connection: &Connection) -> PersistenceResult<()> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use rusqlite::Connection;

    use super::initialize_at_path;
    use crate::persistence::CURRENT_SCHEMA_VERSION;

    #[test]
    fn initializes_sqlite_database_with_current_schema() {
        let db_path = unique_test_database_path("initializes_sqlite_database_with_current_schema");

        initialize_at_path(&db_path).expect("database should initialize");

        let connection = Connection::open(&db_path).expect("database file should exist");
        let schema_version = user_version(&connection);

        assert_eq!(schema_version, CURRENT_SCHEMA_VERSION as i64);
        assert!(table_exists(&connection, "workspaces"));
        assert!(table_exists(&connection, "groups"));
        assert!(table_exists(&connection, "projects"));

        drop(connection);
        cleanup_test_database(&db_path);
    }

    #[test]
    fn initialization_is_idempotent() {
        let db_path = unique_test_database_path("initialization_is_idempotent");

        initialize_at_path(&db_path).expect("first initialization should succeed");
        initialize_at_path(&db_path).expect("second initialization should succeed");

        let connection = Connection::open(&db_path).expect("database file should exist");

        assert_eq!(user_version(&connection), CURRENT_SCHEMA_VERSION as i64);

        drop(connection);
        cleanup_test_database(&db_path);
    }

    fn user_version(connection: &Connection) -> i64 {
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("user_version should be readable")
    }

    fn table_exists(connection: &Connection, table_name: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table_name],
                |row| row.get::<_, i64>(0),
            )
            .expect("sqlite_master should be readable")
            == 1
    }

    fn unique_test_database_path(test_name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();

        env::temp_dir()
            .join("centralita-tests")
            .join(format!("{test_name}-{suffix}.sqlite3"))
    }

    fn cleanup_test_database(path: &Path) {
        if path.exists() {
            let _ = fs::remove_file(path);
        }

        let wal_path = PathBuf::from(format!("{}-wal", path.display()));
        if wal_path.exists() {
            let _ = fs::remove_file(wal_path);
        }

        let shm_path = PathBuf::from(format!("{}-shm", path.display()));
        if shm_path.exists() {
            let _ = fs::remove_file(shm_path);
        }

        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent);
        }
    }
}
