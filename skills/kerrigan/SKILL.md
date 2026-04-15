---
name: kerrigan
description: >
  Manage DAG-based task graphs via the kerrigan CLI. Use to view ready nodes,
  read predecessor results, record execution outcomes, add follow-up nodes,
  and check task status.
compatibility: Requires kerrigan binary in PATH
---

# Kerrigan

Manage tasks and their DAG (Directed Acyclic Graph) of work nodes. All commands output JSON.

## Output Format

Every command returns a JSON envelope:

```json
{
  "data": <command-specific result>,
  "reminder": "How to use Kerrigan well? ..."
}
```

**Always access the result via the `data` field.**

Errors go to stderr in bare format: `{"error": "<actionable message with recovery instructions>"}`

## Quick Start

```bash
# 1. Initialize (run once, idempotent)
kerrigan init

# 2. Create a task
kerrigan task create --summary "Refactor auth module" --workdir /home/user/project
# → data: {"task_id": "abc-123"}

# 3. Build the DAG
kerrigan dag add --task abc-123 --summary "Analyze current auth flow" --description "Read all files in src/auth/"
kerrigan dag add --task abc-123 --summary "Review security requirements" --description "Check docs/security.md"
kerrigan dag add --task abc-123 --summary "Design new auth architecture" --depends-on <node1>,<node2>

# 4. Execute ready nodes
kerrigan dag ready --task abc-123
kerrigan dag get --task abc-123 --node <node-id>
# (do the work)
kerrigan dag update --task abc-123 --input '[{"id":"<node-id>","status":"succeeded","metadata":{"result_summary":"Found 3 REST endpoints."}}]'
```

## Command Reference

### Task Management

| Command | Description | `data` field |
|---------|-------------|--------------|
| `kerrigan init` | Create ~/.kerrigan/ (idempotent) | `{"ok": true}` |
| `kerrigan task create --summary "..." --workdir .` | Create a new task | `{"task_id": "<uuid>"}` |
| `kerrigan task list` | List all registered tasks | `[{"id", "summary", "workdir", "created_at"}]` |
| `kerrigan status` | List all tasks (alias) | Same as task list |
| `kerrigan status <task-id>` | Show task's DAG overview | Array of NodeSummary |

### DAG Operations

| Command | Description | `data` field |
|---------|-------------|--------------|
| `kerrigan dag add --task <id> --summary "..." [--description "..."] [--depends-on a,b]` | Add a node | `{"node_id": "<uuid>"}` |
| `kerrigan dag list --task <id>` | List all nodes | `[{id, summary, successors, status}]` |
| `kerrigan dag ready --task <id>` | List ready-to-execute nodes | `[{id, summary, successors, status}]` |
| `kerrigan dag get --task <id> --node <id>` | Full node details | `{id, summary, description, predecessors, successors, status, metadata}` |
| `kerrigan dag update --task <id> --input '<json>'` | Batch update nodes | `{"ok": true}` |

### Execution & Communication

| Command | Description | `data` field |
|---------|-------------|--------------|
| `kerrigan exec --task <id> --node <id> [--adapter claude] [--detach]` | Execute a node | With --detach: `{pid, session_id, log_path}` |
| `kerrigan send --node <id> --message "..." [--interrupt]` | Message a running agent | `{queued, exec_running, action}` |
| `kerrigan stop --node <id>` | Stop a running worker | `{stopped: true/false, ...}` |
| `kerrigan watch <task-id> [--interval 2]` | Stream DAG events (JSONL) | One JSON per line (no envelope) |

## Node Status Flow

```
pending  ──→  ready  ──→  running  ──→  succeeded
                                    └──→  failed
```

- **pending → ready**: Automatic when all predecessors have succeeded (or immediately if no predecessors)
- **ready → running**: Set by `kerrigan exec`
- **running → succeeded**: Set by `kerrigan dag update` with `status: "succeeded"`
- **running → failed**: Set by `kerrigan dag update` with `status: "failed"`

Any other transition is invalid and will return an error.

## Metadata Fields

When updating a node via `dag update`, the `metadata` object supports:

| Field | When to use | Type |
|-------|------------|------|
| `result_summary` | On success — describe what was accomplished and key outputs | string |
| `failure_reason` | On failure — describe root cause and suggested fix direction | string |
| `spawned_tasks` | When creating sub-tasks from within a node | string[] |
| `session_ids` | Track agent session IDs for debugging | string[] |

**Always write `result_summary` on success** — downstream nodes depend on reading it.
**Always write `failure_reason` on failure** — it guides recovery.

## dag update Input Format

The `--input` flag accepts a JSON array of update objects:

```json
[
  {
    "id": "<node-id>",
    "status": "succeeded",
    "metadata": {
      "result_summary": "## Execution Summary\nFound 3 REST endpoints.\n\n## Key Outputs\n- src/api/auth.rs modified\n\n## Recommendations for Downstream Nodes\nUse JWT for session management."
    }
  }
]
```

You can update multiple nodes in one call. If `--input` is omitted, reads from stdin.

## Error Handling

All errors include actionable recovery instructions. Common errors:

| Error | Cause | Recovery |
|-------|-------|----------|
| `not initialized` | ~/.kerrigan/ doesn't exist | Run `kerrigan init` |
| `task not found` | Invalid task ID | Run `kerrigan task list` to see valid IDs |
| `node not found` | Invalid node ID | Run `kerrigan dag list --task <id>` to see valid IDs |
| `invalid state transition` | Wrong status update order | Check current status with `kerrigan dag get`, follow the valid flow |
| `node already has an active process` | Trying to exec while running | `kerrigan stop --node <id>` first, or use `kerrigan send` |
| `cycle detected` | Circular dependency in --depends-on | Check `kerrigan dag list` and remove the circular reference |
| `json parse error` | Malformed --input JSON | Use format: `'[{"id":"...","status":"...","metadata":{...}}]'` |

## Examples

### Execute ready nodes

```bash
# 1. List ready nodes
kerrigan dag ready --task abc-123

# 2. Get full details and predecessor results before executing
kerrigan dag get --task abc-123 --node n1

# 3. (Execute the work)

# 4. Record the result
kerrigan dag update --task abc-123 --input '[{"id":"n1","status":"succeeded","metadata":{"result_summary":"Found 3 REST endpoints."}}]'
```

### Handle failure and spawn recovery

```bash
# Mark the node as failed
kerrigan dag update --task abc-123 --input '[{"id":"n2","status":"failed","metadata":{"failure_reason":"Database connection refused on port 5432"}}]'

# Add a fix node, then a retry node that depends on the fix
kerrigan dag add --task abc-123 --summary "Fix: ensure database is running" --description "Check docker-compose status and restart postgres if needed"
kerrigan dag add --task abc-123 --summary "Retry: migrate database schema" --depends-on <fix-node-id>
```

### Cross-validation pattern

```bash
# Create independent analysis nodes for cross-validation
kerrigan dag add --task abc-123 --summary "Analyze performance via profiling" --description "Run flamegraph on src/api/"
kerrigan dag add --task abc-123 --summary "Analyze performance via benchmarks" --description "Run criterion benchmarks in benches/"
# Aggregation node depends on both
kerrigan dag add --task abc-123 --summary "Compare and synthesize performance findings" --depends-on <profiling-node>,<benchmark-node>
```

## Important Rules

- Nodes execute **exactly once**. Never retry in place — create new nodes with `dag add`.
- Always write `result_summary` on success — downstream nodes depend on this.
- Always write `failure_reason` on failure — it guides recovery.
- Use `dag get` to read predecessor `result_summary` before executing.
- Status flow: `pending` → `ready` → `running` → `succeeded` / `failed`.
- Design multiple independent cross-verification tasks for important judgments.
- Do not plan more than two dependency layers ahead — decompose iteratively based on results.

## Common Mistakes

1. **Forgetting to read predecessor results** — Always `dag get` predecessors before executing. Their `result_summary` is your input context.
2. **Empty result_summary** — Downstream nodes will have no context. Always document what you found/did.
3. **Updating a non-running node** — You can only transition `running → succeeded/failed`. If the node is still `ready`, use `exec` first.
4. **Over-planning the DAG upfront** — Add 1-2 layers of nodes, execute them, then decide what's next based on results.
5. **Not using cross-validation** — For important decisions, create multiple independent nodes that assess the same question from different angles, then aggregate.
6. **Creating dependencies where none are needed** — Only depend on nodes whose output you actually need. Extra dependencies block parallelism.
