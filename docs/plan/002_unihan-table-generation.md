---
type: plan
number: "002"
date: 2026-09-06
title: 把手写小表换成 Unihan 生成表
tags: [normalize, cjk, variant-table, codegen, license]
status: draft（已开 issue #5 发车）
related:
  - plan/001_normalization-design-decisions
  - research/001_variant-table-source
  - experiments/exp002-unihan-coverage
---

# 把手写小表换成 Unihan 生成表

> `plan/001` 是**为什么这样设计**（决策三已定），这一份是**将要怎么做**。
> 动手前写，落地后改 `status`。

## Context

`plan/001` 决策三已定：变体数据取 Unicode Unihan 一手。但至今**未落地**——
`src/42find-core/src/variants.rs:13-18` 仍是 **6 条**手写映射，只够 `vault/truth/queries.tsv` 那 15 条查询。
真实语料一进来，表外字就是绝大多数，**行为退回 rg 的 50%**。

`exp002` 又给这次改造挂上一个**必现的 bug**：`expand.rs:67-71` 是 `if/else if`，
而 Unihan 里一个字可以同时是两个文件的 key（如 `裡`），**换表那一刻反向映射会被静默吃掉**。
两件事必须一起改，分开做会先引入一个查不出来的错。

## 现状（已核实）

| 事实 | 出处 |
|---|---|
| 手写表 6 条 | `src/42find-core/src/variants.rs:13-18` |
| 查表是**线性** `.iter().find()` | `src/42find-core/src/variants.rs` 的 `variants_of` / `canonical_of` |
| 展开是 `if/else if`，换表后必错 | `src/42find-core/src/expand.rs:67-71` |
| `find42-core` **零第三方依赖** | `src/42find-core/Cargo.toml` 的 `[dependencies]` 为空 |
| 黄金查询集 15 条 · 语料 7 份 | `vault/truth/queries.tsv` · `vault/truth/corpus/` |
| **`resources/**` 不进仓** | `.gitignore:12` |
| Unihan 数据在本地 | `resources/unihan-database/`（Unicode License V3，@2026-07-24） |

### 这条约束决定了整个方案形状

**`build.rs` 不能读 `resources/`。** 那个目录不进仓——别人 clone 完仓库没跑 `clone.sh`，
`cargo build` 就炸，CI 一样炸。所以**不能**走「构建期从原始数据生成」。

| 方案 | 结果 |
|---|---|
| A. `build.rs` 读 `resources/` | ❌ 仓库不自洽，CI 必炸 |
| B. **一次性脚本生成 `.rs`，生成物入库** | ✅ 选它。构建不依赖任何仓外文件，`core` 保持零依赖 |
| C. 把 Unihan 原始 `.txt` 拷进仓 + `build.rs` 读 | ⚠️ 可行（约 1 MB，未触铁律 8 的 10 MB 线；Unicode License 允许再分发），但**多引入一个 csv 解析依赖**，违反命门 2 的「零依赖」初衷 |

## 目标

1. 变体表覆盖 Unihan 的 **12,972 字**（`exp002` 实测规模），行为不退。
2. `expand.rs` 的短路 bug 在**换表之前**修掉。
3. `find42-core` 仍然**零第三方依赖**，`cargo build` 不依赖任何仓外文件。
4. `bash scripts/bench.sh` 的三个数**都不退**，且 `exp002` 那 20 个词进黄金查询集后全绿。

## 执行步骤

### 1 · 先修 `expand.rs` 的短路（独立一个 PR，先于换表）

`src/42find-core/src/expand.rs:67-71` 的 `if let ... else if let ...` 改成**并集**：
两个方向都 `extend`，不用 `else`。

**为什么先做**：手写小表下这个 bug **不暴露**（一个字要么是 key、要么在 value 里，
从不同时是两者），所以这一步**行为零变化、三个数零变化**——
它是一次纯粹的「把地雷拆掉」，混在换表里做就分不清是谁的错了。
并集**不破坏非对称**：`expand(發) = {發} ∪ 变体集{} ∪ 规范形{发}`，仍不含 `髮`
（`exp002` 已实测，`src/42find-core/src/lib.rs` 的两条钉子测试守着）。

### 2 · 生成脚本 `scripts/gen-variants.py`

- 读 `resources/unihan-database/` 的 **五个字段**（`exp002` 模式 B）：
  `kSimplifiedVariant` · `kTraditionalVariant` · `kSemanticVariant` ·
  `kSpecializedSemanticVariant` · `kZVariant`。**不开两跳**（待拍板，见「风险」）。
- **剥掉 431 条自指**（[unihan-database#408](https://github.com/unicode-org/unihan-database/issues/408)）。
- 输出 `src/42find-core/src/variants_generated.rs`：**按 char 排序**的
  `&[(char, &[char])]` 两张表（规范形→变体集 · 变体→规范形）。
- 文件头写死三样：**机器生成勿手改** · 生成脚本路径 · **数据来源与 Unicode License V3 版权声明**。
- 复用现成的：`exp002` 的 `docs/experiments/exp002-unihan-coverage/probe.py` 里
  `load()` 与 `expand_char()` 已经是验证过的解析与规则，**照搬逻辑，别重写**。

### 3 · `variants.rs` 改为查生成表，并把线性查找换成二分

现在 `variants_of` / `canonical_of` 是 `.iter().find()`。6 条时无所谓，
**12,972 条时每个查询字符都要扫一遍全表**。表已排序，改 `binary_search_by_key`。

### 4 · 语料与黄金查询集跟着扩

`exp002` 的 20 个查询词与其 ground truth 进 `vault/truth/`，包含**四个已知不能的**
（`資料庫` `網路` `軟體` `發佈`）——**要把「不能」也写进期望值**，
否则日后有人「修好」了它们，反而是把范围外的东西拉了进来。
⚠️ 语料一改，`queries.tsv` 头部的 `corpus-sum` 必须重算（`scripts/bench.sh` 会警告）。

### 5 · 跑三个数

## 验收（端到端怎么测）

```bash
cargo fmt --all -- --check; echo "rc=$?"      # 三道闸门分开判退出码
cargo clippy --workspace --all-targets; echo "rc=$?"
cargo test --workspace; echo "rc=$?"
bash scripts/bench.sh                          # rg 基线 vs 42find，三个数
python3 scripts/gen-variants.py && git diff --exit-code src/42find-core/src/variants_generated.rs
```

| 判据 | 通过线 |
|---|---|
| 三道闸门 | 各自退出码 0（**别用管道判**，见 `state/memory/20260906-管道退出码.md`） |
| 召回 | 扩充后的黄金查询集上**不低于换表前**，且 `exp002` 那 20 个词按其实测表现全对 |
| 精确 | **仍是 100%** —— 两条非对称钉子测试必须绿 |
| 延迟 | 与换表前同量级（二分查找，不该有数量级变化） |
| 生成物可复现 | 重跑脚本后 `git diff` **为空** |
| 二进制自洽 | 在**没有 `resources/`** 的干净 clone 上 `cargo build --release` 成功 |

## 风险与权衡

| 风险 | 应对 |
|---|---|
| **生成的 `.rs` 入库 = 一个不能手改的文件** | 文件头写死警告；验收里加「重跑脚本 `git diff` 为空」这条规则验证 |
| **Unicode License 的声明义务** | 生成文件头 + `README.md` 各带一份版权声明。License 允许放随附文档，但**必须有** |
| **`kSimplified`/`kTraditional` 是 provisional 字段** | 生成脚本把数据的 commit hash 写进文件头，Unicode 改了能追溯 |
| 表变大后编译时间与二进制体积 | 落地时量一次；真涨得离谱再换紧凑编码（但**不能引入依赖**） |
| **两跳开不开，还没拍板** | 本蓝图**按不开做**。`exp002` 的账：开了词级 19/20→20/20，代价是打破非对称（`expand(發)` 变成 `發发髮`）。要开就是另一个 PR，不混在这次 |

## 范围外

两跳传递闭包（待拍板）· 索引（真难题①）· 地区词（`軟體`/`網路`，是翻译不是字形）·
日韩汉字与假名 · 全半角（走整块偏移规则，不经变体表）。

## 相关

**issue**：<https://github.com/JamieLee0510/42find-demo/issues/5>（照本蓝图执行，内部拆 5 个 PR）

`plan/001_normalization-design-decisions` 决策三与三、四
· `research/001_variant-table-source` 授权链与选型
· `experiments/exp002-unihan-coverage` 字段选择与那个短路 bug
