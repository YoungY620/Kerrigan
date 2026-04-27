use crate::engine::{RegistryEngine, TaskEngine};
use crate::error::Result;

fn extract_by_schema(
    result_json: &serde_json::Value,
    schema: &serde_json::Value,
) -> serde_json::Value {
    let props: std::collections::HashSet<&str> = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|obj| obj.keys().map(String::as_str).collect())
        .unwrap_or_default();

    if props.is_empty() {
        return result_json.clone();
    }

    if let Some(obj) = result_json.as_object() {
        let filtered: serde_json::Map<_, _> = obj
            .iter()
            .filter(|(k, _)| props.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        serde_json::Value::Object(filtered)
    } else {
        result_json.clone()
    }
}

pub fn build_prompt(
    registry: &RegistryEngine,
    task_engine: &TaskEngine,
    task_id: &str,
    node_id: &str,
) -> Result<String> {
    let task = registry.task_get(task_id)?;
    let node = task_engine.get_node(node_id)?;
    let all_nodes = task_engine.list_all()?;

    // Collect predecessor inputs — filtered to declared schema fields
    let mut pred_section = String::new();
    for pred_id in &node.predecessors {
        let pred = task_engine.get_node(pred_id)?;
        pred_section.push_str(&format!("### Node {}: {}\n", pred.id, pred.summary));

        let schema = node
            .contract
            .as_ref()
            .and_then(|c| c.predecessor.as_ref())
            .and_then(|p| p.get(pred_id.as_str()));

        if let Some(ref result_str) = pred.metadata.result_summary {
            let display = if let Some(schema) = schema {
                if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(result_str) {
                    let extracted = extract_by_schema(&result_json, schema);
                    serde_json::to_string_pretty(&extracted).unwrap_or_else(|_| result_str.clone())
                } else {
                    result_str.clone()
                }
            } else {
                result_str.clone()
            };
            pred_section.push_str(&format!("```json\n{}\n```\n\n", display));
        } else {
            pred_section.push_str("*(no result yet)*\n\n");
        }
    }
    if pred_section.is_empty() {
        pred_section = "No predecessors.\n".to_string();
    }

    // Output schema for this node
    let output_schema_str = node
        .contract
        .as_ref()
        .and_then(|c| c.output.as_ref())
        .and_then(|s| serde_json::to_string_pretty(s).ok())
        .unwrap_or_else(|| "not declared".to_string());

    let dag_json = serde_json::to_string_pretty(&all_nodes).unwrap_or_default();

    let skills_path = registry
        .skills_dir()
        .join("kerrigan")
        .join("SKILL.md")
        .to_string_lossy()
        .into_owned();
    let guide_path = registry
        .skills_dir()
        .join("kerrigan")
        .join("guide.md")
        .to_string_lossy()
        .into_owned();

    Ok(format!(
        r#"You are executing a node in a Kerrigan multi-agent DAG task.

## Task
{task_summary}

## Your Assignment
- **Node ID**: {node_id}
- **Summary**: {node_summary}

### Description
{node_description}

## Predecessor Inputs
{pred_section}

## Your Output Schema
Your `result_summary` must satisfy this JSON Schema:
```json
{output_schema_str}
```

## DAG Overview
```json
{dag_json}
```

## Tools
You have the **kerrigan** skill available (docs at `{skills_path}`).
Read the guide at `{guide_path}` for execution principles and result templates.

Key commands:
- `kerrigan dag get --task {task_id} --node <id>` — read node details
- `kerrigan dag add --task {task_id} --summary "..." --depends-on ...` — add follow-up nodes
- `kerrigan dag update --task {task_id} --input '[...]'` — update node status/metadata

## Rules
1. Execute your assigned node thoroughly.
2. When done, update your node status:
   - Success: `kerrigan dag update --task {task_id} --input '[{{"id":"{node_id}","status":"succeeded","metadata":{{"result_summary":"..."}}}}]'`
   - Failure: `kerrigan dag update --task {task_id} --input '[{{"id":"{node_id}","status":"failed","metadata":{{"failure_reason":"..."}}}}]'`
3. `result_summary` must be valid JSON matching Your Output Schema above.
4. You may add new nodes if you discover additional work needed.
5. Never hide limitations — disclose all known issues.
6. When finished, stop."#,
        task_summary = task.summary,
        node_id = node.id,
        node_summary = node.summary,
        node_description = if node.description.is_empty() {
            "(no description)"
        } else {
            &node.description
        },
        pred_section = pred_section.trim(),
        output_schema_str = output_schema_str,
        dag_json = dag_json,
        skills_path = skills_path,
        guide_path = guide_path,
        task_id = task_id,
    ))
}
