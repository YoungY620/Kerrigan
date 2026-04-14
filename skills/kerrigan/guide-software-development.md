# Software Development Management Guide

## Overall Approach: Small-Step Iteration

### Mapping Features to Branches

Each external feature request corresponds to a single feature branch in git. If the feature is too large, break it down into subtasks, each with its own sub-branch off the feature branch. This is **recursive** — if a sub-branch is still too large, continue splitting into sub-sub-branches.

**Splitting criteria** (auxiliary guidelines — split if any one is met):
- The description exceeds 500 words and still cannot clearly specify what needs to be done
- The estimated changes span more than 10 files or 200 lines of code

**Completion is a backtracking process**: the lowest-level sub-branch is completed → merged into the parent feature branch → the parent performs integration validation → backtracking continues → the top-level feature branch is delivered as a whole → then a decision is made on how to merge into the main branch.

```text
main
 └── feature/add-auth                          ← external requirement
      ├── feature/add-auth/middleware           ← subtask
      │    ├── feature/add-auth/middleware/jwt     ← further split
      │    └── feature/add-auth/middleware/session
      └── feature/add-auth/tests

Backtrack merge: jwt → middleware → add-auth → (validate) → main
```

### Relationship Between Parallel Tracks and the Branch Tree

Parallel tracks are **execution resources** and are not restricted to a specific task level. At any given moment, each track executes either a **leaf node** in the branch tree (the lowest-level incomplete sub-branch) or a **feature node whose child nodes have all been merged and is ready for integration work at its own level**. A track never holds a "large feature that hasn't been fully broken down yet."

For detailed operations on branch management, parallel tracks, and merge workflows, refer to `{{ shared_dir }}/guide-git-management.md`.

### Size Control for Each Feature

Each feature that is ultimately implemented (a leaf node of the branch tree) should be "the smallest meaningful change that can be fully delivered and tested in one pass."

**Iteration loop**: analyze requirements → implement → validate from multiple angles → deliver. Skipping validation and delivering directly is not allowed.

Any work involving technology selection or solution design must follow the decision-making and A/B testing workflow guide (`{{ shared_dir }}/guide-decision-ab-testing.md`).

---

## Required Subtask Types

Validation tasks require **multiple independent nodes executing separately from different angles** to achieve cross-validation.

### Requirements Analysis (Independent, Multiple Times)

Verify the reasonableness of the requirements themselves:
- Whether the requirements contain internal contradictions
- Whether they conflict with the existing system
- Whether they comply with the small-step iteration principle (changes spanning 10+ files should be considered for splitting)
- **When irreconcilable contradictions are found, halt execution** and report via failure_reason

### Development Implementation

- Follow the project's existing code style and architectural patterns
- Keep each change as atomic as possible — one commit does one thing

### Static Code Review (Independent, Multiple Times)

Independent from the developer, each review focuses on a different dimension:
- **Structural review**: excessive nesting, unreasonable abstractions (only propose refactoring when it genuinely affects maintainability — no preventive refactoring)
- **Consistency review**: logic inconsistent with comments, implementation inconsistent with requirements, behavior inconsistent with existing functionality
- **Standards review**: naming conventions, error handling patterns, logging standards
- **Risk review**: security vulnerabilities (injection, privilege escalation, information leakage), crash paths (unhandled exceptions, resource leaks, deadlocks), performance issues (N+1 queries, unnecessary full scans, memory blowups)

Key point: reviewers should not know where the developer thinks problems might exist — reviews must be independent and unbiased.

### Test Development

- Cover both normal and exception paths
- Include boundary condition tests
- Tests should be able to run independently

### Test Review (Independent, Multiple Times)

Independent from the test developer, check for:
- **Fake tests**: tests that always pass but don't actually verify anything
- **Invalid assertions**: assertion conditions that are too broad
- **Missing scenarios**: whether critical paths, exception paths, and concurrency scenarios are covered
- **Maintainability**: whether tests are overly coupled to implementation details

### Security Analysis (Independent, Multiple Times, Two Phases)

**Phase 1: Independent Analysis (Parallel, No Code Modifications)**
- White-box analysis: identify injection, privilege escalation, and information leakage risks
- Adversarial analysis: determine which inputs could trigger crashes, data corruption, or security vulnerabilities
- Assertion placement proposals: suggest where defensive assertions should be added (to prevent silent failures) — **propose only, do not modify code**
- Multiple independent analysis nodes each produce their own proposals; may reference OWASP Top 10, CWE

**Phase 2: Proposal Consolidation and Implementation (Sequential, After Analysis Is Complete)**
- Aggregate all proposals and merge into a set of uncontested changes
- Discuss and reach consensus on disputed items before execution
- A single node executes the code modifications to avoid parallel conflicts

**General rule**: any validation method that requires modifying source code (assertions, logging probes, fuzzing harnesses, stress test configurations, etc.) follows the "parallel proposals → sequential implementation" two-phase model.

---

## DAG Topology Reference

```mermaid
graph TD
    RA[Requirements Analysis A] --> RC[Requirements Consensus]
    RB[Requirements Analysis B] --> RC
    RC -->|Pass| DEV[Development Implementation]
    DEV --> CR1[Review - Structure]
    DEV --> CR2[Review - Consistency]
    DEV --> CR3[Review - Standards]
    DEV --> CR4[Review - Risk]
    CR1 --> CRC[Review Consensus]
    CR2 --> CRC
    CR3 --> CRC
    CR4 --> CRC
    CRC -->|Pass| TEST[Test Development]
    CRC -->|Pass| SA1[Security Analysis A]
    CRC -->|Pass| SA2[Security Analysis B]
    TEST --> TR1[Test Review A]
    TEST --> TR2[Test Review B]
    SA1 --> SAC[Security Proposal Consolidation & Implementation]
    SA2 --> SAC
    TR1 --> VC[Validation Consensus]
    TR2 --> VC
    SAC --> VC
    VC -->|Pass| SHIP[Delivery]

    %% Requirements consensus finds contradictions → fail and escalate or spawn requirements clarification
    RC -.->|Contradiction found| RCFIX[Requirements Clarification/Split]
    RCFIX -.-> RA2[Re-run Requirements Analysis A]
    RCFIX -.-> RB2[Re-run Requirements Analysis B]
    RA2 -.-> RC2[Re-run Requirements Consensus]
    RB2 -.-> RC2
    RC2 -.->|Pass| DEV
    RC2 -.->|Irreconcilable| RCFAIL[FAIL: Escalate Requirements Contradiction]

    style RCFIX fill:#f96
    style RA2 fill:#fca
    style RB2 fill:#fca
    style RC2 fill:#fca
    style RCFAIL fill:#f33,color:#fff

    %% Review finds issues → spawn fix → full re-review
    CRC -.->|Issues found| FIX[Fix: Review Issues]
    FIX -.-> CR1R[Re-review - Structure]
    FIX -.-> CR2R[Re-review - Consistency]
    FIX -.-> CR3R[Re-review - Standards]
    FIX -.-> CR4R[Re-review - Risk]
    CR1R -.-> CRCR[Re-review Consensus]
    CR2R -.-> CRCR
    CR3R -.-> CRCR
    CR4R -.-> CRCR
    CRCR -.->|Pass| TEST
    CRCR -.->|Issues found again| FIX2[Fix: Second Round Issues]

    %% Test review finds issues (fake tests/invalid assertions/missing scenarios) → rewrite tests → full re-review
    VC -.->|Test issues| TFIX[Rewrite Tests]
    TFIX -.-> TFIX_TEST[Re-run Test Development]
    TFIX_TEST -.-> TR1R[Re-run Test Review A]
    TFIX_TEST -.-> TR2R[Re-run Test Review B]
    TR1R -.-> VCR[Re-run Validation Consensus]
    TR2R -.-> VCR
    VCR -.->|Pass| SHIP
    VCR -.->|Issues found again| TFIX2[Fix: Second Round Test Issues]

    %% Security analysis finds critical issues → code changes needed → back to development + full re-review
    VC -.->|Security issues requiring code changes| SFIX[Fix: Security Defects]
    SFIX -.-> CR1S[Re-review - Structure]
    SFIX -.-> CR2S[Re-review - Consistency]
    SFIX -.-> CR3S[Re-review - Standards]
    SFIX -.-> CR4S[Re-review - Risk]
    CR1S -.-> CRCS[Re-review Consensus]
    CR2S -.-> CRCS
    CR3S -.-> CRCS
    CR4S -.-> CRCS
    CRCS -.->|Pass| TEST

    style FIX fill:#f96
    style FIX2 fill:#f96
    style TFIX fill:#f96
    style TFIX2 fill:#f96
    style TFIX_TEST fill:#fca
    style SFIX fill:#f96
    style CR1S fill:#fca
    style CR2S fill:#fca
    style CR3S fill:#fca
    style CR4S fill:#fca
    style CRCS fill:#fca
    style CR1R fill:#fca
    style CR2R fill:#fca
    style CR3R fill:#fca
    style CR4R fill:#fca
    style CRCR fill:#fca
    style TR1R fill:#fca
    style TR2R fill:#fca
    style VCR fill:#fca
```

**Legend**: solid lines = normal flow, dashed lines + orange nodes = rework paths spawned when reviews find issues.

**Key rules**:
- When any phase (requirements analysis, code review, test review, security analysis) finds issues, **do not return to the original node** (each node executes only once). Spawn a "fix" node, and after the fix is complete, enter a **full re-validation** (independent, multiple times, at the same rigor as the first pass).
- If re-validation finds more issues, continue spawning fix nodes — recurse until all checks pass.
- When security analysis identifies a need for code changes, the fix must go back to the code review phase and re-run the full workflow (because the code has changed).
- When irreconcilable contradictions are found during requirements analysis, fail immediately and escalate — do not spawn fix nodes.

This is not a fixed template — adjust based on actual circumstances. Fixing a typo does not require a full review and security analysis workflow. **Complexity should be proportional to the risk of the change.**
