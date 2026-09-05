# 42find · 本地全文检索工具 · 状态板（给 AI · 跨会话唯一接续点）

> 开工先读 `CLAUDE.md` + **`.42cog/` 四份**（`intent` · `real` · `cog` · `meta`）+ 本文件 + `state/memory/MEMORY.md`。
> **非轮规则：每轮有效工作必更新本文件**（倒序追加，新的在上，带日期与 commit hash）。
> 每完成一个可命名的逻辑单元存一次；破坏性操作之前也存一次。这是给你自己留的后路，不是给别人看的历史。

> ## ⛳ 待办（2026-09-05 · 初始化留下的欠账）
> 1. **远程仓库还没配**——你说有地址，把它发我：`git remote add origin <地址>`。
>    `_build/ _tmp/ _archive/` 与所有忽略件**没有远端副本，本地一丢就没了**，这一步不能一直欠着。
>    另：CI 现在按 GitHub Actions 起草（`.github/workflows/ci.yml`）；如果远端不是 GitHub（例如 CNB），换成 `.cnb.yml`。
> 2. **Rust 工具链本机未安装**——`rustup / rustc / cargo` 全缺，`cargo fmt`、`cargo clippy` 同缺。
>    装：`brew install rustup && rustup-init`。装完跑 `bash scripts/check-tools.sh` 复验。
> 3. **`rust-toolchain.toml` 里的 `1.89.0` 是已知可用的下限，不是实测值**（初始化时本机没有 rustup）。
>    装好后 `rustc --version`，把它换成实测版本。
> 4. **包名 ≠ 目录名**：目录 `src/42find-cli`，包名 `find42-cli`（cargo 限制包名不以数字开头），命令名仍是 `42find`。
>    装好 cargo 后 `cargo metadata` 验一眼；若该限制不成立，可把包名改回 `42find-cli`。
> 5. 其余缺件（`rg` / `gh` / `gitleaks` / `opencode`）见 `bash scripts/check-tools.sh`。
>    **`opencode` 关系到下一步**：对抗性评审要换谱系，缺了就少一双不同来路的眼睛。

> ## 🌱 系统开仓（2026-09-05 第一轮）
> `aias-meta-init` 生成六组骨架：① `README.md` `CLAUDE.md` `.42cog/` `specs/` ② `vault/` `notes/` `resources/` ③ `skills/` `scripts/` `plugin.json` ④ `src/` ⑤ `state/` `docs/` ⑥ `_build/` `_tmp/` `_archive/`（忽略）。
> **收敛方向**（草稿，待人确认）：见 `.42cog/intent.md`——**那句话只有一份，别抄到这里来**。
> **下一步**：确认收敛方向 → `42find-research` 找依据、排真相源权重 → 回来改这一句。方向第一版粗是正常的，四步是循环。

> ## 🔧 初始化第三步 · 配置与扩展（2026-09-05）
> `scripts/check-tools.sh` 专属段已按 Rust workspace 填好（`STACK_EMPTY=0`）｜ 版本锁定 `rust-toolchain.toml` ｜
> 目录语义走生态原生位置：根 `Cargo.toml` 的 workspace（**不另造配置文件**）｜ 最小 CI `.github/workflows/ci.yml`：
> `cargo fmt --check` + `cargo clippy` + `cargo test` ｜ 编排清单写进 `skills/README.md`（不新开文件）。
