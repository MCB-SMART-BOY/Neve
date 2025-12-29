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
#[command(author, version, about = "Neve - A pure functional language for system configuration", long_about = None)]
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
        /// Topic to view (quickstart, tutorial, spec, api, philosophy, install, changelog).
        /// 要查看的主题（quickstart, tutorial, spec, api, philosophy, install, changelog）。
        topic: Option<String>,

        /// Show only English section. / 仅显示英文部分。
        #[arg(long)]
        en: bool,

        /// Show only Chinese section. / 仅显示中文部分。
        #[arg(long)]
        zh: bool,

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
    /// Rollback to previous configuration. / 回滚到上一个配置。
    Rollback,
    /// List configuration generations. / 列出配置代。
    List,
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
        Commands::Doc {
            topic,
            en,
            zh,
            list,
        } => {
            if list || topic.is_none() {
                commands::doc::list()
            } else {
                let lang = if en {
                    Some("en")
                } else if zh {
                    Some("zh")
                } else {
                    None
                };
                commands::doc::view(topic.as_deref().unwrap(), lang)
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
        Commands::Build { package, backend } => commands::build::run(package.as_deref(), &backend),
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
            ConfigAction::Rollback => commands::config::rollback(),
            ConfigAction::List => commands::config::list_generations(),
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
