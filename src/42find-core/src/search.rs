//! 在原文上按展开后的查询扫描，报出**原文的**行与字节列。

use crate::expand::Expansion;

/// 一处命中。`col` 是 **1-based 字节列**，与 `rg --column` 同单位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// 1-based 行号。
    pub line: usize,
    /// 1-based 字节列。
    pub col: usize,
    /// 原文里实际命中的那一段（不是归一形）。
    pub text: String,
}

/// 从 `line` 的字节位置 `start` 起能否完整匹配；能则返回结束字节位置。
fn match_at(exp: &Expansion, line: &str, start: usize) -> Option<usize> {
    let mut chars = line[start..].char_indices();
    let mut end = start;
    for class in exp.classes() {
        let (off, ch) = chars.next()?;
        if !class.contains(&ch) {
            return None;
        }
        end = start + off + ch.len_utf8();
    }
    Some(end)
}

/// 扫一行。允许重叠命中（与 `vault/truth/queries.tsv` 的计数口径一致）。
#[must_use]
pub fn search_line(exp: &Expansion, lineno: usize, line: &str) -> Vec<Match> {
    if exp.is_empty() {
        return Vec::new();
    }
    line.char_indices()
        .filter_map(|(i, _)| {
            match_at(exp, line, i).map(|end| Match {
                line: lineno,
                col: i + 1,
                text: line[i..end].to_owned(),
            })
        })
        .collect()
}

/// 扫整段文本。
#[must_use]
pub fn search(exp: &Expansion, text: &str) -> Vec<Match> {
    text.lines()
        .enumerate()
        .flat_map(|(i, line)| search_line(exp, i + 1, line))
        .collect()
}
