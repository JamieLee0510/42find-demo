//! 42find 命令行入口：参数解析、遍历、输出格式、退出码。**不放检索逻辑**（见 `.42cog/cog.md`）。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "\
42find — 中文友好的本地全文检索：簡繁互查、全半角归一。

用法：
    42find [选项] -- <查询词> [路径...]

选项：
    --column        输出里带上 1-based 字节列（与 rg --column 同单位）
    --glob <模式>   只搜匹配的文件，支持 `*.后缀` 或精确文件名
    -h, --help      显示本帮助

行为：
    查询词逐字展开成等价写法再扫原文——**不改语料**，所以命中的行列
    就是原文的行列。查「检索」命中「檢索」，查「query」命中「ｑｕｅｒｙ」。
    展开是非对称的：查「發」不会命中兄弟变体「髮」。
    变体表覆盖不到的字原样匹配，不报错也不丢弃。

输出：
    路径:行[:字节列]:命中的原文

退出码：
    0 有命中 · 1 无命中 · 2 参数错误，或给定路径读不了／不是常规文件
    （空查询词是参数错误，不当作「匹配所有行」）
    （单个文件**不是 UTF-8** —— 跳过，不算错；**权限拒绝或 IO 错误** —— 报到 stderr 并退 2）

遍历：
    递归时只收常规文件、不跟随符号链接（与 rg 默认一致）——FIFO / socket / 设备节点
    会让读取永久阻塞。命令行上显式给出的路径仍然跟随。
";

struct Args {
    column: bool,
    glob: Option<String>,
    query: String,
    paths: Vec<PathBuf>,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut column = false;
    let mut glob = None;
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
            eprintln!("42find: 读不了 {}：{e}", path.display());
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
            eprintln!("42find: 不是常规文件，跳过：{}", path.display());
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
            eprintln!("42find: 读不了目录 {}：{e}", dir.display());
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
                eprintln!("42find: 枚举 {} 时出错：{e}", dir.display());
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

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Some(a)) => a,
        Ok(None) => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("42find: {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };

    let exp = find42_core::expand(&args.query);
    let mut files = Vec::new();
    let mut ok = true;
    for p in &args.paths {
        ok &= collect(p, Origin::Explicit, args.glob.as_deref(), &mut files);
    }

    let mut found = false;
    for file in &files {
        // 非 UTF-8 → 跳过（不算错）；权限拒绝 / IO 错误 → 报出来并计入退出码 2。
        // 两者原先走同一条 `continue`，于是 mode-000 的文件被静默当成「无命中」——
        // 零 stderr、退出码 1，是**静默假阴性**。rg 在这种情况下报 Permission denied 并退 2。
        let text = match std::fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => continue,
            Err(e) => {
                eprintln!("42find: 读不了 {}：{e}", file.display());
                ok = false;
                continue;
            }
        };
        for m in find42_core::search(&exp, &text) {
            found = true;
            if args.column {
                println!("{}:{}:{}:{}", file.display(), m.line, m.col, m.text);
            } else {
                println!("{}:{}:{}", file.display(), m.line, m.text);
            }
        }
    }

    // 读不了的路径优先于「无命中」——它是错误，不该伪装成搜完了没找到
    if !ok {
        ExitCode::from(2)
    } else if found {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
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
