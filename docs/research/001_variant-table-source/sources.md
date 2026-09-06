# 两类来源 · 簡繁与异体的映射数据从哪来

> 2026-09-06。按**谁积累的**分尽：自己积累的、别人积累的。
> **两类进来都问同一句：它今天还算数吗？**
>
> ⚠️ 「都没有所以自己跑」不在这里——**那不是来源，是做法**，见 `gap.md` 的「它们没做的」。
>
> ⚠️ 下面的市场与站点**会变**，用前核一眼；名字对不上就搜同类。**方法不变，名单会老。**

---

## 一、自有积累 —— 可照搬，先确认权利与时效

自家旧项目、老代码、团队规约、你自己写过的东西。

### 怎么查

**先跑一把**：`bash skills/aias-meta-research/scripts/scan-own.sh "<关键词>"`
——一次扫完本机项目、提交历史、过往对话记录，按最后改动时间排。

| 去哪 | 怎么翻 |
|---|---|
| **过去的项目目录** | 直接看。做过类似的事没有？哪一版跑通过？ |
| **自己的提交历史** | `git log --oneline --all` · `git log --author=<你>` ——**代码记做成什么样，提交记当时为什么这么定** |
| **过往对话记录** | 记忆和现实对不上时，去翻当时的讨论——**文档说 A、代码做了 B、你记得是 C，谁都别直接信** |
| **团队的仓库与规约** | 别人踩过的坑，规约里往往留着痕迹 |
| **自己写过的文档** | 笔记、总结、发过的文章 |

### 登记

| 是什么 | 在哪 | 还算数吗 | 判据 |
|---|---|---|---|
| **exp001 · rg 中文基准线** | `docs/experiments/exp001-rg-baseline-cjk/` | **算** | 本机真跑出来的行为（rg 15.2.0），真相源权重第一档。它证明了簡繁/全半角确实是 rg 的短板，也推翻了「词在句中」那条 |
| **issue #4 的手写小表与非对称展开** | `src/42find-core/src/variants.rs`、`expand.rs` | **算** | 三道闸门 + 8 个单测 + 固定语料实测（召回 50%→100%），两条钉子测试钉住了非对称性 |
| **黄金查询集与固定语料** | `vault/truth/` | **算，但覆盖面已知不足** | 只有 15 条查询 / 6 个映射字。**它是尺子，不是数据源**——正因为它小，才暴露了本次要作的这个决定 |
| **§2 的 crate 扫描** | `_tmp/issue-4/triage.md`（不入库，会随 `_tmp/` 清掉） | **只算一半** | 是 `cargo search` + `cargo info` 的十分钟速览，**只看了 crates.io 的元数据，没有回过任何一份源码或词典文件**。已排除的两个（`zhconv` GPL-2.0、`opencc-rust` C 绑定）证据够硬；其余「候选」三个字当不得数 |

⚠️ 最后一行是本次分析的**起点也是警告**：上一轮我拿元数据当了依据。
元数据说得出许可字段，说不出**数据文件本身**归谁管（命门 1），更说不出它**暴不暴露字符级映射**（命门 3）。

⚠️ **不自动可信**。同是自家的东西，判决可能相反：
「这个规约可能过时了，可以参考，但不能完全从它出发」vs「这个我反复测试过、生产跑通了，尽量复用」。

⚠️ **你三个月前写的东西，和陌生人写的东西，享受同一套审查。区别只在于——你有权照搬它。**

---

## 二、他人积累 —— 只读借鉴，先查协议

### 查四类，用同一套方法

**换对象，方法不变**——都是同一个决策点、同一套派活四件事（见 `assign.md`）。

| 查什么 | 查什么问题 | 去哪查 |
|---|---|---|
| **技能生态** | 这件事有没有人做成过技能？ | 各家插件市场与技能仓库；搜 `awesome-<你的领域>` 一类的清单仓 |
| **命令行工具** | 有哪些硬工具，用哪个？ | 三大分发市场：**Homebrew**（Mac）· **npm**（JS，四百万包）· **PyPI**（Python，八十多万项目）。Windows 看 **Scoop** / **winget** |
| **模型** | 这个活该用哪个模型？ | 公开评测榜单与各家技术报告。**要多模态、要长上下文，就得专门看那一栏** |
| **别人的项目** | 源码 · 提交历史 · 放弃过什么 | 代码托管平台（GitHub / GitLab / 国内平台）。**浅克隆到参考区再读**，别在网页上翻 |

**取材**：`bash skills/aias-meta-research/scripts/clone.sh <url>`（要挖提交历史加 `-d 200`）

### 查模型这一栏，多问三句

榜单也是他人积累，**同样要判时效、判来路**：

- **谁做的榜**？做榜的人和被测的模型有没有利害关系？
- **什么时候的**？半年前的榜单在这个行当里基本等于过期。
- **测的是不是你要的那件事**？综合分高，不代表你这一类任务上强。

**最后一句最要紧**：能自己拿三五个真实任务试一遍，胜过读十份榜单。

### 登记

**⚠️ 尚未取材。** `clone.sh` 要人显式调用——把外部代码拉进本地是你的决定，不是它的（技能边界）。
下面是**待取材清单**，按命门 3（能不能抽出字符级非对称关系）排的优先级：

| 是什么 | 从哪取 | 预期协议 | 角色 | 读多深 | 为什么排这个序 |
|---|---|---|---|---|---|
| **Unicode Unihan 数据库** | unicode.org 的 Unihan 压缩包（**不是 GitHub 镜像**，要一手） | Unicode License | **要比的** | 逐条核查带字段名 | 命门 3 最可能原生满足的一条：它本来就是「字符 → 变体」的关系表，不是转换器 |
| **OpenCC 原始词典** | `github.com/BYVoid/OpenCC` 的 `data/dictionary/` | 待核（**代码与数据可能不同**） | **要比的** | 逐条核查带文件名与行样例 | 覆盖面最完整；但要确认它给不给得出字符级一对多 |
| `ferrous-opencc` | `github.com/apoint123/ferrous-opencc` | Apache-2.0（元数据） | **零件** | 只看公开 API 一两行 | 只需回答一句：暴不暴露底层映射表 |
| `zhhz` | `github.com/ljh-sh/zhhz` | Apache-2.0（元数据） | **零件** | 同上 | 同上，自称 data-embedded，值得看它怎么嵌 |
| `fast2s` | `github.com/tyrchen/fast2s` | MIT（元数据） | **要学的** | 只读架构 | 单向 t2s、FST 实现——**它的数据组织方式**可能比它的功能更有用 |

**取材命令**（要挖提交历史加 `-d 200`）：

```bash
bash .claude/skills/aias-meta-research/scripts/clone.sh https://github.com/BYVoid/OpenCC
```

⚠️ 技能文档里写的是 `skills/aias-meta-research/...`，**本仓的技能实际在 `.claude/skills/` 下**，
`skills/` 目录里只有一份 README。照文档抄会 `No such file`。

### 实际读到的（2026-09-06，**未 clone，靠一手文档 + docs.rs 源码页 + 本机 `cargo info`**）

| 是什么 | 在哪 | 协议 | 角色 | 读到哪一步 |
|---|---|---|---|---|
| **`irg-kvariants` 的数据结构** | [docs.rs 源码页](https://docs.rs/irg-kvariants/0.1.1/src/irg_kvariants/lib.rs.html) | 元数据 MIT，**数据层无 LICENSE** | 要比的 | **逐条**：`KVariant` 三字段、`KVariantClass` 五类、`include_bytes!` 构建期内嵌 |
| **charabia 的 `ChineseNormalizer`** | [github.com/meilisearch/charabia](https://github.com/meilisearch/charabia) | MIT | 要学的 | 只读做法：按 `KVARIANTS` 归到 canonical，字符级 |
| **`irg-kvariants/` 目录的授权** | [charabia/irg-kvariants](https://github.com/meilisearch/charabia/tree/main/irg-kvariants) | **无 LICENSE 文件** | 要比的 | 逐条：README 只写「wrapping hfhchan/irg 的 kVariants.md」 |
| **`hfhchan/irg`（上游数据源）** | [github.com/hfhchan/irg](https://github.com/hfhchan/irg) | **看不到 LICENSE** | 要比的 | 一两行：这就是 P0 的来源 |
| **Lucene ICU 分析器** | [analyzers-icu 概览](https://lucene.apache.org/core/8_11_4/analyzers-icu/overview-summary.html) | Apache-2.0 | 要学的 | 只读做法：`ICUTransformFilter(Traditional-Simplified)` + `CJKWidthFilter` |
| **UAX #38 Unihan** | [unicode.org/reports/tr38](https://www.unicode.org/reports/tr38/) | Unicode License V3 | 要比的 | 逐条：`kSimplifiedVariant` / `kTraditionalVariant` 是 **provisional** |
| **Unicode License V3** | [unicode.org/license.txt](https://www.unicode.org/license.txt) | — | 要比的 | 逐条：允许 use/copy/modify/distribute/sell，声明放随附文档即可 |
| **11 个 crate 的许可** | 本机 `cargo info` | 见 `gap.md` 横向表 | 要比的 | 逐条：版本 + 许可字段 |

### 已取材（2026-09-06 · `clone.sh --depth 1`，共 11.8 MB，`resources/**` 不进仓）

| 仓 | 角色 | 许可（**落地核实**） | 最近提交 | 判决 |
|---|---|---|---|---|
| `resources/unihan-database` | **要比的** | **Unicode License V3**（有 LICENSE，Copyright 2021-2026 Unicode Inc.） | **2026-07-24** | **选它**。装着 `kSimplifiedVariant` / `kTraditionalVariant` / `kZVariant` / `kSemanticVariant` / `kSpoofingVariant` |
| `resources/charabia` | **要学的** | 根 LICENSE = **MIT**（Meili SAS）；`irg-kvariants/` 子目录**无独立 LICENSE** | 活跃 | 学它的**分类思路**（`KVariantClass` 五分类），数据一行不用 |
| `resources/irg` | 要比的 | **全仓 `find` 无 LICENSE / COPYING / NOTICE** | **2023-05-26** | **出局**：又老又没授权 |

⚠️ **落地才看得出的两件事**，网页视图给不了：
① `irg` 三年没动过 ② Unihan 两个变体文件**单向不闭合 0 处**。
这两条直接把拍板点 1 从「A+B」改成了「A 独用」。

⚠️ **读懂 → 自己写，绝不复制粘贴。** GPL / AGPL 只参考不链接。
⚠️ **协议不只回答「能不能用」，还决定「能读到哪一步」。**

---

## 两类都不算数时

那不是失败，那是**下一步的入口**：确认了没有，才该去跑实验。
接着填 `gap.md` 的「它们没做的」——**「压根没有」是查完之后的结果，不是时间判断。**

## 最后

**读，是复用别人的；跑，是长出你自己的。** 这一份只管前半——材料从哪来；
后半在 `gap.md` 与 `decision.md`。
