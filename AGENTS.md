# Agent Guidelines

本项目所有 agent（Claude Code、compatible agent CLI 等）在操作本仓库时遵守以下规则。

---

## 核心设计决策（必读）

在做任何功能设计或实现之前，必须理解以下约束：

### Append-Only 存储

**Kerrigan 的存储是严格 append-only 的。** 节点、边、日志只能增加，不能删除或修改结构（除节点 status 字段外）。

**原因**：失败和错误节点必须保留完整痕迹，这是 reflect 分析、因果归因（blame）和 causal analysis 的数据基础。允许删除会使诊断能力失去历史依据。

**实践含义**：
- 不设计"删除节点"或"删除边"的 API
- 错误处理（如 ContractCreationError）：旧节点标记 failed 并保留，创建新节点，而非覆盖
- dag override 等命令只新增记录，不删除或覆盖既有数据
- 所有新功能设计前先确认是否破坏 append-only 约束

### 结果即通信

Agent 间没有消息通道。节点的唯一对外输出是 `result_summary`（v1.1.0 起必须是满足 `output.schema` 的 JSON 对象）。任何协调需求都通过 DAG 结构（增节点、加依赖）而非消息传递来表达。

### 不可变 vs 可变边界

| 不可变（永不修改）| 可变（系统可修改）|
|---|---|
| Rust 二进制、DAG 执行语义、engine.rs | `~/.kerrigan/skills/*.md` |
| cascade_ready / cascade_pending 算法 | `~/.kerrigan/gene.db`（未来）|
| | `~/.kerrigan/proposals/`（未来）|

---

## Git Commit 规范

使用 Conventional Commits 格式：

```
<type>: <description>
```

### Type 与版本号的对应关系

| Type | 含义 | 版本递增 |
|------|------|---------|
| `feat:` | 新功能 | patch (0.0.Z) |
| `fix:` | 修复 bug | patch (0.0.Z) |
| `release:` | 正式发布稳定版本 | minor (0.Y.0) |
| `BREAKING CHANGE:` | 彻底重写/换架构/换语言 | major (X.0.0) |
| `docs:` | 文档变更 | 不触发版本递增 |
| `chore:` | 构建/工具/依赖 | 不触发版本递增 |
| `ci:` | CI 配置 | 不触发版本递增 |
| `refactor:` | 重构（不改行为） | 不触发版本递增 |
| `test:` | 测试 | 不触发版本递增 |

### 规则

- commit message 第一行必须以上述 type 开头
- 不添加 `Co-Authored-By`、`Signed-off-by` 或任何 AI 署名
- 不修改 git author 或 committer 配置
- 每个内聚功能单独一个 commit，不要把多个独立功能合并在一次提交里

---

## 主分支保护

**main 分支上的每个 commit 都必须是立即可用的完整版本。**

- 不允许存在实现了一半的功能、编译不通过、测试不过的 commit
- 如果功能需要分步实现，在开发分支完成后再合并到 main
- 合并前确保 `cargo test` 全部通过
- push 到 origin（GitHub）

---

## 发版

使用 `make release` 发版：

```bash
make release              # 自动版本号
make release VERSION=1.1.0  # 手动覆盖
```

脚本自动从 commit types 推断版本号，更新 Cargo.toml，打 tag 并 push。

---

## 项目结构

单 crate Rust 项目（v1.0.0 起从 monorepo 合并为单 crate）：

```
kerrigan/
├── src/              # Rust 源码
│   ├── engine.rs     # DAG 核心引擎（TaskEngine + RegistryEngine）
│   ├── db.rs         # SQLite schema + 迁移机制（PRAGMA user_version）
│   ├── model.rs      # 数据模型（NodeContract 等）
│   ├── cli.rs        # CLI 参数定义
│   ├── main.rs       # 命令路由
│   ├── exec.rs       # Worker 执行器
│   ├── send.rs       # 消息注入
│   └── watch.rs      # 事件流监控
├── skills/kerrigan/  # 内嵌 skill 文件（编译时通过 include_str! 嵌入）
├── tests/            # CLI 集成测试（assert_cmd）
├── docs/             # 设计文档
└── Cargo.toml
```

---

## 构建与测试

```bash
cargo build           # debug build
cargo build --release # release build
cargo test            # 单元测试 + CLI 集成测试（全部必须通过）
make install          # 安装到 ~/.cargo/bin/
```

安装到本机 PATH（`~/.local/bin/`）：

```bash
cp target/release/kerrigan ~/.local/bin/kerrigan
kerrigan init
```
