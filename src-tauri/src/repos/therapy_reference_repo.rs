use super::year_theme_repo::map_constraint_violation;
use crate::domain::models::TherapyTaskReference;
use crate::domain::types::TherapyReferenceRelation;
use crate::error::AppError;
use crate::{id, time};
use rusqlite::{params, Connection};
pub trait TherapyTaskReferenceRepo {
    fn create(
        &self,
        c: &Connection,
        r: &str,
        t: &Option<String>,
        rel: TherapyReferenceRelation,
        n: &Option<String>,
    ) -> Result<TherapyTaskReference, AppError>;
    fn list_for_record(
        &self,
        c: &Connection,
        r: &str,
    ) -> Result<Vec<TherapyTaskReference>, AppError>;
    fn remove(&self, c: &Connection, id: &str) -> Result<(), AppError>;
}
const COLS: &str =
    "id,therapy_record_id,task_id,relation,note_markdown,created_by,created_at,removed_at";
fn row(x: &rusqlite::Row<'_>) -> Result<TherapyTaskReference, rusqlite::Error> {
    let rel: String = x.get(3)?;
    Ok(TherapyTaskReference {
        id: x.get(0)?,
        therapy_record_id: x.get(1)?,
        task_id: x.get(2)?,
        relation: rel.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(3, "relation".into(), rusqlite::types::Type::Text)
        })?,
        note_markdown: x.get(4)?,
        created_by: x.get(5)?,
        created_at: x.get(6)?,
        removed_at: x.get(7)?,
    })
}
pub struct SqliteTherapyTaskReferenceRepo;
impl TherapyTaskReferenceRepo for SqliteTherapyTaskReferenceRepo {
    fn create(
        &self,
        c: &Connection,
        r: &str,
        t: &Option<String>,
        rel: TherapyReferenceRelation,
        n: &Option<String>,
    ) -> Result<TherapyTaskReference, AppError> {
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        c.execute("INSERT INTO therapy_task_references (id,therapy_record_id,task_id,relation,note_markdown,created_by,created_at) VALUES (?1,?2,?3,?4,?5,'user',?6)",params![id,r,t,rel.to_string(),n,now]).map_err(map_constraint_violation)?;
        c.query_row(
            &format!("SELECT {COLS} FROM therapy_task_references WHERE id=?1"),
            [id],
            row,
        )
        .map_err(AppError::from)
    }
    fn list_for_record(
        &self,
        c: &Connection,
        r: &str,
    ) -> Result<Vec<TherapyTaskReference>, AppError> {
        let mut s=c.prepare(&format!("SELECT {COLS} FROM therapy_task_references WHERE therapy_record_id=?1 AND removed_at IS NULL ORDER BY created_at"))?;
        let result = s.query_map([r], row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
    fn remove(&self, c: &Connection, id: &str) -> Result<(), AppError> {
        if c.execute(
            "UPDATE therapy_task_references SET removed_at=?2 WHERE id=?1 AND removed_at IS NULL",
            params![id, time::now_rfc3339()],
        )? == 0
        {
            return Err(AppError::not_found("The therapy reference was not found."));
        }
        Ok(())
    }
}
