pub mod claude;
pub mod kimi;
pub mod session;
pub mod traits;

pub use claude::ClaudeAdapter;
pub use kimi::KimiAdapter;
pub use session::generate_session_id;
pub use traits::{AgentAdapter, AgentOutput};

/// Create an adapter by name.
pub fn create_adapter(name: &str) -> Option<Box<dyn AgentAdapter>> {
    match name {
        "claude" => Some(Box::new(ClaudeAdapter::new())),
        "kimi" => Some(Box::new(KimiAdapter::new())),
        _ => None,
    }
}
