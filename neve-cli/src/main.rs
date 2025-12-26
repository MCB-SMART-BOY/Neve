//! Neve CLI - The Neve language command line interface.

mod commands;
mod output;

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

    /// Build a package
    Build {
        /// Package name or path
        package: Option<String>,
    },

    /// Package management commands
    Package {
        #[command(subcommand)]
        action: PackageAction,
    },

    /// Search for packages
    Search {
        /// Search query
        query: String,
    },

    /// Show package information
    Info {
        /// Package name
        package: String,
    },

    /// Update dependencies
    Update,

    /// System configuration commands
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Store management commands
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
        Commands::Eval { expr } => commands::eval::run(&expr, cli.verbose),
        Commands::Run { file } => commands::run::run(&file, cli.verbose),
        Commands::Check { file } => commands::check::run(&file, cli.verbose),
        Commands::Fmt { action } => match action {
            FmtAction::File { file, write } => commands::fmt::run(&file, write),
            FmtAction::Check { file } => commands::fmt::check(&file),
            FmtAction::Dir { dir, write } => commands::fmt::format_dir(&dir, write),
        },
        Commands::Repl => commands::repl::run(),
        Commands::Build { package } => commands::build::run(package.as_deref()),
        Commands::Package { action } => match action {
            PackageAction::Install { package } => commands::install::run(&package),
            PackageAction::Remove { package } => commands::remove::run(&package),
            PackageAction::List => commands::install::list(),
            PackageAction::Rollback => commands::remove::rollback(),
        },
        Commands::Search { query } => commands::search::run(&query),
        Commands::Info { package } => commands::info::run(&package),
        Commands::Update => commands::update::run(),
        Commands::Config { action } => match action {
            ConfigAction::Build => commands::config::build(),
            ConfigAction::Switch => commands::config::switch(),
            ConfigAction::Rollback => commands::config::rollback(),
            ConfigAction::List => commands::config::list_generations(),
        },
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
