## DAG Consistency Issues Detected

The following issues were found after your last round. Please fix them using `kerrigan-storage update` or `kerrigan-storage add`.

{% for issue in issues %}
- **{{ issue.node_id }}**: {{ issue.description }}
{% endfor %}

Fix all issues above, then stop. The framework will validate again after you're done.
