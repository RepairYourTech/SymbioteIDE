use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Performance monitoring system for SymbioteIDE
/// Tracks startup time, memory usage, and operation performance
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    startup_time: Arc<RwLock<Option<Instant>>>,
    metrics: Arc<RwLock<HashMap<String, PerformanceMetric>>>,
    operation_timers: Arc<RwLock<HashMap<String, Instant>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub name: String,
    pub duration_ms: u64,
    pub memory_usage_mb: f64,
    pub timestamp: String,
    pub operation_type: OperationType,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Startup,
    AgentExecution,
    CodeParsing,
    ContextRetrieval,
    WorkflowExecution,
    FileOperation,
    AIProviderCall,
    DatabaseQuery,
    UIRender,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub startup_time_ms: Option<u64>,
    pub current_memory_mb: f64,
    pub peak_memory_mb: f64,
    pub total_operations: usize,
    pub average_operation_time_ms: f64,
    pub slowest_operations: Vec<PerformanceMetric>,
    pub error_rate: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceTargets {
    pub max_startup_time_ms: u64,    // Target: 3000ms
    pub max_memory_usage_mb: f64,    // Target: 500MB
    pub max_operation_time_ms: u64,  // Target: 100ms for most operations
    pub max_error_rate: f64,         // Target: <1%
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hit_count: u64,
    pub miss_count: u64,
    pub total_requests: u64,
    pub hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePerformance {
    pub average_lookup_time_ms: f64,
    pub cache_size_mb: f64,
    pub eviction_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub cache_stats: CacheStats,
    pub cache_performance: CachePerformance,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            max_startup_time_ms: 3000,  // 3 seconds
            max_memory_usage_mb: 500.0, // 500MB
            max_operation_time_ms: 100, // 100ms
            max_error_rate: 0.01,       // 1%
        }
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            startup_time: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            operation_timers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Mark the start of application startup
    pub async fn mark_startup_begin(&self) {
        let mut startup_time = self.startup_time.write().await;
        *startup_time = Some(Instant::now());
    }

    /// Mark the end of application startup and record the metric
    pub async fn mark_startup_complete(&self) -> Result<u64, String> {
        let startup_time = self.startup_time.read().await;
        
        if let Some(start_time) = *startup_time {
            let duration = start_time.elapsed();
            let duration_ms = duration.as_millis() as u64;
            
            let metric = PerformanceMetric {
                name: "application_startup".to_string(),
                duration_ms,
                memory_usage_mb: self.get_current_memory_usage(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                operation_type: OperationType::Startup,
                success: true,
                error_message: None,
            };

            let mut metrics = self.metrics.write().await;
            metrics.insert("startup".to_string(), metric);

            println!("🚀 SymbioteIDE startup completed in {}ms", duration_ms);
            
            // Check against performance targets
            let targets = PerformanceTargets::default();
            if duration_ms > targets.max_startup_time_ms {
                println!("⚠️  Startup time {}ms exceeds target {}ms", duration_ms, targets.max_startup_time_ms);
            } else {
                println!("✅ Startup time within target ({}/{}ms)", duration_ms, targets.max_startup_time_ms);
            }

            Ok(duration_ms)
        } else {
            Err("Startup time not marked".to_string())
        }
    }

    /// Start timing an operation
    pub async fn start_operation(&self, operation_id: &str) {
        let mut timers = self.operation_timers.write().await;
        timers.insert(operation_id.to_string(), Instant::now());
    }

    /// Complete timing an operation and record the metric
    pub async fn complete_operation(
        &self, 
        operation_id: &str, 
        operation_type: OperationType,
        success: bool,
        error_message: Option<String>
    ) -> Result<u64, String> {
        let mut timers = self.operation_timers.write().await;
        
        if let Some(start_time) = timers.remove(operation_id) {
            let duration = start_time.elapsed();
            let duration_ms = duration.as_millis() as u64;
            
            let metric = PerformanceMetric {
                name: operation_id.to_string(),
                duration_ms,
                memory_usage_mb: self.get_current_memory_usage(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                operation_type,
                success,
                error_message,
            };

            let mut metrics = self.metrics.write().await;
            metrics.insert(operation_id.to_string(), metric);

            // Log slow operations
            let targets = PerformanceTargets::default();
            if duration_ms > targets.max_operation_time_ms {
                println!("⚠️  Slow operation: {} took {}ms (target: {}ms)", operation_id, duration_ms, targets.max_operation_time_ms);
            }

            Ok(duration_ms)
        } else {
            Err(format!("Operation {} not found in timers", operation_id))
        }
    }

    /// Get current memory usage in MB
    pub fn get_current_memory_usage(&self) -> f64 {
        // Use system memory information
        #[cfg(target_os = "windows")]
        {
            // Simplified memory usage for Windows - use a fallback approach
            return 0.0; // Stub implementation
        }
        
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<f64>() {
                                return kb / 1024.0; // Convert KB to MB
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback estimate
        0.0
    }

    /// Generate a comprehensive performance report
    pub async fn generate_report(&self) -> PerformanceReport {
        let metrics = self.metrics.read().await;
        let startup_time = self.startup_time.read().await;
        
        let startup_time_ms = if let Some(start_time) = *startup_time {
            Some(start_time.elapsed().as_millis() as u64)
        } else {
            None
        };

        let current_memory = self.get_current_memory_usage();
        let total_operations = metrics.len();
        
        let mut total_duration = 0u64;
        let mut error_count = 0;
        let mut peak_memory = 0.0f64;
        let mut slowest_operations: Vec<PerformanceMetric> = Vec::new();

        for metric in metrics.values() {
            total_duration += metric.duration_ms;
            if !metric.success {
                error_count += 1;
            }
            if metric.memory_usage_mb > peak_memory {
                peak_memory = metric.memory_usage_mb;
            }
            slowest_operations.push(metric.clone());
        }

        // Sort by duration and take top 10 slowest
        slowest_operations.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
        slowest_operations.truncate(10);

        let average_operation_time = if total_operations > 0 {
            total_duration as f64 / total_operations as f64
        } else {
            0.0
        };

        let error_rate = if total_operations > 0 {
            error_count as f64 / total_operations as f64
        } else {
            0.0
        };

        let uptime_seconds = if let Some(start_time) = *startup_time {
            start_time.elapsed().as_secs()
        } else {
            0
        };

        PerformanceReport {
            startup_time_ms,
            current_memory_mb: current_memory,
            peak_memory_mb: peak_memory,
            total_operations,
            average_operation_time_ms: average_operation_time,
            slowest_operations,
            error_rate,
            uptime_seconds,
        }
    }

    /// Check if performance targets are being met
    pub async fn check_performance_targets(&self) -> Vec<String> {
        let report = self.generate_report().await;
        let targets = PerformanceTargets::default();
        let mut violations = Vec::new();

        if let Some(startup_time) = report.startup_time_ms {
            if startup_time > targets.max_startup_time_ms {
                violations.push(format!(
                    "Startup time {}ms exceeds target {}ms", 
                    startup_time, targets.max_startup_time_ms
                ));
            }
        }

        if report.current_memory_mb > targets.max_memory_usage_mb {
            violations.push(format!(
                "Memory usage {:.1}MB exceeds target {:.1}MB", 
                report.current_memory_mb, targets.max_memory_usage_mb
            ));
        }

        if report.error_rate > targets.max_error_rate {
            violations.push(format!(
                "Error rate {:.2}% exceeds target {:.2}%", 
                report.error_rate * 100.0, targets.max_error_rate * 100.0
            ));
        }

        violations
    }

    /// Get metrics for a specific operation type
    pub async fn get_metrics_by_type(&self, operation_type: &OperationType) -> Vec<PerformanceMetric> {
        let metrics = self.metrics.read().await;
        metrics.values()
            .filter(|metric| std::mem::discriminant(&metric.operation_type) == std::mem::discriminant(operation_type))
            .cloned()
            .collect()
    }

    /// Clear all metrics (useful for testing or reset)
    pub async fn clear_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        let mut timers = self.operation_timers.write().await;
        metrics.clear();
        timers.clear();
    }
}

// Global performance monitor instance
lazy_static::lazy_static! {
    pub static ref PERFORMANCE_MONITOR: PerformanceMonitor = PerformanceMonitor::new();
}

/// Convenience macro for timing operations
#[macro_export]
macro_rules! time_operation {
    ($operation_id:expr, $operation_type:expr, $code:block) => {{
        let monitor = &crate::performance_monitor::PERFORMANCE_MONITOR;
        monitor.start_operation($operation_id).await;
        
        let result = $code;
        
        let success = result.is_ok();
        let error_message = if let Err(ref e) = result {
            Some(e.to_string())
        } else {
            None
        };
        
        monitor.complete_operation($operation_id, $operation_type, success, error_message).await.ok();
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_performance_monitoring() {
        let monitor = PerformanceMonitor::new();
        
        // Test startup timing
        monitor.mark_startup_begin().await;
        sleep(Duration::from_millis(100)).await;
        let startup_time = monitor.mark_startup_complete().await.unwrap();
        assert!(startup_time >= 100);

        // Test operation timing
        monitor.start_operation("test_operation").await;
        sleep(Duration::from_millis(50)).await;
        let operation_time = monitor.complete_operation(
            "test_operation", 
            OperationType::Custom("test".to_string()), 
            true, 
            None
        ).await.unwrap();
        assert!(operation_time >= 50);

        // Test report generation
        let report = monitor.generate_report().await;
        assert!(report.total_operations >= 2);
        assert!(report.startup_time_ms.is_some());
    }

    #[tokio::test]
    async fn test_performance_targets() {
        let monitor = PerformanceMonitor::new();
        
        // Simulate slow startup
        monitor.mark_startup_begin().await;
        sleep(Duration::from_millis(100)).await;
        monitor.mark_startup_complete().await.unwrap();

        let violations = monitor.check_performance_targets().await;
        // Should have no violations for 100ms startup
        assert!(violations.is_empty());
    }
}
