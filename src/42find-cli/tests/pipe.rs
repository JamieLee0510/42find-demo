#![cfg(unix)]

//! 端到端管道回归：**真起进程、真接管道、单独取退出码**。
//!
//! 为什么非得起进程：issue #7 的第三个症状（`42find … | head` panic 退 101）
//! 只在这一层可见。`emit` / `exit_code` 的单测证明不了 `main` 的编排
//! ——参数分派 → 缓冲 → flush → 退出码，这一串先前一行测试都没有。
//!
//! 而 `CLAUDE.md` 第三节写着「能用规则判的，绝不留给人判」：
//! 退出码正是能用规则判的。先前它只被人手跑过一遍、记在 `state/board.md` 里
//! （见 `state/memory/20260907-真实使用是一层独立验证.md`）。
//!
//! ⚠️ **只在 unix 上跑**（`#![cfg(unix)]`）。断管道走的是 POSIX 的 EPIPE 语义，
//! Windows 上 `drop(reader)` 之后的错误种类未经核实——CI 也只跑 ubuntu。
//! 与 `main.rs` 里那组文件系统测试（`#[cfg(all(test, unix))]`）同一条约定。
//!
//! ⚠️ **语料必须够大**。玩具语料只有几十行输出，在下游关掉管道之前就全进了
//! 管道缓冲，于是压根走不到断管道那条路——issue #7 里这个 bug 就是被
//! 固定语料藏了整整六轮双谱系评审。这里造 ~2 MB 输出，稳稳超过管道缓冲。

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_42find");

/// 临时语料，靠 `Drop` 清理（断言 panic 时也不会在 /tmp 里积攒）。
struct Corpus(PathBuf);

impl Corpus {
    /// `lines` 行、每行都含「检索」——命中数与输出量都由行数决定。
    fn new(tag: &str, lines: usize) -> Self {
        let dir = std::env::temp_dir().join(format!("42find-pipe-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        let body =
            "先归一再检索，还是先检索再归一——这一行够长，好让输出量超过管道缓冲。\n".repeat(lines);
        std::fs::write(dir.join("a.txt"), &body).expect("写语料");
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

/// 写一个自定义字节内容的文件到语料目录，返回它的路径字符串。
fn write_raw(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let p = dir.join(name);
    std::fs::write(&p, bytes).expect("写文件");
    p.to_str().expect("路径是 UTF-8").to_owned()
}

impl Drop for Corpus {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 跑一次 42find，**读走 `head_lines` 行就关掉读端**，然后取退出码与 stderr。
///
/// 关掉读端就是 `| head -n` 对上游做的事。`head_lines` 为 `None` 时读完全部。
fn run(args: &[&str], head_lines: Option<usize>) -> (Option<i32>, String) {
    let mut child = Command::new(BIN)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("起 42find");

    let stdout = child.stdout.take().expect("拿到 stdout");
    let mut reader = BufReader::new(stdout);
    match head_lines {
        Some(n) => {
            for _ in 0..n {
                let mut line = String::new();
                if reader.read_line(&mut line).expect("读一行") == 0 {
                    break;
                }
            }
        }
        None => {
            let mut sink = String::new();
            let _ = std::io::Read::read_to_string(&mut reader, &mut sink);
        }
    }
    drop(reader); // ← 关掉读端；上游再写就是 EPIPE

    let out = child.wait_with_output().expect("等 42find 退出");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// ★ 钉子：`42find … | head` 不许 panic，退出码必须在承诺的 {0,1} 里。
///
/// 改之前这里是 `Some(101)` 加一行 `panicked at … Broken pipe (os error 32)`。
#[test]
fn 有命中时下游关掉管道退0且无panic() {
    let c = Corpus::new("head", 20_000); // ~2 MB 输出，远超管道缓冲
    let (code, err) = run(
        &["--", "检索", c.path().to_str().expect("路径是 UTF-8")],
        Some(2),
    );
    assert!(!err.contains("panicked"), "断管道不许 panic，stderr：{err}");
    assert_eq!(
        code,
        Some(0),
        "有命中且下游关了管道，应退 0（stderr：{err}）"
    );
}

/// 无命中时同一构造退 1——别把「没找到」也一并抹成 0。
#[test]
fn 无命中时下游关掉管道退1() {
    let c = Corpus::new("nohit", 20_000);
    let (code, err) = run(
        &[
            "--",
            "这个词语料里没有xyz",
            c.path().to_str().expect("路径是 UTF-8"),
        ],
        Some(2),
    );
    assert!(!err.contains("panicked"), "不许 panic，stderr：{err}");
    assert_eq!(code, Some(1), "无命中应退 1（stderr：{err}）");
}

/// `42find --help | head -1` 一样会断管道——帮助文本也不走 `print!`。
#[test]
fn help接管道也不panic() {
    let (code, err) = run(&["--help"], Some(1));
    assert!(!err.contains("panicked"), "不许 panic，stderr：{err}");
    assert_eq!(code, Some(0));
}

/// 不接管道、读完全部：命中退 0、无命中退 1、参数错退 2。三条承诺各钉一次。
#[test]
fn 不接管道时的三个退出码() {
    let c = Corpus::new("plain", 4);
    let dir = c.path().to_str().expect("路径是 UTF-8");

    assert_eq!(run(&["--", "检索", dir], None).0, Some(0), "有命中");
    assert_eq!(run(&["--", "没有的词zzz", dir], None).0, Some(1), "无命中");
    assert_eq!(run(&["--", "", dir], None).0, Some(2), "空查询词是参数错误");
    assert_eq!(run(&["--不认识的选项"], None).0, Some(2), "未知选项");
}

/// 输出形态：默认给**整行**、一个匹配行一行；`--column` 逐命中并带字节列。
///
/// 这条守的是 issue #7 的症状 ①②——它们在单测里也钉着，但那是对 `emit` 函数；
/// 这里钉的是**用户真敲那条命令时看到的东西**，中间隔着参数解析与遍历。
#[test]
fn 输出形态是路径行号可选列整行() {
    let c = Corpus::new("shape", 1);
    let dir = c.path().to_str().expect("路径是 UTF-8");
    let plain = String::from_utf8(
        Command::new(BIN)
            .args(["--", "检索", dir])
            .output()
            .expect("跑 42find")
            .stdout,
    )
    .expect("输出是 UTF-8");
    let col = String::from_utf8(
        Command::new(BIN)
            .args(["--column", "--", "检索", dir])
            .output()
            .expect("跑 42find")
            .stdout,
    )
    .expect("输出是 UTF-8");

    assert_eq!(plain.lines().count(), 1, "同一行两处命中只出一行：{plain}");
    assert_eq!(col.lines().count(), 2, "--column 逐命中：{col}");
    assert!(
        plain.trim_end().ends_with("好让输出量超过管道缓冲。"),
        "默认模式必须给整行，不是只给命中的那个词：{plain}"
    );
    for line in col.lines() {
        assert!(
            line.ends_with("好让输出量超过管道缓冲。"),
            "--column 也要给整行：{line}"
        );
    }
}

/// UTF-8 BOM 必须先剥再扫——否则第一行的字节列恒偏 3，
/// 而「`--column` 与 `rg --column` 同单位」是钉死的硬约束。BOM 也不该随整行进 stdout。
///
/// Windows 上写的中文文本带 BOM 是常态，正是这个工具的目标语料。
#[test]
fn bom被剥掉后列号与不带bom的一致() {
    let c = Corpus::new("bom", 1);
    let mut with_bom = vec![0xEF, 0xBB, 0xBF];
    with_bom.extend_from_slice("检索开头\n".as_bytes());
    let bom = write_raw(c.path(), "bom.txt", &with_bom);
    let plain = write_raw(c.path(), "plain.txt", "检索开头\n".as_bytes());

    let col_of = |p: &str| {
        let o = Command::new(BIN)
            .args(["--column", "--", "检索", p])
            .output()
            .expect("跑 42find");
        String::from_utf8(o.stdout).expect("输出是 UTF-8")
    };
    let b = col_of(&bom);
    let n = col_of(&plain);

    assert!(b.contains(":1:1:"), "带 BOM 的第一行也该是第 1 字节列：{b}");
    assert!(!b.contains('\u{feff}'), "BOM 不该随整行进 stdout：{b:?}");
    assert_eq!(
        b.rsplit_once(':').expect("有冒号").1,
        n.rsplit_once(':').expect("有冒号").1,
        "剥 BOM 之后，整行内容应与不带 BOM 的完全一致"
    );
}

/// 含 NUL 的文件默认只报一行、不打印内容；`--text` 才照搜。
///
/// ★ 这条守的是**输出整行新引入的注入面**：改之前打印的是查询词展开后匹配到的片段，
/// 字符集受用户自己敲的东西约束；改成整行之后，被搜文件里任意字节都出得来——
/// 包括能改终端标题、甚至写剪贴板的 OSC 序列。`rg` 用同一条 NUL 判据挡它。
#[test]
fn 含nul的文件默认不打印内容() {
    let c = Corpus::new("nul", 1);
    let mut bytes = "检索".as_bytes().to_vec();
    bytes.extend_from_slice(b"\x00\x1b]52;c;QUJD\x07\n"); // NUL + 写剪贴板的 OSC 52
    let p = write_raw(c.path(), "bin.txt", &bytes);

    let run_args = |extra: &[&str]| {
        let mut args: Vec<&str> = extra.to_vec();
        args.extend_from_slice(&["--", "检索", &p]);
        let o = Command::new(BIN).args(&args).output().expect("跑 42find");
        (
            o.status.code(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
        )
    };

    let (code, out) = run_args(&[]);
    assert_eq!(code, Some(0), "二进制文件有命中，仍算命中");
    assert!(out.contains("二进制文件有命中"), "应只报一行：{out:?}");
    assert!(
        !out.contains('\u{1b}') && !out.contains('\0'),
        "控制字节不该进 stdout：{out:?}"
    );

    let (code, out) = run_args(&["--text"]);
    assert_eq!(code, Some(0));
    assert!(out.contains('\u{1b}'), "--text 就是要照搜、原样给：{out:?}");
}

/// 整行默认截到 512 字节：单行大文件的输出量从**平方**降回**线性**。
///
/// ★ pr-ready 三视角同时抓到的 P0。10 KB 单行文件查一个常见字：
/// 不截时 `--column` 输出 106 MB，默认上限下 6.6 MB。
///
/// ⚠️ **上限把平方降成线性，不是降成常数**——输出行数仍正比于命中数，
/// 只是每行不再正比于行长。真正的不变量是**每条输出行有界**，下面钉的就是它。
#[test]
fn 单行大文件默认不再爆输出() {
    let c = Corpus::new("longline", 1);
    let p = write_raw(c.path(), "one.txt", &vec![b'a'; 10 * 1024]); // 10 KB，无换行

    let run = |extra: &[&str]| {
        let mut args: Vec<&str> = vec!["--column"];
        args.extend_from_slice(extra);
        args.extend_from_slice(&["--", "a", &p]);
        Command::new(BIN)
            .args(&args)
            .output()
            .expect("跑 42find")
            .stdout
    };

    let default = run(&[]);
    let widest = default
        .split(|b| *b == b'\n')
        .map(<[u8]>::len)
        .max()
        .expect("有输出");
    // 一行 = 路径 + `:行:列:` + 截断后的 512 字节 + 截断标注。上界与行长无关，只与路径长度有关。
    let bound = 512 + p.len() + 64;
    assert!(
        widest <= bound,
        "每条输出行必须有界（≤ {bound}），实测最宽 {widest} 字节"
    );

    let unbounded = run(&["--max-columns", "0"]);
    assert!(
        unbounded.len() > 10 * default.len(),
        "不截时该是平方级：{} vs 默认 {}",
        unbounded.len(),
        default.len()
    );
    assert!(
        unbounded.len() > 50 * 1024 * 1024,
        "`--max-columns 0` 是明说要原样整行的逃生口，实测 {} 字节",
        unbounded.len()
    );
}
