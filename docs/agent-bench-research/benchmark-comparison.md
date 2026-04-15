# Benchmark Comparison for Agent DAG Prompt Optimization

## Setup Simplicity Ranking

| # | Benchmark | Install | Dataset Size | Eval Time/Sample | Deterministic? | Format |
|---|-----------|---------|-------------|-------------------|----------------|--------|
| 1 | GSM8K | `pip install datasets` + HF load | 1,319 test | <1s (string match) | Yes | question → numeric answer |
| 2 | HumanEval | `pip install evalplus` | 164 | 0.13s (unit tests) | Yes | function sig+docstring → code → pass/fail |
| 3 | EvalPlus (HumanEval+) | `pip install evalplus` | 164 (80x more tests) | 0.13s | Yes | Same as HumanEval, 80x more test cases |
| 4 | MBPP+ | `pip install evalplus` | 378 | ~0.1s | Yes | task description → function → pass/fail |
| 5 | MMLU | `pip install lm-eval` | 15,908 (57 subjects) | <1s (MC logprob) | Yes* | question + 4 choices → letter (A/B/C/D) |
| 6 | MATH | `pip install datasets` + HF load | 5,000 test | <1s (symbolic match) | Yes | competition problem → LaTeX answer |
| 7 | ARC | `pip install lm-eval` | 2,590 (Challenge) | <1s (MC) | Yes | science question + 4 choices → letter |
| 8 | BFCL | `pip install bfcl-eval` | ~2,000 | ~1s (AST comparison) | Yes | user query + schemas → function call JSON |
| 9 | GAIA | `pip install inspect-evals` + Docker | 450 | Minutes | Yes (exact match) | multi-step question → single answer |
| 10 | LLM-Coordination | conda + pip + game envs | 198 + 4 game envs | Minutes | Partial | multi-agent game → coordination score |
| 11 | AgentBench | conda + Docker + config | 8 environments | Minutes | Partial | interactive env → multi-turn actions |
| 12 | MARBLE | conda + Poetry + API keys | Multiple scenarios | Minutes | No (stochastic) | multi-agent scenario → collaboration metrics |

*MMLU: evaluation is deterministic but 6.5% label errors make it unsuitable as optimization target.

## Data Correctness / Reliability Ranking

| Benchmark | Ground Truth Type | Contamination Risk | Label Quality | Eval Determinism | Overall |
|-----------|-------------------|-------------------|---------------|-------------------|---------|
| ARC-AGI-2 | Exact grid match | Low (adversarial, private) | Very high (97-98% human) | Fully deterministic | **A** |
| GAIA | Exact factual match | Low (requires tool use) | High (unambiguous) | Deterministic | **A-** |
| SWE-bench Verified | Test-suite execution | Medium (repo leakage) | High (human-validated) | Deterministic* | **B** |
| HumanEval+ (EvalPlus) | Test-suite (80x tests) | High (saturated) | High (EvalPlus fixed 11%) | Fully deterministic | **B** |
| MBPP+ (EvalPlus) | Test-suite (35x tests) | High | Moderate-High | Fully deterministic | **B** |
| MATH | Exact symbolic match | Medium | High (expert-verified) | Deterministic | **B** |
| BFCL | AST comparison | Low-Medium | High (formal spec) | Deterministic | **B** |
| ARC-Challenge | MC label | Medium-High | Moderate | Deterministic | **C** |
| GSM8K | Exact numeric match | **Very High** (13% inflation) | High (math verifiable) | Deterministic | **C** |
| MMLU | MC label | High | **Poor** (6.5% errors) | Deterministic but ceiling ~93.5% | **D** |
| HellaSwag | MC label | High | **Very Poor** (36% errors) | Broken (68% unchanged w/o prompt) | **D** |

*SWE-bench Verified: ~7.8% insufficient test coverage (UTBoost finding)

## Combined Score: Simplicity × Reliability

Best candidates for rapid prompt optimization iteration:

| Benchmark | Simplicity | Reliability | Combined Fit | Notes |
|-----------|-----------|-------------|-------------|-------|
| **EvalPlus (HumanEval+/MBPP+)** | Tier 1 | B | **Best** | Sweet spot. 542 problems, deterministic, fast. |
| **MATH** | Tier 2 | B | **Good** | 5K problems, exact answers, no label noise. |
| **BFCL** | Tier 1 | B | **Good** | Directly relevant to tool-calling agents. |
| **GSM-Symbolic** | Tier 1 | A- | **Excellent** | 5K problems, same format as GSM8K, contamination-free (canary). |
| **GSM-Plus** | Tier 1 | B+ | **Good** | 10.5K problems with perturbation types, also contamination-resistant. |
| GSM8K | Tier 1 | C | **Acceptable** | Easiest but contaminated. Use for relative testing only. |
| GAIA | Tier 3 | A- | **Deferred** | Best reliability but too complex for rapid iteration. |
| ARC-AGI-2 | Tier 3 | A | **Deferred** | Best data quality but specialized (abstract reasoning). |

## Benchmark Contamination Issues

Key findings from literature (2024-2026):

- **GSM8K**: GSM1k study (NeurIPS 2024) confirmed 13% accuracy inflation from memorization. Models score systematically lower on GSM1k (new problems) than GSM8K. Use GSM-Symbolic or GSM1k for uncontaminated evaluation.
- **MMLU**: 6.5% overall error rate across all questions. Virology subset has 57% error rate. MMLU-Pro (NeurIPS 2024) and MMLU-CF are cleaner alternatives.
- **HellaSwag**: 36% of validation set contains errors. 21%+ have multiple plausible answers. 40% ungrammatical prompts. 68% of model predictions unchanged without the prompt. Disqualified.
- **HumanEval**: Original has known test case bugs (EvalPlus fixed 11%). Use EvalPlus versions only.
- **SWE-bench**: Repository-level leakage possible (issues/patches may appear in training data). SWE-bench Verified (500 human-validated) mitigates but doesn't eliminate.

Sources:
- [GSM8K Careful Examination (NeurIPS 2024)](https://arxiv.org/pdf/2405.00332)
- [EvalPlus (NeurIPS 2023)](https://arxiv.org/pdf/2305.01210)
- [MMLU-CF Contamination-Free (2024)](https://arxiv.org/html/2412.15194v1)
- [HellaSwag or HellaBad? (Surge AI)](https://surgehq.ai/blog/hellaswag-or-hellabad-36-of-this-popular-llm-benchmark-contains-errors)
- [Awesome Data Contamination](https://github.com/lyy1994/awesome-data-contamination)
- [Survey on Data Contamination (2025)](https://arxiv.org/html/2502.14425v2)
