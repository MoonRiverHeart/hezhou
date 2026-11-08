//! `hezhou-player-build`：Player 构建器 CLI（dry-run 计划 + 真实产物构建）。
//!
//! dry-run（不写产物二进制）：
//! ```text
//! hezhou-player-build --config <build-config.json> --cook <cook-manifest.json> \
//!   [--scripts-dir <dir>] [--runtime-dir <dir>] \
//!   [--target both|windows|linux] [--plan-out <dir>] [--json]
//! ```
//!
//! 真实构建（复制 exe/assets/scripts/runtime/manifest 到 `<build-out>/<当前平台>/`）：
//! ```text
//! hezhou-player-build --config ... --cook ... \
//!   --exe <cargo 产出的 player 可执行文件> \
//!   --assets-dir <cook 资产根目录> \
//!   [--scripts-dir <dir>] [--runtime-dir <dir>] \
//!   --build-out <dir>
//! ```
//!
//! 真实构建只允许当前平台；双平台发布由 CI 在 Windows/Linux 分别执行。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use hezhou_player::builder::{
    BuildSources, CookManifest, PlayerBuildInput, PlayerBuildPlan, PlayerBuilder, RuntimePayload,
    ScriptAssembly,
};
use hezhou_player::config::PlayerBuildConfig;
use hezhou_player::cook_bridge::CookedAssets;
use hezhou_player::platform::TargetPlatform;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetArg {
    Both,
    Only(TargetPlatform),
}

fn main() {
    exit(run());
}

fn run() -> i32 {
    match parse_args() {
        Ok(Args::Help) => {
            print_help();
            0
        }
        Ok(Args::Build(args)) => execute(args),
        Err(message) => {
            eprintln!("错误：{message}");
            print_help();
            2
        }
    }
}

enum Args {
    Help,
    Build(BuildArgs),
}

#[derive(Default)]
struct BuildArgs {
    config: Option<PathBuf>,
    cook: Option<PathBuf>,
    scripts_dir: Option<PathBuf>,
    runtime_dir: Option<PathBuf>,
    exe: Option<PathBuf>,
    assets_dir: Option<PathBuf>,
    build_out: Option<PathBuf>,
    plan_out: Option<PathBuf>,
    target: Option<TargetArg>,
    json: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = BuildArgs::default();
    let mut raw = std::env::args().skip(1);
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Args::Help),
            "--config" => args.config = Some(take_path(&mut raw, "--config")?),
            "--cook" => args.cook = Some(take_path(&mut raw, "--cook")?),
            "--scripts-dir" => args.scripts_dir = Some(take_path(&mut raw, "--scripts-dir")?),
            "--runtime-dir" => args.runtime_dir = Some(take_path(&mut raw, "--runtime-dir")?),
            "--exe" => args.exe = Some(take_path(&mut raw, "--exe")?),
            "--assets-dir" => args.assets_dir = Some(take_path(&mut raw, "--assets-dir")?),
            "--build-out" => args.build_out = Some(take_path(&mut raw, "--build-out")?),
            // `--out` 保留为 `--plan-out` 的别名，避免旧命令失效。
            "--plan-out" | "--out" => {
                args.plan_out = Some(take_path(&mut raw, "--plan-out")?)
            }
            "--json" => args.json = true,
            "--target" => {
                let value = raw
                    .next()
                    .ok_or_else(|| "--target 需要一个值（both/windows/linux）".to_string())?;
                args.target = Some(match value.as_str() {
                    "both" => TargetArg::Both,
                    "windows" => TargetArg::Only(TargetPlatform::Windows),
                    "linux" => TargetArg::Only(TargetPlatform::Linux),
                    other => {
                        return Err(format!(
                            "--target 只支持 both/windows/linux，收到 `{other}`"
                        ))
                    }
                });
            }
            other => return Err(format!("未知参数 `{other}`")),
        }
    }
    Ok(Args::Build(args))
}

fn take_path(raw: &mut impl Iterator<Item = String>, name: &str) -> Result<PathBuf, String> {
    raw.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{name} 需要一个路径参数"))
}

fn execute(args: BuildArgs) -> i32 {
    match build(args) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("构建失败：{message}");
            2
        }
    }
}

fn build(args: BuildArgs) -> Result<(), String> {
    let config_path = args
        .config
        .clone()
        .ok_or_else(|| "缺少 --config <build-config.json>".to_string())?;
    let cook_path = args
        .cook
        .clone()
        .ok_or_else(|| "缺少 --cook <cook-manifest.json>".to_string())?;

    let config_text = fs::read_to_string(&config_path)
        .map_err(|error| format!("读取构建配置 {} 失败：{error}", config_path.display()))?;
    let config: PlayerBuildConfig = serde_json::from_str(&config_text)
        .map_err(|error| format!("解析构建配置 {} 失败：{error}", config_path.display()))?;

    // `--cook` 同时支持 hezhou-asset 的 cook_manifest.json（entries[]）与
    // Player 的 legacy 简单清单（assets[]），通过顶层字段自动识别。
    let loaded = load_cook(&cook_path)?;

    let scripts = match &args.scripts_dir {
        Some(dir) => ScriptAssembly::from_dir(dir).map_err(|error| error.to_string())?,
        None => Vec::new(),
    };
    let runtime = match &args.runtime_dir {
        Some(dir) => RuntimePayload::from_dir(dir).map_err(|error| error.to_string())?,
        None => RuntimePayload::empty(),
    };

    let mut input = PlayerBuildInput::new(
        config,
        loaded.legacy.unwrap_or_else(CookManifest::default),
    );
    input.scripts = scripts;
    input.runtime = runtime;
    input.cooked = loaded.cooked;
    let builder = PlayerBuilder::new(input);

    if let Some(build_out) = &args.build_out {
        build_real_artifacts(&builder, &args, build_out)?;
    } else {
        dry_run(&builder, &args)?;
    }
    Ok(())
}

/// dry-run：生成双平台计划（可选写计划元数据）。
fn dry_run(builder: &PlayerBuilder, args: &BuildArgs) -> Result<(), String> {
    let plans = requested_plans(builder, args.target.unwrap_or(TargetArg::Both))?;
    for plan in &plans {
        println!("{plan}");
    }
    if let Some(plan_out) = &args.plan_out {
        for plan in &plans {
            plan.write_plan(&plan_out.join(plan.target.as_str()))
                .map_err(|error| error.to_string())?;
        }
        eprintln!("计划已写入 {}", plan_out.display());
    }
    if args.json {
        print_json(&plans)?;
    }
    Ok(())
}

/// 真实构建：复制产物并通过运行时校验（只构建当前平台）。
fn build_real_artifacts(
    builder: &PlayerBuilder,
    args: &BuildArgs,
    build_out: &PathBuf,
) -> Result<(), String> {
    let host = TargetPlatform::host();
    let target = match args.target.unwrap_or(TargetArg::Both) {
        TargetArg::Both => {
            eprintln!(
                "提示：真实构建只构建当前平台 {host}；双平台请在 Windows/Linux 分别执行（dry-run 仍可双平台预览）"
            );
            host
        }
        TargetArg::Only(platform) => {
            if platform != host {
                return Err(format!(
                    "真实构建只能构建当前平台 {host}，不能构建 {platform}"
                ));
            }
            platform
        }
    };

    let exe = args
        .exe
        .clone()
        .ok_or_else(|| "真实构建缺少 --exe <可执行文件>".to_string())?;
    // 真实 cook 清单的资产来源是清单所在目录；legacy 清单才需要 --assets-dir。
    let assets_root = args
        .assets_dir
        .clone()
        .or_else(|| {
            builder
                .input()
                .cooked
                .as_ref()
                .map(|cooked| cooked.output_root.clone())
        })
        .ok_or_else(|| {
            "真实构建缺少 --assets-dir（legacy cook 清单需要资产根目录；\
             hezhou-asset cook_manifest.json 会自动使用清单所在目录）"
                .to_string()
        })?;
    let sources = BuildSources::new(exe, assets_root);

    let output = builder
        .build(target, &sources, &build_out.join(host.as_str()))
        .map_err(|error| error.to_string())?;
    println!("{output}");
    if args.json {
        let json = serde_json::to_string_pretty(&output)
            .map_err(|error| format!("序列化构建结果失败：{error}"))?;
        println!("{json}");
    }
    if let Some(plan_out) = &args.plan_out {
        let plan = builder.plan_for(target).map_err(|error| error.to_string())?;
        plan.write_plan(&plan_out.join(target.as_str()))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn requested_plans(
    builder: &PlayerBuilder,
    target: TargetArg,
) -> Result<Vec<PlayerBuildPlan>, String> {
    match target {
        TargetArg::Both => builder.plan_both_targets().map_err(|error| error.to_string()),
        TargetArg::Only(platform) => builder
            .plan_for(platform)
            .map(|plan| vec![plan])
            .map_err(|error| error.to_string()),
    }
}

fn print_json(plans: &[PlayerBuildPlan]) -> Result<(), String> {
    for plan in plans {
        let json = serde_json::to_string_pretty(plan)
            .map_err(|error| format!("序列化计划失败：{error}"))?;
        println!("{json}");
    }
    Ok(())
}

struct LoadedCook {
    /// Player legacy 简单清单（`{ "assets": [...] }`）。
    legacy: Option<CookManifest>,
    /// hezhou-asset 真实 cook 输出（`cook_manifest.json` + `imported/` + `assets/`）。
    cooked: Option<CookedAssets>,
}

/// 自动识别 cook 清单类型：
/// - 顶层含 `entries[]` → hezhou-asset `CookManifest`（产物根目录 = 清单所在目录）；
/// - 否则按 Player legacy `{ "assets": [...] }` 解析。
fn load_cook(path: &Path) -> Result<LoadedCook, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("读取 cook 清单 {} 失败：{error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("解析 cook 清单 {} 失败：{error}", path.display()))?;
    if value.get("entries").is_some() {
        let cooked = CookedAssets::load(path).map_err(|error| error.to_string())?;
        Ok(LoadedCook {
            legacy: None,
            cooked: Some(cooked),
        })
    } else {
        let legacy: CookManifest = serde_json::from_value(value)
            .map_err(|error| format!("解析 legacy cook 清单 {} 失败：{error}", path.display()))?;
        Ok(LoadedCook {
            legacy: Some(legacy),
            cooked: None,
        })
    }
}

fn print_help() {
    println!(
        "hezhou-player-build — Hezhou Player 构建器（dry-run + 真实产物）\n\
         \n\
         用法：\n\
           hezhou-player-build --config <build-config.json> --cook <cook-manifest.json> [选项]\n\
         \n\
         输入选项：\n\
           --scripts-dir <dir>   预编译 C# 程序集目录（枚举 *.dll）\n\
           --runtime-dir <dir>   运行时目录（递归枚举，映射到 runtime/）\n\
           --exe <file>          真实构建：cargo 产出的 Player 可执行文件\n\
           --assets-dir <dir>    legacy cook 清单的资产根目录\n\
         \n\
         输出选项：\n\
           --build-out <dir>     真实构建输出根目录（产物写入 <dir>/<当前平台>/）\n\
           --plan-out <dir>      写出 manifest.json、plan.json、file-list.txt（别名 --out）\n\
           --target <value>      both（默认）/ windows / linux（真实构建只允许当前平台）\n\
           --json                以 JSON 输出计划/构建结果\n\
           --help, -h            打印帮助\n\
         \n\
         cook 清单：\n\
           --cook 支持 hezhou-asset 的 cook_manifest.json（推荐，entries[]）与 Player\n\
           legacy 清单（assets[]）。真实 cook 输出根目录取清单所在目录；\n\
           资产按 manifest 的 output 原样映射到 Player 包 assets/ 下。\n\
         \n\
         退出码：0 成功；2 构建失败（失败会清理 staging，不留下半成品）"
    );
}
