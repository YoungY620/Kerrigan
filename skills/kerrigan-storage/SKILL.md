---
name: kerrigan-storage
description: >
  Manage DAG-based task graphs via the kerrigan-storage CLI. Use when you need
  to view ready nodes, read predecessor results, record execution outcomes,
  or spawn follow-up nodes in a Kerrigan task DAG.
compatibility: Requires kerrigan-storage binary in PATH
---

# Kerrigan Storage

Manage tasks and their DAG (Directed Acyclic Graph) of work nodes. All commands output JSON.

## Quick Start

```bash
# See what's ready to execute
kerrigan-storage ready --task <task-id>

# Read a node's details and predecessor results
kerrigan-storage get --task <task-id> --node <node-id>

# Mark a node as succeeded with results
kerrigan-storage update --task <task-id> --input '[{"id":"<node-id>","status":"succeeded","metadata":{"result_summary":"Found 3 files that need updating."}}]'
```

## Examples

### Check ready nodes and execute them

```bash
# 1. List ready nodes
kerrigan-storage ready --task abc-123
# Output: [{"id":"n1","summary":"Analyze API endpoints","successors":["n3"],"status":"ready"}, ...]

# 2. Get full details before executing
kerrigan-storage get --task abc-123 --node n1
# Output: {"id":"n1","summary":"...","description":"...","predecessors":[],"successors":["n3"],"status":"ready","metadata":{}}

# 3. (Execute the work via subagent)

# 4. Record the result
kerrigan-storage update --task abc-123 --input '[{"id":"n1","status":"succeeded","metadata":{"result_summary":"Found 3 REST endpoints: /users, /orders, /products. All use JSON. Auth via Bearer token."}}]'
```

### Handle failure and spawn retry

```bash
# Mark the node as failed
kerrigan-storage update --task abc-123 --input '[{"id":"n2","status":"failed","metadata":{"failure_reason":"Database connection refused on port 5432"}}]'

# Add a fix node, then a retry node that depends on the fix
kerrigan-storage add --task abc-123 --summary "Fix: ensure database is running" --description "Check docker-compose status and restart postgres if needed"
# Output: {"node_id":"n4"}

kerrigan-storage add --task abc-123 --summary "Retry: migrate database schema" --description "Re-attempt the migration after database is confirmed running" --depends-on n4
# Output: {"node_id":"n5"}
```

### Create a task and build its DAG

```bash
# Create a task
kerrigan-storage task create --summary "Refactor auth module" --workdir /home/user/project
# Output: {"task_id":"abc-123"}

# Add root nodes (no dependencies = immediately ready)
kerrigan-storage add --task abc-123 --summary "Analyze current auth flow" --description "Read all files in src/auth/ and document the current authentication flow"
# Output: {"node_id":"n1"}

kerrigan-storage add --task abc-123 --summary "Review security requirements" --description "Check docs/security.md for compliance requirements"
# Output: {"node_id":"n2"}

# Add a dependent node
kerrigan-storage add --task abc-123 --summary "Design new auth architecture" --description "Based on analysis and requirements, design the new auth module" --depends-on n1,n2
# Output: {"node_id":"n3"}
```

### View DAG overview

```bash
kerrigan-storage list --task abc-123
# Output: [
#   {"id":"n1","summary":"Analyze current auth flow","successors":["n3"],"status":"succeeded"},
#   {"id":"n2","summary":"Review security requirements","successors":["n3"],"status":"ready"},
#   {"id":"n3","summary":"Design new auth architecture","successors":[],"status":"pending"}
# ]
```

## Command Reference

| Command | Description |
|---------|-------------|
| `task list` | List all registered tasks |
| `task create --summary "..." --workdir .` | Create a new task |
| `list --task <id>` | List all nodes (id, summary, successors, status) |
| `ready --task <id>` | List nodes ready for execution |
| `get --task <id> --node <id>` | Get full node details |
| `add --task <id> --summary "..." [--description "..."] [--depends-on a,b]` | Add a new node |
| `update --task <id> --input '<json>'` | Batch update nodes (status, metadata) |

## ⚠️ Important Rules

- Nodes execute **exactly once**. Never retry in place — create new nodes with `add`.
- Always write `result_summary` on success — downstream nodes depend on this.
- Always write `failure_reason` on failure.
- Use `get` to read predecessor `result_summary` before executing — this is how nodes communicate.
- Status flow: `pending` → `ready` → `running` → `succeeded` / `failed`.
