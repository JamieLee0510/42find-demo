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
# `cargo package` 只带 package 目录里的文件，根目录的 NOTICE / LICENSE 到不了
# Cargo 消费者手里。这两份由脚本机器同步，不手抄——手抄就会走样（已走样过一次）。
PKGS = [ROOT / "src/42find-core", ROOT / "src/42find-cli"]

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
        "https://github.com/unicode-org/unihan-database\n"
        "  # ⚠️ clone.sh 拿的是默认分支的 HEAD。要复现入库的生成物，还要钉到记录的 commit：\n"
        "  #   git -C resources/unihan-database fetch --depth 50 && git -C ... checkout <生成物头部的 commit>"
    )


def load(fn):
    """解析一个 Unihan 变体文件。

    ⚠️ `docs/experiments/exp002-unihan-coverage/probe.py` 里有一份**同形但已漂移**的实现
    （那边是 `m[src] = [...]` 后写覆盖，这边是 `setdefault().extend()` 累加）。
    当前 Unihan 每字段每字符一行，两者结果相同；**上游哪天在一个字段里给同一个字符出两行，
    探针与生成器就会给出不同的表**——而 exp002 的结论正是拿来给这里选字段的依据。
    改这里必须同步改那边（那份是实验的事实记录，不主动重构）。

    行格式：`U+XXXX 字\\t字段名\\tU+YYYY U+ZZZZ<来源`"""
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
def data_rev():
    """记数据版本。**必须连 dirty 一起记**——只记 HEAD 的话，本地改过 Unihan 文件
    生成出来的表，头部会写着一个复现不出它的 commit；而「重跑生成器 git diff 为空」
    这条验收在同一个 dirty checkout 上照样通过，查不出来。"""
    def git(*a):
        return subprocess.run(["git", "-C", str(SRC), *a],
                              capture_output=True, text=True).stdout.strip()
    rev = git("rev-parse", "--short", "HEAD") or "unknown"
    if git("status", "--porcelain"):
        rev += "-dirty"
    return rev


rev = data_rev()


def upstream_copyright():
    """从上游 LICENSE 里取版权行。**不要写死**——写死过一次，年份就编错了一次
    （写成 1991-2026，上游实为 2021-2026，2026-09-06 评审抓到）。"""
    for line in (SRC / "LICENSE").read_text(encoding="utf-8").splitlines():
        if line.startswith("Copyright"):
            return line.strip()
    raise SystemExit("上游 LICENSE 里找不到版权行——不要凭印象补，去核实")


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
    # 版权行**从上游 LICENSE 里读**，不写死——写死过一次，年份就编错了一次
    f"// {upstream_copyright()}",
    "// Licensed under the Unicode License V3 (SPDX: Unicode-3.0)；本表是该数据的衍生物。",
    "// 完整许可文本见同 crate 根目录的 NOTICE（上游 LICENSE 的逐字照抄）。",
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

# 分发义务：Unicode-3.0 要求 notice 随副本或随附文档出现。
# ⚠️ **NOTICE 全文由这里生成，上游许可正文从文件读**——先前是手抄 41 行，
# 而同一份文件里 `upstream_copyright()` 的 docstring 写着「不要写死——写死过一次，
# 年份就编错了一次」。那条教训当时只落实到了 1/41 行。
UPSTREAM_LICENSE = (SRC / "LICENSE").read_text(encoding="utf-8").rstrip("\n")


def notice_for(table_path: str, license_ref: str, extra_head: str = "") -> str:
    return f"""# NOTICE — 第三方材料声明
{extra_head}
本项目代码以 MIT 许可发布（见 {license_ref}）。
除此之外，产出的 `42find` 二进制中**内嵌了一份 Unicode 数据的衍生物**，
其许可与代码不同，声明如下。

## Unicode Han Database (Unihan)

- **文件**：{table_path}
- **来源**：<https://github.com/unicode-org/unihan-database>
- **SPDX**：`Unicode-3.0`

下面是**上游 `LICENSE` 的逐字照抄**，由 `scripts/gen-variants.py` 从文件读出写入。
⚠️ 不要手改、不要转述——2026-09-06 的评审抓到过一次：
版权年份被写成 `1991-2026`，上游实为 `2021-2026`。许可文本只能照抄。

```
{UPSTREAM_LICENSE}
```

> Unicode License V3 允许声明「随副本」**或**「随附文档」出现。
> 本文件即那份随附文档；**分发二进制时必须一并带上它**。
"""


GEN_HEAD = ("\n> 本文件由 `scripts/gen-variants.py` 生成，路径已改写为 crate 视角。"
            "**不要手改。**\n")
(ROOT / "NOTICE").write_text(
    notice_for("`src/42find-core/src/variants_generated.rs`（机器生成）",
               "[`LICENSE`](LICENSE)"), encoding="utf-8")
(PKGS[0] / "NOTICE").write_text(
    notice_for("`src/variants_generated.rs`（机器生成）", "同目录的 `LICENSE`", GEN_HEAD),
    encoding="utf-8")
(PKGS[1] / "NOTICE").write_text(
    notice_for("依赖 `find42-core` 里的 `src/variants_generated.rs`"
               "（本包不含该文件，但产出的二进制内嵌了它）",
               "同目录的 `LICENSE`", GEN_HEAD),
    encoding="utf-8")
# MIT 正文逐字复制，无需改写
for pkg in PKGS:
    (pkg / "LICENSE").write_text((ROOT / "LICENSE").read_text(encoding="utf-8"), encoding="utf-8")

print(f"数据版本 unihan-database @ {rev}")
if rev.endswith("-dirty"):
    print("  ⚠️ 数据 checkout 不干净——生成物无法由该 commit 复现，别拿它入库")
for fn, n in per_field.items():
    print(f"  {fn.removesuffix('.txt'):<30} {n:>6} 条关系")
print(f"→ {OUT.relative_to(ROOT)}：{len(keys)} 个字符，"
      f"{sum(len(v) for v in clean.values())} 条展开关系，"
      f"{OUT.stat().st_size / 1024:.0f} KB")
