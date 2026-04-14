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

## Quick Start

```bash
# See what's ready to execute
kerrigan dag ready --task <task-id>

# Read a node's details and predecessor results
kerrigan dag get --task <task-id> --node <node-id>

# Mark a node as succeeded with results
kerrigan dag update --task <task-id> --input '[{"id":"<node-id>","status":"succeeded","metadata":{"result_summary":"Found 3 files that need updating."}}]'

# Add a follow-up node
kerrigan dag add --task <task-id> --summary "Implement the fix" --depends-on <node-id>
```

## Examples

### Check ready nodes and execute them

```bash
# 1. List ready nodes
kerrigan dag ready --task abc-123

# 2. Get full details before executing
kerrigan dag get --task abc-123 --node n1

# 3. (Execute the work)

# 4. Record the result
kerrigan dag update --task abc-123 --input '[{"id":"n1","status":"succeeded","metadata":{"result_summary":"Found 3 REST endpoints."}}]'
```

### Handle failure and spawn retry

```bash
# Mark the node as failed
kerrigan dag update --task abc-123 --input '[{"id":"n2","status":"failed","metadata":{"failure_reason":"Database connection refused on port 5432"}}]'

# Add a fix node, then a retry node that depends on the fix
kerrigan dag add --task abc-123 --summary "Fix: ensure database is running" --description "Check docker-compose status and restart postgres if needed"
kerrigan dag add --task abc-123 --summary "Retry: migrate database schema" --depends-on <fix-node-id>
```

### Create a task and build its DAG

```bash
kerrigan task create --summary "Refactor auth module" --workdir /home/user/project
# → {"task_id":"abc-123"}

kerrigan dag add --task abc-123 --summary "Analyze current auth flow" --description "Read all files in src/auth/"
kerrigan dag add --task abc-123 --summary "Review security requirements" --description "Check docs/security.md"
kerrigan dag add --task abc-123 --summary "Design new auth architecture" --depends-on <node1>,<node2>
```

## Command Reference

| Command | Description |
|---------|-------------|
| `kerrigan task create --summary "..." --workdir .` | Create a new task |
| `kerrigan task list` | List all registered tasks |
| `kerrigan dag add --task <id> --summary "..." [--description "..."] [--depends-on a,b]` | Add a new node |
| `kerrigan dag list --task <id>` | List all nodes (id, summary, successors, status) |
| `kerrigan dag ready --task <id>` | List nodes ready for execution |
| `kerrigan dag get --task <id> --node <id>` | Get full node details |
| `kerrigan dag update --task <id> --input '<json>'` | Batch update nodes (status, metadata) |
| `kerrigan status` | List all tasks |
| `kerrigan status <task-id>` | Show task's DAG overview |

## Important Rules

- Nodes execute **exactly once**. Never retry in place — create new nodes with `dag add`.
- Always write `result_summary` on success — downstream nodes depend on this.
- Always write `failure_reason` on failure.
- Use `dag get` to read predecessor `result_summary` before executing.
- Status flow: `pending` → `ready` → `running` → `succeeded` / `failed`.
