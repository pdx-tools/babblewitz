use anyhow::Context;
use clap::{Parser, ValueEnum};
use jomini::TextTape;
use std::io::{Cursor, Read, Write};

#[derive(Parser)]
#[command(about = "Jomini implementation for Babblewitz testing")]
struct Cli {
    /// Task to perform (can-parse, deserialization, etc.)
    #[arg(short, long)]
    task: Task,
    /// Game tag associated with this content (can be specified multiple times)
    #[arg(short, long, action = clap::ArgAction::Append)]
    game: Vec<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Task {
    #[value(alias("can-parse"))]
    CanParse,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut stdin = std::io::stdin().lock();
    let mut content = Vec::new();
    stdin
        .read_to_end(&mut content)
        .context("failed to read stdin")?;

    let start = std::time::Instant::now();
    let mut output = Cursor::new(Vec::<u8>::new());

    match cli.task {
        Task::CanParse => match TextTape::from_slice(&content) {
            Ok(tape) => {
                writeln!(output, "{}", tape.tokens().len())?;
            }
            Err(_) => {
                writeln!(output, "-1")?;
            }
        },
    };

    let elapsed_us = start.elapsed().as_micros();
    println!("{}", elapsed_us);

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(&output.get_ref())
        .context("failed to write to stdout")?;
    Ok(())
}
