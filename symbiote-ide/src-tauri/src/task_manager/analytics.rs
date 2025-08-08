use crate::task_manager::core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task analytics and reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalytics {
    pub total_tasks: usize,
    pub tasks_by_status: HashMap<TaskStatus, usize>,
    pub tasks_by_priority: HashMap<TaskPriority, usize>,
    pub tasks_by_type: HashMap<String, usize>,
    pub tasks_by_assignee: HashMap<String, usize>,
    pub overdue_tasks: usize,
    pub blocked_tasks: usize,
    pub completion_rate: f32,
    pub average_completion_time: Option<f32>, // in hours
    pub burndown_data: Vec<BurndownPoint>,
    pub velocity_data: Vec<VelocityPoint>,
}

/// Project analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalytics {
    pub project_id: String,
    pub project_name: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub in_progress_tasks: usize,
    pub blocked_tasks: usize,
    pub overdue_tasks: usize,
    pub completion_percentage: f32,
    pub estimated_hours: f32,
    pub actual_hours: f32,
    pub time_variance: f32, // percentage difference
    pub team_performance: HashMap<String, TeamMemberStats>,
}

/// Team member performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberStats {
    pub assigned_tasks: usize,
    pub completed_tasks: usize,
    pub completion_rate: f32,
    pub average_completion_time: Option<f32>,
    pub total_hours_logged: f32,
}

/// Burndown chart data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurndownPoint {
    pub date: u64,
    pub remaining_tasks: usize,
    pub remaining_hours: f32,
    pub ideal_remaining: f32,
}

/// Velocity chart data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityPoint {
    pub period: String, // e.g., "2024-W01" for week 1
    pub completed_tasks: usize,
    pub completed_hours: f32,
    pub story_points: Option<f32>,
}

/// Task trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTrends {
    pub creation_trend: Vec<TrendPoint>,
    pub completion_trend: Vec<TrendPoint>,
    pub priority_distribution_trend: Vec<PriorityTrendPoint>,
    pub bottleneck_analysis: Vec<BottleneckPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: u64,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityTrendPoint {
    pub date: u64,
    pub priority_counts: HashMap<TaskPriority, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckPoint {
    pub status: TaskStatus,
    pub average_time_in_status: f32, // hours
    pub task_count: usize,
}

/// Analytics engine for task management
pub struct TaskAnalyticsEngine;

impl TaskAnalyticsEngine {
    pub fn new() -> Self {
        Self
    }
    
    /// Generate comprehensive task analytics
    pub fn generate_analytics(
        &self,
        tasks: &HashMap<String, Task>,
        projects: &HashMap<String, Project>,
        time_entries: &[TimeEntry],
    ) -> TaskAnalytics {
        let total_tasks = tasks.len();
        
        // Tasks by status
        let mut tasks_by_status = HashMap::new();
        for status in [TaskStatus::Todo, TaskStatus::InProgress, TaskStatus::Blocked, TaskStatus::Review, TaskStatus::Done, TaskStatus::Cancelled] {
            tasks_by_status.insert(status, 0);
        }
        
        // Tasks by priority
        let mut tasks_by_priority = HashMap::new();
        for priority in [TaskPriority::Low, TaskPriority::Medium, TaskPriority::High, TaskPriority::Critical] {
            tasks_by_priority.insert(priority, 0);
        }
        
        let mut tasks_by_type = HashMap::new();
        let mut tasks_by_assignee = HashMap::new();
        let mut overdue_tasks = 0;
        let mut blocked_tasks = 0;
        let mut completed_tasks = 0;
        
        let current_time = current_timestamp();
        
        for task in tasks.values() {
            // Count by status
            *tasks_by_status.entry(task.status.clone()).or_insert(0) += 1;
            
            // Count by priority
            *tasks_by_priority.entry(task.priority).or_insert(0) += 1;
            
            // Count by type
            let type_name = match &task.task_type {
                TaskType::Custom(name) => name.clone(),
                other => format!("{:?}", other),
            };
            *tasks_by_type.entry(type_name).or_insert(0) += 1;
            
            // Count by assignee
            if let Some(ref assignee) = task.assigned_to {
                *tasks_by_assignee.entry(assignee.clone()).or_insert(0) += 1;
            }
            
            // Check if overdue
            if let Some(due_date) = task.due_date {
                if due_date < current_time && task.status != TaskStatus::Done {
                    overdue_tasks += 1;
                }
            }
            
            // Check if blocked
            if task.status == TaskStatus::Blocked || !task.blockers.is_empty() {
                blocked_tasks += 1;
            }
            
            // Count completed tasks
            if task.status == TaskStatus::Done {
                completed_tasks += 1;
            }
        }
        
        let completion_rate = if total_tasks > 0 {
            (completed_tasks as f32 / total_tasks as f32) * 100.0
        } else {
            0.0
        };
        
        // Calculate average completion time
        let average_completion_time = self.calculate_average_completion_time(tasks);
        
        // Generate burndown data (last 30 days)
        let burndown_data = self.generate_burndown_data(tasks, 30);
        
        // Generate velocity data (last 12 weeks)
        let velocity_data = self.generate_velocity_data(tasks, time_entries, 12);
        
        TaskAnalytics {
            total_tasks,
            tasks_by_status,
            tasks_by_priority,
            tasks_by_type,
            tasks_by_assignee,
            overdue_tasks,
            blocked_tasks,
            completion_rate,
            average_completion_time,
            burndown_data,
            velocity_data,
        }
    }
    
    /// Generate project-specific analytics
    pub fn generate_project_analytics(
        &self,
        project_id: &str,
        tasks: &HashMap<String, Task>,
        projects: &HashMap<String, Project>,
        time_entries: &[TimeEntry],
    ) -> Option<ProjectAnalytics> {
        let project = projects.get(project_id)?;
        
        // Filter tasks for this project
        let project_tasks: Vec<&Task> = tasks.values()
            .filter(|task| task.project_id.as_ref() == Some(&project_id.to_string()))
            .collect();
        
        let total_tasks = project_tasks.len();
        let completed_tasks = project_tasks.iter()
            .filter(|task| task.status == TaskStatus::Done)
            .count();
        let in_progress_tasks = project_tasks.iter()
            .filter(|task| task.status == TaskStatus::InProgress)
            .count();
        let blocked_tasks = project_tasks.iter()
            .filter(|task| task.status == TaskStatus::Blocked || !task.blockers.is_empty())
            .count();
        
        let current_time = current_timestamp();
        let overdue_tasks = project_tasks.iter()
            .filter(|task| {
                task.due_date.map_or(false, |due| due < current_time && task.status != TaskStatus::Done)
            })
            .count();
        
        let completion_percentage = if total_tasks > 0 {
            (completed_tasks as f32 / total_tasks as f32) * 100.0
        } else {
            0.0
        };
        
        let estimated_hours: f32 = project_tasks.iter()
            .filter_map(|task| task.estimated_hours)
            .sum();
        
        let actual_hours: f32 = project_tasks.iter()
            .filter_map(|task| task.actual_hours)
            .sum();
        
        let time_variance = if estimated_hours > 0.0 {
            ((actual_hours - estimated_hours) / estimated_hours) * 100.0
        } else {
            0.0
        };
        
        // Team performance analysis
        let mut team_performance = HashMap::new();
        let mut assignee_tasks: HashMap<String, Vec<&Task>> = HashMap::new();
        
        for task in &project_tasks {
            if let Some(ref assignee) = task.assigned_to {
                assignee_tasks.entry(assignee.clone()).or_default().push(task);
            }
        }
        
        for (assignee, tasks) in assignee_tasks {
            let assigned_count = tasks.len();
            let completed_count = tasks.iter()
                .filter(|task| task.status == TaskStatus::Done)
                .count();
            let completion_rate = if assigned_count > 0 {
                (completed_count as f32 / assigned_count as f32) * 100.0
            } else {
                0.0
            };
            
            // Calculate average completion time for this assignee
            let completion_times: Vec<f32> = tasks.iter()
                .filter(|task| task.status == TaskStatus::Done)
                .filter_map(|task| {
                    if let (Some(start), Some(actual)) = (task.start_date, task.actual_hours) {
                        Some(actual)
                    } else {
                        None
                    }
                })
                .collect();
            
            let average_completion_time = if !completion_times.is_empty() {
                Some(completion_times.iter().sum::<f32>() / completion_times.len() as f32)
            } else {
                None
            };
            
            // Calculate total hours logged
            let total_hours_logged: f32 = time_entries.iter()
                .filter(|entry| entry.user_id == assignee)
                .filter(|entry| {
                    tasks.iter().any(|task| task.id == entry.task_id)
                })
                .filter_map(|entry| entry.duration_hours)
                .sum();
            
            team_performance.insert(assignee.clone(), TeamMemberStats {
                assigned_tasks: assigned_count,
                completed_tasks: completed_count,
                completion_rate,
                average_completion_time,
                total_hours_logged,
            });
        }
        
        Some(ProjectAnalytics {
            project_id: project_id.to_string(),
            project_name: project.name.clone(),
            total_tasks,
            completed_tasks,
            in_progress_tasks,
            blocked_tasks,
            overdue_tasks,
            completion_percentage,
            estimated_hours,
            actual_hours,
            time_variance,
            team_performance,
        })
    }
    
    /// Generate task trends analysis
    pub fn generate_trends(
        &self,
        tasks: &HashMap<String, Task>,
        days: u32,
    ) -> TaskTrends {
        let current_time = current_timestamp();
        let start_time = current_time - (days as u64 * 24 * 60 * 60);
        
        // Creation trend
        let mut creation_trend = Vec::new();
        let mut completion_trend = Vec::new();
        let mut priority_distribution_trend = Vec::new();
        
        for day in 0..days {
            let day_start = start_time + (day as u64 * 24 * 60 * 60);
            let day_end = day_start + (24 * 60 * 60);
            
            // Count tasks created on this day
            let created_count = tasks.values()
                .filter(|task| task.created_at >= day_start && task.created_at < day_end)
                .count();
            
            creation_trend.push(TrendPoint {
                date: day_start,
                count: created_count,
            });
            
            // Count tasks completed on this day (approximation based on updated_at)
            let completed_count = tasks.values()
                .filter(|task| {
                    task.status == TaskStatus::Done &&
                    task.updated_at >= day_start && task.updated_at < day_end
                })
                .count();
            
            completion_trend.push(TrendPoint {
                date: day_start,
                count: completed_count,
            });
            
            // Priority distribution for tasks created on this day
            let mut priority_counts = HashMap::new();
            for priority in [TaskPriority::Low, TaskPriority::Medium, TaskPriority::High, TaskPriority::Critical] {
                let count = tasks.values()
                    .filter(|task| {
                        task.created_at >= day_start && task.created_at < day_end &&
                        task.priority == priority
                    })
                    .count();
                priority_counts.insert(priority, count);
            }
            
            priority_distribution_trend.push(PriorityTrendPoint {
                date: day_start,
                priority_counts,
            });
        }
        
        // Bottleneck analysis
        let bottleneck_analysis = self.analyze_bottlenecks(tasks);
        
        TaskTrends {
            creation_trend,
            completion_trend,
            priority_distribution_trend,
            bottleneck_analysis,
        }
    }
    
    /// Calculate average completion time
    fn calculate_average_completion_time(&self, tasks: &HashMap<String, Task>) -> Option<f32> {
        let completion_times: Vec<f32> = tasks.values()
            .filter(|task| task.status == TaskStatus::Done)
            .filter_map(|task| task.actual_hours)
            .collect();
        
        if completion_times.is_empty() {
            None
        } else {
            Some(completion_times.iter().sum::<f32>() / completion_times.len() as f32)
        }
    }
    
    /// Generate burndown chart data
    fn generate_burndown_data(&self, tasks: &HashMap<String, Task>, days: u32) -> Vec<BurndownPoint> {
        let current_time = current_timestamp();
        let start_time = current_time - (days as u64 * 24 * 60 * 60);
        
        let mut burndown_data = Vec::new();
        
        for day in 0..=days {
            let day_time = start_time + (day as u64 * 24 * 60 * 60);
            
            // Count remaining tasks at this point in time
            let remaining_tasks = tasks.values()
                .filter(|task| {
                    task.created_at <= day_time &&
                    (task.status != TaskStatus::Done || task.updated_at > day_time)
                })
                .count();
            
            // Calculate remaining hours
            let remaining_hours: f32 = tasks.values()
                .filter(|task| {
                    task.created_at <= day_time &&
                    (task.status != TaskStatus::Done || task.updated_at > day_time)
                })
                .filter_map(|task| task.estimated_hours)
                .sum();
            
            // Calculate ideal remaining (linear burndown)
            let total_tasks = tasks.len();
            let ideal_remaining = if days > 0 {
                total_tasks as f32 * (1.0 - (day as f32 / days as f32))
            } else {
                total_tasks as f32
            };
            
            burndown_data.push(BurndownPoint {
                date: day_time,
                remaining_tasks,
                remaining_hours,
                ideal_remaining,
            });
        }
        
        burndown_data
    }
    
    /// Generate velocity chart data
    fn generate_velocity_data(
        &self,
        tasks: &HashMap<String, Task>,
        time_entries: &[TimeEntry],
        weeks: u32,
    ) -> Vec<VelocityPoint> {
        let current_time = current_timestamp();
        let mut velocity_data = Vec::new();
        
        for week in 0..weeks {
            let week_start = current_time - ((week + 1) as u64 * 7 * 24 * 60 * 60);
            let week_end = current_time - (week as u64 * 7 * 24 * 60 * 60);
            
            // Count completed tasks in this week
            let completed_tasks = tasks.values()
                .filter(|task| {
                    task.status == TaskStatus::Done &&
                    task.updated_at >= week_start && task.updated_at < week_end
                })
                .count();
            
            // Calculate completed hours from time entries
            let completed_hours: f32 = time_entries.iter()
                .filter(|entry| {
                    entry.start_time >= week_start && entry.start_time < week_end
                })
                .filter_map(|entry| entry.duration_hours)
                .sum();
            
            // Generate period string (e.g., "2024-W01")
            let period = format!("Week-{}", weeks - week);
            
            velocity_data.push(VelocityPoint {
                period,
                completed_tasks,
                completed_hours,
                story_points: None, // Could be calculated if story points are tracked
            });
        }
        
        velocity_data.reverse(); // Show oldest to newest
        velocity_data
    }
    
    /// Analyze workflow bottlenecks
    fn analyze_bottlenecks(&self, tasks: &HashMap<String, Task>) -> Vec<BottleneckPoint> {
        let mut status_times: HashMap<TaskStatus, Vec<f32>> = HashMap::new();
        
        // This is a simplified analysis - in a real implementation,
        // you'd track state transitions and time spent in each state
        for task in tasks.values() {
            // Estimate time in current status based on last update
            let current_time = current_timestamp();
            let time_in_status = if task.updated_at > 0 {
                ((current_time - task.updated_at) as f32) / 3600.0 // Convert to hours
            } else {
                0.0
            };
            
            status_times.entry(task.status.clone()).or_default().push(time_in_status);
        }
        
        let mut bottlenecks = Vec::new();
        
        for (status, times) in status_times {
            if !times.is_empty() {
                let average_time = times.iter().sum::<f32>() / times.len() as f32;
                bottlenecks.push(BottleneckPoint {
                    status,
                    average_time_in_status: average_time,
                    task_count: times.len(),
                });
            }
        }
        
        // Sort by average time (highest first - these are the bottlenecks)
        bottlenecks.sort_by(|a, b| b.average_time_in_status.partial_cmp(&a.average_time_in_status).unwrap());
        
        bottlenecks
    }
}
