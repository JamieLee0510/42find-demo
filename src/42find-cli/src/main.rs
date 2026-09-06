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
    （单个文件读不出来或不是 UTF-8 —— 跳过，不算错，不影响退出码）

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
    let mut rest: Vec<String> = Vec::new();
    let mut only_positional = false;

    // 用 args_os：`std::env::args()` 遇到非 UTF-8 参数会**直接 panic**，
    // 而帮助文本只承诺「文件不是 UTF-8 就跳过」，没承诺参数也能这样崩。
    let mut it = std::env::args_os().skip(1).map(|a| {
        a.into_string()
            .map_err(|bad| format!("参数不是合法 UTF-8：{}", bad.to_string_lossy()))
    });
    while let Some(a) = it.next().transpose()? {
        if only_positional {
            rest.push(a);
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => return Ok(None),
            "--column" => column = true,
            "--glob" => glob = Some(it.next().transpose()?.ok_or("--glob 后面要跟一个模式")?),
            "--" => only_positional = true,
            other if other.starts_with('-') => return Err(format!("不认识的选项：{other}")),
            other => rest.push(other.to_owned()),
        }
    }

    let mut rest = rest.into_iter();
    let query = rest.next().ok_or("缺少查询词")?;
    if query.is_empty() {
        // rg 对空模式是「匹配所有行」，这里语义相反。与其静默返回「无命中」，
        // 不如明说——沉默的相反语义比报错难查得多。
        return Err("查询词是空的（本工具不把空模式当作匹配所有行）".to_owned());
    }
    let paths: Vec<PathBuf> = rest.map(PathBuf::from).collect();
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
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match pat.strip_prefix('*') {
        Some(suffix) => name.ends_with(suffix),
        None => name == pat,
    }
}

/// 收集要搜的文件。返回是否一路顺利——有读不了的目录就是 `false`（决定退出码 2）。
///
/// ⚠️ **递归时只收常规文件，且不跟随符号链接**（与 `rg` 默认一致）。跟随会让 `d/loop -> ..` 这类环
/// 把同一个文件反复收进来：实测一处命中被报 **32 次**（rg 报 1 次）。
/// macOS 的 `PATH_MAX` 会让路径涨到千余字符后 `read_dir` 失败，所以表现不是卡死，
/// 而是**静默重复**——更隐蔽，且会直接污染 `scripts/bench.sh` 的召回与精确。
/// 命令行上**显式给出**的路径仍然跟随（下面的 `is_dir()` 用的是跟随语义）。
fn collect(path: &Path, glob: Option<&str>, out: &mut Vec<PathBuf>) -> bool {
    if !path.is_dir() {
        // ⚠️ 顶层分支同样要挡住非常规文件。第三轮只修了下面那个递归循环，
        // 于是 `42find -- 词 /tmp/pipe.txt` 依然永久阻塞——**同一个 bug 只修了一半**。
        // （显式路径走跟随语义，所以这里用 `metadata()` 而非 `symlink_metadata()`。）
        match path.metadata() {
            Ok(md) if md.is_file() => {}
            Ok(_) => {
                eprintln!("42find: 不是常规文件，跳过：{}", path.display());
                return false;
            }
            Err(e) => {
                eprintln!("42find: 读不了 {}：{e}", path.display());
                return false;
            }
        }
        if glob_matches(glob, path) {
            out.push(path.to_owned());
        }
        return true;
    }

    let Ok(entries) = std::fs::read_dir(path) else {
        eprintln!("42find: 读不了目录：{}", path.display());
        return false;
    };
    // 不能用 `entries.flatten()`——它会**静默丢掉** ReadDir 迭代途中的 Err，
    // 于是「目录读不了 → rc=2 + stderr」这条契约在部分枚举错误下失效。
    let mut ok = true;
    let mut children: Vec<PathBuf> = Vec::new();
    for entry in entries {
        match entry {
            Ok(e) => children.push(e.path()),
            Err(e) => {
                eprintln!("42find: 枚举 {} 时出错：{e}", path.display());
                ok = false;
            }
        }
    }
    children.sort();
    for child in children {
        // 判的是链接自身，不是它指向的东西
        match std::fs::symlink_metadata(&child) {
            // 符号链接一律不进（递归不跟随）
            Ok(md) if md.is_symlink() => continue,
            // ⚠️ 只收**常规文件**与目录。FIFO / socket / 设备节点匹配到 glob 时，
            // 后面的 `read_to_string` 会**永久阻塞**——实测拿 mkfifo 造一个 `pipe.txt`，
            // 进程 6 秒不返回，只能强杀。
            Ok(md) if !md.is_file() && !md.is_dir() => continue,
            Ok(_) => {}
            Err(e) => {
                eprintln!("42find: 读不了 {}：{e}", child.display());
                ok = false;
                continue;
            }
        }
        ok &= collect(&child, glob, out);
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
        // 显式给的路径不存在，是参数错误，不是「无命中」
        if !p.exists() {
            eprintln!("42find: 路径不存在：{}", p.display());
            ok = false;
            continue;
        }
        ok &= collect(p, args.glob.as_deref(), &mut files);
    }

    let mut found = false;
    for file in &files {
        // 读不出来或不是 UTF-8 的，跳过——不是错误，只是不参与检索
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
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

    /// 造一个自己指向父目录的符号链接环，确认 `collect` 不会把同一个文件反复收进来。
    ///
    /// 这是评审抓到的 P1：改之前，环下同一处命中被报 **32 次**（`rg` 报 1 次）。
    /// 表现不是卡死——macOS 的 `PATH_MAX` 会让路径涨到千余字符后 `read_dir` 失败，
    /// 于是变成**静默重复**，直接污染 `scripts/bench.sh` 的召回与精确。
    #[test]
    fn 符号链接环不会重复收集() {
        let dir = std::env::temp_dir().join(format!("42find-symlink-{}", std::process::id()));
        let sub = dir.join("d");
        std::fs::create_dir_all(&sub).expect("建目录");
        std::fs::write(sub.join("x.txt"), "检索\n").expect("写文件");
        let link = sub.join("loop");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("..", &link).expect("建符号链接");

        let mut files = Vec::new();
        let ok = collect(&dir, Some("*.txt"), &mut files);

        std::fs::remove_dir_all(&dir).ok();

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
    /// 实测：用 `mkfifo` 造一个匹配 glob 的 `pipe.txt`，改之前进程 6 秒不返回、只能强杀。
    /// 这里用 `UnixListener` 造 socket，因为 std 没有 mkfifo，而造 FIFO 就得引依赖
    /// （`find42-cli` 也该保持零第三方依赖）。两者走的是同一条判断。
    #[test]
    fn 非常规文件被跳过() {
        let dir = std::env::temp_dir().join(format!("42find-sock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("建目录");
        std::fs::write(dir.join("real.txt"), "检索\n").expect("写文件");
        let sock = dir.join("s.txt");
        let _ = std::fs::remove_file(&sock);
        let listener = std::os::unix::net::UnixListener::bind(&sock).expect("建 socket");

        let mut files = Vec::new();
        collect(&dir, Some("*.txt"), &mut files);

        drop(listener);
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(files.len(), 1, "socket 不该被当成待搜文件：{files:?}");
        assert!(files[0].ends_with("real.txt"));
    }

    /// **显式**给出的非常规文件也要拒绝，不只是递归时。
    ///
    /// 第三轮只修了递归循环，`42find -- 词 /tmp/pipe.txt` 依然永久阻塞——
    /// 同一个 bug 只修了一半，是换一条谱系评审才抓出来的。
    #[test]
    fn 显式给出的非常规文件被拒绝() {
        let dir = std::env::temp_dir().join(format!("42find-explicit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("建目录");
        let sock = dir.join("s.txt");
        let _ = std::fs::remove_file(&sock);
        let listener = std::os::unix::net::UnixListener::bind(&sock).expect("建 socket");

        let mut files = Vec::new();
        let ok = collect(&sock, None, &mut files);

        drop(listener);
        std::fs::remove_dir_all(&dir).ok();

        assert!(!ok, "显式给一个不可搜的路径，应报失败（退出码 2）");
        assert!(files.is_empty(), "非常规文件不该进待搜列表：{files:?}");
    }

    #[test]
    fn 符号链接本身不被当成待搜文件() {
        let dir = std::env::temp_dir().join(format!("42find-symlink2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("建目录");
        std::fs::write(dir.join("real.txt"), "检索\n").expect("写文件");
        let link = dir.join("alias.txt");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("real.txt", &link).expect("建符号链接");

        let mut files = Vec::new();
        collect(&dir, Some("*.txt"), &mut files);

        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(files.len(), 1, "链接指向的文件不该被搜两遍：{files:?}");
        assert!(files[0].ends_with("real.txt"));
    }
}
