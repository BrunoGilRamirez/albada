use uuid::Uuid;

/// Generate a new version-4 UUID string for stable entity IDs.
pub fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Parse a UUID string, rejecting invalid identifiers.
pub fn parse_uuid(id: &str) -> Result<Uuid, crate::error::AppError> {
    Uuid::parse_str(id).map_err(Into::into)
}
