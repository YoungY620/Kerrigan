/// Generate a unique session ID.
///
/// Returns a UUID v4 string, which is compatible with both Claude Code
/// (requires valid UUID for --session-id) and Kimi CLI (accepts any string).
pub fn generate_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn session_id_is_valid_uuid() {
        let id = generate_session_id();
        assert!(uuid::Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn session_id_uniqueness() {
        let ids: HashSet<String> = (0..1000).map(|_| generate_session_id()).collect();
        assert_eq!(ids.len(), 1000);
    }
}
