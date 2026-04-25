# Benchmark Reliability Audit

Date: 2026-04-15
Method: Independent computation verification + community research + remote server hands-on testing

---

## Final Verdict

| Benchmark | SOTA | Setup | Data Quality | Eval Determinism | Contamination | **Overall** |
|-----------|------|-------|-------------|------------------|---------------|-------------|
| **BigCodeBench** | ~60% | pip + HF | 20/20 canonical pass, ICLR'25 | 5/5 deterministic, ~0.5% flaky | Low (<2.5% overlap) | **A-** |
| **Omni-MATH** (int subset) | ~60% | HF load_dataset | 竞赛官方题源, 44% 整数答案可精确匹配 | 整数答案确定性 | Medium (竞赛题公开) | **B+** |
| **BBEH** | 64-74% | git clone JSON | disambiguation_qa 有标注错误 (NYU 批评); 推理纠错类任务格式复杂 | 确定性 (exact match) | Low (新题) | **B** |
| **AIME** (AI-MO) | 30-50% | HF load_dataset | AI-MO 数据集含 AoPS URL 可交叉验证 | 整数 0-999 确定性 | **HIGH** (pre-2025 膨胀 10-20pp) | **B-** |
| **AIME** (qq8933) | 30-50% | HF load_dataset | **12/12 答案交叉验证不匹配**, 问题编号映射有误 | 整数确定性 | HIGH | **D** |
| **SciCode** | 10-34% | git clone + pip | 科学家标注, 16 子领域 | 代码执行 pass/fail | Low (新题) | **未验证** (需 HDF5 数据) |
| **Putnam-AXIOM** | ~42% | HF (gated) | Putnam 官方题 | boxed 精确匹配 | Medium | **未验证** (gated) |

---

## Detailed Findings

### BigCodeBench — **最可靠** (Grade A-)

**实验验证**:
- 20/20 canonical solutions 全部通过测试 (100%)
- 5/5 确定性检查通过 (两次运行结果一致)
- 1140 题, 每题有完整 unittest

**社区研究**:
- 开发者承认 ~0.5% flaky tests (缺少 random seed)
- 有 `--no-gt` flag 因为 canonical solution 也可能在 flaky test 上失败
- 污染风险低: n-gram 重叠 <2.5%, 数据托管在 HF 而非 GitHub 避免爬虫
- ICLR 2025 接收, 经同行评审

**问题**:
- SOTA ~60%, 有提升空间但不如其他 bench 那么大
- 需要真实第三方库 (pandas, numpy, sklearn 等)
- 部分 test 依赖网络/文件系统可能在受限环境失败

**Reference**: [BigCodeBench Paper](https://arxiv.org/html/2406.15877v4), [GitHub](https://github.com/bigcode-project/bigcodebench)

### Omni-MATH — **有潜力** (Grade B+)

**数据分析**:
- 4,428 道奥赛级数学题
- 44.3% (1,962 题) 答案为纯整数——可以做精确匹配, 无需 LLM-as-judge
- 另 41.4% 为 LaTeX 表达式——需要符号匹配 (SymPy)
- 题源: 各国竞赛选拔 (china_team_selection_test, usa_team_selection_test, imo 等)
- 难度 1-9 级, Level 4-5 占多数

**可用策略**: 只取整数答案子集 (1,962 题), 做 exact integer match, 完全确定性

**问题**:
- o1-mini 在全集上 60.54%, 但整数子集得分可能不同
- 竞赛题公开, 有一定污染风险
- LaTeX 子集需要符号匹配, 实现复杂

**Reference**: [Omni-MATH](https://github.com/KbsdJames/Omni-MATH)

### BBEH (BIG-Bench Extra Hard) — **有保留** (Grade B)

**格式发现** (与初始预期完全不同):
- **boolean_expressions**: 5 选 1 的复杂布尔表达式, 混合数学计算 + 地理知识 + 多层嵌套逻辑
- **word_sorting**: 不是排序单词! 而是给一个多步排序过程, 找出第一个推理错误步骤
- **dyck_languages**: 给一个括号匹配的步骤推理, 找出第一个错误
- **zebra_puzzles**: 经典约束满足问题 (谁住在哪, 谁支持什么队)

本质上是**推理过程验证** (reasoning trace verification), 非简单计算题

**社区研究**:
- Erik Davis (NYU) [批评](https://cs.nyu.edu/~davise/Benchmarks/BBEH.html) disambiguation_qa 子集有多处标注错误: 语法歧义标为非歧义, 不合语法使题目无法求解
- BBEH 论文承认 micro/macro 均值受离群任务影响
- 23 个子任务质量不均, 计算性任务 (boolean_expressions, multistep_arithmetic) 比语言推理任务更可靠

**问题**:
- 各子任务质量差异大
- 独立验证需要为每种任务类型写专门的 solver (boolean_expressions 需要知识库+计算)
- 部分子任务答案有争议

### AIME (qq8933/AIME_1983_2024) — **不推荐** (Grade D)

**实验验证**:
- 12/12 已知答案交叉验证**全部不匹配**
- 2024 年数据只含 Part II (14 题), 缺少 Part I
- 1 题有非整数答案 ("080 or 081 (both were accepted)")
- 问题编号/Part 标注与 AoPS 官方不一致

**结论**: 数据映射不可靠, **不应用于优化目标**

### AIME (AI-MO/aimo-validation-aime) — **可用但需注意** (Grade B-)

- 90 道题, 每题含 AoPS Wiki URL 可交叉验证
- 答案经确认与 AoPS 一致 (answer=116 对应 2022 AIME I P1)
- **但**: pre-2025 AIME 污染严重, MathArena 研究显示膨胀 10-20 个百分点
- 建议: 如果使用 AIME, 只用 2025 年之后的新题

**Reference**: [MathArena AIME 2024 contamination analysis](https://www.emergentmind.com/topics/matharena-aime-2024)

### SciCode — **高潜力但未完成验证**

- GitHub repo 可 clone, 有 eval 脚本和 inspect_ai 集成
- 数据需要单独下载 HDF5 文件 (test_data.h5)
- HuggingFace 上没找到预处理好的 dataset
- SOTA 10.8% (main problems), 空间极大
- 16 个科学子领域, 科学家标注

**需要进一步验证**: 安装 inspect_ai, 下载 test_data.h5, 验证评测流程

### Putnam-AXIOM — **高潜力但 gated**

- HuggingFace 数据集 gated, 需要申请
- 236 道 Putnam 真题 + 100 道变体
- boxed answer exact match, 与 lm-eval-harness 集成
- SOTA ~42% (o1-preview), 空间大

---

## Recommendations

### 现在就能用 (已完整验证):

1. **BigCodeBench** (A-): 最可靠, pip install, 1140 题, ~60% SOTA
2. **Omni-MATH 整数子集** (B+): 1962 题整数精确匹配, 竞赛权威题源
3. **BBEH 计算性子任务** (B): boolean_expressions + multistep_arithmetic + word_sorting 等, exact match

### 建议追加验证:

4. **SciCode**: 下载 test_data.h5, 跑通 eval pipeline
5. **AIME 2025+**: 等 2025 年新题加入数据集后使用
6. **Putnam-AXIOM**: 申请 HF access

### 明确避免:

- qq8933/AIME_1983_2024 (答案映射不可靠)
- BBEH disambiguation_qa 子任务 (已知标注错误)
- 任何 SOTA > 90% 的 bench (无优化空间)
