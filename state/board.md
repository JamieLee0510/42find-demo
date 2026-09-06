# 42find · 本地全文检索工具 · 状态板（给 AI · 跨会话唯一接续点）

> 开工先读 `CLAUDE.md` + **`.42cog/` 四份**（`intent` · `real` · `cog` · `meta`）+ 本文件 + `state/memory/MEMORY.md`。
> **非轮规则：每轮有效工作必更新本文件**（倒序追加，新的在上，带日期与 commit hash）。
> 每完成一个可命名的逻辑单元存一次；破坏性操作之前也存一次。这是给你自己留的后路，不是给别人看的历史。

> ## ⛳ 待你确认（2026-09-06）
> **`LICENSE` 的版权人是我按 `git config user.name` 填的**：`Copyright (c) 2026 Jamie Lee`。
> **版权人是 AI 不该替人主张的东西**——请确认或改掉。
> （评审第三轮抓到：我在对话与提交信息里说「已标注需人确认」，但**仓里找不到这个标注**。
> 说过的话要落在仓里才算数，这条就是补落地的。）
>
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

> ## 🔒 main 的闸门补齐（2026-09-05）
> ruleset `main branch protection`（id 22319680）原本只有 `deletion` / `non_fast_forward` / `pull_request(approvals=1)`——
> **没有 required_status_checks，意思是 CI 红着也能合**。前面五次全绿是巧合不是保证。
> 已补：`required_status_checks → gate`（GitHub Actions app id 15368），`strict=false`
> （不强制分支先跟上 main；单人仓开 true 会天天 rebase，收益不抵摩擦）。
>
> **⚠️ 仍未解决：PR 的发起身份。** `gh` 用的是本人 token，本地会话开的 PR 必然署 `JamieLee0510`，
> 而 GitHub **禁止 self-approve**（产品层面，没有开关），`bypass_actors` 也是空的 →
> **本地开的 PR 现在合不掉**（PR #1 状态：`MERGEABLE` 但 `BLOCKED` / `REVIEW_REQUIRED`）。
> 解法：装 Claude GitHub App（`/install-github-app`，接 `anthropics/claude-code-action@v1`），
> PR 署 `claude[bot]`，人来 approve。**代价**：活儿从本地会话搬到 GitHub Actions（`@claude` 触发）。
> 要让本地会话也能开 bot 署名的 PR，只有第二个 GitHub 账号 + 它自己的 PAT 这一条路。

> ## 🔓 审批与身份：改走 admin bypass（2026-09-05）
> **决定**：人给自己在 ruleset 里加了 `RepositoryRole(always)` 的 bypass，`current_user_can_bypass=always`。
> 于是「PR 必须由非本人发起才能 approve」这个约束不再需要绕——**单人仓，直接放行**。
>
> 随之移除的：`.github/workflows/claude.yml` 与 `claude-code-review.yml`（PR #2，**从未进入 main**，已关闭并删分支）。
> 记一笔它们当时的状态，省得以后重装再踩一遍：**生成出来是只读的**
> （`contents: read` / `pull-requests: read`），`claude-code-review` 在 PR #2 上跑过一轮 SUCCESS 但一个字没发——
> 要它能推分支、开 PR、发评论，必须改成 `contents: write` / `pull-requests: write` / `issues: write`。
> 仓库 secret `CLAUDE_CODE_OAUTH_TOKEN` **还留着**（workflow 没了，它现在没人用）。
>
> **闸门现状**：`main` 仍要求 PR + 1 approval + `gate` 绿，但**你可以 bypass**。
> 也就是说闸门对 AI 有效、对你无效——这是有意的，不是漏配。

> ## 🧪 exp001：rg 基准线实测，推翻了意向书里的一条（2026-09-05）
> 装齐 `ripgrep 15.2.0`（`cargo install`）与 `gitleaks 8.30.1`（官方 release + 核 checksum）。
> gitleaks 扫工作区与全部 6 个提交：**no leaks found**。
>
> `docs/experiments/exp001-rg-baseline-cjk/` 拿真 rg 跑了四类中文查询，结论：
> **簡繁不互查 ✅ 坐实 · 全半角不归一 ✅ 坐实 · 「詞在句中」❌ 假设被推翻**——
> rg 做子串匹配、根本不分词，句中命中毫无问题。
> **这一条要回写进 `.42cog/intent.md`**：切分是 42find 用索引后自己给自己挖的坑，
> 不是相对 rg 的优势点；真正能赢的是字符层面的归一。
>
> ⚠️ 坑：Claude Code 注入了一个同名 `rg` shell 函数（转给内置的 14.1.1），
> 交互式敲有输出、脚本里查不到。**测基准必须走 `~/.cargo/bin/rg`。**
> `scripts/check-tools.sh` 的 PATH 补丁已前移到所有 check 之前（先前只覆盖了「本系统专属」那一段）。
> 安装提示也改成本项目真实走的路子（rg / hyperfine → `cargo install`，gitleaks → 官方 release）。
>
> **现在只缺**：`hyperfine`（延迟那个数）与 `opencode`（换谱系评审，下一步要用）。

> ## 🧰 工具清单第一次全绿（2026-09-06）
> `bash scripts/check-tools.sh` → **「都齐了」，退出码 0**。补装的两件：
> - `hyperfine 1.20.0`（`cargo install hyperfine`）——延迟那个数从此量得出来
> - `opencode 1.18.29`（`npm install -g opencode-ai`）——换谱系评审的第二套装置。
>   ⚠️ 装之前核过：npm 上 `opencode-ai` 的 metadata **没有 repository/homepage/description**，
>   不能凭包名就装；去 `opencode.ai/docs` 确认了它确实是官方包名才装的。
>   （`check-tools.sh` 自己写着「只从官方或可信源装；装之前核一眼包名与维护状态」——这次是照着做的。）
>
> **MCP（`sequential-thinking` / `playwright`）：配置在，会话里没连上。**
> 二者都配在 `~/.claude.json` 的本项目 `mcpServers` 下，走 `npx` 现拉。实测两个包都能跑
> （`@playwright/mcp 0.0.80`；sequential-thinking 对 `initialize` 正常应答，version 2026.8.31），
> playwright 的浏览器早就下过（`~/Library/Caches/ms-playwright`，1.1G）。
> **MCP 只在会话启动那一刻连接**——所以要用它们得重开会话，装无可装。
> 另：本项目用不到 playwright（42find 是本地命令行工具，不碰浏览器）。

> ## ✍️ exp001 结论回写完毕（2026-09-06）
> 上一轮只改了收敛方向那一句，剩下两处矛盾这轮补齐：
> ① `.42cog/intent.md` 第 3 行副标题还挂着被推翻的「詞在句中」——已换成「簡繁互查、全半角归一」。
> ② exp001 readme 末尾用「真难题①②」指代意向书，**编号对不上**（意向书①是索引一致性、②才是切分，
> 「字符归一」根本不在那张表上）。已改成按内容指代，不再用编号。
> 同时在意向书「真难题」第 2 条下补一行实测锚点，并写明**字符归一不是难题、是收敛方向的本体**
> （基础映射就是查表；难的只有繁简一对多、异体字、日文假名那截尾巴，跟着第 2 条一起解）。
>
> ⚠️ **本文件的顺序与自己写的规则相反**：抬头写「倒序追加，新的在上」，实际是最新的在最下面。
> 这轮按现状追加在末尾，没有擅自重排——要不要按规则翻过来，你定。
>
> **下一步**：`src/` 还是两个空壳 crate、0 个测试，验证闭环缺**固定语料**与**黄金查询集**两个必需件
> （exp001 那四条查询是第一批素材，归宿 `vault/truth/`）。第一刀切在 `42find-core` 的字符归一层。

> ## 🚀 issue #4 发车：字符归一第一刀（2026-09-06 · `fdd07cd`）
> **仓里原本一条 issue 都没有**，`dev-launch` 的入口是 issue，所以先按 board 的「下一步」开了
> [#4](https://github.com/JamieLee0510/42find-demo/issues/4)，再走调研 → 分流 → 拍板 → 实现。
> 分支 `feat/issue-4-corpus` → `-golden-set` → `-normalize`（三个小 PR 成链）。**未推送**。
>
> **两个数**（`bash scripts/bench.sh`）：
> `rg 15.2.0` 召回 **50%**（26/51）· 精确 100% · 6.9 ms ｜ `42find` 召回 **100%**（51/51）· 精确 100% · 1.3 ms。
> ⚠️ **延迟那个数不作性能结论**——871 字节语料证明不了什么，只能作同语料前后对比。
> ⚠️ **召回 100% 有自证成分**——期望值由同一套展开标准生成，验的是「实现与标准一致」；
> 独立的那一半是 rg 在同一套语料上只有 50%，说明语料确实有区分度。
>
> **拍板了三条**：① 归一形态 = **查询扩展**（不动语料，查询词逐字展开成等价写法再扫，
> 偏移天然精确、精确率不掉）② 等价表 = **手写小表**先跑通，零外部依赖 ③ 评审链现在修。
>
> **调研排除了两个 crate，理由要记住**：`zhconv` 是 **GPL-2.0**（本 workspace 是 MIT，会传染整个二进制）；
> `opencc-rust` / `opencc` 是 **C 绑定**（要 libopencc，直接毁掉「能装到别人机器上」这个前提）。
>
> **一个设计错误，自己抓出来的**：最初想做「等价类」。等价关系天然**对称且传递**——
> 写下 `发≡發` 与 `发≡髮` 就隐含 `發≡髮`，查「發」会命中「髮」，
> 靠查询扩展避开的假阳性从后门溜回来。改成**「规范形 → 变体集」的非对称展开**，
> 变体只回连规范形、不连兄弟变体。两条钉子测试盯着这件事。
>
> **待你拍板（都在 issue #4 评论区）**：
> ① 语料与黄金查询集的**标准归人**，AI 出的是初稿，等你过目。
> ② 意向书「不做什么」写着排除「无索引的纯正则全盘扫描」。本刀的扫描**是参照实现不是产品形态**
>    （定义「归一之后正确的召回长什么样」，是日后上索引不许退的尺子），建议这样写进意向书。
> ③ 意向书判据只有召回与延迟两个数。查询扩展让**精确率**成为守得住的承诺，建议补成第三个数。
>
> ## 🔧 评审谱系：三条链全躺，§7 卡住（2026-09-06）
> 配置落在 `.claude/dev-launch.review.md`（已 gitignore，因机器而异）。
> - `codex`：**两层问题**。① npm 缓存有截断条目——registry 说 186 MB，落盘只有 3.9 MB；
>   `npm cache verify` 不够，要 `cache clean --force` + 卸载重装，180 MB 二进制才真落盘。
>   ② 落盘之后**又被 macOS XProtect 删了**（17:11 装上，17:18 目录已空；本机无 MDM 无 EDR）。
>   **AI 修不了第二层**——要你在「隐私与安全性」里放行，或换签名分发渠道。
> - `gemini`：Google 停止免费层对该客户端的支持（`UNSUPPORTED_CLIENT`，让迁 Antigravity）。
> - `opencode` 可跑，但凭据只有 OpenAI + Anthropic——**Anthropic 是作者谱系，不能自审**。
>
> → 非作者谱系只剩 OpenAI 一条。按 §7「双谱系不可降级为单评审」**停在这里**。
>
> ⚠️ 本轮踩的那个「假绿灯」已沉淀成记忆：`state/memory/20260906-管道退出码.md`。

> ## 🔎 research/001：变体表数据源（2026-09-06 · `66e2412`）
> 人改了出发点：**从本地检索工具出发，MIT 优先**。全文 `docs/research/001_variant-table-source/`。
>
> **命门 2、3 已实现，命门 1 是 P0。** 关键发现：`irg-kvariants`（charabia/Meilisearch 在用）
> 的**数据层就是「字符 → 变体」关系表**——`KVariant{source, classification, destination}` +
> `KVariantClass{Simplified, Old, Wrong, SementicVariant, Equal}`，多对一，
> **按 destination 反转就得到「规范形 → 变体集」**，还自带分类能控哪几类参与展开。
> 我起草命门时判断「现成的全是转换器，给不出等价关系」——**错在只看了 `convert()` 那一层**。
>
> **但授权链是断的**：crate 元数据写 MIT，`irg-kvariants/` 目录没有 LICENSE，
> README 说数据来自 `hfhchan/irg` 的 `kVariants.md`，**那个仓看不到 LICENSE**。
> 出路：换 **Unihan 一手**（Unicode License V3，声明放随附文档即可，与 MIT 无摩擦）。
>
> **「MIT 优先」几乎没花代价**——本地检索这条线上 MIT 就是主流
> （tantivy · charabia · lindera · jieba-rs · cang-jie · character_converter 全 MIT；ripgrep 是 Unlicense OR MIT）。
>
> **最强反对依据**：Lucene（`ICUTransformFilter` + `CJKWidthFilter`）与 Meilisearch 都做**折叠**，
> **没有一家做查询扩展**；且折叠在索引期更便宜（一个 term vs 多 term OR）。
> 我们仍走查询扩展（偏移精确 + 非对称精确率），但**站在少数派一边，欠着一笔索引期的账**。
>
> **四个拍板点已列未代填**（见 `decision.md`）：数据源三选一 · 哪几类变体参与展开 ·
> 索引期那笔账现在还是以后还 · 精确率补不补成第三个数。
> **仍未补齐的那一处**：Unihan 那两个字段是 provisional，**真实语料覆盖率没有数**——该跑不该查，
> 等 `vault/raw/` 放进真语料。

> ## 📐 plan/001·002 与 issue #5（2026-09-06）
> `docs/plan/001` 定稿四个技术选型（**为什么这样设计**）；`docs/plan/002` 是换 Unihan 的执行蓝图
> （**将要怎么做** + 验收段），据此开了 [issue #5](https://github.com/JamieLee0510/42find-demo/issues/5)。
> issue 只给指针不重述需求，并写明**不要重新调研**那四份真相源。
>
> **002 的核心约束是核出来的**：`.gitignore:12` 让 `resources/**` 不进仓 → `build.rs` **不能**读 Unihan 原始数据
> （别人 clone 完没跑 `clone.sh` 就 `cargo build` 必炸，CI 一样）。方案因此定为
> **一次性脚本生成 `.rs`、生成物入库**，`core` 保持零依赖。
>
> **PR-1 必须先合且单独合**：修 `expand.rs:67-71` 的 `if/else if` 短路。
> 手写表下它行为零变化、三个数零变化，是纯拆雷；混进换表就分不清是谁的错。
>
> ⚠️ **本地已积 12 个提交，一个都没推**（`git push` 按铁律 14 要先问你）。
> 所以 issue #5 里指向 `docs/plan/002` 的链接**现在是 404** —— 文件还在
> `feat/issue-4-normalize` 分支上，没进 `main`。推了就好。

> ## 🚀 issue #5 落地：手写小表 → Unihan 生成表（2026-09-06 · `1621e14`）
> `dev-launch` 走完 §1–§6。分支链 `feat/issue-5-expand-union` → `-gen-table` → `-golden-set`。**未推送。**
>
> **三个数**（`bash scripts/bench.sh`，34 条查询 / 115 处期望）：
> `rg 15.2.0` 召回 **50%**（58/115）· 精确 100% · 6.2 ms
> `42find` 召回 **99%**（114/115）· 精确 **100%** · 1.7 ms
> 唯一那个漏是 `干净→乾淨`（净→凈→淨 隔两跳），**与 exp002 预测一致，没修饰**。
> 旧那 15 条查询仍全部 100%，无退化。
>
> 变体表：**15,564 字 / 17,550 条关系 / 414 KB**，二进制 811K，release 构建 0.6s。
>
> **六条验收全过**：三道闸门各自 rc=0（10 测试）· 召回不退 · 精确 100% 且两条钉子绿 ·
> 延迟同量级 · **重跑生成器 `git diff` 为空**（连 `cargo fmt` 之后也稳）·
> **无 `resources/` 的干净 clone 能构建并跑通**。
>
> **两件值得记的**：
> ① **我踩了自己写的规则**：加语料后只算了新 20 条的期望值，没重算旧 15 条，
>    精确率 100%→83%，看着像 bug 其实是期望值过期。`queries.tsv` 头部已记死
>    「语料一改，**全部**期望值都要重算」。
> ② **一个设计简化让 PR-1 变得多余**：改成一张合并表后，PR-1 修的那个 `else if` 短路
>    从结构上就不存在了。PR-1 没白做（去重修正留着、验证了行为零变化），
>    但要记住：**换个设计能消掉的 bug，比修掉它更好。**
>
> **有意留的陷阱**：`資料庫`/`網路`/`軟體`/`發佈` 在语料里但不在期望里。谁哪天让 42find
> 命中它们，精确率就掉、闸门会抓住——不这么留，日后「修好」了反而没人发现。
>
> ⚠️ **§7 双谱系评审仍过不去**：`codex` 二进制又被 XProtect 删了（目录仍空）；
> `gemini` 免费层被停；`opencode` 只有 OpenAI + Anthropic，而 Anthropic 是作者谱系。
> 按技能「双谱系不可降级为单评审」**停在这里**。
> ⚠️ **本地已积 19 个提交，一个都没推。**

> ## 🔧 codex 的真因：我诊断错了两次（2026-09-06）
> 人问「codex CLI 为什么不能用」，回去核实，**前两次诊断都是错的**：
> ① 「npm 缓存截断」——半对，清完缓存二进制确实落盘，但仍跑不起来。
> ② 「macOS XProtect 在删它」——**错，是没证据的推断**（只凭「17:11 在、17:18 没了」）。
> ③ ✅ **真因：签名证书被吊销。** `spctl -a -vv -t execute` 报 `CSSMERR_TP_CERT_REVOKED`。
>    签名是真的（`Developer ID Application: OpenAI OpCo, LLC (2DC432GLL2)`，2026-04-25），
>    但证书已吊销，macOS 拒绝执行。
> **修法：装 latest。** 我全程在重装 `0.125.0`，而 npm latest 是 `0.153.4`——差 28 个小版本。
> `@latest` 装完立刻输出 `codex-cli 0.153.4`。
>
> **两条教训，都写进 `.claude/dev-launch.review.md` 了**：
> ① 二进制跑不起来，**先 `spctl -a -vv -t execute` 与 `codesign -dvvv` 各看一眼**，
>    再去猜安装器/缓存/杀毒。签名与吊销状态是一句话问得出的事实，比推断便宜得多。
> ② **别默认沿用现有版本号**——我一直在修一个上游早已修好的问题。
>
> **还差一步**：`codex exec` 报 401 `refresh_token_reused`（auth.json 是 5-09 的），
> 要人跑 `codex login`（交互式）。登录后 GPT 系这条链就通了，
> §7 只剩「第二条非 Anthropic 谱系」没着落。
