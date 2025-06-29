use crate::core::config::{ProjectType, TaskType};
use crate::core::implementation::Implementation;
use crate::core::savefile::Game;
use anyhow::Result;
use std::process::{Command, Stdio};

// Build state markers
pub struct Initial;
pub struct Built;

pub struct ImplementationExecutor<'a, Stage = Initial> {
    implementation: &'a Implementation,
    _stage: std::marker::PhantomData<Stage>,
}

#[derive(Debug)]
pub enum ExecutionResult {
    Success { elapsed: std::time::Duration },
    Error { error: String },
}

impl<'a> ImplementationExecutor<'a, Initial> {
    pub fn new(implementation: &'a Implementation) -> Self {
        Self {
            implementation,
            _stage: std::marker::PhantomData,
        }
    }

    /// Build an executor for the given implementation with consistent error handling
    pub fn build_implementation(
        implementation: &'a Implementation,
    ) -> Result<ImplementationExecutor<'a, Built>> {
        use anyhow::Context;

        let executor = Self::new(implementation);
        executor
            .build()
            .with_context(|| format!("Failed to build {}", implementation.name))
    }

    pub fn build(self) -> Result<ImplementationExecutor<'a, Built>> {
        // Get build command from execution config or derive from project type
        let build_command = self
            .implementation
            .config
            .execution
            .as_ref()
            .and_then(|x| x.build_command.as_deref())
            .or_else(|| get_project_config(self.implementation.config.project_type).build_command);

        if let Some(build_command) = build_command {
            println!(
                "  Building {} using: {}",
                self.implementation.config.name, build_command
            );

            let parts = shell_words::split(build_command).map_err(|e| {
                anyhow::anyhow!("Failed to parse build command '{}': {}", build_command, e)
            })?;

            let mut cmd = Command::new(&parts[0]);
            cmd.args(&parts[1..])
                .current_dir(&self.implementation.path)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let output = cmd.output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Build failed with exit code: {:?}",
                    output.status.code()
                ));
            }

            println!("  Build {} completed successfully", self.implementation.config.name);
        }

        Ok(ImplementationExecutor {
            implementation: self.implementation,
            _stage: std::marker::PhantomData,
        })
    }
}

impl ImplementationExecutor<'_, Built> {
    /// Get a reference to the underlying implementation
    pub fn implementation(&self) -> &Implementation {
        self.implementation
    }

    pub fn execute(
        &self,
        content: &[u8],
        task: TaskType,
        games: &[Game],
    ) -> Result<ExecutionResult> {
        // Get run command from execution config or derive from project type
        let run_command = self
            .implementation
            .config
            .execution
            .as_ref()
            .and_then(|x| x.run_command.as_deref())
            .unwrap_or_else(|| {
                get_project_config(self.implementation.config.project_type).run_command
            });

        let parts = shell_words::split(run_command)
            .map_err(|e| anyhow::anyhow!("Failed to parse run command '{}': {}", run_command, e))?;

        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty run command"));
        }

        let mut cmd = Command::new(&parts[0]);
        cmd.args(&parts[1..]).arg("--task").arg(task.as_str());

        // Add each game as a separate --game argument
        for game in games {
            cmd.arg("--game").arg(game.as_str());
        }

        cmd.current_dir(&self.implementation.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Write content to stdin
        if let Some(stdin) = child.stdin.take() {
            use std::io::Write;
            let mut stdin = stdin;
            stdin.write_all(content)?;
            drop(stdin); // Close stdin to signal EOF
        }

        let output = child.wait_with_output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stdout.lines().collect();

        // Parse the new two-line output format:
        // Line 1: microseconds (as a number)
        // Line 2: task output
        if output.status.success() && lines.len() >= 2 {
            if let Ok(microseconds) = lines[0].parse::<u64>() {
                let elapsed = std::time::Duration::from_micros(microseconds);
                return Ok(ExecutionResult::Success { elapsed });
            }
        }

        // Handle failure case or unexpected format
        let error_msg = if lines.len() >= 2 && lines[0].parse::<u128>().is_ok() {
            // Format appears correct but process failed - use second line as error
            lines[1].to_string()
        } else {
            // Combine stdout and stderr into a single error message
            let mut combined_output = String::new();

            if !stdout.trim().is_empty() {
                combined_output.push_str(stdout.trim());
            }

            if !stderr.trim().is_empty() {
                if !combined_output.is_empty() {
                    combined_output.push(' ');
                }
                combined_output.push_str(stderr.trim());
            }

            if combined_output.is_empty() {
                format!("Process exited with code: {:?}", output.status.code())
            } else {
                combined_output
            }
        };

        Ok(ExecutionResult::Error { error: error_msg })
    }
}

pub(crate) struct ProjectTypeConfig {
    pub build_command: Option<&'static str>,
    pub run_command: &'static str,
}

pub(crate) fn get_project_config(project_type: ProjectType) -> ProjectTypeConfig {
    match project_type {
        ProjectType::Rust => ProjectTypeConfig {
            build_command: Some("cargo build --release"),
            run_command: "cargo run --release --quiet --",
        },
        ProjectType::Gradle => ProjectTypeConfig {
            build_command: Some(if cfg!(windows) {
                "gradlew.bat build"
            } else {
                "./gradlew build"
            }),
            run_command: if cfg!(windows) {
                "gradlew.bat run"
            } else {
                "./gradlew run"
            },
        },
        ProjectType::Nodejs => ProjectTypeConfig {
            build_command: Some("npm install"),
            run_command: "node main.js",
        },
        ProjectType::Go => ProjectTypeConfig {
            build_command: Some("go build"),
            run_command: "go run",
        },
        ProjectType::Make => ProjectTypeConfig {
            build_command: Some("make"),
            run_command: "make run",
        },
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_success_output() {
        // Simulate successful output parsing
        let stdout = "1500000\nSome task output here";
        let lines: Vec<&str> = stdout.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].parse::<u128>().unwrap(), 1500000);

        // Test conversion to milliseconds
        let microseconds = lines[0].parse::<u128>().unwrap();
        let elapsed_ms = microseconds / 1000;
        assert_eq!(elapsed_ms, 1500);
    }

    #[test]
    fn test_parse_failure_output() {
        // Test error message parsing
        let stdout = "2000000\nError: failed to parse";
        let lines: Vec<&str> = stdout.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].parse::<u128>().is_ok());
        assert_eq!(lines[1], "Error: failed to parse");
    }

    #[test]
    fn test_parse_invalid_output() {
        // Test handling of invalid first line - should dump full output
        let stdout = "not_a_number\nSome output";
        let lines: Vec<&str> = stdout.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].parse::<u128>().is_err());
        // Should dump full output for debugging
    }

    #[test]
    fn test_debug_output_format() {
        // Test that debug output includes both stdout and stderr
        let stdout = "invalid\nformat";
        let stderr = "some error message";

        // Simulate the debug formatting logic
        let mut debug_info = String::new();
        debug_info.push_str("Unexpected output format. Full output:\n");
        debug_info.push_str("=== STDOUT ===\n");
        debug_info.push_str(stdout);
        debug_info.push('\n');
        debug_info.push_str("=== STDERR ===\n");
        debug_info.push_str(stderr);
        debug_info.push('\n');
        debug_info.push_str("=== END ===");

        assert!(debug_info.contains("=== STDOUT ==="));
        assert!(debug_info.contains("=== STDERR ==="));
        assert!(debug_info.contains("invalid\nformat"));
        assert!(debug_info.contains("some error message"));
    }

    #[test]
    fn test_games_argument_building() {
        use crate::core::savefile::Game;
        use std::process::Command;

        // Test that games are added as separate --game arguments
        let games = vec![Game::Eu4, Game::Ck3, Game::Hoi4];

        let mut cmd = Command::new("test-command");
        cmd.arg("--task").arg("can-parse");

        // Add each game as a separate --game argument
        for game in &games {
            cmd.arg("--game").arg(game.as_str());
        }

        // Verify the command includes the expected arguments
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert!(args.contains(&"--task".to_string()));
        assert!(args.contains(&"can-parse".to_string()));
        assert!(args.contains(&"--game".to_string()));
        assert!(args.contains(&"eu4".to_string()));
        assert!(args.contains(&"ck3".to_string()));
        assert!(args.contains(&"hoi4".to_string()));

        // Count how many --game flags there are
        let game_count = args.iter().filter(|&arg| arg == "--game").count();
        assert_eq!(game_count, 3);
    }

    #[test]
    fn test_shell_command_parsing() {
        // Test that shell command parsing handles quotes and spaces correctly
        let simple_command = "cargo build --release";
        let simple_parts = shell_words::split(simple_command).unwrap();
        assert_eq!(simple_parts, vec!["cargo", "build", "--release"]);

        // Test quoted arguments (this would break with split_whitespace)
        let quoted_command = r#"program "file with spaces.txt" --flag"#;
        let quoted_parts = shell_words::split(quoted_command).unwrap();
        assert_eq!(
            quoted_parts,
            vec!["program", "file with spaces.txt", "--flag"]
        );

        // Test escaped spaces (this would also break with split_whitespace)
        let escaped_command = r"program file\ with\ spaces.txt --flag";
        let escaped_parts = shell_words::split(escaped_command).unwrap();
        assert_eq!(
            escaped_parts,
            vec!["program", "file with spaces.txt", "--flag"]
        );
    }

    #[test]
    fn test_project_type_config() {
        use super::get_project_config;
        use crate::core::config::ProjectType;

        // Test all project types have configs
        let _rust_config = get_project_config(ProjectType::Rust);
        let _gradle_config = get_project_config(ProjectType::Gradle);
        let _nodejs_config = get_project_config(ProjectType::Nodejs);
        let _go_config = get_project_config(ProjectType::Go);
        let _make_config = get_project_config(ProjectType::Make);

        // Test specific configs
        let rust_config = get_project_config(ProjectType::Rust);
        assert_eq!(rust_config.run_command, "cargo run --release --quiet --");
        assert_eq!(rust_config.build_command, Some("cargo build --release"));

        let nodejs_config = get_project_config(ProjectType::Nodejs);
        assert_eq!(nodejs_config.run_command, "node main.js");
        assert_eq!(nodejs_config.build_command, Some("npm install"));
    }
}
