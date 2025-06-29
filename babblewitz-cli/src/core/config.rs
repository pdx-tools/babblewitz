use crate::core::savefile::Game;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskType {
    CanParse,
    Deserialization,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::CanParse => "can-parse",
            TaskType::Deserialization => "deserialization",
        }
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Rust,
    Gradle,
    Nodejs,
    Go,
    Make,
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Rust => "rust",
            ProjectType::Gradle => "gradle",
            ProjectType::Nodejs => "nodejs",
            ProjectType::Go => "go",
            ProjectType::Make => "make",
        }
    }
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImplementationConfig {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "project-type")]
    pub project_type: ProjectType,
    pub execution: Option<ExecutionConfig>,
    pub tasks: HashMap<TaskType, TaskConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionConfig {
    #[serde(rename = "build-command")]
    pub build_command: Option<String>,
    #[serde(rename = "run-command")]
    pub run_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskConfig {
    pub games: Vec<Game>,
}

impl ImplementationConfig {
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get all games supported for a specific task
    pub fn supported_games_for_task(&self, task: TaskType) -> Vec<Game> {
        self.tasks
            .get(&task)
            .map(|task_config| task_config.games.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_type_serde() {
        // Test serialization
        assert_eq!(
            serde_json::to_string(&ProjectType::Rust).unwrap(),
            "\"rust\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectType::Gradle).unwrap(),
            "\"gradle\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectType::Nodejs).unwrap(),
            "\"nodejs\""
        );
        assert_eq!(serde_json::to_string(&ProjectType::Go).unwrap(), "\"go\"");
        assert_eq!(
            serde_json::to_string(&ProjectType::Make).unwrap(),
            "\"make\""
        );

        // Test deserialization
        assert_eq!(
            serde_json::from_str::<ProjectType>("\"rust\"").unwrap(),
            ProjectType::Rust
        );
        assert_eq!(
            serde_json::from_str::<ProjectType>("\"gradle\"").unwrap(),
            ProjectType::Gradle
        );
        assert_eq!(
            serde_json::from_str::<ProjectType>("\"nodejs\"").unwrap(),
            ProjectType::Nodejs
        );
        assert_eq!(
            serde_json::from_str::<ProjectType>("\"go\"").unwrap(),
            ProjectType::Go
        );
        assert_eq!(
            serde_json::from_str::<ProjectType>("\"make\"").unwrap(),
            ProjectType::Make
        );

        // Test invalid value
        assert!(serde_json::from_str::<ProjectType>("\"invalid\"").is_err());
    }

    #[test]
    fn test_project_type_display() {
        assert_eq!(ProjectType::Rust.to_string(), "rust");
        assert_eq!(ProjectType::Gradle.to_string(), "gradle");
        assert_eq!(ProjectType::Nodejs.to_string(), "nodejs");
        assert_eq!(ProjectType::Go.to_string(), "go");
        assert_eq!(ProjectType::Make.to_string(), "make");
    }

    #[test]
    fn test_project_type_as_str() {
        assert_eq!(ProjectType::Rust.as_str(), "rust");
        assert_eq!(ProjectType::Gradle.as_str(), "gradle");
        assert_eq!(ProjectType::Nodejs.as_str(), "nodejs");
        assert_eq!(ProjectType::Go.as_str(), "go");
        assert_eq!(ProjectType::Make.as_str(), "make");
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_config = r#"
            name = "test-impl"
            project-type = "rust"
            
            [tasks]
        "#;

        let config: ImplementationConfig = toml::from_str(toml_config).unwrap();
        assert_eq!(config.project_type, ProjectType::Rust);
        assert_eq!(config.name, "test-impl");
    }

    #[test]
    fn test_task_config_with_games() {
        let toml_config = r#"
            name = "test-impl"
            project-type = "rust"
            
            [tasks.can-parse]
            games = ["eu4", "ck3", "hoi4"]
            
            [tasks.deserialization]
            games = ["vic3", "stellaris"]
        "#;

        let config: ImplementationConfig = toml::from_str(toml_config).unwrap();

        // Test can-parse task games
        let can_parse_games = config.supported_games_for_task(TaskType::CanParse);
        assert_eq!(can_parse_games.len(), 3);
        assert!(can_parse_games.contains(&Game::Eu4));
        assert!(can_parse_games.contains(&Game::Ck3));
        assert!(can_parse_games.contains(&Game::Hoi4));

        // Test deserialization task games
        let deser_games = config.supported_games_for_task(TaskType::Deserialization);
        assert_eq!(deser_games.len(), 2);
        assert!(deser_games.contains(&Game::Vic3));
        assert!(deser_games.contains(&Game::Stellaris));
    }

    #[test]
    fn test_task_config_invalid_game() {
        let toml_config = r#"
            name = "test-impl"
            project-type = "rust"
            
            [tasks.can-parse]
            games = ["eu4", "invalid-game", "ck3"]
        "#;

        // Should fail to deserialize due to invalid game name
        let result: Result<ImplementationConfig, _> = toml::from_str(toml_config);
        assert!(result.is_err());
    }
}
