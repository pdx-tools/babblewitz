use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    Ck3,
    Eu4,
    Hoi4,
    Imperator,
    Stellaris,
    Vic3,
}

impl Game {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "eu4" => Some(Game::Eu4),
            "ck3" => Some(Game::Ck3),
            "vic3" => Some(Game::Vic3),
            "hoi4" => Some(Game::Hoi4),
            "imperator" => Some(Game::Imperator),
            "stellaris" => Some(Game::Stellaris),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Game::Eu4 => "eu4",
            Game::Ck3 => "ck3",
            Game::Vic3 => "vic3",
            Game::Hoi4 => "hoi4",
            Game::Imperator => "imperator",
            Game::Stellaris => "stellaris",
        }
    }
}

impl AsRef<str> for Game {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Game {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Game::from_str(s).with_context(|| format!("Failed to parse game: '{}'", s))
    }
}

#[derive(Debug, Clone)]
pub struct SaveFile {
    pub file_path: PathBuf,
    pub detected_game: Game,
}

impl SaveFile {
    pub fn read(&self) -> Result<Vec<u8>> {
        read_save_content(&self.file_path)
    }
}

pub fn find_save_files<P: AsRef<Path>>(corpus_path: P) -> impl Iterator<Item = SaveFile> {
    WalkDir::new(corpus_path.as_ref())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let file_path = entry.path().to_path_buf();
            let detected_game = detect_game_from_directory(&file_path)?;

            Some(SaveFile {
                file_path,
                detected_game,
            })
        })
}

fn detect_game_from_directory(file_path: &Path) -> Option<Game> {
    let parent = file_path.parent()?;
    Game::from_str(&parent.file_name()?.to_string_lossy())
}

fn read_save_content(file_path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
    let mut buf = vec![0u8; rawzip::RECOMMENDED_BUFFER_SIZE];
    let Ok(archive) = rawzip::ZipArchive::from_file(file, &mut buf) else {
        return Ok(std::fs::read(file_path)?);
    };

    let mut entries = archive.entries(&mut buf);
    while let Some(entry) = entries.next_entry()? {
        if entry.is_dir() {
            continue;
        }

        let wayfinder = entry.wayfinder();
        let entry = archive
            .get_entry(wayfinder)
            .with_context(|| format!("Failed to get entry in zip: {}", file_path.display()))?;

        let reader = flate2::read::DeflateDecoder::new_with_buf(entry.reader(), buf);
        let mut reader = entry.verifying_reader(reader);
        let mut output = Vec::new();
        reader
            .read_to_end(&mut output)
            .with_context(|| format!("Failed to read entry in zip: {}", file_path.display()))?;
        return Ok(output);
    }
    anyhow::bail!(
        "No valid entries found in zip file: {}",
        file_path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_game_from_directory() {
        // Test EU4 game detection
        let path = PathBuf::from("corpus/saves/eu4/france_1444.eu4");
        assert_eq!(detect_game_from_directory(&path), Some(Game::Eu4));

        // Test CK3 game detection
        let path = PathBuf::from("corpus/saves/ck3/ireland_1066.ck3");
        assert_eq!(detect_game_from_directory(&path), Some(Game::Ck3));

        // Test absolute path
        let path = PathBuf::from("/home/user/projects/corpus/saves/hoi4/germany_1936.hoi4");
        assert_eq!(detect_game_from_directory(&path), Some(Game::Hoi4));

        let path = PathBuf::from("corpus/saves/vic3/test.vic3");
        assert_eq!(detect_game_from_directory(&path), Some(Game::Vic3));
    }

    #[test]
    fn test_read_file_content_regular_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"test content";
        fs::write(&file_path, content).unwrap();

        let result = read_save_content(&file_path).unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn test_game_enum() {
        // Test from_str
        assert_eq!(Game::from_str("eu4"), Some(Game::Eu4));
        assert_eq!(Game::from_str("EU4"), Some(Game::Eu4));
        assert_eq!(Game::from_str("ck3"), Some(Game::Ck3));
        assert_eq!(Game::from_str("vic3"), Some(Game::Vic3));
        assert_eq!(Game::from_str("hoi4"), Some(Game::Hoi4));
        assert_eq!(Game::from_str("imperator"), Some(Game::Imperator));
        assert_eq!(Game::from_str("stellaris"), Some(Game::Stellaris));
        assert_eq!(Game::from_str("unknown"), None);

        // Test as_str
        assert_eq!(Game::Eu4.as_str(), "eu4");
        assert_eq!(Game::Ck3.as_str(), "ck3");
        assert_eq!(Game::Vic3.as_str(), "vic3");
        assert_eq!(Game::Hoi4.as_str(), "hoi4");
        assert_eq!(Game::Imperator.as_str(), "imperator");
        assert_eq!(Game::Stellaris.as_str(), "stellaris");

        // Test Display trait
        assert_eq!(format!("{}", Game::Eu4), "eu4");
        assert_eq!(format!("{}", Game::Ck3), "ck3");
    }

    #[test]
    fn test_game_serde() {
        // Test serialization
        assert_eq!(serde_json::to_string(&Game::Eu4).unwrap(), "\"eu4\"");
        assert_eq!(serde_json::to_string(&Game::Ck3).unwrap(), "\"ck3\"");
        assert_eq!(serde_json::to_string(&Game::Vic3).unwrap(), "\"vic3\"");
        assert_eq!(serde_json::to_string(&Game::Hoi4).unwrap(), "\"hoi4\"");
        assert_eq!(
            serde_json::to_string(&Game::Imperator).unwrap(),
            "\"imperator\""
        );
        assert_eq!(
            serde_json::to_string(&Game::Stellaris).unwrap(),
            "\"stellaris\""
        );

        // Test deserialization
        assert_eq!(serde_json::from_str::<Game>("\"eu4\"").unwrap(), Game::Eu4);
        assert_eq!(serde_json::from_str::<Game>("\"ck3\"").unwrap(), Game::Ck3);
        assert_eq!(
            serde_json::from_str::<Game>("\"vic3\"").unwrap(),
            Game::Vic3
        );
        assert_eq!(
            serde_json::from_str::<Game>("\"hoi4\"").unwrap(),
            Game::Hoi4
        );
        assert_eq!(
            serde_json::from_str::<Game>("\"imperator\"").unwrap(),
            Game::Imperator
        );
        assert_eq!(
            serde_json::from_str::<Game>("\"stellaris\"").unwrap(),
            Game::Stellaris
        );

        // Test invalid value
        assert!(serde_json::from_str::<Game>("\"invalid\"").is_err());
    }
}
