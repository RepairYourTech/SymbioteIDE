//! # Workspace Components
//! 
//! Workspace-specific components including journals, tasks, notes, analytics, and goals.

use super::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};

/// Workspace journal system for daily logs and project documentation
#[derive(Debug, Clone)]
pub struct WorkspaceJournal {
    pub workspace_id: String,
    pub daily_entries: HashMap<NaiveDate, JournalEntry>,
    pub project_journals: HashMap<String, ProjectJournal>,
    pub research_notes: HashMap<String, ResearchNote>,
    pub quick_notes: Vec<QuickNote>,
    pub settings: JournalSettings,
}

/// Daily journal entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub date: NaiveDate,
    pub title: String,
    pub content: String,
    pub mood: Option<Mood>,
    pub productivity_score: Option<f64>,
    pub accomplishments: Vec<String>,
    pub challenges: Vec<String>,
    pub tomorrow_goals: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Project-specific journal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectJournal {
    pub id: String,
    pub project_name: String,
    pub description: String,
    pub entries: Vec<ProjectJournalEntry>,
    pub milestones: Vec<ProjectMilestone>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Project journal entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectJournalEntry {
    pub id: String,
    pub title: String,
    pub content: String,
    pub entry_type: ProjectEntryType,
    pub tags: Vec<String>,
    pub attachments: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Research note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchNote {
    pub id: String,
    pub title: String,
    pub content: String,
    pub sources: Vec<ResearchSource>,
    pub topics: Vec<String>,
    pub status: ResearchStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Quick note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNote {
    pub id: String,
    pub content: String,
    pub color: Option<String>,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
}

/// Workspace task management system
#[derive(Debug, Clone)]
pub struct WorkspaceTaskManager {
    pub workspace_id: String,
    pub tasks: HashMap<String, Task>,
    pub task_lists: HashMap<String, TaskList>,
    pub projects: HashMap<String, TaskProject>,
    pub labels: HashMap<String, String>, // Simplified for now
    pub settings: TaskSettings,
}

/// Individual task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub assigned_to: Option<String>,
    pub project_id: Option<String>,
    pub list_id: Option<String>,
    pub labels: Vec<String>,
    pub subtasks: Vec<String>,
    pub dependencies: Vec<String>,
    pub time_estimate: Option<u32>, // minutes
    pub time_spent: u32, // minutes
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Task list/board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub task_ids: Vec<String>,
    pub list_type: TaskListType,
    pub created_at: DateTime<Utc>,
}

/// Task project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProject {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub task_ids: Vec<String>,
    pub team_members: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Workspace notes system
#[derive(Debug, Clone)]
pub struct WorkspaceNotes {
    pub workspace_id: String,
    pub notes: HashMap<String, Note>,
    pub notebooks: HashMap<String, Notebook>,
    pub tags: HashMap<String, NoteTag>,
    pub settings: NotesSettings,
}

/// Individual note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub note_type: NoteType,
    pub notebook_id: Option<String>,
    pub tags: Vec<String>,
    pub attachments: Vec<String>,
    pub links: Vec<NoteLink>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Note notebook/collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub note_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Workspace analytics system
#[derive(Debug, Clone)]
pub struct WorkspaceAnalytics {
    pub workspace_id: String,
    pub productivity_metrics: ProductivityMetrics,
    pub time_tracking: TimeTracking,
    pub code_metrics: CodeMetrics,
    pub collaboration_metrics: CollaborationMetrics,
    pub goal_progress: GoalProgress,
    pub reports: HashMap<String, AnalyticsReport>,
}

/// Productivity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityMetrics {
    pub daily_scores: HashMap<NaiveDate, f64>,
    pub weekly_averages: HashMap<String, f64>, // week_key -> score
    pub focus_time: HashMap<NaiveDate, u32>, // minutes
    pub interruptions: HashMap<NaiveDate, u32>,
    pub tasks_completed: HashMap<NaiveDate, u32>,
    pub code_commits: HashMap<NaiveDate, u32>,
    pub last_updated: DateTime<Utc>,
    // Additional fields for compatibility
    pub lines_of_code: u64,
    pub commits_per_day: f64,
    pub files_modified: u64,
    pub build_success_rate: f64,
    pub test_coverage: f64,
}

/// Time tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTracking {
    pub time_entries: Vec<TimeEntry>,
    pub daily_totals: HashMap<NaiveDate, u32>, // minutes
    pub project_time: HashMap<String, u32>, // project_id -> minutes
    pub activity_breakdown: HashMap<String, u32>, // activity -> minutes
}

/// Time entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_minutes: u32,
    pub activity_type: ActivityType,
    pub created_at: DateTime<Utc>,
}

/// Workspace goals system
#[derive(Debug, Clone)]
pub struct WorkspaceGoals {
    pub workspace_id: String,
    pub goals: HashMap<String, Goal>,
    pub milestones: HashMap<String, Milestone>,
    pub okrs: HashMap<String, OKR>, // Objectives and Key Results
    pub settings: GoalSettings,
}

/// Individual goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub goal_type: GoalType,
    pub status: GoalStatus,
    pub priority: GoalPriority,
    pub target_date: Option<NaiveDate>,
    pub progress: f64, // 0.0 to 1.0
    pub metrics: Vec<GoalMetric>,
    pub milestones: Vec<String>,
    pub related_tasks: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Goal milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub goal_id: String,
    pub title: String,
    pub description: Option<String>,
    pub target_date: NaiveDate,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// OKR (Objectives and Key Results)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OKR {
    pub id: String,
    pub objective: String,
    pub key_results: Vec<KeyResult>,
    pub quarter: String,
    pub year: u32,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
}

/// Key result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyResult {
    pub id: String,
    pub description: String,
    pub target_value: f64,
    pub current_value: f64,
    pub unit: String,
    pub progress: f64,
}

// Enums and supporting types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mood {
    VeryHappy,
    Happy,
    Neutral,
    Sad,
    VerySad,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectEntryType {
    Progress,
    Decision,
    Meeting,
    Research,
    Issue,
    Solution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResearchStatus {
    Active,
    Completed,
    OnHold,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSource {
    pub title: String,
    pub url: Option<String>,
    pub author: Option<String>,
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Review,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskListType {
    Simple,
    Kanban,
    Calendar,
    Timeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteType {
    Text,
    Markdown,
    Code,
    Meeting,
    Research,
    Idea,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteLink {
    pub target_id: String,
    pub target_type: String, // note, task, file, etc.
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteTag {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    Coding,
    Meeting,
    Research,
    Planning,
    Review,
    Documentation,
    Testing,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    Personal,
    Project,
    Team,
    Learning,
    Performance,
    Quality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalMetric {
    pub name: String,
    pub target_value: f64,
    pub current_value: f64,
    pub unit: String,
}

// Settings structures
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JournalSettings {
    pub daily_reminders: bool,
    pub mood_tracking: bool,
    pub productivity_tracking: bool,
    pub auto_save: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskSettings {
    pub default_priority: TaskPriority,
    pub auto_archive_completed: bool,
    pub time_tracking_enabled: bool,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotesSettings {
    pub auto_save: bool,
    pub link_detection: bool,
    pub tag_suggestions: bool,
    pub backup_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalSettings {
    pub progress_reminders: bool,
    pub milestone_notifications: bool,
    pub okr_enabled: bool,
    pub auto_progress_calculation: bool,
}

// Placeholder types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeMetrics;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollaborationMetrics;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalProgress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsReport {
    pub id: String,
    pub name: String,
    pub report_type: String,
    pub data: serde_json::Value,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMilestone {
    pub id: String,
    pub title: String,
    pub target_date: NaiveDate,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

// Implementation methods for workspace components
impl WorkspaceJournal {
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            daily_entries: HashMap::new(),
            project_journals: HashMap::new(),
            research_notes: HashMap::new(),
            quick_notes: Vec::new(),
            settings: JournalSettings::default(),
        }
    }

    pub fn add_daily_entry(&mut self, date: NaiveDate, entry: JournalEntry) {
        self.daily_entries.insert(date, entry);
    }

    pub fn get_daily_entry(&self, date: &NaiveDate) -> Option<&JournalEntry> {
        self.daily_entries.get(date)
    }

    pub fn add_quick_note(&mut self, content: String) -> String {
        let note = QuickNote {
            id: Uuid::new_v4().to_string(),
            content,
            color: None,
            pinned: false,
            created_at: Utc::now(),
        };
        let id = note.id.clone();
        self.quick_notes.push(note);
        id
    }
}

impl WorkspaceTaskManager {
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            tasks: HashMap::new(),
            task_lists: HashMap::new(),
            projects: HashMap::new(),
            labels: HashMap::new(),
            settings: TaskSettings::default(),
        }
    }

    pub fn create_task(&mut self, title: String, description: Option<String>) -> String {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title,
            description,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            due_date: None,
            assigned_to: None,
            project_id: None,
            list_id: None,
            labels: Vec::new(),
            subtasks: Vec::new(),
            dependencies: Vec::new(),
            time_estimate: None,
            time_spent: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };
        let id = task.id.clone();
        self.tasks.insert(id.clone(), task);
        id
    }

    pub fn complete_task(&mut self, task_id: &str) -> Result<()> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = TaskStatus::Done;
            task.completed_at = Some(Utc::now());
            task.updated_at = Utc::now();
            Ok(())
        } else {
            Err(SymbioteError::NotFound("Task not found".to_string()))
        }
    }
}

impl WorkspaceNotes {
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            notes: HashMap::new(),
            notebooks: HashMap::new(),
            tags: HashMap::new(),
            settings: NotesSettings::default(),
        }
    }

    pub fn create_note(&mut self, title: String, content: String, note_type: NoteType) -> String {
        let note = Note {
            id: Uuid::new_v4().to_string(),
            title,
            content,
            note_type,
            notebook_id: None,
            tags: Vec::new(),
            attachments: Vec::new(),
            links: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let id = note.id.clone();
        self.notes.insert(id.clone(), note);
        id
    }
}

impl WorkspaceAnalytics {
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            productivity_metrics: ProductivityMetrics {
                daily_scores: HashMap::new(),
                weekly_averages: HashMap::new(),
                focus_time: HashMap::new(),
                interruptions: HashMap::new(),
                tasks_completed: HashMap::new(),
                code_commits: HashMap::new(),
                last_updated: Utc::now(),
                lines_of_code: 0,
                commits_per_day: 0.0,
                files_modified: 0,
                build_success_rate: 0.0,
                test_coverage: 0.0,
            },
            time_tracking: TimeTracking {
                time_entries: Vec::new(),
                daily_totals: HashMap::new(),
                project_time: HashMap::new(),
                activity_breakdown: HashMap::new(),
            },
            code_metrics: CodeMetrics::default(),
            collaboration_metrics: CollaborationMetrics::default(),
            goal_progress: GoalProgress::default(),
            reports: HashMap::new(),
        }
    }

    pub fn record_productivity_score(&mut self, date: NaiveDate, score: f64) {
        self.productivity_metrics.daily_scores.insert(date, score);
        self.productivity_metrics.last_updated = Utc::now();
    }

    pub fn start_time_tracking(&mut self, description: String, activity_type: ActivityType) -> String {
        let entry = TimeEntry {
            id: Uuid::new_v4().to_string(),
            description,
            project_id: None,
            task_id: None,
            start_time: Utc::now(),
            end_time: None,
            duration_minutes: 0,
            activity_type,
            created_at: Utc::now(),
        };
        let id = entry.id.clone();
        self.time_tracking.time_entries.push(entry);
        id
    }
}

impl WorkspaceGoals {
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            goals: HashMap::new(),
            milestones: HashMap::new(),
            okrs: HashMap::new(),
            settings: GoalSettings::default(),
        }
    }

    pub fn create_goal(&mut self, title: String, description: Option<String>, goal_type: GoalType) -> String {
        let goal = Goal {
            id: Uuid::new_v4().to_string(),
            title,
            description,
            goal_type,
            status: GoalStatus::Draft,
            priority: GoalPriority::Medium,
            target_date: None,
            progress: 0.0,
            metrics: Vec::new(),
            milestones: Vec::new(),
            related_tasks: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let id = goal.id.clone();
        self.goals.insert(id.clone(), goal);
        id
    }

    pub fn update_goal_progress(&mut self, goal_id: &str, progress: f64) -> Result<()> {
        if let Some(goal) = self.goals.get_mut(goal_id) {
            goal.progress = progress.clamp(0.0, 1.0);
            goal.updated_at = Utc::now();
            if goal.progress >= 1.0 {
                goal.status = GoalStatus::Completed;
            }
            Ok(())
        } else {
            Err(SymbioteError::NotFound("Goal not found".to_string()))
        }
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Medium
    }
}
