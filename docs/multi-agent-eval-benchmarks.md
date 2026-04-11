# 多智能体系统评估 Benchmark 综述

多智能体及 Agent 系统的评估 benchmark 系统性调研，涵盖评估方法论、指标和资源。

相关文档：
- [相关工作：DAG 与无通信协调](survey-related-work.md)
- [任务分解与协调策略研究](survey-task-decomposition.md)

---

## 1. Software Engineering / Coding

### 1.1 SWE-bench Family

| Variant | Year | Dataset | Key Metric | Top Result | URL |
|---------|------|---------|------------|------------|-----|
| **SWE-bench** | 2024 (ICLR) | 2,294 Python issues from 12 repos | Resolve Rate | ~20% | [github](https://github.com/SWE-bench/SWE-bench), [leaderboard](https://www.swebench.com/) |
| **SWE-bench Verified** | 2024 | 500 human-validated problems | Resolve Rate (pass@1) | ~80.9% (Claude Opus 4.5) | [leaderboard](https://epoch.ai/benchmarks/swe-bench-verified/) |
| **SWE-bench Multimodal** | 2025 (ICLR) | 617 JS tasks, 17 libraries | Resolve Rate | 12% (SWE-agent) | [arxiv](https://arxiv.org/abs/2410.03859) |
| **SWE-bench Pro** | 2025 | 1,865 tasks, 41 repos, 123 languages | Resolve Rate | 23.3% (GPT-5) | [arxiv](https://arxiv.org/abs/2509.16941), [leaderboard](https://labs.scale.com/leaderboard/swe_bench_pro_public) |

Evaluation: given a codebase + GitHub issue, generate a patch. Evaluated by running the repo's test suite (fail-to-pass + pass-to-pass). Not inherently multi-agent, but the de facto standard for all coding agent systems.

### 1.2 HumanEval / MBPP

| Benchmark | Year | Dataset | Key Metric | URL |
|-----------|------|---------|------------|-----|
| **HumanEval** | 2021 | 164 Python functions | pass@k | [github](https://github.com/openai/human-eval) |
| **MBPP** | 2021 | 974 Python problems | pass@k | [huggingface](https://huggingface.co/datasets/google-research-datasets/mbpp) |
| **EvalPlus** (HumanEval+/MBPP+) | 2023 (NeurIPS) | 80x/35x expanded tests | pass@k | [github](https://github.com/evalplus/evalplus), [leaderboard](https://evalplus.github.io/leaderboard.html) |

Function-level code generation from docstrings. Not multi-agent specific, but the universal baseline for multi-agent code systems (MapCoder, AgentCoder, MetaGPT).

### 1.3 Multi-Agent Code Generation Systems (with eval)

| System | Year | Agents | Evaluated On | Best Result | URL |
|--------|------|--------|-------------|-------------|-----|
| **MapCoder** | 2024 (ACL) | 4 (recall, plan, code, debug) | HumanEval, MBPP, APPS, CodeContests | 93.9% HumanEval | [github](https://github.com/Md-Ashraful-Pramanik/MapCoder) |
| **AgentCoder** | 2024 | 3 (programmer, test designer, executor) | HumanEval, MBPP | 96.3% HumanEval | [github](https://github.com/huangd1999/AgentCoder) |
| **ChatDev** | 2024 (ACL) | Role-based (CEO, CTO, Programmer, Tester) | Completeness, Executability, Consistency | -- | [github](https://github.com/OpenBMB/ChatDev) |
| **MetaGPT** | 2024 (ICLR Oral) | SOP-based roles | HumanEval, MBPP, SoftwareDev (70 tasks) | 85.9% HumanEval | [github](https://github.com/FoundationAgents/MetaGPT) |
| **SOEN-101/FlowGen** | 2025 (ICSE) | Waterfall/TDD/Scrum roles | HumanEval, MBPP variants | 75.2% HumanEval | [arxiv](https://arxiv.org/abs/2403.15852) |

### 1.4 Repository-Level Benchmarks

| Benchmark | Year | What | Key Metric | URL |
|-----------|------|------|------------|-----|
| **DevBench** | 2024 | Full SDLC: design, setup, implement, test. 22 repos, 4 languages | Phase-level completion | [github](https://github.com/open-compass/DevBench) |
| **RepoBench** | 2024 (ICLR) | Cross-file code completion. 3 sub-tasks (Retrieval, Completion, Pipeline) | EM, Edit Similarity, CodeBLEU | [github](https://github.com/Leolty/repobench) |
| **CrossCodeEval** | 2023 (NeurIPS) | Cross-file completion requiring static analysis context. 10K examples, 4 languages | Completion accuracy | [github](https://github.com/amazon-science/cceval) |
| **CodeAgentBench** | 2024 (ACL) | Repo-level generation. 101 samples, 5 Python projects | Performance improvement | [arxiv](https://arxiv.org/abs/2401.07339) |

---

## 2. General Reasoning / Planning / Collaboration

### 2.1 Explicitly Multi-Agent Benchmarks

| Benchmark | Year | Focus | Agents | Key Metrics | URL |
|-----------|------|-------|--------|-------------|-----|
| **MultiAgentBench (MARBLE)** | 2025 (ACL) | Collaboration + competition: research, Minecraft, coding, Werewolf, bargaining | Multi | Milestone KPIs, Communication Score | [github](https://github.com/MultiagentBench/MARBLE) |
| **AgentsNet** | 2025 | Distributed computing: leader election, consensus, graph coloring. Up to 100 agents | 2-100 | Success rate vs network size | [arxiv](https://arxiv.org/html/2507.08616v1) |
| **REALM-Bench** | 2025 | Dynamic planning/scheduling (Job-Shop). Tests LangGraph, AutoGen, CrewAI, Swarm | Multi | Planning quality, coordination efficiency | [github](https://github.com/genglongling/REALM-Bench) |
| **AgentVerse** | 2024 (ICLR) | Emergent collaborative behaviors: volunteer, conformity, destructive | Multi | Task improvement over single-agent | [arxiv](https://arxiv.org/abs/2308.10848) |
| **BattleAgentBench** | 2024 | Single -> paired -> multi-agent, progressive difficulty | 1-N | Per-stage success rate | [arxiv](https://arxiv.org/abs/2408.15971) |

### 2.2 Agent Benchmarks (single-agent, widely used for agent eval)

| Benchmark | Year | Environments | Key Metrics | URL |
|-----------|------|-------------|-------------|-----|
| **AgentBench** | 2024 (ICLR) | 8 environments: OS, DB, KG, Web, Games | Success rate per environment | [github](https://github.com/THUDM/AgentBench) |
| **AgentBoard** | 2024 (NeurIPS Oral) | 9 tasks, 4 categories, 1013 environments | Progress Rate, Success Rate | [website](https://hkust-nlp.github.io/agentboard/) |
| **GAIA** | 2023 | 466 real-world questions, 3 difficulty levels | Accuracy (human 92%, GPT-4 15%) | [leaderboard](https://huggingface.co/spaces/gaia-benchmark/leaderboard) |
| **Gaia2** | 2026 (ICLR) | 1,120 async smartphone-like tasks | Pass@1 (GPT-5: 42%) | [arxiv](https://arxiv.org/abs/2602.11964) |
| **WebArena** | 2024 | 812 web tasks across simulated sites | Task completion (best: 61.7%) | [website](https://webarena.dev/) |
| **OSWorld** | 2024 (NeurIPS) | 369 cross-app desktop tasks, Ubuntu/Windows/macOS | Execution-based success rate | [website](https://os-world.github.io/) |
| **TravelPlanner** | 2024 (ICML Spotlight) | 1,225 planning intents, ~4M data records | Success rate (GPT-4: 0.6%) | [github](https://github.com/OSU-NLP-Group/TravelPlanner) |

### 2.3 Multi-Agent Evaluation Frameworks

| Framework | Year | What | URL |
|-----------|------|------|-----|
| **MASEval** | 2026 | Framework-agnostic eval infra. Adapters for smolagents, LangGraph, LlamaIndex | [github](https://github.com/parameterlab/MASEval) |
| **MAESTRO** | 2026 | MAS config, execution stability, resource profiles. Tested 12 MAS | [github](https://github.com/sands-lab/maestro) |
| **AutoGenBench** | 2024 | CLI tool for evaluating AutoGen agents on standard benchmarks | [github](https://github.com/microsoft/autogen) |

### 2.4 Multi-Agent Debate / Communication

| Benchmark | Year | Method | Key Metrics | URL |
|-----------|------|--------|-------------|-----|
| **Multi-Agent Debate** | 2024 (ICML) | Multiple LLM instances debate over rounds | Accuracy improvement on GSM8K, MMLU | [github](https://github.com/composable-models/llm_multiagent_debate) |
| **ChatEval** | 2024 (ICLR) | LLM "referees" debate text quality | Correlation with human eval (+6.2%) | [github](https://github.com/thunlp/ChatEval) |

---

## 3. Game / Simulation

### 3.1 Cooperative Games

| Benchmark | Year | Game | Key Metrics | URL |
|-----------|------|------|-------------|-----|
| **Collab-Overcooked** | 2025 (EMNLP) | Kitchen collaboration, 30 tasks, 6 complexity levels | Goal interpretation, active collaboration, adaptation | [github](https://github.com/YusaeMeow/Collab-Overcooked) |
| **LLM-Hanabi** | 2025 | Imperfect-information card game, Theory-of-Mind | Game score (/25), ToM accuracy | [arxiv](https://arxiv.org/abs/2510.04980) |
| **LLM-Coordination** | 2025 (NAACL) | 4 coordination games + 198 QA questions | Coordination success, ToM reasoning | [github](https://github.com/eric-ai-lab/llm_coordination) |
| **CuisineWorld/MindAgent** | 2024 (NAACL) | Virtual kitchen, 33 dishes, time-constrained | Collaboration Score (CoS) | [website](https://mindagent.github.io/) |
| **RoCoBench/RoCo** | 2023 | Multi-robot tabletop manipulation, 6 tasks | Task completion, coordination quality | [website](https://project-roco.github.io/) |

### 3.2 Competitive / Mixed Games

| Benchmark | Year | Game | Key Metrics | URL |
|-----------|------|------|-------------|-----|
| **MAgIC** | 2023 | Chameleon, Undercover, game theory scenarios | Cognition, adaptability, rationality, collaboration scores | [github](https://github.com/cathyxl/MAgIC) |
| **Werewolf Arena** | 2024 | Social deduction with bid-based turn-taking | Win rate, deception detection | [arxiv](https://arxiv.org/abs/2407.13943) |
| **AvalonBench** | 2024 | Resistance: Avalon | Win rate, strategy alignment | [github](https://github.com/jonathanmli/avalon-llm) |
| **CICERO (Diplomacy)** | 2022 (Science) | 7-player Diplomacy, NL negotiation + strategy | Game score (2x avg), top 10% | [github](https://github.com/facebookresearch/diplomacy_cicero) |
| **SMAC/SMACv2** | 2019/2023 | StarCraft II micromanagement, decentralized MARL | Win rate per scenario | [github](https://github.com/oxwhirl/smacv2) |

### 3.3 Simulation / Social

| Benchmark | Year | Setting | Key Metrics | URL |
|-----------|------|---------|-------------|-----|
| **Generative Agents (Smallville)** | 2023 (UIST) | 25 agents in simulated town, 2-day simulation | Believability (crowdworker-judged) | [github](https://github.com/joonspk-research/generative_agents) |
| **SOTOPIA** | 2024 (ICLR) | 90 social scenarios, 40 characters with hidden info | Goal completion, social intelligence dims | [website](https://sotopia.world/) |
| **AgentSims** | 2023 | Customizable sandbox, GUI-based task building | Task-dependent | [github](https://github.com/py499372727/AgentSims) |
| **ChatArena** | 2023 | Multi-agent language games (MDP framework) | Game-specific outcomes | [github](https://github.com/Farama-Foundation/chatarena) |

---

## 4. Long-Horizon / Sustained Tasks

### 4.1 Time Horizon Measurement

| Benchmark | Year | Scope | Key Metric | Top Result | URL |
|-----------|------|-------|------------|------------|-----|
| **METR Time Horizons** | 2025-2026 | 228 tasks, 1min-8h+ human time | 50% time horizon | Claude Opus 4.6: ~14.5h | [metr.org](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/), [epoch tracker](https://epoch.ai/benchmarks/metr-time-horizons/) |
| **HCAST** | 2025 | 189 tasks, 563 human baselines, 1500+ hours | Pass rate at time thresholds | -- | [github](https://github.com/METR/hcast-public) |
| **RE-Bench** | 2024 | 7 open-ended ML research tasks, 8h limit | Score vs human experts | -- | [github](https://github.com/METR/RE-Bench) |

METR measures the human-calibrated task duration at which models reach 50% success. Current doubling time: ~4 months. Models succeed 70-80% on sub-1h tasks but under 20% on 4h+ tasks.

### 4.2 Long-Horizon Software Engineering

| Benchmark | Year | Scale | Key Finding | URL |
|-----------|------|-------|-------------|-----|
| **SWE-EVO** | 2025 | 48 tasks, avg 21 files, 874 tests per instance | GPT-5.4: 25% (vs 72.8% SWE-bench Verified) | [arxiv](https://arxiv.org/abs/2512.18470) |
| **FeatureBench** | 2026 (ICLR) | 200 tasks, 24 repos, multi-commit features | Claude 4.5 Opus: 11.0% (vs 74.4% SWE-bench) | [github](https://github.com/LiberCoders/FeatureBench) |
| **NL2Repo-Bench** | 2025 | 104 tasks, build entire repos from NL spec (~18.8K tokens) | Best: <40% test pass rate | [github](https://github.com/multimodal-art-projection/NL2RepoBench) |
| **ABC-Bench** | 2026 | 224 tasks, 8 languages, full DevOps lifecycle | Best: 63.2% pass@1 | [github](https://github.com/OpenMOSS/ABC-Bench) |
| **ProjDevBench** | 2026 | 20 problems, full project development | 27.38% acceptance | [arxiv](https://arxiv.org/abs/2602.01655) |

### 4.3 Research / Science / ML Pipelines

| Benchmark | Year | Domain | Key Metric | URL |
|-----------|------|--------|------------|-----|
| **MLAgentBench** | 2024 (ICML) | 13 ML tasks (CIFAR-10, BabyLM, etc.) | Success rate (37.5% Claude Opus) | [arxiv](https://arxiv.org/abs/2310.03302) |
| **MLE-bench** | 2024 (OpenAI) | 75 Kaggle competitions | Medal achievement (16.9% bronze) | [github](https://github.com/openai/mle-bench) |
| **ScienceAgentBench** | 2025 (ICLR) | 102 tasks from 44 papers, 4 science domains | Program correctness (32.4%) | [github](https://github.com/OSU-NLP-Group/ScienceAgentBench) |
| **FIRE-Bench** | 2026 | Rediscover ML findings: full research cycle | -- | [arxiv](https://arxiv.org/abs/2602.02905) |
| **Agent Laboratory** | 2025 (EMNLP) | Full research workflow: literature -> experiment -> paper | -- | [github](https://github.com/SamuelSchmidgall/AgentLaboratory) |
| **DataSciBench** | 2025 | End-to-end data science workflows | TFC framework | [github](https://github.com/THUDM/DataSciBench) |
| **KramaBench** | 2025 | 104 tasks, 1700 datasets, data-to-insight pipelines | Pipeline success | [github](https://github.com/mitdbg/KramaBench) |

### 4.4 Agent Memory / Ultra-Long Trajectories

| Benchmark | Year | What | Key Finding | URL |
|-----------|------|------|-------------|-----|
| **UltraHorizon** | 2025 | 200K+ tokens, 400+ tool calls per trajectory | Naive scaling doesn't solve it | [arxiv](https://arxiv.org/abs/2509.21766) |
| **LORE / Agent's Marathon** | 2025 | Controllable horizon length, error accumulation | Performance degrades with length | [openreview](https://openreview.net/forum?id=dAn82lpLx4) |
| **AgentLongBench** | 2026 | 32K-4M token contexts via environment rollouts | Retrieval augmentation unreliable | [arxiv](https://arxiv.org/abs/2601.20730) |
| **MEMTRACK** | 2025 (NeurIPS) | Multi-platform (Slack, Linear, Git) state tracking | GPT-5: 60% correctness | [arxiv](https://arxiv.org/abs/2510.01353) |
| **LongMemEval** | 2024 | 5 memory abilities across long chat histories | 30% accuracy drop over sessions | [project](https://xiaowu0162.github.io/long-mem-eval/) |
| **LoCoMo** | 2024 | 300 turns, 35 sessions, multi-modal | LLMs + RAG struggle | [project](https://snap-research.github.io/locomo/) |

### 4.5 Workplace / Planning

| Benchmark | Year | Setting | Top Result | URL |
|-----------|------|---------|------------|-----|
| **TheAgentCompany** | 2024 | 175 tasks in simulated software company | 30.3% (Gemini 2.5 Pro) | [github](https://github.com/TheAgentCompany/TheAgentCompany) |
| **OdysseyBench** | 2025 | Long-horizon office workflows (Word, Excel, Email, Calendar) | -- | [github](https://github.com/microsoft/OdysseyBench) |
| **DeepPlanning** | 2026 | Multi-day travel + shopping with verifiable constraints | -- | [arxiv](https://arxiv.org/abs/2601.18137) |
| **HeroBench** | 2025 | RPG-inspired multi-layered dependency planning | -- | [arxiv](https://arxiv.org/abs/2508.12782) |

---

## 5. Tool Use

| Benchmark | Year | Scale | Key Metric | URL |
|-----------|------|-------|------------|-----|
| **BFCL/Gorilla** | 2025 (ICML) | V1-V4: AST eval, enterprise, multi-turn, agentic | Overall accuracy | [leaderboard](https://gorilla.cs.berkeley.edu/leaderboard.html) |
| **ToolBench** | 2023 | 16,000+ APIs, 3,451 tools | API call accuracy | [github](https://github.com/sambanova/toolbench) |
| **API-Bank** | 2023 (EMNLP) | 73 API tools, multi-turn + multi-call | API call accuracy, response quality | [paper](https://aclanthology.org/2023.emnlp-main.187.pdf) |

---

## 6. Safety

| Benchmark | Year | Scope | Key Finding | URL |
|-----------|------|-------|-------------|-----|
| **HAICOSYSTEM** | 2025 (COLM) | 132 scenarios, 7 domains, 8000+ simulations | Multi-turn surfaces 3x more risks than static | [arxiv](https://arxiv.org/abs/2409.16427) |

---

## 7. Survey Papers

| Title | Venue | Focus | URL |
|-------|-------|-------|-----|
| Evaluation and Benchmarking of LLM Agents: A Survey | SIGKDD 2025 | Two-dimensional taxonomy: objectives x process | [arxiv](https://arxiv.org/abs/2507.21504) |
| Survey on Evaluation of LLM-based Agents | 2025 | Planning, tool use, self-reflection, memory | [arxiv](https://arxiv.org/abs/2503.16416) |
| LLM-based Multi-Agents: A Survey of Progress and Challenges | IJCAI 2024 | Architecture, communication, evaluation | [github](https://github.com/taichengguo/LLM_MultiAgents_Survey_Papers) |
| Multi-Agent Collaboration Mechanisms: A Survey of LLMs | 2025 | Actors, types, structures, strategies, protocols | [arxiv](https://arxiv.org/abs/2501.06322) |
| A Survey on LLM-based Multi-Agent System: Recent Advances | 2024 | Applications: complex tasks, simulation, evaluation | [arxiv](https://arxiv.org/abs/2412.17481) |
| Generative Multi-Agent Collaboration in Embodied AI | IJCAI 2025 | First MAS + Embodied AI + foundation models survey | [paper](https://www.ijcai.org/proceedings/2025/1190.pdf) |
| Multi-Agent Coordination across Diverse Applications | 2025 | Collective intelligence for complex problems | [arxiv](https://arxiv.org/abs/2502.14743) |
| A Survey on LLM-based Autonomous Agents | 2023 | Seminal early survey | [arxiv](https://arxiv.org/abs/2308.11432) |

### Curated Paper Lists

- [Awesome Agent Papers](https://github.com/luo-junyu/Awesome-Agent-Papers)
- [LLM Agent Benchmark List](https://github.com/zhangxjohn/LLM-Agent-Benchmark-List)
- [LLM MultiAgents Survey Papers](https://github.com/taichengguo/LLM_MultiAgents_Survey_Papers)

---

## 8. Cross-Cutting Observations

### Performance Cliff on Long-Horizon Tasks

All benchmarks consistently show a dramatic performance drop as task complexity/duration increases:

| Short Task | Score | Long Equivalent | Score | Drop |
|------------|-------|-----------------|-------|------|
| SWE-bench Verified | ~80% | SWE-bench Pro | ~23% | -57pp |
| SWE-bench Verified | ~74% | FeatureBench | ~11% | -63pp |
| SWE-bench Verified | ~73% | SWE-EVO | ~25% | -48pp |
| HCAST (<1h tasks) | ~70-80% | HCAST (>4h tasks) | <20% | -50+pp |

METR time horizon doubling rate: ~4 months (2024-2026), but the gap between short and long tasks remains the central bottleneck.

### Common Evaluation Metrics by Category

| Category | Primary Metrics |
|----------|----------------|
| Code generation | pass@k, Resolve Rate |
| Agent tasks | Task completion / success rate |
| Multi-agent coordination | Milestone KPIs, Collaboration Score, Communication Score |
| Game | Win rate, game score, ToM accuracy |
| Social simulation | Believability, social intelligence dimensions |
| Long-horizon | 50% time horizon, pass rate at duration thresholds |
| Planning | Constraint satisfaction, plan quality |
| Tool use | API call accuracy, AST accuracy |
| Memory | Correctness, temporal reasoning accuracy |
| Safety | Multi-dimensional risk scores |

---

## 9. Multi-Agent-Specific Benchmarks (require multi-agent by design)

The benchmarks below are **specifically designed** for multi-agent evaluation — tasks that structurally require multiple agents, not single-agent benchmarks borrowed by multi-agent systems.

### 9.1 Collaboration

| Benchmark | Year | Why Multi-Agent Required | Agents | Multi-Agent Metrics | URL |
|-----------|------|--------------------------|--------|---------------------|-----|
| **MARBLE** | 2025 (ACL) | Role specialization + information partitioning; no single agent has full skill/info | 2-5 | Milestone KPIs, Communication Score, per-agent contribution | [github](https://github.com/MultiagentBench/MARBLE) |
| **AgentsNet** | 2025 | Distributed computing: each agent has only local info (neighbors) | 4-100 | Correctness, comm efficiency, scale degradation | [arxiv](https://arxiv.org/html/2507.08616v1) |
| **REALM-Bench** | 2025 | Inter-agent dependencies + dynamic disruptions in scheduling | Multi | Replanning success, coordination overhead | [github](https://github.com/genglongling/REALM-Bench) |
| **Collab-Overcooked** | 2025 (EMNLP) | Isolated environments with distinct action spaces; requires communication | 2 | TES, ITES, Initiating/Responding Capability | [github](https://github.com/YusaeMeow/Collab-Overcooked) |
| **CuisineWorld** | 2024 (NAACL) | Parallel workflows with shared utensils; cannot sequentialize | 2-4 | Collaboration Score (CoS) | [website](https://mindagent.github.io/) |
| **RoCoBench** | 2024 (ICRA) | Physical manipulation requires multiple simultaneous manipulators | 2 | Success rate, coordination quality | [website](https://project-roco.github.io/) |
| **LLM-Coordination** | 2025 (NAACL) | Pure coordination games: cannot "coordinate with itself" | 2 | Env Comprehension, ToM Reasoning, Joint Planning | [github](https://github.com/eric-ai-lab/llm_coordination) |
| **ColBench** | 2025 | Multi-turn human-agent pair-programming | 2 | Success rate, win rate | [github](https://github.com/facebookresearch/sweet_rl) |
| **SwarmBench** | 2025 | Strict decentralization: local perception only, no global controller | Multi | Swarm efficiency metrics | [github](https://github.com/RUC-GSAI/YuLan-SwarmIntell) |
| **LoopBench** | 2025 | Provably unsolvable by single deterministic agent (symmetry breaking) | Multi | Deadlock escape rate | [arxiv](https://arxiv.org/abs/2512.13713) |

### 9.2 Competition / Social Deduction

| Benchmark | Year | Game Type | Why Multi-Agent Required | URL |
|-----------|------|-----------|--------------------------|-----|
| **MAgIC** | 2024 (EMNLP) | Chameleon, Undercover, game theory | Hidden roles + deception + opponent modeling | [github](https://github.com/cathyxl/MAgIC) |
| **Werewolf Arena** | 2024 | Werewolf with bid-based turns | Asymmetric info: werewolves know each other, villagers don't | [arxiv](https://arxiv.org/abs/2407.13943) |
| **AvalonBench** | 2023 | Resistance: Avalon | Hidden identity: Merlin, Servants, Minion, Assassin | [github](https://github.com/jonathanmli/avalon-llm) |
| **CICERO** | 2022 (Science) | Diplomacy (7-player) | Negotiation + alliance + betrayal among 7 players | [github](https://github.com/facebookresearch/diplomacy_cicero) |
| **LLMArena** | 2024 (ACL) | 7 dynamic game environments | Opponent modeling + team collaboration | [paper](https://aclanthology.org/2024.acl-long.705/) |
| **CivBench** | 2026 | Civilization V multiplayer | Multi-hundred-turn strategy vs opponents | [arxiv](https://arxiv.org/abs/2604.07733) |
| **VS-Bench** | 2025 | 10 visual strategy environments | Cooperative, competitive, mixed-motive interactions | [website](https://vs-bench.github.io/) |
| **BattleAgentBench** | 2024 | Tank battles, 3 difficulty levels | Team coordination + competitive strategy | [arxiv](https://arxiv.org/abs/2408.15971) |

### 9.3 Negotiation / Communication

| Benchmark | Year | Setting | Key Finding | URL |
|-----------|------|---------|-------------|-----|
| **NegotiationArena** | 2024 (ICML) | Ultimatum, trading, price negotiation | LLMs boost payoffs 20% by pretending to be desperate | [arxiv](https://arxiv.org/abs/2402.05863) |
| **LLM-Deliberation** | 2024 (NeurIPS) | Multi-stakeholder negotiation with hidden values | Zero-shot CoT enables consistent deals | [github](https://github.com/S-Abdelnabi/LLM-Deliberation) |
| **M3MAD-Bench** | 2026 | Multi-agent debate across 5 domains + multimodal | MAD methods consume more tokens with little improvement | [github](https://github.com/liaolea/M3MAD-Bench) |
| **DEBATE** | 2025 (NeurIPS) | Opinion dynamics vs 30K real human messages | LLM groups show stronger opinion convergence than humans | [arxiv](https://arxiv.org/abs/2510.25110) |
| **ChatArena** | 2023 | MDP-based language games | Emergent communication patterns | [github](https://github.com/Farama-Foundation/chatarena) |

### 9.4 Social Simulation

| Benchmark | Year | Scale | Key Multi-Agent Evaluation | URL |
|-----------|------|-------|----------------------------|-----|
| **SOTOPIA** | 2024 (ICLR) | 90 scenarios, 40 characters | 7-dim social intelligence: believability, relationship, knowledge, secret-keeping, social rules, financial, goal completion | [website](https://sotopia.world/) |
| **Generative Agents** | 2023 (UIST) | 25 agents, 2-day simulation | Emergent behavior: info diffusion, relationship formation, event coordination | [github](https://github.com/joonspk-research/generative_agents) |
| **CAMEL** | 2023 (NeurIPS) | 2 agents, role-playing | 76.3% human eval win rate for multi-agent conversations | [github](https://github.com/camel-ai/camel) |

### 9.5 Multi-Agent Evaluation Frameworks

| Framework | Year | What It Measures | Key Finding | URL |
|-----------|------|------------------|-------------|-----|
| **MAESTRO** | 2026 | System-level: latency, cost, failure, variance | Architecture dominates over model choice | [github](https://github.com/sands-lab/maestro) |
| **MASEval** | 2026 | Framework-agnostic comparison (3x3x3 matrix) | Framework choice matters as much as model choice | [github](https://github.com/parameterlab/MASEval) |
| **MAFBench** | 2026 | Architecture taxonomy + performance | Framework design alone can increase latency 100x, reduce accuracy 30% | [arxiv](https://arxiv.org/abs/2602.03128) |

---

## 10. Evaluation Results of Existing Multi-Agent Systems

### 10.1 SWE-bench Leaderboard (April 2026)

**SWE-bench Verified (top model-level):**

| Agent/Model | Resolve Rate |
|-------------|-------------|
| Claude Mythos Preview | 93.9% |
| GPT-5.3 Codex | 85% |
| Claude Opus 4.5 | 80.9% |
| Gemini 3.1 Pro | 80.6% |
| MiniMax M2.5 | 80.2% |
| Average (80+ models) | 62.7% |

**Key insight:** Agent scaffolding matters enormously — three different frameworks running the same model scored **17 issues apart on 731 problems** (Feb 2026 test).

### 10.2 Multi-Agent Code Generation (concrete numbers)

| System | HumanEval pass@1 | Tokens/Task | Cost Factor |
|--------|-------------------|-------------|-------------|
| AgentCoder (3 agents) | 96.3% | 56.9K | 1x |
| MapCoder (4 agents) | 93.9% | -- | -- |
| MetaGPT (5 agents) | 85.9% | 138.2K | 2.4x |
| AgentVerse | -- | 149.2K | 2.6x |
| ChatDev (7 agents) | -- | 183.7K | 3.2x |

### 10.3 MARBLE Results (by topology)

| Topology | Task Score | Notes |
|----------|-----------|-------|
| Graph-mesh | Best | Best planning efficiency, moderate token cost |
| Star | Comparable | Reduced parallelism |
| Chain | Intermediate | -- |
| Tree | Worst | -- |

gpt-4o-mini achieved highest average task score. Cognitive planning improved milestones by +3%.

### 10.4 Game Benchmark Results

| Benchmark | Model | Result |
|-----------|-------|--------|
| Hanabi | DeepSeek-R1 | Score 30/25, ToM 77.68 |
| Hanabi | gpt-4.1 | Score 28.56, ToM 78.14 |
| Overcooked | GPT-4-turbo | Comparable to SOTA RL |
| Diplomacy | CICERO | 2x human average, top 10% |
| Avalon | ChatGPT (Good) | 22.2% win rate (vs 38.2% rule-based) |

---

## 11. Scaling Properties and Negative Results

### 11.1 When Multi-Agent Helps

| Condition | Evidence | Source |
|-----------|----------|--------|
| **Parallelizable tasks** | +80.8% over single-agent (financial reasoning) | Google Scaling (2025) |
| **Heterogeneous models** | Win rate 0.810 vs 0.512 for homogeneous | Understanding Agent Scaling (2026) |
| **Verification loops** | AgentCoder: independent test designer catches blind spots | AgentCoder (2024) |
| **Specialized roles** | SOEN-101 Scrum: +15% over single-agent baseline | FlowGen (2025) |

### 11.2 When Multi-Agent Hurts

| Condition | Evidence | Source |
|-----------|----------|--------|
| **Sequential reasoning** | 39-70% degradation vs single-agent | Google Scaling (2025) |
| **Equal token budget** | Single-agent matches or beats multi-agent on multi-hop reasoning | [arxiv](https://arxiv.org/abs/2604.02460) (2026) |
| **Homogeneous agents** | Single-agent with KV cache reuse is strictly better | [arxiv](https://arxiv.org/abs/2601.12307) (2025) |
| **>3-4 agents** | Communication overhead scales super-linearly (exponent 1.724) | Google Scaling |
| **Baseline >45% accuracy** | Coordination yields diminishing/negative returns | Google Scaling |
| **Debate (MAD)** | Does not reliably beat self-consistency or ensembling | [ICML 2024](https://arxiv.org/abs/2311.17371) |

### 11.3 Failure Modes (MAST Taxonomy, 2025)

Analyzed 200+ tasks across 7 MAS frameworks, 1,600+ annotated failure traces:

| Category | Share | Examples |
|----------|-------|---------|
| **Specification & Design** | 41.8% | Role confusion, unclear task decomposition, inadequate context sharing |
| **Inter-Agent Misalignment** | 36.9% | Sycophancy (r=0.902 with answer corruption), echo chambers, conformity pressure |
| **Task Verification** | 21.3% | Missing validation, incorrect success criteria |

**87.5% of multi-agent configurations underperformed baseline systems** in the broader scaling study.

### 11.4 Cost Analysis

| Metric | Multi-Agent | Single-Agent | Ratio |
|--------|-------------|--------------|-------|
| Tokens per HumanEval task (ChatDev) | 183.7K | ~56K | 3.2x |
| Token budget split: input (communication tax) | 53.9% | -- | -- |
| Token budget on verification/review | 86% | -- | -- |
| Monthly production cost (customer service) | $47K | $22.7K | 2.07x |
| Latency per query (high load) | 6.8s | 2.3s | 3.0x |

### 11.5 Key Scaling Findings

1. **Homogeneous scaling fails.** Adding identical agents hits diminishing returns rapidly; gains require **diversity** (different models, prompts, tools).
2. **Independent agents amplify errors 17.2x.** Centralized orchestration reduces to 4.4x.
3. **Architecture dominates model choice.** MAESTRO: MAS architecture is the primary driver of cost-latency-accuracy tradeoff.
4. **Framework design alone** can increase latency 100x and reduce accuracy 30% (MAFBench).
5. **Single-agent with skills** can reduce token usage 53.7% and latency ~50% vs multi-agent while maintaining accuracy ([arxiv](https://arxiv.org/abs/2601.04748)).
