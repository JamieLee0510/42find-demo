//! 在原文上按展开后的查询扫描，报出**原文的**行与字节列。

use crate::expand::Expansion;

/// 一处命中。`col` 是 **1-based 字节列**，与 `rg --column` 同单位。
///
/// 借用被扫的那段文本，不复制——查「的」在本仓上有两千余处命中，
/// 逐处克隆一份整行是纯浪费。
///
/// ⚠️ **整个结构只留一个字符串视图**（`line_text`），命中的那一段由 `text()` 现算。
/// 先前是 `text` 与 `line_text` 两个字段并列，而 `text` 只是 `line_text` 的一个子切片——
/// 那正是 issue #7 第一个症状的形状：**输出层挑错了字段，把用户自己敲的那个词还给了他**。
/// 两个重叠的视图并列时，类型系统拦不住下一个人再挑错一次；只留一个就拦得住。
/// 顺带把结构从 48 字节缩到 40（清理评审实测：57,400 处命中省下约 920 KB 分配与 memcpy）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match<'a> {
    /// 1-based 行号。
    pub line: usize,
    /// 1-based 字节列。
    pub col: usize,
    /// 命中那一段的**字节长度**。与 `col` 一起在 `line_text` 上定位，见 `text()`。
    pub len: usize,
    /// 命中所在的**整行**（不含行尾换行）。
    ///
    /// 光给命中的那一段等于「只把你自己敲的那个词还给你」——得再打开文件才知道
    /// 那句话在说什么。输出层要的是 `rg` 那种 `路径:行号:整行`，
    /// 而整行只有扫描时手里有，事后由行号回查等于把文本再读一遍。
    pub line_text: &'a str,
}

impl<'a> Match<'a> {
    /// 原文里实际命中的那一段——**是原文的写法，不是归一形**。
    ///
    /// 查「检索」命中「檢索」时，这里给的是「檢索」。本刀在原文上扫、不改语料，
    /// 这个方法就是那条承诺的载体（钉子测试 `簡繁互查` 守着它）。
    #[must_use]
    pub fn text(&self) -> &'a str {
        let start = self.col - 1;
        &self.line_text[start..start + self.len]
    }
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
///
/// 按**字节列升序**产出。
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
                len: end - i,
                line_text: line,
            })
        })
        .collect()
}

/// 扫整段文本。
///
/// **按 (行号, 字节列) 升序产出。** 这是写下来的契约，不是实现细节——
/// `42find-cli` 的 `emit` 不带 `--column` 时按行去重，哨兵只跟**上一行**比，
/// 靠的就是这条。判据住在提供保证的这一头，不住在用它的那一头
/// （钉子测试 `命中按行号与字节列升序产出`）。
#[must_use]
pub fn search<'a>(exp: &Expansion, text: &'a str) -> Vec<Match<'a>> {
    text.lines()
        .enumerate()
        .flat_map(|(i, line)| search_line(exp, i + 1, line))
        .collect()
}
