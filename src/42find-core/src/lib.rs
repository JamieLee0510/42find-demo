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
    fn 全半角双向归一() {
        assert!(class_of("q", 0).contains(&'ｑ'));
        assert!(class_of("ｑ", 0).contains(&'q'));
        assert!(class_of(" ", 0).contains(&'\u{3000}'));
        assert!(class_of("\u{3000}", 0).contains(&' '));
    }

    #[test]
    fn 表外字符原样不报错不丢弃() {
        assert_eq!(class_of("ひ", 0), vec!['ひ']);
        assert_eq!(class_of("龘", 0), vec!['龘']);
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
