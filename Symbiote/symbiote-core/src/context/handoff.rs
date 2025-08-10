//! # Safe Context Handoff System
//! 
//! Implements atomic, validated context handoffs between systems with full
//! rollback capabilities and integrity verification.

use crate::{Result, SymbioteError};
use super::{SystemId, ContextUpdate, tokenizer::{ContextTokenizer, TokenizedContext}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Context handoff package with validation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHandoffPackage {
    pub handoff_id: String,
    pub context: TokenizedContext,
    pub source_system: SystemId,
    pub target_system: SystemId,
    pub token_count: usize,
    pub checksum: String,
    pub compression_applied: bool,
    pub created_at: DateTime<Utc>,
    pub timeout_at: DateTime<Utc>,
}

/// Handoff transaction state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandoffState {
    Preparing,
    ReadyToCommit,
    Committed,
    Aborted,
    Failed(String),
}

/// Handoff transaction for atomic operations
#[derive(Debug, Clone)]
pub struct HandoffTransaction {
    pub id: String,
    pub package: ContextHandoffPackage,
    pub state: HandoffState,
    pub operations: Vec<HandoffOperation>,
    pub compensation_actions: Vec<CompensationAction>,
    pub created_at: DateTime<Utc>,
}

/// Individual handoff operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandoffOperation {
    PrepareSource { system: SystemId, context_id: String },
    PrepareTarget { system: SystemId, context: TokenizedContext },
    TransferContext { from: SystemId, to: SystemId, context: TokenizedContext },
    VerifyTransfer { system: SystemId, context_id: String },
    CleanupSource { system: SystemId, context_id: String },
}

/// Compensation action for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompensationAction {
    RestoreContext { system: SystemId, context: TokenizedContext },
    CleanupReceiver { system: SystemId, context_id: String },
    NotifyFailure { system: SystemId, error: String },
}

/// Safe context handoff coordinator
#[derive(Debug)]
pub struct ContextHandoffCoordinator {
    /// Active handoff transactions
    active_handoffs: Arc<RwLock<HashMap<String, HandoffTransaction>>>,
    /// Context tokenizer for validation
    tokenizer: Arc<ContextTokenizer>,
    /// Handoff timeout in seconds
    timeout_seconds: u64,
}

impl ContextHandoffCoordinator {
    pub fn new(tokenizer: Arc<ContextTokenizer>, timeout_seconds: u64) -> Self {
        Self {
            active_handoffs: Arc::new(RwLock::new(HashMap::new())),
            tokenizer,
            timeout_seconds,
        }
    }
    
    /// Execute safe context handoff with full validation
    pub async fn execute_handoff(
        &self,
        context: TokenizedContext,
        from: SystemId,
        to: SystemId,
    ) -> Result<String> {
        // 1. Create handoff package with validation
        let handoff_id = Uuid::new_v4().to_string();
        let package = self.create_handoff_package(handoff_id.clone(), context, from.clone(), to.clone()).await?;
        
        // 2. Validate context before handoff
        self.validate_handoff_package(&package).await?;
        
        // 3. Create transaction
        let transaction = HandoffTransaction {
            id: handoff_id.clone(),
            package: package.clone(),
            state: HandoffState::Preparing,
            operations: Vec::new(),
            compensation_actions: Vec::new(),
            created_at: Utc::now(),
        };
        
        // 4. Store transaction
        {
            let mut handoffs = self.active_handoffs.write().await;
            handoffs.insert(handoff_id.clone(), transaction);
        }
        
        // 5. Execute handoff saga
        match self.execute_handoff_saga(&handoff_id).await {
            Ok(_) => {
                // Clean up successful transaction
                let mut handoffs = self.active_handoffs.write().await;
                handoffs.remove(&handoff_id);
                Ok(handoff_id)
            },
            Err(e) => {
                // Execute compensation and cleanup
                self.compensate_handoff(&handoff_id).await?;
                let mut handoffs = self.active_handoffs.write().await;
                handoffs.remove(&handoff_id);
                Err(e)
            }
        }
    }
    
    /// Create validated handoff package
    async fn create_handoff_package(
        &self,
        handoff_id: String,
        context: TokenizedContext,
        from: SystemId,
        to: SystemId,
    ) -> Result<ContextHandoffPackage> {
        let token_count = context.token_metadata.total_tokens;
        let checksum = context.checksum.clone();
        let compression_applied = context.token_metadata.compression_applied;
        
        Ok(ContextHandoffPackage {
            handoff_id,
            context,
            source_system: from,
            target_system: to,
            token_count,
            checksum,
            compression_applied,
            created_at: Utc::now(),
            timeout_at: Utc::now() + chrono::Duration::seconds(self.timeout_seconds as i64),
        })
    }
    
    /// Validate handoff package integrity
    async fn validate_handoff_package(&self, package: &ContextHandoffPackage) -> Result<()> {
        // 1. Check token limits
        if package.token_count > self.tokenizer.max_tokens() {
            return Err(SymbioteError::ContextCorruption(
                format!("Context exceeds token limit: {} > {}", package.token_count, self.tokenizer.max_tokens())
            ));
        }
        
        // 2. Validate compression integrity
        if !self.tokenizer.validate_compression(&package.context, &package.checksum).await? {
            return Err(SymbioteError::ContextCorruption(
                "Context compression validation failed".to_string()
            ));
        }
        
        // 3. Check timeout
        if Utc::now() > package.timeout_at {
            return Err(SymbioteError::Timeout(
                "Handoff package has expired".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Execute handoff saga pattern
    async fn execute_handoff_saga(&self, handoff_id: &str) -> Result<()> {
        let mut transaction = {
            let handoffs = self.active_handoffs.read().await;
            handoffs.get(handoff_id)
                .ok_or_else(|| SymbioteError::NotFound(format!("Handoff not found: {}", handoff_id)))?
                .clone()
        };
        
        // Extract values to avoid borrowing issues
        let source_system = transaction.package.source_system.clone();
        let target_system = transaction.package.target_system.clone();
        let context_id = transaction.package.context.token_metadata.created_at.to_rfc3339();
        let context = transaction.package.context.clone();
        let handoff_id = transaction.id.clone();

        // Step 1: Prepare source system
        self.add_operation(&mut transaction, HandoffOperation::PrepareSource {
            system: source_system.clone(),
            context_id: context_id.clone(),
        });
        self.add_compensation(&mut transaction, CompensationAction::NotifyFailure {
            system: source_system.clone(),
            error: "Handoff preparation failed".to_string(),
        });
        
        // Step 2: Prepare target system
        self.add_operation(&mut transaction, HandoffOperation::PrepareTarget {
            system: target_system.clone(),
            context: context.clone(),
        });
        self.add_compensation(&mut transaction, CompensationAction::CleanupReceiver {
            system: target_system.clone(),
            context_id: handoff_id.clone(),
        });
        
        // Step 3: Transfer context
        self.add_operation(&mut transaction, HandoffOperation::TransferContext {
            from: source_system.clone(),
            to: target_system.clone(),
            context: context.clone(),
        });
        self.add_compensation(&mut transaction, CompensationAction::RestoreContext {
            system: source_system.clone(),
            context: context.clone(),
        });

        // Step 4: Verify transfer
        self.add_operation(&mut transaction, HandoffOperation::VerifyTransfer {
            system: target_system.clone(),
            context_id: handoff_id.clone(),
        });

        // Step 5: Cleanup source
        self.add_operation(&mut transaction, HandoffOperation::CleanupSource {
            system: source_system.clone(),
            context_id: context_id.clone(),
        });
        
        // Execute all operations
        for operation in &transaction.operations {
            if let Err(e) = self.execute_operation(operation).await {
                transaction.state = HandoffState::Failed(e.to_string());
                self.update_transaction(&handoff_id, transaction).await?;
                return Err(e);
            }
        }
        
        // Mark as committed
        transaction.state = HandoffState::Committed;
        self.update_transaction(&handoff_id, transaction).await?;
        
        Ok(())
    }
    
    /// Add operation to transaction
    fn add_operation(&self, transaction: &mut HandoffTransaction, operation: HandoffOperation) {
        transaction.operations.push(operation);
    }
    
    /// Add compensation action to transaction
    fn add_compensation(&self, transaction: &mut HandoffTransaction, action: CompensationAction) {
        transaction.compensation_actions.push(action);
    }
    
    /// Execute individual operation (placeholder - would integrate with actual systems)
    async fn execute_operation(&self, operation: &HandoffOperation) -> Result<()> {
        match operation {
            HandoffOperation::PrepareSource { system, context_id } => {
                tracing::info!("Preparing source system {:?} for context {}", system, context_id);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
            HandoffOperation::PrepareTarget { system, context } => {
                tracing::info!("Preparing target system {:?} for context", system);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
            HandoffOperation::TransferContext { from, to, context } => {
                tracing::info!("Transferring context from {:?} to {:?}", from, to);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
            HandoffOperation::VerifyTransfer { system, context_id } => {
                tracing::info!("Verifying transfer in system {:?} for context {}", system, context_id);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
            HandoffOperation::CleanupSource { system, context_id } => {
                tracing::info!("Cleaning up source system {:?} for context {}", system, context_id);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
        }
    }
    
    /// Execute compensation actions for rollback
    async fn compensate_handoff(&self, handoff_id: &str) -> Result<()> {
        let transaction = {
            let handoffs = self.active_handoffs.read().await;
            handoffs.get(handoff_id)
                .ok_or_else(|| SymbioteError::NotFound(format!("Handoff not found: {}", handoff_id)))?
                .clone()
        };
        
        // Execute compensation actions in reverse order
        for action in transaction.compensation_actions.iter().rev() {
            if let Err(e) = self.execute_compensation(action).await {
                tracing::error!("Compensation failed: {:?}", e);
                // Continue with other compensations
            }
        }
        
        Ok(())
    }
    
    /// Execute compensation action (placeholder)
    async fn execute_compensation(&self, action: &CompensationAction) -> Result<()> {
        match action {
            CompensationAction::RestoreContext { system, context } => {
                tracing::info!("Restoring context in system {:?}", system);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
            CompensationAction::CleanupReceiver { system, context_id } => {
                tracing::info!("Cleaning up receiver system {:?} for context {}", system, context_id);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
            CompensationAction::NotifyFailure { system, error } => {
                tracing::info!("Notifying system {:?} of failure: {}", system, error);
                // TODO: Integrate with actual system APIs
                Ok(())
            },
        }
    }
    
    /// Update transaction state
    async fn update_transaction(&self, handoff_id: &str, transaction: HandoffTransaction) -> Result<()> {
        let mut handoffs = self.active_handoffs.write().await;
        handoffs.insert(handoff_id.to_string(), transaction);
        Ok(())
    }
    
    /// Clean up expired handoffs
    pub async fn cleanup_expired_handoffs(&self) -> Result<()> {
        let mut handoffs = self.active_handoffs.write().await;
        let now = Utc::now();
        
        let expired_ids: Vec<String> = handoffs.iter()
            .filter(|(_, transaction)| now > transaction.package.timeout_at)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired_ids {
            if let Some(transaction) = handoffs.remove(&id) {
                tracing::warn!("Cleaning up expired handoff: {}", id);
                // Execute compensation for expired handoff
                drop(handoffs); // Release lock before async operation
                self.compensate_handoff(&id).await?;
                handoffs = self.active_handoffs.write().await; // Re-acquire lock
            }
        }
        
        Ok(())
    }
}
