//! Integration tests for the full task lifecycle across repositories.
//!
//! These exercise the same code paths the Tauri commands use, without
//! rendering the UI: standalone tasks, parent/child trees, auto-completion,
//! manual closure, and dependencies.

use albada_lib::db::connection;
use albada_lib::domain::types::{ClosureMode, HistorySource, Priority, TaskStatus};
use albada_lib::repos::dependency_repo::{DependencyRepo, SqliteDependencyRepo};
use albada_lib::repos::task_repo::{complete_task, SqliteTaskRepo, TaskRepo};
use albada_lib::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};

fn setup() -> (rusqlite::Connection, String) {
    let conn = connection::open_in_memory().expect("open in-memory connection");
    albada_lib::db::migrations::run_migrations(&conn).expect("run migrations");
    let ws_id = SqliteWorkspaceRepo
        .create(&conn, "Personal")
        .expect("ws")
        .id;
    (conn, ws_id)
}

fn start(conn: &rusqlite::Connection, task_id: &str) {
    albada_lib::domain::task_closure::transition_status(
        conn,
        task_id,
        TaskStatus::InProgress,
        &None,
        HistorySource::User,
    )
    .expect("start task");
}

#[test]
fn create_and_complete_trivial_standalone_task() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let task = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Send email",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create standalone task");
    assert_eq!(task.parent_task_id, None);

    start(&conn, &task.id);
    let completed = complete_task(&conn, &task.id, &None).expect("complete task");
    assert_eq!(completed.status, TaskStatus::Completed);
    assert!(completed.completed_at.is_some());
}

#[test]
fn divide_large_task_into_children_and_auto_complete_parent() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let parent = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Write the book",
            &None,
            &None,
            Priority::High,
            &None,
            &None,
            &None,
        )
        .expect("create parent");

    let child1 = repo
        .create_child_task(
            &conn,
            &parent.id,
            &ws_id,
            "Outline",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create child 1");
    let child2 = repo
        .create_child_task(
            &conn,
            &parent.id,
            &ws_id,
            "Draft chapter one",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create child 2");

    assert_eq!(child1.parent_task_id.as_deref(), Some(parent.id.as_str()));
    assert_eq!(child2.parent_task_id.as_deref(), Some(parent.id.as_str()));

    start(&conn, &child1.id);
    start(&conn, &child2.id);
    complete_task(&conn, &child1.id, &None).expect("complete child 1");
    complete_task(&conn, &child2.id, &None).expect("complete child 2");

    let parent_after = repo.get(&conn, &parent.id).expect("parent after");
    assert_eq!(parent_after.status, TaskStatus::Completed);
    assert_eq!(parent_after.closure_mode, Some(ClosureMode::Automatic));
}

#[test]
fn manually_close_parent_with_open_children() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let parent = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Organize garage",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create parent");
    let child = repo
        .create_child_task(
            &conn,
            &parent.id,
            &ws_id,
            "Sort boxes",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create child");

    albada_lib::domain::task_closure::close_parent_manually(
        &conn,
        &parent.id,
        ClosureMode::ManualAcceptedPartial,
        "Good enough for now.",
    )
    .expect("manual closure");

    let parent_after = repo.get(&conn, &parent.id).expect("parent after");
    assert_eq!(parent_after.status, TaskStatus::Completed);
    assert_eq!(
        parent_after.closure_mode,
        Some(ClosureMode::ManualAcceptedPartial)
    );
    assert_eq!(
        parent_after.closure_reason_markdown.as_deref(),
        Some("Good enough for now.")
    );

    // Children keep their own state.
    let child_after = repo.get(&conn, &child.id).expect("child after");
    assert_eq!(child_after.status, TaskStatus::Inbox);
}

#[test]
fn add_and_remove_dependency_unblocks_task() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let prerequisite = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Install tools",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("prerequisite");
    let blocked = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Run the build",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("blocked");

    let dep_repo = SqliteDependencyRepo;
    let dep = dep_repo
        .add_dependency(&conn, &blocked.id, &prerequisite.id, &None)
        .expect("add dependency");

    let blocking = dep_repo
        .list_blocking_tasks(&conn, &blocked.id)
        .expect("list blocking");
    assert_eq!(blocking.len(), 1);
    assert_eq!(blocking[0].task_id, prerequisite.id);

    dep_repo
        .remove_dependency(&conn, &dep.id, &None)
        .expect("remove dependency");

    let blocking = dep_repo
        .list_blocking_tasks(&conn, &blocked.id)
        .expect("list blocking after remove");
    assert!(blocking.is_empty());
}

#[test]
fn dependency_cycle_and_self_dependency_are_rejected() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let a = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "A",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("a");
    let b = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "B",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("b");
    let c = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "C",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("c");

    let dep_repo = SqliteDependencyRepo;
    dep_repo
        .add_dependency(&conn, &a.id, &b.id, &None)
        .expect("a depends on b");
    dep_repo
        .add_dependency(&conn, &b.id, &c.id, &None)
        .expect("b depends on c");
    assert!(dep_repo.add_dependency(&conn, &c.id, &a.id, &None).is_err());

    assert!(dep_repo.add_dependency(&conn, &a.id, &a.id, &None).is_err());
}

#[test]
fn archive_restore_and_delete_subtree() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let parent = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Parent",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create parent");
    let child = repo
        .create_child_task(
            &conn,
            &parent.id,
            &ws_id,
            "Child",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create child");

    // Only completed tasks can be archived, so complete the child first.
    albada_lib::domain::task_closure::transition_status(
        &conn,
        &child.id,
        TaskStatus::InProgress,
        &None,
        HistorySource::User,
    )
    .expect("start child");
    complete_task(&conn, &child.id, &None).expect("complete child");

    // Archive the subtree; only the child is archived, the parent stays visible.
    repo.archive_subtree(&conn, &parent.id)
        .expect("archive subtree");
    assert!(
        repo.get(&conn, &parent.id)
            .expect("parent")
            .archived_at
            .is_none(),
        "parent remains visible when archiving its subtree"
    );
    assert!(repo
        .get(&conn, &child.id)
        .expect("child")
        .archived_at
        .is_some());

    // Restore the subtree; the child is visible again.
    repo.restore_subtree(&conn, &parent.id)
        .expect("restore subtree");
    assert!(repo
        .get(&conn, &parent.id)
        .expect("parent")
        .archived_at
        .is_none());
    assert!(repo
        .get(&conn, &child.id)
        .expect("child")
        .archived_at
        .is_none());

    // Delete the subtree; the task and its children are gone.
    repo.delete_subtree(&conn, &parent.id)
        .expect("delete subtree");
    assert!(repo.get(&conn, &parent.id).is_err());
    assert!(repo.get(&conn, &child.id).is_err());
}

#[test]
fn blocking_note_allows_blocked_without_dependency() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let task = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Waiting on someone",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create task");

    // A task in inbox can go to planned, then blocked with a note.
    albada_lib::domain::task_closure::transition_status(
        &conn,
        &task.id,
        TaskStatus::Planned,
        &None,
        HistorySource::User,
    )
    .expect("to planned");
    albada_lib::domain::task_closure::transition_status(
        &conn,
        &task.id,
        TaskStatus::Blocked,
        &Some("Waiting for a reply".into()),
        HistorySource::User,
    )
    .expect("to blocked with note");

    let updated = repo.get(&conn, &task.id).expect("get task");
    assert_eq!(updated.status, TaskStatus::Blocked);
    assert_eq!(
        updated.blocking_note_markdown.as_deref(),
        Some("Waiting for a reply")
    );
}

#[test]
fn adding_new_child_after_auto_completion_reopens_parent() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;
    let parent = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Parent",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create parent");

    let child1 = repo
        .create_child_task(
            &conn,
            &parent.id,
            &ws_id,
            "Child 1",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create child 1");
    let child2 = repo
        .create_child_task(
            &conn,
            &parent.id,
            &ws_id,
            "Child 2",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create child 2");

    for child in [&child1, &child2] {
        albada_lib::domain::task_closure::transition_status(
            &conn,
            &child.id,
            TaskStatus::InProgress,
            &None,
            HistorySource::User,
        )
        .expect("start child");
        complete_task(&conn, &child.id, &None).expect("complete child");
    }

    let parent_done = repo.get(&conn, &parent.id).expect("parent done");
    assert_eq!(parent_done.status, TaskStatus::Completed);

    // Add a new child that starts in progress; the parent must reopen.
    repo.create_child_task(
        &conn,
        &parent.id,
        &ws_id,
        "Child 3",
        &None,
        &None,
        Priority::Normal,
        &None,
        &None,
        &None,
    )
    .expect("create child 3");

    let parent_after = repo.get(&conn, &parent.id).expect("parent after new child");
    assert_eq!(
        parent_after.status,
        TaskStatus::InProgress,
        "parent reopens when new open work appears"
    );
    assert_eq!(parent_after.closure_mode, None);
}

#[test]
fn archived_tasks_can_be_listed_and_restored() {
    let (conn, ws_id) = setup();
    let repo = SqliteTaskRepo;

    let parent = repo
        .create(
            &conn,
            &ws_id,
            &None,
            &None,
            &None,
            "Parent",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create parent");
    let child = repo
        .create_child_task(
            &conn,
            &parent.id,
            &ws_id,
            "Child",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("create child");

    // Only completed tasks can be archived, so complete the child first.
    albada_lib::domain::task_closure::transition_status(
        &conn,
        &child.id,
        TaskStatus::InProgress,
        &None,
        HistorySource::User,
    )
    .expect("start child");
    complete_task(&conn, &child.id, &None).expect("complete child");

    // Archive hides only the child; the parent stays visible.
    repo.archive_subtree(&conn, &parent.id)
        .expect("archive subtree");

    let archived = repo.list_archived(&conn, &ws_id).expect("list archived");
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].id, child.id);

    let standalone = repo
        .list_standalone(&conn, &ws_id)
        .expect("list standalone");
    assert!(
        standalone.iter().all(|t| t.id != child.id),
        "archived child hidden from lists"
    );

    // Restore the archived task (and its descendants) from the settings flow.
    repo.restore_task_including_subtree(&conn, &child.id)
        .expect("restore archived task");
    assert!(repo
        .get(&conn, &child.id)
        .expect("child")
        .archived_at
        .is_none());

    let archived = repo
        .list_archived(&conn, &ws_id)
        .expect("list archived after");
    assert!(archived.is_empty());
}
