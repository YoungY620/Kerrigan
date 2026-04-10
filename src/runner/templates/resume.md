You are resuming orchestration of a Kerrigan DAG task in a **new session**.

## Task
{{ task_summary }}

## Context
This is a fresh session. You need to understand the current state before proceeding.

**Important:** Read all available information carefully:
1. Run `kerrigan-storage list --task {{ task_id }}` to see the full DAG overview
2. For any node that interests you, use `kerrigan-storage get --task {{ task_id }} --node <node-id>` to read details
3. Pay special attention to `result_summary` of succeeded predecessors — they contain information you need

## Ready Nodes
{% for node in ready_nodes %}
- **{{ node.id }}**: {{ node.summary }}
{% endfor %}

## Rules
- Each node executes **exactly once**. Never retry in place — create new nodes instead.
- Always write `result_summary` when marking a node as `succeeded`.
- Always write `failure_reason` when marking a node as `failed`.
- When done with all ready nodes, stop.
