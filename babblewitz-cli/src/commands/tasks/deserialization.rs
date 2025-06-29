use crate::core::common::{calculate_impl_width, print_table_header};
use crate::core::config::TaskType;
use crate::core::executor::{Built, ExecutionResult, ImplementationExecutor};
use crate::core::implementation::Implementation;
use crate::core::savefile::{find_save_files, Game, SaveFile};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize)]
pub struct PerformanceResult {
    pub game: Game,
    pub test_type: String,
    pub total_files: usize,
    pub avg_throughput_mbps: f64,
    pub total_data_mb: f64,
    pub failed_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub game: Game,
    pub implementation: String,
    pub data_size_bytes: u64, // Size of data sent to stdin (uncompressed)
    pub result: FileTestResult,
}

#[derive(Debug, Clone)]
pub enum FileTestResult {
    Success { elapsed_ms: u128 },
    Failed,
}

#[derive(Debug)]
pub struct PerformanceTable {
    pub files: Vec<FileResult>,
    pub implementations: Vec<String>,
}

pub fn run_impl_benchmarks(implementation_path: &Path) -> Result<Vec<PerformanceResult>> {
    let implementation = Implementation::load_from_path(implementation_path)?;
    run_implementation_benchmarks(&implementation)
}

fn run_implementation_benchmarks(
    implementation: &Implementation,
) -> Result<Vec<PerformanceResult>> {
    let performance_tasks = &[TaskType::Deserialization];
    let mut results = Vec::new();
    for task in performance_tasks {
        let supported_games = implementation.games_for_task(*task);

        for game in supported_games {
            println!("Running {} benchmark for game: {}", task, game);
            let result = run_benchmark_with_implementation(implementation, &game, *task)?;
            results.push(result);
        }
    }

    Ok(results)
}

/// Core function to run benchmarks on save files with a given executor
fn run_benchmarks_on_files(
    executor: &ImplementationExecutor<'_, Built>,
    save_files: impl Iterator<Item = SaveFile>,
    task_type: TaskType,
) -> Result<Vec<FileResult>> {
    let mut file_results = Vec::new();

    for save_file in save_files {
        let file_data = save_file.read().with_context(|| {
            format!(
                "Failed to read save file: {}",
                save_file.file_path.display()
            )
        })?;
        let data_size_bytes = file_data.len() as u64;

        match executor.execute(&file_data, task_type, &[save_file.detected_game]) {
            Ok(result) => {
                let test_result = match result {
                    ExecutionResult::Success { elapsed } => FileTestResult::Success {
                        elapsed_ms: elapsed.as_millis(),
                    },
                    ExecutionResult::Error { .. } => FileTestResult::Failed,
                };

                file_results.push(FileResult {
                    game: save_file.detected_game,
                    implementation: executor.implementation().name.clone(),
                    data_size_bytes,
                    result: test_result,
                });
            }
            Err(_) => {
                file_results.push(FileResult {
                    game: save_file.detected_game,
                    implementation: executor.implementation().name.clone(),
                    data_size_bytes,
                    result: FileTestResult::Failed,
                });
            }
        }
    }

    Ok(file_results)
}

/// Ensure corpus directory exists and return the corpus path
fn ensure_corpus_directory_exists() -> Result<PathBuf> {
    let corpus_path = PathBuf::from("corpus").join("saves");

    // Check if corpus/saves directory exists, if not, run sync-assets
    if !corpus_path.exists() {
        println!("Corpus saves directory not found. Running sync-assets...");
        crate::commands::sync_assets::sync_assets()?;
    }

    Ok(corpus_path)
}

fn run_benchmark_with_implementation(
    implementation: &Implementation,
    game: &Game,
    task_type: TaskType,
) -> Result<PerformanceResult> {
    let executor = ImplementationExecutor::build_implementation(implementation)?;

    let corpus_path = ensure_corpus_directory_exists()?;

    let save_files = find_save_files(&corpus_path);
    let filtered_files: Vec<_> = save_files
        .filter(|save_file| &save_file.detected_game == game)
        .collect();

    println!("  Running actual performance measurements...");

    let file_results = run_benchmarks_on_files(&executor, filtered_files.into_iter(), task_type)?;

    let mut throughputs = Vec::new();
    let mut failed_files = Vec::new();
    let mut total_data_bytes = 0u64;
    let total_files = file_results.len();

    for result in file_results {
        total_data_bytes += result.data_size_bytes;

        match result.result {
            FileTestResult::Success { elapsed_ms } => {
                // Calculate throughput: MB/s
                let mb_size = result.data_size_bytes as f64 / (1024.0 * 1024.0);
                let seconds = elapsed_ms as f64 / 1000.0;
                let throughput = if seconds > 0.0 {
                    mb_size / seconds
                } else {
                    0.0
                };
                throughputs.push(throughput);
            }
            FileTestResult::Failed => {
                failed_files.push(format!("File failed: {:?}", result.game));
            }
        }
    }

    let avg_throughput_mbps = if throughputs.is_empty() {
        0.0
    } else {
        throughputs.iter().sum::<f64>() / throughputs.len() as f64
    };

    let total_data_mb = total_data_bytes as f64 / (1024.0 * 1024.0);

    Ok(PerformanceResult {
        game: *game,
        test_type: task_type.to_string(),
        total_files,
        avg_throughput_mbps,
        total_data_mb,
        failed_files,
    })
}

/// Run benchmark tests across all implementations and return table data
pub fn run_benchmark_table() -> Result<PerformanceTable> {
    // Find all implementations that support deserialization
    let implementations =
        crate::core::implementation::find_implementations_for_task(TaskType::Deserialization)?;

    // Ensure corpus assets are available
    let corpus_path = ensure_corpus_directory_exists()?;
    let save_files: Vec<_> = find_save_files(&corpus_path).collect();

    // Run tests for each implementation
    let mut all_file_results = Vec::new();

    for implementation in &implementations {
        println!("Testing implementation: {}", implementation.name);

        // Check which games this implementation supports
        let supported_games = implementation.games_for_task(TaskType::Deserialization);

        // Filter save files to only those with supported games
        let supported_files: Vec<_> = save_files
            .iter()
            .filter(|save_file| supported_games.contains(&save_file.detected_game))
            .cloned()
            .collect();

        if supported_files.is_empty() {
            continue;
        }

        // Build the executor once per implementation
        let executor = match ImplementationExecutor::build_implementation(implementation) {
            Ok(executor) => executor,
            Err(e) => {
                println!("  Failed to build {}: {}", implementation.name, e);
                // Add failed results for all files this implementation should support
                for save_file in &supported_files {
                    all_file_results.push(FileResult {
                        game: save_file.detected_game,
                        implementation: implementation.name.clone(),
                        data_size_bytes: 0,
                        result: FileTestResult::Failed,
                    });
                }
                continue;
            }
        };

        let file_results = run_benchmarks_on_files(
            &executor,
            supported_files.into_iter(),
            TaskType::Deserialization,
        )?;

        all_file_results.extend(file_results);
    }

    let implementation_names: Vec<String> = implementations
        .iter()
        .map(|impl_| impl_.name.clone())
        .collect();

    Ok(PerformanceTable {
        files: all_file_results,
        implementations: implementation_names,
    })
}

/// Print benchmark results as a table (transposed: implementations as rows, games as columns)
pub fn print_benchmark_table(table: &PerformanceTable) {
    // Extract unique games from the results directly
    let mut games_set = std::collections::HashSet::new();
    for result in &table.files {
        games_set.insert(result.game);
    }
    let mut games: Vec<_> = games_set.into_iter().collect();
    games.sort();

    // Group results by implementation and game for averaging
    let mut impl_game_results: HashMap<String, HashMap<Game, Vec<f64>>> = HashMap::new();
    let mut impl_game_failures: HashMap<String, HashMap<Game, bool>> = HashMap::new();

    for result in &table.files {
        let impl_name = &result.implementation;

        match &result.result {
            FileTestResult::Success { elapsed_ms } => {
                // Calculate throughput for successful results
                let mb_size = result.data_size_bytes as f64 / (1024.0 * 1024.0);
                let seconds = *elapsed_ms as f64 / 1000.0;
                let throughput = if seconds > 0.0 {
                    mb_size / seconds
                } else {
                    0.0
                };

                impl_game_results
                    .entry(impl_name.clone())
                    .or_default()
                    .entry(result.game)
                    .or_default()
                    .push(throughput);
            }
            FileTestResult::Failed => {
                // Mark this implementation/game combination as having failures
                impl_game_failures
                    .entry(impl_name.clone())
                    .or_default()
                    .insert(result.game, true);
            }
        }
    }

    // Calculate column widths
    let max_impl_width = calculate_impl_width(&table.implementations);
    let game_col_width = 12; // Fixed width for game columns

    // Print header
    print_table_header(max_impl_width, &games, game_col_width);

    // Print data rows
    for impl_name in &table.implementations {
        print!("{:<width$} ", impl_name, width = max_impl_width);

        // Print average throughput for each game, or caution emoji if there are failures
        for game in &games {
            let display_value = if let Some(failures) = impl_game_failures.get(impl_name) {
                if *failures.get(game).unwrap_or(&false) {
                    "⚠️".to_string()
                } else if let Some(game_results) = impl_game_results.get(impl_name) {
                    if let Some(throughputs) = game_results.get(game) {
                        if !throughputs.is_empty() {
                            let avg_throughput =
                                throughputs.iter().sum::<f64>() / throughputs.len() as f64;
                            format!("{:.1} MB/s", avg_throughput)
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                }
            } else if let Some(game_results) = impl_game_results.get(impl_name) {
                if let Some(throughputs) = game_results.get(game) {
                    if !throughputs.is_empty() {
                        let avg_throughput =
                            throughputs.iter().sum::<f64>() / throughputs.len() as f64;
                        format!("{:.1} MB/s", avg_throughput)
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            };

            print!("{:>width$} ", display_value, width = game_col_width);
        }
        println!();
    }
}

pub fn print_benchmark_results(results: &[PerformanceResult]) -> Result<()> {
    println!("=== BENCHMARK RESULTS ===");
    for result in results {
        println!("Game: {} ({})", result.game, result.test_type);
        println!("  Files tested: {}", result.total_files);
        println!("  Total data processed: {:.1} MB", result.total_data_mb);
        println!(
            "  Average throughput: {:.1} MB/s",
            result.avg_throughput_mbps
        );
        if !result.failed_files.is_empty() {
            println!("  Failed files: {}", result.failed_files.len());
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_test_result_types() {
        // Test Success result
        let success = FileTestResult::Success { elapsed_ms: 1500 };
        match success {
            FileTestResult::Success { elapsed_ms } => assert_eq!(elapsed_ms, 1500),
            _ => panic!("Expected Success variant"),
        }

        // Test Failed result
        let failed = FileTestResult::Failed;
        match failed {
            FileTestResult::Failed => {}
            _ => panic!("Expected Failed variant"),
        }
    }

    #[test]
    fn test_performance_table_structure() {
        let files = vec![
            FileResult {
                game: Game::Eu4,
                implementation: "jomini-reader".to_string(),
                data_size_bytes: 1024,
                result: FileTestResult::Success { elapsed_ms: 100 },
            },
            FileResult {
                game: Game::Ck3,
                implementation: "jomini-reader".to_string(),
                data_size_bytes: 2048,
                result: FileTestResult::Failed,
            },
        ];

        let table = PerformanceTable {
            files,
            implementations: vec!["jomini-reader".to_string()],
        };

        assert_eq!(table.files.len(), 2);
        assert_eq!(table.implementations.len(), 1);
    }
}
