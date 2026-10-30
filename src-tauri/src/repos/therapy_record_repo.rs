use super::year_theme_repo::map_constraint_violation;
use crate::domain::models::TherapyRecord;
use crate::domain::types::{TherapyRecordStatus, TherapyRecordType};
use crate::domain::validation::validate_title;
use crate::error::AppError;
use crate::{id, time};
use rusqlite::{params, Connection, OptionalExtension};
pub trait TherapyRecordRepo {
    fn create(
        &self,
        c: &Connection,
        collection_id: &str,
        t: TherapyRecordType,
        title: &str,
        content: &str,
        occurred_on: &Option<String>,
    ) -> Result<TherapyRecord, AppError>;
    fn get(&self, c: &Connection, id: &str) -> Result<TherapyRecord, AppError>;
    fn list_for_collection(&self, c: &Connection, id: &str)
        -> Result<Vec<TherapyRecord>, AppError>;
    fn list_all(&self, c: &Connection, w: &str) -> Result<Vec<TherapyRecord>, AppError>;
    /// List non-archived therapy record IDs in chronological order (occurred_on ASC NULLS LAST, title ASC, id ASC),
    /// with pagination for low-RAM chunked resolution.
    fn list_ids_paged(
        &self,
        c: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<String>, AppError>;
    /// List a page of non-archived therapy records in chronological order (occurred_on ASC NULLS LAST, title ASC, id ASC).
    fn list_page(
        &self,
        c: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TherapyRecord>, AppError>;
    /// List a page of non-archived therapy records within a date range (occurred_on BETWEEN from AND to),
    /// in chronological order (occurred_on ASC NULLS LAST, title ASC, id ASC).
    /// Records with NULL occurred_on are excluded.
    fn list_range_page(
        &self,
        c: &Connection,
        workspace_id: &str,
        from: &str,
        to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TherapyRecord>, AppError>;
    /// Count non-archived therapy records for a workspace.
    fn count(&self, c: &Connection, workspace_id: &str) -> Result<i64, AppError>;
    fn update(
        &self,
        c: &Connection,
        id: &str,
        t: TherapyRecordType,
        title: &str,
        content: &str,
        occurred_on: &Option<String>,
        status: TherapyRecordStatus,
    ) -> Result<TherapyRecord, AppError>;
    fn archive(&self, c: &Connection, id: &str) -> Result<TherapyRecord, AppError>;
}
const COLS:&str="r.id,r.therapy_collection_id,r.type,r.title,r.content_markdown,r.occurred_on,r.status,r.privacy_scope,r.created_at,r.updated_at,r.archived_at";
fn row(x: &rusqlite::Row<'_>) -> Result<TherapyRecord, rusqlite::Error> {
    let t: String = x.get(2)?;
    let s: String = x.get(6)?;
    let p: String = x.get(7)?;
    Ok(TherapyRecord {
        id: x.get(0)?,
        therapy_collection_id: x.get(1)?,
        record_type: t.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(2, "type".into(), rusqlite::types::Type::Text)
        })?,
        title: x.get(3)?,
        content_markdown: x.get(4)?,
        occurred_on: x.get(5)?,
        status: s.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(6, "status".into(), rusqlite::types::Type::Text)
        })?,
        privacy_scope: p.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                7,
                "privacy_scope".into(),
                rusqlite::types::Type::Text,
            )
        })?,
        created_at: x.get(8)?,
        updated_at: x.get(9)?,
        archived_at: x.get(10)?,
    })
}
pub struct SqliteTherapyRecordRepo;
impl TherapyRecordRepo for SqliteTherapyRecordRepo {
    fn create(
        &self,
        c: &Connection,
        k: &str,
        t: TherapyRecordType,
        n: &str,
        v: &str,
        d: &Option<String>,
    ) -> Result<TherapyRecord, AppError> {
        validate_title(n)?;
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        c.execute("INSERT INTO therapy_records (id,therapy_collection_id,type,title,content_markdown,occurred_on,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?7)",params![id,k,t.to_string(),n.trim(),v,d,now]).map_err(map_constraint_violation)?;
        self.get(c, &id)
    }
    fn get(&self, c: &Connection, id: &str) -> Result<TherapyRecord, AppError> {
        c.query_row(
            &format!("SELECT {COLS} FROM therapy_records r WHERE r.id=?1"),
            [id],
            row,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The therapy record was not found."))
    }
    fn list_for_collection(
        &self,
        c: &Connection,
        id: &str,
    ) -> Result<Vec<TherapyRecord>, AppError> {
        let mut s=c.prepare(&format!("SELECT {COLS} FROM therapy_records r WHERE r.therapy_collection_id=?1 AND r.archived_at IS NULL ORDER BY r.occurred_on DESC,r.created_at DESC"))?;
        let result = s.query_map([id], row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
    fn list_all(&self, c: &Connection, w: &str) -> Result<Vec<TherapyRecord>, AppError> {
        let mut s=c.prepare(&format!("SELECT {COLS} FROM therapy_records r JOIN therapy_collections c ON c.id=r.therapy_collection_id WHERE c.workspace_id=?1 AND r.archived_at IS NULL ORDER BY r.occurred_on DESC,r.created_at DESC"))?;
        let result = s.query_map([w], row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }

    fn list_ids_paged(
        &self,
        c: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<String>, AppError> {
        let mut stmt = c.prepare(
            "SELECT r.id FROM therapy_records r JOIN therapy_collections c ON c.id=r.therapy_collection_id WHERE c.workspace_id=?1 AND r.archived_at IS NULL ORDER BY r.occurred_on ASC NULLS LAST, r.title ASC, r.id ASC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![workspace_id, limit as i64, offset as i64], |row| {
            row.get(0)
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_page(
        &self,
        c: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TherapyRecord>, AppError> {
        let mut stmt = c.prepare(&format!(
            "SELECT {COLS} FROM therapy_records r JOIN therapy_collections c ON c.id=r.therapy_collection_id WHERE c.workspace_id=?1 AND r.archived_at IS NULL ORDER BY r.occurred_on ASC NULLS LAST, r.title ASC, r.id ASC LIMIT ?2 OFFSET ?3"
        ))?;
        let rows = stmt.query_map(params![workspace_id, limit as i64, offset as i64], row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_range_page(
        &self,
        c: &Connection,
        workspace_id: &str,
        from: &str,
        to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<TherapyRecord>, AppError> {
        let mut stmt = c.prepare(&format!(
            "SELECT {COLS} FROM therapy_records r JOIN therapy_collections c ON c.id=r.therapy_collection_id WHERE c.workspace_id=?1 AND r.archived_at IS NULL AND r.occurred_on IS NOT NULL AND r.occurred_on BETWEEN ?2 AND ?3 ORDER BY r.occurred_on ASC NULLS LAST, r.title ASC, r.id ASC LIMIT ?4 OFFSET ?5"
        ))?;
        let rows = stmt.query_map(
            params![workspace_id, from, to, limit as i64, offset as i64],
            row,
        )?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn count(&self, c: &Connection, workspace_id: &str) -> Result<i64, AppError> {
        let count: i64 = c.query_row(
            "SELECT COUNT(*) FROM therapy_records r JOIN therapy_collections c ON c.id=r.therapy_collection_id WHERE c.workspace_id=?1 AND r.archived_at IS NULL",
            [workspace_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    fn update(
        &self,
        c: &Connection,
        id: &str,
        t: TherapyRecordType,
        n: &str,
        v: &str,
        d: &Option<String>,
        s: TherapyRecordStatus,
    ) -> Result<TherapyRecord, AppError> {
        validate_title(n)?;
        let archived = if s == TherapyRecordStatus::Archived {
            Some(time::now_rfc3339())
        } else {
            None
        };
        if c.execute("UPDATE therapy_records SET type=?2,title=?3,content_markdown=?4,occurred_on=?5,status=?6,archived_at=?7,updated_at=?8 WHERE id=?1",params![id,t.to_string(),n.trim(),v,d,s.to_string(),archived,time::now_rfc3339()])?==0{return Err(AppError::not_found("The therapy record was not found."))}
        self.get(c, id)
    }
    fn archive(&self, c: &Connection, id: &str) -> Result<TherapyRecord, AppError> {
        if c.execute(
            "UPDATE therapy_records SET status='archived',archived_at=?2,updated_at=?2 WHERE id=?1",
            params![id, time::now_rfc3339()],
        )? == 0
        {
            return Err(AppError::not_found("The therapy record was not found."));
        }
        self.get(c, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task_closure::transition_status;
    use crate::domain::types::{
        HistorySource, Priority, TaskStatus, TherapyPrivacyScope, TherapyReferenceRelation,
    };
    use crate::repos::task_repo::{complete_task, SqliteTaskRepo, TaskRepo};
    use crate::repos::therapy_collection_repo::{
        SqliteTherapyCollectionRepo, TherapyCollectionRepo,
    };
    use crate::repos::therapy_reference_repo::{
        SqliteTherapyTaskReferenceRepo, TherapyTaskReferenceRepo,
    };
    use crate::test_utils::test_conn;

    fn fixture(conn: &Connection) -> (TherapyRecord, String) {
        let collection = SqliteTherapyCollectionRepo
            .create(
                conn,
                "w1",
                "Private",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("collection");
        let record = SqliteTherapyRecordRepo
            .create(
                conn,
                &collection.id,
                TherapyRecordType::Note,
                "Note",
                "content",
                &None,
            )
            .expect("record");
        let task = SqliteTaskRepo
            .create(
                conn,
                "w1",
                &None,
                &None,
                &None,
                "Task",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("task");
        (record, task.id)
    }

    #[test]
    fn therapy_reference_does_not_make_task_a_child() {
        let conn = test_conn();
        let (record, task_id) = fixture(&conn);
        SqliteTherapyTaskReferenceRepo
            .create(
                &conn,
                &record.id,
                &Some(task_id.clone()),
                TherapyReferenceRelation::RelatedTo,
                &None,
            )
            .expect("reference");
        assert!(SqliteTaskRepo
            .get(&conn, &task_id)
            .expect("task")
            .parent_task_id
            .is_none());
    }

    #[test]
    fn task_and_therapy_record_lifecycles_are_independent() {
        let conn = test_conn();
        let (record, task_id) = fixture(&conn);
        transition_status(
            &conn,
            &task_id,
            TaskStatus::InProgress,
            &None,
            HistorySource::User,
        )
        .expect("start task");
        complete_task(&conn, &task_id, &None).expect("complete task");
        assert_eq!(
            SqliteTherapyRecordRepo
                .get(&conn, &record.id)
                .expect("record")
                .status,
            TherapyRecordStatus::Open
        );

        SqliteTherapyRecordRepo
            .archive(&conn, &record.id)
            .expect("archive record");
        assert_eq!(
            SqliteTaskRepo.get(&conn, &task_id).expect("task").status,
            TaskStatus::Completed
        );
    }

    #[test]
    fn archiving_task_keeps_therapy_record_and_reference() {
        let conn = test_conn();
        let (record, task_id) = fixture(&conn);
        let reference = SqliteTherapyTaskReferenceRepo
            .create(
                &conn,
                &record.id,
                &Some(task_id.clone()),
                TherapyReferenceRelation::Supports,
                &None,
            )
            .expect("reference");
        transition_status(
            &conn,
            &task_id,
            TaskStatus::InProgress,
            &None,
            HistorySource::User,
        )
        .expect("start task");
        complete_task(&conn, &task_id, &None).expect("complete task");
        SqliteTaskRepo
            .archive(&conn, &task_id)
            .expect("archive task");
        assert!(SqliteTherapyRecordRepo.get(&conn, &record.id).is_ok());
        assert_eq!(
            SqliteTherapyTaskReferenceRepo
                .list_for_record(&conn, &record.id)
                .expect("references")[0]
                .id,
            reference.id
        );
    }

    #[test]
    fn deleting_task_keeps_therapy_record_and_reference() {
        let conn = test_conn();
        let (record, task_id) = fixture(&conn);
        let reference = SqliteTherapyTaskReferenceRepo
            .create(
                &conn,
                &record.id,
                &Some(task_id.clone()),
                TherapyReferenceRelation::RelatedTo,
                &None,
            )
            .expect("reference");
        SqliteTaskRepo
            .delete_subtree(&conn, &task_id)
            .expect("delete task");
        assert!(SqliteTherapyRecordRepo.get(&conn, &record.id).is_ok());
        let remaining = SqliteTherapyTaskReferenceRepo
            .list_for_record(&conn, &record.id)
            .expect("references");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, reference.id);
        assert_eq!(remaining[0].task_id, None);
    }
}
