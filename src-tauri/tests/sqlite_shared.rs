//! Shared SQLite guarantees exercised through the public database and repository APIs.

use std::sync::mpsc;
use std::sync::TryLockError;
use std::thread;

use albada_lib::domain::types::Priority;
use albada_lib::repos::task_repo::{SqliteTaskRepo, TaskRepo};
use albada_lib::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
use albada_lib::state::Db;

#[test]
fn transaction_rolls_back_when_the_operation_returns_an_error() {
    let db = Db::open_in_memory().expect("open in-memory database");
    db.with_conn(|conn| {
        conn.execute_batch("CREATE TABLE rollback_probe (id TEXT PRIMARY KEY);")?;
        Ok(())
    })
    .expect("create synthetic probe table");

    let result = db.with_tx::<()>(|conn| {
        conn.execute("INSERT INTO rollback_probe (id) VALUES ('synthetic')", [])?;
        Err(albada_lib::error::AppError::validation("rollback test"))
    });

    assert!(result.is_err());
    let row_count = db
        .with_conn(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM rollback_probe", [], |row| {
                    row.get::<_, i64>(0)
                })?,
            )
        })
        .expect("count synthetic probe rows");
    assert_eq!(row_count, 0);
}

#[test]
fn cloned_db_handle_serializes_reads_on_the_shared_connection() {
    let db = Db::open_in_memory().expect("open in-memory database");
    let holding_db = db.clone_handle();
    let (locked_tx, locked_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let holder = thread::spawn(move || {
        holding_db.with_conn(|_| {
            locked_tx.send(()).expect("signal held connection");
            let _ = release_rx.recv();
            Ok(())
        })
    });
    locked_rx.recv().expect("connection lock acquired");

    let reader_db = db.clone_handle();
    let (blocked_tx, blocked_rx) = mpsc::channel();
    let (read_tx, read_rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        blocked_tx
            .send(matches!(
                reader_db.0.try_lock(),
                Err(TryLockError::WouldBlock)
            ))
            .expect("send lock contention result");
        let value = reader_db
            .with_conn(|conn| Ok(conn.query_row("SELECT 42", [], |row| row.get::<_, i64>(0))?))
            .expect("read after lock released");
        read_tx.send(value).expect("send read result");
    });

    assert!(blocked_rx.recv().expect("read lock status"));
    release_tx.send(()).expect("release connection lock");
    holder
        .join()
        .expect("join lock holder")
        .expect("release lock");
    reader.join().expect("join reader");
    assert_eq!(read_rx.recv().expect("receive read result"), 42);
}

#[test]
fn task_update_preserves_its_stable_id() {
    let db = Db::open_in_memory().expect("open in-memory database");
    let (workspace_id, task_id) = db
        .with_tx(|conn| {
            let workspace = SqliteWorkspaceRepo.create(conn, "Synthetic workspace")?;
            let task = SqliteTaskRepo.create(
                conn,
                &workspace.id,
                &None,
                &None,
                &None,
                "Before update",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )?;
            Ok((workspace.id, task.id))
        })
        .expect("create synthetic task");

    let updated = db
        .with_tx(|conn| {
            SqliteTaskRepo.update(
                conn,
                &task_id,
                "After update",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
        })
        .expect("update task through repository");

    assert_eq!(updated.workspace_id, workspace_id);
    assert_eq!(updated.id, task_id);
    assert_eq!(updated.title, "After update");
}
