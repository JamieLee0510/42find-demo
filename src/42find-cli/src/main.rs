//! 42find 命令行入口：参数解析、遍历、输出格式、退出码。**不放检索逻辑**（见 `.42cog/cog.md`）。

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// 往 stderr 写一行诊断，**忽略写失败**。
///
/// 不能用 `eprintln!`——它遇到写失败会 **panic**（退 101），把「有个文件读不了」
/// 升级成整个进程崩掉。而诊断都写不出去的时候，唯一还能做的事就是接着干活：
/// 没有第二个地方可以报告「报告失败了」。写失败的**退出码**由标准输出那条线负责。
macro_rules! warn {
    ($($arg:tt)*) => {{
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

const HELP: &str = "\
42find — 中文友好的本地全文检索：簡繁互查、全半角归一。

用法：
    42find [选项] -- <查询词> [路径...]

选项：
    --column            输出里带上 1-based 字节列（与 rg --column 同单位）
    --glob <模式>       只搜匹配的文件，支持 `*.后缀` 或精确文件名
    -M, --max-columns <N>
                        整行超过 N 字节就截断并标注（默认 512，`0` 表示不截）
    -a, --text          把含 NUL 字节的文件也当文本搜（默认只报「二进制文件有命中」）
    -h, --help          显示本帮助

行为：
    查询词逐字展开成等价写法再扫原文——**不改语料**，所以命中的行列
    就是原文的行列。查「检索」命中「檢索」，查「query」命中「ｑｕｅｒｙ」。
    展开是非对称的：查「發」不会命中兄弟变体「髮」。
    变体表覆盖不到的字原样匹配，不报错也不丢弃。

输出：
    路径:行号[:字节列]:整行            （与 rg 同形，不自创）
    不带 --column：**一个匹配行只输出一行**（同行多处命中不重复报）
    带  --column：**每处命中输出一行**，由字节列区分同一行上的多处命中

    ⚠️ 整行默认截到 512 字节，截断处标注 `… [整行 N 字节，已截断至 M]`
    ——**这个标注是本工具自己的，不是 rg 的格式**（rg 的 `-M` 是整行换成一句话）。
    ⚠️ **截断取的是行首，所以命中若在上限之后，预览里看不到它**（`rg -M` 与
    `--max-columns-preview` 同样如此）。这类行本来也不是给人读的；真要看用 `--max-columns 0`。`--column` 下每处命中各写一遍整行，而同一行的命中数
    正比于行长——不截的话输出量对行长呈**平方**：一个 10 KB 的单行文件（minified
    js/json、单行 csv、老 Mac 的 CR 换行文本都是「一行」）查一个常见字，
    输出就是 106 MB。要原样整行用 `--max-columns 0`。

    含 NUL 字节的文件默认只报一行「二进制文件有命中」、不打印内容（与 rg 同）。
    整行输出会把被搜文件里的 ANSI / OSC 控制字节原样送进终端，而 OSC 序列能改
    终端标题、甚至写剪贴板。要照搜用 `--text`。
    ⚠️ NUL 只是**部分**防线：不含 NUL 的控制字节照样原样输出（与 rg 同）。
    ⚠️ `--text` 只放宽 NUL 这一条，**不放开非 UTF-8**——那类文件仍然跳过（见下）。

    文件开头的 UTF-8 BOM 会被剥掉再扫（与 rg 同），否则第一行的字节列会恒偏 3。

退出码：
    0 有命中 · 1 无命中 · 2 参数错误，或给定路径读不了／不是常规文件
    （空查询词是参数错误，不当作「匹配所有行」）
    （单个文件**不是 UTF-8** —— 跳过，不算错；**权限拒绝或 IO 错误** —— 报到 stderr 并退 2）
    （下游关掉管道，如 `| head` —— **尽早停止**，不 panic，**沿用本来该退的码**；与 rg 一致。
      注意两点：① 输出攒在缓冲里时，断管要到 flush 才发现，在那之前的文件照常读完；
      ② 就此停下之后尚未检查的文件不再计入退出码。所以这里可能是 0 / 1，
      也可能是 2 —— 前面已经有路径读不了时，那个错误优先）
    （标准输出真的写失败，如 EIO —— 报到 stderr 并退 2，不静默成 0。
      ⚠️ 两个像是「写失败」但到不了这里的：关掉 fd 1（Rust 启动时会把已关闭的 0/1/2
      补成 /dev/null，写不会失败）；磁盘／配额写满（SIGXFSZ 在 write 返回之前
      就把进程打死了，退 153））

遍历：
    递归时只收常规文件、不跟随符号链接（与 rg 默认一致）——FIFO / socket / 设备节点
    会让读取永久阻塞。命令行上显式给出的路径仍然跟随。
";

/// 整行输出的默认字节上限。
///
/// 为什么必须有个默认、而不是像 `rg -M` 那样默认不限：`rg` **没有**「逐命中 × 整行」
/// 这个组合（`rg -o` 只给片段、`rg` 不带 `-o` 一个匹配行只给一行），所以它不限也不会炸。
/// 我们两者兼有，输出量就成了 O(行长²)——实测 10 KB 单行文件查一个常见字，
/// `--column` 输出 **106 MB**（默认模式与 rg 都是 10 KB）。
/// 512 字节 ≈ 170 个汉字，比这更长的行本来也不是给人读的。
const DEFAULT_MAX_COLUMNS: usize = 512;

struct Args {
    column: bool,
    glob: Option<String>,
    /// 整行输出的字节上限，`0` 表示不截。
    max_columns: usize,
    /// 含 NUL 字节的文件也当文本搜。
    text: bool,
    query: String,
    paths: Vec<PathBuf>,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut column = false;
    let mut glob = None;
    let mut max_columns = DEFAULT_MAX_COLUMNS;
    let mut text = false;
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    let mut only_positional = false;

    // 用 args_os：`std::env::args()` 遇到非 UTF-8 参数会**直接 panic**，
    // 而帮助文本只承诺「文件不是 UTF-8 就跳过」，没承诺参数也能这样崩。
    // **只有选项与查询词要求 UTF-8**；路径一律留在 `OsString` 里。
    // 先前对所有 argv 都 `into_string()`，于是 ext4 上非 UTF-8 的文件名
    // **没法被点名指定**——而 `glob_matches` 那套 `OsStr` 字节比只救得到
    // `read_dir` 发现的文件。「只修一半」这个模式已经咬过两次了。
    let mut it = std::env::args_os().skip(1);
    while let Some(a_os) = it.next() {
        if only_positional {
            rest.push(a_os);
            continue;
        }
        // ⚠️ **在 `Option` 上分派，不要 `to_str().unwrap_or_default()`**。
        // 那个哨兵值把「不是文本」和「空串」混成一件事：非 UTF-8 的参数会变成 `""`，
        // 错过 `starts_with('-')` 那一支，被**静默重分类**成位置参数。
        // 同一条规则 `glob_matches` 里写着，但只落实在了触发它的那一处——
        // 评审发现关在了单点，没升成规则，于是同样的构造在这里活了下来。
        match a_os.to_str() {
            Some("-h" | "--help") => return Ok(None),
            Some("--column") => column = true,
            Some("--glob") => {
                let g = it.next().ok_or("--glob 后面要跟一个模式")?;
                glob = Some(g.into_string().map_err(|b| {
                    format!("--glob 的模式不是合法 UTF-8：{}", b.to_string_lossy())
                })?);
            }
            Some("-M" | "--max-columns") => {
                let v = it.next().ok_or("--max-columns 后面要跟一个字节数")?;
                let v = v.to_str().ok_or("--max-columns 的值不是合法 UTF-8")?;
                max_columns = v
                    .parse()
                    .map_err(|_| format!("--max-columns 要一个非负整数，收到：{v}"))?;
            }
            Some("-a" | "--text") => text = true,
            Some("--") => only_positional = true,
            Some(o) if o.starts_with('-') => return Err(format!("不认识的选项：{o}")),
            // 非 UTF-8 但以 `-` 开头：仍然是选项写错了，别当路径
            None if a_os.as_encoded_bytes().starts_with(b"-") => {
                return Err(format!("不认识的选项：{}", a_os.to_string_lossy()));
            }
            _ => rest.push(a_os),
        }
    }

    let mut rest = rest.into_iter();
    // 查询词必须是 UTF-8——它要被逐字符展开
    let query = rest
        .next()
        .ok_or("缺少查询词")?
        .into_string()
        .map_err(|b| format!("查询词不是合法 UTF-8：{}", b.to_string_lossy()))?;
    if query.is_empty() {
        // rg 对空模式是「匹配所有行」，这里语义相反。与其静默返回「无命中」，
        // 不如明说——沉默的相反语义比报错难查得多。
        return Err("查询词是空的（本工具不把空模式当作匹配所有行）".to_owned());
    }
    let paths: Vec<PathBuf> = rest.map(PathBuf::from).collect(); // OsString → PathBuf 无损
    let paths = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths
    };
    Ok(Some(Args {
        column,
        glob,
        max_columns,
        text,
        query,
        paths,
    }))
}

/// `*.后缀` 按后缀比，其余按文件名精确比。没给模式就全收。
fn glob_matches(glob: Option<&str>, path: &Path) -> bool {
    let Some(pat) = glob else { return true };
    // ⚠️ 按 `OsStr` 的字节比，**不要先 `to_str().unwrap_or_default()`**：
    // 那会把非 UTF-8 文件名当成空串，于是 `--glob "*.txt"` 下这类文件被**静默丢弃**；
    // 而不给 `--glob` 时它反而会被正常收入——同一条路径两种行为。
    // macOS 的 APFS 拒绝非 UTF-8 文件名，本机复现不了；Linux 的 ext4 上是真实场景。
    let Some(name) = path.file_name() else {
        return false;
    };
    match pat.strip_prefix('*') {
        Some(suffix) => name.as_encoded_bytes().ends_with(suffix.as_bytes()),
        None => name.as_encoded_bytes() == pat.as_bytes(),
    }
}

/// 这个路径是**用户显式给的**，还是**遍历发现的**？
///
/// 两条策略轴都由它决定，不再由 `is_dir()` 兼职回答：
/// ① 跟不跟随符号链接（显式跟随、遍历不跟随，与 `rg` 默认一致）
/// ② 非常规文件是**报错**还是**静默跳过**
///
/// ⚠️ 先前这两条轴硬编码在**两份**拷贝里，靠 `!path.is_dir()` 挑用哪一份——
/// 而 `is_dir()` 回答的是另一个问题，两者只是在调用图上碰巧重合。
/// 代价有记录：「第三轮只修了下面那个递归循环，于是 `42find -- 词 /tmp/pipe.txt`
/// 依然永久阻塞——同一个 bug 只修了一半」。**一个判据住两处，每次修改都留一个只修一半的坑。**
#[derive(Clone, Copy)]
enum Origin {
    /// 命令行上写出来的
    Explicit,
    /// `read_dir` 遍历发现的
    Discovered,
}

enum Kind {
    File,
    Dir,
    /// 符号链接、FIFO、socket、设备节点……
    Other,
}

/// 一次 `stat` 定性。先前每个文件要 stat 两到三次（循环里一次、`collect` 顶上一到两次）。
fn classify(path: &Path, origin: Origin) -> std::io::Result<Kind> {
    let md = match origin {
        Origin::Explicit => path.metadata()?,           // 跟随符号链接
        Origin::Discovered => path.symlink_metadata()?, // 判链接自身
    };
    Ok(if md.is_symlink() || !(md.is_file() || md.is_dir()) {
        // ⚠️ FIFO / socket / 设备节点：`read_to_string` 在它们上会**永久阻塞**。
        // 实测 `mkfifo` 造一个匹配 glob 的 `pipe.txt`，进程 6 秒不返回、只能强杀。
        Kind::Other
    } else if md.is_dir() {
        Kind::Dir
    } else {
        Kind::File
    })
}

/// 收集要搜的文件。返回是否一路顺利——有读不了的路径就是 `false`（决定退出码 2）。
fn collect(path: &Path, origin: Origin, glob: Option<&str>, out: &mut Vec<PathBuf>) -> bool {
    let kind = match classify(path, origin) {
        Ok(k) => k,
        Err(e) => {
            // 带上真实 errno——先前顶层先做一次 `exists()` 预检，
            // 于是「父目录没权限」会被报成「路径不存在」。
            warn!("42find: 读不了 {}：{e}", path.display());
            return false;
        }
    };

    // 一张决策表，每个 (来源, 类型) 格子只有一行——「只修一半」在这里无法表达
    match (origin, kind) {
        (_, Kind::File) => {
            if glob_matches(glob, path) {
                out.push(path.to_owned());
            }
            true
        }
        (_, Kind::Dir) => walk(path, glob, out),
        (Origin::Explicit, Kind::Other) => {
            warn!("42find: 不是常规文件，跳过：{}", path.display());
            false
        }
        (Origin::Discovered, Kind::Other) => true,
    }
}

/// 遍历一个目录。子项一律按 `Origin::Discovered` 处理。
fn walk(dir: &Path, glob: Option<&str>, out: &mut Vec<PathBuf>) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("42find: 读不了目录 {}：{e}", dir.display());
            return false;
        }
    };

    let mut ok = true;
    let mut children: Vec<PathBuf> = Vec::new();
    // 不能用 `entries.flatten()`——它会**静默丢掉** `ReadDir` 迭代途中的 `Err`
    for entry in entries {
        match entry {
            Ok(e) => children.push(e.path()),
            Err(e) => {
                warn!("42find: 枚举 {} 时出错：{e}", dir.display());
                ok = false;
            }
        }
    }
    children.sort();

    for child in children {
        ok &= collect(&child, Origin::Discovered, glob, out);
    }
    ok
}

/// 按 `--max-columns` 把整行截到字节上限。返回（要打印的那一段, 是否截断了）。
///
/// **必须切在字符边界上**——切进多字节字符中间会产出非法 UTF-8，
/// 而这个工具的语料按定义就是中文，每个字三字节，切中的概率是三分之二。
fn clip(line: &str, cap: usize) -> (&str, bool) {
    if cap == 0 || line.len() <= cap {
        return (line, false);
    }
    let mut end = cap;
    while end > 0 && !line.is_char_boundary(end) {
        end -= 1;
    }
    (&line[..end], true)
}

/// 写一处命中。**输出格式只在这里定义一次**，两种模式共用。
fn write_hit(
    out: &mut impl Write,
    name: &str,
    m: &find42_core::Match<'_>,
    column: bool,
    max_columns: usize,
) -> std::io::Result<()> {
    let (shown, clipped) = clip(m.line_text(), max_columns);
    if column {
        write!(out, "{}:{}:{}:{}", name, m.line(), m.col(), shown)?;
    } else {
        write!(out, "{}:{}:{}", name, m.line(), shown)?;
    }
    if clipped {
        // ⚠️ 报 `shown.len()` 而**不是** `max_columns`：`clip` 会往回退到字符边界，
        // 实际打出去的字节数多半小于上限。中文三字节一个字，512 的上限实际是 510——
        // 标注若报上限，就是在报一个没发生过的数。
        write!(
            out,
            "… [整行 {} 字节，已截断至 {}]",
            m.line_text().len(),
            shown.len()
        )?;
    }
    out.write_all(b"\n")
}

/// 把一个文件的命中写出去。
///
/// **不用 `println!`**——它遇到写失败**直接 panic**（退 101），而 `| head`、`| less`、
/// `| grep -m1` 是这类工具最常见的三种用法。写失败在这里如实返回，由调用方定退出码。
///
/// 输出的是**整行**，不是命中的那一段：只把用户自己敲的那个词还给他，
/// 等于还得再打开文件才知道那句话在说什么。形态对齐 `rg`，不自创。
///
/// `matches` 是**惰性**迭代器，不物化——单行大文件能产出上亿处命中。
/// `found` 用 `&mut` 传进来而不是返回：写失败时得保住「已经看到过命中」这个事实，
/// 否则一处命中写到一半断管道，退出码会从 0 掉成 1。
fn emit<'a>(
    out: &mut impl Write,
    file: &Path,
    args: &Args,
    matches: impl Iterator<Item = find42_core::Match<'a>>,
    found: &mut bool,
) -> std::io::Result<()> {
    // 路径在整个循环里是常量。`Path::display()` 每次都要走一遍 lossy UTF-8 分块，
    // 塞进 `write!` 就是每行重做一次——清理评审实测 57,400 行时占 4.5 ms，提出来省一半。
    // `to_string_lossy()` 对非法字节的处理与 `display()` 完全一致（都是 U+FFFD），输出逐字节不变。
    let name = file.to_string_lossy();

    if args.column {
        // 每处命中一行，由字节列区分同一行上的多处。
        // ⚠️ **这一支不能改成按行去重**：`scripts/bench.sh` 量 42find 走的正是 `--column`，
        // 34 条黄金查询集的分母就是逐命中数，去重会把召回与精确一起改掉。
        for m in matches {
            *found = true;
            write_hit(out, &name, &m, true, args.max_columns)?;
        }
        return Ok(());
    }

    // 不带 `--column` 就没有能区分同行两处命中的字段，于是原先会输出两行
    // **逐字节相同**的结果，看着像重复。`rg` 在这个模式下也是一个匹配行一行。
    //
    // 哨兵只跟**上一行**比，靠的是 `find42_core::search` 按 (行号, 字节列) 升序产出。
    // 那条保证写在 core 那头的文档与测试里（`命中按行号与字节列升序产出`）——
    // 判据住在提供它的一头，不住在用它的一头。
    let mut printed = 0usize; // 0 不是合法行号，拿来当「还没输出过任何一行」
    for m in matches {
        *found = true;
        if m.line() != printed {
            printed = m.line();
            write_hit(out, &name, &m, false, args.max_columns)?;
        }
    }
    Ok(())
}

/// 哪些写失败**算错**。唯一判据——告警那一侧与退出码那一侧共用这一条。
///
/// - `BrokenPipe`：下游关掉了管道（`| head` / `| less` / `| grep -m1`）。**不算错**，
///   停止输出、沿用本来该退的码，与 `rg` 一致。先前用 `println!`，这里是直接 panic 退 101。
/// - 其余写失败（磁盘满、EIO）：**算错**。不能静默成 0——那正是本项目在**读**那一侧
///   花力气消灭的「静默假阴性」，不能换到**写**这一侧再犯一遍。
///
/// ⚠️ **别把这个条件在调用点再写一遍。** 本仓吃过三次「一个判据住两处、每次只修一半」的亏
///（见上面 `Origin` 那段注释）。这里第一版就是那个形状：`exit_code` 判一次、
/// `main` 里决定要不要告警时又原地判了一次——两条清理视角同时点了名。
fn is_fatal_write_err(kind: std::io::ErrorKind) -> bool {
    // 为什么只排 `BrokenPipe` 一个：`Interrupted`（EINTR）到不了这里——
    // `write_all` 与 `BufWriter::flush_buf` 都在内部重试它，不会往上冒。
    // 别为了「看起来周全」把它也加进来，那是一条永不执行的分支。
    kind != std::io::ErrorKind::BrokenPipe
}

/// 退出码。返回 `u8` 而不是 `ExitCode`，是为了**能测**：后者既不能比较，也取不回里面的值。
fn exit_code(normal: u8, write_err: Option<std::io::ErrorKind>) -> u8 {
    if write_err.is_some_and(is_fatal_write_err) {
        2
    } else {
        normal
    }
}

/// 遍历、扫描、输出。返回（没有写失败时该退的码，标准输出的写失败）。
fn search_all(args: &Args, out: &mut impl Write) -> (u8, Option<std::io::Error>) {
    let exp = find42_core::expand(&args.query);
    let mut files = Vec::new();
    let mut paths_ok = true;
    for p in &args.paths {
        paths_ok &= collect(p, Origin::Explicit, args.glob.as_deref(), &mut files);
    }

    let mut found = false;
    let mut write_err = None;
    for file in &files {
        // 非 UTF-8 → 跳过（不算错）；权限拒绝 / IO 错误 → 报出来并计入退出码 2。
        // 两者原先走同一条 `continue`，于是 mode-000 的文件被静默当成「无命中」——
        // 零 stderr、退出码 1，是**静默假阴性**。rg 在这种情况下报 Permission denied 并退 2。
        let content = match std::fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => continue,
            Err(e) => {
                warn!("42find: 读不了 {}：{e}", file.display());
                paths_ok = false;
                continue;
            }
        };

        // 剥掉文件开头的 UTF-8 BOM 再扫（与 `rg` 同）。不剥的话第一行的字节列恒偏 3,
        // 而「`--column` 与 `rg --column` 同单位」是钉死的硬约束；BOM 还会随整行进 stdout。
        // Windows 上写的中文文本带 BOM 是常态，正是本工具的目标语料。
        let text = content.strip_prefix('\u{feff}').unwrap_or(&content);

        // 含 NUL 的按二进制处理：只报一行，不打印内容（与 `rg` 同，`--text` 可关）。
        //
        // ⚠️ 这条是**输出整行**新引入的暴露面。改之前打印的是查询词展开后匹配到的片段，
        // 字符集受用户自己敲的东西约束；改成整行之后，被搜文件里**任意**字节都出得来——
        // 包括 OSC 序列 `\x1b]0;…\a`（改终端标题）与 `\x1b]52;c;<base64>\a`（**写剪贴板**）。
        // 对一个「在别人的语料上跑检索」的工具，那是一条从被搜文件到终端状态的注入路径。
        //
        // ⚠️ **NUL 是部分防线，不是完整防线**：不含 NUL 的 ANSI/OSC 照样会原样输出。
        // 这里对齐的是 `rg` 的判据，不是「控制字节一律不出」——别把这段注释读成后者。
        if !args.text && text.contains('\0') {
            // 只问「有没有」，不物化命中——一个 200 MB 的单行文件能产出两亿个 `Match`，
            // 为了打印一句话把它们全建出来就是白白 OOM（GPT 系评审的 P2）。
            if !find42_core::has_match(&exp, text) {
                continue;
            }
            found = true;
            if let Err(e) = writeln!(
                out,
                "{}: 二进制文件有命中（含 NUL 字节，用 --text 照搜）",
                file.display()
            ) {
                write_err = Some(e);
                break;
            }
            continue;
        }

        if let Err(e) = emit(out, file, args, find42_core::search(&exp, text), &mut found) {
            // 下游已经走了，后面的文件不必再读
            write_err = Some(e);
            break;
        }
    }

    // 读不了的路径优先于「无命中」——它是错误，不该伪装成搜完了没找到
    let normal = if !paths_ok {
        2
    } else if found {
        0
    } else {
        1
    };
    (normal, write_err)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a, // `None` 是 -h / --help
        Err(e) => {
            warn!("42find: {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };

    // 一个缓冲、一次 flush、一条写失败判据——帮助文本与命中输出走同一条路。
    let stdout = std::io::stdout();
    // **终端上不缓冲，管道/文件上用 64 KiB 大块。** 与 `rg` 同策略。
    //
    // 容量 0 时 `BufWriter` 是直通的（每次写都 `buf.len() >= capacity`，直接交给内层），
    // 于是又落回 `Stdout` 自己的 `LineWriter`——逐行出。这样搜大目录时第一条命中立刻可见，
    // Ctrl-C 也不会把攒着的那一批算好的命中丢掉（信号终止不跑 destructor）。
    // 用容量 0 而不是分两条 `Box<dyn Write>` 分支，是为了**只有一个类型、一条 flush 判据**。
    //
    // 非终端时 64 KiB 而不是默认 8 KiB：块越小内层 `LineWriter` 被唤醒得越频繁，
    // 每次都要扫块尾找换行。清理评审实测（4.6 MB 输出）8K → 1.54 ms，64K → 0.40 ms，之后走平。
    let cap = if stdout.is_terminal() { 0 } else { 64 * 1024 };
    let mut out = std::io::BufWriter::with_capacity(cap, stdout.lock());

    let (normal, write_err) = match &args {
        // `42find --help | head -1` 一样会断管道，所以帮助文本也不能用 `print!`
        None => (0, out.write_all(HELP.as_bytes()).err()),
        Some(a) => search_all(a, &mut out),
    };

    // ⚠️ flush **必须显式查**：`BufWriter` 的 `Drop` 会**吞掉** flush 的错误，
    // 于是写失败被静默成退出码 0。全仓先前 0 处 flush，写这一侧的静默假阴性
    // 就是这么来的（`state/memory/20260907-真实使用是一层独立验证.md`）。
    // ⚠️ **无条件 flush，别写成 `write_err.or_else(|| out.flush().err())`。**
    // 那样只要前面报过一次错就不跑显式 flush，剩下的缓冲交给 `BufWriter::Drop` 去写——
    // 而 `Drop` 吞掉 flush 错误，正是这段代码上面刚点名批判的事。
    // 两个错都在时按**致命优先**合并：断管道之后真出了 EIO，不该被那个不算错的 EPIPE 盖住。
    let flush_err = out.flush().err();
    let write_err = match (write_err, flush_err) {
        (Some(a), Some(b)) => Some(if is_fatal_write_err(a.kind()) { a } else { b }),
        (a, b) => a.or(b),
    };

    if let Some(e) = &write_err
        && is_fatal_write_err(e.kind())
    {
        warn!("42find: 写标准输出失败：{e}");
    }
    ExitCode::from(exit_code(normal, write_err.map(|e| e.kind())))
}

/// 输出层的回归测试。与下面那组不同，**不依赖 unix**——`emit` 和 `exit_code` 都是纯逻辑。
#[cfg(test)]
mod output_tests {
    use super::*;

    /// 一个写就失败的 `Write`。造「下游关掉管道」和「坏 fd」两种写失败用。
    struct FailingWriter(std::io::ErrorKind);

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(self.0, "测试构造的写失败"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::new(self.0, "测试构造的写失败"))
        }
    }

    /// 跑一次 `emit`，返回（输出, 是否看到命中）。
    fn run(text: &str, q: &str, args: &Args) -> (String, bool) {
        let exp = find42_core::expand(q);
        let mut buf = Vec::new();
        let mut found = false;
        emit(
            &mut buf,
            Path::new("a.md"),
            args,
            find42_core::search(&exp, text),
            &mut found,
        )
        .expect("写进内存不会失败");
        (String::from_utf8(buf).expect("输出是 UTF-8"), found)
    }

    /// 只填 `emit` 会看的三个字段；其余给不影响输出的占位值。
    fn opts(column: bool, max_columns: usize) -> Args {
        Args {
            column,
            glob: None,
            max_columns,
            text: false,
            query: "检索".to_owned(),
            paths: Vec::new(),
        }
    }

    /// ★ 钉子：输出必须是**整行**，不是「把你自己敲的那个词还给你」。
    ///
    /// 这条是 issue #7 的第一症状：`.../01-research.md:1:判据` 得再打开文件才读得懂，
    /// 而 `rg` 给的是 `1:# 01 · 调研：判据先行，结论定位到行`。
    /// 十三个评审视角没有一个发现它——它们审的是代码对不对，不是这东西能不能用。
    #[test]
    fn 默认输出整行且同一行只报一次() {
        let text = "先归一再检索，还是先检索再归一。\n无关的一行\n";
        let (out, found) = run(text, "检索", &opts(false, 0));
        assert!(found);
        assert_eq!(
            out, "a.md:1:先归一再检索，还是先检索再归一。\n",
            "同一行两处命中，原先输出两行**逐字节相同**的结果"
        );
    }

    /// `--column` 逐命中一行，靠字节列区分同一行上的多处命中。
    ///
    /// ⚠️ 这个口径不能动：`scripts/bench.sh` 的 `engine_argv` 里 42find 走的正是
    /// `--column`，34 条黄金查询集量的就是逐命中数，去重会让召回/精确的分母变掉。
    #[test]
    fn column_模式逐命中且整行照给() {
        let text = "先归一再检索，还是先检索再归一。";
        let out = run(text, "检索", &opts(true, 0)).0;
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "两处命中必须能区分：{out}");
        assert_eq!(lines[0], format!("a.md:1:13:{text}"));
        assert_eq!(lines[1], format!("a.md:1:31:{text}"));
    }

    /// 多个文件各自从头去重——哨兵是 `emit` 的局部变量，不跨文件串场。
    ///
    /// 这条挡的是「把 `printed` 提到调用方复用」那种写法：两个文件的第 1 行都有命中时，
    /// 第二个文件的第 1 行会被当成重复**静默吞掉**，而单文件测试全绿。
    #[test]
    fn 去重不跨文件串场() {
        let text = "检索一次\n";
        let exp = find42_core::expand("检索");
        let mut buf = Vec::new();
        let mut found = false;
        for f in ["a.md", "b.md"] {
            emit(
                &mut buf,
                Path::new(f),
                &opts(false, 0),
                find42_core::search(&exp, text),
                &mut found,
            )
            .expect("写进内存不会失败");
        }
        assert_eq!(
            String::from_utf8(buf).expect("输出是 UTF-8"),
            "a.md:1:检索一次\nb.md:1:检索一次\n"
        );
    }

    /// 断管道如实返回，**不 panic**——先前 `println!` 在这里直接崩，退 101。
    #[test]
    fn 写失败如实返回而不panic() {
        let text = "检索\n";
        for kind in [
            std::io::ErrorKind::BrokenPipe,
            std::io::ErrorKind::PermissionDenied,
        ] {
            let exp = find42_core::expand("检索");
            let mut found = false;
            let e = emit(
                &mut FailingWriter(kind),
                Path::new("a.md"),
                &opts(false, 0),
                find42_core::search(&exp, text),
                &mut found,
            )
            .expect_err("写失败必须返回 Err，而不是 panic");
            assert_eq!(e.kind(), kind);
            assert!(
                found,
                "★ 写失败也不能把「已经看到过命中」这个事实丢掉——否则退出码会从 0 掉成 1"
            );
        }
    }

    /// ★ 钉子：`--column` 下不截断，输出量对行长呈**平方**。
    ///
    /// 这是 pr-ready 三个视角同时抓到的 P0：每处命中各写一遍整行，而同一行的命中数
    /// 正比于行长。实测 10 KB 单行文件查一个常见字 → 106 MB（默认模式与 rg 都是 10 KB）。
    /// 玩具语料整类藏住了它——`vault/truth/corpus` 最长的一行只有 78 字节。
    #[test]
    fn column_下不截断则输出量随行长呈平方() {
        let line = "检索".repeat(200); // 1200 字节、200 处命中
        let n = |cap| run(&line, "检索", &opts(true, cap)).0.len();
        let unbounded = n(0);
        let capped = n(512);
        assert!(
            unbounded > 200 * 1200,
            "不截断时应当是 O(行长²)：{unbounded} 字节"
        );
        assert!(
            capped < unbounded / 2,
            "上限必须真的把它压下来：{capped} vs {unbounded}"
        );
    }

    /// 截断必须切在**字符边界**上——切进多字节字符中间会产出非法 UTF-8。
    /// 语料按定义是中文，每字三字节，随手切中的概率是三分之二。
    #[test]
    fn 截断切在字符边界且带标注() {
        let line = "检索".repeat(10); // 60 字节
        // 7 不是 3 的倍数，必然要往回退
        let out = run(&line, "检索", &opts(false, 7)).0;
        assert!(
            out.contains("… [整行 60 字节，已截断至 6]"),
            "标注要报**实际打出去的**字节数（回退到边界后是 6，不是上限 7）：{out}"
        );
        assert!(out.starts_with("a.md:1:检索"), "6 字节处退到边界：{out}");
    }

    /// `clip` 的边界：不截、正好、要回退、上限 0（不限）、上限小于一个字符。
    #[test]
    fn clip的边界() {
        assert_eq!(clip("检索", 0), ("检索", false), "0 表示不截");
        assert_eq!(clip("检索", 99), ("检索", false), "短于上限不截");
        assert_eq!(clip("检索", 6), ("检索", false), "正好等于上限不截");
        assert_eq!(clip("检索", 5), ("检", true), "回退到字符边界");
        assert_eq!(clip("检索", 1), ("", true), "上限小于一个字符时给空串");
        assert_eq!(clip("", 5), ("", false));
    }

    /// 退出码真值表。断管道沿用原码，其余写失败一律 2。
    #[test]
    fn 退出码_断管道沿用原码_其余写失败退2() {
        use std::io::ErrorKind::{BrokenPipe, PermissionDenied};
        for normal in [0, 1, 2] {
            assert_eq!(exit_code(normal, None), normal, "没写失败就照原样");
            assert_eq!(
                exit_code(normal, Some(BrokenPipe)),
                normal,
                "`| head` 关掉管道不算错，与 rg 一致"
            );
            assert_eq!(
                exit_code(normal, Some(PermissionDenied)),
                2,
                "真的写失败不许被静默成 0"
            );
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// 临时目录脚手架。**靠 `Drop` 清理**——先前四个测试各抄一遍「断言在前、清理在后」，
    /// 任一断言 panic 就把目录连同 unix socket 一起留在 `/tmp` 里积攒。
    /// 零依赖约束下没有 `tempfile` 可用，所以更该只写一份。
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("42find-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).expect("建临时目录");
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn write(&self, name: &str, body: &str) -> PathBuf {
            let p = self.0.join(name);
            std::fs::write(&p, body).expect("写文件");
            p
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// 造一个自己指向父目录的符号链接环，确认 `collect` 不会把同一个文件反复收进来。
    ///
    /// 这是评审抓到的 P1：改之前，环下同一处命中被报 **32 次**（`rg` 报 1 次）。
    /// 表现不是卡死——macOS 的 `PATH_MAX` 会让路径涨到千余字符后 `read_dir` 失败，
    /// 于是变成**静默重复**，比卡住更隐蔽。
    #[test]
    fn 符号链接环不会重复收集() {
        let t = TempDir::new("symlink");
        let sub = t.path().join("d");
        std::fs::create_dir_all(&sub).expect("建子目录");
        std::fs::write(sub.join("x.txt"), "检索\n").expect("写文件");
        std::os::unix::fs::symlink("..", sub.join("loop")).expect("建符号链接");

        let mut files = Vec::new();
        let ok = collect(t.path(), Origin::Explicit, Some("*.txt"), &mut files);

        assert!(ok, "正常目录不该报读取失败");
        assert_eq!(
            files.len(),
            1,
            "环下同一个文件被收了 {} 次：{files:?}",
            files.len()
        );
    }

    /// 非常规文件（socket / FIFO / 设备节点）必须跳过——`read_to_string` 在它们上会**永久阻塞**。
    ///
    /// 用 `UnixListener` 造 socket，因为 std 没有 `mkfifo`，而造 FIFO 就得引依赖
    /// （`find42-cli` 也该保持零第三方依赖）。两者走的是同一条判断。
    #[test]
    fn 非常规文件被跳过() {
        let t = TempDir::new("sock");
        t.write("real.txt", "检索\n");
        let listener =
            std::os::unix::net::UnixListener::bind(t.path().join("s.txt")).expect("建 socket");

        let mut files = Vec::new();
        collect(t.path(), Origin::Explicit, Some("*.txt"), &mut files);
        drop(listener);

        assert_eq!(files.len(), 1, "socket 不该被当成待搜文件：{files:?}");
        assert!(files[0].ends_with("real.txt"));
    }

    /// **显式**给出的非常规文件也要拒绝，不只是递归时。
    ///
    /// 第三轮只修了递归循环，`42find -- 词 /tmp/pipe.txt` 依然永久阻塞——
    /// 同一个 bug 只修了一半，是换一条谱系评审才抓出来的。
    #[test]
    fn 显式给出的非常规文件被拒绝() {
        let t = TempDir::new("explicit");
        let sock = t.path().join("s.txt");
        let listener = std::os::unix::net::UnixListener::bind(&sock).expect("建 socket");

        let mut files = Vec::new();
        let ok = collect(&sock, Origin::Explicit, None, &mut files);
        drop(listener);

        assert!(!ok, "显式给一个不可搜的路径，应报失败（退出码 2）");
        assert!(files.is_empty(), "非常规文件不该进待搜列表：{files:?}");
    }

    /// 遍历发现的符号链接不跟随，且链接本身不当文件搜。
    #[test]
    fn 符号链接本身不被当成待搜文件() {
        let t = TempDir::new("symlink2");
        t.write("real.txt", "检索\n");
        std::os::unix::fs::symlink("real.txt", t.path().join("alias.txt")).expect("建符号链接");

        let mut files = Vec::new();
        collect(t.path(), Origin::Explicit, Some("*.txt"), &mut files);

        assert_eq!(files.len(), 1, "链接指向的文件不该被搜两遍：{files:?}");
        assert!(files[0].ends_with("real.txt"));
    }

    /// 遍历发现的非常规文件**静默跳过**，显式给出的**报错**——两条策略轴由 `Origin` 决定，
    /// 不再由 `is_dir()` 兼职回答。
    #[test]
    fn 来源决定非常规文件是报错还是静默跳过() {
        let t = TempDir::new("origin");
        t.write("real.txt", "检索\n");
        let sock = t.path().join("s.txt");
        let listener = std::os::unix::net::UnixListener::bind(&sock).expect("建 socket");

        let mut a = Vec::new();
        let discovered_ok = collect(t.path(), Origin::Explicit, Some("*.txt"), &mut a);
        let mut b = Vec::new();
        let explicit_ok = collect(&sock, Origin::Explicit, None, &mut b);
        drop(listener);

        assert!(discovered_ok, "遍历中遇到 socket 应静默跳过，不影响退出码");
        assert!(!explicit_ok, "显式给 socket 应报失败");
    }
}
