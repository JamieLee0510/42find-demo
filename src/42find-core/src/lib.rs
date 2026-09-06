//! 42find 的能力层：查询扩展与扫描。**不认识命令行**——依赖是单向的（见 `.42cog/cog.md`）。
//!
//! 这一刀做的是**查询扩展**，不是语料归一：查询词逐字展开成等价写法再扫原文，
//! 所以命中的行列就是原文的行列，精确率不因归一而下降。
//!
//! ⚠️ 本层的扫描**不是产品形态，是参照实现**——它定义「归一之后正确的召回长什么样」，
//! 是日后上索引时不许退的那把尺子。意向书排除的是「重造一个 ripgrep」，不是这把尺子。

mod expand;
mod search;
mod variants;
#[rustfmt::skip]
mod variants_generated;

pub use expand::{Expansion, expand};
pub use search::{Match, search, search_line};

#[cfg(test)]
mod tests {
    use super::*;

    fn class_of(q: &str, i: usize) -> Vec<char> {
        expand(q).class(i).expect("位置存在").to_vec()
    }

    #[test]
    fn 简体规范形展开全部变体() {
        let cs = class_of("发", 0);
        assert!(cs.contains(&'发') && cs.contains(&'發') && cs.contains(&'髮'));
    }

    #[test]
    fn 繁体变体不得命中兄弟变体() {
        // ★ 钉子：等价「类」是错的抽象——展开必须非对称
        let fa = class_of("發", 0);
        assert!(fa.contains(&'發') && fa.contains(&'发'));
        assert!(!fa.contains(&'髮'), "查「發」不该命中「髮」");

        let fa2 = class_of("髮", 0);
        assert!(fa2.contains(&'髮') && fa2.contains(&'发'));
        assert!(!fa2.contains(&'發'), "查「髮」不该命中「發」");
    }

    #[test]
    fn 一个字同时是两个字段的key时反向映射不被吞() {
        // ★ 钉子：「裡」在 kTraditionalVariant 里指向自己、在 kSimplifiedVariant 里指向「里」。
        // 拆成两次查找再 else if，反向映射会被静默吃掉（exp002 实测：繁→简 20/27 → 21/27）。
        // 手写小表时代构造不出这个用例，换生成表后才测得了。
        assert!(class_of("裡", 0).contains(&'里'), "「裡」必须能查回「里」");
        assert!(class_of("里", 0).contains(&'裡'));
        assert!(class_of("里", 0).contains(&'裏'));
    }

    #[test]
    fn 展开集无重复字符() {
        // Vec::dedup 只去相邻重复；生成表下重复未必相邻
        for q in ["发", "發", "里", "裡", "台", "系"] {
            let cs = class_of(q, 0);
            let mut sorted = cs.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(cs.len(), sorted.len(), "{q} 的展开集有重复：{cs:?}");
        }
    }

    /// **全表结构不变量：展开不得发明数据里没有的边。**
    ///
    /// 对每个字符 `c`，`expand(c)` 的结果只能是 `{c}` ∪ `VALS[c]` ∪ 全半角换算——
    /// 不得出现任何第三来源的字符。这条挡住的是**传递闭包偷偷长回来**：
    /// 两跳会让 `expand(發)` 变成 `發发髮`，而 exp002 实测两跳一个样本都修不好
    /// （`docs/experiments/exp002-unihan-coverage/`）。
    ///
    /// ⚠️ **不要把它写成「兄弟变体不得互查」**——我试过两次，两次都太强：
    /// 第一次全表 1986 组违反（语义变体在 Unihan 里交叉登记、天然对称）；
    /// 第二次缩到「简→繁一对多」仍有 76 组（`為/爲`、`麼/麽`、`眾/衆` 是同一个繁体字的
    /// 两种字形，互查本来就对）。**一简对多繁里哪些该互查、哪些不该，是 Unihan
    /// 逐对编码的语义判断，结构上推不出来。** 那一截由「发/發/髮」等钉子测试按判据守。
    #[test]
    fn 全表结构不变量_展开不发明新边() {
        use crate::variants_generated::{KEYS, VALS};
        let mut invented = Vec::new();
        for (i, &k) in KEYS.iter().enumerate() {
            let allowed: Vec<char> = std::iter::once(k).chain(VALS[i].chars()).collect();
            let e = expand(&k.to_string());
            let Some(class) = e.class(0) else { continue };
            for &c in class {
                // 全半角换算是规则不是查表，单独放行
                let width_pair = (c as u32).abs_diff(k as u32) == 0xFEE0;
                if !allowed.contains(&c) && !width_pair {
                    invented.push((k, c));
                }
            }
        }
        assert!(
            invented.is_empty(),
            "展开发明了 {} 条数据里没有的边，前几例 {:?}",
            invented.len(),
            &invented[..invented.len().min(5)]
        );
    }

    #[test]
    fn 全半角双向归一() {
        assert!(class_of("q", 0).contains(&'ｑ'));
        assert!(class_of("ｑ", 0).contains(&'q'));
        assert!(class_of(" ", 0).contains(&'\u{3000}'));
        assert!(class_of("\u{3000}", 0).contains(&' '));
    }

    #[test]
    fn 表外字符原样不报错不丢弃() {
        // 用的字必须**核实过**真的不在表里。原先写「龘」是错的——
        // 它在 Unihan 里有 kSimplifiedVariant→𮹝，只是手写小表时代看不见。
        // 「索」「日」「月」经 resources/unihan-database 全字段扫描确认无任何变体。
        assert_eq!(class_of("ひ", 0), vec!['ひ']); // 假名，本刀不做假名归一
        assert_eq!(class_of("索", 0), vec!['索']);
        assert_eq!(class_of("日", 0), vec!['日']);
        assert_eq!(expand("").len(), 0);
    }

    #[test]
    fn 簡繁互查() {
        let hits = search(&expand("检索"), "繁體寫法：檢索、歸一。");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "檢索", "命中的是原文的写法，不是归一形");
    }

    #[test]
    fn 字节列与多字节字符() {
        // 「异体字：」= 4 个字符 × 3 字节 = 12，故「户」起于第 13 字节
        let hits = search(&expand("户"), "异体字：户口");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 1);
        assert_eq!(
            hits[0].col, 13,
            "col 必须是 1-based 字节列，与 rg --column 同单位"
        );
    }

    #[test]
    fn 词夹在句中无空格也能命中() {
        let hits = search(&expand("检索"), "先归一再检索，还是先检索再归一。");
        assert_eq!(
            hits.len(),
            2,
            "回归护栏：查询扩展不得把 rg 本来就过关的情形弄坏"
        );
    }

    #[test]
    fn 多行报出正确行号() {
        let hits = search(&expand("户"), "第一行无\n第二行有户");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 2);
    }
}
