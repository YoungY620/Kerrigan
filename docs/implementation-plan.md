# Kerrigan CLI 实现计划

## Context

将 thinking-space/kerrigan 中与 kitty 耦合的多智能体 swarm 机制，用 Rust 重新实现为独立的 kerrigan-cli。现有 Python 实现（`kitty-worker/src/kitty_worker/swarm/`）提供了参考，但 Rust 版本按设计文档重新设计，不是直接移植。

核心原则：每一步高内聚、充分测试、对外接口承诺绝对有效。

## 步骤

### Step 1: Rust 项目脚手架 + DAG 引擎核心库

**目标**：实现纯库层的 DAG 引擎，5 个接口全部可用，带完整单元测试。

**文件**：
```
kerrigan-cli/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── dag/
│   │   ├── mod.rs         # pub mod
│   │   ├── engine.rs      # DagEngine struct + 5 个接口实现
│   │   ├── model.rs       # Task, TaskSummary, TaskUpdate, TaskStatus, TaskMetadata
│   │   ├── persist.rs     # JSON 序列化/反序列化 + 文件锁
│   │   └── error.rs       # DagError 枚举
│   └── ...
├── tests/
│   └── dag_engine_test.rs # 集成测试
```

**实现内容**：
- `TaskStatus` 枚举：Pending / Ready / Running / Succeeded / Failed
- `Task` 结构体：id, summary, description, predecessors, successors, status, metadata
- `TaskMetadata`：result_summary, failure_reason, spawned_tasks, session_ids
- `DagEngine` 5 个接口：
  - `add_task(NewTask, depends_on: Vec<TaskId>) -> Result<TaskId>`
  - `list_all() -> Vec<TaskSummary>`
  - `list_ready() -> Vec<TaskSummary>`
  - `update_tasks(Vec<TaskUpdate>) -> Result<()>`（含级联 Ready 逻辑）
  - `get_task(TaskId) -> Option<Task>`
- JSON 持久化：每次变更后原子写入（write tmp + rename）
- 文件锁：`fs2` crate 的 flock，保证并发安全

**依赖**：serde, serde_json, uuid, thiserror, fs2

**测试要点**：
- 基础 CRUD：add → get → list_all 一致性
- 依赖解析：add A, add B(depends_on A) → B 是 Pending；complete A → B 变 Ready
- 多前驱：C depends_on [A, B]，A 完成后 C 仍 Pending，B 也完成后 C 变 Ready
- 级联深度：A→B→C→D 链式级联
- 失败不传播：A Failed → B 永远 Pending
- update_tasks 批量：一次更新多个任务
- 环检测：add_task 不能引入环（可选，DAG 应该由引擎保证无环）
- 持久化往返：engine → save → load → engine，状态完全一致
- 并发文件锁：两个进程同时操作同一个 dag.json

**验证方式**：`cargo test`，100% 通过

---

### Step 2: DAG CLI

**目标**：将 DAG 引擎包装为 CLI 子命令，JSON 输入输出，可被 agent skills 直接调用。

**文件**：
```
kerrigan-cli/src/
├── main.rs
├── cli/
│   ├── mod.rs
│   └── dag.rs        # kerrigan dag add/list/ready/update/get 子命令
```

**CLI 接口**：
```bash
kerrigan dag add --dag-file <path> --summary "..." --description "..." --depends-on id1,id2
kerrigan dag list --dag-file <path>                    # list_all, JSON output
kerrigan dag ready --dag-file <path>                   # list_ready, JSON output
kerrigan dag update --dag-file <path> --input <json>   # update_tasks, JSON input
kerrigan dag get --dag-file <path> --id <task_id>      # get_task, JSON output
```

- 所有输出为 JSON（方便 agent 解析）
- `--dag-file` 指定 dag.json 路径
- `dag update` 从 stdin 或 `--input` 读取 JSON

**依赖**：clap (derive)

**测试要点**：
- CLI 端到端：通过 `assert_cmd` 测试每个子命令的 stdin/stdout/exit code
- 错误场景：不存在的 dag-file、不存在的 task id、非法 JSON 输入
- 与 Step 1 的库层一致性：CLI 输出 == 直接调用引擎的结果

**验证方式**：`cargo test` + 手动 CLI 调用验证

---

### Step 3: `~/.kerrigan/` 目录管理 + 任务注册表

**目标**：实现跨任务的全局管理层，kerrigan CLI 的顶层命令。

**文件**：
```
kerrigan-cli/src/
├── registry/
│   ├── mod.rs
│   ├── store.rs       # Registry CRUD (registry.json)
│   └── model.rs       # RegistryEntry
├── cli/
│   ├── task.rs        # kerrigan task create/list/status/inspect 子命令
│   └── init.rs        # kerrigan init
```

**目录结构**：
```
~/.kerrigan/
├── registry.json
├── lock
└── tasks/<task-id>/
    ├── dag.json
    ├── config.json
    ├── sessions.json
    ├── board.md
    └── shared/
```

**CLI 接口**：
```bash
kerrigan init                                      # 初始化 ~/.kerrigan/
kerrigan task create --summary "..." --workdir .   # 创建任务，生成目录
kerrigan task list                                 # 列出所有注册的任务
kerrigan task status <task-id>                     # 查看某任务的 DAG 状态摘要
kerrigan task inspect <task-id>                    # 查看详细信息
```

- `task create` 自动生成 task-id（时间戳 + 随机后缀）、创建目录、注册到 registry.json
- 所有文件操作通过全局文件锁保护
- `task list` 显示 id / summary / 状态统计 / 创建时间

**测试要点**：
- init 幂等：多次调用不破坏已有数据
- create → list 一致性
- 并发 create：两个进程同时创建不冲突
- 目录结构完整性：create 后所有文件都存在

**验证方式**：`cargo test` + 在真实 `~/.kerrigan/` 下手动验证

---

### Step 4: Agent Adapter

**目标**：实现 agent CLI 的封装层，支持 Claude Code 作为首个 adapter。

**文件**：
```
kerrigan-cli/src/
├── adapter/
│   ├── mod.rs
│   ├── traits.rs      # AgentAdapter trait
│   ├── claude.rs      # Claude Code adapter
│   └── session.rs     # session id 生成与管理
```

**AgentAdapter trait**：
```rust
pub trait AgentAdapter {
    /// 非交互式执行 prompt，返回输出
    async fn run(&self, prompt: &str, session_id: &str, skills_dir: Option<&Path>) -> Result<AgentOutput>;
    /// resume 已有 session
    async fn resume(&self, session_id: &str, prompt: &str) -> Result<AgentOutput>;
}
```

**Claude Code adapter 实现**：
- 调用 `claude --print --session-id <id> --prompt <prompt>` 非交互式模式
- 支持 `--allowedTools`
- 支持通过 `--append-system-prompt` 或环境变量注入 skills
- 解析 stdout 为 AgentOutput（文本 + 结构化结果）

**Session 管理**：
- 生成格式：`krg_{timestamp}_{random}`
- 记录到 `sessions.json`

**测试要点**：
- trait 可 mock：用 MockAdapter 测试上层逻辑
- session id 唯一性：生成 1000 个不重复
- Claude adapter 命令行构造正确性（不需要真实调用，验证构造的命令参数）
- 真实集成测试（可选，需要 claude 可用）：简单 prompt → 获取输出

**验证方式**：`cargo test`，可选 `cargo test --features integration`

---

### Step 5: 提示词模板 + 主循环 (Loop of Agent Loop)

**目标**：实现 Kerrigan 的核心编排循环。

**文件**：
```
kerrigan-cli/src/
├── loop_engine/
│   ├── mod.rs
│   ├── runner.rs      # 主循环逻辑
│   ├── templates.rs   # 提示词模板渲染
│   └── board.rs       # 留言板监控
├── cli/
│   └── run.rs         # kerrigan run <task-id>
```

**三个提示词模板**（内置，可覆盖）：
1. **init.md**：首轮提示词，注入 DAG skills 说明 + 当前 ready 节点 + 任务描述
2. **continue.md**：后续轮次，注入最新 DAG 状态 + ready 节点
3. **resume.md**：新 session 恢复，引导读取所有可用信息后继续

**主循环**：
```
loop {
    ready = dag.list_ready()
    if ready.is_empty() { break }

    for task in ready {
        // 构造 prompt（注入前置任务的 result_summary）
        // 通过 adapter 启动 subagent
        // 解析结果，调用 dag.update_tasks()
        // 如果 agent 衍生了新任务，调用 dag.add_task()
    }
}
```

**留言板**：监控 `board.md` 的 mtime，变化时在下一轮注入内容

**CLI 接口**：
```bash
kerrigan run <task-id> --adapter claude   # 启动/恢复任务执行
kerrigan run <task-id> --dry-run          # 只显示会执行什么，不真正调用 agent
```

**测试要点**：
- 模板渲染：给定 DAG 状态 → 正确的 prompt 输出
- 循环终止：mock adapter，DAG 全部完成后退出
- 失败处理：某节点失败 → 不传播，循环继续处理其他 ready 节点
- 衍生任务：agent 返回新任务 → 正确插入 DAG
- dry-run 模式：不调用 adapter，只输出计划
- 留言板注入：board.md 变化 → 下一轮 prompt 包含留言内容

**验证方式**：`cargo test` + 用 mock adapter 端到端跑完一个 3 节点 DAG

---

### Step 6: Skills 封装

**目标**：将 DAG 引擎和 Kerrigan CLI 封装为 skills，供 agent 使用。

**文件**：
```
kerrigan-cli/
├── skills/
│   ├── kerrigan-dag/
│   │   ├── SKILL.md           # DAG 操作 skill 说明
│   │   └── scripts/           # 调用 kerrigan dag 子命令的 wrapper
│   └── kerrigan-swarm/
│       ├── SKILL.md           # Kerrigan 任务管理 skill 说明
│       └── scripts/           # 调用 kerrigan task/run 的 wrapper
```

**DAG skills（供 loop 内 agent 使用）**：
- SKILL.md 描述何时触发、可用操作
- 脚本调用 `kerrigan dag add/list/ready/update/get`

**Kerrigan swarm skills（供外部 agent 使用）**：
- SKILL.md 描述如何创建和管理后台任务
- 脚本调用 `kerrigan task create/list/status` 和 `kerrigan run`

**测试要点**：
- SKILL.md 内容完整性：包含所有接口说明和使用示例
- 脚本可执行：每个脚本独立运行不报错
- 与真实 agent 集成测试（可选）：注入 skills 后 agent 能正确调用

**验证方式**：手动验证 + 可选集成测试

---

## 依赖总览

| Crate | 用途 |
|-------|------|
| serde, serde_json | 序列化 |
| clap (derive) | CLI 解析 |
| uuid | task id 生成 |
| thiserror | 错误类型 |
| fs2 | 文件锁 |
| tokio | 异步运行时（adapter 层） |
| tera 或 minijinja | 模板渲染 |
| assert_cmd, predicates | CLI 测试 |
| tempfile | 测试临时目录 |

## 执行顺序与依赖

```
Step 1 (DAG 引擎库)
  └→ Step 2 (DAG CLI)
       └→ Step 3 (~/.kerrigan/ + 注册表)
            └→ Step 4 (Agent Adapter)
                 └→ Step 5 (主循环)
                      └→ Step 6 (Skills)
```

每一步完成后提交 + push，确保 CI 绿色后再进入下一步。
