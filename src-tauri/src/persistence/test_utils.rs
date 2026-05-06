use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::persistence::{initialize_at_path, AppDatabase};

pub(crate) struct TestDatabase {
    database: AppDatabase,
}

impl TestDatabase {
    pub(crate) fn new(test_name: &str) -> Self {
        let path = unique_test_database_path(test_name);
        let database = initialize_at_path(&path).expect("test database should initialize");

        Self { database }
    }

    pub(crate) fn database(&self) -> AppDatabase {
        self.database.clone()
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        cleanup_test_database(self.database.path());
    }
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
