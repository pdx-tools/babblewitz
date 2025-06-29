# Babblewitz

The Babblewitz project is an effort to qualify the ecosystem of Clausewitz / Jomini parsers for games from Paradox Development Studio like EU4, CK3, Vic3, HOI4, etc. As a proprietary format lacking a specification, a proliferation of parsers have emerged of varying syntactical support, performance, and use cases.

The Babblewitz project qualifies implementations along three axes:

- Conformance. The syntax between games vary, so it is important to see over what inputs a parser can successfully ingest. There is no standardized interpretation of a given input (and standardizing on one is out of scope for Babblewitz). Thus, the creation of the Babblewitz Corpus, a suite of test files categorized by game of origin.
- Performance. How fast can a parser ingest the given input and perform a task
- Ergonomics. It's not too hard to write a lexer that is extremely fast and can parse all inputs, however by itself, the lexed input is not too useful. So a metric is needed to measure the amount of effort required to accomplish a task.

There is no one size fits all parser. A parser designed for game files that focuses on syntax highlighting and semantic analysis, will be poorly suited for efficiently extracting information from a 200 MB save file.

And if no existing parser meets expectations (or one wants to write their own parser for educational purposes), the Babblewitz project can help one make an informed decision about what (if any) project can be used for inspiration and the corpus serves as a test suite.

The name Babblewitz is derived from the "babbling buffoon" in EU4 and the Clausewitz game engine. The name was inspired by the Babelmark project.

## Getting Started

Babblewitz uses [mise](https://mise.jdx.dev/) to configure a consistent environment to build and run implementations. Currently the only requirement is Rust, so mise is more of a formality, but as more implementations are added, mise will be able to help juggle languages and frameworks.

One installed, run the Babblewitz "can-parse" task:

```bash
cargo babblewitz task can-parse
```

## Babblewitz Corpus

Housed in the `corpus` directory are a collection of files that represent various syntax and values a parser will encounter.

The files are a simple format:

```
# @babblewitz:games: vic3 ck3 hoi4 stellaris
name="Jåhkåmåhkke"
```

The first line is a babblewitz directive that lists all the games that is associated with that input. In this case, games with UTF-8 encoding.

For universal syntax, the "all" alias can be used:

```
# @babblewitz:games: all
foo=bar
```

With an abundance of caution, actual game files are not included in this repository. The provided test files are synthetic examples covering common syntax patterns.

Due to the size of save files, they are stored remotely. To retrieve the save files, ensure that `rclone` is available (installed via `mise`) and run:

```bash
cargo babblewitz sync-assets
```

## Adding a New Implementation

An implementation accepts 3 parameter:

- The data to parse (stdin)
- The task to perform (eg: `--task can-parse`)
- A list of game tags as separate arguments representing the games associated with the input (eg: `--game eu4 --game hoi4`)

An implementation defines a configuration (`babblewitz.config.toml`) that dictates the project type (ie: how to build and run the project), and what tasks and games are supported.

Below is a simple example where a parser broadcasts only EU4 support.

```toml
name = "the-coolest-parser"
description = "Blah blah"
project_type = "rust"

[tasks]
[tasks.can-parse]
games = ["eu4"]

[tasks.deserialization]
games = ["eu4"]
```

An implementation has 2 lines of output:

- Line 1: Self reported length of time required to complete the task. Time is reported as whole microseconds. The self reporting is to assuage stdin ingestion and the startup cost of implementations with large runtimes.
- Line 2: The result of the task

## Tasks

### Can Parse

The `can-parse` task requires implementations to parse stdin data and output how many tokens or values were encountered (after outputting the duration in microseconds on the first line). There is no expected answer.

### Deserialization

The `deserialization` task requires implementations to extract information from a save file and outputs how fast each implementation can accomplish the task.

For EU4, the task is to print the longest name of a war from all currently active wars.

To avoid the `deserialization` benchmark timing different DEFLATE implementations, the input data sent via stdin is the single file text output from each game.
