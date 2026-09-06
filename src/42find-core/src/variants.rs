//! 变体表的查表层。数据在 `variants_generated.rs`（机器生成，勿手改）。
//!
//! ⚠️ 表里存的**不是**成对等价。等价关系天然对称且传递——写下 `发≡發` 与 `发≡髮`，
//! 就隐含了 `發≡髮`，于是查「發」会命中「髮」，我们靠查询扩展避开的假阳性从后门溜回来。
//! 实际语感是**非对称**的：简体「发」本就同时对应「發」与「髮」，
//! 但繁体用户查「發」绝不期待命中「髮」。
//!
//! **这层非对称在数据里，不在代码里**：Unihan 把它拆成了两个字段——
//! `kTraditionalVariant` 简→繁一对多、`kSimplifiedVariant` 繁→简多对一。
//! 生成脚本取五个字段的并集，得到的仍是非对称表：
//! `expand(发)={发,發,髮}` 而 `expand(發)={發,发}`。
//! `src/42find-core/src/lib.rs` 有两条钉子测试守着这件事。

use crate::variants_generated::{KEYS, VALS};

/// `c` 展开时额外并入的字符（不含 `c` 自身）。表外字符返回 `None`——
/// 调用方据此原样匹配，**不报错也不丢弃**。
///
/// `KEYS` 已排序，二分查找。1.5 万条时线性扫描会让每个查询字符都过一遍全表。
pub(crate) fn variants_of(c: char) -> Option<&'static str> {
    KEYS.binary_search(&c).ok().map(|i| VALS[i])
}
