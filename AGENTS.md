# Agent Guidelines

本项目所有 agent（Claude Code、Kimi CLI 等）在操作本仓库时遵守以下规则。

## Git Commit 规范

使用 Conventional Commits 格式：

```
<type>: <description>
```

### Type 与版本号的对应关系（Semantic Versioning X.Y.Z）

| Type | 含义 | 版本递增 |
|------|------|---------|
| `fix:` | 修复 bug | patch (0.0.Z) |
| `feat:` | 新功能 | patch (0.0.Z) |
| `docs:` | 文档变更 | patch (0.0.Z) |
| `chore:` | 构建/工具/依赖 | patch (0.0.Z) |
| `ci:` | CI 配置 | patch (0.0.Z) |
| `refactor:` | 重构（不改行为） | patch (0.0.Z) |
| `test:` | 测试 | patch (0.0.Z) |
| `release:` | 正式发布稳定版本 | minor (0.Y.0) |
| `BREAKING CHANGE:` | 彻底重写/换架构/换语言 | major (X.0.0) |

### 规则

- commit message 第一行必须以上述 type 开头
- BREAKING CHANGE 可以作为 type 前缀，也可以写在 commit body 中
- 不添加 `Co-Authored-By`、`Signed-off-by` 或任何 AI 署名
- 不修改 git author 或 committer 配置

## 主分支保护

**main 分支上的每个 commit 都必须是立即可用的完整版本。** 不允许存在实现了一半的功能、编译不通过、测试不过的 commit。

- 如果一个功能需要分步实现，必须在专门的开发分支上完成所有步骤后，再合并到 main
- 合并前确保 `make check` 在 kerrigan-storage 和 kerrigan-cli 上都通过
- 不允许向 main 直接 push 半成品——宁可多开一个分支，也不要让 main 处于不可用状态

### 示例

```
feat: add guide file deployment to init

fix: resolve cargo fmt formatting in tests

docs: update README for monorepo structure

BREAKING CHANGE: rewrite storage engine in Go
```

## 项目结构

Monorepo，两个 Rust crate：

- `kerrigan-storage/` — DAG 引擎 + 存储管理
- `kerrigan-cli/` — 编排运行器（依赖 kerrigan-storage via path）
- `docs/` — 设计文档和调研

## 构建

```bash
cd kerrigan-storage && make check
cd kerrigan-cli && make check
```
