//! File models

use crate::ProjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub project_id: Option<ProjectId>,
    pub size: u64,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub language: Option<String>,
}
