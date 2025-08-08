use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::process::{Child, Command as TokioCommand};
use tokio::io::{AsyncBufReadExt, BufReader};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use uuid::Uuid;

/// Symbiote Terminal - AI-Enhanced Terminal System
/// Our Warp killer with natural language commands, AI assistance, and workflow automation
#[derive(Debug, Clone)]
pub struct SymbioteTerminal {
    sessions: Arc<RwLock<HashMap<String, TerminalSession>>>,
    ai_assistant: Arc<RwLock<AITerminalAssistant>>,
    command_history: Arc<RwLock<Vec<CommandHistoryEntry>>>,
    workflows: Arc<RwLock<HashMap<String, TerminalWorkflow>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub name: String,
    pub working_directory: String,
    pub environment_vars: HashMap<String, String>,
    pub shell_type: ShellType,
    pub status: SessionStatus,
    pub created_at: String,
    pub last_activity: String,
    pub process_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellType {
    PowerShell,
    Cmd,
    Bash,
    Zsh,
    Fish,
    Nushell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Idle,
    Busy,
    Error,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    pub id: String,
    pub session_id: String,
    pub original_input: String,
    pub interpreted_command: Option<String>,
    pub natural_language: bool,
    pub execution_time_ms: u64,
    pub exit_code: Option<i32>,
    pub output: String,
    pub error_output: String,
    pub timestamp: String,
    pub ai_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalWorkflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub commands: Vec<WorkflowCommand>,
    pub triggers: Vec<WorkflowTrigger>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommand {
    pub command: String,
    pub natural_language_description: String,
    pub expected_output_pattern: Option<String>,
    pub error_recovery: Option<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowTrigger {
    FileChange(String),
    DirectoryChange(String),
    GitCommit,
    ErrorPattern(String),
    Schedule(String),
    Manual,
}

#[derive(Debug, Clone)]
pub struct AITerminalAssistant {
    pub enabled: bool,
    pub auto_suggest: bool,
    pub auto_correct: bool,
    pub explain_commands: bool,
    pub safety_checks: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NaturalLanguageRequest {
    pub input: String,
    pub context: TerminalContext,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalContext {
    pub working_directory: String,
    pub shell_type: ShellType,
    pub environment_vars: HashMap<String, String>,
    pub recent_commands: Vec<String>,
    pub project_type: Option<String>,
    pub git_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandSuggestion {
    pub command: String,
    pub explanation: String,
    pub safety_level: SafetyLevel,
    pub estimated_time: Option<String>,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SafetyLevel {
    Safe,
    Caution,
    Dangerous,
    RequiresConfirmation,
}

impl SymbioteTerminal {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            ai_assistant: Arc::new(RwLock::new(AITerminalAssistant {
                enabled: true,
                auto_suggest: true,
                auto_correct: false,
                explain_commands: true,
                safety_checks: true,
            })),
            command_history: Arc::new(RwLock::new(Vec::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new terminal session
    pub async fn create_session(&self, name: String, shell_type: ShellType) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let working_directory = std::env::current_dir()
            .map_err(|e| anyhow!("Failed to get current directory: {}", e))?
            .to_string_lossy()
            .to_string();

        let session = TerminalSession {
            id: session_id.clone(),
            name,
            working_directory,
            environment_vars: std::env::vars().collect(),
            shell_type,
            status: SessionStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_activity: chrono::Utc::now().to_rfc3339(),
            process_id: None,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        println!("🚀 Created new Symbiote Terminal session: {}", session_id);
        Ok(session_id)
    }

    /// Execute a command with AI assistance
    pub async fn execute_command(
        &self,
        session_id: &str,
        input: &str,
        natural_language: bool,
    ) -> Result<CommandHistoryEntry> {
        let start_time = std::time::Instant::now();
        
        // Get session
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id)
                .ok_or_else(|| anyhow!("Session not found: {}", session_id))?
                .clone()
        };

        // Process command through AI if needed
        let (final_command, ai_suggestions) = if natural_language {
            self.process_natural_language(input, &session).await?
        } else {
            let suggestions = self.get_ai_suggestions(input, &session).await?;
            (input.to_string(), suggestions)
        };

        // Execute the command
        let (output, error_output, exit_code) = self.execute_shell_command(&final_command, &session).await?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Create history entry
        let history_entry = CommandHistoryEntry {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            original_input: input.to_string(),
            interpreted_command: if natural_language { Some(final_command) } else { None },
            natural_language,
            execution_time_ms: execution_time,
            exit_code,
            output,
            error_output,
            timestamp: chrono::Utc::now().to_rfc3339(),
            ai_suggestions,
        };

        // Add to history
        let mut history = self.command_history.write().await;
        history.push(history_entry.clone());

        // Update session activity
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = chrono::Utc::now().to_rfc3339();
            session.status = if exit_code == Some(0) { SessionStatus::Idle } else { SessionStatus::Error };
        }

        Ok(history_entry)
    }

    /// Process natural language input into shell commands
    async fn process_natural_language(&self, input: &str, session: &TerminalSession) -> Result<(String, Vec<String>)> {
        // This would integrate with our AI provider system
        // For now, implement basic pattern matching and common command translation
        
        let command = match input.to_lowercase().as_str() {
            s if s.contains("list files") || s.contains("show files") => {
                match session.shell_type {
                    ShellType::PowerShell => "Get-ChildItem".to_string(),
                    ShellType::Cmd => "dir".to_string(),
                    _ => "ls -la".to_string(),
                }
            },
            s if s.contains("current directory") || s.contains("where am i") => {
                match session.shell_type {
                    ShellType::PowerShell => "Get-Location".to_string(),
                    ShellType::Cmd => "cd".to_string(),
                    _ => "pwd".to_string(),
                }
            },
            s if s.contains("git status") || s.contains("check git") => {
                "git status".to_string()
            },
            s if s.contains("install") && s.contains("npm") => {
                if let Some(package) = extract_package_name(s) {
                    format!("npm install {}", package)
                } else {
                    "npm install".to_string()
                }
            },
            s if s.contains("run") && s.contains("build") => {
                "npm run build".to_string()
            },
            s if s.contains("start") && s.contains("server") => {
                "npm start".to_string()
            },
            s if s.contains("create") && s.contains("folder") => {
                if let Some(folder_name) = extract_folder_name(s) {
                    match session.shell_type {
                        ShellType::PowerShell => format!("New-Item -ItemType Directory -Name '{}'", folder_name),
                        ShellType::Cmd => format!("mkdir {}", folder_name),
                        _ => format!("mkdir {}", folder_name),
                    }
                } else {
                    "mkdir new_folder".to_string()
                }
            },
            _ => {
                // Fallback: return original input with suggestion
                input.to_string()
            }
        };

        let suggestions = vec![
            "Use 'help' to see available commands".to_string(),
            "Try natural language like 'list files' or 'show git status'".to_string(),
        ];

        Ok((command, suggestions))
    }

    /// Get AI suggestions for a command
    async fn get_ai_suggestions(&self, command: &str, session: &TerminalSession) -> Result<Vec<String>> {
        let assistant = self.ai_assistant.read().await;
        
        if !assistant.enabled {
            return Ok(vec![]);
        }

        let mut suggestions = Vec::new();

        // Safety checks
        if assistant.safety_checks {
            if command.contains("rm -rf") || command.contains("del /s") || command.contains("Remove-Item -Recurse") {
                suggestions.push("⚠️ This command will delete files recursively. Use with caution!".to_string());
            }
            if command.contains("sudo") || command.contains("admin") {
                suggestions.push("⚠️ This command requires elevated privileges.".to_string());
            }
        }

        // Command explanations
        if assistant.explain_commands {
            match command.split_whitespace().next() {
                Some("git") => suggestions.push("💡 Git command for version control operations".to_string()),
                Some("npm") => suggestions.push("💡 Node.js package manager command".to_string()),
                Some("cargo") => suggestions.push("💡 Rust package manager and build tool".to_string()),
                Some("docker") => suggestions.push("💡 Container management command".to_string()),
                _ => {}
            }
        }

        // Auto-suggestions
        if assistant.auto_suggest {
            if command.starts_with("git") && !command.contains("status") {
                suggestions.push("💡 Try 'git status' to see repository state".to_string());
            }
            if command.starts_with("npm") && !command.contains("install") {
                suggestions.push("💡 Use 'npm install' to install dependencies".to_string());
            }
        }

        Ok(suggestions)
    }

    /// Execute a shell command
    async fn execute_shell_command(
        &self,
        command: &str,
        session: &TerminalSession,
    ) -> Result<(String, String, Option<i32>)> {
        let (shell_cmd, shell_args) = match session.shell_type {
            ShellType::PowerShell => ("powershell", vec!["-Command", command]),
            ShellType::Cmd => ("cmd", vec!["/C", command]),
            ShellType::Bash => ("bash", vec!["-c", command]),
            ShellType::Zsh => ("zsh", vec!["-c", command]),
            ShellType::Fish => ("fish", vec!["-c", command]),
            ShellType::Nushell => ("nu", vec!["-c", command]),
        };

        let mut child = TokioCommand::new(shell_cmd)
            .args(shell_args)
            .current_dir(&session.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn command: {}", e))?;

        let output = child.wait_with_output().await
            .map_err(|e| anyhow!("Failed to wait for command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();

        Ok((stdout, stderr, exit_code))
    }

    /// Create a workflow from command history
    pub async fn create_workflow_from_history(
        &self,
        name: String,
        description: String,
        command_ids: Vec<String>,
    ) -> Result<String> {
        let history = self.command_history.read().await;
        let mut workflow_commands = Vec::new();

        for command_id in command_ids {
            if let Some(entry) = history.iter().find(|e| e.id == command_id) {
                let workflow_command = WorkflowCommand {
                    command: entry.interpreted_command.as_ref()
                        .unwrap_or(&entry.original_input)
                        .clone(),
                    natural_language_description: if entry.natural_language {
                        entry.original_input.clone()
                    } else {
                        format!("Execute: {}", entry.original_input)
                    },
                    expected_output_pattern: None,
                    error_recovery: None,
                    timeout_seconds: 30,
                };
                workflow_commands.push(workflow_command);
            }
        }

        let workflow_id = Uuid::new_v4().to_string();
        let workflow = TerminalWorkflow {
            id: workflow_id.clone(),
            name,
            description,
            commands: workflow_commands,
            triggers: vec![WorkflowTrigger::Manual],
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id.clone(), workflow);

        println!("📋 Created workflow: {}", workflow_id);
        Ok(workflow_id)
    }

    /// Execute a workflow
    pub async fn execute_workflow(&self, workflow_id: &str, session_id: &str) -> Result<Vec<CommandHistoryEntry>> {
        let workflow = {
            let workflows = self.workflows.read().await;
            workflows.get(workflow_id)
                .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?
                .clone()
        };

        let mut results = Vec::new();

        for workflow_command in workflow.commands {
            println!("🔄 Executing workflow command: {}", workflow_command.natural_language_description);
            
            let result = self.execute_command(
                session_id,
                &workflow_command.command,
                false,
            ).await?;

            results.push(result);

            // Check for errors
            if let Some(exit_code) = results.last().unwrap().exit_code {
                if exit_code != 0 {
                    println!("❌ Workflow command failed with exit code: {}", exit_code);
                    if let Some(recovery) = &workflow_command.error_recovery {
                        println!("🔧 Attempting error recovery: {}", recovery);
                        self.execute_command(session_id, recovery, false).await?;
                    }
                }
            }
        }

        println!("✅ Workflow completed: {}", workflow.name);
        Ok(results)
    }

    /// Get session information
    pub async fn get_session(&self, session_id: &str) -> Result<TerminalSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))
            .map(|s| s.clone())
    }

    /// Get command history for a session
    pub async fn get_session_history(&self, session_id: &str) -> Vec<CommandHistoryEntry> {
        let history = self.command_history.read().await;
        history.iter()
            .filter(|entry| entry.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Get all workflows
    pub async fn get_workflows(&self) -> HashMap<String, TerminalWorkflow> {
        let workflows = self.workflows.read().await;
        workflows.clone()
    }
}

// Helper functions for natural language processing
fn extract_package_name(input: &str) -> Option<String> {
    // Simple pattern matching for package names
    if let Some(install_pos) = input.find("install") {
        let after_install = &input[install_pos + 7..];
        after_install.split_whitespace().next().map(|s| s.to_string())
    } else {
        None
    }
}

fn extract_folder_name(input: &str) -> Option<String> {
    // Simple pattern matching for folder names
    if let Some(folder_pos) = input.find("folder") {
        let after_folder = &input[folder_pos + 6..];
        after_folder.trim().split_whitespace().next().map(|s| s.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_session_creation() {
        let terminal = SymbioteTerminal::new();
        let session_id = terminal.create_session("test".to_string(), ShellType::PowerShell).await.unwrap();
        assert!(!session_id.is_empty());

        let session = terminal.get_session(&session_id).await.unwrap();
        assert_eq!(session.name, "test");
        assert_eq!(session.shell_type, ShellType::PowerShell);
    }

    #[tokio::test]
    async fn test_natural_language_processing() {
        let terminal = SymbioteTerminal::new();
        let session_id = terminal.create_session("test".to_string(), ShellType::Bash).await.unwrap();
        
        let result = terminal.execute_command(&session_id, "list files", true).await.unwrap();
        assert!(result.natural_language);
        assert_eq!(result.interpreted_command, Some("ls -la".to_string()));
    }

    #[tokio::test]
    async fn test_workflow_creation() {
        let terminal = SymbioteTerminal::new();
        let session_id = terminal.create_session("test".to_string(), ShellType::Bash).await.unwrap();
        
        // Execute some commands
        let cmd1 = terminal.execute_command(&session_id, "echo hello", false).await.unwrap();
        let cmd2 = terminal.execute_command(&session_id, "echo world", false).await.unwrap();
        
        // Create workflow
        let workflow_id = terminal.create_workflow_from_history(
            "Test Workflow".to_string(),
            "Test workflow description".to_string(),
            vec![cmd1.id, cmd2.id],
        ).await.unwrap();
        
        assert!(!workflow_id.is_empty());
        
        let workflows = terminal.get_workflows().await;
        assert!(workflows.contains_key(&workflow_id));
    }
}
