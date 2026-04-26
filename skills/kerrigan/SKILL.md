---
name: kerrigan
description: >
  Manage DAG-based task graphs via the kerrigan CLI. Use to view ready nodes,
  read predecessor results, record execution outcomes, add follow-up nodes,
  and check task status.
compatibility: Requires kerrigan 1.1.0+ in PATH
---

# Kerrigan

Manage tasks and their DAG (Directed Acyclic Graph) of work nodes. All commands output JSON.

## Output Format

Every command returns a JSON envelope:

```json
{
  "data": <command-specific result>
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

# 3. Build the DAG — every node must declare a contract
kerrigan dag add --task abc-123 \
  --summary "Analyze current auth flow" \
  --description "Read all files in src/auth/" \
  --contract '{"output":{"type":"object","required":["endpoints"],"properties":{"endpoints":{"type":"array"}}}}'

# Node that depends on the first:
kerrigan dag add --task abc-123 \
  --summary "Design new auth architecture" \
  --depends-on <node1> \
  --contract '{
    "predecessor": {
      "<node1-id>": {"type":"object","required":["endpoints"],"properties":{"endpoints":{"type":"array"}}}
    },
    "output": {"type":"object","required":["design"],"properties":{"design":{"type":"string"}}}
  }'

# 4. Execute ready nodes
kerrigan dag ready --task abc-123
kerrigan dag get --task abc-123 --node <node-id>
# (do the work, then update with valid JSON result_summary)
kerrigan dag update --task abc-123 --input '[{"id":"<node-id>","status":"succeeded","metadata":{"result_summary":"{\"endpoints\":[\"/api/login\",\"/api/logout\"]}"}}]'
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
| `kerrigan dag add --task <id> --summary "..." [--description "..."] [--depends-on a,b] [--successor c,d] --contract '<json>'` | Add a node | `{"node_id": "<uuid>"}` |
| `kerrigan dag list --task <id>` | List all nodes | `[{id, summary, successors, status}]` |
| `kerrigan dag ready --task <id>` | List ready-to-execute nodes | `[{id, summary, successors, status}]` |
| `kerrigan dag get --task <id> --node <id>` | Full node details incl. contract | `{id, summary, description, predecessors, successors, status, metadata, contract}` |
| `kerrigan dag update --task <id> --input '<json>'` | Batch update nodes | `{"ok": true}` |
| `kerrigan dag override --task <id> --node <id> --result-summary "..."` | Bypass schema validation (see below) | `{"ok": true, "node_id": "..."}` |

### Execution & Communication

| Command | Description | `data` field |
|---------|-------------|--------------|
| `kerrigan exec --task <id> --node <id> [--adapter kimi\|claude] [--detach]` | Execute a node | With --detach: `{pid, session_id, log_path}` |
| `kerrigan send --node <id> --message "..." [--interrupt]` | Message a running agent | `{queued, exec_running, action}` |
| `kerrigan stop --node <id>` | Stop a running worker | `{stopped: true/false, ...}` |
| `kerrigan watch <task-id> [--interval 2]` | Stream DAG events (JSONL) | One JSON per line (no envelope) |

## Node Contract (required since v1.1.0)

Every node **must** declare a `--contract` when created. The contract has two parts:

### `predecessor` map
Maps each predecessor node ID to the JSON Schema its `result_summary` must satisfy.
- Must cover **all** nodes in `--depends-on` (bijection: one schema per predecessor, no extras)
- Root nodes (no `--depends-on`): omit or pass empty predecessor

### `output` schema
JSON Schema that **this node's own** `result_summary` must satisfy when marking succeeded.
- Always required, even for root nodes
- Use `{"type":"object"}` as minimum if you don't want strict validation

### Contract format

```json
{
  "predecessor": {
    "<node-id-A>": {
      "type": "object",
      "required": ["field1", "field2"],
      "properties": {
        "field1": {"type": "string"},
        "field2": {"type": "array", "items": {"type": "string"}}
      }
    }
  },
  "output": {
    "type": "object",
    "required": ["summary", "files_changed"],
    "properties": {
      "summary": {"type": "string"},
      "files_changed": {"type": "array"},
      "error_count": {"type": "integer", "maximum": 0}
    }
  }
}
```

### What is validated when

| Timing | What is checked |
|--------|----------------|
| `dag add` | Contract present; output.schema valid; predecessor map covers all depends-on; each predecessor schema is valid JSON Schema; predecessor's declared output.schema satisfies this node's requirement (static subsumption) |
| `dag update --status succeeded` | Each predecessor's actual result_summary validates against this node's predecessor schema; this node's result_summary validates against output.schema |

**Both checks block on failure.** Fix the contract or the result and retry.

## result_summary must be valid JSON

Since v1.1.0, `result_summary` **must be a JSON object** that satisfies the node's declared `output.schema`.

```bash
# ✅ Correct — valid JSON matching schema
kerrigan dag update --task T --input '[{
  "id": "N",
  "status": "succeeded",
  "metadata": {
    "result_summary": "{\"summary\":\"Found 5 issues\",\"files_changed\":[\"src/auth.rs\"]}"
  }
}]'

# ❌ Wrong — plain text is rejected
kerrigan dag update --task T --input '[{"id":"N","status":"succeeded","metadata":{"result_summary":"Done"}}]'
```

## Handling Schema Validation Failures on Update

When `dag update --status succeeded` is blocked because `result_summary` doesn't match `output.schema`:

**Option A — The execution was actually wrong** (fix and retry):
```bash
# Fix the result_summary to match the schema, then retry dag update
```

**Option B — The contract was wrong at creation time** (execution was correct):
```bash
# Use dag override to bypass schema validation
kerrigan dag override --task <task-id> --node <node-id> --result-summary '{"summary":"actual output"}'
```

`dag override` requires:
1. Node is in `Running` state
2. A schema validation failure is on record for this node

It marks the node as `succeeded`, stores the result, and lets downstream nodes proceed normally.

## Node Status Flow

```
pending  ──→  ready  ──→  running  ──→  succeeded
                                    └──→  failed
```

- **pending → ready**: Automatic when all predecessors succeeded (or no predecessors)
- **ready → running**: Set by `kerrigan exec`
- **running → succeeded**: Set by `kerrigan dag update` with valid result_summary
- **running → failed**: Set by `kerrigan dag update` with `status: "failed"`

## dag add — --successor flag

Add a new node as a predecessor of existing nodes:

```bash
kerrigan dag add --task T --summary "N" \
  --successor existing-node-id \
  --contract '{"output":{...}}'
```

- `--successor` is the **reverse** of `--depends-on`: existing nodes will depend on the new node
- If the new node is pending, `--successor` nodes that were ready are automatically demoted to pending
- Cycle detection runs at add time

## Metadata Fields

| Field | When to use | Type |
|-------|------------|------|
| `result_summary` | On success — **must be valid JSON** satisfying output.schema | JSON string |
| `failure_reason` | On failure — root cause and suggested fix direction | string |
| `spawned_tasks` | When creating sub-tasks from within a node | string[] |
| `session_ids` | Track agent session IDs for debugging | string[] |

## Error Reference

| Error | Cause | Recovery |
|-------|-------|----------|
| `not initialized` | ~/.kerrigan/ doesn't exist | Run `kerrigan init` |
| `task not found` | Invalid task ID | Run `kerrigan task list` |
| `node not found` | Invalid node ID | Run `kerrigan dag list --task <id>` |
| `invalid state transition` | Wrong status update order | Check current status with `kerrigan dag get` |
| `node already has an active process` | Exec while running | `kerrigan stop` or `kerrigan send` |
| `cycle detected` | Circular dependency | Check `kerrigan dag list` and remove the circular reference |
| `contract violation` | Missing contract, bijection mismatch, invalid schema syntax | Fix `--contract` JSON and retry `dag add` |
| `output schema validation failed` | result_summary doesn't match output.schema | Fix result_summary (Option A) or use `dag override` (Option B) |
| `has no schema validation failure on record` | `dag override` called without prior schema failure | Use `dag update` instead |

## Important Rules

- Nodes execute **exactly once**. Never retry in place — create new nodes with `dag add`.
- **`--contract` is required** for every `dag add` since v1.1.0.
- **`result_summary` must be valid JSON** satisfying the declared `output.schema`.
- Always write `result_summary` on success — downstream nodes depend on reading it.
- Always write `failure_reason` on failure — it guides recovery.
- Use `dag get` to read predecessor `result_summary` before executing.
- Design multiple independent cross-verification tasks for important judgments.
- Do not plan more than two dependency layers ahead — decompose iteratively.

## Common Mistakes

1. **Missing `--contract`** — Every `dag add` requires `--contract`. No contract = error.
2. **Plain-text `result_summary`** — Must be a JSON object (e.g. `'{"summary":"..."}'`), not free text.
3. **Predecessor map doesn't cover all `--depends-on`** — Every depends-on ID needs a schema entry.
4. **Forgetting to read predecessor results** — Always `dag get` predecessors before executing.
5. **Updating a non-running node** — Only `running → succeeded/failed` is valid. Use `exec` first.
6. **Over-planning the DAG upfront** — Add 1-2 layers, execute, then decide what's next.
7. **Not using cross-validation** — For important decisions, create multiple independent nodes.
