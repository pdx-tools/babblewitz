use crate::core::config::{ImplementationConfig, TaskType};
use crate::core::savefile::Game;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Implementation {
    pub name: String,
    pub path: PathBuf,
    pub config: ImplementationConfig,
}

impl Implementation {
    /// Load an implementation from a directory path
    pub fn load_from_path<P: AsRef<Path>>(impl_path: P) -> Result<Self> {
        let path = impl_path.as_ref().to_path_buf();
        let config_path = path.join("babblewitz.config.toml");
        let config = ImplementationConfig::load_from_file(&config_path)
            .with_context(|| format!("Failed to load config from {}", config_path.display()))?;

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Ok(Implementation { name, path, config })
    }

    /// Check if this implementation supports a specific task
    pub fn supports_task(&self, task: TaskType) -> bool {
        !self.config.supported_games_for_task(task).is_empty()
    }

    /// Get all games supported by this implementation for a task
    pub fn games_for_task(&self, task: TaskType) -> Vec<Game> {
        self.config.supported_games_for_task(task)
    }
}

/// Find all implementations in the impls directory
pub fn find_all_implementations() -> Result<Vec<Implementation>> {
    let impls_dir = PathBuf::from("impls");
    let mut implementations = Vec::new();

    for entry in std::fs::read_dir(&impls_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let implementation = Implementation::load_from_path(&path).with_context(|| {
                format!("Failed to load implementation from {}", path.display())
            })?;
            implementations.push(implementation);
        }
    }

    if implementations.is_empty() {
        return Err(anyhow::anyhow!("No valid implementations found"));
    }

    implementations.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(implementations)
}

/// Find all implementations that support a specific task
pub fn find_implementations_for_task(task: TaskType) -> Result<Vec<Implementation>> {
    let all_implementations = find_all_implementations()?;

    let task_implementations: Vec<Implementation> = all_implementations
        .into_iter()
        .filter(|x| x.supports_task(task))
        .collect();

    anyhow::ensure!(
        !task_implementations.is_empty(),
        "No implementations found for task '{}'",
        task
    );
    Ok(task_implementations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_implementation(temp_dir: &Path, name: &str, config_content: &str) -> PathBuf {
        let impl_dir = temp_dir.join("impls").join(name);
        fs::create_dir_all(&impl_dir).unwrap();

        let config_path = impl_dir.join("babblewitz.config.toml");
        fs::write(&config_path, config_content).unwrap();

        impl_dir
    }

    #[test]
    fn test_implementation_load_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
            name = "test-impl"
            project-type = "rust"
            
            [tasks.can-parse]
            games = ["eu4", "ck3"]
            
            [tasks.deserialization]
            games = ["vic3"]
        "#;

        let impl_path = create_test_implementation(temp_dir.path(), "test-impl", config_content);

        let implementation = Implementation::load_from_path(&impl_path).unwrap();
        assert_eq!(implementation.name, "test-impl");
        assert_eq!(implementation.path, impl_path);
        assert_eq!(implementation.config.name, "test-impl");
    }

    #[test]
    fn test_implementation_supports_task() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
            name = "test-impl"
            project-type = "rust"
            
            [tasks.can-parse]
            games = ["eu4", "ck3"]
        "#;

        let impl_path = create_test_implementation(temp_dir.path(), "test-impl", config_content);
        let implementation = Implementation::load_from_path(&impl_path).unwrap();

        assert!(implementation.supports_task(TaskType::CanParse));
        assert!(!implementation.supports_task(TaskType::Deserialization));
    }

    #[test]
    fn test_load_nonexistent_implementation() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent");

        let result = Implementation::load_from_path(&nonexistent_path);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Failed to load config from") || error_msg.contains("No such file")
        );
    }

    #[test]
    fn test_load_implementation_without_config() {
        let temp_dir = TempDir::new().unwrap();
        let impl_dir = temp_dir.path().join("no-config");
        fs::create_dir_all(&impl_dir).unwrap();

        let result = Implementation::load_from_path(&impl_dir);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Failed to load config from") || error_msg.contains("No such file")
        );
    }
}
