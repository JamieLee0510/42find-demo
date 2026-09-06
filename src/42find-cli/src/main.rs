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
    0 有命中 · 1 无命中 · 2 参数或读取出错
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

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if only_positional {
            rest.push(a);
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => return Ok(None),
            "--column" => column = true,
            "--glob" => glob = Some(it.next().ok_or("--glob 后面要跟一个模式")?),
            "--" => only_positional = true,
            other if other.starts_with('-') => return Err(format!("不认识的选项：{other}")),
            other => rest.push(other.to_owned()),
        }
    }

    let mut rest = rest.into_iter();
    let query = rest.next().ok_or("缺少查询词")?;
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

fn collect(path: &Path, glob: Option<&str>, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        children.sort();
        for child in children {
            collect(&child, glob, out);
        }
    } else if glob_matches(glob, path) {
        out.push(path.to_owned());
    }
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
    for p in &args.paths {
        collect(p, args.glob.as_deref(), &mut files);
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

    if found {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
