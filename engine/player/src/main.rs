//! `hezhou-player`：Hezhou Player 最小运行时入口。
//!
//! 设计约束：
//! - Windows 发布版为 GUI 子系统（`windows_subsystem = "windows"`），不弹终端；
//! - 所有诊断写入 `player.log`，失败时另写 `startup-error.log`；
//! - 路径按包根目录相对解析，安装目录只读时日志回退用户数据目录。
//!
//! 开发调试构建（debug）保留控制台，便于直接看到启动摘要。

// 发布版 Windows 无终端；debug 版保留控制台便于开发。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;

use hezhou_player::runtime::RuntimeOptions;
use hezhou_player::RUNTIME_ENGINE_VERSION;

fn main() {
    std::process::exit(run_cli());
}

fn run_cli() -> i32 {
    let mut options = RuntimeOptions::default();
    let mut args = std::env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("hezhou-player {RUNTIME_ENGINE_VERSION}");
                return 0;
            }
            "--help" | "-h" => {
                print_help();
                return 0;
            }
            "--validate" => options.validate_only = true,
            "--run" => options.run = true,
            "--host" => match args.next() {
                Some(value) => {
                    options.host = Some(value);
                    options.run = true;
                }
                None => {
                    eprintln!("错误：--host 需要一个名称参数（本轮支持 default）");
                    return 2;
                }
            },
            "--manifest" => match args.next() {
                Some(value) => options.manifest_path = Some(PathBuf::from(value)),
                None => {
                    eprintln!("错误：--manifest 需要一个路径参数");
                    return 2;
                }
            },
            "--log-dir" => match args.next() {
                Some(value) => options.log_dir = Some(PathBuf::from(value)),
                None => {
                    eprintln!("错误：--log-dir 需要一个目录参数");
                    return 2;
                }
            },
            "--player-root" => match args.next() {
                Some(value) => options.player_root = Some(PathBuf::from(value)),
                None => {
                    eprintln!("错误：--player-root 需要一个目录参数");
                    return 2;
                }
            },
            other => {
                eprintln!("错误：未知参数 `{other}`");
                print_help();
                return 2;
            }
        }
    }

    if options.validate_only && options.run {
        eprintln!("提示：同时指定 --validate 与 --run/--host，按 --validate 执行（不调用 host）");
    }

    let failure_root = failure_log_root(&options);
    match hezhou_player::runtime::run(options) {
        Ok(report) => {
            println!("{}", report.summary());
            report.exit_code()
        }
        Err(error) => {
            eprintln!("Player 启动失败：{error}");
            if let Some(root) = failure_root {
                if let Some(path) = hezhou_player::runtime::write_failure_log(&root, &error) {
                    eprintln!("错误日志：{}", path.display());
                }
            }
            2
        }
    }
}

/// 失败日志落点：清单所在目录 > 指定包根目录 > 可执行文件目录。
fn failure_log_root(options: &RuntimeOptions) -> Option<PathBuf> {
    if let Some(manifest) = &options.manifest_path {
        if let Some(parent) = manifest.parent() {
            if !parent.as_os_str().is_empty() {
                return Some(parent.to_path_buf());
            }
        }
    }
    if let Some(root) = &options.player_root {
        return Some(root.clone());
    }
    hezhou_player::runtime::resolve_player_root().ok()
}

fn print_help() {
    println!(
        "hezhou-player {RUNTIME_ENGINE_VERSION} — Hezhou Player 最小运行时\n\
         \n\
         用法：hezhou-player [选项]\n\
         \n\
         模式（默认只做启动检查并退出 0）：\n\
           --validate            只做启动检查（含资产索引），不进游戏循环\n\
           --run                 加载资产索引后调用 PlayerHost（本轮默认 host 会明确\n\
                                 报告“等待 game host 接入”并退出 3）\n\
           --host <name>         选择 PlayerHost（本轮支持 default/headless/null），\n\
                                 隐含 --run\n\
         \n\
         选项：\n\
           --player-root <dir>   指定 Player 包根目录（默认：可执行文件所在目录）\n\
           --manifest <file>     指定 manifest.json 路径（默认：<player-root>/manifest.json）\n\
           --log-dir <dir>       指定日志目录（默认：用户数据目录）\n\
           --version, -V         打印版本\n\
           --help, -h            打印帮助\n\
         \n\
         退出码：0 启动检查/host 运行成功；2 启动检查失败（详见 startup-error.log）；\n\
                 3 host pending（本轮 game host 尚未接入，资产索引已加载）"
    );
}

