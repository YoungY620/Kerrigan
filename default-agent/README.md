# Kerrigan default-agent 安装指导

本目录是一个自定义的 Kitty Agent 配置包,包含:

| 文件 | 作用 |
|---|---|
| `agent.yaml` | 主 Agent 配置:名称、工具列表、subagent 定义 |
| `sub.yaml` | Subagent 配置:继承主 Agent,排除 agent/todo/mail 工具,适合并行任务 |
| `system.md` | 系统提示词模板(会被 Worker 运行时注入 `${KITTY_*}` / `${KIMI_*}` 变量) |

本文档指导**AI agent 或运维人员**如何把这套配置部署到 Kitty 上。按下面两个场景二选一。

---

## 前置依赖

- Python 3.12+
- 已安装 `uv`(`uv --version` 验证)
- 已安装 `tmux`(进程托管)
- 可访问 `https://pypi.org/simple`
- 飞书应用(已开通 `im:message` / `im:chat` / `docx:document` / `drive:drive` 等权限,记下 `App ID` 和 `App Secret`)
- `~/.kimi/config.toml` 已配好模型 + Provider(让运维提供,或参考 kitty 官方文档)

---

## 必装 Skills(本 Agent 运行依赖)

本 Agent 的 `system.md` 强制要求读 `kerrigan` skill("**Before every new task**, re-read the Kerrigan skill")—— 不装这个 skill,Agent 的任务分解协议就无法工作。

### 需要装的 Skills

| Skill | 位置(本仓库) | 必要性 | 说明 |
|---|---|---|---|
| `kerrigan` | `skills/kerrigan/` | **必须** | DAG 任务分解方法论,`system.md` 强制引用 |
| `feishu-research` | `skills/feishu-research/` | **强烈推荐** | 飞书多群聊调研任务,典型主场景 |

Worker 内置的 Skills(无需额外安装):`kitty-feishu` / `kitty-manager` / `kitty-help`。

### 部署到 Worker 可访问的位置

`--skills` 参数指定的目录里,**每个子目录是一个 skill**(目录里有 `SKILL.md`)。惯例位置:`~/.kerrigan/skills/`。

**方式 A — 软链接本仓库的 skills(开发机推荐)**:

```bash
mkdir -p ~/.kerrigan
ln -sfn /path/to/kerrigan/skills ~/.kerrigan/skills
```

仓库更新后 skill 即时生效,不用再拷贝。

**方式 B — 拷贝本仓库的 skills**:

```bash
mkdir -p ~/.kerrigan/skills
cp -r /path/to/kerrigan/skills/kerrigan         ~/.kerrigan/skills/
cp -r /path/to/kerrigan/skills/feishu-research  ~/.kerrigan/skills/
# 可选
cp -r /path/to/kerrigan/skills/changelog        ~/.kerrigan/skills/
```

仓库更新后需要手动再 `cp -r` 一次。

**方式 C — 用 `kerrigan init`(仅装 kerrigan skill,不含 feishu-research)**:

适合机器上已经装了 `kerrigan` CLI 二进制的情况:

```bash
kerrigan init
# 会把仓库内打包进二进制的 skills/kerrigan/ 写到 ~/.kerrigan/skills/kerrigan/
```

`feishu-research` 和 `changelog` **不**被 `kerrigan init` 安装(它们是 repo-only skill),仍需走 A 或 B。

### 验证

```bash
ls ~/.kerrigan/skills/
# 至少应看到: kerrigan  feishu-research
ls ~/.kerrigan/skills/kerrigan/SKILL.md  ~/.kerrigan/skills/feishu-research/SKILL.md
# 两个文件都能 stat
```

Worker 启动后,Agent 会通过 `${KIMI_SKILLS}` 变量在系统提示词里看到已加载的 skill 清单。Worker 日志里也会打印 `Loaded skills: [kerrigan, feishu-research, kitty-feishu, ...]`。

---

## 公共步骤:部署 default-agent 到 Kitty 约定位置

Worker 从 `--agent <path>` 读取 Agent 配置。惯例位置是 `~/.kitty/agents/<name>/`。

把本目录复制(或软链接)过去:

```bash
# 方式 A: 拷贝一份(后续改动需要同步)
mkdir -p ~/.kitty/agents
cp -r /path/to/kerrigan/default-agent ~/.kitty/agents/default-agent

# 方式 B: 软链接(改动实时生效,推荐开发场景用)
mkdir -p ~/.kitty/agents
ln -sfn /path/to/kerrigan/default-agent ~/.kitty/agents/default-agent
```

验证:

```bash
ls ~/.kitty/agents/default-agent/
# 应看到: agent.yaml  sub.yaml  system.md
```

> 也可以不移动,直接在 Worker 启动命令里用 `--agent /path/to/kerrigan/default-agent` 指绝对路径。但惯例位置能让多个 Worker / 多次重启共享同一份配置。

---

## 场景 A:从零启动 Server + 自定义 Worker

### A.1 启动 Server

```bash
tmux new-session -d -s kitty-server
tmux send-keys -t kitty-server:0 "
KITTY_FEISHU_APP_ID=cli_xxxxxxxxxx \
KITTY_FEISHU_APP_SECRET=your_app_secret \
KITTY_TRANSFER_MODE=base64 \
uvx --extra-index-url https://pypi.org/simple \
  kitty-server@latest \
  2>&1 | tee -a ~/.kitty/logs/server.log
" Enter
```

启动后在日志里找 Dashboard 地址(含 token),记录备用。Server 监听 `0.0.0.0:8765`(默认)。

在飞书开放平台"事件订阅"填:`https://<服务器公网IP>:8765/webhook/event`

### A.2 启动自定义 Worker(含本 Agent 配置)

先完成上面的"公共步骤"把 `default-agent/` 放到 `~/.kitty/agents/default-agent/`。

```bash
tmux new-window -t kitty-server: -n kitty-worker
tmux send-keys -t kitty-server:kitty-worker "
uvx --extra-index-url https://pypi.org/simple \
  --from kitty-worker@latest \
  kitty-worker-auto \
  --server localhost:8765 \
  --chat-in '<CHAT_IDS>' \
  --agent ~/.kitty/agents/default-agent \
  --skills ~/.kerrigan/skills \
  --config ~/.kimi/config.toml \
  2>&1 | tee -a ~/.kitty/logs/worker.log
" Enter
```

参数见下面 [§参数说明](#参数说明)。

---

## 场景 B:Server 已存在,只启动自定义 Worker

适用:已有别的团队 / 别的用户运行着 Server,你只需要用自己的 Agent 配置跑 Worker 接管自己负责的聊天。

### B.1 拿到 Server 地址

问运维(或 Dashboard 上看):`host:port`,例如 `10.x.x.x:8765`。

### B.2 完成"公共步骤"部署 `default-agent/`

见上面公共步骤,把本目录放到 `~/.kitty/agents/default-agent/`。

### B.3 启动 Worker

```bash
tmux new-session -d -s kitty-worker
tmux send-keys -t kitty-worker "
uvx --extra-index-url https://pypi.org/simple \
  --from kitty-worker@latest \
  kitty-worker-auto \
  --server <SERVER_HOST>:<SERVER_PORT> \
  --chat-in '<CHAT_IDS>' \
  --agent ~/.kitty/agents/default-agent \
  --skills ~/.kerrigan/skills \
  --config ~/.kimi/config.toml \
  2>&1 | tee -a ~/.kitty/logs/worker.log
" Enter
```

Worker 连上后,日志会出现 `Connected to ws://<server>:<port>/ws/worker?ids=...`。

---

## 参数说明

| 参数 | 说明 | 示例 |
|---|---|---|
| `--server` | Server 的 `host:port` | `localhost:8765` 或 `10.x.x.x:8765` |
| `--chat-in` | 本 Worker 监听的飞书 ID 列表(**必填**),逗号分隔;`ou_xxx` 是用户 open_id(p2p),`oc_xxx` 是群聊 chat_id | `ou_183d6bee7ad...,oc_48b5af2a2fc...` |
| `--agent` | 本 Agent 目录路径(即放置 `default-agent/` 的位置) | `~/.kitty/agents/default-agent` |
| `--skills` | Skills 目录路径(包含 `kerrigan/` / `feishu-research/` 等 Skill 子目录) | `~/.kerrigan/skills` |
| `--config` | kimi-cli 配置文件路径(模型 + Provider 定义) | `~/.kimi/config.toml` |

### 关于 `--chat-in`(重要)

**一个 Kitty Server 下的同一个 workspace 只能运行一个 Worker**。不要用两个共享 workspace 的 Worker 分别处理不同 chat——否则后启动的 Worker 会覆盖前者的 Session 映射,触发 `403: session not owned by this worker`。

正确做法:把你要处理的**所有**飞书 ID(用户 + 群)合并到这一个 Worker 的 `--chat-in` 里。例如:

```bash
--chat-in 'ou_183d6bee7adce2fa4feaa822db3ff407,oc_48b5af2a2fc2bf2f04c56ae3cab8a88f,oc_3c4d37599c1f04315a5544245346b653'
```

### 怎么拿 `ou_xxx` / `oc_xxx`

**首选方式 —— 从飞书"设置"页直接复制**:

1. 打开目标**群聊**或目标**私信**的"设置"页(右上角齿轮图标)
2. 拉到**页面最底部**,有一串以 `oc_` 开头(群)或 `ou_` 开头(私信)的 ID
3. 直接复制这串 ID 作为 `--chat-in` 的值

**兜底方式(首选找不到时)**:

- **`oc_xxx`(群)**:把机器人拉进群并 `@` 它发一条消息,Server 日志会打出 `chat_id: oc_xxx`
- **`ou_xxx`(私信)**:让目标用户给机器人发一条消息,Dashboard 或 Server 日志里看 `sub_id`

---

## 验证 Worker 就绪

1. **日志层**:`~/.kitty/logs/worker.log` 出现
   ```
   Connected to ws://<server>/ws/worker?ids=ou_xxx,oc_xxx
   Applied S3 storage path from server: ...
   ```
2. **飞书层**:给机器人发"你好",应在 2-5 秒内收到回复

未出现回复时排查顺序:
- 检查机器人是否在对应群/是否被加为好友
- `ou_xxx` / `oc_xxx` 是否写对
- Server 的飞书 Webhook 是否配对且可达
- 看 `worker.log` 末尾有没有 exception

---

## 修改本 Agent 配置后如何生效

- 改 `system.md` 的系统提示词 → **下一轮对话自动生效**(Worker 在每轮开始时重新渲染模板)
- 改 `agent.yaml` 工具列表 / subagent 定义 → **需要重启 Worker**
- 改 `sub.yaml` → 同上
- 改 `~/.kimi/config.toml` 模型配置 → **需要重启 Worker**

重启命令(kitty-worker-auto 会自动重启):

```bash
tmux send-keys -t kitty-worker 'C-c' && sleep 2
# 再按上面的启动命令重新 send-keys
```

---

## 参考

- Worker / Server 源码 + 完整参数:`kitty-worker --help` / `kitty-server --help`
- Kitty 框架仓库:`https://github.com/YoungY620/Kerrigan`
