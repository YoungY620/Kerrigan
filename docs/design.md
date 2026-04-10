# Kerrigan: 一个无通信的 Agent 间协作协议

## 为什么需要 Multi-Agent

即使仅仅是并行优化，多智能体也是有意义的。而且即使不考虑并行，**agent 运行的最佳状态是刚好看到所有它需要的上下文**（关键假设）。

后面的所有设计或多或少都基于这个假设（但这个假设也可能被推翻）。

## 无通信的通信协议

很多 multi-agent 架构的论文或框架不管叙事如何，都会涉及一个关键问题：agent 之间的通信复杂度。点对点通信、广播会导致上下文写放大，同时主动沟通还会打断另一个 agent 的正常工作（污染 session）。对人类来说这无法避免——这也是《人月神话》的核心。但对 agent 来说完全没有必要。

任何复杂度的任务最终都可以做任务分解。agent 间通信的原因一定是任务之间存在依赖，那么一定可以把复杂任务通过任务分解和依赖分析构建为一个有向无环图（DAG）。DAG 可以做拓扑排序——每一轮遍历整个图，找到所有没有前置依赖的任务节点，这些节点可以无依赖地并行执行。因此，只需要模型支持 subagent/task 功能，就可以实现复杂任务的拆分和并行执行。每个 subagent 执行具体小任务时拿到的是刚好足够的上下文。

如果 agent 在执行中发现需要沟通，或从执行结果发现需要进一步的任务？任何任务都可以拆分为"沟通前"和"沟通后"。agent 只需停止当前任务，维护整个 DAG（增加/修改节点），下一轮继续按拓扑排序找到可并行的任务执行即可。

## 维护 DAG

上面的需求实际上只需要实现：

1. 一个 Storage 层（DAG 管理 + 存储）及相应的 CLI
2. 相应的 skills
3. 一个 agent adapter，使任何 agent CLI（kimi-cli、claude code 等）只需支持 `--resume`、subagent 和非交互式调用即可接入，最好还支持自定义 skills 目录

然后实现一个循环：每一轮调用一个主 agent（注入 DAG skills 到系统提示词），由该 agent 自行通过 DAG skills 查找可执行节点、用 subagent 并行执行、回写结果。kerrigan 的 loop 只负责检查终止条件（ready 节点为空）并反复调用主 agent，编排逻辑全在 agent 侧通过 skills 完成。

### Storage（DAG 引擎 + 存储管理）

#### 任务数据模型

一个任务包含以下字段：

- **id**：任务唯一标识
- **summary**：任务简述，足够简短，足够让人一眼理解任务意图
- **description**：任务详细描述和相关引用，整体为 markdown 格式字符串
- **successors**：依赖此任务的后继节点 id 列表
- **status**：任务状态枚举（Pending / Ready / Running / Succeeded / Failed）
- **metadata**：有限的可选字段
  - 失败时：包括失败原因、是否需要增加新任务或新资源等。任务不管成功失败永远不会"继续"，只会执行一次并留下结果。如果需要"继续"，则生成新节点与现有节点建立依赖关系，产生新的结果。失败的任务会留下一个或多个后续任务，说明如何解决错误或进行相关探索，这些尝试的后续任务本质上是对原任务的重试。
  - 成功时：给出执行结果的详细总结，保证后续 agent 能拿到足够的信息而不需要重新探索。

#### 存储管理

Storage 层统一管理 `~/.kerrigan/` 下的全局状态，所有接口基于 task id 调用，调用方不需要知道文件路径细节：

```text
~/.kerrigan/
├── registry.json          # 注册表：所有任务的索引
├── lock                   # 全局文件锁，保证并发安全
└── tasks/
    └── <task-id>/
        ├── dag.json       # 该任务的 DAG 状态
        ├── config.json    # 任务配置（工作目录、agent adapter 类型等）
        ├── sessions.json  # 使用过的 session id 列表，用于 debug
        ├── board.md       # 留言板，供人工向运行中的任务注入信息
        └── shared/        # 该任务的共享文件（跨节点可访问的资源）
```

- **registry.json**：全局注册表，记录每个任务的 id、摘要、状态、创建时间、工作目录路径等
- **tasks/\<task-id\>/**：每个任务独立一个目录，DAG 文件、session 记录、留言板等都在这里
- **并发安全**：多个后台 swarm job 并发访问注册表、公共区域文件时，通过文件锁机制保证一致性

#### 引擎接口

所有接口基于 task id，引擎内部自动定位到 `~/.kerrigan/tasks/<task-id>/dag.json`：

- `init()`：初始化 `~/.kerrigan/` 目录结构
- `task_create(summary, workdir) -> task_id`：创建任务，生成目录，注册到 registry
- `task_list() -> Vec<TaskEntry>`：列出所有注册的任务
- `add_task(task_id, node, depends_on) -> node_id`：向指定任务的 DAG 添加新节点，指定依赖的前置节点 id 列表。自动维护前驱/后继双向关系，若所有前置已完成则状态直接为 Ready。
- `list_all(task_id) -> Vec<NodeSummary>`：返回指定任务所有节点的轻量摘要（id、summary、successors、status），同时可用于统计各状态数量，了解全局进度。
- `list_ready(task_id) -> Vec<NodeSummary>`：返回指定任务所有 status 为 Ready 的节点。
- `update_nodes(task_id, Vec<NodeUpdate>)`：批量更新指定任务中一个或多个节点的字段值（status、metadata 等）。引擎内部根据状态变更自动处理级联——当某节点变为 Succeeded 时，检查其后继节点的前置是否全部完成，是则将后继变为 Ready。
- `get_node(task_id, node_id) -> Node`：查询指定任务中单个节点的完整详情，包括 description 和 metadata。供 agent 读取前置节点的执行结果摘要。

#### 关键设计原则

- **基于 id 操作**：所有接口通过 task id 和 node id 调用，不暴露文件路径细节，降低调用方复杂度。
- **一次执行**：节点只执行一次，永不原地重试。失败后通过 `add_task` 衍生新的修复/重试节点并建立依赖关系。
- **终止条件**：`list_ready()` 为空，即没有可执行节点。
- **持久化**：DAG 状态持久化为 JSON 文件，每次状态变更后落盘，天然支持 `--resume` 恢复。
- **职责分离**：引擎不负责执行，只维护图和状态。agent adapter 层负责调用具体 agent，拿到结果后调 `update_nodes` 回写。
- **结果即通信**：`result_summary` 是核心通信机制——后续节点的 agent 通过 `get_node` 读取前置的结果摘要，而非共享文件或消息队列。

### Agent Adapter

封装 agent CLI 的两个核心能力：subagent 调用和 session 管理。首批支持 Claude Code 和 Kimi CLI 两个 adapter。

- 非交互式 prompt 下发，支持 subagent，支持指定 session id（便于后续 resume）
- 在指定的 session id 上 resume
- 支持指定 skills 目录
- 默认开启 YOLO 模式（自动批准所有工具调用，无需人工确认），agent 子任务需要全自动执行

Session 由各 agent CLI 自行创建和管理（claude/kimi 各自的 session 机制），kerrigan 不做 session 归一化存储，只在 `sessions.json` 中记录 node_id 与 agent session id 的映射关系，用于 debug 追溯。

### Loop of Agent Loop

初始化一个任务后，Kerrigan 框架准备好标准的文件系统，然后进入循环。

同一任务内所有轮次复用同一个 agent session。每经过 N 次 continue turn（默认 N=10）后，主动更换为新 session 并使用 resume 模板恢复上下文，避免单个 session 上下文过长导致质量下降。中断停机后恢复同样开新 session。每一轮提示词由 kerrigan 程序侧预先查询当前 ready 节点的 summary 信息并注入，同时要求 agent 使用 Storage skill 来获取节点详情、执行任务、回写结果。

每轮 agent 执行完成后，kerrigan 程序侧对 DAG 进行静态一致性检查：
- 前驱为空的节点是否为 Ready 状态
- Ready 节点是否确实没有未完成的前驱
- Succeeded/Failed 节点是否有 result_summary / failure_reason
- 必要字段（summary、description）是否非空

如果检查不通过，构造修复提示词 resume 同一 session 要求 agent 修正，直到 DAG 状态一致。

需要准备三个提示词模板：

1. **初始化**：一个任务的第一个提示词，第一个 turn。注入所有 ready 节点的 summary，要求 agent 通过 Storage skill 获取详情和前置结果，用 subagent 并行执行，完成后通过 skill 回写结果和衍生新节点。
2. **Continue turn**：resume 当前 session，注入最新 ready 节点 summary。
3. **Resume 任务**：中断恢复时开一个新 session。注入最新 ready 节点 summary，引导 agent 仔细阅读所有可用、可读、可索引的信息，了解问题全貌后继续。

### 外部信息输入

提供一个留言板文件。在每个提示词中引导主 agent 在 subagent 运行时检测此文件的变化，并决定是否修改迭代 DAG 或中断某些 subagent。

### 跨任务打通

不同任务之间分别管理各自的 DAG，通过 Storage 的统一接口（`task_list`、`task_create` 等）统一管理。存储结构和注册表由 Storage 层维护（见上文"存储管理"），Kerrigan CLI 基于这些接口实现上层编排，所有任务在后台执行，用户通过 CLI 查询和维护。

### Skills

两个层次的 skills：

1. **Storage skills**：Storage CLI 封装为 skills，供 Kerrigan loop 中调用的 agent CLI 使用
2. **Kerrigan CLI skills**：Kerrigan CLI 封装为 skills，供外部任何 agent 使用，用于部署后台服务，或直接适配到 chatbot
   - 是否需要一个 SDK（优先级非常低）
