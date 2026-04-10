## DAG Update — Round {{ round }}

New ready nodes are available. Continue orchestrating.

## Ready Nodes
{% for node in ready_nodes %}
- **{{ node.id }}**: {{ node.summary }}
{% endfor %}

Use the **kerrigan-storage** skill to read node details, execute via subagent, and update status.
