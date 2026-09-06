#!/usr/bin/env python3
"""exp002 · Unihan 变体表在真实查询词上的覆盖探针。

**真实依赖**：直接读 resources/unihan-database/ 的一手数据文件，不是手写的模拟表。
**展开规则**：与 src/42find-core/src/expand.rs 逐条一致（非对称：变体只回连规范形）。
"""
import re, pathlib, sys

UNIHAN = pathlib.Path("resources/unihan-database")
if not UNIHAN.is_dir():
    sys.exit("缺 resources/unihan-database——先跑 clone.sh")

def load(fn):
    # ⚠️ `scripts/gen-variants.py` 里有一份同形实现，且**两者已漂移**：
    # 这边 `m[src] = [...]`（后写覆盖），那边 `setdefault().extend()`（累加）。
    # 当前数据下结果相同；上游同字段同字符出两行时会分岔。改一边要同步另一边。
    m = {}
    for line in (UNIHAN / fn).read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        src = chr(int(parts[0].split()[0][2:], 16))
        tgts = [chr(int(c[2:], 16)) for c in re.findall(r"U\+[0-9A-F]+", parts[2])]
        m[src] = [t for t in tgts if t != src]      # 去掉 431 条自指（unihan-database#408）
    return m

SIMP = load("kSimplifiedVariant.txt")    # 繁 → 简（多对一）
TRAD = load("kTraditionalVariant.txt")   # 简 → 繁（一对多）
EXTRA = {}                               # 三个附加字段，模式 B 才启用
for fn in ("kSemanticVariant.txt", "kSpecializedSemanticVariant.txt", "kZVariant.txt"):
    for k, v in load(fn).items():
        EXTRA.setdefault(k, []).extend(v)

MODE = "A"   # A=只用两个主字段（当前实现）· B=+三个附加字段（仍一跳）· C=A 的两跳传递闭包

def expand_char(c):
    """与 expand.rs 同规则：规范形展开全部变体；变体只回连规范形，不连兄弟变体。"""
    out = [c]
    code = ord(c)
    if 0xFF01 <= code <= 0xFF5E:  out.append(chr(code - 0xFEE0))
    elif 0x21 <= code <= 0x7E:    out.append(chr(code + 0xFEE0))
    elif c == "　":           out.append(" ")
    elif c == " ":                out.append("　")
    # ⚠️ 必须并集，不能 if/elif：一个字可以同时是两个文件的 key
    # （如「裡」在 kTraditionalVariant 里指向自己、在 kSimplifiedVariant 里指向「里」）。
    # 自指条目被剥掉后 key 仍在，if/elif 会被前者短路，**静默吃掉反向映射**。
    # 并集不破坏非对称：expand(發) = {發} ∪ TRAD[發]{} ∪ SIMP[發]{发}，仍不含「髮」。
    out.extend(TRAD.get(c, []))                # 简体规范形 → 全部繁体变体
    out.extend(SIMP.get(c, []))                # 繁体变体 → 简体规范形
    if MODE in ("B", "D"):
        out.extend(EXTRA.get(c, []))
    if MODE in ("C", "D"):                     # 两跳：决策二明确拒绝的那一步
        for m in list(out):
            out.extend(TRAD.get(m, [])); out.extend(SIMP.get(m, []))
            if MODE == "D":
                out.extend(EXTRA.get(m, []))
    seen, uniq = set(), []
    for ch in out:
        if ch not in seen:
            seen.add(ch); uniq.append(ch)
    return uniq

def matches(query, target):
    """query 展开后能否逐位匹配 target（等长逐字比对，与 search.rs 的判定一致）。"""
    if len(query) != len(target):
        return False
    return all(t in expand_char(q) for q, t in zip(query, target))

# ── 样本：真实场景里会敲的查询词 ──
# 先定死再跑。**故意包含预期会失败的**：地区词（软件/軟體、网络/網路）不是字形问题，
# 意向书已划出范围，但真实用户一定会撞上，所以必须量出来而不是回避。
# ground truth 由人写：该词在一份繁体文档里实际的写法。两地写法不同的，两个都列。
SAMPLES = [
    ("检索",   ["檢索"],           "常规簡繁"),
    ("数据库", ["數據庫", "資料庫"], "两地写法不同"),
    ("网络",   ["網絡", "網路"],   "两地写法不同"),
    ("软件",   ["軟件", "軟體"],   "两地写法不同"),
    ("头发",   ["頭髮"],           "一对多：发→發/髮"),
    ("发布",   ["發布", "發佈"],   "一对多 + 布/佈"),
    ("干净",   ["乾淨"],           "一对多：干→幹/乾"),
    ("后台",   ["後台"],           "一对多：后→後/后"),
    ("里面",   ["裡面", "裏面"],   "一对多：里→裡/裏"),
    ("台湾",   ["臺灣", "台灣"],   "一对多：台→臺/台"),
    ("面条",   ["麵條"],           "一对多：面→麵/面"),
    ("系统",   ["系統"],           "一对多：系→係/繫/系"),
    ("用户",   ["用戶"],           "异体字：户/戶"),
    ("冲突",   ["衝突"],           "一对多：冲→沖/衝"),
    ("划分",   ["劃分"],           "一对多：划→劃/划"),
    ("复制",   ["複製"],           "一对多：复→復/複/覆"),
    ("历史",   ["歷史"],           "一对多：历→歷/曆"),
    ("制作",   ["製作"],           "一对多：制→製/制"),
    ("钟表",   ["鐘錶"],           "两字都一对多"),
    ("只有",   ["只有", "祇有"],   "只→隻/只"),
]

def run():
  rows = []
  for simp, trads, note in SAMPLES:
    hits = [(t, matches(simp, t)) for t in trads]
    back = [(t, matches(t, simp)) for t in trads]
    rows.append((simp, note, hits, back))
  return rows

def fmt(pairs):
    return " ".join(f"{t}{'✅' if ok else '❌'}" for t, ok in pairs)

MODES = [("A", "只用 kSimplified + kTraditional（当前实现）"),
         ("B", "A + kSemantic + kSpecializedSemantic + kZ（仍一跳）"),
         ("C", "A 的两跳传递闭包（决策二明确拒绝的那一步）"),
         ("D", "B + 两跳：能修到的上限，代价也最大")]

first = None
for mode, desc in MODES:
    globals()["MODE"] = mode
    rows = run()
    if first is None:
        first = rows
        print(f"{'查询':<8}{'→繁':<22}{'繁→简':<22}说明")
        print("-" * 74)
        for simp, note, hits, back in rows:
            print(f"{simp:<8}{fmt(hits):<22}{fmt(back):<22}{note}")
        print()
    fwd = [ok for _, _, h, _ in rows for _, ok in h]
    bwd = [ok for _, _, _, b in rows for _, ok in b]
    usable = sum(1 for _, _, h, _ in rows if any(ok for _, ok in h))
    # 代价：两条非对称钉子 + 平均展开集大小
    nail1 = "髮" in expand_char("發")
    nail2 = "發" in expand_char("髮")
    size = sum(len(expand_char(c)) for c in "检索归一发發髮里裡干净台面系复历制钟只用户") / 21
    print(f"模式 {mode} · {desc}")
    print(f"   简→繁 {sum(fwd)}/{len(fwd)}   繁→简 {sum(bwd)}/{len(bwd)}   词级可用 {usable}/{len(rows)}")
    print(f"   代价：钉子破了吗 expand(發)∋髮={nail1} expand(髮)∋發={nail2}   平均展开集 {size:.2f} 字")
    if mode != "A":
        base = {(s_, t) for s_, _, h, _ in first for t, ok in h if not ok}
        now = {(s_, t) for s_, _, h, _ in rows for t, ok in h if not ok}
        fixed = base - now
        print(f"   相对 A 修好了：{', '.join(t for _, t in sorted(fixed)) or '（一个都没有）'}")
    print()
