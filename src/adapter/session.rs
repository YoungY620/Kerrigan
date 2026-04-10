use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a unique session ID for kerrigan agent invocations.
///
/// Format: `krg_{timestamp_ms}_{random_suffix}`
pub fn generate_session_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let rand: u32 = rand_u32();
    format!("krg_{ts}_{rand:08x}")
}

/// Simple random u32 without pulling in a rand crate.
/// Uses timestamp nanos + thread id as entropy source.
fn rand_u32() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let thread_id = std::thread::current().id();
    let tid_hash = format!("{thread_id:?}").len() as u32;
    nanos.wrapping_mul(2654435761).wrapping_add(tid_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn session_id_format() {
        let id = generate_session_id();
        assert!(id.starts_with("krg_"));
        let parts: Vec<&str> = id.split('_').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn session_id_uniqueness() {
        let ids: HashSet<String> = (0..100).map(|_| generate_session_id()).collect();
        // With millisecond timestamps, some may collide in tight loops,
        // but we should get a reasonable number of unique IDs
        assert!(ids.len() > 50);
    }
}
