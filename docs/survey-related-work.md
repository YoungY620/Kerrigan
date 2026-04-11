# 相关工作：DAG 与无通信协调

与 Kerrigan 的设计哲学（DAG 任务分解 + 无 agent 间通信 + 共享状态协调）相关的学术工作。

---

## 1. 高度相似

### Flash-Searcher (2025)

将 web 搜索任务编译为 DAG：独立子查询并行执行，有依赖的节点等待前置完成。中间发现信息不足时动态向 DAG 添加新节点。

- BrowseComp 67.7%，执行步骤 -35%，时间 -65%
- 与 Kerrigan 的区别：专用于 web 搜索，内存状态而非文件持久化，没有把"无通信"上升为设计原则
- [arxiv](https://arxiv.org/abs/2509.25301)

### Pressure Fields（压力场协调，2026）

形式化证明了 **stigmergy（趋化性）协调**的有效性：agent 只通过观察共享制品（artifact）的状态变化来协调，零直接消息传递。引入时间衰减避免重复关注。

- **30x 优于层级控制，4x 优于基于对话的方法**
- 证明了收敛性
- 与 Kerrigan 的区别：没有 DAG，agent 纯靠观察 artifact 压力自组织——完全去结构化
- [arxiv](https://arxiv.org/abs/2601.08129)

### VMAO（Verified Multi-Agent Orchestration，2026）

复杂问题→子问题 DAG→并行执行→LLM 验证→发现 gap 后动态 replan（添加新子问题）。与 Kerrigan 的 ready→execute→validate→add nodes 循环几乎一致。

- Completeness 3.1→4.2，Source quality 2.6→4.1（1-5 分制）
- 与 Kerrigan 的区别：有中心化的 synthesizer 做最终聚合；LLM-based 验证 vs 规则检查
- [arxiv](https://arxiv.org/abs/2603.11445)

### CodeCRDT（2025）

多 agent 协作写代码，通过观察共享 CRDT 文档（而非消息传递）协调。Outliner 生成代码骨架，Implementation Agent 认领 TODO，乐观写+验证协议处理冲突。

- 与 Kerrigan 的区别：CRDT 而非 DAG+JSON；无依赖关系概念
- [arxiv](https://arxiv.org/abs/2510.18893)

### A.DOT（2026）

将自然语言查询编译为 DAG 执行计划，支持缓存和重执行。

- 正确性 +14.8%，完整性 +10.7% vs 标准 RAG
- 与 Kerrigan 的区别：专用于混合数据湖上的 QA
- [arxiv](https://arxiv.org/abs/2603.14229)

---

## 2. DAG-Based，但保留 Agent 间通信

### TDAG（2024，Neural Networks 2025）

动态任务分解为 DAG + **按需生成专用 agent**——agent 根据子任务需求临时构造（包括 prompt、tools），用完销毁。与 Kerrigan 的"agent 没有生命周期，context 才有"吻合。

- 与 Kerrigan 的区别：agent 之间可以通信；在旅行规划上评估
- [arxiv](https://arxiv.org/abs/2402.10178)

### ReSo（2025，EMNLP Main）

DAG 分解 + 拓扑排序 + **协作奖励模型**从动态数据库中选择最适合的 agent。

- [paper](https://aclanthology.org/2025.emnlp-main.808/)

### DeMAC（2025，EMNLP Findings）

动态 DAG + Manager-Player 双反馈循环。在 Overcooked 上评估。

- [paper](https://aclanthology.org/2025.findings-emnlp.757/)

### S-DAG（2025，AAAI）

用 GNN 推断主题间依赖关系，构建 DAG，并通过 LLM 能力画像做主题-模型匹配。

- [arxiv](https://arxiv.org/abs/2511.06727)

### MacNet（2024，ICLR 2025）

DAG 拓扑，edge 上放 critic agent（发出指令），node 上放 actor agent（生产输出）。**支持 1000+ agent。不规则拓扑优于规则拓扑。** 协作 scaling law 呈 logistic 增长。

- +7% over AgentVerse，+13% over 单 agent CoT
- [arxiv](https://arxiv.org/abs/2406.07155)

---

## 3. 支持无通信设计的实证证据

### AgentPrune（ICLR 2025）

分析多智能体系统的 agent 间通信内容：**28.1%-72.8% 的通信 token 是冗余的**。剪掉后 cost 从 $43.7 降到 $5.6，效果还变好了。

- [arxiv](https://arxiv.org/abs/2410.02506)

### Drop the Hierarchy and Roles（2026）

25,000 任务实验：最小结构 + 角色自主优于严格层级 14%。Agent 从 8 个实例中自发产生了 5,006 个不同的角色。

- [arxiv](https://arxiv.org/abs/2603.28990)

### When Should We Orchestrate Multiple Agents?（2025）

形式化证明：多智能体编排仅在 agent 之间存在性能或成本差异时才有效。

- [arxiv](https://arxiv.org/abs/2503.13577)

### Blackboard Architecture for LLM Agents（2025）

经典 AI 的黑板架构：agent 通过读写共享工作区协调，不直接消息传递。13%-57% 提升。与 Kerrigan 的"结果即通信"一致，但黑板是非结构化的，Kerrigan 用结构化 DAG。

- [arxiv](https://arxiv.org/abs/2510.01285)

---

## 4. Kerrigan 的定位

现有工作在单个维度上与 Kerrigan 相似，但没有系统同时具备以下全部特性：

1. **DAG 任务分解**（Flash-Searcher, TDAG, VMAO 等都有）
2. **严格无 agent 间消息传递**（Pressure Fields, CodeCRDT 有，但无 DAG）
3. **文件系统（JSON）作为协调基底**（无主要学术系统采用）
4. **结构化 result_summary 作为通信机制**（Blackboard 有共享状态，但非结构化）
5. **动态 DAG 修改**（VMAO, TDAG, DeMAC 有）
6. **每节点上下文隔离**（AOrchestra 有类似理念）

Kerrigan 的独特贡献在于这六个属性的组合，以及背后"事决定人"的哲学基础。
