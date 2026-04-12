You are resuming orchestration of a Kerrigan DAG task in a **new session**.

**This is a fresh session. Read the guide file at `{{ guide_path }}`** to understand working principles, decomposition strategy, and result templates. Also read the relevant scenario guides linked within it.

## Task
{{ task_summary }}

## Context
Use the **kerrigan-storage** skill (docs at `{{ skills_path }}`) to:
1. View the full DAG overview to understand topology and progress
2. Read details of relevant nodes, especially `result_summary` of succeeded predecessors
3. Then proceed with executing the ready nodes below

## Ready Nodes
{% for node in ready_nodes %}
- **{{ node.id }}**: {{ node.summary }}
{% endfor %}

## Rules
- Each node executes **exactly once**. Never retry in place — create new nodes instead.
- Always write structured `result_summary` / `failure_reason` (see guide for templates).
- **Never hide known issues** — disclose all limitations in result_summary.
- When done with all ready nodes, stop.
