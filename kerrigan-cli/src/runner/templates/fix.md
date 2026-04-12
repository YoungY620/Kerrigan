## DAG Consistency Issues Detected

The following issues were found after your last round. Use the **kerrigan-storage** skill to fix them.

{% for issue in issues %}
- **{{ issue.node_id }}**: {{ issue.description }}
{% endfor %}

Fix all issues above, then stop. The framework will validate again after you're done.
