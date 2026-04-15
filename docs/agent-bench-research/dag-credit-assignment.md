# DAG Responsibility Chains and Credit Assignment

## Core Problem

In a multi-agent DAG (A → B → C), when the final output fails a benchmark test, which node's prompt should be optimized?

## Credit Assignment Approaches

### 1. Counterfactual / Shapley-Based

**Method**: Remove or replace one agent's output with a "gold" alternative, re-run downstream nodes, measure the delta.

**Pros**: Theoretically clean, principled attribution
**Cons**: O(n) full re-runs per evaluation (expensive for large DAGs)

**Best for**: Small DAGs (3-7 nodes) where precision matters.

**Research**:
- [HiveMind (2024)](https://arxiv.org/pdf/2512.06432) -- Shapley values for agent contribution measurement
- [Explainable Credit Assignment (AAMAS 2025)](https://arxiv.org/abs/2502.16863) -- LLM-guided Shapley for MARL

### 2. LLM-as-Judge

**Method**: A separate LLM reads the full execution trace and assigns per-node blame scores.

**Pros**: Cheap (single LLM call), scales to large graphs
**Cons**: Noisy, judge LLM may have its own biases

**Best for**: Large DAGs where exhaustive ablation is too expensive.

**Research**:
- [Turn-Level Credit Assignment (2025)](https://arxiv.org/pdf/2505.11821) -- 100K+ token multi-turn trajectories

### 3. End-to-End Joint Optimization (DSPy approach)

**Method**: Don't explicitly assign blame. Instead, optimize all node prompts jointly using end-to-end metric. The optimizer implicitly finds which nodes benefit most from prompt changes.

**Pros**: No per-node ground truth needed, simple to implement
**Cons**: Noisy gradient signal, may miss non-obvious bottlenecks

**Best for**: Starting point. Use ablation to diagnose when optimization plateaus.

### 4. Intermediate Checkpoints

**Method**: Add validation at DAG boundaries. Each node's output must satisfy format/quality constraints before passing downstream.

**Pros**: Catches errors early, provides per-node signal
**Cons**: Requires designing per-node validation criteria

**Examples**:
- Planner node: output must be valid JSON task list
- Researcher node: output must contain at least N citations
- Coder node: output must parse and pass basic syntax check
- Reviewer node: must reference specific code lines

## Practical Recommendation for Kerrigan

Use a layered approach (in order of implementation priority):

### Layer 1: End-to-End DSPy Optimization
```
Kerrigan DAG → DSPy Module Graph → MIPROv2 + benchmark metric → optimized prompts
```
Start here. No per-node ground truth needed. DSPy handles credit assignment implicitly.

### Layer 2: Intermediate Checkpoints
Add format/quality validators at DAG edges:
```python
# After Planner node
assert is_valid_json(planner_output), "Planner must output valid JSON plan"
assert len(plan_steps) >= 2, "Plan must have at least 2 steps"

# After Coder node  
assert python_syntax_check(code), "Code must be syntactically valid"

# After Reviewer node
assert 'approved' in review or 'rejected' in review, "Review must have verdict"
```

### Layer 3: Ablation Diagnostics
When optimization plateaus, identify the bottleneck:
```python
for node in dag.nodes:
    # Replace this node's output with gold/ideal output
    # Re-run downstream nodes
    # If final score improves significantly → this node is the bottleneck
    delta = run_ablation(dag, node, gold_output=ideal_outputs[node])
    print(f"Node {node}: ablation delta = {delta}")
```

## From Test Benchmarks to "Training"

The key insight: benchmark test suites become training data for prompt optimization.

```
Feedback Loop:
1. Run DAG on benchmark suite
2. Collect pass/fail per test case
3. Credit-assign to nodes (implicit via DSPy, or explicit via ablation)
4. Optimize worst-performing node's prompt
5. Re-evaluate on held-out set
6. Repeat until convergence
```

This is essentially **online prompt evolution** with test suites as the fitness function.

**Critical caveat**: Small test suites lead to prompt overfitting. Always maintain:
- Training set: used by optimizer
- Validation set: early stopping / hyperparameter selection
- Test set: final evaluation only, never shown to optimizer

## Key Challenges

1. **Compositional blame**: Node A's bad plan causes Node C's code to fail. Naive ablation blames C. Need trace-aware attribution.
2. **Prompt interaction effects**: Optimizing Node B may degrade Node D if they share implicit assumptions. Joint optimization (DSPy MIPROv2) mitigates this.
3. **Overfitting**: Small benchmarks + many optimization iterations = prompt overfitting. Use holdout sets.
4. **Cost**: Each optimization iteration requires full DAG execution. Cache intermediate results at DAG edges.

## Related Work

- [MASS (2025)](https://arxiv.org/abs/2502.02533) -- Joint prompt + topology optimization for multi-agent systems
- [Tests as Prompt (2025)](https://arxiv.org/abs/2505.09027) -- Using test suites as both specification and evaluation
- [LLM-Guided Credit Assignment in MARL (2025)](https://arxiv.org/abs/2502.03723)
- [Trace is the Next AutoDiff (NeurIPS 2024)](https://proceedings.neurips.cc/paper_files/paper/2024/file/83ba7056bce2c3c3c27e17397cf3e1f0-Paper-Conference.pdf)
