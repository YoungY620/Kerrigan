# Kerrigan 实现计划

## Context

将 thinking-space/kerrigan 中与 kitty 耦合的多智能体 swarm 机制，用 Rust 重新实现。按设计文档重新设计，不是直接移植。

**Repo 结构**：

- `kerrigan-dag-engine`（独立 repo/submodule）：DAG 引擎核心库 + CLI，包含 `~/.kerrigan/` 存储管理、任务注册表、DAG 图维护，所有接口基于 id
- `kerrigan-cli`（独立 repo/submodule）：编排层，依赖 kerrigan-dag-engine，包含 agent adapter、主循环、skills

核心原则：每一步高内聚、充分测试、对外接口承诺绝对有效。

## 步骤

### Step 1: DAG 引擎核心库 + 存储管理（kerrigan-dag-engine）

**目标**：实现 DAG 引擎的纯库层，包含 `~/.kerrigan/` 目录管理、任务注册表、DAG 图操作，所有接口基于 id，带完整单元测试。

**文件**：

```text
kerrigan-dag-engine/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs      # DagEngine struct：init, task_create, task_list + DAG 操作
│   ├── model.rs       # Task, Node, NodeSummary, NodeUpdate, TaskStatus, NodeMetadata, TaskEntry
│   ├── persist.rs     # JSON 序列化/反序列化 + 原子写入 + 文件锁
│   └── error.rs       # DagError 枚举
├── tests/
│   └── engine_test.rs
```

**实现内容**：

存储管理：
- `init()`：初始化 `~/.kerrigan/` 目录（幂等）
- `task_create(summary, workdir) -> task_id`：生成 task-id、创建目录结构、注册到 registry.json
- `task_list() -> Vec<TaskEntry>`：列出所有注册的任务
- 文件锁：`fs2` crate，保证并发安全
- 原子写入：write tmp + rename

DAG 操作（所有接口通过 task_id 定位 dag.json）：
- `TaskStatus` 枚举：Pending / Ready / Running / Succeeded / Failed
- `Node` 结构体：id, summary, description, predecessors, successors, status, metadata
- `NodeMetadata`：result_summary, failure_reason, spawned_tasks, session_ids
- `add_node(task_id, NewNode, depends_on) -> Result<NodeId>`
- `list_all(task_id) -> Vec<NodeSummary>`
- `list_ready(task_id) -> Vec<NodeSummary>`
- `update_nodes(task_id, Vec<NodeUpdate>) -> Result<()>`（含级联 Ready 逻辑）
- `get_node(task_id, node_id) -> Option<Node>`

**依赖**：serde, serde_json, uuid, thiserror, fs2

**测试要点**：
- init 幂等：多次调用不破坏已有数据
- task_create → task_list 一致性
- 并发 task_create：两个进程同时创建不冲突
- 目录结构完整性：create 后 dag.json / config.json / sessions.json / board.md / shared/ 都存在
- 基础 CRUD：add_node → get_node → list_all 一致性
- 依赖解析：add A, add B(depends_on A) → B 是 Pending；complete A → B 变 Ready
- 多前驱：C depends_on [A, B]，A 完成后 C 仍 Pending，B 也完成后 C 变 Ready
- 级联深度：A→B→C→D 链式级联
- 失败不传播：A Failed → B 永远 Pending
- update_nodes 批量：一次更新多个节点
- 环检测：add_node 不能引入环
- 持久化往返：engine → save → load → engine，状态完全一致
- 并发文件锁：两个进程同时操作同一个 dag.json

**验证方式**：`cargo test`，100% 通过

---

### Step 2: DAG CLI（kerrigan-dag-engine）

**目标**：在同一 repo 中为 DAG 引擎提供 CLI 入口，所有接口基于 id，JSON 输出，可被 agent skills 直接调用。

**文件**：

```text
kerrigan-dag-engine/src/
├── main.rs
├── cli.rs
```

**CLI 接口**：

```bash
kerrigan-dag init                                                    # 初始化 ~/.kerrigan/
kerrigan-dag task create --summary "..." --workdir .                 # 创建任务
kerrigan-dag task list                                               # 列出所有任务

kerrigan-dag add --task <task-id> --summary "..." --description "..." --depends-on n1,n2
kerrigan-dag list --task <task-id>                                   # list_all, JSON output
kerrigan-dag ready --task <task-id>                                  # list_ready, JSON output
kerrigan-dag update --task <task-id> --input <json>                  # update_nodes, JSON input
kerrigan-dag get --task <task-id> --node <node-id>                   # get_node, JSON output
```

- 所有输出为 JSON（方便 agent 解析）
- 所有操作通过 task-id 定位，不暴露文件路径
- `update` 从 stdin 或 `--input` 读取 JSON

**依赖**：clap (derive), assert_cmd, predicates (dev)

**测试要点**：
- CLI 端到端：通过 `assert_cmd` 测试每个子命令的 stdin/stdout/exit code
- 错误场景：不存在的 task-id、不存在的 node-id、非法 JSON 输入
- 与 Step 1 的库层一致性：CLI 输出 == 直接调用引擎的结果

**验证方式**：`cargo test` + 手动 CLI 调用验证

---

### Step 3: Agent Adapter（kerrigan-cli）

**目标**：在 kerrigan-cli 中实现 agent CLI 的封装层，支持 Claude Code 作为首个 adapter。

**文件**：

```text
kerrigan-cli/
├── Cargo.toml              # 依赖 kerrigan-dag-engine
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── adapter/
│   │   ├── mod.rs
│   │   ├── traits.rs      # AgentAdapter trait
│   │   ├── claude.rs      # Claude Code adapter
│   │   └── session.rs     # session id 生成与管理
│   └── cli/
│       └── mod.rs
```

**AgentAdapter trait**：

```rust
pub trait AgentAdapter {
    async fn run(&self, prompt: &str, session_id: &str, skills_dir: Option<&Path>) -> Result<AgentOutput>;
    async fn resume(&self, session_id: &str, prompt: &str) -> Result<AgentOutput>;
}
```

**Claude Code adapter**：
- 调用 `claude --print --session-id <id> --prompt <prompt>` 非交互式模式
- 支持 `--allowedTools` 和 skills 注入
- 解析 stdout 为 AgentOutput

**Session 管理**：`krg_{timestamp}_{random}`，记录到 `~/.kerrigan/tasks/<task-id>/sessions.json`

**测试要点**：
- trait 可 mock：用 MockAdapter 测试上层逻辑
- session id 唯一性
- Claude adapter 命令行构造正确性（不真实调用）
- 可选集成测试

**验证方式**：`cargo test`

---

### Step 4: 提示词模板 + 主循环（kerrigan-cli）

**目标**：实现 Kerrigan 的核心编排循环。

**文件**：

```text
kerrigan-cli/src/
├── runner/
│   ├── mod.rs
│   ├── loop.rs        # 主循环逻辑
│   ├── templates.rs   # 提示词模板渲染
│   └── board.rs       # 留言板监控
├── cli/
│   └── run.rs         # kerrigan run <task-id>
```

**三个提示词模板**（内置，可覆盖）：
1. **init.md**：首轮，注入 DAG skills 说明 + ready 节点 + 任务描述
2. **continue.md**：后续轮次，注入最新 DAG 状态
3. **resume.md**：新 session 恢复，引导读取所有信息后继续

**主循环**：

```text
loop {
    ready = kerrigan-dag list_ready(task_id)
    if ready.is_empty() { break }
    for node in ready {
        // 构造 prompt（注入前置节点的 result_summary）
        // 通过 adapter 启动 subagent
        // 解析结果，调用 kerrigan-dag update_nodes()
    }
}
```

**CLI 接口**：

```bash
kerrigan run <task-id> --adapter claude
kerrigan run <task-id> --dry-run
```

**测试要点**：
- 模板渲染正确性
- 循环终止：mock adapter，DAG 全部完成后退出
- 失败不传播，循环继续
- 衍生任务正确插入
- dry-run 不调用 adapter
- 留言板注入

**验证方式**：`cargo test` + mock adapter 端到端跑完 3 节点 DAG

---

### Step 5: Skills 封装

**目标**：将 DAG 引擎和 Kerrigan CLI 封装为 skills。

**文件**（放在 monorepo 或 kerrigan-cli 下）：

```text
skills/
├── kerrigan-dag/
│   ├── SKILL.md           # DAG 操作 skill
│   └── scripts/           # 调用 kerrigan-dag 子命令
└── kerrigan-swarm/
    ├── SKILL.md           # 任务管理 skill
    └── scripts/           # 调用 kerrigan task/run
```

两层 skills：
1. **kerrigan-dag skills**：供 loop 内 agent 操作 DAG
2. **kerrigan-swarm skills**：供外部 agent 创建/管理后台任务

**验证方式**：手动验证 + 可选集成测试

---

## 依赖总览

| Crate | 用途 | Repo |
|-------|------|------|
| serde, serde_json | 序列化 | both |
| clap (derive) | CLI 解析 | both |
| uuid | task/node id 生成 | dag-engine |
| thiserror | 错误类型 | both |
| fs2 | 文件锁 | dag-engine |
| tokio | 异步运行时 | kerrigan-cli |
| minijinja | 模板渲染 | kerrigan-cli |
| assert_cmd, predicates | CLI 测试 | both (dev) |
| tempfile | 测试临时目录 | both (dev) |

## 执行顺序

```text
kerrigan-dag-engine:
  Step 1 (引擎库 + 存储管理) → Step 2 (CLI)

kerrigan-cli:
  Step 3 (Agent Adapter) → Step 4 (主循环) → Step 5 (Skills)
```

Step 1-2 在 kerrigan-dag-engine repo 完成后，kerrigan-cli 通过 path 依赖引用。每一步完成后提交 + push。
