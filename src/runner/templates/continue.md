## DAG Update — Round {{ round }}

New ready nodes are available. Continue orchestrating.

## Ready Nodes
{% for node in ready_nodes %}
- **{{ node.id }}**: {{ node.summary }}
{% endfor %}

Use `kerrigan-storage get --task {{ task_id }} --node <node-id>` to read details and predecessor results, then execute each node via subagent and update status.
