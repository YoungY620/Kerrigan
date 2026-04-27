# Kerrigan Guide

You are executing a task using the Kerrigan DAG framework. This document contains universal principles and conventions applicable to all tasks. Please read it before starting your work.

Depending on the type of your current task, you should also read the corresponding scenario guide (see the "Scenario Guides" section at the end).

---

## Core Principle: Your result_summary IS the communication

There is no `sendMessage`, no message queue, no way to talk to another agent directly. The **only** signal downstream agents receive from you is the `result_summary` you write when you complete your node (or the `failure_reason` if you fail).

This has three consequences you must internalize:

1. **`result_summary` must expose everything downstream might need to know — especially risks, caveats, failures, and unresolved problems.** Write it for a reader who knows nothing about what you did; not for yourself, not for the orchestrator. Anything you omit — a skipped check, a shaky assumption, a partial result, a known gap — silently becomes a false assumption that downstream agents act on. When in doubt, surface it.
2. **If you need something from another agent, you do NOT message them — you `add` a new node with the right dependency.** Any coordination need can be expressed structurally.
3. **DAG edges = information requirements, not organizational hierarchy.** Declare a predecessor only when you genuinely need its `result_summary` to proceed. Over-declaring blocks parallelism; under-declaring causes execution on missing context.

Everything else in this guide — decomposition, cross-validation, transparent disclosure, error handling — is a consequence of this principle.

---

## Execution Workflow

There are two distinct roles. Know which one you are in.

### Orchestrator workflow (building and dispatching the DAG)

1. **Create the task**: `kerrigan task create --summary "..." --workdir .`
2. **Build the DAG**: Add nodes with `dag add --contract ...`. Add 1–2 layers at a time; don't over-plan.
3. **Find ready nodes**: `kerrigan dag ready --task <task-id>`
4. **Dispatch each ready node to an agent worker**: `kerrigan exec --task <task-id> --node <node-id> --adapter claude`
   - `exec` sets status → running and launches an agent subprocess
   - The agent worker reads its own prompt, does the work, and calls `dag update` itself
   - You do **not** call `dag update` for nodes you exec
5. **Monitor progress**: `kerrigan watch <task-id>` or poll `dag ready`
6. **Once a node succeeds**, its successors automatically become ready — loop back to step 3
7. **Add follow-up nodes** as needed after seeing results: `dag add --depends-on <completed-node>`

### Worker workflow (inside an exec session)

You are launched by `exec` with a prompt describing your node. Follow this:

1. **Read node details**: `kerrigan dag get --task <task-id> --node <node-id>`
2. **Read predecessor inputs**: they are already pre-filtered to only the fields you declared in your `contract.predecessor` schema — no need to strip noise yourself. Parse the JSON fields you need.
3. **Check sufficiency**:
   - Sufficient → proceed
   - Insufficient → fail immediately with a clear `failure_reason`; do not guess
4. **Execute the task**
5. **Discover follow-up work?** Add new nodes: `dag add --depends-on <your-node-id> --contract ...`
6. **Write back your result**: `dag update` with `status: "succeeded"` and valid JSON `result_summary`, or `status: "failed"` with `failure_reason`

You do **not** call `exec` from inside a worker — you are already inside one.

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

### result_summary — Must Be Valid JSON (since v1.1.0)

`result_summary` **must be a JSON object** that satisfies the node's declared `output.schema`. Plain text is rejected.

Wrap your content in a JSON object. The required fields are whatever you declared in `output.schema`. A typical schema and matching result:

```json
// output.schema declared at dag add:
{
  "type": "object",
  "required": ["summary", "key_outputs", "limitations"],
  "properties": {
    "summary":     {"type": "string"},
    "key_outputs": {"type": "array"},
    "limitations": {"type": "string"}
  }
}

// Matching result_summary value (this is what you pass as a JSON string):
{
  "summary": "Found 3 REST endpoints requiring auth migration",
  "key_outputs": ["/api/login", "/api/logout", "/api/refresh"],
  "limitations": "Did not check GraphQL endpoints under src/graphql/"
}
```

Pass it as an escaped JSON string in the `--input` argument:

```bash
kerrigan dag update --task T --input '[{
  "id": "<node-id>",
  "status": "succeeded",
  "metadata": {
    "result_summary": "{\"summary\":\"Found 3 endpoints\",\"key_outputs\":[\"/api/login\"],\"limitations\":\"none\"}"
  }
}]'
```

### Recommended fields (adapt to your schema)

| Field | Purpose |
|-------|---------|
| `summary` | One-sentence conclusion |
| `key_outputs` | Files changed / findings / artifacts — verifiable |
| `recommendations` | What downstream nodes need to know |
| `limitations` | Honest disclosure: uncovered scenarios, workarounds, skipped tests, TODOs |

**Must disclose everything in `limitations`**: anything omitted becomes a false assumption downstream agents act on. Concealment is more harmful than the defect itself.

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

**When `dag update --status succeeded` is blocked by schema validation**:

Two scenarios — pick the right one:

- **Option A — execution was wrong**: Fix your output to match the declared `output.schema`, then retry `dag update`.
- **Option B — the contract was wrong at creation time** (your work was actually correct, but the schema you declared was wrong): Use `dag override` to bypass validation and record the actual result:
  ```bash
  kerrigan dag override --task <task-id> --node <node-id> --result-summary '{"summary":"actual result"}'
  ```
  This marks the node succeeded and lets downstream nodes proceed normally.

**Do not**:
- Write "I will keep trying" in failure_reason — the node will not be re-executed
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
