# Kerrigan Guide

You are executing a task using the Kerrigan DAG framework. This document contains universal principles and conventions applicable to all tasks. Please read it before starting your work.

Depending on the type of your current task, you should also read the corresponding scenario guide (see the "Scenario Guides" section at the end).

---

## Execution Workflow

After receiving a ready node, follow this workflow:

1. **Read node details**: `kerrigan-storage get --task <task-id> --node <node-id>`
2. **Read the result_summary of all predecessor nodes**: Run `get` for each predecessor to understand upstream outputs
3. **Determine whether predecessor information is sufficient**:
   - Sufficient -> Proceed with execution
   - Insufficient -> Specify what information is missing in failure_reason, **do not force execution when information is lacking**
4. **Execute the task**
5. **Determine whether derived work is needed**:
   - New subtasks discovered -> Create new nodes using `add`
   - Not needed -> Continue
6. **Write back results**: Use `update` to write status + result_summary or failure_reason

---

## Task Decomposition Principles

### Execute First, Decompose Later

- **Do not plan more than two dependency layers ahead.** Planning depth (sequential chain length) should not exceed two steps — don't try to predict what happens after the next step's results come in. However, the number of parallel nodes at each layer is unlimited and should be as many as the task requires (e.g., multiple independent cross-validation nodes, parallel analysis from different angles). In subsequent rounds, decide whether further depth is needed based on execution results.
- **Act first, but with small-scale probing.** When unsure whether decomposition is needed, start with a minimal-scope attempt to verify feasibility and manageable complexity. Ensure each step can be promptly rolled back to avoid going too far astray. If complexity exceeds expectations, stop immediately and decompose into sub-nodes.
- **Validate feasibility before full implementation.** Before committing to full execution, use multiple independent cross-validation nodes to assess whether the approach is viable, dependencies are met, and assumptions hold. Eliminate infeasible paths at minimal cost.
- **Existing plans can be restructured at any time.** When you find the decomposition is suboptimal, add new nodes and establish new dependencies to adjust the path.
- **Do not over-refine prematurely.** Planning without empirical evidence is just guessing, and the cost of guessing wrong is an entire subtree going to waste.

### Granularity Guidelines

- A node should represent "work that a single agent can reasonably complete in one session"
- If the description exceeds 500 words and still isn't clear -> it should be further decomposed
- If it can be done with a single command -> the granularity may be too fine; consider merging

### Dependencies = Information Requirements

A node's predecessors are exactly the nodes whose outputs it needs to execute. No more, no less — extra dependencies add noise and block parallelism; missing dependencies cause the node to fail from insufficient context.

Parallelize wherever possible. Only establish an A->B dependency when B genuinely requires A's output to begin.

---

## Node Description Conventions

### summary

One sentence, starting with a verb, clearly stating what needs to be done.

Good: `Analyze call relationships of all authentication-related code under src/auth/`
Bad: `Authentication module`

### description

A self-contained task specification. Assume the executor knows nothing about the project. Must include:

- **Objective**: What this node should produce
- **Context**: Which predecessor node results to reference (node IDs)
- **Scope**: Which files/directories/resources to operate on
- **Constraints**: What should not be done, where the boundaries are
- **Output requirements**: What information must be included in result_summary

---

## Result Feedback Conventions

### result_summary Structure

```text
## Execution Summary
[One-sentence conclusion]

## Key Outputs
[Specific files changed / findings / artifacts produced -- verifiable outputs]

## Recommendations for Downstream Nodes
[Information that downstream agents need to know]

## Known Limitations and Unfinished Items
[Must disclose honestly: uncovered scenarios, issues temporarily worked around, skipped tests, compromises made, known but unfixed defects, TODOs/FIXMEs. Concealment is more harmful than the defect itself -- downstream nodes will operate on false assumptions]
```

### failure_reason Structure

```text
## Failure Reason
[Root cause category: insufficient upstream input / unclear task description / insufficient execution capability / external dependency issue]

## Methods Attempted
[What was done, how far it progressed, what specific errors occurred]

## Suggested Fix Direction
[What nodes should be added, what dependencies should be modified, what information should be supplemented]
```

---

## Transparent Disclosure

**Do not conceal any known issues.** Anything not done, temporarily worked around, tests skipped, compromises made -- all must be honestly documented in the "Known Limitations" section of result_summary.

- Skipped certain test cases -> State which ones and why they were skipped
- Worked around an error -> State the workaround method and potential risks
- Made a compromise -> State the reason for the compromise and the direction of a better approach
- Have TODOs / FIXMEs -> Reflect them in result_summary; do not leave them only in code comments

---

## Cross-Validation

**Do not blindly trust the result of any single task.** Important judgments must have multi-perspective cross-validation.

At the DAG level: Multiple independent nodes make judgments on the same question, and an aggregation node compares the conclusions. Consistent conclusions are trustworthy; inconsistent ones trigger further investigation.

---

## Task Identity

The framework does not bind fixed roles, but each node has its own category. When dispatching a subagent, describe a temporary identity based on the node category:

- Analysis type: Tell the subagent "You are an analyst, your objective is..."
- Implementation type: Tell the subagent "You are a developer, your objective is..."
- Review type: Tell the subagent "You are an independent reviewer, your objective is to find issues..."

This is a temporary identity description, not a permanent role assignment.

---

## Error Handling

**When to fail**:
- Predecessor information is insufficient, making reasonable execution impossible
- The task is found to be infeasible after attempting it
- External dependencies are unavailable

**When to derive new nodes**:
- Subtasks not accounted for in the original plan are discovered during execution
- Prerequisite work needs to be done first
- The main work is complete but follow-up verification/cleanup is needed

**Do not**:
- Write "I will keep trying" in failure_reason -- the node will not be re-executed
- Cram multiple unrelated pieces of work into a single node

---

## Scenario Guides

Tasks are typically combinations of multiple types. Read the corresponding guides based on the types involved in your current task:

- **Software Development Management** -> `{{ shared_dir }}/guide-software-development.md`
  Read when the task involves writing, modifying, or testing code

- **Information Gathering and Research** -> `{{ shared_dir }}/guide-research.md`
  Read when the task involves investigation, competitive analysis, or technology selection

- **Decision-Making and A/B Testing** -> `{{ shared_dir }}/guide-decision-ab-testing.md`
  Read when the task involves comparing approaches, validating technology choices, or evaluating outcomes

- **Git Parallel Development Management** -> `{{ shared_dir }}/guide-git-management.md`
  Read when the task involves parallel feature development or code merging
