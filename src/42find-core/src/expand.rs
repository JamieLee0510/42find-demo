//! 查询扩展：把查询词逐字展开成「这个位置可以接受哪些字符」。
//!
//! **不动语料**——所以命中的行列就是原文的行列，偏移天然精确。

use crate::variants::variants_of;

/// 全角 ASCII 块 `U+FF01..=U+FF5E` 与半角 `!..~` 之间的固定偏移。
const FULLWIDTH_OFFSET: u32 = 0xFEE0;
const FULLWIDTH_START: u32 = 0xFF01;
const FULLWIDTH_END: u32 = 0xFF5E;
/// 表意空格 `U+3000`，半角对应普通空格。
const IDEOGRAPHIC_SPACE: char = '\u{3000}';

/// 展开后的查询：每个位置一组可接受的字符。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    classes: Vec<Vec<char>>,
}

impl Expansion {
    /// 位置数（即原查询的字符数）。
    #[must_use]
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// 空查询？
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    /// 第 `i` 个位置接受的字符集合。
    #[must_use]
    pub fn class(&self, i: usize) -> Option<&[char]> {
        self.classes.get(i).map(Vec::as_slice)
    }

    pub(crate) fn classes(&self) -> &[Vec<char>] {
        &self.classes
    }
}

/// 单个字符展开成它的可接受集合。
///
/// 规则依次是：全半角整块换算 → 表意空格 → 规范形展开全部变体 →
/// 变体只回连规范形（**不含兄弟变体**）→ 表外字符原样。
fn expand_char(c: char) -> Vec<char> {
    let mut out = vec![c];

    let code = c as u32;
    if (FULLWIDTH_START..=FULLWIDTH_END).contains(&code) {
        // 全角 → 半角：整块偏移，六行代码全覆盖，比手写表更完整且同样零依赖
        if let Some(half) = char::from_u32(code - FULLWIDTH_OFFSET) {
            out.push(half);
        }
    } else if ('!'..='~').contains(&c) {
        if let Some(full) = char::from_u32(code + FULLWIDTH_OFFSET) {
            out.push(full);
        }
    } else if c == IDEOGRAPHIC_SPACE {
        out.push(' ');
    } else if c == ' ' {
        out.push(IDEOGRAPHIC_SPACE);
    }

    // 并集在**生成期**就做完了（`scripts/gen-variants.py` 取五个字段的并集），
    // 所以这里只有一次查表。这样从结构上消掉了「两张表用 `else if` 会短路」那一类 bug
    // ——「裡」同时是 `kTraditionalVariant` 与 `kSimplifiedVariant` 的 key，
    // 拆成两次查找再 `else if` 就会静默吃掉反向映射（exp002 实测）。
    //
    // 并集不破坏非对称：非对称在数据里（简→繁一对多、繁→简多对一），不在代码里。
    if let Some(vs) = variants_of(c) {
        out.extend(vs.chars());
    }

    // `Vec::dedup` 只去掉**相邻**重复；并集之后重复未必相邻，所以按出现顺序去重。
    let mut seen = Vec::with_capacity(out.len());
    out.retain(|ch| {
        if seen.contains(ch) {
            false
        } else {
            seen.push(*ch);
            true
        }
    });
    out
}

/// 把查询词展开。表外字符原样保留，不报错、不丢弃。
#[must_use]
pub fn expand(query: &str) -> Expansion {
    Expansion {
        classes: query.chars().map(expand_char).collect(),
    }
}
