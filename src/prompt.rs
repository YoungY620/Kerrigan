use crate::engine::{RegistryEngine, TaskEngine};
use crate::error::Result;

pub fn build_prompt(
    registry: &RegistryEngine,
    task_engine: &TaskEngine,
    task_id: &str,
    node_id: &str,
) -> Result<String> {
    let task = registry.task_get(task_id)?;
    let node = task_engine.get_node(node_id)?;
    let all_nodes = task_engine.list_all()?;

    // Collect predecessor results
    let mut pred_section = String::new();
    for pred_id in &node.predecessors {
        let pred = task_engine.get_node(pred_id)?;
        pred_section.push_str(&format!("### Node {}: {}\n", pred.id, pred.summary));
        if let Some(ref result) = pred.metadata.result_summary {
            pred_section.push_str(&format!("**Result**: {}\n\n", result));
        } else {
            pred_section.push_str("**Result**: (no result)\n\n");
        }
    }
    if pred_section.is_empty() {
        pred_section = "No predecessors.\n".to_string();
    }

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

## Predecessor Results
{pred_section}

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
3. Use structured result_summary and failure_reason.
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
        dag_json = dag_json,
        skills_path = skills_path,
        guide_path = guide_path,
        task_id = task_id,
    ))
}
