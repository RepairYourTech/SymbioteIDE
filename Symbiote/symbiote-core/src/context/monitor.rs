//! # Context Health Monitor
//! 
//! Real-time monitoring and alerting for context health across the entire Symbiote ecosystem.

use crate::{Result, SymbioteError};
use super::{GlobalContext, tokenizer::{ContextTokenizer, ContextMetrics, ContextHealth}};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

/// Health alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Context health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAlert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub context_id: String,
    pub metrics: ContextMetrics,
    pub suggested_actions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub acknowledged: bool,
}

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Token usage warning threshold (percentage)
    pub warning_threshold: f64,
    /// Token usage critical threshold (percentage)
    pub critical_threshold: f64,
    /// How often to check health (seconds)
    pub check_interval_seconds: u64,
    /// Maximum alerts to keep in history
    pub max_alert_history: usize,
    /// Enable automatic recovery actions
    pub auto_recovery_enabled: bool,
}

/// Context health statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStats {
    pub total_checks: u64,
    pub healthy_checks: u64,
    pub warning_checks: u64,
    pub critical_checks: u64,
    pub emergency_checks: u64,
    pub average_token_usage: f64,
    pub peak_token_usage: f64,
    pub last_check: DateTime<Utc>,
    pub uptime_percentage: f64,
}

/// Context health monitor
pub struct ContextHealthMonitor {
    /// Tokenizer for health analysis
    tokenizer: Arc<ContextTokenizer>,
    /// Monitoring configuration
    config: MonitorConfig,
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, ContextAlert>>>,
    /// Alert history
    alert_history: Arc<RwLock<VecDeque<ContextAlert>>>,
    /// Health statistics
    stats: Arc<RwLock<HealthStats>>,
    /// Health check history for trend analysis
    health_history: Arc<RwLock<VecDeque<(DateTime<Utc>, ContextMetrics)>>>,
}

impl ContextHealthMonitor {
    /// Create new context health monitor
    pub fn new(tokenizer: Arc<ContextTokenizer>, config: MonitorConfig) -> Self {
        Self {
            tokenizer,
            config,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(HealthStats {
                total_checks: 0,
                healthy_checks: 0,
                warning_checks: 0,
                critical_checks: 0,
                emergency_checks: 0,
                average_token_usage: 0.0,
                peak_token_usage: 0.0,
                last_check: Utc::now(),
                uptime_percentage: 100.0,
            })),
            health_history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }
    
    /// Perform comprehensive health check
    pub async fn check_health(&self, context: &GlobalContext) -> Result<ContextMetrics> {
        let metrics = self.tokenizer.analyze_context_health(context).await?;
        
        // Update statistics
        self.update_stats(&metrics).await;
        
        // Store health history
        {
            let mut history = self.health_history.write().await;
            history.push_back((Utc::now(), metrics.clone()));
            
            // Keep only last 24 hours of data
            let cutoff = Utc::now() - Duration::hours(24);
            while let Some((timestamp, _)) = history.front() {
                if *timestamp < cutoff {
                    history.pop_front();
                } else {
                    break;
                }
            }
        }
        
        // Check for alerts
        self.check_for_alerts(&metrics).await?;
        
        // Trigger automatic recovery if enabled
        if self.config.auto_recovery_enabled {
            self.trigger_auto_recovery(&metrics).await?;
        }
        
        Ok(metrics)
    }
    
    /// Update health statistics
    async fn update_stats(&self, metrics: &ContextMetrics) {
        let mut stats = self.stats.write().await;
        
        stats.total_checks += 1;
        stats.last_check = Utc::now();
        
        // Update counters based on health status
        match metrics.health_status {
            ContextHealth::Healthy => stats.healthy_checks += 1,
            ContextHealth::Warning => stats.warning_checks += 1,
            ContextHealth::Critical => stats.critical_checks += 1,
            ContextHealth::Overflow => stats.emergency_checks += 1,
        }
        
        // Update usage statistics
        let total_usage = stats.average_token_usage * (stats.total_checks - 1) as f64 + metrics.usage_percentage;
        stats.average_token_usage = total_usage / stats.total_checks as f64;
        
        if metrics.usage_percentage > stats.peak_token_usage {
            stats.peak_token_usage = metrics.usage_percentage;
        }
        
        // Calculate uptime percentage
        stats.uptime_percentage = (stats.healthy_checks + stats.warning_checks) as f64 / stats.total_checks as f64 * 100.0;
    }
    
    /// Check for alert conditions
    async fn check_for_alerts(&self, metrics: &ContextMetrics) -> Result<()> {
        let alert = match metrics.health_status {
            ContextHealth::Overflow => Some(self.create_alert(
                AlertSeverity::Emergency,
                "Context Overflow",
                "Context has exceeded token limits and may cause system instability",
                metrics,
                vec![
                    "Immediately compress context".to_string(),
                    "Archive old data".to_string(),
                    "Restart affected systems".to_string(),
                ],
            )),
            ContextHealth::Critical => Some(self.create_alert(
                AlertSeverity::Critical,
                "Critical Token Usage",
                &format!("Context usage at {:.1}% - approaching limits", metrics.usage_percentage),
                metrics,
                vec![
                    "Compress context data".to_string(),
                    "Review active workflows".to_string(),
                    "Clear unnecessary cache".to_string(),
                ],
            )),
            ContextHealth::Warning => {
                if metrics.usage_percentage > self.config.warning_threshold {
                    Some(self.create_alert(
                        AlertSeverity::Warning,
                        "High Token Usage",
                        &format!("Context usage at {:.1}% - monitor closely", metrics.usage_percentage),
                        metrics,
                        vec![
                            "Monitor usage trends".to_string(),
                            "Prepare compression strategies".to_string(),
                        ],
                    ))
                } else {
                    None
                }
            },
            ContextHealth::Healthy => None,
        };
        
        if let Some(alert) = alert {
            self.add_alert(alert).await;
        }
        
        Ok(())
    }
    
    /// Create a new alert
    fn create_alert(
        &self,
        severity: AlertSeverity,
        title: &str,
        description: &str,
        metrics: &ContextMetrics,
        suggested_actions: Vec<String>,
    ) -> ContextAlert {
        ContextAlert {
            id: Uuid::new_v4().to_string(),
            severity,
            title: title.to_string(),
            description: description.to_string(),
            context_id: "global".to_string(), // TODO: Use actual context ID
            metrics: metrics.clone(),
            suggested_actions,
            created_at: Utc::now(),
            acknowledged: false,
        }
    }
    
    /// Add alert to active alerts and history
    async fn add_alert(&self, alert: ContextAlert) {
        // Add to active alerts
        {
            let mut active = self.active_alerts.write().await;
            active.insert(alert.id.clone(), alert.clone());
        }
        
        // Add to history
        {
            let mut history = self.alert_history.write().await;
            history.push_back(alert.clone());
            
            // Limit history size
            while history.len() > self.config.max_alert_history {
                history.pop_front();
            }
        }
        
        // Log alert
        match alert.severity {
            AlertSeverity::Emergency => tracing::error!("EMERGENCY ALERT: {} - {}", alert.title, alert.description),
            AlertSeverity::Critical => tracing::error!("CRITICAL ALERT: {} - {}", alert.title, alert.description),
            AlertSeverity::Warning => tracing::warn!("WARNING ALERT: {} - {}", alert.title, alert.description),
            AlertSeverity::Info => tracing::info!("INFO ALERT: {} - {}", alert.title, alert.description),
        }
    }
    
    /// Trigger automatic recovery actions
    async fn trigger_auto_recovery(&self, metrics: &ContextMetrics) -> Result<()> {
        match metrics.health_status {
            ContextHealth::Overflow | ContextHealth::Critical => {
                tracing::info!("Triggering automatic context compression due to high usage");
                // TODO: Integrate with actual compression system
                // self.context_bus.compress_context_intelligent().await?;
            },
            _ => {}
        }
        Ok(())
    }
    
    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<ContextAlert> {
        let active = self.active_alerts.read().await;
        active.values().cloned().collect()
    }
    
    /// Acknowledge alert
    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let mut active = self.active_alerts.write().await;
        if let Some(alert) = active.get_mut(alert_id) {
            alert.acknowledged = true;
            tracing::info!("Alert acknowledged: {}", alert_id);
        }
        Ok(())
    }
    
    /// Clear acknowledged alerts
    pub async fn clear_acknowledged_alerts(&self) -> Result<()> {
        let mut active = self.active_alerts.write().await;
        active.retain(|_, alert| !alert.acknowledged);
        Ok(())
    }
    
    /// Get health statistics
    pub async fn get_health_stats(&self) -> HealthStats {
        self.stats.read().await.clone()
    }
    
    /// Get health trend analysis
    pub async fn get_health_trends(&self) -> Result<HealthTrendAnalysis> {
        let history = self.health_history.read().await;
        
        if history.len() < 2 {
            return Ok(HealthTrendAnalysis {
                trend_direction: TrendDirection::Stable,
                average_usage: 0.0,
                usage_variance: 0.0,
                prediction_next_hour: 0.0,
            });
        }
        
        let usage_values: Vec<f64> = history.iter().map(|(_, metrics)| metrics.usage_percentage).collect();
        let average_usage = usage_values.iter().sum::<f64>() / usage_values.len() as f64;
        
        // Calculate variance
        let variance = usage_values.iter()
            .map(|&x| (x - average_usage).powi(2))
            .sum::<f64>() / usage_values.len() as f64;
        
        // Simple trend analysis (last 5 vs previous 5)
        let recent_avg = if usage_values.len() >= 10 {
            let recent: Vec<f64> = usage_values.iter().rev().take(5).cloned().collect();
            let previous: Vec<f64> = usage_values.iter().rev().skip(5).take(5).cloned().collect();
            
            let recent_avg = recent.iter().sum::<f64>() / recent.len() as f64;
            let previous_avg = previous.iter().sum::<f64>() / previous.len() as f64;
            
            if recent_avg > previous_avg + 5.0 {
                TrendDirection::Increasing
            } else if recent_avg < previous_avg - 5.0 {
                TrendDirection::Decreasing
            } else {
                TrendDirection::Stable
            }
        } else {
            TrendDirection::Stable
        };
        
        // Simple prediction (linear extrapolation)
        let prediction = if usage_values.len() >= 3 {
            let last_three: Vec<f64> = usage_values.iter().rev().take(3).cloned().collect();
            let trend = (last_three[0] - last_three[2]) / 2.0; // Simple slope
            (last_three[0] + trend).max(0.0).min(100.0)
        } else {
            average_usage
        };
        
        Ok(HealthTrendAnalysis {
            trend_direction: recent_avg,
            average_usage,
            usage_variance: variance,
            prediction_next_hour: prediction,
        })
    }
}

/// Health trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTrendAnalysis {
    pub trend_direction: TrendDirection,
    pub average_usage: f64,
    pub usage_variance: f64,
    pub prediction_next_hour: f64,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 70.0,
            critical_threshold: 90.0,
            check_interval_seconds: 30,
            max_alert_history: 1000,
            auto_recovery_enabled: true,
        }
    }
}
