# Kerrigan Storage - DAG Management Skill

Use this skill to manage tasks and their DAG (Directed Acyclic Graph) of work nodes. All commands output JSON.

## When to use

- When you need to view, create, or update nodes in a task's DAG
- When you need to check which nodes are ready for execution
- When you need to read a node's details or predecessor results before executing work
- When you need to record execution results or spawn follow-up nodes after completing work

## Available operations

### Task management

**List all tasks:**
```bash
kerrigan-storage task list
```
Returns: JSON array of `{id, summary, workdir, created_at}`.

**Create a new task:**
```bash
kerrigan-storage task create --summary "description of the task" --workdir /path/to/workdir
```
Returns: `{"task_id": "..."}`.

### DAG node operations

All node operations require `--task <task-id>` to specify which task's DAG to operate on.

**List all nodes (overview):**
```bash
kerrigan-storage list --task <task-id>
```
Returns: JSON array of `{id, summary, successors, status}`. Use this to understand the full DAG topology and progress.

**List ready nodes:**
```bash
kerrigan-storage ready --task <task-id>
```
Returns: JSON array of nodes with status `ready` - these are the nodes you should execute next.

**Get node details:**
```bash
kerrigan-storage get --task <task-id> --node <node-id>
```
Returns: Full node JSON including `description`, `metadata.result_summary` (from predecessors), and `metadata.failure_reason`. Use this to read predecessor results before starting work on a dependent node.

**Add a new node:**
```bash
kerrigan-storage add --task <task-id> --summary "what this node does" --description "detailed markdown description" --depends-on node-id-1,node-id-2
```
Returns: `{"node_id": "..."}`. The node starts as `ready` if it has no dependencies or all dependencies are already `succeeded`; otherwise it starts as `pending`.

**Update nodes (batch):**
```bash
kerrigan-storage update --task <task-id> --input '<json>'
```
Input is a JSON array of updates. Each update has `id` (required), and optional `status` and `metadata` fields.

Status values: `pending`, `ready`, `running`, `succeeded`, `failed`.

Example - mark a node as succeeded with results:
```bash
kerrigan-storage update --task <task-id> --input '[{"id":"<node-id>","status":"succeeded","metadata":{"result_summary":"Completed successfully. Found 3 API endpoints that need updating."}}]'
```

Example - mark a node as failed:
```bash
kerrigan-storage update --task <task-id> --input '[{"id":"<node-id>","status":"failed","metadata":{"failure_reason":"Could not access the database. Connection refused."}}]'
```

## Typical workflow

1. **Check ready nodes:** `kerrigan-storage ready --task <task-id>`
2. **For each ready node:**
   a. **Read details:** `kerrigan-storage get --task <task-id> --node <node-id>` to get the full description and predecessor results
   b. **Execute the work** using subagent
   c. **Record results:** `kerrigan-storage update --task <task-id> --input '[{"id":"<node-id>","status":"succeeded","metadata":{"result_summary":"..."}}]'`
3. **On failure:** Update with `failed` status and `failure_reason`, then add new retry/fix nodes:
   ```bash
   kerrigan-storage update --task <task-id> --input '[{"id":"<node-id>","status":"failed","metadata":{"failure_reason":"..."}}]'
   kerrigan-storage add --task <task-id> --summary "retry: fix the issue" --description "..." --depends-on <other-node-id>
   ```
4. **Repeat** until `kerrigan-storage ready --task <task-id>` returns an empty list.

## Important rules

- Nodes execute **exactly once**. Never retry a node in place. Instead, create a new node with `add` and set up dependencies.
- Always write a `result_summary` when marking a node as `succeeded` - downstream nodes depend on this information.
- Always write a `failure_reason` when marking a node as `failed`.
- Use `get` to read predecessor results before executing a node - this is how information flows between nodes without direct communication.
