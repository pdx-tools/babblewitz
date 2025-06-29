use crate::core::savefile::Game;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Expand game aliases like "all" to their full game lists
fn expand_game_aliases(games_part: &str) -> Result<Vec<Game>> {
    let mut expanded_games = Vec::new();

    for token in games_part.split_whitespace() {
        match token {
            "all" => {
                // Add all supported games
                expanded_games.extend([
                    Game::Eu4,
                    Game::Ck3,
                    Game::Hoi4,
                    Game::Vic3,
                    Game::Imperator,
                    Game::Stellaris,
                ]);
            }
            game => {
                if let Some(parsed_game) = Game::from_str(game) {
                    expanded_games.push(parsed_game);
                } else {
                    return Err(anyhow::anyhow!("Unrecognized game: '{}'", game));
                }
            }
        }
    }

    // Remove duplicates while preserving order
    let mut unique_games = Vec::new();
    for game in expanded_games {
        if !unique_games.contains(&game) {
            unique_games.push(game);
        }
    }

    Ok(unique_games)
}

#[derive(Debug, Clone)]
pub struct CorpusFile {
    pub path: PathBuf,
    pub games: Vec<Game>,
    pub content: Vec<u8>,
}

/// Parse a corpus file directive from content bytes and extract games list and content
pub fn parse_corpus_content(content_bytes: &[u8]) -> Result<(Vec<Game>, Vec<u8>)> {
    // Try to parse as UTF-8 first for directive parsing
    let content_str = String::from_utf8_lossy(content_bytes);
    let lines: Vec<&str> = content_str.lines().collect();

    if lines.is_empty() {
        return Ok((vec![], content_bytes.to_vec()));
    }

    // Check if first line is a games directive
    let first_line = lines[0].trim();
    if first_line.starts_with("# @babblewitz:games:") {
        // Parse the games list
        let games_part = first_line
            .strip_prefix("# @babblewitz:games:")
            .unwrap_or("")
            .trim();
        let games: Vec<Game> = expand_game_aliases(games_part)?;

        // Content is everything after the first line (as bytes)
        let content_without_directive = if lines.len() > 1 {
            // Find the position after the first newline in the original bytes
            let mut split_pos = 0;
            for (i, &byte) in content_bytes.iter().enumerate() {
                if byte == b'\n' {
                    split_pos = i + 1;
                    break;
                }
                if byte == b'\r' {
                    split_pos = i + 1;
                    // Check for CRLF
                    if i + 1 < content_bytes.len() && content_bytes[i + 1] == b'\n' {
                        split_pos = i + 2;
                    }
                    break;
                }
            }
            content_bytes[split_pos..].to_vec()
        } else {
            Vec::new()
        };

        Ok((games, content_without_directive))
    } else {
        // No directive found, return empty games list and full content
        Ok((vec![], content_bytes.to_vec()))
    }
}

/// Parse a corpus file directive and extract games list and content
pub fn parse_corpus_file(file_path: &Path) -> Result<CorpusFile> {
    let content_bytes = std::fs::read(file_path)?;
    let (games, content) = parse_corpus_content(&content_bytes)?;
    Ok(CorpusFile {
        path: file_path.to_path_buf(),
        games,
        content,
    })
}

/// Collect corpus files relevant to the specified games
pub fn collect_relevant_corpus_files(games_to_test: &[Game]) -> Result<Vec<CorpusFile>> {
    let corpus_dir = PathBuf::from("corpus").join("game");
    let mut all_corpus_files = Vec::new();

    // Walk through all files in corpus/game directory
    for entry in WalkDir::new(&corpus_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let corpus_file = parse_corpus_file(entry.path()).with_context(|| {
                format!("Failed to parse corpus file {}", entry.path().display())
            })?;

            // Only include files that are relevant to our games
            if corpus_file
                .games
                .iter()
                .any(|game| games_to_test.contains(game))
            {
                all_corpus_files.push(corpus_file);
            }
        }
    }

    anyhow::ensure!(
        !all_corpus_files.is_empty(),
        "No corpus files found in {}",
        corpus_dir.display()
    );

    Ok(all_corpus_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_game_aliases() {
        // Test "all" alias expansion
        let result = expand_game_aliases("all").unwrap();
        assert_eq!(
            result,
            vec![
                Game::Eu4,
                Game::Ck3,
                Game::Hoi4,
                Game::Vic3,
                Game::Imperator,
                Game::Stellaris
            ]
        );

        // Test specific games
        let result = expand_game_aliases("eu4 ck3").unwrap();
        assert_eq!(result, vec![Game::Eu4, Game::Ck3]);

        // Test mixed aliases and specific games
        let result = expand_game_aliases("all vic3").unwrap();
        assert_eq!(
            result,
            vec![
                Game::Eu4,
                Game::Ck3,
                Game::Hoi4,
                Game::Vic3,
                Game::Imperator,
                Game::Stellaris
            ]
        );

        // Test deduplication
        let result = expand_game_aliases("eu4 all eu4").unwrap();
        assert_eq!(
            result,
            vec![
                Game::Eu4,
                Game::Ck3,
                Game::Hoi4,
                Game::Vic3,
                Game::Imperator,
                Game::Stellaris
            ]
        );

        // Test empty input
        let result = expand_game_aliases("").unwrap();
        assert!(result.is_empty());

        // Test unrecognized game
        let result = expand_game_aliases("invalid_game");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unrecognized game: 'invalid_game'"));

        // Test mix of valid and invalid games
        let result = expand_game_aliases("eu4 invalid_game ck3");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unrecognized game: 'invalid_game'"));
    }

    #[test]
    fn test_parse_corpus_content_with_directive() {
        let content = b"# @babblewitz:games: eu4 ck3\ndate=1444.11.11\nplayer=\"FRA\"";
        let (games, parsed_content) = parse_corpus_content(content).unwrap();

        assert_eq!(games, vec![Game::Eu4, Game::Ck3]);
        assert_eq!(parsed_content, b"date=1444.11.11\nplayer=\"FRA\"");
    }

    #[test]
    fn test_parse_corpus_content_with_all_alias() {
        let content = b"# @babblewitz:games: all\ndate=1444.11.11\nplayer=\"FRA\"";
        let (games, parsed_content) = parse_corpus_content(content).unwrap();

        assert_eq!(
            games,
            vec![
                Game::Eu4,
                Game::Ck3,
                Game::Hoi4,
                Game::Vic3,
                Game::Imperator,
                Game::Stellaris
            ]
        );
        assert_eq!(parsed_content, b"date=1444.11.11\nplayer=\"FRA\"");
    }

    #[test]
    fn test_parse_corpus_content_without_directive() {
        let content = b"date=1444.11.11\nplayer=\"FRA\"";
        let (games, parsed_content) = parse_corpus_content(content).unwrap();

        assert!(games.is_empty());
        assert_eq!(parsed_content, content);
    }

    #[test]
    fn test_parse_corpus_content_only_directive() {
        let content = b"# @babblewitz:games: eu4";
        let (games, parsed_content) = parse_corpus_content(content).unwrap();

        assert_eq!(games, vec![Game::Eu4]);
        assert_eq!(parsed_content, b"");
    }

    #[test]
    fn test_parse_corpus_content_empty() {
        let content = b"";
        let (games, parsed_content) = parse_corpus_content(content).unwrap();

        assert!(games.is_empty());
        assert_eq!(parsed_content, b"");
    }

    #[test]
    fn test_parse_corpus_content_old_format_ignored() {
        let content = b"# @games: eu4 ck3\ndate=1444.11.11";
        let (games, parsed_content) = parse_corpus_content(content).unwrap();

        assert!(games.is_empty());
        assert_eq!(parsed_content, content);
    }

    #[test]
    fn test_parse_corpus_content_with_binary_content() {
        // Create content with some non-UTF8 bytes (simulating Windows-1252)
        let mut content = b"# @babblewitz:games: eu4\n".to_vec();
        content.extend_from_slice(b"name=\"M\xfcnchen\""); // ü in Windows-1252 is 0xfc

        let (games, parsed_content) = parse_corpus_content(&content).unwrap();
        assert_eq!(games, vec![Game::Eu4]);
        assert_eq!(parsed_content, b"name=\"M\xfcnchen\"");
    }

    #[test]
    fn test_parse_corpus_content_with_windows_1252_encoding() {
        // Create content with various Windows-1252 characters
        let mut content = b"# @babblewitz:games: all\r\n".to_vec(); // CRLF line ending
                                                                    // Add various Windows-1252 characters:
                                                                    // 0x80 = € (Euro sign), 0xc0 = À, 0xe9 = é, 0xf1 = ñ, 0xfc = ü
        content.extend_from_slice(b"currency=\"\x80\"\n");
        content.extend_from_slice(b"city=\"\xc0msterdam\"\n");
        content.extend_from_slice(b"name=\"Caf\xe9\"\n");
        content.extend_from_slice(b"country=\"Espa\xf1a\"\n");
        content.extend_from_slice(b"leader=\"M\xfcller\"\n");

        let (games, parsed_content) = parse_corpus_content(&content).unwrap();
        assert_eq!(
            games,
            vec![
                Game::Eu4,
                Game::Ck3,
                Game::Hoi4,
                Game::Vic3,
                Game::Imperator,
                Game::Stellaris
            ]
        );

        // Verify the content bytes are preserved exactly
        let expected_content = b"currency=\"\x80\"\ncity=\"\xc0msterdam\"\nname=\"Caf\xe9\"\ncountry=\"Espa\xf1a\"\nleader=\"M\xfcller\"\n";
        assert_eq!(parsed_content, expected_content);
    }

    #[test]
    fn test_parse_corpus_content_with_crlf() {
        // Test CRLF line ending handling
        let content = b"# @babblewitz:games: eu4 ck3\r\ndate=1444.11.11\r\nplayer=\"FRA\"";
        let (games, parsed_content) = parse_corpus_content(content).unwrap();

        assert_eq!(games, vec![Game::Eu4, Game::Ck3]);
        assert_eq!(parsed_content, b"date=1444.11.11\r\nplayer=\"FRA\"");
    }

    #[test]
    fn test_parse_corpus_content_with_invalid_game() {
        // Test that invalid games return an error
        let content = b"# @babblewitz:games: eu4 invalid_game\ndate=1444.11.11";
        let result = parse_corpus_content(content);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unrecognized game: 'invalid_game'"));
    }
}
