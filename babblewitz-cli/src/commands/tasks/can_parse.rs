use crate::core::common::{calculate_impl_width, print_table_header};
use crate::core::config::TaskType;
use crate::core::corpus;
use crate::core::executor::ExecutionResult;
use crate::core::executor::ImplementationExecutor;
use crate::core::implementation::Implementation;
use crate::core::savefile::Game;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct CanParseGameResult {
    pub game: Game,
    pub total_tests: usize,
    pub passed_tests: usize,
}

impl CanParseGameResult {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            total_tests: 0,
            passed_tests: 0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tests > 0 {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanParseFileResult {
    pub implementation: String,
    pub game: Game,
    pub success_rate: f64,
}

#[derive(Debug)]
pub struct FailureDetail {
    pub implementation: String,
    pub corpus_file: String,
    pub error_message: String,
}

#[derive(Debug)]
pub struct ResultsTable {
    pub results: Vec<CanParseFileResult>,
    pub implementations: Vec<String>,
    pub games: Vec<Game>,
    pub failures: Vec<FailureDetail>,
}

pub fn run_can_parse_tests(implementation_path: &Path) -> Result<ResultsTable> {
    let implementation = Implementation::load_from_path(implementation_path)?;

    let mut all_results = Vec::new();
    let mut all_failures = Vec::new();

    process_implementation_can_parse(&implementation, &mut all_results, &mut all_failures)?;

    // Derive games from results
    let mut games: Vec<Game> = all_results
        .iter()
        .map(|r| r.game)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    games.sort();

    Ok(ResultsTable {
        results: all_results,
        implementations: vec![implementation.name.clone()],
        games,
        failures: all_failures,
    })
}

fn run_can_parase_tests_with_implementation(
    implementation: &Implementation,
    failures: &mut Vec<FailureDetail>,
) -> Result<Vec<CanParseGameResult>> {
    let games_to_test = implementation.games_for_task(TaskType::CanParse);
    let all_corpus_files = corpus::collect_relevant_corpus_files(&games_to_test)?;

    let mut game_results = games_to_test
        .iter()
        .copied()
        .map(|game| (game, CanParseGameResult::new(game)))
        .collect::<HashMap<_, _>>();

    let executor = match ImplementationExecutor::build_implementation(implementation) {
        Ok(executor) => executor,
        Err(e) => {
            failures.push(FailureDetail {
                implementation: implementation.name.clone(),
                corpus_file: String::from("build"),
                error_message: e.to_string(),
            });
            return Ok(Vec::new());
        }
    };

    for corpus_file in all_corpus_files {
        // Find which of our target games this file applies to
        let applicable_games: Vec<Game> = games_to_test
            .iter()
            .filter(|game| corpus_file.games.contains(game))
            .copied()
            .collect();

        if applicable_games.is_empty() {
            continue;
        }

        for game in &applicable_games {
            game_results.get_mut(game).unwrap().total_tests += 1;
        }

        let corpus_file_name = corpus_file
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut add_failure = |error_msg: String| {
            failures.push(FailureDetail {
                implementation: implementation.name.clone(),
                corpus_file: corpus_file_name.clone(),
                error_message: error_msg,
            });
        };

        match executor.execute(&corpus_file.content, TaskType::CanParse, &applicable_games) {
            Ok(ExecutionResult::Success { .. }) => {
                for game in applicable_games {
                    game_results.get_mut(&game).unwrap().passed_tests += 1;
                }
            }
            Ok(ExecutionResult::Error { error }) => add_failure(error),
            Err(error) => add_failure(error.to_string()),
        }
    }

    let mut results: Vec<_> = game_results.into_values().collect();
    results.sort_by_key(|r| r.game);
    Ok(results)
}

/// Process can parse tests for a single implementation
fn process_implementation_can_parse(
    implementation: &Implementation,
    all_results: &mut Vec<CanParseFileResult>,
    all_failures: &mut Vec<FailureDetail>,
) -> Result<()> {
    let results = run_can_parase_tests_with_implementation(implementation, all_failures)?;

    for result in results {
        all_results.push(CanParseFileResult {
            implementation: implementation.name.clone(),
            game: result.game,
            success_rate: result.success_rate(),
        });
    }

    Ok(())
}

/// Run can parse tests across all implementations and return table data
pub fn run_all_can_parse() -> Result<ResultsTable> {
    let implementations =
        crate::core::implementation::find_implementations_for_task(TaskType::CanParse)?;

    let mut all_results = Vec::new();
    let mut all_failures = Vec::new();

    // Process each implementation
    for implementation in &implementations {
        process_implementation_can_parse(implementation, &mut all_results, &mut all_failures)?;
    }

    // Pick out all the games we tested
    let mut games: Vec<Game> = all_results
        .iter()
        .map(|r| r.game)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    games.sort();

    let implementation_names: Vec<String> = implementations
        .iter()
        .map(|impl_| impl_.name.clone())
        .collect();

    Ok(ResultsTable {
        results: all_results,
        implementations: implementation_names,
        games,
        failures: all_failures,
    })
}

/// Print can parse results as a GitHub-friendly markdown summary
pub fn print_github_summary(table: &ResultsTable) {
    // Group results by implementation for display
    let mut impl_results: HashMap<String, HashMap<Game, &CanParseFileResult>> = HashMap::new();
    for result in &table.results {
        impl_results
            .entry(result.implementation.clone())
            .or_default()
            .insert(result.game, result);
    }

    println!(
        "| Implementation | {} |",
        table
            .games
            .iter()
            .map(|g| g.to_string().to_uppercase())
            .collect::<Vec<_>>()
            .join(" | ")
    );
    println!("|{}|", vec!["---"; table.games.len() + 1].join("|"));

    for impl_name in &table.implementations {
        print!("| **{}** |", impl_name);

        let game_results = impl_results
            .get(impl_name)
            .expect("Implementation should have results");

        for game in &table.games {
            // Not all implementations support all games
            let display_value = match game_results.get(game) {
                Some(result) if result.success_rate >= 100.0 => " ✅",
                Some(_) => " ⚠️",
                None => " ",
            };

            print!(" {} |", display_value);
        }
        println!();
    }

    println!();
    println!(
        "_Last updated: {}_",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!();
    println!("<!-- conformance-results -->");
}

/// Print detailed failure logs after the table
pub fn print_failure_details(table: &ResultsTable) {
    if table.failures.is_empty() {
        return;
    }

    println!("\nFailed corpus files:");
    for failure in &table.failures {
        println!(
            "{} ({}): {}",
            failure.implementation, failure.corpus_file, failure.error_message
        );
    }
}

/// Print can parse results as a table
pub fn print_can_parse_table(table: &ResultsTable) {
    let max_impl_width = calculate_impl_width(&table.implementations);
    let game_col_width = 10;

    // Print header
    let game_strings: Vec<String> = table.games.iter().map(|g| g.to_string()).collect();
    print_table_header(max_impl_width, &game_strings, game_col_width);

    // Print data rows
    for impl_name in &table.implementations {
        print!("{:<width$} ", impl_name, width = max_impl_width);

        let game_results = table
            .results
            .iter()
            .filter(|r| r.implementation == *impl_name)
            .map(|r| (r.game, r))
            .collect::<HashMap<_, _>>();

        for game in &table.games {
            // Not all implementations support all games
            let display_value = match game_results.get(game) {
                Some(result) if result.success_rate >= 100.0 => String::from("✓"),
                Some(result) => format!("{:.0}%", result.success_rate),
                None => String::from(""),
            };
            print!("{:>width$} ", display_value, width = game_col_width);
        }
        println!();
    }
}
