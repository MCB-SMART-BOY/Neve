//! Neve CLI - The Neve language command line interface.
//! Neve CLI - Neve 语言的命令行界面。

mod commands;
mod output;
mod platform;

use clap::{Parser, Subcommand};

/// Main CLI structure.
/// 主 CLI 结构体。
#[derive(Parser)]
#[command(name = "neve")]
#[command(
    author,
    version,
    about = "Neve - A standalone language for system configuration and structured shell automation",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output. / 启用详细输出。
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress output. / 抑制输出。
    #[arg(short, long, global = true)]
    quiet: bool,
}

/// Available CLI commands.
/// 可用的 CLI 命令。
#[derive(Subcommand)]
enum Commands {
    /// Evaluate an expression. / 求值表达式。
    Eval {
        /// The expression to evaluate. / 要求值的表达式。
        expr: String,
    },

    /// Run a Neve file. / 运行 Neve 文件。
    Run {
        /// The file to run. / 要运行的文件。
        file: String,
    },

    /// Type check a file. / 类型检查文件。
    Check {
        /// The file to check. / 要检查的文件。
        file: String,
    },

    /// Format a file or directory. / 格式化文件或目录。
    Fmt {
        #[command(subcommand)]
        action: FmtAction,
    },

    /// Start an interactive REPL. / 启动交互式 REPL。
    Repl,

    /// View documentation (like man pages). / 查看文档（类似 man 手册）。
    Doc {
        /// Topic to view (quickstart, tutorial, spec, api, diagnostics, philosophy, install, architecture, onboarding, changelog).
        /// 要查看的主题（quickstart, tutorial, spec, api, diagnostics, philosophy, install, architecture, onboarding, changelog）。
        topic: Option<String>,

        /// List all available topics. / 列出所有可用主题。
        #[arg(long, short)]
        list: bool,
    },

    /// Build a package (Unix only). / 构建软件包（仅限 Unix）。
    #[cfg(unix)]
    Build {
        /// Package name or path. / 软件包名称或路径。
        package: Option<String>,

        /// Build backend (native, docker, simple). / 构建后端（native, docker, simple）。
        #[arg(long, default_value = "auto")]
        backend: String,

        /// Binary cache URL (repeatable). / 二进制缓存 URL（可重复）。
        #[arg(long = "cache-url", value_name = "URL")]
        cache_urls: Vec<String>,

        /// Local binary cache directory (repeatable). / 本地二进制缓存目录（可重复）。
        #[arg(long = "cache-dir", value_name = "DIR")]
        cache_dirs: Vec<String>,

        /// Disable substituter downloads. / 禁用 substituter 下载。
        #[arg(long = "no-substitute")]
        no_substitute: bool,

        /// Upload successful outputs to writable caches. / 将成功产物上传到可写缓存。
        #[arg(long = "cache-upload")]
        cache_upload: bool,

        /// Public key for narinfo signature verification (`ed25519:<base64>`, repeatable).
        /// narinfo 签名验证公钥（`ed25519:<base64>`，可重复）。
        #[arg(long = "cache-public-key", value_name = "KEY")]
        cache_public_keys: Vec<String>,

        /// Private key for narinfo signing on upload (`ed25519:<base64>`, repeatable).
        /// 上传时用于 narinfo 签名的私钥（`ed25519:<base64>`，可重复）。
        #[arg(long = "cache-private-key", value_name = "KEY")]
        cache_private_keys: Vec<String>,
    },

    /// Package management commands (Unix only). / 软件包管理命令（仅限 Unix）。
    #[cfg(unix)]
    Package {
        #[command(subcommand)]
        action: PackageAction,
    },

    /// Search for packages (Unix only). / 搜索软件包（仅限 Unix）。
    #[cfg(unix)]
    Search {
        /// Search query. / 搜索查询。
        query: String,
    },

    /// Show package or platform information. / 显示软件包或平台信息。
    Info {
        /// Package name (Unix only). / 软件包名称（仅限 Unix）。
        package: Option<String>,

        /// Show platform capabilities. / 显示平台功能。
        #[arg(long, short = 'p')]
        platform: bool,
    },

    /// Update dependencies (Unix only). / 更新依赖（仅限 Unix）。
    #[cfg(unix)]
    Update,

    /// System configuration commands (Unix only). / 系统配置命令（仅限 Unix）。
    #[cfg(unix)]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Store management commands (Unix only). / 存储管理命令（仅限 Unix）。
    #[cfg(unix)]
    Store {
        #[command(subcommand)]
        action: StoreAction,
    },
}

/// Format subcommands.
/// 格式化子命令。
#[derive(Subcommand)]
enum FmtAction {
    /// Format a file. / 格式化文件。
    File {
        /// The file to format. / 要格式化的文件。
        file: String,
        /// Write changes to file. / 将更改写入文件。
        #[arg(short, long)]
        write: bool,
    },
    /// Check if a file is formatted. / 检查文件是否已格式化。
    Check {
        /// The file to check. / 要检查的文件。
        file: String,
    },
    /// Format all files in a directory. / 格式化目录中的所有文件。
    Dir {
        /// The directory to format. / 要格式化的目录。
        dir: String,
        /// Write changes to files. / 将更改写入文件。
        #[arg(short, long)]
        write: bool,
    },
}

/// Package management subcommands (Unix only).
/// 软件包管理子命令（仅限 Unix）。
#[cfg(unix)]
#[derive(Subcommand)]
enum PackageAction {
    /// Install a package. / 安装软件包。
    Install {
        /// Package to install. / 要安装的软件包。
        package: String,
    },
    /// Remove a package. / 移除软件包。
    Remove {
        /// Package to remove. / 要移除的软件包。
        package: String,
    },
    /// List installed packages. / 列出已安装的软件包。
    List,
    /// Rollback to previous generation. / 回滚到上一代。
    Rollback,
}

/// Configuration management subcommands (Unix only).
/// 配置管理子命令（仅限 Unix）。
#[cfg(unix)]
#[derive(Subcommand)]
enum ConfigAction {
    /// Build system configuration. / 构建系统配置。
    Build,
    /// Switch to new configuration. / 切换到新配置。
    Switch,
    /// Interactively switch to a specific generation. / 交互式切换到特定代。
    SwitchTo,
    /// Rollback to previous configuration. / 回滚到上一个配置。
    Rollback,
    /// List configuration generations. / 列出配置代。
    List,
    /// Verify generation activation snapshot integrity. / 校验 generation 激活快照完整性。
    Verify {
        /// Verify all generations instead of only current. / 校验全部 generation，而不是仅当前。
        #[arg(long, short = 'a', conflicts_with = "generation")]
        all: bool,
        /// Generation number to verify. / 要校验的 generation 编号。
        #[arg(conflicts_with = "all")]
        generation: Option<u64>,
    },
}

/// Store management subcommands (Unix only).
/// 存储管理子命令（仅限 Unix）。
#[cfg(unix)]
#[derive(Subcommand)]
enum StoreAction {
    /// Run garbage collection. / 运行垃圾回收。
    Gc,
    /// Show store information. / 显示存储信息。
    Info,
}

/// Main entry point.
/// 主入口点。
fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        // Cross-platform commands (language features)
        // 跨平台命令（语言功能）
        Commands::Eval { expr } => commands::eval::run(&expr, cli.verbose),
        Commands::Run { file } => commands::run::run(&file, cli.verbose),
        Commands::Check { file } => commands::check::run(&file, cli.verbose),
        Commands::Fmt { action } => match action {
            FmtAction::File { file, write } => commands::fmt::run(&file, write),
            FmtAction::Check { file } => commands::fmt::check(&file),
            FmtAction::Dir { dir, write } => commands::fmt::format_dir(&dir, write),
        },
        Commands::Repl => commands::repl::run(),
        Commands::Doc { topic, list } => {
            if list || topic.is_none() {
                commands::doc::list()
            } else {
                commands::doc::view(topic.as_deref().unwrap())
            }
        }
        Commands::Info { package, platform } => {
            if platform || package.is_none() {
                commands::info::platform_info()
            } else {
                #[cfg(unix)]
                {
                    commands::info::run(package.as_deref().unwrap())
                }
                #[cfg(not(unix))]
                {
                    let _ = package;
                    eprintln!("Package info is only available on Unix systems");
                    Ok(())
                }
            }
        }

        // Unix-only commands (package management)
        // 仅限 Unix 的命令（软件包管理）
        #[cfg(unix)]
        Commands::Build {
            package,
            backend,
            cache_urls,
            cache_dirs,
            no_substitute,
            cache_upload,
            cache_public_keys,
            cache_private_keys,
        } => commands::build::run(commands::build::BuildRunArgs {
            package: package.as_deref(),
            backend_arg: &backend,
            cli_cache_urls: &cache_urls,
            cli_cache_dirs: &cache_dirs,
            cli_substitute: !no_substitute,
            cli_cache_upload: cache_upload,
            cli_cache_public_keys: &cache_public_keys,
            cli_cache_private_keys: &cache_private_keys,
        }),
        #[cfg(unix)]
        Commands::Package { action } => match action {
            PackageAction::Install { package } => commands::install::run(&package),
            PackageAction::Remove { package } => commands::remove::run(&package),
            PackageAction::List => commands::install::list(),
            PackageAction::Rollback => commands::remove::rollback(),
        },
        #[cfg(unix)]
        Commands::Search { query } => commands::search::run(&query),
        #[cfg(unix)]
        Commands::Update => commands::update::run(),
        #[cfg(unix)]
        Commands::Config { action } => match action {
            ConfigAction::Build => commands::config::build(),
            ConfigAction::Switch => commands::config::switch(),
            ConfigAction::SwitchTo => commands::config::switch_interactive(),
            ConfigAction::Rollback => commands::config::rollback(),
            ConfigAction::List => commands::config::list_generations(),
            ConfigAction::Verify { all, generation } => commands::config::verify(generation, all),
        },
        #[cfg(unix)]
        Commands::Store { action } => match action {
            StoreAction::Gc => commands::store::gc(),
            StoreAction::Info => commands::store::info(),
        },
    };

    if let Err(e) = result {
        if !cli.quiet {
            eprintln!("error: {}", e);
        }
        std::process::exit(1);
    }
}
