# Decision-Making and AB Testing Guide

When facing multiple options, decisions should not be made through discussion — they should be validated through **experiments**.

Applicable scenarios: Technology selection, architecture decisions, algorithm/strategy selection, configuration tuning.

Product comparisons and quantitative experiments conducted during research must also follow the experimental design requirements in this guide. For the research process, see `{{ shared_dir }}/guide-research.md`.

---

## Decision Process

```text
[Define the decision problem] → [Define candidate solutions] → [Design experiment]
  → [Independently implement Group A] (parallel)
  → [Independently implement Group B] (parallel)
  → [Collect data]
  → [Independent Analysis 1] (parallel)
  → [Independent Analysis 2] (parallel)
  → [Cross-review: Compare conclusions from the two independent analyses]
  → [Produce decision report]
```

Key point: **Both implementation and analysis must have multiple independent rounds of cross-validation.** The same agent must not both conduct the experiment and analyze the results.

---

## Experiment Design Requirements

- **Clear hypothesis**: In a falsifiable form ("Solution A's P99 latency is lower than Solution B's", not "Solution A is better")
- **Controlled variables**: All conditions except the tested variable must be identical
- **Sample size**: Randomized tests require sufficient repetitions to achieve statistical significance
- **Evaluation metrics**:
  - Primary metrics: Directly measure the decision objective
  - Secondary metrics: Worth monitoring but not used as decision criteria
  - Guardrail metrics: Baselines that must not deteriorate

---

## AB Testing Report Template

```text
## Decision Problem
[One-sentence description]

## Candidate Solutions
- Solution A: [Description]
- Solution B: [Description]

## Hypothesis
[Falsifiable hypothesis statement]

## Experiment Design
- Environment: [Hardware/Software/Dataset]
- Controlled variables: [List]
- Variable: [Tested difference]
- Number of repetitions: [N]

## Raw Data
[Table or data attachment]

## Statistical Analysis
- Primary metric comparison: [A vs B, with confidence intervals]
- Secondary metric comparison:
- Guardrail metric check:

## Cross-review Results
- Independent Analysis 1 conclusion: [...]
- Independent Analysis 2 conclusion: [...]
- Consistency: [Consistent / Discrepancy and reasons]

## Conclusion
[Data-driven decision recommendation with explicit confidence level]

## Limitations
[Uncovered scenarios, factors that may affect conclusions]
```

---

## Unqualified Decisions

The following situations should be sent back for redo:
- No experimental data; decision based solely on "it should theoretically be better"
- Inconsistent environments between A/B groups
- Only looked at primary metrics, ignoring deterioration in guardrail metrics
- Only one analysis with no cross-review
- Conclusions use vague wording ("possibly", "should") instead of data-supported judgments
