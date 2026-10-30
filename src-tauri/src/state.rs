use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::db::migrations::run_migrations;
use crate::error::AppError;

/// Managed Tauri state holding the single SQLite connection.
///
/// `rusqlite::Connection` is not `Sync`, so a `Mutex` is required. All
/// mutations that span multiple statements run inside a transaction obtained
/// from the locked connection.
///
/// The connection is wrapped in an application-owned `Arc` so a controlled
/// blocking worker (e.g. `tauri::async_runtime::spawn_blocking`) can own a
/// clone of the handle without borrowing Tauri `State`. The `Arc` is shared,
/// not the connection: the `Mutex` still serializes access, and the backup
/// API's `&mut Connection` requirement is preserved through the lock guard.
#[derive(Clone)]
pub struct Db(pub Arc<Mutex<Connection>>);

impl Db {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        // Read the marker through a read-only handle before opening/configuring
        // the writable connection: WAL setup or recovery can mutate the file.
        if path.exists() {
            let check = Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            crate::db::migrations::ensure_supported_version(&check)?;
        }
        let conn = crate::db::connection::open(path)?;
        run_migrations(&conn)?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = crate::db::connection::open_in_memory()?;
        run_migrations(&conn)?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    /// Run `f` inside a transaction. Commits on success, rolls back on error.
    ///
    /// The closure receives a `&Connection` view of the transaction so it can
    /// be used with the repository functions.
    pub fn with_tx<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut conn = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let tx = conn.transaction()?;
        let result = f(&tx);
        match result {
            Ok(value) => {
                tx.commit()?;
                Ok(value)
            }
            Err(err) => {
                log::error!("transaction failed: {err:?}");
                let _ = tx.rollback();
                Err(err)
            }
        }
    }

    /// Run `f` on the connection without a wrapping transaction.
    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let conn = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let result = f(&conn);
        if let Err(err) = &result {
            log::error!("database operation failed: {err:?}");
        }
        result
    }

    /// Return a clone of the shared connection handle for a controlled blocking
    /// worker. The worker owns this clone and never borrows Tauri `State`.
    pub fn clone_handle(&self) -> Self {
        Self(Arc::clone(&self.0))
    }

    /// Run `f` on the connection with mutable access. Used by the backup API,
    /// which requires `&mut Connection`.
    pub fn with_conn_mut<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut conn = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let result = f(&mut conn);
        if let Err(err) = &result {
            log::error!("mutable database operation failed: {err:?}");
        }
        result
    }
}
