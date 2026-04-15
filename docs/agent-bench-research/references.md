# References

## Benchmarks

### Code Generation
- [HumanEval](https://github.com/openai/human-eval) -- 164 Python function completion problems (OpenAI, 2021)
- [MBPP](https://huggingface.co/datasets/google-research-datasets/mbpp) -- 974 Python programming problems (Google, 2021)
- [EvalPlus](https://github.com/evalplus/evalplus) -- HumanEval+/MBPP+ with 80x/35x more tests ([NeurIPS 2023](https://arxiv.org/pdf/2305.01210))
- [SWE-bench](https://www.swebench.com/) -- 2,294 real GitHub issues ([ICLR 2024](https://github.com/SWE-bench/SWE-bench))
- [SWE-bench Verified](https://openai.com/index/introducing-swe-bench-verified/) -- 500 human-validated subset
- [SWE-bench Pro](https://labs.scale.com/leaderboard/swe_bench_pro_public) -- 1,865 tasks across 123 languages ([arXiv 2509.16941](https://arxiv.org/abs/2509.16941))

### Math / Reasoning
- [GSM8K](https://github.com/openai/grade-school-math) -- 8.5K grade school math problems (OpenAI)
- [MATH](https://github.com/hendrycks/math) -- 12.5K competition mathematics problems (Hendrycks et al.)
- [ARC-AGI-2](https://arcprize.org/) -- Abstract reasoning with exact grid match

### Tool Use
- [BFCL/Gorilla](https://gorilla.cs.berkeley.edu/leaderboard.html) -- Berkeley Function Calling Leaderboard ([ICML 2025](https://gorilla.cs.berkeley.edu/))

### Agent / Multi-Agent
- [GAIA](https://huggingface.co/spaces/gaia-benchmark/leaderboard) -- 466 real-world questions with unambiguous answers ([arXiv 2311.12983](https://arxiv.org/abs/2311.12983))
- [AgentBench](https://github.com/THUDM/AgentBench) -- 8 environments ([ICLR 2024](https://arxiv.org/abs/2308.03688))
- [MARBLE](https://github.com/MultiagentBench/MARBLE) -- Multi-agent collaboration + competition ([ACL 2025](https://arxiv.org/abs/2504.01559))
- [REALM-Bench](https://github.com/genglongling/REALM-Bench) -- Dynamic planning/scheduling for multi-agent systems

### Evaluation Frameworks
- [lm-evaluation-harness](https://github.com/EleutherAI/lm-evaluation-harness) -- Unified benchmark runner (EleutherAI)
- [MASEval](https://github.com/parameterlab/MASEval) -- Framework-agnostic multi-agent evaluation (2026)
- [MAESTRO](https://github.com/sands-lab/maestro) -- Multi-agent system configuration and profiling (2026)

## Prompt Optimization Frameworks

- [DSPy](https://github.com/stanfordnlp/dspy) -- Modular LLM pipeline optimization (Stanford, 33.7K stars)
  - [MIPROv2 documentation](https://dspy.ai/learn/optimization/optimizers/)
  - [GEPA optimizer](https://github.com/stanfordnlp/dspy) (Generalized Few-Shot Prompt Optimization)
- [TextGrad](https://github.com/zou-group/textgrad) -- Text gradient descent for LLM optimization (Nature, 3.5K stars)
- [SAMMO](https://github.com/microsoft/sammo) -- Symbolic prompt DAG optimization (Microsoft, 751 stars)
- [OPRO](https://arxiv.org/abs/2309.03409) -- Optimization by PROmpting (Google DeepMind)
- [EvoPrompt](https://arxiv.org/abs/2309.08532) -- Evolutionary prompt optimization
- [PromptBreeder](https://arxiv.org/abs/2309.16797) -- Self-referential self-improvement (DeepMind)
- [APE](https://arxiv.org/abs/2211.01910) -- Automatic Prompt Engineer

## Credit Assignment / Multi-Agent Optimization

- [HiveMind](https://arxiv.org/pdf/2512.06432) -- Contribution-guided prompt optimization via Shapley values (2024)
- [Explainable Credit Assignment in MARL](https://arxiv.org/abs/2502.16863) -- LLM-guided Shapley (AAMAS 2025)
- [LLM-Guided Credit Assignment in MARL](https://arxiv.org/abs/2502.03723) -- LLM credit assignment (2025)
- [Turn-Level Credit Assignment](https://arxiv.org/pdf/2505.11821) -- 100K+ token multi-turn trajectories (2025)
- [MASS: Multi-Agent System Design](https://arxiv.org/abs/2502.02533) -- Joint prompt + topology optimization (2025)
- [Self-Optimizing Multi-Agent Systems](https://arxiv.org/abs/2604.02988) -- Self-improvement loops (2025)
- [Trace is the Next AutoDiff](https://proceedings.neurips.cc/paper_files/paper/2024/file/83ba7056bce2c3c3c27e17397cf3e1f0-Paper-Conference.pdf) -- Theoretical foundation (NeurIPS 2024)
- [Tests as Prompt](https://arxiv.org/abs/2505.09027) -- TDD as prompt specification (2025)

## Benchmark Contamination / Reliability

- [GSM8K Careful Examination](https://arxiv.org/pdf/2405.00332) -- 13% inflation from memorization (NeurIPS 2024)
- [MMLU-CF Contamination-Free](https://arxiv.org/html/2412.15194v1) -- Clean MMLU alternative (2024)
- [MMLU-Pro](https://github.com/TIGER-AI-Lab/MMLU-Pro) -- Improved MMLU (NeurIPS 2024)
- [HellaSwag or HellaBad?](https://surgehq.ai/blog/hellaswag-or-hellabad-36-of-this-popular-llm-benchmark-contains-errors) -- 36% error rate (Surge AI)
- [HellaSwag Validity Study](https://arxiv.org/html/2504.07825v1) -- Formal analysis (2025)
- [Awesome Data Contamination](https://github.com/lyy1994/awesome-data-contamination) -- Curated paper list
- [Survey on Data Contamination for LLMs](https://arxiv.org/html/2502.14425v2) -- Comprehensive survey (2025)
- [What's in a Benchmark? SWE-Bench](https://arxiv.org/pdf/2602.04449) -- SWE-bench quality analysis (2026)

## Multi-Agent Code Generation Systems

- [MapCoder](https://github.com/Md-Ashraful-Pramanik/MapCoder) -- 4-agent code generation (ACL 2024, 93.9% HumanEval)
- [AgentCoder](https://github.com/huangd1999/AgentCoder) -- 3-agent with independent test designer (96.3% HumanEval)
- [MetaGPT](https://github.com/FoundationAgents/MetaGPT) -- SOP-based multi-role (ICLR 2024 Oral)
- [ChatDev](https://github.com/OpenBMB/ChatDev) -- Role-based software company (ACL 2024)

## Scaling / Negative Results

- [Google Scaling Study (2025)](https://arxiv.org/abs/2502.18440) -- Multi-agent scaling laws
- [Single-Agent vs Multi-Agent (2026)](https://arxiv.org/abs/2604.02460) -- Equal token budget comparison
- [MAST Failure Taxonomy (2025)](https://arxiv.org/abs/2503.09890) -- 1,600+ annotated failure traces
- [MAFBench (2026)](https://arxiv.org/abs/2602.03128) -- Framework architecture impact on performance

## Survey Papers

- [Evaluation and Benchmarking of LLM Agents (SIGKDD 2025)](https://arxiv.org/abs/2507.21504)
- [Survey on Evaluation of LLM-based Agents (2025)](https://arxiv.org/abs/2503.16416)
- [LLM-based Multi-Agents Survey (IJCAI 2024)](https://github.com/taichengguo/LLM_MultiAgents_Survey_Papers)
- [Multi-Agent Collaboration Mechanisms Survey (2025)](https://arxiv.org/abs/2501.06322)
