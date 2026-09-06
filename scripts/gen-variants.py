#!/usr/bin/env python3
"""从 Unicode Unihan 一手数据生成 42find 的变体展开表。

用法：python3 scripts/gen-variants.py
输入：resources/unihan-database/（Unicode License V3，不进仓，用 clone.sh 取）
输出：src/42find-core/src/variants_generated.rs（**入库**）

字段选择依据 docs/experiments/exp002-unihan-coverage/：模式 B（五字段、一跳）。
**不做传递闭包**——exp002 实测两跳一个都修不好还打破非对称（docs/plan/001 决策二）。
"""
import re, pathlib, subprocess, sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "resources/unihan-database"
OUT = ROOT / "src/42find-core/src/variants_generated.rs"

# exp002 模式 B。顺序不影响结果（最终取并集），按规模排便于阅读。
FIELDS = [
    "kSimplifiedVariant.txt",
    "kTraditionalVariant.txt",
    "kSemanticVariant.txt",
    "kSpecializedSemanticVariant.txt",
    "kZVariant.txt",
]

if not SRC.is_dir():
    sys.exit(
        f"缺 {SRC.relative_to(ROOT)}——先取一手数据：\n"
        "  bash .claude/skills/aias-meta-research/scripts/clone.sh "
        "https://github.com/unicode-org/unihan-database"
    )


def load(fn):
    """解析一个 Unihan 变体文件。行格式：`U+XXXX 字\\t字段名\\tU+YYYY U+ZZZZ<来源`"""
    out = {}
    for line in (SRC / fn).read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        src = chr(int(parts[0].split()[0][2:], 16))
        tgts = [chr(int(c[2:], 16)) for c in re.findall(r"U\+[0-9A-F]+", parts[2])]
        # 剥掉自指：unihan-database#408，两个主字段各 431 条。
        # 不剥的话展开集里会留重复字符，且 key 仍在会误导调用方。
        out.setdefault(src, []).extend(t for t in tgts if t != src)
    return out


table: dict[str, list[str]] = {}
per_field = {}
for fn in FIELDS:
    m = load(fn)
    per_field[fn] = sum(len(v) for v in m.values())
    for k, vs in m.items():
        table.setdefault(k, []).extend(vs)

# 去重保序；丢掉并集后为空的 key（只有自指的那些）
clean = {}
for k, vs in table.items():
    seen, uniq = set(), []
    for v in vs:
        if v not in seen:
            seen.add(v)
            uniq.append(v)
    if uniq:
        clean[k] = uniq

keys = sorted(clean)                       # 排序是二分查找的前提
rev = subprocess.run(["git", "-C", str(SRC), "rev-parse", "--short", "HEAD"],
                     capture_output=True, text=True).stdout.strip() or "unknown"


def rs_char(c):
    return f"'\\u{{{ord(c):X}}}'"


def rs_str(s):
    return '"' + "".join(f"\\u{{{ord(c):X}}}" for c in s) + '"'


lines = [
    "// @generated —— 机器生成，**不要手改**。",
    "// 生成脚本：scripts/gen-variants.py（改规则请改脚本后重跑）",
    "//",
    "// 数据来源：Unicode Han Database (Unihan)，",
    f"//   unicode-org/unihan-database @ {rev}",
    f"//   字段：{', '.join(f.removesuffix('.txt') for f in FIELDS)}",
    "//   已剥离自指条目（unihan-database#408）。**不做传递闭包**——",
    "//   exp002 实测两跳修不好任何样本，还打破非对称。",
    "//",
    "// Copyright © 1991-2026 Unicode, Inc. All rights reserved.",
    "// Distributed under the Terms of Use in https://www.unicode.org/copyright.html",
    "// Licensed under the Unicode License V3; 本表是该数据的衍生物。",
    "//",
    "// ⚠️ kSimplifiedVariant / kTraditionalVariant 在 UAX #38 里是 provisional 字段。",
    "",
    "/// 参与展开的字符，**已排序**——`variants_of` 靠它二分查找。",
    f"pub(crate) const KEYS: &[char] = &[",
]
lines += ["    " + ", ".join(rs_char(c) for c in keys[i:i + 8]) + ","
          for i in range(0, len(keys), 8)]
lines += [
    "];",
    "",
    "/// 与 `KEYS` 逐位对应：该字符展开时**额外**并入的字符（不含自身）。",
    "pub(crate) const VALS: &[&str] = &[",
]
lines += ["    " + ", ".join(rs_str("".join(clean[c])) for c in keys[i:i + 4]) + ","
          for i in range(0, len(keys), 4)]
lines += ["];", ""]

OUT.write_text("\n".join(lines), encoding="utf-8")

print(f"数据版本 unihan-database @ {rev}")
for fn, n in per_field.items():
    print(f"  {fn.removesuffix('.txt'):<30} {n:>6} 条关系")
print(f"→ {OUT.relative_to(ROOT)}：{len(keys)} 个字符，"
      f"{sum(len(v) for v in clean.values())} 条展开关系，"
      f"{OUT.stat().st_size / 1024:.0f} KB")
