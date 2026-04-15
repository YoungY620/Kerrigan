# Agent Benchmark Research for DAG-Based Prompt Optimization

Research conducted: 2026-04-15

## Goal

Find the simplest possible agent benchmarks with absolutely reliable data correctness that can serve as optimization datasets for multi-agent DAG systems. The core insight: if benchmarks can **test** agent DAG outputs, their pass/fail signals can **train** (optimize) the system prompts of each DAG node.

## Research Methodology

- Kerrigan DAG task graph with 4-phase structure
- Phase 1: 4 independent parallel research nodes (simplicity, reliability, prompt-opt frameworks, DAG mapping)
- Phase 2: Cross-validation merging all Phase 1 findings
- Phase 3: Hands-on experiments on Linux server (EvalPlus, GSM8K, DSPy integration)
- Phase 4: This report

## Key Findings

### 1. Recommended Benchmarks (Ranked)

| Priority | Benchmark | Setup Time | Problems | Eval Speed | Reliability | Why |
|----------|-----------|-----------|----------|------------|-------------|-----|
| 1 | **EvalPlus (HumanEval+/MBPP+)** | <5 min | 164+378 | 0.13s/sample | B (test-suite verified) | Best simplicity+reliability. 80x more tests than original. Deterministic pass/fail. |
| 2 | **GSM-Symbolic** (Apple) | <5 min | 5,000 | <1s/sample | A- (contamination-free) | Same format as GSM8K but with canary string. Drop-in replacement. Verified on remote server. |
| 3 | **MATH** | <5 min | 5,000 | <1s/sample | B (exact symbolic) | Competition math, expert-verified answers. Wider difficulty range than GSM8K. |
| 4 | **GSM-Plus** | <5 min | 10,552 | <1s/sample | B+ (perturbation-based) | Largest math dataset, perturbation types for robustness testing. |
| 5 | **BFCL/Gorilla** | <10 min | ~2,000 | <1s/sample | B (AST-based) | Tool-calling benchmark, directly relevant to agent systems. |
| 6 | **lm-eval harness** | <15 min | Many | Varies | Varies | Meta-tool bundling GSM8K+MMLU+ARC+MATH+HumanEval. Use `--limit N` for fast iteration. |

### 2. Benchmarks to Avoid

| Benchmark | Why |
|-----------|-----|
| MMLU | 6.5% label errors, Virology subset 57% error rate. Optimization floor too high. |
| HellaSwag | 36% validation error rate. Not measuring what it claims. |
| GSM8K (for absolute scores) | 13% accuracy inflation from training data contamination. |
| GAIA/AgentBench/MARBLE | Complex setup (Docker, multiple services), slow eval (minutes/sample). Not suitable for rapid iteration. |

### 3. Optimization Framework: DSPy

**DSPy is the clear choice** for multi-agent DAG prompt optimization.

- 33.7K GitHub stars, actively maintained (v3.1.3, Apr 2026)
- Native module composition maps directly to DAG nodes
- MIPROv2 optimizer: optimizes all node prompts jointly with ~75 LLM calls
- GEPA optimizer: better for multi-module pipelines
- End-to-end metric (benchmark pass/fail) drives per-module optimization
- No per-node ground truth needed -- only final metric required

Alternatives:
- **TextGrad**: gradient-style per-node feedback, but vanishes at depth
- **SAMMO** (Microsoft): symbolic DAG mutations, good for structural optimization
- OPRO/APE/EvoPrompt: single-prompt only, require custom outer loop

### 4. DAG Responsibility Chain → Prompt Optimization

The mapping from Kerrigan DAG to DSPy optimization:

```
Kerrigan DAG Node  →  DSPy Module (dspy.Predict with Signature)
DAG edge (A→B)     →  Module output feeds next module's input
Benchmark pass/fail →  End-to-end optimization metric
Credit assignment   →  Ablation: fix all nodes except one, measure impact
```

Three optimization strategies (use in order):

1. **End-to-end** (start here): Run full DAG, score final output, DSPy MIPROv2 optimizes all prompts jointly
2. **Intermediate checkpoints**: Add per-node validation (e.g., planner output must be valid JSON task list)
3. **Ablation diagnostics**: When a node is suspected bottleneck, replace its output with gold data and re-run

### 5. Experimental Verification

Conducted on remote Linux server (61GB RAM, 8 cores, Python 3.12):

| Test | Result |
|------|--------|
| EvalPlus install | `pip install evalplus` -- 1 command, <30s |
| HumanEval+ load | 164 problems, 0.13s load time |
| Full evaluation (164 canonical solutions) | 21.5s total, 0.13s/sample, 99.4% pass@1 |
| Per-sample determinism | Correct code → PASS, wrong code → FAIL, 100% repeatable |
| GSM8K load (via hf-mirror) | 1,319 test problems |
| DSPy MathSolverDAG | 3-module pipeline (Planner→Executor→Reviewer) successfully defined |
| DSPy metric | `gsm8k_exact_match` verified: numeric comparison, fully deterministic |
| DSPy optimization config | MIPROv2 and GEPA ready to run with API key |

## Proposed Pipeline

```
┌──────────────────────────────────────────────────────┐
│              Benchmark-Driven DAG Optimization         │
│                                                        │
│  1. Define DAG as DSPy modules                        │
│     (each Kerrigan node = dspy.Predict with Signature)│
│                                                        │
│  2. Load benchmark (EvalPlus / GSM8K / MATH)          │
│     Convert to dspy.Example trainset + devset          │
│                                                        │
│  3. Define metric function                             │
│     Code: test-suite pass/fail                         │
│     Math: exact numeric match                          │
│                                                        │
│  4. Run DSPy MIPROv2 optimizer                        │
│     Optimizes ALL node prompts jointly                 │
│     Uses ~75 LLM calls for 'medium' setting            │
│                                                        │
│  5. Evaluate on held-out devset                        │
│     Report per-node and end-to-end metrics             │
│                                                        │
│  6. (Optional) Ablation-based blame attribution        │
│     Identify bottleneck nodes for targeted re-opt      │
│                                                        │
│  7. Export optimized prompts back to Kerrigan DAG      │
│     Update node system prompts with DSPy results       │
└──────────────────────────────────────────────────────┘
```

## Next Steps

1. **Provide LLM API key** to remote server to run actual DSPy optimization
2. **Build Kerrigan↔DSPy bridge**: auto-convert Kerrigan DAG structure to DSPy module graph
3. **Run optimization loop**: EvalPlus as first target, measure before/after accuracy
4. **Add intermediate checkpoints**: validate each node's output format before passing downstream
5. **Scale to more benchmarks**: MATH for reasoning, BFCL for tool-use agents

## Files in This Directory

- `README.md` -- This report
- `benchmark-comparison.md` -- Detailed benchmark comparison table
- `prompt-optimization-frameworks.md` -- Framework survey with integration details
- `dag-credit-assignment.md` -- DAG responsibility chain analysis
- `experiment-log.md` -- Remote server experiment results
- `references.md` -- All cited papers, repos, and resources
