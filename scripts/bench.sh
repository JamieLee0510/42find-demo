#!/usr/bin/env bash
# 在固定语料上量三个数：召回 · 精确 · 延迟。
#
# ⚠️ **延迟那一栏跨引擎不可直接比。** rg 走 `-o`（只输出命中片段），42find 输出**整行**
#    （issue #7 起），两边写出的字节数不再同量级，时间里混进了纯 I/O 量差。
#    这一栏只作**同引擎的前后自比**；正确性那两栏不受影响（awk 只取前三列）。
#
# 用法：bash scripts/bench.sh [rg|42find]     不给引擎则两个都跑（42find 未构建时自动跳过）
#
# ⚠️ rg 写死绝对路径：Claude Code 会注入一个同名 `rg` shell 函数，转给它内置的 14.1.1。
#    走 PATH 测出来的是**另一个版本**的基准线（exp001 已踩过这个坑）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS="$ROOT/vault/truth/corpus"
QUERIES="$ROOT/vault/truth/queries.tsv"
RG="$HOME/.cargo/bin/rg"
HYPERFINE="$HOME/.cargo/bin/hyperfine"
BIN="$ROOT/target/release/42find"
WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT

die() { printf '✗ %s\n' "$1" >&2; exit 1; }

[ -x "$RG" ] || die "找不到 $RG——基准线必须走真二进制，不能走 PATH 里那个 shell 函数"
[ -d "$CORPUS" ] || die "语料不在：$CORPUS"
[ -f "$QUERIES" ] || die "黄金查询集不在：$QUERIES"

# ⚠️ 不接管道取首行：`| head -1` 会给 rg 送 SIGPIPE，配上 `set -o pipefail` + `set -e`
#    就把整个脚本杀了（见 state/memory/20260906-管道退出码.md）。
RG_VER="$("$RG" --version)"; RG_VER="${RG_VER%%$'\n'*}"
printf '引擎基线：%s\n' "$RG_VER"
case "$RG_VER" in
  "ripgrep 15.2.0"*) ;;
  *) printf '⚠️  rg 版本与 exp001 基线（15.2.0）不一致，跨版本数字不可直接比\n' >&2 ;;
esac

# 语料一变，期望值就过期——.42cog/cog.md 已写死：三件不同步，召回率就是假的
CORPUS_SUM="$(cat "$CORPUS"/*.txt | shasum -a 256 | cut -c1-12)"
RECORDED="$(sed -n 's/^# corpus-sum: //p' "$QUERIES" || true)"
if [ -n "$RECORDED" ] && [ "$RECORDED" != "$CORPUS_SUM" ]; then
  printf '⚠️  语料已变（%s → %s）：queries.tsv 的**全部**期望值必须重新过人眼\n' "$RECORDED" "$CORPUS_SUM" >&2
  printf '   过完人眼后，把下面这行原样替换 queries.tsv 里的同名行（别自己复现算法——错过一次了）：\n' >&2
  printf '   # corpus-sum: %s\n' "$CORPUS_SUM" >&2
fi

# ⚠️ **引擎命令只在这里写一遍。** 先前同一条命令在本文件里写了三遍，
# 而第三份（延迟那处）已经漂移——丢了 `--with-filename`、也没有 `cd` + `.`，
# 于是**延迟量的和正确性量的根本不是同一条命令**。
# ⚠️ **别用 `eval`。** 先前是 `eval "$(engine_argv "$engine")" -- "$q" .`——
# eval 会把**已经展开过的** `"$q"` 再切一次词。黄金查询集里 `the quick` 这条含空格，
# 于是变成「查 the，在路径 quick 和 . 里找」，引擎报 `quick: No such file or directory` 退 2；
# 而当时那行末尾的 `2>/dev/null || true` 把它抹平成「这个查询 0 处命中」。
# **34 条里有一条一直在假测，两轮都没人发现**——正是这个脚本自己在量的那种静默假阴性。
# 改用数组：参数原样传，一次词都不多切。
engine_argv() {
  case "$1" in
    rg)     ENGINE_ARGV=("$RG" --column --no-heading --with-filename -o -F --glob '*.txt') ;;
    42find) ENGINE_ARGV=("$BIN" --column --glob '*.txt') ;;
    *)      die "不认识的引擎：$1" ;;
  esac
}

# 某查询下的原始命中（不去重），规范成 file:line:col
#
# ⚠️ **引擎的退出码要判，不能 `|| true` 抹平，stderr 也不能 `2>/dev/null` 丢掉。**
#    0 有命中、1 无命中都正常；**2 是读或写失败**——抹平之后它表现成「这个查询 0 处命中」，
#    召回率静静地掉，看着像检索 bug。issue #7 刚在二进制里消灭了这种静默假阴性，
#    别在 harness 外面一层原样复发（state/memory/20260907-真实使用是一层独立验证.md）。
engine_run() {
  local engine="$1" q="$2" rc=0
  engine_argv "$engine"
  (cd "$CORPUS" && "${ENGINE_ARGV[@]}" -- "$q" .) \
    > "$WORK/hits" 2> "$WORK/hits.err" || rc=$?
  if [ "$rc" -gt 1 ]; then
    sed 's/^/    /' "$WORK/hits.err" >&2
    # ⚠️ 查询词经 %s 传给 printf，别直接插进双引号串——`${q}」` 里那个 CJK 括号
    # 会被 bash 当成变量名的一部分，`set -u` 下直接 unbound variable（我刚踩了一次）。
    printf '✗ %s 查「%s」退出码 %s（读或写失败，非「无命中」），本轮数字作废\n' \
      "$engine" "$q" "$rc" >&2
    exit 1
  fi
  awk -F: 'NF>=3 { sub(/^\.\//,"",$1); print $1":"$2":"$3 }' "$WORK/hits"
}

bench_engine() {
  local engine="$1"
  local tp_all=0 exp_all=0 act_all=0
  printf '\n══ %s ══\n' "$engine"
  printf '%-12s %6s %6s %6s  %s\n' 查询 召回 精确 命中 说明

  while IFS=$'\t' read -r q expected note; do
    case "$q" in ''|'#'*) continue ;; esac

    if [ "$expected" = "-" ]; then : > "$WORK/exp"; else
      printf '%s\n' "$expected" | tr ',' '\n' | sort -u > "$WORK/exp"
    fi
    # sort -u 会把重复命中吃掉，所以另数一遍原始行数——符号链接环那类 bug
    # 只会表现为重复，不会让召回/精确变化（2026-09-06 评审指出，原先的注释写错了）
    engine_run "$engine" "$q" > "$WORK/raw"
    sort -u "$WORK/raw" > "$WORK/act"
    local n_raw n_uniq
    n_raw=$(wc -l < "$WORK/raw" | tr -d ' '); n_uniq=$(wc -l < "$WORK/act" | tr -d ' ')
    [ "$n_raw" -ne "$n_uniq" ] && printf '⚠️  「%s」有 %s 处重复命中（原始 %s / 去重 %s）\n' \
        "$q" "$((n_raw - n_uniq))" "$n_raw" "$n_uniq" >&2

    local n_exp n_act n_tp
    n_exp=$(wc -l < "$WORK/exp" | tr -d ' ')
    n_act=$(wc -l < "$WORK/act" | tr -d ' ')
    n_tp=$(comm -12 "$WORK/exp" "$WORK/act" | wc -l | tr -d ' ')

    tp_all=$((tp_all + n_tp)); exp_all=$((exp_all + n_exp)); act_all=$((act_all + n_act))
    printf '%-12s %5s%% %5s%% %3s/%-3s %s\n' "$q" \
      "$(pct "$n_tp" "$n_exp")" "$(pct "$n_tp" "$n_act")" "$n_tp" "$n_exp" "$note"
  done < "$QUERIES"

  printf '\n合计  召回 %s%%（%s/%s）  精确 %s%%（%s/%s）\n' \
    "$(pct "$tp_all" "$exp_all")" "$tp_all" "$exp_all" \
    "$(pct "$tp_all" "$act_all")" "$tp_all" "$act_all"
}

pct() { # $1/$2 → 整数百分比；分母为 0 记 0
  [ "${2:-0}" -eq 0 ] 2>/dev/null && { printf 0; return; }
  printf '%d' $(( $1 * 100 / $2 ))
}

bench_latency() {
  local engine="$1" cmd
  [ -x "$HYPERFINE" ] || { printf '\n⚠️  没有 %s，跳过延迟\n' "$HYPERFINE" >&2; return; }
  # 与正确性量的走同一条命令（同一个 engine_argv），不再各写各的
  engine_argv "$engine"
  cmd="cd $(printf '%q' "$CORPUS") && $(printf '%q ' "${ENGINE_ARGV[@]}") -- 检索 ."
  printf '\n── %s 延迟 ──\n' "$engine"
  # 判成败的命令不接管道：先落文件、单独判 rc，再看输出（同 memory：管道退出码）
  "$HYPERFINE" --warmup 3 --runs 50 --style basic "$cmd" > "$WORK/lat" 2>&1 \
    || { cat "$WORK/lat" >&2; die "hyperfine 跑失败"; }
  sed -n '/Time/,/Range/p' "$WORK/lat"
}

engines="${1:-}"
if [ -z "$engines" ]; then
  engines="rg"
  [ -x "$BIN" ] && engines="rg 42find" || printf '\n（%s 未构建，只跑 rg 基线）\n' "$BIN"
fi
for e in $engines; do bench_engine "$e"; bench_latency "$e"; done
