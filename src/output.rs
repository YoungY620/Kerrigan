use serde::Serialize;

pub const DEFAULT_REMINDER: &str = "How to use Kerrigan well? \
    Consciously use Kerrigan to decompose tasks at fine granularity, \
    and deliberately design multiple independent cross-verification tasks.";

/// Wrap any serializable value in the standard output envelope and print to stdout.
///
/// All Kerrigan command output goes through this function to ensure a consistent
/// `{"data": ..., "reminder": "..."}` envelope.
pub fn output_json(value: impl Serialize) {
    let value = serde_json::to_value(value).expect("serialization failed");
    let wrapped = serde_json::json!({
        "data": value,
        "reminder": DEFAULT_REMINDER,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&wrapped).expect("serialization failed")
    );
}
