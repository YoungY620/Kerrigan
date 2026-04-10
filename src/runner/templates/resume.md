You are resuming orchestration of a Kerrigan DAG task in a **new session**.

## Task
{{ task_summary }}

## Context
This is a fresh session. You need to understand the current state before proceeding.

**Important:** Use the **kerrigan-storage** skill to:
1. View the full DAG overview to understand topology and progress
2. Read details of relevant nodes, especially `result_summary` of succeeded predecessors
3. Then proceed with executing the ready nodes below

## Ready Nodes
{% for node in ready_nodes %}
- **{{ node.id }}**: {{ node.summary }}
{% endfor %}

## Rules
- Each node executes **exactly once**. Never retry in place — create new nodes instead.
- Always write `result_summary` when marking a node as `succeeded`.
- Always write `failure_reason` when marking a node as `failed`.
- When done with all ready nodes, stop.
