# 42find · 本地全文检索工具 · 状态板（给 AI · 跨会话唯一接续点）

> 开工先读 `CLAUDE.md` + **`.42cog/` 四份**（`intent` · `real` · `cog` · `meta`）+ 本文件 + `state/memory/MEMORY.md`。
> **非轮规则：每轮有效工作必更新本文件**（倒序追加，新的在上，带日期与 commit hash）。
> 每完成一个可命名的逻辑单元存一次；破坏性操作之前也存一次。这是给你自己留的后路，不是给别人看的历史。

> ## ⛳ 待办（2026-09-05 · 初始化留下的欠账）
> 1. ~~远程仓库~~ **已配**：`origin` → https://github.com/JamieLee0510/42find-demo（2026-09-05）。
>    CI 平台确认为 GitHub → `.github/workflows/ci.yml` 是对的，不用换 `.cnb.yml`。
>    **已推**：`main` → `origin/main`。⚠️ 远程走 **SSH**（`git@github.com:...`）——HTTPS 本机没凭据（没装 `gh`，keychain 里也没有），
>    换台机器或换用 HTTPS 时会卡在 `could not read Username`。
> 2. ~~Rust 工具链未安装~~ **已解决**：其实早就装了，只是 `~/.cargo/bin` 不在非登录 shell 的 PATH 里，
>    `command -v` 全查不到 → `check-tools.sh` 把装好的报成「一个都没装」。**误报比不检查更糟**。
>    已在 `scripts/check-tools.sh` 里分三态处理（在 PATH / 装了但不在 PATH / 真没装）。
>    要长期生效，在你的 shell 配置里加：`source "$HOME/.cargo/env"`。
> 3. ~~版本号是猜的~~ **已实测**：`rustc 1.89.0` / `cargo 1.89.0` / `rustfmt 1.8.0` / `clippy 0.1.89`，
>    与 `rust-toolchain.toml` 钉的 `1.89.0` 一致，`rustup show` 确认版本由该文件接管。**锁生效**。
> 4. ~~包名 ≠ 目录名待验~~ **已验，必须如此**：`cargo check` 直接报
>    `invalid character \`4\` in package name: the name cannot start with a digit`。
>    所以包名只能是 `find42-cli` / `find42-core`；目录名 `src/42find-*` 与命令名 `42find` 不受影响。
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

> ## ✅ 三道闸门本地全绿（2026-09-05）
> `cargo fmt --all -- --check` ✓ ｜ `cargo clippy --workspace --all-targets` ✓（`workspace.lints.clippy.all = deny`）｜
> `cargo test --workspace` ✓（0 个测试——作品还没开始，闸门先立着）。
> `target/` 已加进 `.gitignore`；**`Cargo.lock` 入库**（出二进制的项目，锁文件是版本锁定的另一半）。

> ## 🔧 gh 装法（2026-09-05 · 不走 brew）
> 官方 release 二进制 → `~/.local/bin/gh`（该路径已在 PATH 里，无需 sudo）：
> `gh 2.100.0`，下载后核过官方 `checksums.txt`，SHA256 一致；man page 进 `~/.local/share/man/man1/`。
> **升级就是重跑同样三步**（下载新 tag → 核校验和 → 覆盖 `~/.local/bin/gh`）。
> ⚠️ `scripts/check-tools.sh` 里 gh 的安装提示仍写着 `brew install gh`——那是通用底座段的默认写法，本机没按它走。
> **还没登录**：`gh auth login` 是交互式的，要人自己跑。
