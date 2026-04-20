# Markdown 模板(主题详细文档 & 主报告)

本文件是 `SKILL.md` Phase 7.2 的**复现模板**,仅补充具体格式示例。章节的硬性要求、字段清单、长度约束以 `SKILL.md` Phase 7.2 为准 —— 本文件不定义新规则。

**何时读本文**:

- 即将动手撰写 `./topics/{X}.md`(Stage 3 的主题详细文档)
- 即将动手撰写 `./main_report.md`(Stage 4a 的主报告)
- 想快速复用或对比参考一份可运行的 markdown 骨架

本文件**不含**执行逻辑,只含模板。读完模板后要回 `SKILL.md` Phase 7.2 核对章节顺序与约束是否都满足。

---

## 模板 1:主题详细文档 (`./topics/{X}.md`)

每个 `topic-X-investigation` 节点产出,**完整、详尽**。章节选择按主题命中的 Types 和 hands-on 标签决定(见 `SKILL.md` Phase 7.2.a 表),未命中对应类型的章节省略。

```markdown
# {主题名称}

**来源**: 群 A (2026-04-18 14:30–15:10), 群 B (2026-04-18 15:05–15:45)
**Types**: tool-or-code + event
**hands-on**: hands-on-available

## 讨论还原
(2-4 段,提炼讨论脉络,不是 message 级堆砌)

## 聊天原图
![chat-img-1](./figs/topic1-chat-a.jpg)
![chat-img-2](./figs/topic1-chat-b.jpg)

## 实验环境
- Docker 镜像: ubuntu:24.04
- 硬件: Apple M2 Pro, 16GB RAM
- 软件版本: redis 8.0.0, wrk 4.2.0
- 资源限制: `docker run --cpus 4 --memory 8g`

## 实验步骤
1. 拉镜像: `docker pull redis:8.0.0`
2. 启动容器: `docker run -d ...`
3. 压测: `wrk -t4 -c100 -d30s http://...`
   (关键参数解释)
...

## 性能实测
![perf-chart](./figs/topic1-perf.png)

| 工具 | QPS | P99 延迟 | 内存 |
|------|-----|---------|------|
| Redis 8 | 120k | 3.2ms | 350MB |
| KeyDB 6 | 95k | 4.1ms | 420MB |

完整日志 ≥ 2000 字 → [主题 1 · 实验日志全文 →](feishu_url_grandchild)

## 多源交叉验证(命中 event 时)
...

## 原文索引
- msg_xxx (群 A, 14:32, @张三): "......"
- msg_yyy (群 B, 15:08, @李四): "......"
```

---

## 模板 2:主报告 (`./main_report.md`)

Stage 4a 产出,**简洁、指路型**,每主题章节 ≤ 10 行(不含 1 张可选关键图)。

```markdown
# 群聊调研总览 — 2026-04-18

## 概览
- 覆盖群: 群 A (oc_xxx), 群 B (oc_yyy), ...
- 时间窗: 2026-04-18 09:00 – 18:00
- 提取主题: N 个
- Types 分布: tool-or-code X / event Y / opinion Z / incident W / team-ops V / social U
- hands-on: N 个主题 hands-on-available, M 个 read-only
- 上一期: [2026-04-17 调研](https://xxx.feishu.cn/docx/<prev_doc_id>)

## 本期通知与安排
- 2026-04-17 · 下周公司内 beta · 内部群组 / @xxx · 影响全体
- 2026-04-18 · 下周整顿非本 CLI 项目 · 内部群组 / @xxx · 产品线收敛

# {主题 1}

**来源**: 群 A (14:30–15:10), 群 B (15:05–15:45)
**Types**: tool-or-code + event
**hands-on**: hands-on-available
**核心结论**: 一句话结论(如 "Redis 8 在本机压测下对 KeyDB 有 ~25% QPS 优势")

![key-chart](./figs/topic1-perf.png)

→ **[详见主题 1 详细文档](https://xxx.feishu.cn/docx/<topic1_doc_id>)**

# {主题 2}
...
→ **[详见主题 2 详细文档](https://xxx.feishu.cn/docx/<topic2_doc_id>)**

# 聊天参考(未深度调研)
- 主题 X (群 B): 原因 — 数据不足
- 主题 Y (群 A): 原因 — 无沙盒环境
```

---

## 使用提醒

- 两份模板只是最常见的**章节组合**。命中的 Types 不同(tool-or-code / event / opinion / incident / team-ops / social 及其组合)时,照 `SKILL.md` Phase 7.2.a 的章节表增减。没有 `verify-*` 产出的主题不要硬塞"实验环境/步骤"空章节
- 图片路径必须是工作目录下**相对路径**,PNG/JPG 放 `./figs/`
- 所有 `[text](https://xxx.feishu.cn/docx/...)` 必须保留为 inline link,Stage 4b 的流水线会渲染成可点击
- 写完 md 后,按 `SKILL.md` Phase 7.3 的流水线转飞书文档,不要自己另写转换逻辑
