//! Neve CLI - The Neve language command line interface.

mod commands;
mod output;
mod platform;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "neve")]
#[command(author, version, about = "Neve - A pure functional language for system configuration", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress output
    #[arg(short, long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Evaluate an expression
    Eval {
        /// The expression to evaluate
        expr: String,
    },

    /// Run a Neve file
    Run {
        /// The file to run
        file: String,
    },

    /// Type check a file
    Check {
        /// The file to check
        file: String,
    },

    /// Format a file or directory
    Fmt {
        #[command(subcommand)]
        action: FmtAction,
    },

    /// Start an interactive REPL
    Repl,

    /// Build a package (Unix only)
    #[cfg(unix)]
    Build {
        /// Package name or path
        package: Option<String>,

        /// Build backend (native, docker, simple)
        #[arg(long, default_value = "auto")]
        backend: String,
    },

    /// Package management commands (Unix only)
    #[cfg(unix)]
    Package {
        #[command(subcommand)]
        action: PackageAction,
    },

    /// Search for packages (Unix only)
    #[cfg(unix)]
    Search {
        /// Search query
        query: String,
    },

    /// Show package or platform information
    Info {
        /// Package name (Unix only)
        package: Option<String>,

        /// Show platform capabilities
        #[arg(long, short = 'p')]
        platform: bool,
    },

    /// Update dependencies (Unix only)
    #[cfg(unix)]
    Update,

    /// System configuration commands (Unix only)
    #[cfg(unix)]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Store management commands (Unix only)
    #[cfg(unix)]
    Store {
        #[command(subcommand)]
        action: StoreAction,
    },
}

#[derive(Subcommand)]
enum FmtAction {
    /// Format a file
    File {
        /// The file to format
        file: String,
        /// Write changes to file
        #[arg(short, long)]
        write: bool,
    },
    /// Check if a file is formatted
    Check {
        /// The file to check
        file: String,
    },
    /// Format all files in a directory
    Dir {
        /// The directory to format
        dir: String,
        /// Write changes to files
        #[arg(short, long)]
        write: bool,
    },
}

#[cfg(unix)]
#[derive(Subcommand)]
enum PackageAction {
    /// Install a package
    Install {
        /// Package to install
        package: String,
    },
    /// Remove a package
    Remove {
        /// Package to remove
        package: String,
    },
    /// List installed packages
    List,
    /// Rollback to previous generation
    Rollback,
}

#[cfg(unix)]
#[derive(Subcommand)]
enum ConfigAction {
    /// Build system configuration
    Build,
    /// Switch to new configuration
    Switch,
    /// Rollback to previous configuration
    Rollback,
    /// List configuration generations
    List,
}

#[cfg(unix)]
#[derive(Subcommand)]
enum StoreAction {
    /// Run garbage collection
    Gc,
    /// Show store information
    Info,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        // Cross-platform commands (language features)
        Commands::Eval { expr } => commands::eval::run(&expr, cli.verbose),
        Commands::Run { file } => commands::run::run(&file, cli.verbose),
        Commands::Check { file } => commands::check::run(&file, cli.verbose),
        Commands::Fmt { action } => match action {
            FmtAction::File { file, write } => commands::fmt::run(&file, write),
            FmtAction::Check { file } => commands::fmt::check(&file),
            FmtAction::Dir { dir, write } => commands::fmt::format_dir(&dir, write),
        },
        Commands::Repl => commands::repl::run(),
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
