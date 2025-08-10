//! # Natural Language Processing
//! 
//! NLP engine for understanding user intent and extracting actionable information.

use super::*;

/// Natural language processor for understanding user intent
#[derive(Debug)]
pub struct NaturalLanguageProcessor {
    intent_patterns: Vec<IntentPattern>,
}

impl NaturalLanguageProcessor {
    pub fn new() -> Self {
        Self {
            intent_patterns: Self::create_default_patterns(),
        }
    }

    pub async fn process_message(
        &self,
        message: &UserMessage,
        context: &SymbioteContext,
    ) -> Result<UserIntent> {
        let content = message.content.to_lowercase();
        
        // Simple pattern matching (would use real NLP in practice)
        for pattern in &self.intent_patterns {
            if pattern.matches(&content) {
                return Ok(UserIntent {
                    intent_type: pattern.intent_type.clone(),
                    query: message.content.clone(),
                    confidence: pattern.confidence,
                    parameters: self.extract_parameters(&content, &pattern).await?,
                    entities: self.extract_entities(&content).await?,
                });
            }
        }
        
        // Default intent
        Ok(UserIntent {
            intent_type: "general_query".to_string(),
            query: message.content.clone(),
            confidence: 0.5,
            parameters: HashMap::new(),
            entities: Vec::new(),
        })
    }

    async fn extract_parameters(
        &self,
        content: &str,
        pattern: &IntentPattern,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut parameters = HashMap::new();
        
        // Extract quoted strings as parameters
        if let Some(start) = content.find('"') {
            if let Some(end) = content[start + 1..].find('"') {
                let query = &content[start + 1..start + 1 + end];
                parameters.insert("query".to_string(), serde_json::json!(query));
            }
        }
        
        Ok(parameters)
    }

    async fn extract_entities(&self, content: &str) -> Result<Vec<String>> {
        let mut entities = Vec::new();
        
        // Simple entity extraction
        if content.contains("workflow") {
            entities.push("workflow".to_string());
        }
        if content.contains("notebook") {
            entities.push("notebook".to_string());
        }
        if content.contains("crypto") || content.contains("trading") {
            entities.push("crypto".to_string());
        }
        
        Ok(entities)
    }

    fn create_default_patterns() -> Vec<IntentPattern> {
        vec![
            IntentPattern {
                intent_type: "web_search".to_string(),
                keywords: vec!["search", "find", "look up", "research"],
                confidence: 0.9,
            },
            IntentPattern {
                intent_type: "create_workflow".to_string(),
                keywords: vec!["create workflow", "new workflow", "build workflow"],
                confidence: 0.95,
            },
            IntentPattern {
                intent_type: "send_email".to_string(),
                keywords: vec!["send email", "email", "message"],
                confidence: 0.85,
            },
            IntentPattern {
                intent_type: "crypto_trade".to_string(),
                keywords: vec!["trade", "buy", "sell", "crypto", "bitcoin"],
                confidence: 0.9,
            },
        ]
    }
}

/// Intent pattern for matching user requests
#[derive(Debug, Clone)]
pub struct IntentPattern {
    pub intent_type: String,
    pub keywords: Vec<&'static str>,
    pub confidence: f64,
}

impl IntentPattern {
    pub fn matches(&self, content: &str) -> bool {
        self.keywords.iter().any(|keyword| content.contains(keyword))
    }
}
