#!/usr/bin/env bash
# 在固定语料上量三个数：召回 · 精确 · 延迟。
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

RG_VER="$("$RG" --version | head -1)"
printf '引擎基线：%s\n' "$RG_VER"
case "$RG_VER" in
  "ripgrep 15.2.0"*) ;;
  *) printf '⚠️  rg 版本与 exp001 基线（15.2.0）不一致，跨版本数字不可直接比\n' >&2 ;;
esac

# 语料一变，期望值就过期——.42cog/cog.md 已写死：三件不同步，召回率就是假的
CORPUS_SUM="$(cat "$CORPUS"/*.txt | shasum -a 256 | cut -c1-12)"
RECORDED="$(sed -n 's/^# corpus-sum: //p' "$QUERIES" || true)"
if [ -n "$RECORDED" ] && [ "$RECORDED" != "$CORPUS_SUM" ]; then
  printf '⚠️  语料已变（%s → %s）：queries.tsv 的期望值必须重新过人眼\n' "$RECORDED" "$CORPUS_SUM" >&2
fi

# 把一个引擎在某查询下的实际命中，规范成排序去重的 file:line:col
run_engine() {
  local engine="$1" q="$2"
  case "$engine" in
    rg)     (cd "$CORPUS" && "$RG" --column --no-heading --with-filename -o -F --glob "*.txt" -- "$q" . 2>/dev/null || true) ;;
    42find) (cd "$CORPUS" && "$BIN" --column --glob "*.txt" -- "$q" . 2>/dev/null || true) ;;
  esac | awk -F: 'NF>=3 { sub(/^\.\//,"",$1); print $1":"$2":"$3 }' | sort -u
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
    run_engine "$engine" "$q" > "$WORK/act"

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
  case "$engine" in
    rg)     cmd="$RG --column --no-heading -o -F --glob *.txt -- 检索 $CORPUS" ;;
    42find) cmd="$BIN --column --glob *.txt -- 检索 $CORPUS" ;;
  esac
  printf '\n── %s 延迟 ──\n' "$engine"
  "$HYPERFINE" --warmup 3 --runs 50 --style basic "$cmd" 2>&1 | sed -n '/Time/,/Range/p'
}

engines="${1:-}"
if [ -z "$engines" ]; then
  engines="rg"
  [ -x "$BIN" ] && engines="rg 42find" || printf '\n（%s 未构建，只跑 rg 基线）\n' "$BIN"
fi
for e in $engines; do bench_engine "$e"; bench_latency "$e"; done
