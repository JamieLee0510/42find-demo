---
type: plan
number: "001"
date: 2026-09-06
title: 字符归一第一刀的四个技术选型
tags: [normalize, cjk, variant-table, license, offset]
status: 三条已落地并跑通固定语料；数据源那条已定未落地
related:
  - research/001_variant-table-source
  - experiments/exp001-rg-baseline-cjk
---

# 字符归一第一刀的四个技术选型

> 本项目 `docs/plan/` 的定位是「**哪一处该我定、我当时定了什么、依据是什么**」。
> 本文只写**为什么这样设计**；怎么跑、跑出什么数，见 `state/board.md` 与 issue #4。

## 背景动机

意向书的收敛方向是「簡繁互查、全半角归一——`rg` 因字形差异漏掉的那些，它找得到」。
`exp001` 实测坐实了前两条、推翻了「词在句中」那条。此后 `src/` 仍是两个空壳 crate，
验证闭环缺**固定语料**与**黄金查询集**。issue #4 把这一刀切下去时，撞上四个必须先定的岔路口。

**为什么四个一起写**：它们是同一个子系统（`find42-core` 的归一层）的一条决策链，
后一个的前提是前一个。拆成四份，回看时反而串不起来。

---

## 决策一 · 归一形态：查询扩展，不是折叠

**选**：不动语料，把查询词**逐字展开成等价写法**再扫原文。
**弃**：把语料与查询同折到一个规范形再比对（业界通行做法）。

| | 查询扩展（选） | 折叠（弃） |
|---|---|---|
| 命中的行列 | **就是原文的行列**，无需换算 | 要另维护偏移映射表 |
| 精确率 | 展开集可控，非对称（见决策二） | 折叠天然对称，`發` 与 `髮` 同归 `发` 后互相可查 |
| 上索引之后 | **一个查询词要展成多 term 做 OR** | 一个 term，更便宜 |
| 谁在这么做 | 查遍本地检索工具，**没有一家** | Lucene · Meilisearch · 各家 |

**理由**：这一刀**还没有索引**，所以折叠的成本优势现在拿不到，而它的精确率代价立刻就要付。

**⚠️ 一句自我订正。** 拍板当时我的理由之一是「折叠路线天生丢偏移」。
**那句话是错的**——读了 charabia 才知道它用 `char_map` 解决了：
`resources/charabia/charabia/src/token.rs:57-59` 定义「原文每字符字节数 → 归一后字节数」，
`normalizer/mod.rs:198-210` 建立、`182-196` 逐层叠加、`token.rs:139-152` 换算回原文。
而且 `normalizer/mod.rs:85` 是 `create_char_map: false`——**它们自己也认为这笔钱按需才付**。

订正后的理由只剩一条，但它够硬：**非对称精确率**（决策二）。
折叠做不到非对称，因为归到同一个 canonical 之后，信息就没了。

**欠着的账**：上索引时，多 term OR 的成本要还。已记进 `state/board.md` 的拍板点 3。

---

## 决策二 · 变体表：「规范形 → 变体集」，不是等价类

**选**：非对称映射——`expand(规范形)` = 全部变体；`expand(变体)` = 只回连规范形。
**弃**：等价类（同类字符互相可查）。

**理由**：**等价关系天然对称且传递。** 写下 `发≡發` 与 `发≡髮`，就隐含了 `發≡髮`——
于是查「發」会命中「髮」，**靠查询扩展避开的假阳性从后门溜回来**。
而实际语感是非对称的：简体「发」本就同时对应「發」与「髮」；繁体用户查「發」绝不期待命中「髮」。

**落地**：`src/42find-core/src/variants.rs` 存 `(规范形, &[变体])`；
展开规则 `src/42find-core/src/expand.rs`；两条钉子测试在 `src/42find-core/src/lib.rs`
（`繁体变体不得命中兄弟变体`）。

**旁证（读源码得来）**：charabia 也拒绝传递闭包，并用测试钉住——
`resources/charabia/irg-kvariants/src/lib.rs:126-142` 的 `test_no_loop` 断言
destination 不许再有 destination。同一个判断的两种表述。

---

## 决策三 · 数据源：Unicode Unihan 一手，不是 `irg-kvariants`

**选**：构建期从 `kTraditionalVariant` + `kSimplifiedVariant` 生成静态表。
**弃**：`irg-kvariants`（Meilisearch 的 charabia 在用的那份）。

**理由一 · 授权链断了。** `irg-kvariants` crate 元数据写 MIT，但
`resources/charabia/irg-kvariants/` 目录**没有 LICENSE 文件**，README 说数据来自
`hfhchan/irg` 的 `kVariants.md`，而那个仓**全仓 `find` 无 LICENSE / COPYING / NOTICE**。
意向书要「一个能装到别人机器上的二进制」——**授权链断了就不能分发**。
Unihan 是 Unicode License V3，允许 use/copy/modify/distribute/sell，声明放随附文档即可。

**理由二 · Unihan 原生就编码了决策二的非对称。**

```
U+53D1 发   kTraditionalVariant   U+767C U+9AEE     ← 发 → {發, 髮}
U+767C 發   kSimplifiedVariant    U+53D1            ← 發 → 发（不含 髮）
U+9AEE 髮   kSimplifiedVariant    U+53D1            ← 髮 → 发（不含 發）
```

与 `variants.rs` 里靠语感手推的规则**逐字吻合**。简→繁天然一对多、繁→简天然多对一，
Unicode 早把这层不对称拆成了两个字段。**不需要反转，不需要分组启发式。**

**理由三 · 落地实测的两条，网页视图给不出**：
`hfhchan/irg` **2023-05-26 之后没动过**；Unihan 两文件**单向不闭合 0 处**（自洽，
交叉校验没有可校验的东西）。

**同时排除**：`zhconv`（GPL-2.0，会传染整个 MIT 二进制）· `opencc-rust`（C 绑定，
要用户先装 libopencc，直接毁掉「一个二进制」这个前提）。

**规模**：参与展开 12,972 字；一对多 489 例（466 字 2 变体 / 17 字 3 / 6 字 4）。
**已知陷阱（exp002 实测后加重）**：两文件各有 431 条自指条目
（[unihan-database#408](https://github.com/unicode-org/unihan-database/issues/408)）。
我原先写「去重即可，不致错」——**低估了**。剥掉自指后 key 仍在，
于是一个字可以同时是两个文件的 key（如 `裡`），`if/elif` 会被前者短路、
**静默吃掉反向映射**。`expand.rs` 现在也是 `if/else if`，手写小表下不暴露，
**换 Unihan 的那一刻就会现形**——决策三落地时必须一并改成并集。

---

## 决策四 · 全半角：整块偏移规则，不是查表

**选**：`U+FF01..=U+FF5E` 与 ASCII 之间固定 `0xFEE0` 偏移，另加 `U+3000 ↔ U+0020`。
**弃**：把全半角对照也写进变体表。

**理由**：六行代码全覆盖该区块，比手写表更完整且同样零依赖。
落地在 `src/42find-core/src/expand.rs` 的 `expand_char`。
**半角片假名 `U+FF61..=U+FF9F` 不做**——日文假名已在 issue #4 划为范围外。

---

## 数据模型

```
VARIANTS: &[(char, &[char])]          // 规范形 → 变体集（决策二、三）
Expansion { classes: Vec<Vec<char>> } // 每个位置一组可接受字符（决策一）
Match { line, col, text }             // col 是 1-based 字节列，与 rg --column 同单位
```

`col` 用字节列而非字符列，是为了**跟 rg 基线对得上**——单位不一致，召回率就是假的。

---

## 相关文件

| 文件 | 装什么 |
|---|---|
| `src/42find-core/src/variants.rs` | 决策二、三的数据 |
| `src/42find-core/src/expand.rs` | 决策一、二、四的规则 |
| `src/42find-core/src/search.rs` | 扫描与偏移 |
| `src/42find-core/src/lib.rs` | 8 个单测，含两条非对称钉子 |
| `vault/truth/corpus/` · `vault/truth/queries.tsv` | 尺子（标准归人） |
| `scripts/bench.sh` | 召回 / 精确 / 延迟 |

## 相关变更

`b0d9359` 固定语料 ｜ `dd6584a` 黄金查询集 + rg 基线 ｜ `fdd07cd` 归一层
｜ `66e2412` `c5b7d5a` `ba12776` research/001

## 仍待拍板（AI 不代填）

1. 哪几个 Unihan 字段进。**原建议「`kSemanticVariant` 默认不进（最伤精确率）」已被 exp002 推翻**
   ——实测加进三个附加字段净赚一条、两条钉子都守住、展开集只从 2.43 涨到 2.76 字。
   **改建议：`kSimplified` + `kTraditional` + `kSemantic` + `kSpecializedSemantic` + `kZ` 全进（一跳）。**
   真正要你拍的是另一件：**要不要开两跳**——两跳能把词级从 19/20 做到 20/20（多修好「乾淨」），
   代价是**打破非对称**（`expand(發)` 会变成 `發发髮`）。见 `experiments/exp002-unihan-coverage/`。
2. 上索引时多 term OR 那笔账，现在设计还是以后再说（建议以后，但**要写进意向书真难题①**）。
3. 意向书判据要不要补**精确率**成第三个数（建议补：这条路唯一不可替代的好处就是它）。
4. 语料与黄金查询集的标准归人，AI 出的是初稿，等过目。

## 范围外

索引（真难题①）· 词组级歧义（干/幹·乾）· 地区词（软件/軟體，那是翻译不是字形）·
日韩汉字与假名归一 · GUI / 联网 / 语义检索（意向书「不做什么」）。
