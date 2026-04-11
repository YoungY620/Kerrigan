## DAG Update — Round {{ round }}

New ready nodes are available. Continue orchestrating.

## Ready Nodes
{% for node in ready_nodes %}
- **{{ node.id }}**: {{ node.summary }}
{% endfor %}

Use the **kerrigan-storage** skill (docs at `{{ skills_path }}`) to read node details, execute via subagent, and update status. Follow the execution workflow and result templates in the guide (`{{ guide_path }}`).
