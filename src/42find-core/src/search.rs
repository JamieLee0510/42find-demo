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
///
/// ⚠️ **字段私有、只经 `search_line` 构造**，不是为了封装好看，是为了让一条
/// **跨字段不变量**（`col-1` 与 `col-1+byte_len` 都落在 `line_text` 的字符边界上）
/// 在类型上无法被违反。字段公开时它只由 `search_line` 一处遵守，而
/// `Match { col: 0, .. }.text()` 会下溢 panic、`col` 落在多字节字符中间会非边界 panic，
/// 拿一个合法 `Match` 把 `line_text` 截短再复用也会越界 panic——**编译通过、零警告**。
/// 上面刚说过「留两个重叠视图拦不住下一个人挑错字段」；留一堆可写字段，
/// 一样拦不住下一个人填错值。**能让类型系统判的，别留给注释判。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match<'a> {
    line: usize,
    col: usize,
    byte_len: usize,
    line_text: &'a str,
}

impl<'a> Match<'a> {
    /// 1-based 行号。
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// 1-based **字节**列，与 `rg --column` 同单位。
    #[must_use]
    pub fn col(&self) -> usize {
        self.col
    }

    /// 命中那一段的字节长度。与 `col` 同单位。
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// 命中所在的**整行**（不含行尾换行）。
    #[must_use]
    pub fn line_text(&self) -> &'a str {
        self.line_text
    }

    /// 原文里实际命中的那一段——**是原文的写法，不是归一形**。
    ///
    /// 查「检索」命中「檢索」时，这里给的是「檢索」。本刀在原文上扫、不改语料，
    /// 这个方法就是那条承诺的载体（钉子测试 `簡繁互查` 守着它）。
    ///
    /// ⚠️ 这里的切片**不会 panic**，因为 `col`/`byte_len` 只可能由 `search_line` 写入——
    /// 见上面结构体文档里「字段私有」那一段。
    #[must_use]
    pub fn text(&self) -> &'a str {
        let start = self.col - 1;
        &self.line_text[start..start + self.byte_len]
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
                byte_len: end - i,
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
///
/// ⚠️ **`Match` 借的是传进来的 `text`**，所以调用方必须把整段文本留到命中用完为止。
/// 今天 `42find-cli` 是 `read_to_string` 整文件读入——**日后要改成逐行流式读，
/// 必须和这里的借用设计一起改**，否则 `line_text` 的生命周期挂在一个已经不存在的
/// `String` 上，编译期就会挡住，别到那时才发现是设计冲突。
#[must_use]
pub fn search<'a>(exp: &Expansion, text: &'a str) -> Vec<Match<'a>> {
    text.lines()
        .enumerate()
        .flat_map(|(i, line)| search_line(exp, i + 1, line))
        .collect()
}
