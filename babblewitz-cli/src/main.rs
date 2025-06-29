use crate::commands::tasks::{can_parse, deserialization};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod core;

#[derive(Parser)]
#[command(name = "babblewitz")]
#[command(about = "A CLI runner for Clausewitz/Jomini parser conformance and performance tests")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum TaskType {
    /// Run can-parse conformance tests
    CanParse {
        /// Path to implementation directory (if omitted, runs against all impls)
        #[arg(short, long)]
        implementation: Option<PathBuf>,
        /// Output format (table, github)
        #[arg(long, default_value_t = Format::Table)]
        format: Format,
    },
    /// Run deserialization performance tests
    Deserialization {
        /// Path to implementation directory (if omitted, runs against all impls)
        #[arg(short, long)]
        implementation: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Run tasks (can-parse, deserialization)
    Task {
        #[command(subcommand)]
        task_type: TaskType,
    },
    /// Build all impls to verify they compile
    Build {
        /// Optional specific implementation directory to build
        #[arg(short, long)]
        implementation: Option<PathBuf>,
    },
    /// Sync remote assets from S3, downloading if local files don't match
    SyncAssets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Table,
    Github,
}

impl Format {
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Table => "table",
            Format::Github => "github",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Task { task_type } => match task_type {
            TaskType::CanParse {
                implementation,
                format,
            } => match implementation {
                Some(impl_path) => {
                    let table = can_parse::run_can_parse_tests(&impl_path)?;
                    match format {
                        Format::Github => can_parse::print_github_summary(&table),
                        Format::Table => can_parse::print_can_parse_table(&table),
                    }

                    can_parse::print_failure_details(&table);
                }
                None => {
                    println!("Running can-parse tests across all implementations...");
                    let table = can_parse::run_all_can_parse()?;
                    match format {
                        Format::Github => can_parse::print_github_summary(&table),
                        Format::Table => can_parse::print_can_parse_table(&table),
                    }

                    can_parse::print_failure_details(&table);
                }
            },
            TaskType::Deserialization { implementation } => match implementation {
                Some(impl_path) => {
                    let results = deserialization::run_impl_benchmarks(&impl_path)?;
                    deserialization::print_benchmark_results(&results)?;
                }
                None => {
                    println!("Running deserialization benchmarks across all implementations...");
                    let table = deserialization::run_benchmark_table()?;
                    deserialization::print_benchmark_table(&table);
                }
            },
        },
        Commands::Build { implementation } => match implementation {
            Some(impl_path) => {
                println!("Building implementation: {}", impl_path.display());
                commands::build::build_implementation(&impl_path)?;
            }
            None => {
                println!("Building all impls...");
                commands::build::build_all_implementations()?;
            }
        },
        Commands::SyncAssets => {
            commands::sync_assets::sync_assets()?;
        }
    }

    Ok(())
}
