//! # Result Synthesis
//! 
//! Combines results from multiple agents into coherent responses.

use super::*;

/// Result synthesizer for combining agent outputs
#[derive(Debug)]
pub struct ResultSynthesizer {
    synthesis_strategies: Vec<SynthesisStrategy>,
}

impl ResultSynthesizer {
    pub fn new() -> Self {
        Self {
            synthesis_strategies: vec![
                SynthesisStrategy::Concatenation,
                SynthesisStrategy::Summarization,
                SynthesisStrategy::Prioritization,
            ],
        }
    }

    pub async fn synthesize_results(
        &self,
        agent_results: &[AgentResult],
        intent: &UserIntent,
        context: &SymbioteContext,
    ) -> Result<SymbioteResponse> {
        let mut content = String::new();
        let mut agent_actions = Vec::new();
        let mut results = Vec::new();
        let mut suggestions = Vec::new();

        // Process each agent result
        for result in agent_results {
            if result.success {
                if let Some(output) = &result.output {
                    content.push_str(&self.format_agent_output(result, output).await?);
                    content.push('\n');
                }
                
                agent_actions.push(AgentAction {
                    agent_id: result.agent_id.clone(),
                    agent_type: "unknown".to_string(), // Would get from registry
                    action: "executed".to_string(),
                    parameters: HashMap::new(),
                    timestamp: Utc::now(),
                    status: ActionStatus::Completed,
                });

                results.push(ActionResult {
                    action_id: Uuid::new_v4().to_string(),
                    agent_id: result.agent_id.clone(),
                    result_type: ResultType::Data,
                    data: serde_json::json!({}),
                    success: true,
                    error_message: None,
                    timestamp: Utc::now(),
                });
            } else {
                content.push_str(&format!("❌ Agent {} failed: {}", 
                    result.agent_id, 
                    result.error.as_deref().unwrap_or("Unknown error")));
                content.push('\n');
            }
        }

        // Generate suggestions based on results
        suggestions.extend(self.generate_suggestions(agent_results, intent).await?);

        // Determine response status
        let status = if agent_results.iter().all(|r| r.success) {
            ResponseStatus::Success
        } else if agent_results.iter().any(|r| r.success) {
            ResponseStatus::Partial
        } else {
            ResponseStatus::Failed
        };

        Ok(SymbioteResponse {
            id: Uuid::new_v4().to_string(),
            content: if content.is_empty() { 
                "Task completed successfully!".to_string() 
            } else { 
                content 
            },
            timestamp: Utc::now(),
            agent_actions,
            results,
            suggestions,
            status,
        })
    }

    async fn format_agent_output(&self, result: &AgentResult, output: &serde_json::Value) -> Result<String> {
        match result.agent_id.as_str() {
            "web_research" => {
                if let Some(search_results) = output.get("search_results") {
                    Ok(format!("🔍 **Search Results:**\n{}", 
                        serde_json::to_string_pretty(search_results).unwrap_or_default()))
                } else {
                    Ok("🔍 Web research completed".to_string())
                }
            },
            "workflow" => {
                Ok("⚡ Workflow operation completed".to_string())
            },
            "email" => {
                Ok("📧 Email operation completed".to_string())
            },
            "crypto_trading" => {
                Ok("💰 Crypto trading operation completed".to_string())
            },
            _ => {
                Ok(format!("✅ {} completed successfully", result.agent_id))
            }
        }
    }

    async fn generate_suggestions(
        &self,
        agent_results: &[AgentResult],
        intent: &UserIntent,
    ) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();

        // Generate suggestions based on intent and results
        match intent.intent_type.as_str() {
            "web_search" => {
                suggestions.push(Suggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::NextAction,
                    title: "Refine Search".to_string(),
                    description: "Try a more specific search query".to_string(),
                    action: Some("web_search".to_string()),
                    confidence: 0.8,
                });
            },
            "create_workflow" => {
                suggestions.push(Suggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::NextAction,
                    title: "Test Workflow".to_string(),
                    description: "Run the workflow to test it works correctly".to_string(),
                    action: Some("execute_workflow".to_string()),
                    confidence: 0.9,
                });
            },
            _ => {
                suggestions.push(Suggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::Learning,
                    title: "Explore More".to_string(),
                    description: "Ask me anything else you'd like to do!".to_string(),
                    action: None,
                    confidence: 0.7,
                });
            }
        }

        Ok(suggestions)
    }
}

/// Synthesis strategies
#[derive(Debug, Clone)]
pub enum SynthesisStrategy {
    Concatenation,
    Summarization,
    Prioritization,
}
