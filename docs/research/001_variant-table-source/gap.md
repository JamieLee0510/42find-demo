# 两类发现 · 簡繁与异体的映射数据从哪来

> 2026-09-06。**从本地检索工具出发**取的材，MIT 优先。
> 分析的成果不是综述，是下面这两张表。

## 一、它们做到的

### 本地检索工具在这件事上的现成做法

| # | 命门 | 状态 | 差距是什么 | 修起来要什么 | 分级 |
|---|---|---|---|---|---|
| 1 | **MIT 优先，且覆盖数据不只是代码** | **部分** | 本地检索这条线上 MIT 是主流（见下表），但**最合手的那份数据授权不明**——`irg-kvariants` crate 元数据写 MIT，其 `irg-kvariants/` 目录**没有 LICENSE 文件**，README 只说数据来自 [hfhchan/irg 的 kVariants.md](https://github.com/hfhchan/irg/blob/master/kVariants.md)，而**那个仓看不到任何 LICENSE** | 换 Unihan 一手（Unicode License V3，文档里带一份版权声明即可） | **P0** |
| 2 | **纯 Rust、构建期内嵌、零 C 依赖** | **已实现** | 无。`irg-kvariants` 用 `include_bytes!(concat!(env!("OUT_DIR"), "/kVariants.min.csv"))` 构建期嵌入，纯 Rust | — | — |
| 3 | **能抽出「字符 → 变体集」的非对称关系** | **已实现（且超出预期）** | 无。见下 | — | — |

### 命门 3 的细节：预判错了，实际更好

`irg-kvariants` 的数据结构（[docs.rs 源码](https://docs.rs/irg-kvariants/0.1.1/src/irg_kvariants/lib.rs.html)）：

```rust
pub enum KVariantClass { Wrong, SementicVariant, Simplified, Old, Equal }
pub struct KVariant { source_ideograph: char, classification: KVariantClass, destination_ideograph: char }
pub static KVARIANTS: Lazy<HashMap<char, KVariant>>
```

- **字符级**：`HashMap<char, KVariant>`，一个字一条，不是词组规则。
- **多对一**：多个 `source_ideograph` 指向同一个 `destination_ideograph`（源码有 debug assertion 禁止一源多的）。
  → **按 destination 分组就反转成「规范形 → 变体集」**，正是我们要的那张关系。
- **自带分类**：`Simplified`（简化）· `Old`（旧字形）· `Wrong`（讹字）· `SementicVariant`（语义变体）· `Equal`。
  → **能控制哪几类参与展开**——这是我起草命门 3 时没想到的一层，直接关系到精确率。

**我的预判在这一条上被推翻**：我以为「现成的全是转换器，给不出等价关系」。
`convert()` 那一层确实是转换器，但**它下面的数据层是关系表，而且是公开的**。

### 许可横向表（`cargo info` 本机实测，2026-09-06）

| crate | 版本 | 许可 | 角色 |
|---|---|---|---|
| `tantivy` | 0.26.1 | **MIT** | Rust 的 Lucene——上索引时的候选 |
| `charabia` | 0.10.0 | **MIT** | Meilisearch 的分词/归一层，**中文走 kvariant 归一** |
| `irg-kvariants` | 0.1.1 | **MIT**（元数据；**数据层存疑，见 P0**） | 字符级变体关系表 |
| `lindera` | 6.0.0 | **MIT** | 形态分析（日文为主） |
| `jieba-rs` | 0.10.3 | **MIT** | 中文分词 |
| `cang-jie` / `tantivy-jieba` | 0.20.0 | **MIT** | tantivy 的中文分词器 |
| `ripgrep` | 15.2.0 | Unlicense **OR MIT** | 基线本身 |
| `character_converter` | 2.1.5 | **MIT** | 簡繁转换器 |
| `unicode-normalization` | 0.1.25 | MIT OR Apache-2.0 | NFKC |
| ~~`zhconv`~~ | 0.4.1 | ~~GPL-2.0-or-later~~ | **排除**：会传染整个二进制 |
| ~~`opencc-rust`~~ | 2.0.1 | Apache-2.0 但 **C 绑定** | **排除**：要 libopencc，毁掉「一个二进制」 |

> **「MIT 优先」这条约束几乎没花代价**——本地检索这条线上 MIT 就是主流。
> 真正被它筛掉的只有 `zhconv`（GPL），而那个 §2 就已经排除了。

### 别人怎么解这件事（做法，不是数据）

| 谁 | 怎么做 | 出处 |
|---|---|---|
| **Lucene** | `ICUTransformFilter(Transliterator.getInstance("Traditional-Simplified"))` 做簡繁；`CJKWidthFilter` 折叠全角 ASCII 与半角片假名 | [Lucene analyzers-icu 概览](https://lucene.apache.org/core/8_11_4/analyzers-icu/overview-summary.html) |
| **Meilisearch（charabia）** | `ChineseNormalizer` 按 `KVARIANTS` 把 Z/简化/语义/旧/讹 变体**归到 canonical**；可选再转拼音 | [charabia](https://github.com/meilisearch/charabia) |
| **tantivy** | **本体不做**中日韩归一，交给第三方分词器（`cang-jie` / `tantivy-jieba` / `lindera`）；[issue #718](https://github.com/quickwit-oss/tantivy/issues/718) 记着 QueryParser 先按英文切、CJK 查询因此对不上 | [tantivy](https://github.com/quickwit-oss/tantivy) |
| **ripgrep** | **不做**。子串匹配、不分词、不归一（exp001 已实测坐实） | `docs/experiments/exp001-rg-baseline-cjk/` |

⚠️ **三家做到的都是「折叠」，没有一家做「查询扩展」。** 这是本次分析最该记住的一条，见 `decision.md` 的反对依据。

## 二、它们没做的

| 找过什么 | 结论 | 是哪一种 |
|---|---|---|
| **非对称展开**（查「發」不命中「髮」） | **无人涉足**——Lucene / charabia 都折叠到 canonical，折叠天然对称，`發` 与 `髮` 同归 `发` 之后就互相可查 | **没顾上做**：他们的场景是「找到就行」，精确率损失可接受；42find 承诺的是「所有出现处」，能不能承受这个损失是**产品取舍不是技术天堑** |
| **保住原文偏移的归一** | **有但不适用**：折叠路线天生丢偏移，各家靠索引存 term 位置绕开——而 42find 这一刀**还没有索引** | **做不到**（在折叠路线里）；换成查询扩展就自然没有这个问题 |
| **tantivy 的 CJK 查询一致性** | 有但不适用：[issue #718](https://github.com/quickwit-oss/tantivy/issues/718) 长期开着 | **做不到**——QueryParser 与字段分词器不同步是架构层的，提醒我们上索引时会撞上同一堵墙 |

### 该不该自己跑一个

| 判据 | 结论 |
|---|---|
| Unihan 的 `kSimplifiedVariant`/`kTraditionalVariant` 覆盖多少字、一对多有几例 | **可以查**，去 [UAX #38](https://www.unicode.org/reports/tr38/) 核 |
| **真实语料上，覆盖前 N 个高频字需要多大的表** | **跑它**——`vault/raw/` 还是空的，这个数没人能替我们回答 |
| **哪几类变体参与展开，精确率各掉多少** | **跑它**——`Simplified` / `Old` / `Wrong` / `SementicVariant` 逐类开关，量四次召回与精确。**这正是「输出清楚、不知道拿什么喂」的场合** |

→ 两个都归 `docs/experiments/`，**不在本目录另开一处**。
