use super::year_theme_repo::map_constraint_violation;
use crate::domain::models::TherapyCollection;
use crate::domain::types::TherapyPrivacyScope;
use crate::domain::validation::validate_title;
use crate::error::AppError;
use crate::{id, time};
use rusqlite::{params, Connection, OptionalExtension};

pub trait TherapyCollectionRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        name: &str,
        privacy_scope: TherapyPrivacyScope,
        encryption_status: &Option<String>,
    ) -> Result<TherapyCollection, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<TherapyCollection, AppError>;
    fn list(
        &self,
        conn: &Connection,
        workspace_id: &str,
    ) -> Result<Vec<TherapyCollection>, AppError>;
    fn list_including_archived(
        &self,
        conn: &Connection,
        workspace_id: &str,
    ) -> Result<Vec<TherapyCollection>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        name: &str,
        privacy_scope: TherapyPrivacyScope,
        encryption_status: &Option<String>,
    ) -> Result<TherapyCollection, AppError>;
    fn archive(&self, conn: &Connection, id: &str) -> Result<TherapyCollection, AppError>;
}
const COLS: &str =
    "id, workspace_id, name, privacy_scope, encryption_status, created_at, updated_at, archived_at";
fn row(row: &rusqlite::Row<'_>) -> Result<TherapyCollection, rusqlite::Error> {
    let p: String = row.get(3)?;
    Ok(TherapyCollection {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        privacy_scope: p.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                3,
                "privacy_scope".into(),
                rusqlite::types::Type::Text,
            )
        })?,
        encryption_status: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        archived_at: row.get(7)?,
    })
}
pub struct SqliteTherapyCollectionRepo;
impl TherapyCollectionRepo for SqliteTherapyCollectionRepo {
    fn create(
        &self,
        c: &Connection,
        w: &str,
        n: &str,
        p: TherapyPrivacyScope,
        e: &Option<String>,
    ) -> Result<TherapyCollection, AppError> {
        validate_title(n)?;
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        c.execute("INSERT INTO therapy_collections (id, workspace_id, name, privacy_scope, encryption_status, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?6)", params![id,w,n.trim(),p.to_string(),e,now]).map_err(map_constraint_violation)?;
        self.get(c, &id)
    }
    fn get(&self, c: &Connection, id: &str) -> Result<TherapyCollection, AppError> {
        c.query_row(
            &format!("SELECT {COLS} FROM therapy_collections WHERE id=?1"),
            [id],
            row,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The therapy collection was not found."))
    }
    fn list(&self, c: &Connection, w: &str) -> Result<Vec<TherapyCollection>, AppError> {
        let mut s = c.prepare(&format!(
            "SELECT {COLS} FROM therapy_collections WHERE workspace_id=?1 AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let result = s.query_map([w], row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
    fn list_including_archived(
        &self,
        c: &Connection,
        w: &str,
    ) -> Result<Vec<TherapyCollection>, AppError> {
        let mut s = c.prepare(&format!(
            "SELECT {COLS} FROM therapy_collections WHERE workspace_id=?1 ORDER BY created_at"
        ))?;
        let result = s.query_map([w], row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
    fn update(
        &self,
        c: &Connection,
        id: &str,
        n: &str,
        p: TherapyPrivacyScope,
        e: &Option<String>,
    ) -> Result<TherapyCollection, AppError> {
        validate_title(n)?;
        if c.execute("UPDATE therapy_collections SET name=?2,privacy_scope=?3,encryption_status=COALESCE(?4,encryption_status),updated_at=?5 WHERE id=?1",params![id,n.trim(),p.to_string(),e,time::now_rfc3339()])?==0{return Err(AppError::not_found("The therapy collection was not found."))}
        self.get(c, id)
    }
    fn archive(&self, c: &Connection, id: &str) -> Result<TherapyCollection, AppError> {
        if c.execute(
            "UPDATE therapy_collections SET archived_at=?2,updated_at=?2 WHERE id=?1",
            params![id, time::now_rfc3339()],
        )? == 0
        {
            return Err(AppError::not_found("The therapy collection was not found."));
        }
        self.get(c, id)
    }
}

#[cfg(test)]
mod tests {
    use super::{SqliteTherapyCollectionRepo, TherapyCollectionRepo};
    use crate::domain::types::TherapyPrivacyScope;
    use crate::test_utils::test_conn;

    #[test]
    fn updating_and_archiving_collections_persists_and_list_hides_archived() {
        let conn = test_conn();
        let collection = SqliteTherapyCollectionRepo
            .create(
                &conn,
                "w1",
                "Synthetic collection",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("create collection");
        let active_collection = SqliteTherapyCollectionRepo
            .create(
                &conn,
                "w1",
                "Another active collection",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("create active collection");

        let updated = SqliteTherapyCollectionRepo
            .update(
                &conn,
                &collection.id,
                "Renamed collection",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("update collection");
        assert_eq!(updated.name, "Renamed collection");

        let archived = SqliteTherapyCollectionRepo
            .archive(&conn, &collection.id)
            .expect("archive collection");
        assert!(archived.archived_at.is_some());

        let listed = SqliteTherapyCollectionRepo
            .list(&conn, "w1")
            .expect("list active collections");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, active_collection.id);

        let all_collections = SqliteTherapyCollectionRepo
            .list_including_archived(&conn, "w1")
            .expect("list all collections for import identity lookup");
        assert_eq!(all_collections.len(), 2);
    }

    #[test]
    fn renaming_collection_preserves_omitted_encryption_status() {
        let conn = test_conn();
        let collection = SqliteTherapyCollectionRepo
            .create(
                &conn,
                "w1",
                "Synthetic collection",
                TherapyPrivacyScope::TherapyPrivate,
                &Some("synthetic-status".to_string()),
            )
            .expect("create collection");

        let updated = SqliteTherapyCollectionRepo
            .update(
                &conn,
                &collection.id,
                "Renamed collection",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("update collection");

        assert_eq!(
            updated.encryption_status.as_deref(),
            Some("synthetic-status")
        );
    }
}
