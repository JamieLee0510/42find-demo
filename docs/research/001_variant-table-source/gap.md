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

### 落地实测（2026-09-06 取材后，`resources/` 三个浅克隆）

**Unihan 原生就编码了我们要的非对称关系**，不需要反转、不需要分组启发式：

```
U+53D1 发   kTraditionalVariant   U+767C U+9AEE     ← 发 → {發, 髮}
U+767C 發   kSimplifiedVariant    U+53D1            ← 發 → 发（不含 髮）
U+9AEE 髮   kSimplifiedVariant    U+53D1            ← 髮 → 发（不含 發）
```

`expand(发)={发,發,髮}` · `expand(發)={發,发}` · `expand(髮)={髮,发}`——
**与 `src/42find-core/src/variants.rs` 里手推的规则逐字吻合。**
简→繁天然一对多、繁→简天然多对一，Unicode 早把这层不对称拆成了两个字段。

| 项 | 实测 | 意味着 |
|---|---|---|
| `kSimplifiedVariant` / `kTraditionalVariant` 条目 | 6929 / 6475 | — |
| `kTraditionalVariant` 一对多 | 466 字 2 变体 · 17 字 3 · 6 字 4 | 发/發/髮 是 **489 例之一**，不是孤例 |
| **单向不闭合**（A 说简化成 B，B 的繁体表却没有 A） | **0 处** | 两文件完全自洽——**交叉校验闭合性没有必要** |
| 自指条目（X 的变体是 X 自己） | 各 **431** 条 | 已知问题 [unihan-database#408](https://github.com/unicode-org/unihan-database/issues/408)，**去重即可，但必须显式处理** |
| 参与展开的字符总数（去重） | **12,972** | 静态表规模完全可接受 |
| `resources/` 三仓合计 | 11.8 MB（`--depth 1`） | 不进仓（`.gitignore: resources/**`） |

**时效判决**：

| 仓 | 最近提交 | 许可 | 判决 |
|---|---|---|---|
| `unicode-org/unihan-database` | **2026-07-24** | **Unicode License V3**（有 LICENSE，Copyright 2021-2026 Unicode Inc.） | **活着，授权干净** |
| `meilisearch/charabia` | 活跃 | 根 **MIT**（Meili SAS） | 代码 MIT，**但数据不是它的** |
| `hfhchan/irg` | **2023-05-26**（三年未动） | **全仓 `find` 无 LICENSE / COPYING / NOTICE** | **又老又没授权 —— 出局** |

`charabia/irg-kvariants/dictionaries/source/kVariants.tsv` 10,629 行，分类分布
`simp 3942 · sem 3546 · = 2129 · wrong! 602 · old 410`，方向是**多对一归到繁体 canonical**——
与我们要的方向相反。它独有的 `old` / `wrong!` 两类，Unihan 有 `kZVariant`(149) /
`kSemanticVariant`(3538) / `kSpoofingVariant` 对应，**且授权干净**。

→ **`irg-kvariants` 从「交叉校验的第二意见」降为「一个可以学的分类思路」。** 数据一行都不用它的。

## 二、它们没做的

| 找过什么 | 结论 | 是哪一种 |
|---|---|---|
| **非对称展开**（查「發」不命中「髮」） | **无人涉足**——Lucene / charabia 都折叠到 canonical，折叠天然对称，`發` 与 `髮` 同归 `发` 之后就互相可查 | **没顾上做**：他们的场景是「找到就行」，精确率损失可接受；42find 承诺的是「所有出现处」，能不能承受这个损失是**产品取舍不是技术天堑** |
| **保住原文偏移的归一** | ~~折叠路线天生丢偏移~~ **这句错了，取材后已订正**：charabia 用 `char_map` 解决了（`charabia/src/token.rs:57-59`），代价是每 token 一个 `Vec<(u8,u8)>`，且**默认关**（`charabia/src/normalizer/mod.rs:85`） | **做得到，只是要掏钱**。查询扩展省掉的是这笔钱，不是解决了一个无解问题——见「三、逐条核查」第 2 节第 2 条 |
| **tantivy 的 CJK 查询一致性** | 有但不适用：[issue #718](https://github.com/quickwit-oss/tantivy/issues/718) 长期开着 | **做不到**——QueryParser 与字段分词器不同步是架构层的，提醒我们上索引时会撞上同一堵墙 |

### 该不该自己跑一个

| 判据 | 结论 |
|---|---|
| Unihan 的 `kSimplifiedVariant`/`kTraditionalVariant` 覆盖多少字、一对多有几例 | **可以查**，去 [UAX #38](https://www.unicode.org/reports/tr38/) 核 |
| **真实语料上，覆盖前 N 个高频字需要多大的表** | **跑它**——`vault/raw/` 还是空的，这个数没人能替我们回答 |
| **哪几类变体参与展开，精确率各掉多少** | **跑它**——`Simplified` / `Old` / `Wrong` / `SementicVariant` 逐类开关，量四次召回与精确。**这正是「输出清楚、不知道拿什么喂」的场合** |

→ 两个都归 `docs/experiments/`，**不在本目录另开一处**。

---

## 三、逐条核查：`resources/charabia` 读后（2026-09-06）

> 规矩：**给不出「哪个文件、哪几行」的结论一律不写。**
> 另两个仓（`unihan-database` / `irg`）是纯数据，无实现可读，本节只针对 `charabia`。

### 1 · 我要做的这件事，它是怎么做的

| 做法 | 出处 |
|---|---|
| **逐字符折叠**：查表取 `destination_ideograph`，**查不到就原样返回 `c`**（表外字符不报错不丢弃——与我们一致） | `charabia/src/normalizer/chinese.rs:19-22` |
| **全半角走 NFKD**（不是 NFKC），且在**始终生效**那一档 | `charabia/src/normalizer/compatibility_decomposition.rs:18`；档位见 `charabia/src/normalizer/mod.rs:52-61` |
| **簡繁归一在另一档**：`ChineseNormalizer` 属于 `LOSSY_NORMALIZERS` | `charabia/src/normalizer/mod.rs:64-77`，具体第 `69-70` 行 |
| **偏移靠 `char_map` 维护**：原文每字符字节数 → 归一后字节数 | 定义 `charabia/src/token.rs:57-59`；首次建立 `charabia/src/normalizer/mod.rs:198-210`；逐层叠加 `mod.rs:182-196`；换算回原文 `charabia/src/token.rs:139-152` |
| **应用范围显式收窄**：只在 `Script::Cj` 且语言为 None/Cmn/Zho 时生效 | `charabia/src/normalizer/chinese.rs:40-43` |
| **数据构建期压成三列再内嵌** | `irg-kvariants/build.rs:23-45`；`irg-kvariants/src/lib.rs:37` |

### 2 · 该照着抄的设计判断（抄判断，不抄代码）

| # | 判断 | 出处 | 对 42find 意味着什么 |
|---|---|---|---|
| 1 | **把「有损」写进结构，而不是写进注释**：常规归一与有损归一是**两个列表**，簡繁归一在有损那一档 | `charabia/src/normalizer/mod.rs:52-61` vs `64-77`（`ChineseNormalizer` 在 `69-70`） | `kSemanticVariant`(3538) 该归进「有损」并**默认关**，靠类型和默认值挡住，不靠注释提醒 |
| 2 | **「折叠 + 保偏移」有现成解法，代价是每 token 一个 `Vec<(u8,u8)>`；而且默认不付这个代价** | `charabia/src/token.rs:57-59`；默认值 `charabia/src/normalizer/mod.rs:85`（`create_char_map: false`） | **直接修正我上面写的判断**——折叠不是「天生丢偏移」，是**要额外掏钱**。我们选查询扩展省掉的是这笔钱，不是解决了一个无解问题 |
| 3 | **拒绝传递闭包，并用测试钉住**：destination 不许再有 destination | `irg-kvariants/src/lib.rs:126-142`（`test_no_loop`） | 与我们「变体只回连规范形、不连兄弟变体」是同一判断的两种说法。他们用测试钉，我们已有两条钉子测试，方向对 |
| 4 | **暂不处理的事，把「以后怎么处理」一起写在断言里** | `irg-kvariants/src/lib.rs:60-65`（`debug_assert!` 的消息写明「以后要按 classification 定优先级」） | **这条是警告不是范本**：他们的数据一源一目标，我们的 Unihan **有 489 例一对多**，所以这个优先级我们**现在就得定** |

### 3 · 它明确没做什么，以及是能力还是刻意

| # | 没做的 | 出处 | 判定 |
|---|---|---|---|
| 1 | **没有反向查询**——给不出「规范形 → 变体集」。canonical 字符根本不是 key | `irg-kvariants/src/lib.rs:123`：`assert_eq!(KVARIANTS.get(&'刃'), None)` | **刻意**。折叠只需单向，反向对它无用 |
| 2 | **没做一源多目标** | `irg-kvariants/src/lib.rs:60-65` | **刻意推迟**。断言里写明了将来怎么补 |
| 3 | **没做传递闭包** | `irg-kvariants/src/lib.rs:126-142` | **刻意**，有测试守着 |
| 4 | **拼音归一默认不开** | `charabia/src/normalizer/chinese.rs:1` 与 `:27`（`#[cfg(feature = "chinese-normalization-pinyin")]`） | **刻意**。拼音把同音字全混在一起，损失过大，做成可选 |
| 5 | **偏移映射默认关** | `charabia/src/normalizer/mod.rs:85` | **刻意**。代价按需付 |
| 6 | **一份死代码没删干净**：`charabia/src/normalizer/chinese/kvariants.rs` 全文与 `irg-kvariants/src/lib.rs` 重复，但**全仓没有任何 `mod kvariants` 引用它**（`chinese.rs:19` 用的是外部 crate `irg_kvariants`），且它 `include_str!` 的 `dictionaries/txt/chinese/kVariants.tsv` **在仓里不存在** | 死文件 `charabia/src/normalizer/chinese/kvariants.rs:37`；实际引用 `charabia/src/normalizer/chinese.rs:19` | **不是刻意，是遗漏**。把 in-tree 版本抽成独立 crate 之后留下的残骸。**对我们的意义**：铁律 7「唯一编码」不是洁癖——留副本就会留出这种东西，而且是在一个维护良好的仓里 |
