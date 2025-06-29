use anyhow::Context;
use clap::{Parser, ValueEnum};
use jomini::{JominiDeserialize, TextDeserializer};
use serde::Deserialize;
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
    #[value(alias("deserialization"))]
    Deserialization,
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
        Task::CanParse => {
            let mut count = 0;
            let mut reader = jomini::text::TokenReader::from_slice(&content);
            loop {
                match reader.next() {
                    Ok(Some(_)) => {
                        count += 1;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        writeln!(output, "-1")?;
                        break;
                    }
                }
            }

            writeln!(output, "{}", count)?;
        }
        Task::Deserialization => {
            #[derive(Debug, JominiDeserialize)]
            struct Gamestate {
                #[jomini(duplicated)]
                active_war: Vec<ActiveWar>,
            }

            #[derive(Debug, Deserialize)]
            struct ActiveWar {
                name: String,
            }

            let reader = jomini::text::TokenReader::from_slice(&content["EU4txt".len()..]);
            let data: Gamestate = TextDeserializer::from_windows1252_reader(reader)
                .deserialize()
                .context("unable to deserialize")?;
            let max_war = data
                .active_war
                .iter()
                .max_by(|a, b| a.name.len().cmp(&b.name.len()));
            match max_war {
                None => writeln!(output, "-1")?,
                Some(max_war) => writeln!(output, "{}", max_war.name)?,
            };
        }
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
