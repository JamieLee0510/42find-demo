//! 在原文上按展开后的查询扫描，报出**原文的**行与字节列。

use crate::expand::Expansion;

/// 一处命中。`col` 是 **1-based 字节列**，与 `rg --column` 同单位。
///
/// 借用被扫的那段文本，不复制——查「的」在本仓上有两千余处命中，
/// 逐处克隆一份整行是纯浪费。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match<'a> {
    /// 1-based 行号。
    pub line: usize,
    /// 1-based 字节列。
    pub col: usize,
    /// 原文里实际命中的那一段（不是归一形）。
    pub text: &'a str,
    /// 命中所在的**整行**（不含行尾换行）。
    ///
    /// 光给 `text` 等于「只把你自己敲的那个词还给你」——得再打开文件才知道
    /// 那句话在说什么。输出层要的是 `rg` 那种 `路径:行号:整行`，
    /// 而整行只有扫描时手里有，事后由行号回查等于把文本再读一遍。
    pub line_text: &'a str,
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
pub fn search_line<'a>(exp: &Expansion, lineno: usize, line: &'a str) -> Vec<Match<'a>> {
    if exp.is_empty() {
        return Vec::new();
    }
    line.char_indices()
        .filter_map(|(i, _)| {
            match_at(exp, line, i).map(|end| Match {
                line: lineno,
                col: i + 1,
                text: &line[i..end],
                line_text: line,
            })
        })
        .collect()
}

/// 扫整段文本。
#[must_use]
pub fn search<'a>(exp: &Expansion, text: &'a str) -> Vec<Match<'a>> {
    text.lines()
        .enumerate()
        .flat_map(|(i, line)| search_line(exp, i + 1, line))
        .collect()
}
