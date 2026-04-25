use serde::Serialize;

/// Wrap any serializable value in the standard output envelope and print to stdout.
pub fn output_json(value: impl Serialize) {
    let value = serde_json::to_value(value).expect("serialization failed");
    let wrapped = serde_json::json!({ "data": value });
    println!(
        "{}",
        serde_json::to_string_pretty(&wrapped).expect("serialization failed")
    );
}
